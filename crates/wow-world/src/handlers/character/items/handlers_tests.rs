use crate::handlers::test_support::world::make_session;
use crate::session::{InventoryItem, WorldSession};
use crate::test_fixtures::{
    insert_inventory_item_for_test, insert_inventory_item_object_for_test,
    make_inventory_item_object_for_test,
};
use wow_constants::{EnchantmentSlot, InventoryType, ItemContext};
use wow_core::ObjectGuid;
use wow_packet::packets::item::CancelTempEnchantment;

fn insert_cancel_temp_enchant_test_item(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    slot: u8,
    enchantment_id: i32,
) -> ObjectGuid {
    let item_guid = ObjectGuid::create_item(1, 70_000 + i64::from(slot));
    let _ = insert_inventory_item_for_test(
        session,
        slot,
        InventoryItem {
            guid: item_guid,
            entry_id: 700,
            db_guid: item_guid.counter() as u64,
            inventory_type: Some(InventoryType::Weapon as u8),
        },
    );
    let mut item = make_inventory_item_object_for_test(
        session,
        item_guid,
        700,
        player_guid,
        1,
        0,
        ItemContext::None,
        slot,
    );
    item.set_enchantment(
        EnchantmentSlot::EnhancementTemporary,
        enchantment_id,
        12_000,
        3,
    );
    let _ = insert_inventory_item_object_for_test(session, item);
    item_guid
}

#[tokio::test]
async fn cancel_temp_enchantment_clears_equipped_temporary_enchant_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    let item_guid = insert_cancel_temp_enchant_test_item(&mut session, player_guid, 15, 901);

    session
        .handle_cancel_temp_enchantment(CancelTempEnchantment { slot: 15 })
        .await;

    let item = session
        .inventory_item_objects_like_cpp()
        .get(&item_guid)
        .unwrap();
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::EnhancementTemporary as usize].id,
        0
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn cancel_temp_enchantment_ignores_non_equipment_slot_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    let item_guid = insert_cancel_temp_enchant_test_item(&mut session, player_guid, 36, 902);

    session
        .handle_cancel_temp_enchantment(CancelTempEnchantment { slot: 36 })
        .await;

    let item = session
        .inventory_item_objects_like_cpp()
        .get(&item_guid)
        .unwrap();
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::EnhancementTemporary as usize].id,
        902
    );
    assert!(send_rx.try_recv().is_err());
}
