// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player persistence operation scenarios.

use super::*;
#[tokio::test]
async fn uncage_item_state_read_routes_typed_ids_and_preserves_failure_distinction() {
    let (session, port) = session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 0 });
    port.set_uncage_item_state_outcome(PlayerUncageItemStateLoadOutcomeLikeCpp::Loaded(
        PlayerUncageItemStateLikeCpp {
            owner_guid: Some(42),
            inventory_linked: true,
        },
    ));

    assert_eq!(
        wow_world_inventory::uncage_item_state_like_cpp(&session, 42, 71).await,
        PlayerUncageItemStateLoadOutcomeLikeCpp::Loaded(PlayerUncageItemStateLikeCpp {
            owner_guid: Some(42),
            inventory_linked: true,
        })
    );
    assert_eq!(
        port.uncage_item_state_requests(),
        vec![PlayerUncageItemStateRequestLikeCpp {
            player_guid: 42,
            item_guid: 71,
        }]
    );

    let (session_without_port, _, _) = make_session();
    assert!(matches!(
        wow_world_inventory::uncage_item_state_like_cpp(&session_without_port, 42, 71).await,
        PlayerUncageItemStateLoadOutcomeLikeCpp::Failed { .. }
    ));
}

#[test]
fn uncage_item_state_seam_no_longer_names_statement_or_driver_errors() {
    // #597 and #1363 slice 12b1b moved the uncage seam (now in
    // `wow-world-inventory`); its contract is unchanged, so the scan follows the code.
    let session_source =
        include_str!("../../../../../wow-world-inventory/src/battle_pet_session/item_state.rs");
    let item_source = include_str!("../../../../src/handlers/character/items.rs");
    let (_, helper_and_tail) = session_source
        .split_once("pub async fn uncage_item_state_like_cpp")
        .expect("uncage state helper starts");
    let (helper, _) = helper_and_tail
        // Ends at the helper's own top-level `}`, whatever item follows it.
        .split_once("\n}\n")
        .expect("uncage state helper ends at its own closing brace");

    assert!(helper.contains("load_uncage_item_state_like_cpp"));
    for concrete in ["CharStatements", "DatabaseError", "SEL_UNCAGE_ITEM_STATE"] {
        assert!(
            !helper.contains(concrete),
            "Session uncage seam still names concrete persistence vocabulary {concrete}"
        );
    }
    assert!(!item_source.contains("SEL_UNCAGE_ITEM_STATE"));
}

#[tokio::test]
async fn exclusive_money_mutation_reaches_the_sqlx_free_lifecycle_port_before_publication() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    session.set_player_guid(Some(ObjectGuid::create_player(1, 0x7400_0101)));
    session.set_player_gold_like_cpp(100);

    assert_eq!(
        session
            .mutate_and_persist_player_gold_exclusive_like_cpp(|money| money + 25)
            .await,
        Some((100, 125))
    );
    assert_eq!(session.player_gold_like_cpp(), 125);
    assert_eq!(
        port.money_transactions(),
        vec![PlayerMoneyTransactionRequestLikeCpp {
            player_guid: 0x7400_0101,
            money_after: 125,
            durability_repairs: Vec::new(),
        }]
    );
}

#[tokio::test]
async fn rolled_back_money_mutation_does_not_publish_runtime_state() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Failed {
        reason: "forced rollback".to_owned(),
    });
    session.set_player_guid(Some(ObjectGuid::create_player(1, 0x7400_0102)));
    session.set_player_gold_like_cpp(100);

    assert_eq!(
        session
            .mutate_and_persist_player_gold_exclusive_like_cpp(|money| money + 25)
            .await,
        None
    );
    assert_eq!(session.player_gold_like_cpp(), 100);
    assert_eq!(port.money_transactions().len(), 1);
}

#[tokio::test]
async fn standalone_durability_repair_persists_before_runtime_publication_like_cpp() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 0 });
    let player_guid = ObjectGuid::create_player(1, 0x7400_0104);
    let item_guid = ObjectGuid::create_item(1, 0x7400_0105);
    session.set_player_guid(Some(player_guid));
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: item_guid,
            entry_id: 700,
            db_guid: item_guid.counter() as u64,
            inventory_type: None,
        },
    );
    let mut item = session.make_inventory_item_object(
        item_guid,
        700,
        player_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    item.set_max_durability(50);
    item.set_durability(10);
    session.insert_inventory_item_object(item);

    assert!(
        session
            .repair_inventory_item_durability_like_cpp(item_guid, false, 0.0, 1.0)
            .await
    );
    assert_eq!(
        port.durability_repairs(),
        vec![PlayerDurabilityRepairSaveLikeCpp {
            item_db_guid: item_guid.counter() as u64,
            durability: 50,
        }]
    );
    assert_eq!(
        session.inventory_item_objects_like_cpp()[&item_guid]
            .data()
            .durability,
        50,
        "an applied write publishes the repaired runtime durability even when SQL reports zero changed rows"
    );

    session
        .inventory
        .inventory_item_object_mut_for_test_like_cpp(&item_guid)
        .unwrap()
        .set_durability(10);
    let failed_port = RecordingPortLikeCpp::new(PersistenceOutcomeLikeCpp::Failed {
        reason: "forced durability write failure".to_owned(),
    });
    session.set_player_lifecycle_port_like_cpp(failed_port.clone());

    assert!(
        !session
            .repair_inventory_item_durability_like_cpp(item_guid, false, 0.0, 1.0)
            .await
    );
    assert_eq!(failed_port.durability_repairs().len(), 1);
    assert_eq!(
        session.inventory_item_objects_like_cpp()[&item_guid]
            .data()
            .durability,
        10,
        "a failed durable write cannot publish the repair only in memory"
    );
}

