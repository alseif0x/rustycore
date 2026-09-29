use crate::handlers::character::ExtendedCostItemTurninChange;
use crate::handlers::test_support::world::make_session;
use crate::session::InventoryItem;
use crate::test_fixtures::{
    insert_inventory_item_for_test, insert_inventory_item_object_for_test,
    make_inventory_item_object_for_test,
};
use wow_constants::ItemContext;
use wow_core::ObjectGuid;

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

    assert!(session.has_item_count_direct_inventory(700, 9));
    assert!(!session.has_item_count_direct_inventory(700, 10));
    assert_eq!(
        session.plan_destroy_item_count_direct_inventory(700, 6),
        Some(vec![
            ExtendedCostItemTurninChange::Delete {
                slot: 35,
                item_guid: ObjectGuid::create_item(1, 10),
                db_guid: 10,
            },
            ExtendedCostItemTurninChange::Update {
                slot: 36,
                item_guid: ObjectGuid::create_item(1, 11),
                db_guid: 11,
                new_count: 3,
            },
        ])
    );
}
