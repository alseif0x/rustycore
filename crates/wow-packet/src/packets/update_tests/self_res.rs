//! Self-res dynamic-field projection and packet encoding.

use super::*;

#[test]
fn active_player_create_writes_self_res_spell_sizes_and_values() {
    let mut create = test_player_create_data_with_farsight(ObjectGuid::EMPTY);
    create.self_res_spells = vec![3026, -21169, 3026];

    let mut packet = WorldPacket::new_empty();
    create.write_active_player_data(&mut packet);
    let data = packet.data();

    assert!(data.windows(12).any(|window| {
        window
            == [
                0, 0, 0, 0, // ConditionalTransmog.Size
                3, 0, 0, 0, // SelfResSpells.Size
                0, 0, 0, 0, // CharacterRestrictions.Size
            ]
    }));
    assert!(data.windows(12).any(|window| {
        window
            == [
                0xD2, 0x0B, 0, 0, // 3026
                0x4F, 0xAD, 0xFF, 0xFF, // -21169
                0xD2, 0x0B, 0, 0, // duplicate 3026
            ]
    }));
}

#[test]
fn active_player_self_res_values_update_writes_nested_mask_and_selected_values() {
    let mut data = ActivePlayerDataValuesUpdate::default();
    set_active_player_bit(&mut data, 0);
    set_active_player_bit(&mut data, 12);
    data.self_res_spells = vec![3026, 21169, 3026];
    data.self_res_spells_update_mask = Some(vec![0b101]);

    let mut packet = WorldPacket::new_empty();
    write_active_player_data_values_update_section(&mut packet, &data);

    assert_eq!(
        packet.into_data(),
        [
            1, 0, 0, 0, // Active block group mask
            0, 0, // ActivePlayerData block groups 32..47
            0, 0, 16, 1, // ActivePlayerData block 0 mask (bits 0 and 12)
            0, 0, 0, 3, 0xA0, // SelfResSpells size and changed indices 0/2
            0xD2, 0x0B, 0, 0, // 3026
            0xD2, 0x0B, 0, 0, // duplicate 3026; index 1 is omitted
        ]
    );
}

#[test]
fn active_player_create_writes_native_no_reagent_mask() {
    let mut create = test_player_create_data_with_farsight(ObjectGuid::EMPTY);
    create.no_reagent_cost_mask = [0x0102_0304, 0x1122_3344, 0, u32::MAX];

    let mut packet = WorldPacket::new_empty();
    create.write_active_player_data(&mut packet);
    assert!(packet.data().windows(16).any(|window| {
        window
            == [
                4, 3, 2, 1, 0x44, 0x33, 0x22, 0x11, 0, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF,
            ]
    }));
}

#[test]
fn active_player_values_update_projects_no_reagent_mask_children() {
    let mut data = ActivePlayerDataValuesUpdate::default();
    for bit in [0, 615, 616, 618, 619] {
        set_active_player_bit(&mut data, bit);
    }
    data.no_reagent_cost_mask = [0xAABB_CCDD, 2, 0x1122_3344, 4];

    let mut packet = WorldPacket::new_empty();
    write_active_player_data_values_update_section(&mut packet, &data);
    let bytes = packet.into_data();
    assert!(
        bytes
            .windows(8)
            .any(|window| window == [0xDD, 0xCC, 0xBB, 0xAA, 0x44, 0x33, 0x22, 0x11])
    );
    assert!(bytes.windows(4).any(|window| window == [4, 0, 0, 0]));
    assert!(!bytes.windows(4).any(|window| window == [2, 0, 0, 0]));
}

#[test]
fn self_res_create_setter_does_not_populate_non_owner_player_create() {
    let mut packet = UpdateObject::create_player(
        ObjectGuid::create_player(1, 42),
        1,
        1,
        0,
        1,
        49,
        &wow_core::Position::ZERO,
        571,
        1,
        false,
        [(0, 0, 0); 19],
        [ObjectGuid::EMPTY; 141],
        PlayerCombatStats::default(),
        Vec::new(),
        0,
        Vec::new(),
    );
    let baseline = packet.to_bytes();

    packet.set_player_self_res_spells(vec![3026, 21169]);

    assert_eq!(packet.to_bytes(), baseline);
}
