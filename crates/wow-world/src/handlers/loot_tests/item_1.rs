//! Item scenarios for [`super`].
//!
//! Split out of loot_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn stored_item_money_commit_unknown_requires_joint_balance_and_source_evidence_like_cpp() {
    let outcome = StoredItemMoneyPersistenceOutcomeLikeCpp {
        before: 100,
        after: 107,
        applied_delta: 7,
        notified_amount: 7,
    };
    assert_eq!(
        classify_stored_item_money_reconciliation_like_cpp(outcome, 100, Some(7)),
        StoredItemMoneyReconciliationLikeCpp::RolledBack
    );
    assert_eq!(
        classify_stored_item_money_reconciliation_like_cpp(outcome, 107, None),
        StoredItemMoneyReconciliationLikeCpp::Committed
    );
    assert_eq!(
        classify_stored_item_money_reconciliation_like_cpp(outcome, 100, None),
        StoredItemMoneyReconciliationLikeCpp::Indeterminate { reason: None },
        "a missing source alone cannot attribute a later consumer's commit to this attempt"
    );
    assert_eq!(
        classify_stored_item_money_reconciliation_like_cpp(outcome, 107, Some(7)),
        StoredItemMoneyReconciliationLikeCpp::Indeterminate { reason: None }
    );
}
#[test]
fn stored_item_money_cap_noop_still_reconciles_source_consumption_like_cpp() {
    let outcome = StoredItemMoneyPersistenceOutcomeLikeCpp {
        before: MAX_MONEY_AMOUNT - 1,
        after: MAX_MONEY_AMOUNT - 1,
        applied_delta: 0,
        notified_amount: 2,
    };
    assert_eq!(
        classify_stored_item_money_reconciliation_like_cpp(outcome, MAX_MONEY_AMOUNT - 1, None,),
        StoredItemMoneyReconciliationLikeCpp::Committed
    );
    assert_eq!(
        classify_stored_item_money_reconciliation_like_cpp(outcome, MAX_MONEY_AMOUNT - 1, Some(2),),
        StoredItemMoneyReconciliationLikeCpp::RolledBack
    );
}
#[test]
fn stored_item_money_zero_without_db_source_is_success_but_positive_is_consumed() {
    let zero = stored_item_money_zero_without_source_outcome_like_cpp(41, 0).unwrap();
    assert_eq!(zero.before, 41);
    assert_eq!(zero.after, 41);
    assert_eq!(zero.applied_delta, 0);
    assert_eq!(zero.notified_amount, 0);
    assert!(stored_item_money_zero_without_source_outcome_like_cpp(41, 1).is_none());
}
#[tokio::test]
async fn stored_item_money_worker_requires_and_uses_the_typed_persistence_port_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 501);
    let item_guid = ObjectGuid::create_item(1, 502);
    session.set_player_guid(Some(player_guid));
    session.clear_loot_money_persistence_test_result_like_cpp();

    assert!(
        session
            .persist_and_consume_stored_item_money_like_cpp(item_guid, 7)
            .await
            .is_none(),
        "production-shaped stored money fails closed without its typed port"
    );

    session.set_stored_item_money_persistence_port_like_cpp(Arc::new(
        StoredItemMoneyPortFixtureLikeCpp {
            attempt: StoredItemMoneyPersistenceAttemptLikeCpp::Applied(
                StoredItemMoneyPersistenceOutcomeLikeCpp {
                    before: 100,
                    after: 107,
                    applied_delta: 7,
                    notified_amount: 7,
                },
            ),
        },
    ));
    let (_, _, applied_delta, notified_amount) = session
        .persist_and_consume_stored_item_money_like_cpp(item_guid, 7)
        .await
        .expect("typed stored-money port outcome");
    assert_eq!((applied_delta, notified_amount), (7, 7));
}
#[test]
fn stored_item_money_completion_applies_db_delta_to_divergent_runtime_base_like_cpp() {
    let (db_after, durable_delta) = loot_money_durable_outcome_like_cpp(100, 7);
    let runtime_before = 500;
    let runtime_after = runtime_before + durable_delta;

    assert_eq!(db_after, 107);
    assert_eq!(runtime_after, 507);
    assert_ne!(runtime_after, db_after);
}
#[test]
fn item_instance_guid_allocator_is_shared_across_concurrent_loot_sessions_like_cpp() {
    const WORKERS: usize = 8;
    const GUIDS_PER_WORKER: usize = 128;
    const FIRST_GUID: i64 = 40_000;

    let generator = Arc::new(ObjectGuidGenerator::new(HighGuid::Item, FIRST_GUID));
    let start = Arc::new(Barrier::new(WORKERS));
    let handles = (0..WORKERS)
        .map(|_| {
            let generator = Arc::clone(&generator);
            let start = Arc::clone(&start);
            std::thread::spawn(move || {
                let mut session = make_session();
                session.set_realm_id(7);
                session.set_item_guid_generator_like_cpp(generator);
                start.wait();
                session
                    .allocate_item_instance_guids_like_cpp(GUIDS_PER_WORKER)
                    .expect("shared item allocator must be installed")
            })
        })
        .collect::<Vec<_>>();

    let mut allocated = handles
        .into_iter()
        .flat_map(|handle| handle.join().expect("allocation worker must finish"))
        .collect::<Vec<_>>();
    allocated.sort_unstable_by_key(|(db_guid, _)| *db_guid);

    assert_eq!(allocated.len(), WORKERS * GUIDS_PER_WORKER);
    for (offset, (db_guid, object_guid)) in allocated.iter().enumerate() {
        let expected = FIRST_GUID as u64 + offset as u64;
        assert_eq!(*db_guid, expected);
        assert!(object_guid.is_item());
        assert_eq!(object_guid.counter() as u64, expected);
    }
    assert_eq!(
        generator.next_after_max_used(),
        FIRST_GUID + (WORKERS * GUIDS_PER_WORKER) as i64
    );
}
#[test]
fn item_instance_guid_allocator_fails_closed_and_never_reuses_failed_grant_like_cpp() {
    let mut session = make_session();
    assert_eq!(session.allocate_item_instance_guids_like_cpp(1), None);

    let generator = Arc::new(ObjectGuidGenerator::new(HighGuid::Item, 91_000));
    session.set_item_guid_generator_like_cpp(Arc::clone(&generator));

    // C++ consumes a GUID when Item::CreateItem runs.  A later storage or
    // transaction failure may leave a gap, but must never make that GUID
    // available to a competing durable grant.
    let abandoned_after_persistence_failure = session
        .allocate_item_instance_guids_like_cpp(1)
        .expect("allocator must be installed");
    assert_eq!(abandoned_after_persistence_failure[0].0, 91_000);
    drop(abandoned_after_persistence_failure);

    let next_grant = session
        .allocate_item_instance_guids_like_cpp(1)
        .expect("allocator must remain installed");
    assert_eq!(next_grant[0].0, 91_001);
    assert_eq!(generator.next_after_max_used(), 91_002);
}
#[test]
fn represented_loot_item_push_result_uses_realm_route_and_cpp_encounter_fields() {
    let (mut session, instance_rx) = make_session_with_send();
    let (realm_tx, realm_rx) = flume::bounded(1);
    session.install_realm_send_channel_for_test(realm_tx);
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 700);
    let entry = represented_loot_entry(0, 25, player_guid);

    session.send_loot_item_push_result(player_guid, item_guid, &entry, 0, 0, 0, 1, 1, false, 615);

    assert!(instance_rx.try_recv().is_err());
    let sent = realm_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::ItemPushResult as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), player_guid);
    assert_eq!(sent.read_uint8().unwrap(), u8::from(INVENTORY_SLOT_BAG_0));
    assert_eq!(sent.read_int32().unwrap(), 0);
    assert_eq!(sent.read_int32().unwrap(), 0);
    assert_eq!(sent.read_int32().unwrap(), 1);
    assert_eq!(sent.read_int32().unwrap(), 1);
    assert_eq!(sent.read_int32().unwrap(), 615);
    assert_eq!(sent.read_int32().unwrap(), 0);
    assert_eq!(sent.read_int32().unwrap(), 0);
    assert_eq!(sent.read_uint32().unwrap(), 0);
    assert_eq!(sent.read_int32().unwrap(), 0);
    assert_eq!(sent.read_packed_guid().unwrap(), item_guid);
    assert!(!sent.read_bit().unwrap());
    assert!(!sent.read_bit().unwrap());
    assert_eq!(sent.read_bits(3).unwrap(), 2);
    assert!(!sent.read_bit().unwrap());
    assert!(sent.read_bit().unwrap());
    assert_eq!(sent.read_int32().unwrap(), 25);
}
#[test]
fn represented_loot_response_items_use_cpp_ui_type_decision_tree() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 77);
    let loot_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 100);

    let mut rolling_entry = represented_loot_entry(0, 25, player_guid);
    rolling_entry.flags.blocked = true;

    let mut won_entry = represented_loot_entry(1, 26, player_guid);
    won_entry.roll_winner = player_guid;

    let mut hidden_entry = represented_loot_entry(2, 27, player_guid);
    hidden_entry.roll_winner = other_guid;

    let mut allowed_entry = represented_loot_entry(3, 28, player_guid);
    allowed_entry.flags.under_threshold = true;

    let loot = CreatureLoot {
        loot_guid,
        coins: 0,
        unlooted_count: 0,
        loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
        dungeon_encounter_id: 0,
        loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: vec![player_guid],
        items: vec![rolling_entry, won_entry, hidden_entry, allowed_entry],
        looted_by_player: false,
    };

    let items = represented_loot_response_items_like_cpp(&loot, player_guid);

    assert_eq!(items.len(), 3);
    assert_eq!(items[0].loot_list_id, 0);
    assert_eq!(items[0].ui_type, LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP);
    assert_eq!(items[1].loot_list_id, 1);
    assert_eq!(items[1].ui_type, LOOT_SLOT_TYPE_OWNER_LIKE_CPP);
    assert_eq!(items[2].loot_list_id, 3);
    assert_eq!(items[2].ui_type, LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP);
}
#[test]
fn represented_ffa_loot_uses_player_ffa_items_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 77);
    let loot_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 101);

    let mut ffa_entry = represented_loot_entry(0, 25, player_guid);
    ffa_entry.flags.freeforall = true;
    ffa_entry.allowed_looters.clear();

    let mut loot = CreatureLoot {
        loot_guid,
        coins: 0,
        unlooted_count: 0,
        loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
        dungeon_encounter_id: 0,
        loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: Vec::new(),
        items: vec![ffa_entry],
        looted_by_player: false,
    };

    mark_loot_allowed_for_player_like_cpp(&mut loot, player_guid);
    mark_loot_allowed_for_player_like_cpp(&mut loot, other_guid);
    assert_eq!(loot.unlooted_count, 2);

    let player_items = represented_loot_response_items_like_cpp(&loot, player_guid);
    let other_items = represented_loot_response_items_like_cpp(&loot, other_guid);
    assert_eq!(player_items.len(), 1);
    assert_eq!(other_items.len(), 1);
    assert_eq!(player_items[0].ui_type, LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP);
    assert_eq!(other_items[0].ui_type, LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP);

    mark_loot_item_looted_for_player_like_cpp(&mut loot, 0, player_guid);
    assert_eq!(loot.unlooted_count, 1);

    let player_ffa = loot
        .player_ffa_items
        .iter()
        .find(|(player, _)| *player == player_guid)
        .and_then(|(_, items)| items.iter().find(|item| item.loot_list_id == 0))
        .unwrap();
    let other_ffa = loot
        .player_ffa_items
        .iter()
        .find(|(player, _)| *player == other_guid)
        .and_then(|(_, items)| items.iter().find(|item| item.loot_list_id == 0))
        .unwrap();

    assert!(player_ffa.is_looted);
    assert!(!other_ffa.is_looted);
    assert!(represented_loot_response_items_like_cpp(&loot, player_guid).is_empty());
    assert_eq!(
        represented_loot_response_items_like_cpp(&loot, other_guid).len(),
        1
    );
}
#[test]
fn prospecting_and_milling_release_consume_at_most_five_source_items_like_cpp() {
    assert_eq!(
        direct_item_count_after_loot_release_like_cpp(20, Some(5)),
        15
    );
    assert_eq!(direct_item_count_after_loot_release_like_cpp(5, Some(5)), 0);
    assert_eq!(direct_item_count_after_loot_release_like_cpp(3, Some(5)), 0);
    assert_eq!(direct_item_count_after_loot_release_like_cpp(20, None), 0);
}
#[test]
fn represented_unlooted_count_counts_shared_items_once_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 77);
    let loot_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 102);

    let mut entry = represented_loot_entry(0, 25, player_guid);
    entry.allowed_looters.clear();

    let mut loot = CreatureLoot {
        loot_guid,
        coins: 0,
        unlooted_count: 0,
        loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
        dungeon_encounter_id: 0,
        loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: Vec::new(),
        items: vec![entry],
        looted_by_player: false,
    };

    mark_loot_allowed_for_player_like_cpp(&mut loot, player_guid);
    assert_eq!(loot.unlooted_count, 1);
    assert!(loot.items[0].flags.counted);

    mark_loot_allowed_for_player_like_cpp(&mut loot, other_guid);
    assert_eq!(loot.unlooted_count, 1);

    mark_loot_item_looted_for_player_like_cpp(&mut loot, 0, player_guid);
    assert_eq!(loot.unlooted_count, 0);
    mark_loot_item_looted_for_player_like_cpp(&mut loot, 0, player_guid);
    assert_eq!(loot.unlooted_count, 0);
}
#[test]
fn durable_item_fanout_uses_precommit_union_exact_commit_cut_like_cpp() {
    let before = ObjectGuid::create_player(1, 41);
    let during = ObjectGuid::create_player(1, 42);
    let after = ObjectGuid::create_player(1, 43);

    let viewers =
        super::super::durable_loot_item_fanout_viewers_like_cpp(&[before], &[before, during]);

    assert_eq!(viewers, HashSet::from([before, during]));
    assert!(
        !viewers.contains(&after),
        "a later authority sample must never expand the exact commit fanout"
    );
}
#[tokio::test]
async fn two_sessions_claim_one_authoritative_item_exactly_once_like_cpp() {
    let (mut first, _first_rx, mut second, _second_rx, owner, first_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let grants = Arc::new(AtomicUsize::new(0));
    first.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);
    second.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);
    let barrier = Arc::new(tokio::sync::Barrier::new(3));
    let loot_obj = represented_loot_object_guid_like_cpp(owner);

    let first_barrier = Arc::clone(&barrier);
    let first_task = tokio::spawn(async move {
        first_barrier.wait().await;
        first.handle_loot_item(loot_item_packet(loot_obj, 0)).await;
        first
    });
    let second_barrier = Arc::clone(&barrier);
    let second_task = tokio::spawn(async move {
        second_barrier.wait().await;
        second.handle_loot_item(loot_item_packet(loot_obj, 0)).await;
        second
    });
    barrier.wait().await;

    let mut first = first_task.await.unwrap();
    let _second = second_task.await.unwrap();
    assert_eq!(grants.load(Ordering::SeqCst), 1);
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let snapshot = authority.snapshot_for_player_like_cpp(first_guid).unwrap();
    assert!(snapshot.loot.items[0].taken);
    assert_eq!(snapshot.loot.unlooted_count, 0);
}
#[tokio::test]
async fn durable_direct_item_claim_notifies_removed_before_item_push_like_cpp() {
    let (mut first, first_rx, _second, _second_rx, owner, _first_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    // Use one observer channel for both physical routes so this test can
    // assert their relative C++ wire order. Route separation is covered
    // independently by the ItemPushResult test above.
    let shared_send = first.send_tx().clone();
    first.install_realm_send_channel_for_test(shared_send);
    let grants = Arc::new(AtomicUsize::new(0));
    first.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);

    first
        .handle_loot_item(loot_item_packet(
            represented_loot_object_guid_like_cpp(owner),
            0,
        ))
        .await;

    assert_eq!(grants.load(Ordering::SeqCst), 1);
    assert_eq!(
        drain_server_opcodes_like_cpp(&first_rx),
        vec![
            wow_constants::ServerOpcodes::LootRemoved as u16,
            wow_constants::ServerOpcodes::ItemPushResult as u16,
        ],
        "C++ Player::StoreLootItem notifies removal before SendNewItem"
    );
}
#[tokio::test]
async fn quest_bound_loot_credits_objective_without_physical_item_like_cpp() {
    let (mut first, first_rx, _second, _second_rx, owner, first_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    let quest_id = 8_336;
    let item_id = 25;
    install_quest_bound_loot_objective_like_cpp(&mut first, quest_id, item_id, 5, 6);
    let grants = Arc::new(AtomicUsize::new(0));
    first.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);

    first
        .handle_loot_item(loot_item_packet(
            represented_loot_object_guid_like_cpp(owner),
            0,
        ))
        .await;

    assert_eq!(
        grants.load(Ordering::SeqCst),
        0,
        "C++ StoreNewItem returns nullptr for quest-bound objective credit"
    );
    let status = first.player_quests.get(&quest_id).expect("active quest");
    assert_eq!(status.objective_counts, vec![6]);
    assert_eq!(
        status.status,
        crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP
    );
    assert!(!first.item_loot_quest_status_allows_like_cpp(
        item_id,
        true,
        ItemTemplateAddonLootMetadataLikeCpp::default(),
    ));

    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let snapshot = authority.snapshot_for_player_like_cpp(first_guid).unwrap();
    assert!(snapshot.loot.items[0].taken);
    assert_eq!(snapshot.loot.unlooted_count, 0);

    let opcodes = drain_server_opcodes_like_cpp(&first_rx);
    let bound_credit = wow_constants::ServerOpcodes::ItemPushResult as u16;
    let loot_removed = wow_constants::ServerOpcodes::LootRemoved as u16;
    assert!(opcodes.contains(&bound_credit), "{opcodes:?}");
    assert!(opcodes.contains(&loot_removed), "{opcodes:?}");
    assert!(
        opcodes.iter().position(|opcode| *opcode == bound_credit)
            < opcodes.iter().position(|opcode| *opcode == loot_removed),
        "C++ bound objective notification precedes the committed loot removal: {opcodes:?}"
    );
}
#[tokio::test]
async fn quest_bound_loot_still_requires_can_store_new_item_like_cpp() {
    let (mut first, first_rx, _second, _second_rx, owner, first_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    let quest_id = 8_336;
    let item_id = 25;
    install_quest_bound_loot_objective_like_cpp(&mut first, quest_id, item_id, 5, 6);
    install_limited_test_item_template(&mut first, item_id, 1);
    let existing_guid = ObjectGuid::create_item(1, 83_360);
    first.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: existing_guid,
            entry_id: item_id,
            db_guid: existing_guid.counter() as u64,
            inventory_type: None,
        },
    );
    let existing_item = first.make_inventory_item_object(
        existing_guid,
        item_id,
        first_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    first.insert_inventory_item_object(existing_item);
    let grants = Arc::new(AtomicUsize::new(0));
    first.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);

    first
        .handle_loot_item(loot_item_packet(
            represented_loot_object_guid_like_cpp(owner),
            0,
        ))
        .await;

    assert_eq!(grants.load(Ordering::SeqCst), 0);
    let status = first.player_quests.get(&quest_id).expect("active quest");
    assert_eq!(status.objective_counts, vec![5]);
    assert_eq!(
        status.status,
        crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP
    );
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let snapshot = authority.snapshot_for_player_like_cpp(first_guid).unwrap();
    assert!(!snapshot.loot.items[0].taken);
    assert_eq!(snapshot.loot.unlooted_count, 1);
}
#[tokio::test]
async fn item_grant_commit_unknown_quarantines_claim_and_kicks_even_when_queue_full() {
    let (mut first, _first_rx, _second, _second_rx, owner, first_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(first_guid, 0)
        .await
        .unwrap();
    let (command_tx, command_rx) = flume::bounded(1);
    command_tx
        .send(SessionCommand::KickLikeCpp(KickLikeCppCommand {
            reason: "preexisting".to_string(),
        }))
        .unwrap();

    let worker = super::super::spawn_loot_item_persistence_worker_like_cpp(
        async {
            PersistenceOutcomeLikeCpp::Unknown {
                reason: "lost COMMIT reply".to_string(),
            }
        },
        Some(claim),
        None,
        command_tx,
    )
    .unwrap();
    let result = worker.await.unwrap();

    assert!(matches!(
        result,
        Err(super::super::LootClaimPersistenceWorkerError::Persistence(reason))
            if reason == "lost COMMIT reply"
    ));
    assert_eq!(
        authority.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Quarantined
    );
    assert!(
        authority
            .reserve_item_for_award_like_cpp(first_guid, 0)
            .await
            .is_err(),
        "unknown COMMIT must never reopen the object-owned item claim"
    );

    let _preexisting = command_rx.recv().unwrap();
    let queued = tokio::time::timeout(Duration::from_secs(1), command_rx.recv_async())
        .await
        .expect("full command queue fallback must eventually enqueue the kick")
        .unwrap();
    assert!(matches!(queued, SessionCommand::KickLikeCpp(_)));
}
#[tokio::test]
async fn failed_authoritative_item_store_rolls_back_for_retry_like_cpp() {
    let (mut first, _first_rx, _second, _second_rx, owner, first_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let grants = Arc::new(AtomicUsize::new(0));
    first.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), false);
    let loot_obj = represented_loot_object_guid_like_cpp(owner);

    first.handle_loot_item(loot_item_packet(loot_obj, 0)).await;
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    assert!(
        !authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .items[0]
            .taken
    );

    first.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);
    first.handle_loot_item(loot_item_packet(loot_obj, 0)).await;
    assert_eq!(grants.load(Ordering::SeqCst), 1);
    assert!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .items[0]
            .taken
    );
}
#[tokio::test]
async fn stale_active_item_view_cannot_claim_replacement_generation_like_cpp() {
    let (mut first, _first_rx, _second, _second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let mut replacement = authoritative_test_loot_like_cpp(0, true);
    replacement.loot_guid = represented_loot_object_guid_like_cpp(owner);
    replacement.allowed_looters = vec![first_guid, second_guid];
    replacement.items[0].allowed_looters = vec![first_guid, second_guid];
    let replacement_generation = authority.replace_like_cpp(Some(replacement), HashMap::new());
    let grants = Arc::new(AtomicUsize::new(0));
    first.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);

    first
        .handle_loot_item(loot_item_packet(
            represented_loot_object_guid_like_cpp(owner),
            0,
        ))
        .await;

    assert_eq!(grants.load(Ordering::SeqCst), 0);
    let snapshot = authority.snapshot_for_player_like_cpp(first_guid).unwrap();
    assert_eq!(snapshot.generation, replacement_generation);
    assert!(!snapshot.loot.items[0].taken);
}
#[tokio::test]
async fn item_waiter_waking_on_replacement_rolls_back_new_generation_claim_like_cpp() {
    let (mut first, _first_rx, _second, _second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let blocker = authority
        .reserve_item_like_cpp(first_guid, 0)
        .await
        .unwrap();
    let grants = Arc::new(AtomicUsize::new(0));
    first.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);
    let loot_obj = represented_loot_object_guid_like_cpp(owner);
    let waiter = tokio::spawn(async move {
        first.handle_loot_item(loot_item_packet(loot_obj, 0)).await;
        first
    });
    for _ in 0..4 {
        tokio::task::yield_now().await;
    }

    let mut replacement = authoritative_test_loot_like_cpp(0, true);
    replacement.loot_guid = loot_obj;
    replacement.allowed_looters = vec![first_guid, second_guid];
    replacement.items[0].allowed_looters = vec![first_guid, second_guid];
    let replacement_generation = authority.replace_like_cpp(Some(replacement), HashMap::new());
    drop(blocker);
    let _first = waiter.await.unwrap();

    assert_eq!(grants.load(Ordering::SeqCst), 0);
    let snapshot = authority.snapshot_for_player_like_cpp(first_guid).unwrap();
    assert_eq!(snapshot.generation, replacement_generation);
    assert!(!snapshot.loot.items[0].taken);
}
#[tokio::test]
async fn cancelled_item_waiter_cannot_reopen_a_durable_claim_like_cpp() {
    let (mut first, _first_rx, _second, _second_rx, owner, first_guid, _second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(first_guid, 0)
        .await
        .unwrap();
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = tokio::sync::oneshot::channel();
    let worker = super::super::spawn_loot_claim_persistence_worker_like_cpp(
        async move {
            let _ = started_tx.send(());
            let _ = release_rx.await;
            Ok::<(), ()>(())
        },
        Some(claim),
        None,
    )
    .unwrap();
    let waiter = tokio::spawn(async move { worker.await });

    started_rx.await.unwrap();
    waiter.abort();
    let _ = waiter.await;
    release_tx.send(()).unwrap();
    for _ in 0..4 {
        tokio::task::yield_now().await;
    }

    let snapshot = authority.snapshot_for_player_like_cpp(first_guid).unwrap();
    assert!(snapshot.loot.items[0].taken);
    assert_eq!(snapshot.loot.unlooted_count, 0);
    assert!(
        authority
            .reserve_item_for_award_like_cpp(first_guid, 0)
            .await
            .is_err()
    );
}
#[tokio::test]
async fn durable_item_completion_auto_releases_only_after_items_and_coins_are_empty_like_cpp() {
    for (coins, should_release) in [(0, true), (7, false)] {
        let (mut session, send_rx) = make_session_with_send_capacity(16);
        let player_guid = ObjectGuid::create_player(1, 61_801 + i64::from(coins));
        let owner_guid = ObjectGuid::create_item(1, 61_811 + i64::from(coins));
        install_active_item_loot_completion_fixture_like_cpp(
            &mut session,
            player_guid,
            owner_guid,
            coins,
        );
        let runtime_inventory_applied = Arc::new(AtomicBool::new(true));
        let guard = session.begin_durable_item_loot_persistence_like_cpp();
        super::super::spawn_loot_claim_persistence_worker_like_cpp(
            async { Ok::<(), ()>(()) },
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
                    runtime_inventory_applied,
                },
            )),
        )
        .unwrap()
        .await
        .unwrap()
        .unwrap();

        session.wait_for_active_loot_persistence_like_cpp().await;

        assert!(!session.is_disconnecting());
        assert_eq!(
            session.loot_table.contains_key(&owner_guid),
            !should_release
        );
        assert_eq!(session.is_active_loot_guid(owner_guid), !should_release);
        if !should_release {
            let loot = session.loot_table.get(&owner_guid).unwrap();
            assert!(loot.items[0].taken);
            assert_eq!(loot.coins, coins);
        }
        assert_eq!(
            drain_server_opcodes_like_cpp(&send_rx)
                .into_iter()
                .filter(|opcode| { *opcode == wow_constants::ServerOpcodes::LootRelease as u16 })
                .count(),
            usize::from(should_release),
            "C++ StoreLootItem checks Loot::isLooted(), including money"
        );
    }
}
#[tokio::test]
async fn cancelled_item_handler_after_commit_releases_and_forces_inventory_reload_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 61_830);
    let owner_guid = ObjectGuid::create_item(1, 61_831);
    install_active_item_loot_completion_fixture_like_cpp(&mut session, player_guid, owner_guid, 0);
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
                item_owner_auto_release: true,
                durable_item_money_applied_amount: None,
                durable_item_money_notified_amount: None,
                durable_item_money_balance_applied: None,
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

    assert!(session.is_disconnecting());
    assert!(!session.loot_table.contains_key(&owner_guid));
    assert!(!session.is_active_loot_guid(owner_guid));
    assert_eq!(
        drain_server_opcodes_like_cpp(&send_rx)
            .into_iter()
            .filter(|opcode| *opcode == wow_constants::ServerOpcodes::LootRelease as u16)
            .count(),
        1
    );
}
