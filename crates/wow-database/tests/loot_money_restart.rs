//! Cross-process checks of the production money adapters against disposable InnoDB tables.

use anyhow::{Context, Result, ensure};
use sqlx::{MySqlPool, mysql::MySqlConnectOptions};
use std::{env, process::Command, sync::Arc};
use wow_database::player::lifecycle_adapter::MariaDbPlayerLifecycleAdapterLikeCpp;
use wow_database::stored_item_money_adapter::MariaDbStoredItemMoneyPersistenceAdapterLikeCpp;
use wow_database::{CharacterDatabase, LoginDatabase, WorldDatabase};
use wow_persistence::{
    LootMoneyCriteriaProgress, PlayerAchievementDataLoadLikeCpp, PlayerLifecyclePortLikeCpp,
    PlayerLoginAuxiliaryLoadOutcomeLikeCpp, PlayerLoginAuxiliaryLoadRequestLikeCpp,
    PlayerLoginAuxiliaryLoadedLikeCpp, StoredItemMoneyPersistenceAttemptLikeCpp as Attempt,
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
            base_looted_amount: 0,
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
            base_looted_amount: 0,
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
        let login_db = Arc::new(LoginDatabase::open(&url).await?);
        let world_db = Arc::new(WorldDatabase::open(&url).await?);
        let lifecycle =
            MariaDbPlayerLifecycleAdapterLikeCpp::new(Arc::clone(&db), login_db, world_db);
        if epoch != "after_restock_commit" {
            stored_money_epoch(&epoch, Arc::clone(&db)).await?;
            if epoch == "recover" {
                assert_stored_money_login_projection(&lifecycle).await?;
            }
            shared_money_epoch(&epoch, Arc::clone(&db)).await?;
            if epoch == "recover" {
                assert_shared_money_login_projection(&lifecycle).await?;
            }
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
        criteria_eligible: true,
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
        criteria: None,
    };
    let other = Request {
        item_guid: 72,
        criteria_eligible: false,
        ..request
    };
    let saturated_request = Request {
        criteria_eligible: true,
        player_guid: 3,
        item_guid: 74,
        cached_notified_amount: 1,
        money_bonus_percent: 0,
        max_money: 1_000,
    };
    let saturated_outcome = Outcome {
        before: 1_000,
        after: 1_000,
        applied_delta: 0,
        notified_amount: 1,
        money_mod: 0,
        criteria: None,
    };
    match epoch {
        "claim" => {
            for statement in [
                "CREATE TABLE characters (guid BIGINT UNSIGNED PRIMARY KEY, money BIGINT UNSIGNED NOT NULL) ENGINE=InnoDB",
                "CREATE TABLE item_loot_money (container_id BIGINT UNSIGNED PRIMARY KEY, money INT UNSIGNED NOT NULL) ENGINE=InnoDB",
                "CREATE TABLE character_achievement (guid BIGINT UNSIGNED NOT NULL, achievement INT UNSIGNED NOT NULL, date BIGINT NOT NULL DEFAULT 0, PRIMARY KEY (guid, achievement)) ENGINE=InnoDB",
                "CREATE TABLE character_achievement_progress (guid BIGINT UNSIGNED NOT NULL, criteria INT UNSIGNED NOT NULL, counter BIGINT UNSIGNED NOT NULL, date BIGINT NOT NULL DEFAULT 0, PRIMARY KEY (guid, criteria)) ENGINE=InnoDB",
                "INSERT INTO characters VALUES (1, 100), (2, 999), (3, 1000)",
                "INSERT INTO character_achievement VALUES (1,1234,44)",
                "INSERT INTO item_loot_money VALUES (71, 12), (72, 10), (73, 2), (74, 1)",
                "INSERT INTO character_achievement_progress VALUES (1,4093,999988,10),(2,4093,999998,10),(3,4093,18446744073709551615,123)",
            ] {
                sqlx::query(statement).execute(db.pool()).await?;
            }
            sqlx::query("INSERT INTO character_achievement VALUES (3,1176,1),(3,1177,2),(3,1178,3),(3,1180,4)")
                .execute(db.pool())
                .await?;
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
                sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM character_achievement_progress WHERE guid=999 AND criteria=4093")
                    .fetch_one(db.pool())
                    .await?,
                0,
                "a missing recipient gets no durable criteria credit"
            );
            let Attempt::Applied(applied) =
                adapter.attempt_stored_item_money_like_cpp(request).await
            else {
                anyhow::bail!("eligible stored-item claim should apply")
            };
            assert_eq!(
                (applied.before, applied.after, applied.applied_delta),
                (outcome.before, outcome.after, outcome.applied_delta)
            );
            let item_criteria = applied
                .criteria
                .as_ref()
                .context("eligible stored-item claim should return criterion progress")?;
            assert_eq!(item_criteria.counter, 1_000_000);
            assert!(item_criteria.date > 0);
            assert_eq!(item_criteria.new_achievements, [1176]);
            assert_eq!(item_criteria.achievement_date, Some(item_criteria.date));

            let capped_request = Request {
                criteria_eligible: true,
                player_guid: 2,
                item_guid: 73,
                cached_notified_amount: 1,
                money_bonus_percent: 50,
                max_money: 1000,
            };
            let Attempt::Applied(capped) = adapter
                .attempt_stored_item_money_like_cpp(capped_request)
                .await
            else {
                anyhow::bail!("eligible capped stored-item claim should apply")
            };
            assert_eq!(
                (capped.before, capped.after, capped.applied_delta),
                (999, 999, 0)
            );
            assert_eq!((capped.notified_amount, capped.money_mod), (2, 1));
            let capped_criteria = capped
                .criteria
                .as_ref()
                .context("cap-only payout still advances eligible criteria")?;
            assert_eq!(capped_criteria.counter, 1_000_000);
            assert!(capped_criteria.date > 0);
            assert_eq!(capped_criteria.new_achievements, [1176]);
            assert_eq!(capped_criteria.achievement_date, Some(capped_criteria.date));

            let Attempt::Applied(saturated) = adapter
                .attempt_stored_item_money_like_cpp(saturated_request)
                .await
            else {
                anyhow::bail!("saturated eligible claim should consume its source")
            };
            assert_eq!(
                (saturated.before, saturated.after, saturated.applied_delta),
                (1_000, 1_000, 0)
            );
            let saturated_criteria = saturated
                .criteria
                .as_ref()
                .context("saturated criterion can still complete a missing award")?;
            assert_eq!(saturated_criteria.counter, u64::MAX);
            assert_eq!(saturated_criteria.date, 123);
            assert_eq!(saturated_criteria.new_achievements, [1181]);
            assert!(saturated_criteria.achievement_date.unwrap() > 123);
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT date FROM character_achievement_progress WHERE guid=3 AND criteria=4093"
                )
                .fetch_one(db.pool())
                .await?,
                123,
                "saturated progress retains its previous date"
            );
        }
        "recover" => {
            assert_eq!(
                adapter
                    .reconcile_stored_item_money_like_cpp(request, outcome.clone())
                    .await,
                Reconciliation::Committed
            );
            let capped_request = Request {
                criteria_eligible: true,
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
                criteria: None,
            };
            assert_eq!(
                adapter
                    .reconcile_stored_item_money_like_cpp(capped_request, capped_outcome)
                    .await,
                Reconciliation::Committed,
                "a capped payout still commits the source-row consumption"
            );
            assert_eq!(
                adapter
                    .reconcile_stored_item_money_like_cpp(saturated_request, saturated_outcome)
                    .await,
                Reconciliation::Committed,
                "a capped saturated claim is recovered from source consumption"
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
            assert!(matches!(
                adapter
                    .attempt_stored_item_money_like_cpp(saturated_request)
                    .await,
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
                criteria: None,
                ..outcome
            };
            assert_eq!(
                adapter
                    .reconcile_stored_item_money_like_cpp(other, next.clone())
                    .await,
                Reconciliation::RolledBack
            );
            assert_eq!(
                adapter.attempt_stored_item_money_like_cpp(other).await,
                Attempt::Applied(next.clone())
            );
            let money: u64 = sqlx::query_scalar("SELECT money FROM characters WHERE guid=1")
                .fetch_one(db.pool())
                .await?;
            let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM item_loot_money")
                .fetch_one(db.pool())
                .await?;
            assert_eq!((money, remaining), (133, 0));
            let item_progress: (u64, i64) = sqlx::query_as(
                "SELECT counter, date FROM character_achievement_progress WHERE guid=1 AND criteria=4093",
            )
            .fetch_one(db.pool())
            .await?;
            assert_eq!(item_progress.0, 1_000_000);
            assert!(item_progress.1 > 0);
            assert_eq!(
                item_progress.0,
                sqlx::query_scalar::<_, u64>("SELECT counter FROM character_achievement_progress WHERE guid=2 AND criteria=4093")
                    .fetch_one(db.pool())
                    .await?
            );
            assert_eq!(
                sqlx::query_scalar::<_, u32>("SELECT achievement FROM character_achievement WHERE guid=3 AND achievement=1181")
                    .fetch_one(db.pool())
                    .await?,
                1181
            );
            let saturated_achievement_date: i64 = sqlx::query_scalar(
                "SELECT date FROM character_achievement WHERE guid=3 AND achievement=1181",
            )
            .fetch_one(db.pool())
            .await?;
            assert!(saturated_achievement_date > 123);
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT date FROM character_achievement_progress WHERE guid=3 AND criteria=4093"
                )
                .fetch_one(db.pool())
                .await?,
                123
            );
        }
        _ => anyhow::bail!("unexpected test epoch"),
    }
    Ok(())
}

