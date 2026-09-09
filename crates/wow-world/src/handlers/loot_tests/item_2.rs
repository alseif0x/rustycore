//! Item scenarios for [`super`].
//!
//! Split out of loot_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn failed_item_persistence_publishes_no_removal_or_release_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 61_840);
    let owner_guid = ObjectGuid::create_item(1, 61_841);
    install_active_item_loot_completion_fixture_like_cpp(&mut session, player_guid, owner_guid, 0);
    let guard = session.begin_durable_item_loot_persistence_like_cpp();
    let result = super::super::spawn_loot_claim_persistence_worker_like_cpp(
        async { Err::<(), ()>(()) },
        None,
        Some((
            guard,
            DurableItemLootCompletionLikeCpp {
                owner_guid,
                loot_list_id: 0,
                player_guid,
                item_owner_auto_release: true,
                durable_item_money_applied_amount: None,
                durable_item_money_notified_amount: None,
                durable_item_money_balance_applied: None,
                item_fanout: None,
                runtime_inventory_applied: Arc::new(AtomicBool::new(false)),
            },
        )),
    )
    .unwrap()
    .await
    .unwrap();
    assert!(result.is_err());

    session.wait_for_active_loot_persistence_like_cpp().await;

    assert!(!session.is_disconnecting());
    assert!(session.is_active_loot_guid(owner_guid));
    assert!(!session.loot_table.get(&owner_guid).unwrap().items[0].taken);
    assert!(
        !drain_server_opcodes_like_cpp(&send_rx)
            .contains(&(wow_constants::ServerOpcodes::LootRelease as u16))
    );
}
#[tokio::test]
async fn cancelled_stored_item_money_before_commit_retries_without_local_consumption_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 61_845);
    let owner_guid = ObjectGuid::create_item(1, 61_846);
    install_active_item_loot_completion_fixture_like_cpp(&mut session, player_guid, owner_guid, 7);
    session.set_player_gold_like_cpp(100);

    let durable_source_row = Arc::new(AtomicBool::new(true));
    let first_source = Arc::clone(&durable_source_row);
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (_commit_tx, commit_rx) = tokio::sync::oneshot::channel::<()>();
    let first_balance_applied = Arc::new(AtomicBool::new(false));
    let first_runtime_applied = Arc::new(AtomicBool::new(false));
    let first_guard = session.begin_durable_item_loot_persistence_like_cpp();
    let first_worker = super::super::spawn_loot_claim_persistence_worker_like_cpp(
        async move {
            let _ = started_tx.send(());
            let _ = commit_rx.await;
            first_source
                .compare_exchange(true, false, Ordering::AcqRel, Ordering::Acquire)
                .map(|_| ())
                .map_err(|_| ())
        },
        None,
        Some((
            first_guard,
            DurableItemLootCompletionLikeCpp {
                owner_guid,
                loot_list_id: 0,
                player_guid,
                item_owner_auto_release: false,
                durable_item_money_applied_amount: Some(7),
                durable_item_money_notified_amount: Some(7),
                durable_item_money_balance_applied: Some(Arc::clone(&first_balance_applied)),
                item_fanout: None,
                runtime_inventory_applied: Arc::clone(&first_runtime_applied),
            },
        )),
    )
    .unwrap();
    started_rx.await.unwrap();
    first_worker.abort();
    assert!(first_worker.await.unwrap_err().is_cancelled());

    session.wait_for_active_loot_persistence_like_cpp().await;
    assert!(durable_source_row.load(Ordering::Acquire));
    assert_eq!(session.player_gold_like_cpp(), 100);
    assert_eq!(session.loot_table.get(&owner_guid).unwrap().coins, 7);
    assert!(!first_runtime_applied.load(Ordering::Acquire));

    let retry_source = Arc::clone(&durable_source_row);
    let retry_balance_applied = Arc::new(AtomicBool::new(false));
    let retry_runtime_applied = Arc::new(AtomicBool::new(false));
    let retry_guard = session.begin_durable_item_loot_persistence_like_cpp();
    super::super::spawn_loot_claim_persistence_worker_like_cpp(
        async move {
            retry_source
                .compare_exchange(true, false, Ordering::AcqRel, Ordering::Acquire)
                .map(|_| ())
                .map_err(|_| ())
        },
        None,
        Some((
            retry_guard,
            DurableItemLootCompletionLikeCpp {
                owner_guid,
                loot_list_id: 0,
                player_guid,
                item_owner_auto_release: false,
                durable_item_money_applied_amount: Some(7),
                durable_item_money_notified_amount: Some(7),
                durable_item_money_balance_applied: Some(Arc::clone(&retry_balance_applied)),
                item_fanout: None,
                runtime_inventory_applied: Arc::clone(&retry_runtime_applied),
            },
        )),
    )
    .unwrap()
    .await
    .unwrap()
    .unwrap();

    session.wait_for_active_loot_persistence_like_cpp().await;
    assert!(!durable_source_row.load(Ordering::Acquire));
    assert_eq!(session.player_gold_like_cpp(), 107);
    assert_eq!(session.loot_table.get(&owner_guid).unwrap().coins, 0);
    assert!(retry_runtime_applied.load(Ordering::Acquire));
    assert!(session.is_active_loot_guid(owner_guid));
}
#[tokio::test]
async fn cancelled_stored_item_money_after_commit_is_replayed_before_disconnect_save_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 61_847);
    let owner_guid = ObjectGuid::create_item(1, 61_848);
    install_active_item_loot_completion_fixture_like_cpp(&mut session, player_guid, owner_guid, 7);
    session.set_player_gold_like_cpp(100);

    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (commit_tx, commit_rx) = tokio::sync::oneshot::channel();
    let balance_applied = Arc::new(AtomicBool::new(false));
    let runtime_money_applied = Arc::new(AtomicBool::new(false));
    let guard = session.begin_durable_item_loot_persistence_like_cpp();
    let worker = super::super::spawn_loot_claim_persistence_worker_like_cpp(
        async move {
            let _ = started_tx.send(());
            let _ = commit_rx.await;
            Ok::<(), ()>(())
        },
        None,
        Some((
            guard,
            DurableItemLootCompletionLikeCpp {
                owner_guid,
                loot_list_id: 0,
                player_guid,
                item_owner_auto_release: false,
                durable_item_money_applied_amount: Some(7),
                durable_item_money_notified_amount: Some(7),
                durable_item_money_balance_applied: Some(Arc::clone(&balance_applied)),
                item_fanout: None,
                runtime_inventory_applied: Arc::clone(&runtime_money_applied),
            },
        )),
    )
    .unwrap();
    let waiter = tokio::spawn(async move { worker.await });
    started_rx.await.unwrap();
    waiter.abort();
    let _ = waiter.await;

    let mut save = Box::pin(session.save_disconnect_player_to_db_like_cpp());
    assert!(
        tokio::time::timeout(Duration::from_millis(5), &mut save)
            .await
            .is_err(),
        "disconnect save must remain pending until durable loot publication is idle"
    );
    commit_tx.send(()).unwrap();
    save.await;

    assert_eq!(session.player_gold_like_cpp(), 107);
    assert!(runtime_money_applied.load(Ordering::Acquire));
}
#[tokio::test]
async fn stored_item_money_save_reconciled_balance_still_publishes_source_once_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 61_848);
    let owner_guid = ObjectGuid::create_item(1, 61_849);
    install_active_item_loot_completion_fixture_like_cpp(&mut session, player_guid, owner_guid, 7);
    session.set_player_gold_like_cpp(107);
    let balance_applied = Arc::new(AtomicBool::new(true));
    let publication_applied = Arc::new(AtomicBool::new(false));
    let mut guard = session.begin_durable_item_loot_persistence_like_cpp();
    guard.mark_committed_like_cpp(DurableItemLootCompletionLikeCpp {
        owner_guid,
        loot_list_id: 0,
        player_guid,
        item_owner_auto_release: false,
        durable_item_money_applied_amount: Some(7),
        durable_item_money_notified_amount: Some(7),
        durable_item_money_balance_applied: Some(Arc::clone(&balance_applied)),
        item_fanout: None,
        runtime_inventory_applied: Arc::clone(&publication_applied),
    });
    drop(guard);

    session
        .apply_pending_durable_item_loot_completions_like_cpp()
        .await;

    assert_eq!(session.player_gold_like_cpp(), 107);
    assert_eq!(session.loot_table.get(&owner_guid).unwrap().coins, 0);
    assert!(balance_applied.load(Ordering::Acquire));
    assert!(publication_applied.load(Ordering::Acquire));
    assert!(
        drain_server_opcodes_like_cpp(&send_rx)
            .contains(&(wow_constants::ServerOpcodes::LootMoneyNotify as u16))
    );
}
#[tokio::test]
async fn stored_item_money_delete_cas_allows_exactly_one_durable_grant_like_cpp() {
    assert_eq!(
        super::super::STORED_ITEM_MONEY_SOURCE_ROWS_EXPECTED_LIKE_CPP,
        1,
        "the source-row delete must fail the whole transaction after another winner"
    );
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 61_849);
    let owner_guid = ObjectGuid::create_item(1, 61_850);
    install_active_item_loot_completion_fixture_like_cpp(&mut session, player_guid, owner_guid, 7);
    session.set_player_gold_like_cpp(100);

    let source_row = Arc::new(AtomicBool::new(true));
    let durable_grants = Arc::new(AtomicUsize::new(0));
    let mut workers = Vec::new();
    for _ in 0..2 {
        let source_row = Arc::clone(&source_row);
        let durable_grants = Arc::clone(&durable_grants);
        let guard = session.begin_durable_item_loot_persistence_like_cpp();
        workers.push(
            super::super::spawn_loot_claim_persistence_worker_like_cpp(
                async move {
                    tokio::task::yield_now().await;
                    source_row
                        .compare_exchange(true, false, Ordering::AcqRel, Ordering::Acquire)
                        .map_err(|_| ())?;
                    durable_grants.fetch_add(1, Ordering::SeqCst);
                    Ok::<(), ()>(())
                },
                None,
                Some((
                    guard,
                    DurableItemLootCompletionLikeCpp {
                        owner_guid,
                        loot_list_id: 0,
                        player_guid,
                        item_owner_auto_release: false,
                        durable_item_money_applied_amount: Some(7),
                        durable_item_money_notified_amount: Some(7),
                        durable_item_money_balance_applied: Some(Arc::new(AtomicBool::new(false))),
                        item_fanout: None,
                        runtime_inventory_applied: Arc::new(AtomicBool::new(false)),
                    },
                )),
            )
            .unwrap(),
        );
    }

    let mut successes = 0;
    for worker in workers {
        if matches!(worker.await, Ok(Ok(()))) {
            successes += 1;
        }
    }
    session.wait_for_active_loot_persistence_like_cpp().await;

    assert_eq!(successes, 1);
    assert_eq!(durable_grants.load(Ordering::SeqCst), 1);
    assert_eq!(session.player_gold_like_cpp(), 107);
    assert_eq!(session.loot_table.get(&owner_guid).unwrap().coins, 0);
    assert_eq!(
        drain_server_opcodes_like_cpp(&send_rx)
            .into_iter()
            .filter(|opcode| { *opcode == wow_constants::ServerOpcodes::LootMoneyNotify as u16 })
            .count(),
        1
    );
}
#[tokio::test]
async fn cancelled_zero_stored_item_money_notifies_once_and_normal_completion_does_not_replay_like_cpp()
 {
    for cancelled_waiter in [true, false] {
        let (mut session, send_rx) = make_session_with_send_capacity(16);
        let player_guid =
            ObjectGuid::create_player(1, if cancelled_waiter { 61_851 } else { 61_852 });
        let owner_guid = ObjectGuid::create_item(1, if cancelled_waiter { 61_853 } else { 61_854 });
        install_active_item_loot_completion_fixture_like_cpp(
            &mut session,
            player_guid,
            owner_guid,
            0,
        );

        if cancelled_waiter {
            let (started_tx, started_rx) = tokio::sync::oneshot::channel();
            let (commit_tx, commit_rx) = tokio::sync::oneshot::channel();
            let guard = session.begin_durable_item_loot_persistence_like_cpp();
            let worker = super::super::spawn_loot_claim_persistence_worker_like_cpp(
                async move {
                    let _ = started_tx.send(());
                    let _ = commit_rx.await;
                    Ok::<(), ()>(())
                },
                None,
                Some((
                    guard,
                    DurableItemLootCompletionLikeCpp {
                        owner_guid,
                        loot_list_id: 0,
                        player_guid,
                        item_owner_auto_release: false,
                        durable_item_money_applied_amount: Some(0),
                        durable_item_money_notified_amount: Some(0),
                        durable_item_money_balance_applied: Some(Arc::new(AtomicBool::new(false))),
                        item_fanout: None,
                        runtime_inventory_applied: Arc::new(AtomicBool::new(false)),
                    },
                )),
            )
            .unwrap();
            let waiter = tokio::spawn(async move { worker.await });
            started_rx.await.unwrap();
            waiter.abort();
            let _ = waiter.await;
            commit_tx.send(()).unwrap();
            session.wait_for_active_loot_persistence_like_cpp().await;
        } else {
            session.set_loot_money_persistence_test_result_like_cpp(true);
            session.handle_loot_money(loot_money_packet()).await;
            session.wait_for_active_loot_persistence_like_cpp().await;
        }

        let opcodes = drain_server_opcodes_like_cpp(&send_rx);
        assert_eq!(
            opcodes
                .iter()
                .filter(|opcode| {
                    **opcode == wow_constants::ServerOpcodes::LootMoneyNotify as u16
                })
                .count(),
            1,
            "zero money still notifies, while a normally published completion is not replayed"
        );
        session.wait_for_active_loot_persistence_like_cpp().await;
        assert!(drain_server_opcodes_like_cpp(&send_rx).is_empty());
    }
}
#[tokio::test]
async fn disconnect_waits_for_item_publication_and_releases_once_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 61_850);
    let owner_guid = ObjectGuid::create_item(1, 61_851);
    install_active_item_loot_completion_fixture_like_cpp(&mut session, player_guid, owner_guid, 0);
    let (commit_tx, commit_rx) = tokio::sync::oneshot::channel();
    let guard = session.begin_durable_item_loot_persistence_like_cpp();
    let worker = super::super::spawn_loot_claim_persistence_worker_like_cpp(
        async move {
            let _ = commit_rx.await;
            Ok::<(), ()>(())
        },
        None,
        Some((
            guard,
            DurableItemLootCompletionLikeCpp {
                owner_guid,
                loot_list_id: 0,
                player_guid,
                item_owner_auto_release: true,
                durable_item_money_applied_amount: None,
                durable_item_money_notified_amount: None,
                durable_item_money_balance_applied: None,
                item_fanout: None,
                runtime_inventory_applied: Arc::new(AtomicBool::new(true)),
            },
        )),
    )
    .unwrap();
    let release_commit = tokio::spawn(async move {
        tokio::task::yield_now().await;
        commit_tx.send(()).unwrap();
        worker.await.unwrap().unwrap();
    });

    session
        .cleanup_shared_runtime_state_on_disconnect_like_cpp()
        .await;
    release_commit.await.unwrap();

    assert_eq!(
        drain_server_opcodes_like_cpp(&send_rx)
            .into_iter()
            .filter(|opcode| *opcode == wow_constants::ServerOpcodes::LootRelease as u16)
            .count(),
        1
    );
}
#[test]
fn represented_personal_loot_remote_inventory_and_objective_conditions_use_registry_like_cpp() {
    let (mut session, _) = make_session_with_send_capacity(1);
    let mut quest_store = QuestStore::new();
    let mut quest = test_quest_template(100);
    quest.objectives.push(QuestObjective {
        id: 11,
        quest_id: 100,
        obj_type: 1,
        order: 0,
        storage_index: 0,
        object_id: 7001,
        amount: 7,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    quest_store.quests.insert(100, quest);
    session.set_quest_store(Arc::new(quest_store));
    let mut active_quest_statuses = HashMap::new();
    active_quest_statuses.insert(100, crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    let mut active_quest_objective_counts = HashMap::new();
    active_quest_objective_counts.insert(100, vec![5]);
    let mut inventory_item_counts = HashMap::new();
    inventory_item_counts.insert(9001, 2);
    let remote_context = RepresentedLootPlayerContext {
        race: 1,
        class: 1,
        gender: 0,
        level: 80,
        known_spells: Vec::new(),
        active_quest_statuses,
        active_quest_objective_counts,
        rewarded_quests: HashSet::new(),
        inventory_item_counts,
        is_current: false,
    };

    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(2, 9001, 2, 0),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(2, 9001, 3, 0),
            &remote_context,
        ),
        Some(false)
    );
    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(2, 9001, 2, 1),
            &remote_context,
        ),
        None
    );
    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(48, 11, 0, 5),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(48, 11, 0, 4),
            &remote_context,
        ),
        Some(false)
    );
}
#[test]
fn represented_personal_loot_remote_has_quest_for_item_objective_like_cpp() {
    let (mut session, _) = make_session_with_send_capacity(1);
    install_limited_test_item_template(&mut session, 7001, 0);
    let mut quest_store = QuestStore::new();
    let mut quest = test_quest_template(100);
    quest.objectives.push(QuestObjective {
        id: 1,
        quest_id: 100,
        obj_type: 1,
        order: 0,
        storage_index: 0,
        object_id: 7001,
        amount: 3,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    quest_store.quests.insert(100, quest);
    session.set_quest_store(Arc::new(quest_store));

    let mut active_quest_statuses = HashMap::new();
    active_quest_statuses.insert(100, crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    let mut active_quest_objective_counts = HashMap::new();
    active_quest_objective_counts.insert(100, vec![2]);
    let mut remote_context = RepresentedLootPlayerContext {
        race: 1,
        class: 1,
        gender: 0,
        level: 80,
        known_spells: Vec::new(),
        active_quest_statuses,
        active_quest_objective_counts,
        rewarded_quests: HashSet::new(),
        inventory_item_counts: HashMap::new(),
        is_current: false,
    };

    assert!(session.item_loot_quest_status_allows_for_player_like_cpp(
        7001,
        true,
        ItemTemplateAddonLootMetadataLikeCpp::default(),
        &remote_context,
    ));

    remote_context
        .active_quest_objective_counts
        .insert(100, vec![3]);
    assert!(!session.item_loot_quest_status_allows_for_player_like_cpp(
        7001,
        true,
        ItemTemplateAddonLootMetadataLikeCpp::default(),
        &remote_context,
    ));
}
#[test]
fn represented_personal_loot_remote_has_quest_for_item_drop_like_cpp() {
    let (mut session, _) = make_session_with_send_capacity(1);
    install_limited_test_item_template(&mut session, 7002, 0);
    let mut quest_store = QuestStore::new();
    let mut quest = test_quest_template(200);
    quest.item_drop[0] = 7002;
    quest.item_drop_quantity[0] = 4;
    quest_store.quests.insert(200, quest);
    session.set_quest_store(Arc::new(quest_store));

    let mut active_quest_statuses = HashMap::new();
    active_quest_statuses.insert(200, crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    let mut inventory_item_counts = HashMap::new();
    inventory_item_counts.insert(7002, 3);
    let mut remote_context = RepresentedLootPlayerContext {
        race: 1,
        class: 1,
        gender: 0,
        level: 80,
        known_spells: Vec::new(),
        active_quest_statuses,
        active_quest_objective_counts: HashMap::new(),
        rewarded_quests: HashSet::new(),
        inventory_item_counts,
        is_current: false,
    };

    assert!(session.item_loot_quest_status_allows_for_player_like_cpp(
        7002,
        true,
        ItemTemplateAddonLootMetadataLikeCpp::default(),
        &remote_context,
    ));

    remote_context.inventory_item_counts.insert(7002, 4);
    assert!(!session.item_loot_quest_status_allows_for_player_like_cpp(
        7002,
        true,
        ItemTemplateAddonLootMetadataLikeCpp::default(),
        &remote_context,
    ));
}
#[tokio::test]
async fn loot_item_added_progresses_incomplete_quest_item_objective_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let quest_id = 8_336;
    let item_id = 20_482;
    let mut quest = test_quest_template(quest_id);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: 1,
        order: 0,
        storage_index: 0,
        object_id: item_id as i32,
        amount: 6,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
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

    assert!(session.item_loot_quest_status_allows_like_cpp(
        item_id,
        true,
        ItemTemplateAddonLootMetadataLikeCpp::default(),
    ));

    let changed_quest_ids = session
        .apply_quest_source_item_added_non_bound_objective_progress_like_cpp(item_id, 0, 3)
        .await;

    assert_eq!(changed_quest_ids, vec![quest_id]);
    assert_eq!(
        session
            .player_quests
            .get(&quest_id)
            .expect("quest progress should remain active")
            .objective_counts,
        vec![3]
    );
    assert!(
        send_rx.try_recv().is_err(),
        "C++ UpdateQuestObjectiveProgress suppresses generic credit packets for ITEM objectives"
    );
}
#[test]
fn banked_quest_item_recomputes_objective_and_reopens_quest_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 1, 0);
    let quest_id = 8_338;
    let item_id = 20_484;
    let mut quest = test_quest_template(quest_id);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: 1,
        order: 0,
        storage_index: 0,
        object_id: item_id as i32,
        amount: 3,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![3],
            slot: 0,
        },
    );

    let planned = session.plan_bank_item_quest_persistence_like_cpp(item_id, 0, true, 0, 0);
    assert_eq!(planned.len(), 1);
    assert_eq!(planned[0].objective_counts, vec![0]);
    assert_eq!(
        planned[0].status,
        crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP
    );
    assert_eq!(
        session.apply_quest_item_removed_like_cpp(item_id),
        Some(vec![quest_id])
    );
    let status = session.player_quests.get(&quest_id).expect("active quest");
    assert_eq!(
        status.status,
        crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP
    );
    assert_eq!(status.objective_counts, vec![0]);
    let update = send_rx
        .try_recv()
        .expect("banking a quest item should update the quest-log slot");
    assert_eq!(
        WorldPacket::from_bytes(&update).server_opcode(),
        Some(wow_constants::ServerOpcodes::UpdateObject)
    );
}
#[tokio::test]
async fn withdrawn_banked_item_restores_bound_objective_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(4);
    let quest_id = 8_339;
    let item_id = 20_485;
    let mut quest = test_quest_template(quest_id);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: 1,
        order: 0,
        storage_index: 0,
        object_id: item_id as i32,
        amount: 1,
        flags: 0,
        flags2: 1,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
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

    let planned = session.plan_bank_item_quest_persistence_like_cpp(item_id, 0, false, 0, 1);
    assert_eq!(planned.len(), 1);
    assert_eq!(planned[0].objective_counts, vec![1]);
    assert_eq!(
        planned[0].status,
        crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP
    );
    let changed_quest_ids = session
        .apply_quest_item_added_objective_progress_like_cpp(item_id, 0, 1)
        .await;

    assert_eq!(changed_quest_ids, vec![quest_id]);
    let status = session.player_quests.get(&quest_id).expect("active quest");
    assert_eq!(status.objective_counts, vec![1]);
    assert_eq!(
        status.status,
        crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP
    );
}
#[tokio::test]
async fn loot_item_eligibility_does_not_treat_complete_quest_as_incomplete_like_cpp() {
    let (mut session, _) = make_session_with_send_capacity(1);
    let quest_id = 8_337;
    let item_id = 20_483;
    let mut quest = test_quest_template(quest_id);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: 1,
        order: 0,
        storage_index: 0,
        object_id: item_id as i32,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![0],
            slot: 0,
        },
    );

    assert!(!session.has_incomplete_quest_objective_for_item_like_cpp(item_id));
    assert!(!session.item_loot_quest_status_allows_like_cpp(
        item_id,
        true,
        ItemTemplateAddonLootMetadataLikeCpp::default(),
    ));

    let changed_quest_ids = session
        .apply_quest_source_item_added_non_bound_objective_progress_like_cpp(item_id, 0, 1)
        .await;

    assert!(changed_quest_ids.is_empty());
    assert_eq!(
        session
            .player_quests
            .get(&quest_id)
            .expect("complete quest should not progress as incomplete")
            .objective_counts,
        vec![0]
    );
}
#[test]
fn loot_item_random_context_runtime_fields_match_entry() {
    let item_guid = ObjectGuid::create_item(1, 902);
    let owner_guid = ObjectGuid::create_player(1, 42);
    let mut item = Item::new(0);
    item.initialize_created_state(ItemCreateInfo {
        guid: item_guid,
        item_id: 25,
        context: loot_item_context(2),
        owner: Some(owner_guid),
        max_durability: 0,
        expiration: 0,
        spell_charges: [0; MAX_ITEM_SPELLS],
    });
    item.set_random_properties_id(-77);
    item.set_property_seed(456);

    let data = item.data();
    assert_eq!(data.random_properties_id, -77);
    assert_eq!(data.property_seed, 456);
    assert_eq!(u8::try_from(data.context).unwrap_or(0), 2);
}
#[test]
fn stored_new_item_flags_follow_cpp_new_and_binding_rules() {
    let mut session = make_session();
    let item_id = 25;

    install_limited_test_item_template_with_flags2_and_bonding(
        &mut session,
        item_id,
        0,
        0,
        ItemBondingType::OnAcquire,
    );
    assert_eq!(
        session.stored_new_item_dynamic_flags_like_cpp(item_id, INVENTORY_SLOT_ITEM_START),
        (ItemFieldFlags::NEW_ITEM | ItemFieldFlags::SOULBOUND).bits()
    );

    install_limited_test_item_template_with_flags2_and_bonding(
        &mut session,
        item_id,
        0,
        0,
        ItemBondingType::None,
    );
    assert_eq!(
        session.stored_new_item_dynamic_flags_like_cpp(item_id, INVENTORY_SLOT_ITEM_START),
        ItemFieldFlags::NEW_ITEM.bits(),
        "an unbound backpack item must not acquire SOULBOUND"
    );

    install_limited_test_item_template_with_flags2_and_bonding(
        &mut session,
        item_id,
        0,
        0,
        ItemBondingType::OnEquip,
    );
    assert_eq!(
        session.stored_new_item_dynamic_flags_like_cpp(item_id, INVENTORY_SLOT_ITEM_START),
        ItemFieldFlags::NEW_ITEM.bits(),
        "C++ bind-if-stored does not bind OnEquip items in backpack slots"
    );
}
