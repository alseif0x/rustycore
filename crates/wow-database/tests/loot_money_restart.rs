//! Cross-process checks of the production money adapters against disposable InnoDB tables.

use anyhow::{Context, Result, ensure};
use sqlx::{MySqlPool, mysql::MySqlConnectOptions};
use std::{env, process::Command, sync::Arc};
use wow_database::CharacterDatabase;
use wow_database::stored_item_money_adapter::MariaDbStoredItemMoneyPersistenceAdapterLikeCpp;
use wow_persistence::{
    StoredItemMoneyPersistenceAttemptLikeCpp as Attempt,
    StoredItemMoneyPersistenceOutcomeLikeCpp as Outcome, StoredItemMoneyPersistencePortLikeCpp,
    StoredItemMoneyPersistenceRequestLikeCpp as Request,
    StoredItemMoneyReconciliationLikeCpp as Reconciliation, StoredItemMoneyRollbackKindLikeCpp,
};

const TEST: &str = "money_claims_survive_process_restart";
const SOCKET: &str = "RUSTYCORE_LOOT_TEST_SOCKET";

async fn chest_money_epoch(epoch: &str, db: Arc<CharacterDatabase>) -> Result<()> {
    use wow_database::group_loot_money_adapter::MariaDbGroupLootMoneyPersistenceAdapterLikeCpp;
    use wow_database::respawn_persistence_adapter::MariaDbRespawnPersistenceAdapterLikeCpp;
    use wow_persistence::{
        GroupLootMoneyPayoutLikeCpp, GroupLootMoneyPersistenceAttemptLikeCpp as Attempt,
        GroupLootMoneyPersistencePortLikeCpp, GroupLootMoneyPersistenceRequestLikeCpp,
        RespawnPersistenceKeyLikeCpp, RespawnPersistenceMutationLikeCpp,
        RespawnPersistenceMutationOutcomeLikeCpp, RespawnPersistencePortLikeCpp,
        WorldLootMoneySource,
    };
    let adapter = MariaDbGroupLootMoneyPersistenceAdapterLikeCpp::new(Arc::clone(&db));
    let source = WorldLootMoneySource {
        realm_id: 1,
        map_id: 0,
        instance_id: 0,
        spawn_id: 9106001,
    };
    let bind = async |source, owner| {
        adapter
            .bind_world_chest_money(source, owner)
            .await
            .map_err(anyhow::Error::msg)
    };
    let payout = |claim_id| GroupLootMoneyPersistenceRequestLikeCpp {
        claim_id: Some(claim_id),
        payouts: vec![GroupLootMoneyPayoutLikeCpp {
            recipient_guid: 201,
            requested_delta: 10,
        }],
        max_money: 1000,
    };
    let current = bind(source, 0)
        .await?
        .context("chest should be available")?;
    if epoch == "after_restock_commit" {
        let (deadline, generation): (i64, u64) = sqlx::query_as(
            "SELECT r.respawnTime, h.generation FROM respawn r JOIN loot_money_source_head h ON h.spawn_id=r.spawnId AND h.map_id=r.mapId AND h.instance_id=r.instanceId WHERE r.spawnId=9106001 AND h.realm_id=1"
        ).fetch_one(db.pool()).await?;
        assert!(deadline > 0);
        assert_eq!(generation, 2);
        assert_eq!(current.generation, generation);
        assert!(!current.spent);
        return Ok(());
    }
    if epoch == "claim" {
        assert!(!current.spent);
        sqlx::query("INSERT INTO characters VALUES (201,100)")
            .execute(db.pool())
            .await?;
        assert!(matches!(
            adapter
                .attempt_group_loot_money_like_cpp(payout(current.claim_id))
                .await,
            Attempt::Applied(_)
        ));
        sqlx::query("UPDATE characters SET money=90 WHERE guid=201")
            .execute(db.pool())
            .await?;
        return Ok(());
    }

    // This child knows only the persistent spawn and scope, even after unrelated spending.
    assert!(current.spent);
    assert_eq!(
        adapter
            .attempt_group_loot_money_like_cpp(payout(current.claim_id))
            .await,
        Attempt::AlreadySettled
    );
    let personal = bind(source, 201).await?.unwrap();
    for independent in [
        personal,
        bind(
            WorldLootMoneySource {
                realm_id: 2,
                ..source
            },
            0,
        )
        .await?
        .unwrap(),
        bind(
            WorldLootMoneySource {
                instance_id: 3,
                ..source
            },
            0,
        )
        .await?
        .unwrap(),
        bind(
            WorldLootMoneySource {
                map_id: 1,
                ..source
            },
            0,
        )
        .await?
        .unwrap(),
    ] {
        assert!(!independent.spent);
        assert_ne!(independent.claim_id, current.claim_id);
    }
    let writer = MariaDbRespawnPersistenceAdapterLikeCpp::new(Arc::clone(&db));
    let arm = |generation, deadline| RespawnPersistenceMutationLikeCpp::ArmChestMoney {
        key: RespawnPersistenceKeyLikeCpp {
            object_type_raw: 1,
            spawn_id: source.spawn_id,
            map_id: source.map_id as u16,
            instance_id: source.instance_id,
        },
        realm_id: source.realm_id,
        generation,
        deadline,
        persist_respawn: true,
    };
    let now: i64 = sqlx::query_scalar("SELECT CAST(UNIX_TIMESTAMP() AS SIGNED)")
        .fetch_one(db.pool())
        .await?;
    // The respawn write fails in this schema until its table is installed.
    // Its failure must also roll back the source deadline.
    assert!(matches!(
        writer
            .execute_mutation_like_cpp(arm(current.generation, now + 3))
            .await,
        RespawnPersistenceMutationOutcomeLikeCpp::Failed { .. }
    ));
    assert_eq!(bind(source, 0).await?, Some(current));
    sqlx::query("CREATE TABLE respawn (type SMALLINT UNSIGNED, spawnId BIGINT UNSIGNED, respawnTime BIGINT NOT NULL, mapId SMALLINT UNSIGNED, instanceId INT UNSIGNED, PRIMARY KEY (type, spawnId, mapId, instanceId)) ENGINE=InnoDB")
        .execute(db.pool()).await?;
    assert_eq!(
        writer
            .execute_mutation_like_cpp(arm(current.generation, now + 3))
            .await,
        RespawnPersistenceMutationOutcomeLikeCpp::Applied { affected_rows: 1 }
    );
    assert!(bind(source, 0).await?.is_none());
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    let next = bind(source, 0).await?.context("restock deadline elapsed")?;
    assert_eq!(next.generation, current.generation + 1);
    assert!(!next.spent);
    assert_ne!(next.claim_id, current.claim_id);
    assert!(matches!(
        adapter
            .attempt_group_loot_money_like_cpp(payout(personal.claim_id))
            .await,
        Attempt::DefinitelyRolledBack { .. } | Attempt::AlreadySettled
    ));
    assert_eq!(
        writer
            .execute_mutation_like_cpp(arm(current.generation, now))
            .await,
        RespawnPersistenceMutationOutcomeLikeCpp::Applied { affected_rows: 0 }
    );
    assert_eq!(bind(source, 0).await?, Some(next));
    assert!(matches!(
        adapter
            .attempt_group_loot_money_like_cpp(payout(next.claim_id))
            .await,
        Attempt::Applied(_)
    ));
    assert!(bind(source, 0).await?.unwrap().spent);
    assert_eq!(
        sqlx::query_scalar::<_, u64>("SELECT money FROM characters WHERE guid=201")
            .fetch_one(db.pool())
            .await?,
        100
    );
    // Exit with the retirement transaction committed and no map acknowledgement.
    assert_eq!(
        writer
            .execute_mutation_like_cpp(arm(next.generation, now))
            .await,
        RespawnPersistenceMutationOutcomeLikeCpp::Applied { affected_rows: 1 }
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires a disposable MariaDB Unix socket in RUSTYCORE_LOOT_TEST_SOCKET"]
async fn money_claims_survive_process_restart() -> Result<()> {
    let socket = env::var(SOCKET).context("supply a disposable MariaDB Unix socket")?;
    let user = env::var("USER")?;
    if let Ok(epoch) = env::var("RUSTYCORE_LOOT_TEST_EPOCH") {
        let schema = env::var("RUSTYCORE_LOOT_TEST_SCHEMA")?;
        let url = wow_database::database::build_connection_string_with_ssl_like_cpp(
            "localhost",
            &socket,
            &user,
            "",
            &schema,
            false,
        );
        let db = Arc::new(CharacterDatabase::open(&url).await?);
        if epoch != "after_restock_commit" {
            stored_money_epoch(&epoch, Arc::clone(&db)).await?;
            shared_money_epoch(&epoch, Arc::clone(&db)).await?;
        }
        return chest_money_epoch(&epoch, db).await;
    }
    let server =
        MySqlPool::connect_with(MySqlConnectOptions::new().username(&user).socket(&socket)).await?;
    let schema = format!("rustycore_loot_it_{}", std::process::id());
    sqlx::query(&format!("CREATE DATABASE `{schema}`"))
        .execute(&server)
        .await?;
    let result = (|| -> Result<()> {
        for epoch in ["claim", "recover", "after_restock_commit"] {
            let status = Command::new(env::current_exe()?)
                .args(["--ignored", "--exact", TEST, "--nocapture"])
                .env("RUSTYCORE_LOOT_TEST_EPOCH", epoch)
                .env("RUSTYCORE_LOOT_TEST_SCHEMA", &schema)
                .status()?;
            ensure!(status.success(), "{epoch} process exited with {status}");
        }
        Ok(())
    })();
    sqlx::query(&format!("DROP DATABASE `{schema}`"))
        .execute(&server)
        .await?;
    result
}

async fn stored_money_epoch(epoch: &str, db: Arc<CharacterDatabase>) -> Result<()> {
    let adapter = MariaDbStoredItemMoneyPersistenceAdapterLikeCpp::new(Arc::clone(&db));
    let request = Request {
        player_guid: 1,
        item_guid: 71,
        cached_notified_amount: 10,
        max_money: 1000,
    };
    let outcome = Outcome {
        before: 100,
        after: 110,
        applied_delta: 10,
        notified_amount: 10,
    };
    let other = Request {
        item_guid: 72,
        ..request
    };
    match epoch {
        "claim" => {
            for statement in [
                "CREATE TABLE characters (guid BIGINT UNSIGNED PRIMARY KEY, money BIGINT UNSIGNED NOT NULL) ENGINE=InnoDB",
                "CREATE TABLE item_loot_money (container_id BIGINT UNSIGNED PRIMARY KEY, money INT UNSIGNED NOT NULL) ENGINE=InnoDB",
                "INSERT INTO characters VALUES (1, 100)",
                "INSERT INTO item_loot_money VALUES (71, 10), (72, 10)",
            ] {
                sqlx::query(statement).execute(db.pool()).await?;
            }
            assert!(matches!(
                adapter
                    .attempt_stored_item_money_like_cpp(Request {
                        player_guid: 2,
                        ..other
                    })
                    .await,
                Attempt::DefinitelyRolledBack {
                    kind: StoredItemMoneyRollbackKindLikeCpp::MissingPlayer,
                    ..
                }
            ));
            assert_eq!(
                adapter.attempt_stored_item_money_like_cpp(request).await,
                Attempt::Applied(outcome)
            );
        }
        "recover" => {
            assert_eq!(
                adapter
                    .reconcile_stored_item_money_like_cpp(request, outcome)
                    .await,
                Reconciliation::Committed
            );
            assert!(matches!(
                adapter.attempt_stored_item_money_like_cpp(request).await,
                Attempt::DefinitelyRolledBack {
                    kind: StoredItemMoneyRollbackKindLikeCpp::SourceAlreadyConsumed,
                    ..
                }
            ));
            let next = Outcome {
                before: 110,
                after: 120,
                ..outcome
            };
            assert_eq!(
                adapter
                    .reconcile_stored_item_money_like_cpp(other, next)
                    .await,
                Reconciliation::RolledBack
            );
            assert_eq!(
                adapter.attempt_stored_item_money_like_cpp(other).await,
                Attempt::Applied(next)
            );
            let money: u64 = sqlx::query_scalar("SELECT money FROM characters WHERE guid=1")
                .fetch_one(db.pool())
                .await?;
            let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM item_loot_money")
                .fetch_one(db.pool())
                .await?;
            assert_eq!((money, remaining), (120, 0));
        }
        _ => anyhow::bail!("unexpected test epoch"),
    }
    Ok(())
}

async fn shared_money_epoch(epoch: &str, db: Arc<CharacterDatabase>) -> Result<()> {
    use wow_database::group_loot_money_adapter::MariaDbGroupLootMoneyPersistenceAdapterLikeCpp;
    use wow_persistence::{
        GroupLootMoneyPayoutLikeCpp as Payout, GroupLootMoneyPersistenceAttemptLikeCpp as Attempt,
        GroupLootMoneyPersistenceOutcomeLikeCpp as Outcome, GroupLootMoneyPersistencePortLikeCpp,
        GroupLootMoneyPersistenceRequestLikeCpp as Request,
        GroupLootMoneyReconciliationLikeCpp as Reconciliation,
    };
    let adapter = MariaDbGroupLootMoneyPersistenceAdapterLikeCpp::new(Arc::clone(&db));
    let request = Request {
        claim_id: Some([7; 16]),
        payouts: vec![
            Payout {
                recipient_guid: 101,
                requested_delta: 5,
            },
            Payout {
                recipient_guid: 102,
                requested_delta: 5,
            },
        ],
        max_money: 100,
    };
    let paid = vec![
        Outcome {
            recipient_guid: 101,
            before: 20,
            after: 25,
            applied_delta: 5,
        },
        Outcome {
            recipient_guid: 102,
            before: 99,
            after: 99,
            applied_delta: 0,
        },
    ];
    let mut rolled_back = request.clone();
    rolled_back.claim_id = Some([8; 16]);
    rolled_back.payouts[1].recipient_guid = 103;
    if epoch == "claim" {
        sqlx::raw_sql(include_str!(
            "../../../sql/updates/characters/wotlk_classic/2026_11_22_00_characters.sql"
        ))
        .execute(db.pool())
        .await?;
        sqlx::query("INSERT INTO characters VALUES (101,20),(102,99)")
            .execute(db.pool())
            .await?;
        assert_eq!(
            adapter.attempt_group_loot_money_like_cpp(request).await,
            Attempt::Applied(paid)
        );
        assert!(matches!(
            adapter.attempt_group_loot_money_like_cpp(rolled_back).await,
            Attempt::DefinitelyRolledBack {
                kind: wow_persistence::GroupLootMoneyRollbackKindLikeCpp::MissingPlayer {
                    recipient_guid: 103
                },
                ..
            }
        ));
        // A separate transaction changes the balance after payout. Recovery
        // must still identify the old grant and the aborted transaction.
        sqlx::query("UPDATE characters SET money=20 WHERE guid=101")
            .execute(db.pool())
            .await?;
    } else {
        assert_eq!(
            adapter
                .reconcile_group_loot_money_like_cpp(request.claim_id, paid.clone())
                .await,
            Reconciliation::CommittedOrCapOnlyNoop
        );
        assert_eq!(
            adapter
                .reconcile_group_loot_money_like_cpp(rolled_back.claim_id, paid)
                .await,
            Reconciliation::RolledBack
        );
        assert_eq!(
            adapter
                .attempt_group_loot_money_like_cpp(request.clone())
                .await,
            Attempt::AlreadySettled
        );
        assert_eq!(
            sqlx::query_scalar::<_, u64>("SELECT money FROM characters WHERE guid=101")
                .fetch_one(db.pool())
                .await?,
            20
        );
        // A later, distinct pool remains claimable.
        let next = Request {
            claim_id: Some([9; 16]),
            ..request
        };
        assert!(matches!(
            adapter.attempt_group_loot_money_like_cpp(next).await,
            Attempt::Applied(_)
        ));
        assert_eq!(
            sqlx::query_scalar::<_, u64>("SELECT money FROM characters WHERE guid=101")
                .fetch_one(db.pool())
                .await?,
            25
        );
    }
    Ok(())
}