#[tokio::test]
async fn checked_money_write_uses_the_nontransactional_lifecycle_port_contract() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    session.set_player_guid(Some(ObjectGuid::create_player(1, 0x7400_0103)));

    assert!(
        session
            .persist_player_gold_checked_like_cpp(777)
            .await
            .is_ok()
    );
    assert_eq!(
        port.money_writes(),
        vec![PlayerMoneyWriteRequestLikeCpp {
            player_guid: 0x7400_0103,
            money: 777,
        }]
    );
}

#[tokio::test]
async fn quest_currency_save_reaches_the_sqlx_free_port_before_publication_like_cpp() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    session.set_player_guid(Some(ObjectGuid::create_player(1, 0x7400_0201)));
    session.fixtures.identity.player_race = 1;
    session.set_currency_types_store(Arc::new(CurrencyTypesStore::from_entries([
        currency_entry(395),
    ])));

    let snapshot = session.player_currencies_like_cpp().unwrap();
    assert!(
        session
            .add_currency_quest_reward_like_cpp(395, 7, CurrencyGainSourceLikeCpp::QuestReward,)
            .is_ok()
    );
    assert!(
        session
            .persist_standalone_player_currency_save_like_cpp(0x7400_0201, snapshot)
            .await
            .is_ok()
    );
    assert_eq!(session.player_currency_quantity(395), Some(7));
    let requests = port.currency_saves();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].player_guid, 0x7400_0201);
    assert_eq!(requests[0].rows.len(), 1);
    assert_eq!(
        requests[0].rows[0].kind,
        wow_persistence::PlayerCurrencySaveKindLikeCpp::New
    );
    assert_eq!(requests[0].rows[0].currency_id, 395);
    assert_eq!(requests[0].rows[0].quantity, 7);
    assert_eq!(
        session
            .player_currencies_like_cpp()
            .unwrap()
            .get(&395)
            .map(|currency| currency.state),
        Some(PlayerCurrencyState::Unchanged)
    );
}

#[tokio::test]
async fn missing_currency_persistence_port_keeps_the_existing_unsaved_state_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 0x7400_0203)));
    session.fixtures.identity.player_race = 1;
    session.set_currency_types_store(Arc::new(CurrencyTypesStore::from_entries([
        currency_entry(395),
    ])));

    let snapshot = session.player_currencies_like_cpp().unwrap();
    assert!(
        session
            .add_currency_quest_reward_like_cpp(395, 7, CurrencyGainSourceLikeCpp::QuestReward,)
            .is_ok()
    );
    assert!(
        session
            .persist_standalone_player_currency_save_like_cpp(0x7400_0203, snapshot)
            .await
            .is_ok()
    );
    assert_eq!(session.player_currency_quantity(395), Some(7));
    assert_eq!(
        session
            .player_currencies_like_cpp()
            .unwrap()
            .get(&395)
            .map(|currency| currency.state),
        Some(PlayerCurrencyState::New)
    );
}

#[tokio::test]
async fn unknown_quest_currency_commit_restores_the_pre_save_snapshot_like_cpp() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Unknown {
        reason: "lost COMMIT reply".to_owned(),
    });
    session.set_player_guid(Some(ObjectGuid::create_player(1, 0x7400_0202)));
    session.fixtures.identity.player_race = 1;
    session.set_currency_types_store(Arc::new(CurrencyTypesStore::from_entries([
        currency_entry(395),
    ])));

    let snapshot = session.player_currencies_like_cpp().unwrap();
    assert!(
        session
            .add_currency_quest_reward_like_cpp(395, 7, CurrencyGainSourceLikeCpp::QuestReward,)
            .is_ok()
    );
    assert!(
        session
            .persist_standalone_player_currency_save_like_cpp(0x7400_0202, snapshot)
            .await
            .is_err()
    );
    assert_eq!(session.player_currency_quantity(395), Some(0));
    assert_eq!(port.currency_saves().len(), 1);
}

