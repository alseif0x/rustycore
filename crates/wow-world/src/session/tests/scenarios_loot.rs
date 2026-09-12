//! Session scenarios exercising the represented loot responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn player_registry_publishes_loot_condition_state_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 42);
    let registry = Arc::new(PlayerRegistry::default());
    session.set_player_guid(Some(guid));
    session.player_position = Some(Position::ZERO);
    session.current_map_id = 0;
    bind_canonical_test_player_to_registry_like_cpp(
        &mut session,
        &registry,
        guid,
        Position::ZERO,
        0,
    );
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
    session.player_name = Some("Tester".to_string());
    session.known_spells = vec![12_345];
    session.mutate_player_quest_gameplay_like_cpp(|state| {
        state.insert_status_like_cpp(
            100,
            wow_entities::PlayerQuestStatusRecord {
                quest_id: 100,
                status: 1,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: vec![2, 3],
                slot: 0,
            },
        );
        state.set_objective_counts_for_quest_like_cpp(100, vec![2, 3]);
        state.set_rewarded_like_cpp(200, true);
    });
    let item_guid = ObjectGuid::create_item(1, 500);
    session.insert_inventory_item_like_cpp(
        0,
        InventoryItem {
            guid: item_guid,
            entry_id: 9001,
            db_guid: 500,
            inventory_type: Some(0),
        },
    );
    session.insert_inventory_item_object(session.make_inventory_item_object(
        item_guid,
        9001,
        guid,
        4,
        0,
        ItemContext::None,
        0,
    ));
    let bag_guid = ObjectGuid::create_item(1, 501);
    session.insert_inventory_item_like_cpp(
        1,
        InventoryItem {
            guid: bag_guid,
            entry_id: 8000,
            db_guid: 501,
            inventory_type: Some(18),
        },
    );
    session.insert_inventory_item_object(session.make_inventory_item_object(
        bag_guid,
        8000,
        guid,
        1,
        0,
        ItemContext::None,
        1,
    ));
    let child_guid = ObjectGuid::create_item(1, 502);
    let mut child_item =
        session.make_inventory_item_object(child_guid, 9001, guid, 2, 0, ItemContext::None, 0);
    child_item.set_container_guid_and_slot(bag_guid, 0);
    session.insert_inventory_item_object(child_item);
    session.set_player_registry(Arc::clone(&registry));

    session.register_in_player_registry();
    {
        let info = registry
            .loot_player_context(guid)
            .expect("registered player");
        assert_eq!(info.known_spells, vec![12_345]);
        assert_eq!(info.active_quest_statuses.get(&100), Some(&1));
        assert_eq!(
            info.active_quest_objective_counts.get(&100),
            Some(&vec![2, 3])
        );
        assert!(info.rewarded_quests.contains(&200));
        assert_eq!(info.inventory_item_counts.get(&9001), Some(&6));
    }

    session.known_spells.push(54_321);
    session.mutate_player_quest_gameplay_like_cpp(|state| {
        state.insert_status_like_cpp(
            300,
            wow_entities::PlayerQuestStatusRecord {
                quest_id: 300,
                status: 2,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: vec![7],
                slot: 1,
            },
        );
        state.set_objective_counts_for_quest_like_cpp(300, vec![7]);
        state.set_rewarded_like_cpp(400, true);
    });
    assert!(session.update_inventory_item_object_like_cpp(item_guid, |item| item.set_count(6)));
    session.sync_player_registry_state_like_cpp();

    let info = registry.loot_player_context(guid).expect("synced player");
    assert!(info.known_spells.contains(&54_321));
    assert_eq!(info.active_quest_statuses.get(&300), Some(&2));
    assert_eq!(info.active_quest_objective_counts.get(&300), Some(&vec![7]));
    assert!(info.rewarded_quests.contains(&400));
    assert_eq!(info.inventory_item_counts.get(&9001), Some(&8));
}
#[test]
fn active_loot_guid_tracks_cpp_loot_target_guid_comparisons() {
    let (mut session, _, _) = make_session();
    let loot_guid = ObjectGuid::create_item(1, 700);
    let other_guid = ObjectGuid::create_item(1, 701);

    assert!(!session.is_active_loot_guid(loot_guid));
    session.set_active_loot_guid(loot_guid);
    assert!(session.is_active_loot_guid(loot_guid));
    assert!(!session.is_active_loot_guid(other_guid));

    session.clear_active_loot_guid_if(other_guid);
    assert!(session.is_active_loot_guid(loot_guid));
    session.clear_active_loot_guid_if(loot_guid);
    assert!(!session.is_active_loot_guid(loot_guid));
}