async fn shared_money_epoch(epoch: &str, db: Arc<CharacterDatabase>) -> Result<()> {
    use wow_database::group_loot_money_adapter::MariaDbGroupLootMoneyPersistenceAdapterLikeCpp;
    use wow_persistence::{
        GroupLootMoneyPayoutLikeCpp as Payout, GroupLootMoneyPersistenceAttemptLikeCpp as Attempt,
        GroupLootMoneyPersistencePortLikeCpp, GroupLootMoneyPersistenceRequestLikeCpp as Request,
        GroupLootMoneyReconciliationLikeCpp as Reconciliation,
    };
    let adapter = MariaDbGroupLootMoneyPersistenceAdapterLikeCpp::new(Arc::clone(&db));
    let request = Request {
        claim_id: Some([7; 16]),
        payouts: vec![
            Payout {
                recipient_guid: 101,
                requested_delta: 5,
                base_looted_amount: 10,
            },
            Payout {
                recipient_guid: 102,
                requested_delta: 5,
                base_looted_amount: 1,
            },
            Payout {
                recipient_guid: 104,
                requested_delta: 5,
                base_looted_amount: 0,
            },
            Payout {
                recipient_guid: 105,
                requested_delta: 1,
                base_looted_amount: 250_000_000,
            },
        ],
        max_money: 100,
    };
    let mut rolled_back = request.clone();
    rolled_back.claim_id = Some([8; 16]);
    rolled_back.payouts[1].recipient_guid = 103;
    if epoch == "claim" {
        sqlx::raw_sql(include_str!(
            "../../../sql/updates/characters/wotlk_classic/2026_11_22_00_characters.sql"
        ))
        .execute(db.pool())
        .await?;
        sqlx::query("INSERT INTO characters VALUES (101,20),(102,99),(105,100)")
            .execute(db.pool())
            .await?;
        sqlx::query("INSERT INTO characters VALUES (104,20)")
            .execute(db.pool())
            .await?;
        sqlx::query("INSERT INTO character_achievement_progress VALUES (101,4093,999990,10),(102,4093,999999,10)")
            .execute(db.pool())
            .await?;
        let unreceipted_criteria_only = Request {
            claim_id: None,
            payouts: vec![Payout {
                recipient_guid: 101,
                requested_delta: 0,
                base_looted_amount: 1,
            }],
            max_money: 100,
        };
        assert!(matches!(
            adapter
                .attempt_group_loot_money_like_cpp(unreceipted_criteria_only)
                .await,
            Attempt::DefinitelyRolledBack {
                kind: wow_persistence::GroupLootMoneyRollbackKindLikeCpp::Database,
                ..
            }
        ));
        assert_eq!(
            sqlx::query_scalar::<_, u64>("SELECT counter FROM character_achievement_progress WHERE guid=101 AND criteria=4093")
                .fetch_one(db.pool())
                .await?,
            999_990,
            "criteria-only claims require a receipt nonce"
        );
        assert_eq!(
            adapter
                .attempt_group_loot_money_like_cpp(request.clone())
                .await,
            Attempt::Applied(shared_paid_outcomes_like_cpp(&db).await?)
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
        assert_eq!(
            sqlx::query_scalar::<_, u64>("SELECT counter FROM character_achievement_progress WHERE guid=101 AND criteria=4093")
                .fetch_one(db.pool())
                .await?,
            1_000_000,
            "a later missing recipient rolls back earlier criteria progress"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM character_achievement WHERE guid=101 AND achievement=1176"
            )
            .fetch_one(db.pool())
            .await?,
            1,
            "a rolled back claim cannot duplicate or partially publish awards"
        );
        // A separate transaction changes the balance after payout. Recovery
        // must still identify the old grant and the aborted transaction.
        sqlx::query("UPDATE characters SET money=20 WHERE guid=101")
            .execute(db.pool())
            .await?;
    } else {
        let paid = shared_paid_outcomes_like_cpp(&db).await?;
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
            sqlx::query_scalar::<_, u64>("SELECT counter FROM character_achievement_progress WHERE guid=101 AND criteria=4093")
                .fetch_one(db.pool())
                .await?,
            1_000_000,
            "a duplicate claim after process restart does not advance the criterion again"
        );
        assert_eq!(
            sqlx::query_scalar::<_, u64>("SELECT counter FROM character_achievement_progress WHERE guid=102 AND criteria=4093")
                .fetch_one(db.pool())
                .await?,
            1_000_000
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM character_achievement_progress WHERE guid=104 AND criteria=4093")
                .fetch_one(db.pool())
                .await?,
            0,
            "ineligible recipients receive no criterion progress"
        );
        assert_eq!(
            sqlx::query_scalar::<_, u64>("SELECT counter FROM character_achievement_progress WHERE guid=105 AND criteria=4093")
                .fetch_one(db.pool())
                .await?,
            250_000_000,
            "a cap-only claim advances every eligible threshold"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM character_achievement WHERE guid=105 AND achievement IN (1176,1177,1178,1180,1181)")
                .fetch_one(db.pool())
                .await?,
            5,
            "large increments complete every crossed threshold in one transaction"
        );
        assert_eq!(
            sqlx::query_scalar::<_, u64>("SELECT money FROM characters WHERE guid=105")
                .fetch_one(db.pool())
                .await?,
            100,
            "criteria advancement does not bypass the all-or-nothing money cap"
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
        let Attempt::Applied(next_outcomes) = adapter.attempt_group_loot_money_like_cpp(next).await
        else {
            anyhow::bail!("a later distinct pool should remain claimable")
        };
        assert!(
            next_outcomes
                .iter()
                .filter_map(|outcome| outcome.criteria.as_ref())
                .all(|criteria| criteria.new_achievements.is_empty())
        );
        assert_eq!(
            sqlx::query_scalar::<_, u64>("SELECT money FROM characters WHERE guid=101")
                .fetch_one(db.pool())
                .await?,
            25
        );
    }
    Ok(())
}