#[tokio::test]
async fn talent_reset_reaches_the_sqlx_free_port_before_runtime_publication_like_cpp() {
    let (mut session, port) =
        talent_reset_session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 3 }, 0x7500_0201);

    let committed = session
        .commit_represented_talent_reset_at_like_cpp(123, false)
        .await
        .expect("an applied adapter outcome should return the unpublished runtime plan");

    assert_eq!(
        port.talent_resets(),
        vec![PlayerTalentResetPersistenceRequestLikeCpp {
            player_guid: 0x7500_0201,
            money_before: 100_000,
            money_after: 90_000,
            reset_cost: 10_000,
            reset_time_secs: 123,
            retained_talents: Vec::new(),
        }]
    );
    assert_eq!(session.player_gold_like_cpp(), 100_000);
    drop(committed);
}

#[tokio::test]
async fn definite_talent_reset_rollback_does_not_publish_runtime_state_like_cpp() {
    let (mut session, port) = talent_reset_session_with_port(
        PersistenceOutcomeLikeCpp::Failed {
            reason: "constraint failure before COMMIT".to_owned(),
        },
        0x7500_0202,
    );

    assert!(
        session
            .commit_represented_talent_reset_at_like_cpp(123, false)
            .await
            .is_none()
    );
    assert_eq!(port.talent_resets().len(), 1);
    assert_eq!(session.player_gold_like_cpp(), 100_000);
    assert!(
        !session
            .durable_loot_money_persistence_tracker_like_cpp()
            .is_indeterminate_like_cpp()
    );
}

#[tokio::test]
async fn talent_reset_uses_each_supplied_cost_policy_without_caching_it() {
    let (mut session, port) =
        talent_reset_session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 3 }, 0x7500_0204);
    for (free, expected_money, expected_cost) in [(true, 100_000, 0), (false, 90_000, 10_000)] {
        let committed = session
            .commit_represented_talent_reset_at_like_cpp(123, free)
            .await
            .expect("supplied policy must reach the persistence plan");
        let requests = port.talent_resets();
        let request = requests.last().unwrap();
        assert_eq!(request.money_before, 100_000);
        assert_eq!(request.money_after, expected_money);
        assert_eq!(request.reset_cost, expected_cost);
        assert_eq!(request.reset_time_secs, 123);
        assert_eq!(
            session.player_gold_like_cpp(),
            100_000,
            "publication stays separate"
        );
        drop(committed);
    }
    assert_eq!(port.talent_resets().len(), 2);
}

#[tokio::test]
async fn unknown_talent_reset_commit_quarantines_without_publication_like_cpp() {
    let (mut session, port) = talent_reset_session_with_port(
        PersistenceOutcomeLikeCpp::Unknown {
            reason: "connection lost after COMMIT".to_owned(),
        },
        0x7500_0203,
    );

    assert!(
        session
            .commit_represented_talent_reset_at_like_cpp(123, false)
            .await
            .is_none()
    );
    assert_eq!(port.talent_resets().len(), 1);
    assert_eq!(session.player_gold_like_cpp(), 100_000);
    assert!(
        session
            .durable_loot_money_persistence_tracker_like_cpp()
            .is_indeterminate_like_cpp()
    );
}

#[tokio::test]
async fn represented_xp_reaches_the_port_for_every_classified_outcome_like_cpp() {
    for outcome in [
        PersistenceOutcomeLikeCpp::Applied { rows: 2 },
        PersistenceOutcomeLikeCpp::Failed {
            reason: "rolled back".to_owned(),
        },
        PersistenceOutcomeLikeCpp::Unknown {
            reason: "commit reply lost".to_owned(),
        },
    ] {
        let (mut session, port) = session_with_port(outcome);
        let guid = ObjectGuid::create_player(1, 0x7500_0301);
        let victim = test_creature_guid(0x7500_0302);
        session.set_player_guid(Some(guid));
        session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
        crate::session::hub_mut(&mut session).set_player_next_level_xp_like_cpp(1_000);
        session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_RESTED_LIKE_CPP, 70.0);
        install_tapped_xp_victim_like_cpp(&mut session, victim);

        session.give_xp(50, victim, 1.0).await;

        let requests = port.xp_saves();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].player_guid, guid.counter() as u64);
        assert_eq!(requests[0].xp, 100);
        assert_eq!(
            requests[0]
                .rest
                .expect("rest consumption accompanies XP")
                .rest_bonus,
            20.0
        );
        assert_eq!(session.player_xp_like_cpp(), 100);
    }
}

/// Records each `player.level_changed` with the XP saves already requested.
struct LevelProbe {
    port: Arc<RecordingPortLikeCpp>,
    calls: Mutex<Vec<(u8, u8, usize)>>,
}

