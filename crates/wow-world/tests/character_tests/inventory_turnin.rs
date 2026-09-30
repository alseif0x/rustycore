use super::*;
use super::inventory_runtime::make_session;

#[test]
fn extended_cost_item_turnin_plan_matches_cpp_destroy_order() {
    let (mut session, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 1);
    session.set_player_guid(Some(player_guid));

    for (slot, db_guid, count) in [(35, 10_u64, 4_u32), (36, 11_u64, 5_u32)] {
        let item_guid = ObjectGuid::create_item(1, db_guid as i64);
        let _ = insert_inventory_item_for_test(
            &mut session,
            slot,
            InventoryItem {
                guid: item_guid,
                entry_id: 700,
                db_guid,
                inventory_type: None,
            },
        );
        let item = make_inventory_item_object_for_test(
            &session,
            item_guid,
            700,
            player_guid,
            count,
            0,
            ItemContext::Vendor,
            slot,
        );
        let _ = insert_inventory_item_object_for_test(&mut session, item);
    }

    assert!(inventory_has_item_count_for_test(&session, 700, 9));
    assert!(!inventory_has_item_count_for_test(&session, 700, 10));
    assert_eq!(
        inventory_turnin_plan_for_test(&session, 700, 6),
        Some(InventoryTurninPlanForTest::new(vec![
            InventoryTurninChangeForTest::delete(35, ObjectGuid::create_item(1, 10), 10),
            InventoryTurninChangeForTest::update(36, ObjectGuid::create_item(1, 11), 11, 3),
        ]))
    );
}
