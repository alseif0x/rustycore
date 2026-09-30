//! Original application scenarios over the real persistence lane.
use super::storage_support::*;

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
    let granted_item = 25;
    let inventory_before = applied_loot_item_quantity_for_test(&first, granted_item);
    install_storage_port(
        &mut first,
        PersistenceOutcomeLikeCpp::Applied { rows: 1 },
        None,
    );

    handle_loot_item_for_test(
        &mut first,
        loot_item_packet(represented_loot_object_guid_like_cpp(owner), 0),
    )
    .await;

    assert_eq!(
        (applied_loot_item_quantity_for_test(&first, granted_item)) - inventory_before,
        0,
        "C++ StoreNewItem returns nullptr for quest-bound objective credit"
    );
    let quest_state =
        wow_world::test_fixtures::quest::player_quest_gameplay_snapshot_for_test(&first)
            .expect("resident quest owner");
    let status = quest_state
        .statuses_like_cpp()
        .get(&quest_id)
        .expect("active quest");
    assert_eq!(status.objective_counts, vec![6]);
    assert_eq!(
        status.status,
        wow_world::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP
    );
    assert!(!loot_item_quest_allowed_for_test(
        &first, item_id, true, 0, 0, None
    ));

    let authority =
        wow_world::test_fixtures::loot::loot_recovery_authority_for_test(&mut first, owner)
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
    install_loot_inventory_item_for_test(
        &mut first,
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: existing_guid,
            entry_id: item_id,
            db_guid: existing_guid.counter() as u64,
            inventory_type: None,
        },
        first_guid,
        1,
        0,
        ItemContext::None,
    );
    let granted_item = 25;
    let inventory_before = applied_loot_item_quantity_for_test(&first, granted_item);
    install_storage_port(
        &mut first,
        PersistenceOutcomeLikeCpp::Applied { rows: 1 },
        None,
    );

    handle_loot_item_for_test(
        &mut first,
        loot_item_packet(represented_loot_object_guid_like_cpp(owner), 0),
    )
    .await;

    assert_eq!(
        (applied_loot_item_quantity_for_test(&first, granted_item)) - inventory_before,
        0
    );
    let quest_state =
        wow_world::test_fixtures::quest::player_quest_gameplay_snapshot_for_test(&first)
            .expect("resident quest owner");
    let status = quest_state
        .statuses_like_cpp()
        .get(&quest_id)
        .expect("active quest");
    assert_eq!(status.objective_counts, vec![5]);
    assert_eq!(
        status.status,
        wow_world::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP
    );
    let authority =
        wow_world::test_fixtures::loot::loot_recovery_authority_for_test(&mut first, owner)
            .unwrap();
    let snapshot = authority.snapshot_for_player_like_cpp(first_guid).unwrap();
    assert!(!snapshot.loot.items[0].taken);
    assert_eq!(snapshot.loot.unlooted_count, 1);
}

#[tokio::test]
async fn failed_quest_bound_loot_persistence_rolls_back_credit_and_claim_like_cpp() {
    let (mut first, first_rx, _second, _second_rx, owner, first_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    let quest_id = 8_336;
    install_quest_bound_loot_objective_like_cpp(&mut first, quest_id, 25, 5, 6);
    let granted_item = 25;
    let inventory_before = applied_loot_item_quantity_for_test(&first, granted_item);
    install_storage_port(
        &mut first,
        PersistenceOutcomeLikeCpp::Failed {
            reason: "planned transaction failure".to_owned(),
        },
        None,
    );

    handle_loot_item_for_test(
        &mut first,
        loot_item_packet(represented_loot_object_guid_like_cpp(owner), 0),
    )
    .await;

    assert_eq!(
        (applied_loot_item_quantity_for_test(&first, granted_item)) - inventory_before,
        0
    );
    let quest_state =
        wow_world::test_fixtures::quest::player_quest_gameplay_snapshot_for_test(&first)
            .expect("resident quest owner");
    let status = quest_state
        .statuses_like_cpp()
        .get(&quest_id)
        .expect("active quest");
    assert_eq!(status.objective_counts, vec![5]);
    assert_eq!(
        status.status,
        wow_world::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP
    );
    let authority =
        wow_world::test_fixtures::loot::loot_recovery_authority_for_test(&mut first, owner)
            .unwrap();
    let snapshot = authority.snapshot_for_player_like_cpp(first_guid).unwrap();
    assert!(!snapshot.loot.items[0].taken);
    assert_eq!(snapshot.loot.unlooted_count, 1);
}
