//! Object update-block regressions, part 5 of 5.
//!
//! Moved out of the update_tests.rs root under #640; every test is unchanged.

use super::*;

#[test]
fn active_player_movement_block_adds_721_bytes() {
    // Self-view packets include a 721-byte ActivePlayer block in
    // BuildMovementUpdate: 1 byte (3 bits + flush) + 180 action buttons (720 bytes).
    // Non-self packets don't have this block.
    let guid = ObjectGuid::create_player(1, 42);
    let pos = Position::new(0.0, 0.0, 0.0, 0.0);
    let self_pkt = UpdateObject::create_player(
        guid,
        1,
        1,
        0,
        1,
        49,
        &pos,
        0,
        12,
        true,
        [(0, 0, 0); 19],
        [ObjectGuid::EMPTY; 141],
        PlayerCombatStats::default(),
        Vec::new(),
        0,
        Vec::new(),
    );
    let other_pkt = UpdateObject::create_player(
        guid,
        1,
        1,
        0,
        1,
        49,
        &pos,
        0,
        12,
        false,
        [(0, 0, 0); 19],
        [ObjectGuid::EMPTY; 141],
        PlayerCombatStats::default(),
        Vec::new(),
        0,
        Vec::new(),
    );
    let self_bytes = self_pkt.to_bytes();
    let other_bytes = other_pkt.to_bytes();

    // The difference between self and non-self should include:
    // - 721 bytes from ActivePlayer movement block
    // - plus the ActivePlayerData values block difference
    // The ActivePlayer movement block alone is 721 bytes.
    let diff = self_bytes.len() - other_bytes.len();
    assert!(
        diff > 721,
        "Self/non-self difference ({}) should be > 721 (ActivePlayer block)",
        diff
    );
}

#[test]
fn active_player_movement_block_writes_loaded_action_buttons_like_cpp() {
    let guid = ObjectGuid::create_player(1, 42);
    let pos = Position::new(0.0, 0.0, 0.0, 0.0);
    let mut pkt = UpdateObject::create_player(
        guid,
        1,
        1,
        0,
        1,
        49,
        &pos,
        0,
        12,
        true,
        [(0, 0, 0); 19],
        [ObjectGuid::EMPTY; 141],
        PlayerCombatStats::default(),
        Vec::new(),
        0,
        Vec::new(),
    );
    let sentinel = 0xA1B2_C3D4u32;
    let mut action_buttons = [0; MAX_ACTION_BUTTONS];
    action_buttons[17] = sentinel;
    pkt.set_player_action_buttons_like_cpp(action_buttons);

    let bytes = pkt.to_bytes();
    let sentinel_bytes = sentinel.to_le_bytes();
    assert!(
        bytes.windows(4).any(|window| window == sentinel_bytes),
        "C++ ActivePlayer movement block writes Player::m_actionButtons into self CREATE"
    );
}

#[test]
fn player_create_and_unit_values_publish_summon_and_empty_clear_like_cpp() {
    let guid = ObjectGuid::create_player(1, 42);
    let pos = Position::new(0.0, 0.0, 0.0, 0.0);
    let summon = ObjectGuid::new(0x0102_0304, 0x0506_0708);
    for is_self in [false, true] {
        let mut create = UpdateObject::create_player(
            guid,
            1,
            1,
            0,
            1,
            49,
            &pos,
            0,
            12,
            is_self,
            [(0, 0, 0); 19],
            [ObjectGuid::EMPTY; 141],
            PlayerCombatStats::default(),
            Vec::new(),
            0,
            Vec::new(),
        );
        create.set_player_summon_guid_like_cpp(summon);
        let UpdateBlock::CreateObject { create_data, .. } = &create.blocks[0] else {
            panic!("create_player should emit one CreateObject block");
        };
        assert_eq!(create_data.summon, summon);
        for flags in [0, 1] {
            let mut create_unit_data = WorldPacket::new_empty();
            create_data.write_unit_data(&mut create_unit_data, flags);
            let mut expected_summon = WorldPacket::new_empty();
            expected_summon.write_packed_guid(&summon);
            let start = 46;
            let end = start + expected_summon.data().len();
            assert_eq!(
                &create_unit_data.data()[start..end],
                expected_summon.data(),
                "UnitData::Summon follows the fixed prefix and Charm for flags {flags}"
            );
        }
    }

    for summon in [summon, ObjectGuid::EMPTY] {
        let mut update = UnitDataValuesDeltaUpdate::default();
        update.unit_data_mask[0] = (1 << 0) | (1 << 12);
        update.summon = summon;
        let mut bytes = WorldPacket::new_empty();
        write_full_unit_values_update_block(&mut bytes, &ObjectGuid::EMPTY, &update);
        let mut expected = WorldPacket::new_empty();
        expected.write_packed_guid(&summon);
        let expected = expected.into_data();
        let bytes = bytes.into_data();
        assert_eq!(bytes[0], UpdateType::Values as u8);
        assert_eq!(&bytes[1..3], &[0, 0]);
        assert_eq!(
            u32::from_le_bytes(bytes[3..7].try_into().unwrap()),
            (4 + 5 + expected.len()) as u32
        );
        assert_eq!(
            u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
            VALUES_TYPE_UNIT
        );
        assert_eq!(&bytes[11..16], &[0x01, 0x00, 0x00, 0x10, 0x01]);
        assert_eq!(&bytes[16..], expected.as_slice());
        assert_eq!(bytes.len(), 16 + expected.len());
    }
}