async fn loot_progress_snapshot_like_cpp(
    db: &CharacterDatabase,
    player_guid: u64,
    new_achievements: &[u32],
) -> Result<LootMoneyCriteriaProgress> {
    let (counter, signed_date): (u64, i64) = sqlx::query_as(
        "SELECT counter, date FROM character_achievement_progress WHERE guid=? AND criteria=4093",
    )
    .bind(player_guid)
    .fetch_one(db.pool())
    .await?;
    let date = u64::try_from(signed_date)?;
    let achievement_date = if new_achievements.is_empty() {
        None
    } else {
        let signed_achievement_date: i64 = sqlx::query_scalar(
            "SELECT date FROM character_achievement WHERE guid=? AND achievement=?",
        )
        .bind(player_guid)
        .bind(new_achievements[0])
        .fetch_one(db.pool())
        .await?;
        let achievement_date = u64::try_from(signed_achievement_date)?;
        ensure!(achievement_date == date);
        Some(achievement_date)
    };
    Ok(LootMoneyCriteriaProgress {
        counter,
        date,
        new_achievements: new_achievements.to_vec(),
        achievement_date,
    })
}

async fn shared_paid_outcomes_like_cpp(
    db: &CharacterDatabase,
) -> Result<Vec<wow_persistence::GroupLootMoneyPersistenceOutcomeLikeCpp>> {
    use wow_persistence::GroupLootMoneyPersistenceOutcomeLikeCpp as Outcome;
    Ok(vec![
        Outcome {
            recipient_guid: 101,
            before: 20,
            after: 25,
            applied_delta: 5,
            criteria: Some(loot_progress_snapshot_like_cpp(db, 101, &[1176]).await?),
        },
        Outcome {
            recipient_guid: 102,
            before: 99,
            after: 99,
            applied_delta: 0,
            criteria: Some(loot_progress_snapshot_like_cpp(db, 102, &[1176]).await?),
        },
        Outcome {
            recipient_guid: 104,
            before: 20,
            after: 25,
            applied_delta: 5,
            criteria: None,
        },
        Outcome {
            recipient_guid: 105,
            before: 100,
            after: 100,
            applied_delta: 0,
            criteria: Some(
                loot_progress_snapshot_like_cpp(db, 105, &[1176, 1177, 1178, 1180, 1181]).await?,
            ),
        },
    ])
}

