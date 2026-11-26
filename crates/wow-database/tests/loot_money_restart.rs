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

    // Numeric instance ids can appear under unrelated maps. Retirement is
    // scoped to the complete realm/map/instance identity.
    let old_instance_source = WorldLootMoneySource {
        realm_id: 1,
        map_id: 33,
        instance_id: 77,
        spawn_id: 9106002,
    };
    let other_map_source = WorldLootMoneySource {
        map_id: 34,
        spawn_id: 9106003,
        ..old_instance_source
    };
    let other_realm_source = WorldLootMoneySource {
        realm_id: 2,
        ..old_instance_source
    };
    let old_instance_binding = bind(old_instance_source, 0).await?.unwrap();
    let other_map_binding = bind(other_map_source, 0).await?.unwrap();
    let other_realm_binding = bind(other_realm_source, 0).await?.unwrap();
    sqlx::query("INSERT INTO characters VALUES (202,100)")
        .execute(db.pool())
        .await?;
    let instance_payout = |recipient_guid, claim_id| GroupLootMoneyPersistenceRequestLikeCpp {
        claim_id: Some(claim_id),
        payouts: vec![GroupLootMoneyPayoutLikeCpp {
            recipient_guid,
            requested_delta: 10,
        }],
        max_money: 1000,
    };
    assert_eq!(
        writer
            .execute_mutation_like_cpp(
                RespawnPersistenceMutationLikeCpp::RetireInstanceChestMoney {
                    realm_id: 1,
                    map_id: 33,
                    instance_id: 77,
                }
            )
            .await,
        RespawnPersistenceMutationOutcomeLikeCpp::Applied { affected_rows: 1 }
    );
    assert_eq!(
        writer
            .execute_mutation_like_cpp(
                RespawnPersistenceMutationLikeCpp::RetireInstanceChestMoney {
                    realm_id: 1,
                    map_id: 33,
                    instance_id: 77,
                }
            )
            .await,
        RespawnPersistenceMutationOutcomeLikeCpp::Applied { affected_rows: 0 },
        "retrying a committed retirement is idempotent"
    );
    let generation_while_retired: u64 = sqlx::query_scalar(
        "SELECT generation FROM loot_money_source_head WHERE realm_id=1 AND map_id=33 AND instance_id=77 AND spawn_id=9106002",
    )
    .fetch_one(db.pool())
    .await?;
    assert_eq!(
        generation_while_retired, old_instance_binding.generation,
        "retirement waits to advance the generation until the id is bound again"
    );
    assert_eq!(
        adapter
            .attempt_group_loot_money_like_cpp(instance_payout(202, old_instance_binding.claim_id,))
            .await,
        Attempt::AlreadySettled,
        "retirement rejects the old claim before a replacement binding exists"
    );
    assert_eq!(
        sqlx::query_scalar::<_, u64>("SELECT money FROM characters WHERE guid=202")
            .fetch_one(db.pool())
            .await?,
        100
    );
    assert_eq!(
        bind(other_map_source, 0).await?,
        Some(other_map_binding),
        "retiring map 33 leaves the same numeric instance on map 34 bound"
    );
    assert_eq!(
        bind(other_realm_source, 0).await?,
        Some(other_realm_binding),
        "retiring realm 1 leaves the same map-instance identity in realm 2 bound"
    );
    assert!(matches!(
        adapter
            .attempt_group_loot_money_like_cpp(instance_payout(202, other_map_binding.claim_id,))
            .await,
        Attempt::Applied(_)
    ));
    let reused_instance_binding = bind(old_instance_source, 0).await?.unwrap();
    assert_eq!(
        reused_instance_binding.generation,
        old_instance_binding.generation + 1
    );
    assert!(!reused_instance_binding.spent);
    assert_ne!(
        reused_instance_binding.claim_id,
        old_instance_binding.claim_id
    );
    assert!(matches!(
        adapter
            .attempt_group_loot_money_like_cpp(instance_payout(
                202,
                reused_instance_binding.claim_id,
            ))
            .await,
        Attempt::Applied(_)
    ));
    assert_eq!(
        adapter
            .attempt_group_loot_money_like_cpp(instance_payout(
                202,
                reused_instance_binding.claim_id,
            ))
            .await,
        Attempt::AlreadySettled,
        "the replacement lifetime pays once"
    );

    // Map 609 is a split-by-faction world map. Its team id is nonzero but is
    // not an allocator-owned dungeon, so map-scoped retirement and startup
    // cleanup must leave it intact beside a dungeon using the same number.
    let split_world_source = WorldLootMoneySource {
        realm_id: 1,
        map_id: 609,
        instance_id: 1,
        spawn_id: 9106006,
    };
    let same_number_instance_source = WorldLootMoneySource {
        realm_id: 1,
        map_id: 33,
        instance_id: 1,
        spawn_id: 9106007,
    };
    let split_world_binding = bind(split_world_source, 0).await?.unwrap();
    let same_number_instance_binding = bind(same_number_instance_source, 0).await?.unwrap();
    sqlx::query("INSERT INTO characters VALUES (203,100)")
        .execute(db.pool())
        .await?;
    assert_eq!(
        writer
            .execute_mutation_like_cpp(
                RespawnPersistenceMutationLikeCpp::RetireInstanceChestMoney {
                    realm_id: 1,
                    map_id: 33,
                    instance_id: 1,
                }
            )
            .await,
        RespawnPersistenceMutationOutcomeLikeCpp::Applied { affected_rows: 1 }
    );
    assert_eq!(
        bind(split_world_source, 0).await?,
        Some(split_world_binding),
        "runtime retirement for map 33 does not change map 609's team-1 source"
    );
    assert_eq!(
        adapter
            .attempt_group_loot_money_like_cpp(instance_payout(
                203,
                same_number_instance_binding.claim_id,
            ))
            .await,
        Attempt::AlreadySettled,
        "the old dungeon claim is rejected before a new binding is made"
    );
    let same_number_reused_binding = bind(same_number_instance_source, 0).await?.unwrap();
    assert_eq!(
        same_number_reused_binding.generation,
        same_number_instance_binding.generation + 1
    );
    assert!(matches!(
        adapter
            .attempt_group_loot_money_like_cpp(instance_payout(
                203,
                same_number_reused_binding.claim_id,
            ))
            .await,
        Attempt::Applied(_)
    ));
    assert_eq!(
        adapter
            .attempt_group_loot_money_like_cpp(instance_payout(
                203,
                same_number_reused_binding.claim_id,
            ))
            .await,
        Attempt::AlreadySettled
    );

    let active_instance_source = WorldLootMoneySource {
        realm_id: 1,
        map_id: 40,
        instance_id: 99,
        spawn_id: 9106004,
    };
    let orphaned_instance_source = WorldLootMoneySource {
        map_id: 41,
        instance_id: 100,
        spawn_id: 9106005,
        ..active_instance_source
    };
    let active_before_restart = bind(active_instance_source, 0).await?.unwrap();
    let orphaned_before_restart = bind(orphaned_instance_source, 0).await?.unwrap();
    let chest_adapter = MariaDbGroupLootMoneyPersistenceAdapterLikeCpp::new(Arc::clone(&db));
    let should_retire_startup_pair = |map_id, instance_id| {
        matches!(map_id, 33 | 34 | 40 | 41) && !matches!((map_id, instance_id), (34, 77) | (40, 99))
    };
    chest_adapter
        .retire_unregistered_instance_chest_money_like_cpp(1, should_retire_startup_pair)
        .await?;
    assert_eq!(
        chest_adapter
            .retire_unregistered_instance_chest_money_like_cpp(1, should_retire_startup_pair)
            .await?,
        0,
        "startup retry does not rewrite an already retired generation"
    );
    let orphan_generation_while_retired: u64 = sqlx::query_scalar(
        "SELECT generation FROM loot_money_source_head WHERE realm_id=1 AND map_id=41 AND instance_id=100 AND spawn_id=9106005",
    )
    .fetch_one(db.pool())
    .await?;
    assert_eq!(
        orphan_generation_while_retired, orphaned_before_restart.generation,
        "startup retirement leaves generation advancement to the next bind"
    );
    assert_eq!(
        bind(active_instance_source, 0).await?,
        Some(active_before_restart)
    );
    assert_eq!(
        bind(other_map_source, 0).await?,
        Some(wow_persistence::WorldLootMoneyBinding {
            spent: true,
            ..other_map_binding
        }),
        "startup cleanup preserves the registered exact pair (34, 77)"
    );
    let old_pair_after_restart = bind(old_instance_source, 0).await?.unwrap();
    assert_eq!(
        old_pair_after_restart.generation,
        reused_instance_binding.generation + 1,
        "registered (34, 77) does not protect unregistered (33, 77)"
    );
    assert_eq!(
        bind(split_world_source, 0).await?,
        Some(split_world_binding),
        "startup cleanup skips map 609 even though team id 1 is not registered"
    );
    assert!(matches!(
        adapter
            .attempt_group_loot_money_like_cpp(instance_payout(203, split_world_binding.claim_id,))
            .await,
        Attempt::Applied(_)
    ));
    assert_eq!(
        adapter
            .attempt_group_loot_money_like_cpp(instance_payout(203, split_world_binding.claim_id,))
            .await,
        Attempt::AlreadySettled,
        "the preserved split-world source pays once"
    );
    let orphaned_after_restart = bind(orphaned_instance_source, 0).await?.unwrap();
    assert_eq!(
        orphaned_after_restart.generation,
        orphaned_before_restart.generation + 1
    );
    assert!(!orphaned_after_restart.spent);
    assert_ne!(
        orphaned_after_restart.claim_id,
        orphaned_before_restart.claim_id
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
        money_bonus_percent: 50,
        max_money: 1000,
    };
    let outcome = Outcome {
        before: 100,
        after: 118,
        applied_delta: 18,
        notified_amount: 12,
        money_mod: 6,
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
                "INSERT INTO characters VALUES (1, 100), (2, 999)",
                "INSERT INTO item_loot_money VALUES (71, 12), (72, 10), (73, 2)",
            ] {
                sqlx::query(statement).execute(db.pool()).await?;
            }
            assert!(matches!(
                adapter
                    .attempt_stored_item_money_like_cpp(Request {
                        player_guid: 999,
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
            assert_eq!(
                adapter
                    .attempt_stored_item_money_like_cpp(Request {
                        player_guid: 2,
                        item_guid: 73,
                        cached_notified_amount: 1,
                        money_bonus_percent: 50,
                        max_money: 1000,
                    })
                    .await,
                Attempt::Applied(Outcome {
                    before: 999,
                    after: 999,
                    applied_delta: 0,
                    notified_amount: 2,
                    money_mod: 1,
                }),
                "the base amount plus aura bonus must obey the all-or-nothing money cap"
            );
        }
        "recover" => {
            assert_eq!(
                adapter
                    .reconcile_stored_item_money_like_cpp(request, outcome)
                    .await,
                Reconciliation::Committed
            );
            let capped_request = Request {
                player_guid: 2,
                item_guid: 73,
                cached_notified_amount: 1,
                money_bonus_percent: 50,
                max_money: 1000,
            };
            let capped_outcome = Outcome {
                before: 999,
                after: 999,
                applied_delta: 0,
                notified_amount: 2,
                money_mod: 1,
            };
            assert_eq!(
                adapter
                    .reconcile_stored_item_money_like_cpp(capped_request, capped_outcome)
                    .await,
                Reconciliation::Committed,
                "a capped payout still commits the source-row consumption"
            );
            assert!(matches!(
                adapter
                    .attempt_stored_item_money_like_cpp(capped_request)
                    .await,
                Attempt::DefinitelyRolledBack {
                    kind: StoredItemMoneyRollbackKindLikeCpp::SourceAlreadyConsumed,
                    ..
                }
            ));
            assert!(matches!(
                adapter.attempt_stored_item_money_like_cpp(request).await,
                Attempt::DefinitelyRolledBack {
                    kind: StoredItemMoneyRollbackKindLikeCpp::SourceAlreadyConsumed,
                    ..
                }
            ));
            let next = Outcome {
                before: 118,
                after: 133,
                applied_delta: 15,
                notified_amount: 10,
                money_mod: 5,
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
            assert_eq!((money, remaining), (133, 0));
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
