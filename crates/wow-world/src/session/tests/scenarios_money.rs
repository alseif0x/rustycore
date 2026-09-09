//! Session scenarios exercising the represented money responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn exclusive_money_barrier_waits_and_reconciles_prior_payout_before_derivation() {
    let (mut session, _, _) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 70_002)));
    session.set_player_gold_like_cpp(100);
    let tracker = session.durable_loot_money_persistence_tracker_like_cpp();
    let mut worker = tracker.begin_like_cpp().expect("payout admitted first");
    let applied = Arc::new(AtomicBool::new(false));

    let mut barrier = Box::pin(session.begin_exclusive_player_money_persistence_like_cpp());
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut barrier)
            .await
            .is_err(),
        "the local mutation must wait for a previously admitted payout"
    );
    assert!(
        tracker.begin_like_cpp().is_err(),
        "once the local barrier starts, later payouts must fail admission"
    );

    worker.commit_like_cpp(DurableLootMoneyCompletionLikeCpp {
        durable_money_before: 100,
        durable_money_after: 107,
        durable_applied_amount: 7,
        applied: Arc::clone(&applied),
    });
    let guard = tokio::time::timeout(Duration::from_secs(1), barrier)
        .await
        .expect("barrier must resume after the payout resolves")
        .expect("known payout outcome keeps the session writable");

    assert_eq!(session.player_gold_like_cpp(), 107);
    assert!(applied.load(AtomicOrdering::Acquire));
    assert!(tracker.begin_like_cpp().is_err());
    drop(guard);
    assert!(tracker.begin_like_cpp().is_ok());
}
#[tokio::test]
async fn money_changed_tracking_event_objective_auto_rewards_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let quest_id = 12_507;
    let mut quest = test_quest_template(quest_id);
    quest.flags |= 0x0000_0400; // C++ QUEST_FLAGS_TRACKING_EVENT.
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_MONEY_LIKE_CPP,
        order: 0,
        storage_index: -1,
        object_id: 0,
        amount: 100,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_player_guid(Some(player_guid));
    session.set_player_gold_like_cpp(90);
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![],
            slot: 0,
        },
    );

    adopt_player_quest_fixture_into_canonical_owner_like_cpp(&mut session);
    session.money_changed_like_cpp(100).await;

    assert_canonical_quest_status_like_cpp(&session, quest_id, None, true);
    assert_eq!(
        session.represented_quest_complete_status_updates_like_cpp(),
        &[RepresentedQuestCompleteStatusUpdateLikeCpp {
            quest_id,
            old_status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            new_status: crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
            send_quest_update_called: true,
            quest_slot_state_complete_represented: true,
            quest_slot_state_live_update_unrepresented: true,
            visible_gameobjects_or_spellclicks_refresh_unrepresented: true,
            spell_area_runtime_unrepresented: true,
            tracking_event_auto_reward_unrepresented: false,
            quest_tracker_complete_time_unrepresented: true,
            script_status_change_unrepresented: true,
        }]
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::UpdateObject,
            ServerOpcodes::QuestGiverQuestComplete,
            ServerOpcodes::QuestUpdateComplete,
        ]
    );
}
#[tokio::test]
async fn apply_player_money_change_sets_gold_and_drains_objective_queue_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let quest_id = 12_511;
    let mut quest = test_quest_template(quest_id);
    quest.flags |= 0x0000_0400; // C++ QUEST_FLAGS_TRACKING_EVENT.
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_MONEY_LIKE_CPP,
        order: 0,
        storage_index: -1,
        object_id: 0,
        amount: 100,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_player_guid(Some(player_guid));
    session.set_player_gold_like_cpp(90);
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![],
            slot: 0,
        },
    );

    adopt_player_quest_fixture_into_canonical_owner_like_cpp(&mut session);
    session.apply_player_money_change_like_cpp(90, 100).await;

    assert_eq!(session.player_gold_like_cpp(), 100);
    assert_canonical_quest_status_like_cpp(&session, quest_id, None, true);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::UpdateObject,
            ServerOpcodes::QuestGiverQuestComplete,
            ServerOpcodes::QuestUpdateComplete,
        ]
    );
}
#[tokio::test]
async fn money_changed_loss_marks_complete_money_objective_incomplete_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let quest_id = 12_510;
    let mut quest = test_quest_template(quest_id);
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_MONEY_LIKE_CPP,
        order: 0,
        storage_index: -1,
        object_id: 0,
        amount: 100,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_player_guid(Some(player_guid));
    session.set_player_gold_like_cpp(100);
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![],
            slot: 0,
        },
    );

    adopt_player_quest_fixture_into_canonical_owner_like_cpp(&mut session);
    session.money_changed_like_cpp(50).await;

    assert_canonical_quest_status_like_cpp(
        &session,
        quest_id,
        Some(crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP),
        false,
    );
    assert_eq!(drain_server_opcodes(&send_rx), Vec::<ServerOpcodes>::new());
}
#[tokio::test]
async fn tracking_event_reward_money_drains_money_objective_queue_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    session.set_loot_money_persistence_test_result_like_cpp(true);
    let player_guid = ObjectGuid::create_player(1, 42);
    let reward_quest_id = 12_508;
    let money_objective_quest_id = 12_509;

    let mut reward_quest = test_quest_template(reward_quest_id);
    reward_quest.flags |= 0x0000_0400; // C++ QUEST_FLAGS_TRACKING_EVENT.
    reward_quest.reward_money_difficulty = 100;

    let mut money_objective_quest = test_quest_template(money_objective_quest_id);
    money_objective_quest.flags |= 0x0000_0400; // C++ QUEST_FLAGS_TRACKING_EVENT.
    money_objective_quest
        .objectives
        .push(wow_data::quest::QuestObjective {
            id: money_objective_quest_id * 10,
            quest_id: money_objective_quest_id,
            obj_type: QUEST_OBJECTIVE_MONEY_LIKE_CPP,
            order: 0,
            storage_index: -1,
            object_id: 0,
            amount: 100,
            flags: 0,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        });

    session.set_player_guid(Some(player_guid));
    session.set_player_gold_like_cpp(0);
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [reward_quest.clone(), money_objective_quest],
    )));
    session.player_quests.insert(
        reward_quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id: reward_quest_id,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![],
            slot: 0,
        },
    );
    session.player_quests.insert(
        money_objective_quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id: money_objective_quest_id,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![],
            slot: 1,
        },
    );

    adopt_player_quest_fixture_into_canonical_owner_like_cpp(&mut session);
    assert!(
        session
            .complete_represented_quest_after_add_if_ready_like_cpp(&reward_quest)
            .await
    );

    assert_eq!(session.player_gold_like_cpp(), 100);
    assert_canonical_quest_status_like_cpp(&session, reward_quest_id, None, true);
    assert_canonical_quest_status_like_cpp(&session, money_objective_quest_id, None, true);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::UpdateObject,
            ServerOpcodes::QuestGiverQuestComplete,
            ServerOpcodes::QuestUpdateComplete,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::QuestGiverQuestComplete,
            ServerOpcodes::QuestUpdateComplete,
        ]
    );
}
#[tokio::test]
async fn currency_tracking_event_objective_auto_rewards_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let quest_id = 12_511;
    let currency_id = 395;
    let mut quest = test_quest_template(quest_id);
    quest.flags |= 0x0000_0400; // C++ QUEST_FLAGS_TRACKING_EVENT.
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_CURRENCY_LIKE_CPP,
        order: 0,
        storage_index: -1,
        object_id: currency_id as i32,
        amount: 200,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_currency_types_store(Arc::new(wow_data::CurrencyTypesStore::from_entries([
        currency_entry(currency_id),
    ])));
    session
        .add_currency_quest_reward_like_cpp(
            currency_id,
            100,
            CurrencyGainSourceLikeCpp::QuestReward,
        )
        .expect("currency store")
        .expect("currency gain");
    session.set_player_guid(Some(player_guid));
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![],
            slot: 0,
        },
    );
    let _ = drain_server_opcodes(&send_rx);

    adopt_player_quest_fixture_into_canonical_owner_like_cpp(&mut session);
    session.currency_changed_like_cpp(currency_id, 100).await;

    assert_canonical_quest_status_like_cpp(&session, quest_id, None, true);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SetCurrency,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::QuestGiverQuestComplete,
            ServerOpcodes::QuestUpdateComplete,
        ]
    );
}
#[tokio::test]
async fn have_currency_tracking_event_objective_auto_rewards_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let quest_id = 12_512;
    let currency_id = 395;
    let mut quest = test_quest_template(quest_id);
    quest.flags |= 0x0000_0400; // C++ QUEST_FLAGS_TRACKING_EVENT.
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_HAVE_CURRENCY_LIKE_CPP,
        order: 0,
        storage_index: 0,
        object_id: currency_id as i32,
        amount: 100,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_player_guid(Some(player_guid));
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![0],
            slot: 0,
        },
    );

    adopt_player_quest_fixture_into_canonical_owner_like_cpp(&mut session);
    session.currency_changed_like_cpp(currency_id, 100).await;

    assert_canonical_quest_status_like_cpp(&session, quest_id, None, true);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::QuestUpdateAddCredit,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::QuestGiverQuestComplete,
            ServerOpcodes::QuestUpdateComplete,
        ]
    );
}
#[tokio::test]
async fn obtain_currency_tracking_event_objective_auto_rewards_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let quest_id = 12_513;
    let currency_id = 395;
    let mut quest = test_quest_template(quest_id);
    quest.flags |= 0x0000_0400; // C++ QUEST_FLAGS_TRACKING_EVENT.
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_OBTAIN_CURRENCY_LIKE_CPP,
        order: 0,
        storage_index: 0,
        object_id: currency_id as i32,
        amount: 100,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_player_guid(Some(player_guid));
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![0],
            slot: 0,
        },
    );

    adopt_player_quest_fixture_into_canonical_owner_like_cpp(&mut session);
    session.currency_changed_like_cpp(currency_id, 100).await;

    assert_canonical_quest_status_like_cpp(&session, quest_id, None, true);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::QuestUpdateAddCredit,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::QuestGiverQuestComplete,
            ServerOpcodes::QuestUpdateComplete,
        ]
    );
}
#[tokio::test]
async fn canonical_player_money_follows_active_detached_and_stale_handle_ownership_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_560);
    let position = Position::new(3700.0, 1500.0, 120.0, 0.0);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "MoneyOwner".to_string(),
        position,
        571,
        1,
        1,
        20,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");
    assert!(session.set_player_gold_like_cpp(123));
    assert_eq!(session.resolved_player_money_like_cpp(), Some(123));

    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(session.resolved_player_money_like_cpp(), Some(123));

    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement.set_money(999);
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(session.resolved_player_money_like_cpp(), None);
    assert!(!session.set_player_gold_like_cpp(1));
    session.set_loot_money_persistence_test_result_like_cpp(true);
    assert_eq!(
        session
            .mutate_and_persist_player_gold_exclusive_like_cpp(|money| money + 10)
            .await,
        None
    );
    assert_eq!(session.current_player_save_to_db_snapshot_like_cpp(), None);
    assert!(drain_server_packet_bytes(&send_rx).is_empty());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, Player::money),
        Some(999)
    );
}
#[test]
fn player_currency_helpers_match_cpp_storage_lookup() {
    let (mut session, _, _) = make_session();
    assert_eq!(session.player_currency_quantity(395), Some(0));
    assert!(!session.has_currency(395, 1));

    session.player_currencies.insert(
        395,
        PlayerCurrency {
            state: PlayerCurrencyState::Unchanged,
            quantity: 42,
            weekly_quantity: 5,
            tracked_quantity: 6,
            increased_cap_quantity: 7,
            earned_quantity: 8,
            flags: 9,
        },
    );

    assert_eq!(session.player_currency_quantity(395), Some(42));
    assert!(session.has_currency(395, 42));
    assert!(!session.has_currency(395, 43));
}
#[test]
fn set_currency_flags_preserves_new_state_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_currency_types_store(Arc::new(wow_data::CurrencyTypesStore::from_entries([
        currency_entry(395),
    ])));
    session.player_currencies.insert(
        395,
        PlayerCurrency {
            state: PlayerCurrencyState::New,
            quantity: 1,
            weekly_quantity: 0,
            tracked_quantity: 0,
            increased_cap_quantity: 0,
            earned_quantity: 0,
            flags: 0,
        },
    );

    assert!(session.represented_set_currency_flags_like_cpp(395, 0x04));

    let currency = session.player_currencies.get(&395).unwrap();
    assert_eq!(currency.flags, 0x04);
    assert_eq!(currency.state, PlayerCurrencyState::New);
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&send_rx.try_recv().unwrap()).server_opcode(),
        Some(ServerOpcodes::SetupCurrency)
    );
}
#[test]
fn player_currency_vendor_add_caps_and_marks_state_like_cpp() {
    let (mut session, _, _) = make_session();
    session.player_race = 1;
    session.set_currency_types_store(Arc::new(wow_data::CurrencyTypesStore::from_entries([
        wow_data::CurrencyTypesEntry {
            max_qty: 150,
            max_earnable_per_week: 120,
            flags: wow_constants::CurrencyTypesFlags::TRACK_QUANTITY,
            flags_b: wow_constants::CurrencyTypesFlagsB::USE_TOTAL_EARNED_FOR_EARNED,
            ..currency_entry(395)
        },
        currency_entry(396),
    ])));
    session.player_currencies.insert(
        395,
        PlayerCurrency {
            state: PlayerCurrencyState::Unchanged,
            quantity: 90,
            weekly_quantity: 95,
            tracked_quantity: 4,
            increased_cap_quantity: 0,
            earned_quantity: 7,
            flags: 0,
        },
    );

    let delta = session.add_currency_vendor(395, 70).unwrap().unwrap();
    assert_eq!(delta.currency_id, 395);
    assert_eq!(delta.amount, 25);
    assert_eq!(delta.quantity, 115);
    assert_eq!(delta.weekly_quantity, Some(120));
    assert_eq!(delta.max_quantity, Some(150));
    assert_eq!(delta.total_earned, Some(32));
    assert_eq!(session.player_currency_quantity(395), Some(115));
    assert_eq!(
        session
            .player_currencies
            .get(&395)
            .map(|currency| currency.state),
        Some(PlayerCurrencyState::Changed)
    );

    let delta = session.add_currency_vendor(396, 3).unwrap().unwrap();
    assert_eq!(delta.quantity, 3);
    assert_eq!(
        session
            .player_currencies
            .get(&396)
            .map(|currency| currency.state),
        Some(PlayerCurrencyState::New)
    );
}
#[tokio::test]
async fn loot_money_consumes_only_current_active_loot_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_loot_money_persistence_test_result_like_cpp(true);
    let player_guid = ObjectGuid::create_player(1, 42);
    let active_guid = test_creature_guid(19_001);
    let inactive_guid = test_creature_guid(19_002);
    session.set_player_guid(Some(player_guid));
    session.player_gold = 100;
    session.loot_table.insert(
        active_guid,
        CreatureLoot {
            loot_guid: active_guid,
            coins: 37,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: Vec::new(),
            looted_by_player: false,
        },
    );
    session.loot_table.insert(
        inactive_guid,
        CreatureLoot {
            loot_guid: inactive_guid,
            coins: 91,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: Vec::new(),
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );
    session.set_active_loot_guid(active_guid);

    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(false);
    pkt.flush_bits();
    pkt.reset_read();
    session.handle_loot_money(pkt).await;

    assert_eq!(session.player_gold, 137);
    assert_eq!(session.loot_table.get(&active_guid).unwrap().coins, 0);
    assert_eq!(session.loot_table.get(&inactive_guid).unwrap().coins, 91);

    let coin_removed = send_rx.try_recv().unwrap();
    let mut coin_removed = WorldPacket::from_bytes(&coin_removed);
    assert_eq!(
        coin_removed.read_uint16().unwrap(),
        ServerOpcodes::CoinRemoved as u16
    );
    assert_eq!(coin_removed.read_packed_guid().unwrap(), active_guid);

    let money_notify = send_rx.try_recv().unwrap();
    let mut money_notify = WorldPacket::from_bytes(&money_notify);
    assert_eq!(
        money_notify.read_uint16().unwrap(),
        ServerOpcodes::LootMoneyNotify as u16
    );
    assert_eq!(money_notify.read_uint64().unwrap(), 37);
    assert!(send_rx.try_recv().is_err());
}