async fn load_achievement_data_like_cpp(
    lifecycle: &MariaDbPlayerLifecycleAdapterLikeCpp,
    player_guid: u64,
) -> Result<PlayerAchievementDataLoadLikeCpp> {
    match lifecycle
        .load_login_auxiliary_like_cpp(PlayerLoginAuxiliaryLoadRequestLikeCpp::AchievementData {
            player_guid,
        })
        .await
    {
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::AchievementData(data),
        ) => Ok(data),
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(_) => {
            anyhow::bail!("achievement-data login request returned the wrong typed result")
        }
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
            anyhow::bail!("achievement-data login read failed: {reason}")
        }
    }
}

fn achievement_date_like_cpp(
    data: &PlayerAchievementDataLoadLikeCpp,
    achievement_id: u32,
) -> Result<u64> {
    data.completed_achievements
        .iter()
        .find(|row| row.achievement_id == achievement_id)
        .map(|row| row.date)
        .with_context(|| format!("achievement {achievement_id} missing from login projection"))
}

async fn assert_stored_money_login_projection(
    lifecycle: &MariaDbPlayerLifecycleAdapterLikeCpp,
) -> Result<()> {
    let player_one = load_achievement_data_like_cpp(lifecycle, 1).await?;
    let item_criteria = player_one
        .loot_money_criteria_progress
        .context("stored-item criterion progress missing at restart")?;
    assert_eq!(item_criteria.criteria_id, 4093);
    assert_eq!(item_criteria.counter, 1_000_000);
    assert!(item_criteria.date > 0);
    assert_eq!(
        achievement_date_like_cpp(&player_one, 1176)?,
        item_criteria.date
    );
    assert_eq!(achievement_date_like_cpp(&player_one, 1234)?, 44);

    let capped_player = load_achievement_data_like_cpp(lifecycle, 2).await?;
    let capped_criteria = capped_player
        .loot_money_criteria_progress
        .context("cap-only stored-item criterion progress missing at restart")?;
    assert_eq!(capped_criteria.counter, 1_000_000);
    assert_eq!(
        achievement_date_like_cpp(&capped_player, 1176)?,
        capped_criteria.date
    );

    let saturated_player = load_achievement_data_like_cpp(lifecycle, 3).await?;
    let saturated_criteria = saturated_player
        .loot_money_criteria_progress
        .context("saturated criterion progress missing at restart")?;
    assert_eq!(saturated_criteria.counter, u64::MAX);
    assert_eq!(saturated_criteria.date, 123);
    assert!(achievement_date_like_cpp(&saturated_player, 1181)? > 123);
    Ok(())
}

async fn assert_shared_money_login_projection(
    lifecycle: &MariaDbPlayerLifecycleAdapterLikeCpp,
) -> Result<()> {
    for (guid, expected_counter) in [(101, 1_000_010), (102, 1_000_001), (105, 500_000_000)] {
        let data = load_achievement_data_like_cpp(lifecycle, guid).await?;
        let criteria = data
            .loot_money_criteria_progress
            .context("shared-loot criterion progress missing at restart")?;
        assert_eq!(criteria.criteria_id, 4093);
        assert_eq!(criteria.counter, expected_counter);
        assert!(criteria.date > 0);
        assert!(achievement_date_like_cpp(&data, 1176)? > 0);
    }
    let ineligible = load_achievement_data_like_cpp(lifecycle, 104).await?;
    assert!(ineligible.loot_money_criteria_progress.is_none());
    assert!(ineligible.completed_achievements.is_empty());
    Ok(())
}