impl wow_module_api::PlayerModule for LevelProbe {
    fn on_level_changed(
        &self,
        snapshot: &wow_module_api::PlayerLevelChangedSnapshot,
        effects: &mut wow_module_api::ScopedEffects<'_>,
    ) {
        self.calls.lock().unwrap().push((
            snapshot.old_level,
            snapshot.new_level,
            self.port.xp_saves().len(),
        ));
        effects.send_system_message_self(format!("Level {}!", snapshot.new_level));
    }
}

fn install_level_probe(
    session: &mut WorldSession,
    port: &Arc<RecordingPortLikeCpp>,
) -> Arc<LevelProbe> {
    let probe = Arc::new(LevelProbe {
        port: Arc::clone(port),
        calls: Mutex::new(Vec::new()),
    });
    let mut registry = wow_module_api::ModuleRegistry::new();
    registry
        .register_module(
            wow_module_api::ModuleDescriptor::new(
                "alpha.level",
                wow_module_api::ModuleVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                "Level",
            )
            .unwrap(),
            wow_module_api::ModuleHooks::new().player(Arc::clone(&probe) as _),
        )
        .unwrap();
    session.install_module_hooks_like_cpp(Arc::new(registry));
    probe
}

fn level_session(
    outcome: PersistenceOutcomeLikeCpp,
    counter: i64,
) -> (
    WorldSession,
    Arc<RecordingPortLikeCpp>,
    Arc<LevelProbe>,
    ObjectGuid,
) {
    let (mut session, port) = session_with_port(outcome);
    let probe = install_level_probe(&mut session, &port);
    let guid = ObjectGuid::create_player(1, counter);
    let victim = test_creature_guid(counter + 1);
    session.set_player_guid(Some(guid));
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    install_tapped_xp_victim_like_cpp(&mut session, victim);
    (session, port, probe, victim)
}

/// `player.level_changed` (#1444 H1): AzerothCore `GiveLevel` ->
/// `OnPlayerLevelChanged` (`Player.cpp:2585`). Once per level-up, after the XP
/// save applied; XP that does not change the level raises nothing.
#[tokio::test]
async fn level_changed_hook_runs_once_after_the_xp_save_like_cpp() {
    let (mut session, port, probe, victim) =
        level_session(PersistenceOutcomeLikeCpp::Applied { rows: 2 }, 0x7500_0311);
    crate::session::hub_mut(&mut session).set_player_next_level_xp_like_cpp(1_000);

    session.give_xp(50, victim, 1.0).await;
    assert!(
        probe.calls.lock().unwrap().is_empty(),
        "XP below the next level raises no level hook"
    );

    crate::session::hub_mut(&mut session).set_player_next_level_xp_like_cpp(60);
    session.give_xp(50, victim, 1.0).await;
    assert_eq!(
        *probe.calls.lock().unwrap(),
        vec![(10, 11, port.xp_saves().len())],
        "one level-up, one hook, after its XP save was requested"
    );
}

/// A failed or unknown XP save is not a durable level: no hook (#1444 review
/// F1). The level was still published to the client, as before.
#[tokio::test]
async fn an_unsaved_level_up_raises_no_level_changed_hook_like_cpp() {
    for outcome in [
        PersistenceOutcomeLikeCpp::Failed {
            reason: "rolled back".to_owned(),
        },
        PersistenceOutcomeLikeCpp::Unknown {
            reason: "commit reply lost".to_owned(),
        },
    ] {
        let (mut session, port, probe, victim) = level_session(outcome, 0x7500_0321);
        crate::session::hub_mut(&mut session).set_player_next_level_xp_like_cpp(60);
        session.give_xp(100, victim, 1.0).await;
        assert_eq!(port.xp_saves().len(), 1, "the level-up attempted its save");
        assert!(probe.calls.lock().unwrap().is_empty());
    }
}

/// TrinityCore `GiveXP` calls `GiveLevel` once per level gained
/// (`Player.cpp:2231-2236`), so a multi-level grant reports every step in
/// order (#1444 review F2).
#[tokio::test]
async fn a_multi_level_grant_reports_every_level_step_like_cpp() {
    let (mut session, _port, probe, victim) =
        level_session(PersistenceOutcomeLikeCpp::Applied { rows: 2 }, 0x7500_0331);
    crate::session::hub_mut(&mut session).set_player_next_level_xp_like_cpp(10);
    session.give_xp(1_000_000, victim, 1.0).await;
    let calls = probe.calls.lock().unwrap().clone();
    assert!(
        calls.len() >= 2,
        "a large grant gains several levels: {calls:?}"
    );
    for (index, (old, new, _)) in calls.iter().enumerate() {
        assert_eq!(*old, 10 + index as u8);
        assert_eq!(*new, old + 1);
    }
}
