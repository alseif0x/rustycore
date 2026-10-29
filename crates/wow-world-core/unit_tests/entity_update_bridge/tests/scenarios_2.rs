//! Entity update bridge regression scenarios, part 2 of 2.
//!
//! Moved out of the entity_update_bridge.rs root under #662; every test is unchanged.

use super::*;

#[test]
fn native_death_zero_power_serializes_parent_and_mapped_element_even_for_noop() {
    for index in [0, 3] {
        for before in [0, 37] {
            let mut unit = wow_entities::Unit::new(true);
            unit.set_power_index(wow_constants::PowerType::Mana, Some(index));
            unit.set_max_power(wow_constants::PowerType::Mana, 100);
            unit.set_power(wow_constants::PowerType::Mana, before);
            unit.clear_unit_data_changes();
            unit.set_power_for_native_transition_like_cpp(wow_constants::PowerType::Mana, 0);
            let bytes =
                unit_values_update_to_update_object(ObjectGuid::EMPTY, 0, &unit.values_update())
                    .unwrap()
                    .to_bytes();
            let mut wire = wow_packet::WorldPacket::from_bytes(&bytes);
            wire.read_uint16().unwrap();
            assert_eq!(wire.read_uint32().unwrap(), 1);
            wire.read_uint16().unwrap();
            assert!(!wire.read_bit().unwrap());
            wire.reset_bits();
            wire.read_uint32().unwrap();
            assert_eq!(wire.read_uint8().unwrap(), 0);
            assert_eq!(wire.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
            assert_eq!(
                wire.read_uint32().unwrap(),
                17,
                "type mask, block mask, two field words and Power value"
            );
            assert_eq!(wire.read_uint32().unwrap(), 1 << wow_entities::TYPEID_UNIT);
            assert_eq!(wire.read_bits(8).unwrap(), 0x18, "UnitData words3/4");
            assert_eq!(wire.read_bits(32).unwrap(), 1 << 20, "Power parent116");
            assert_eq!(
                wire.read_bits(32).unwrap(),
                1 << (9 + index),
                "Power137 plus mapped index"
            );
            wire.reset_bits();
            assert_eq!(wire.read_int32().unwrap(), 0);
            assert_eq!(wire.read_position(), bytes.len());
        }
    }
}

#[test]
fn death_presentation_sober_and_flag_noops_serialize_exact_native_masks_and_values() {
    for changed in [false, true] {
        let mut player = Player::new(Some(7), false);
        player.replace_all_player_flags(0x8000 | if changed { 0x4000 } else { 0 });
        player.set_inebriation_like_cpp(if changed { 67 } else { 0 });
        if changed {
            player.unit_mut().set_emote_state_like_cpp(10);
            player
                .unit_mut()
                .set_stand_state_like_cpp(wow_constants::unit::UnitStandStateType::Sit);
        }
        player.clear_data_changes();
        player.unit_mut().reset_death_presentation_like_cpp();
        player.set_drunk_value_like_cpp(0, 0, false);
        player.remove_player_flags_like_cpp(0x4000);
        let update = player.values_update(true);
        let bytes = player_values_update_to_update_object(ObjectGuid::EMPTY, 0, &update)
            .unwrap()
            .to_bytes();
        let mut wire = wow_packet::WorldPacket::from_bytes(&bytes);
        wire.read_uint16().unwrap();
        assert_eq!(wire.read_uint32().unwrap(), 1);
        assert_eq!(wire.read_uint16().unwrap(), 0);
        assert!(!wire.read_bit().unwrap());
        wire.reset_bits();
        wire.read_uint32().unwrap();
        assert_eq!(wire.read_uint8().unwrap(), 0);
        assert_eq!(wire.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
        wire.read_uint32().unwrap();
        assert_eq!(
            wire.read_uint32().unwrap(),
            (1 << wow_entities::TYPEID_UNIT) | (1 << wow_entities::TYPEID_PLAYER)
        );
        assert_eq!(wire.read_bits(8).unwrap(), 6, "only UnitData words1/2");
        assert_eq!(wire.read_bits(32).unwrap(), 0x0100_0001, "parent32/Stand56");
        assert_eq!(wire.read_bits(32).unwrap(), 0x0000_0101, "parent64/Emote72");
        wire.reset_bits();
        assert_eq!(wire.read_uint8().unwrap(), 0);
        assert_eq!(wire.read_int32().unwrap(), 0);
        assert_eq!(wire.read_bits(4).unwrap(), 1, "only PlayerData word0");
        assert_eq!(
            wire.read_bits(32).unwrap(),
            0x4081,
            "parent0/Flags7/Inebriation14"
        );
        assert!(!wire.read_bit().unwrap(), "quest mask is not skipped");
        wire.reset_bits();
        assert_eq!(
            wire.read_uint32().unwrap(),
            0x8000,
            "unrelated Player flags are retained"
        );
        assert_eq!(wire.read_uint8().unwrap(), 0);
        assert_eq!(wire.read_position(), bytes.len());
    }
}

#[test]
fn removed_unit_flags_serialize_parent_mask_and_flags_even_for_noop() {
    use wow_constants::UnitFlags;
    for present in [false, true] {
        let mut unit = wow_entities::Unit::new(true);
        unit.set_unit_flags_like_cpp(
            UnitFlags::PLAYER_CONTROLLED
                | if present {
                    UnitFlags::SKINNABLE
                } else {
                    UnitFlags::empty()
                },
        );
        unit.clear_unit_data_changes();
        unit.remove_unit_flags_like_cpp(UnitFlags::SKINNABLE);
        let packet =
            unit_values_update_to_update_object(ObjectGuid::EMPTY, 0, &unit.values_update())
                .unwrap()
                .to_bytes();
        // The empty GUID puts UnitData at byte 24. C++ uses word 1, parent32/Flags41.
        assert_eq!(&packet[24..], &[2, 0, 0, 2, 1, 8, 0, 0, 0]);
    }
}

#[test]
fn bridges_canonical_local_flags_through_native_active_player_mask() {
    let mut player = Player::new(Some(7), false);
    player.gameplay_state_mut().active_local_flags = 0x8000;
    for set in [true, false] {
        player.clear_data_changes();
        if set {
            player.set_player_local_flags_like_cpp(8);
        } else {
            player.remove_player_local_flags_like_cpp(8);
        }
        let update = player.values_update(true);
        let projected = player_values_update_to_packet(&update).unwrap();
        assert_eq!(
            projected.changed_object_type_mask,
            1 << TYPEID_ACTIVE_PLAYER
        );
        let active = projected.active_player_data.unwrap();
        let flags = if set { 0x8008_u32 } else { 0x8000_u32 };
        assert_eq!(active.local_flags, flags);
        assert!(mask_has(&active.active_player_data_mask, 38));
        assert!(mask_has(&active.active_player_data_mask, 69));
        let packet = player_values_update_to_update_object(ObjectGuid::EMPTY, 0, &update)
            .unwrap()
            .to_bytes();
        assert_eq!(
            &packet[24..42],
            &[7, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0x40, 0, 0, 0, 0x20]
        );
        assert_eq!(&packet[42..], &flags.to_le_bytes());
    }
}

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
fn bridges_canonical_unit_minion_guid_and_empty_clear_like_cpp() {
    let mut player = Player::new(Some(7), false);
    let minion = ObjectGuid::new(7, 14);
    player.clear_data_changes();
    player.unit_mut().set_minion_guid_like_cpp(Some(minion));

    let update = player.values_update(true);
    let packet_update = player_values_update_to_packet(&update).unwrap();
    let unit = packet_update.unit_data.unwrap();
    assert!(mask_has(
        &unit.unit_data_mask,
        wow_entities::UNIT_DATA_SUMMON_BIT
    ));
    assert_eq!(unit.summon, minion);

    player.clear_data_changes();
    player.unit_mut().set_minion_guid_like_cpp(None);
    let update = player.values_update(true);
    let packet_update = player_values_update_to_packet(&update).unwrap();
    let unit = packet_update.unit_data.unwrap();
    assert!(mask_has(
        &unit.unit_data_mask,
        wow_entities::UNIT_DATA_SUMMON_BIT
    ));
    assert_eq!(unit.summon, ObjectGuid::EMPTY);
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
