//! Entity update bridge regression scenarios, part 2 of 2.
//!
//! Moved out of the entity_update_bridge.rs root under #662; every test is unchanged.

use super::*;

#[test]
fn bridges_canonical_unit_critter_guid_like_cpp() {
    let mut player = Player::new(Some(7), false);
    let critter = ObjectGuid::new(7, 12);
    let battle_pet = ObjectGuid::new(7, 13);
    player.clear_data_changes();
    player.unit_mut().set_critter_guid_like_cpp(Some(critter));
    player
        .unit_mut()
        .set_battle_pet_companion_guid_like_cpp(Some(battle_pet));
    player
        .unit_mut()
        .set_battle_pet_companion_name_timestamp_like_cpp(1234);

    let update = player.values_update(true);
    let packet_update = player_values_update_to_packet(&update).unwrap();
    let unit = packet_update.unit_data.unwrap();

    assert!(mask_has(
        &unit.unit_data_mask,
        wow_entities::UNIT_DATA_CRITTER_BIT
    ));
    assert_eq!(unit.critter, critter);
    assert!(mask_has(
        &unit.unit_data_mask,
        wow_entities::UNIT_DATA_BATTLE_PET_COMPANION_GUID_BIT
    ));
    assert!(mask_has(
        &unit.unit_data_mask,
        wow_entities::UNIT_DATA_BATTLE_PET_COMPANION_NAME_TIMESTAMP_BIT
    ));
    assert_eq!(unit.battle_pet_companion_guid, battle_pet);
    assert_eq!(unit.battle_pet_companion_name_timestamp, 1234);
}

#[test]
fn bridges_forced_default_value_deltas() {
    let mut player = Player::new(Some(7), false);
    player.clear_data_changes();
    player.mark_inv_slot_changed(0);
    player.mark_visible_item_slot_changed(0);
    player.unit_mut().mark_virtual_item_changed(0);

    let update = player.values_update(true);
    let packet_update = player_values_update_to_packet(&update).unwrap();
    let active = packet_update.active_player_data.unwrap();
    let unit = packet_update.unit_data.unwrap();

    assert!(mask_has(
        &active.active_player_data_mask,
        wow_entities::ACTIVE_PLAYER_DATA_INV_SLOTS_FIRST_BIT
    ));
    assert!(mask_has(
        &packet_update.player_data_mask,
        wow_entities::PLAYER_DATA_VISIBLE_ITEMS_FIRST_BIT
    ));
    assert!(mask_has(
        &unit.unit_data_mask,
        wow_entities::UNIT_DATA_VIRTUAL_ITEMS_FIRST_BIT
    ));
    assert_eq!(active.inv_slots[0], ObjectGuid::EMPTY);
    assert_eq!(packet_update.visible_items[0].item_id, 0);
    assert_eq!(unit.virtual_items[0].item_id, 0);
}
