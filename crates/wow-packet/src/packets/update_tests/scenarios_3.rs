//! Object update-block regressions, part 3 of 5.
//!
//! Moved out of the update_tests.rs root under #640; every test is unchanged.

use super::*;

#[test]
fn active_player_stats_values_update_matches_cpp_common_runtime_masks() {
    let mut combat_ratings = [0; 32];
    combat_ratings[0] = 11;
    combat_ratings[31] = 99;

    let stats = PlayerStatChanges {
        health: 0,
        max_health: 0,
        min_damage: 0.0,
        max_damage: 0.0,
        base_mana: 0,
        base_health: 0,
        attack_power: 0,
        attack_power_mod_pos: 0,
        attack_power_mod_neg: 0,
        attack_power_multiplier: 0.0,
        ranged_attack_power: 0,
        ranged_attack_power_mod_pos: 0,
        ranged_attack_power_mod_neg: 0,
        ranged_attack_power_multiplier: 0.0,
        min_ranged_damage: 0.0,
        max_ranged_damage: 0.0,
        power0: 0,
        max_power0: 0,
        stats: [0; 5],
        stat_pos_buff: [0; 5],
        stat_neg_buff: [0; 5],
        armor: 0,
        combat_ratings,
        spell_power: 123,
        block_pct: 1.0,
        dodge_pct: 2.0,
        parry_pct: 3.0,
        crit_pct: 4.0,
        ranged_crit_pct: 5.0,
        spell_crit_pct: [6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0],
        mana_regen: 0.0,
        mana_regen_combat: 0.0,
        mana_regen_mp5: 0.0,
        mainhand_expertise: 13.0,
        offhand_expertise: 14.0,
        ranged_expertise: 15.0,
        combat_rating_expertise: 16.0,
        dodge_from_attr: 17.0,
        parry_from_attr: 18.0,
        offhand_crit_pct: 19.0,
        shield_block: 20,
        shield_block_crit_pct: 21.0,
        mod_healing_pct: 1.0,
        mod_healing_done_pct: 1.0,
        mod_periodic_healing_pct: 1.0,
        mod_spell_power_pct: 1.0,
    };

    let mut values = WorldPacket::new_empty();
    write_active_player_data_values_update(&mut values, &[], &[], Some(&stats), None);

    let bytes = values.into_data();
    assert_eq!(&bytes[0..4], &[0x07, 0x01, 0x06, 0x00]); // blocks 0,1,2,8,17,18
    assert_eq!(&bytes[4..6], &[0x00, 0x00]);
    assert_eq!(&bytes[6..10], &[0x00, 0x00, 0x00, 0x01]);
    // block 1 = 0xFFFBFFF0: bits 4,5 + bits 6..31 EXCEPT bit 18 (field 50,
    // ShieldBlockCritPercentage, reserved in the 54261 client grammar).
    assert_eq!(&bytes[10..14], &[0xFF, 0xFB, 0xFF, 0xF0]);
    assert_eq!(&bytes[14..18], &[0x00, 0x00, 0x00, 0x3F]);
    assert_eq!(&bytes[18..22], &[0x0F, 0xFF, 0xE0, 0x00]);
    assert_eq!(&bytes[22..26], &[0xC0, 0x00, 0x00, 0x00]);
    assert_eq!(&bytes[26..30], &[0x7F, 0xFF, 0xFF, 0xFF]);

    let expertise = 13.0f32.to_le_bytes();
    let values_start = bytes
        .windows(4)
        .position(|window| window == expertise)
        .expect("mainhand expertise value must be present after ActivePlayerData masks");
    let mut offset = values_start;
    assert_eq!(
        f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()),
        13.0
    );
    offset += 4;
    assert_eq!(
        f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()),
        14.0
    );
    offset += 4;
    // Parent-38 section is 30 fields (bits 39-49, 51-69); field bit 50 is
    // reserved and not emitted, so skip 30 floats (not 31) to reach SpellCrit.
    offset += 30 * 4;

    for expected in [6.0f32, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0] {
        assert_eq!(
            f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()),
            expected
        );
        offset += 4;
        offset += 4; // ModDamageDonePos for the same school.
    }

    assert_eq!(
        i32::from_le_bytes(
            bytes[bytes.len() - 128..bytes.len() - 124]
                .try_into()
                .unwrap()
        ),
        11
    );
    assert_eq!(
        i32::from_le_bytes(bytes[bytes.len() - 4..].try_into().unwrap()),
        99
    );
}

#[test]
fn active_player_stats_values_update_omits_reserved_bit_50() {
    let mut stats = zeroed_stat_changes();
    stats.shield_block = 20; // field bit 49
    stats.shield_block_crit_pct = 21.0; // field bit 50 (must be dropped)

    let mut values = WorldPacket::new_empty();
    write_active_player_data_values_update(&mut values, &[], &[], Some(&stats), None);
    let bytes = values.into_data();

    // block 1 mask is serialized big-endian at bytes[10..14]. Field bit 50 =
    // block-1 bit 18 MUST be clear, while bit 49 (ShieldBlock) and bit 51
    // (Mastery) MUST be set — proving only the reserved hole is skipped.
    let block1 = u32::from_be_bytes(bytes[10..14].try_into().unwrap());
    assert_eq!((block1 >> 18) & 1, 0, "field bit 50 must NOT be masked");
    assert_eq!(
        (block1 >> 17) & 1,
        1,
        "field bit 49 (ShieldBlock) must be masked"
    );
    assert_eq!(
        (block1 >> 19) & 1,
        1,
        "field bit 51 (Mastery) must be masked"
    );
}

#[test]
fn unit_stats_values_update_writes_cpp_ap_modifiers_and_negative_stat_buffs() {
    let mut stats = zeroed_stat_changes();
    stats.health = 1;
    stats.max_health = 2;
    stats.min_damage = 3.0;
    stats.max_damage = 4.0;
    stats.base_mana = 5;
    stats.base_health = 6;
    stats.attack_power = 7;
    stats.attack_power_mod_pos = 8;
    stats.attack_power_mod_neg = 9;
    stats.attack_power_multiplier = 10.0;
    stats.ranged_attack_power = 11;
    stats.ranged_attack_power_mod_pos = 12;
    stats.ranged_attack_power_mod_neg = 13;
    stats.ranged_attack_power_multiplier = 14.0;
    stats.min_ranged_damage = 15.0;
    stats.max_ranged_damage = 16.0;
    stats.mana_regen = 17.0;
    stats.mana_regen_combat = 18.0;
    stats.power0 = 19;
    stats.max_power0 = 20;
    stats.mana_regen_mp5 = 21.0;
    stats.stats = [22, 25, 28, 31, 34];
    stats.stat_pos_buff = [23, 26, 29, 32, 35];
    stats.stat_neg_buff = [24, 27, 30, 33, 36];
    stats.armor = 37;

    let mut values = WorldPacket::new_empty();
    write_unit_data_values_update(&mut values, &[], Some(&stats));
    let mut values = WorldPacket::from_bytes(&values.into_data());

    assert_eq!(values.read_bits(8).unwrap(), 0x3F);
    assert_eq!(
        values.read_bits(32).unwrap(),
        (1 << 0) | (1 << 5) | (1 << 6)
    );
    assert_eq!(
        values.read_bits(32).unwrap(),
        (1 << 0) | (1 << 20) | (1 << 21)
    );
    assert_eq!(
        values.read_bits(32).unwrap(),
        (1 << 0)
            | (1 << 11)
            | (1 << 12)
            | (1 << 17)
            | (1 << 18)
            | (1 << 19)
            | (1 << 20)
            | (1 << 21)
            | (1 << 22)
            | (1 << 23)
            | (1 << 24)
            | (1 << 27)
            | (1 << 28)
    );
    assert_eq!(
        values.read_bits(32).unwrap(),
        (1 << 20) | (1 << 21) | (1 << 31)
    );
    assert_eq!(
        values.read_bits(32).unwrap(),
        (1 << 9) | (1 << 19) | (1 << 29)
    );
    assert_eq!(values.read_bits(32).unwrap(), u32::MAX << 14);
    values.reset_bits();

    assert_eq!(values.read_int64().unwrap(), 1);
    assert_eq!(values.read_int64().unwrap(), 2);
    assert_eq!(values.read_float().unwrap(), 3.0);
    assert_eq!(values.read_float().unwrap(), 4.0);
    for expected in [5, 6, 7, 8, 9] {
        assert_eq!(values.read_int32().unwrap(), expected);
    }
    assert_eq!(values.read_float().unwrap(), 10.0);
    for expected in [11, 12, 13] {
        assert_eq!(values.read_int32().unwrap(), expected);
    }
    assert_eq!(values.read_float().unwrap(), 14.0);
    for expected in [15.0, 16.0, 17.0, 18.0] {
        assert_eq!(values.read_float().unwrap(), expected);
    }
    assert_eq!(values.read_int32().unwrap(), 19);
    assert_eq!(values.read_int32().unwrap(), 20);
    assert_eq!(values.read_float().unwrap(), 21.0);
    for expected in 22..=36 {
        assert_eq!(values.read_int32().unwrap(), expected);
    }
    assert_eq!(values.read_int32().unwrap(), 37);
    assert_eq!(values.remaining(), 0);
}

#[test]
fn full_active_player_values_update_matches_cpp_coinage_shape() {
    let mut data = ActivePlayerDataValuesUpdate {
        coinage: 1234,
        ..Default::default()
    };
    set_active_player_bit(&mut data, 0);
    set_active_player_bit(&mut data, 28);

    let mut values = WorldPacket::new_empty();
    write_active_player_data_values_update_section(&mut values, &data);

    let bytes = values.into_data();
    assert_eq!(&bytes[0..4], &[0x01, 0x00, 0x00, 0x00]); // group 0: block 0
    assert_eq!(&bytes[4..6], &[0x00, 0x00]); // group 1: no blocks 32..47
    assert_eq!(&bytes[6..10], &[0x10, 0x00, 0x00, 0x01]); // block 0: bits 0 and 28
    assert_eq!(u64::from_le_bytes(bytes[10..18].try_into().unwrap()), 1234);
    assert_eq!(bytes.len(), 18);
}

#[test]
fn full_active_player_values_update_block_uses_active_player_type_mask() {
    let mut data = ActivePlayerDataValuesUpdate {
        coinage: 1234,
        ..Default::default()
    };
    set_active_player_bit(&mut data, 0);
    set_active_player_bit(&mut data, 28);

    let mut block = WorldPacket::new_empty();
    write_full_active_player_values_update_block(&mut block, &ObjectGuid::EMPTY, &data);

    let bytes = block.into_data();
    assert_eq!(bytes[0], UpdateType::Values as u8);
    assert_eq!(&bytes[1..3], &[0, 0]);
    assert_eq!(
        u32::from_le_bytes(bytes[7..11].try_into().unwrap()),
        VALUES_TYPE_ACTIVE_PLAYER
    );
    assert_eq!(u64::from_le_bytes(bytes[21..29].try_into().unwrap()), 1234);
}

#[test]
fn full_active_player_values_update_matches_cpp_late_array_order() {
    let mut data = ActivePlayerDataValuesUpdate::default();
    set_active_player_bit(&mut data, 636);
    set_active_player_bit(&mut data, 637);
    set_active_player_bit(&mut data, 1512);
    set_active_player_bit(&mut data, 1513);
    set_active_player_bit(&mut data, 1519);
    set_active_player_bit(&mut data, 607);
    set_active_player_bit(&mut data, 608);
    data.quest_completed[0] = 0x0102_0304_0506_0708;
    data.glyph_slots[0] = 55;
    data.glyphs[0] = 66;
    data.pvp_info[0] = PvpInfoValuesUpdate {
        pvp_info_mask: 0x0D,
        bracket: 7,
        pvp_rating_id: 99,
        ..Default::default()
    };

    let mut values = WorldPacket::new_empty();
    write_active_player_data_values_update_section(&mut values, &data);

    let bytes = values.into_data();
    let quest_pos = bytes
        .windows(8)
        .position(|window| window == 0x0102_0304_0506_0708u64.to_le_bytes())
        .expect("QuestCompleted value must be present");
    let glyph_slot_pos = bytes
        .windows(4)
        .position(|window| window == 55u32.to_le_bytes())
        .expect("GlyphSlots value must be present");
    let glyph_pos = bytes
        .windows(4)
        .position(|window| window == 66u32.to_le_bytes())
        .expect("Glyphs value must be present");
    let pvp_pos = bytes
        .windows(4)
        .position(|window| window == 99i32.to_le_bytes())
        .expect("PVP rating value must be present");

    assert!(quest_pos < glyph_slot_pos);
    assert!(glyph_slot_pos < glyph_pos);
    assert!(glyph_pos < pvp_pos);
}

#[test]
fn skill_info_values_update_matches_cpp_mask_and_value_order() {
    let mut data = SkillInfoValuesUpdate::default();
    data.skill_info_mask[0] = (1 << 0) | (1 << 1);
    data.skill_info_mask[16] = 1 << 1; // global bit 513: SkillRank[0]
    data.skill_line_id[0] = 164;
    data.skill_rank[0] = 75;

    let mut values = WorldPacket::new_empty();
    write_skill_info_values_update(&mut values, &data);

    let bytes = values.into_data();
    assert_eq!(&bytes[0..4], &[0x01, 0x00, 0x01, 0x00]); // blocks 0 and 16
    assert_eq!(
        u16::from_le_bytes(bytes[bytes.len() - 4..bytes.len() - 2].try_into().unwrap()),
        164
    );
    assert_eq!(
        u16::from_le_bytes(bytes[bytes.len() - 2..].try_into().unwrap()),
        75
    );
}

#[test]
fn active_player_nested_simple_values_update_match_cpp_order() {
    let mut research = WorldPacket::new_empty();
    write_research_values_update(
        &mut research,
        ResearchValuesUpdate {
            research_project_id: -123,
        },
    );
    assert_eq!(
        i16::from_le_bytes(research.into_data().try_into().unwrap()),
        -123
    );

    let mut rest = WorldPacket::new_empty();
    write_rest_info_values_update(
        &mut rest,
        RestInfoValuesUpdate {
            rest_info_mask: 0x07,
            threshold: 10_000,
            state_id: 3,
        },
    );
    let rest_bytes = rest.into_data();
    assert_eq!(rest_bytes[0] & 0xE0, 0xE0); // 3-bit mask 0b111
    assert_eq!(
        u32::from_le_bytes(rest_bytes[1..5].try_into().unwrap()),
        10_000
    );
    assert_eq!(rest_bytes[5], 3);

    let mut pvp = WorldPacket::new_empty();
    write_pvp_info_values_update(
        &mut pvp,
        PvpInfoValuesUpdate {
            pvp_info_mask: 0x0F,
            disqualified: true,
            bracket: -1,
            pvp_rating_id: 42,
            ..Default::default()
        },
    );
    let pvp_bytes = pvp.into_data();
    assert_eq!(pvp_bytes[pvp_bytes.len() - 5], 0xFFu8); // Bracket i8
    assert_eq!(
        i32::from_le_bytes(pvp_bytes[pvp_bytes.len() - 4..].try_into().unwrap()),
        42
    );
}

#[test]
fn active_player_dynamic_entry_values_update_match_cpp_order() {
    let mut restriction = WorldPacket::new_empty();
    write_character_restriction_values_update(
        &mut restriction,
        CharacterRestrictionValuesUpdate {
            field_0: 1,
            field_4: 2,
            field_8: 3,
            restriction_type: 17,
        },
    );
    let restriction_bytes = restriction.into_data();
    assert_eq!(
        i32::from_le_bytes(restriction_bytes[0..4].try_into().unwrap()),
        1
    );
    assert_eq!(
        i32::from_le_bytes(restriction_bytes[4..8].try_into().unwrap()),
        2
    );
    assert_eq!(
        i32::from_le_bytes(restriction_bytes[8..12].try_into().unwrap()),
        3
    );
    assert_eq!(restriction_bytes[12] & 0xF8, 0x88); // 5-bit type 17.

    let mut pct = WorldPacket::new_empty();
    write_spell_pct_mod_by_label_values_update(
        &mut pct,
        SpellPctModByLabelValuesUpdate {
            mod_index: 4,
            modifier_value: 1.5,
            label_id: 6,
        },
    );
    let pct_bytes = pct.into_data();
    assert_eq!(i32::from_le_bytes(pct_bytes[0..4].try_into().unwrap()), 4);
    assert_eq!(f32::from_le_bytes(pct_bytes[4..8].try_into().unwrap()), 1.5);
    assert_eq!(i32::from_le_bytes(pct_bytes[8..12].try_into().unwrap()), 6);

    let mut flat = WorldPacket::new_empty();
    write_spell_flat_mod_by_label_values_update(
        &mut flat,
        SpellFlatModByLabelValuesUpdate {
            mod_index: 7,
            modifier_value: 8,
            label_id: 9,
        },
    );
    assert_eq!(flat.into_data(), [7, 0, 0, 0, 8, 0, 0, 0, 9, 0, 0, 0]);

    let mut cooldown = WorldPacket::new_empty();
    write_category_cooldown_mod_values_update(
        &mut cooldown,
        CategoryCooldownModValuesUpdate {
            spell_category_id: 10,
            mod_cooldown: 11,
        },
    );
    assert_eq!(cooldown.into_data(), [10, 0, 0, 0, 11, 0, 0, 0]);

    let mut weekly = WorldPacket::new_empty();
    write_weekly_spell_use_values_update(
        &mut weekly,
        WeeklySpellUseValuesUpdate {
            spell_category_id: 12,
            uses: 13,
        },
    );
    assert_eq!(weekly.into_data(), [12, 0, 0, 0, 13]);
}

#[test]
fn active_player_dynamic_nested_values_update_match_cpp_order() {
    let mut research_history = WorldPacket::new_empty();
    write_research_history_values_update(
        &mut research_history,
        &ResearchHistoryValuesUpdate {
            research_history_mask: 0x03,
            completed_projects: vec![CompletedProjectValuesUpdate {
                completed_project_mask: 0x0F,
                project_id: 101,
                first_completed: 202,
                completion_count: 3,
            }],
            completed_projects_update_mask: None,
        },
    );
    let rh = research_history.into_data();
    assert_eq!(
        u32::from_le_bytes(rh[rh.len() - 16..rh.len() - 12].try_into().unwrap()),
        101
    );
    assert_eq!(
        i64::from_le_bytes(rh[rh.len() - 12..rh.len() - 4].try_into().unwrap()),
        202
    );
    assert_eq!(
        u32::from_le_bytes(rh[rh.len() - 4..].try_into().unwrap()),
        3
    );

    let mut trait_config = WorldPacket::new_empty();
    write_trait_config_values_update(
        &mut trait_config,
        &TraitConfigValuesUpdate {
            trait_config_mask: 0x07F,
            entries: vec![TraitEntryValuesUpdate {
                trait_node_id: 1,
                trait_node_entry_id: 2,
                rank: 3,
                granted_ranks: 4,
            }],
            entries_update_mask: None,
            id: 55,
            name: "Spec".to_string(),
            config_type: 2,
            skill_line_id: 777,
            ..Default::default()
        },
    );
    let tc = trait_config.into_data();
    assert!(tc.windows(4).any(|window| window == [1, 0, 0, 0]));
    assert!(tc.windows(4).any(|window| window == [55, 0, 0, 0]));
    assert!(tc.windows(4).any(|window| window == 777u32.to_le_bytes()));
    assert!(tc.windows(4).any(|window| window == b"Spec"));

    let mut stable = WorldPacket::new_empty();
    write_stable_info_values_update(
        &mut stable,
        &StableInfoValuesUpdate {
            stable_info_mask: 0x07,
            pets: vec![StablePetInfoValuesUpdate {
                stable_pet_mask: 0xFF,
                pet_slot: 1,
                pet_number: 2,
                creature_id: 3,
                display_id: 4,
                experience_level: 5,
                name: "Pet".to_string(),
                pet_flags: 6,
            }],
            pets_update_mask: None,
            stable_master: ObjectGuid::EMPTY,
        },
    );
    let stable_bytes = stable.into_data();
    assert!(stable_bytes.windows(4).any(|window| window == [1, 0, 0, 0]));
    assert!(stable_bytes.windows(4).any(|window| window == [5, 0, 0, 0]));
    assert!(stable_bytes.windows(3).any(|window| window == b"Pet"));
}

#[test]
fn player_create_writes_party_type_like_cpp() {
    let mut create = test_player_create_data_with_farsight(ObjectGuid::EMPTY);
    create.party_type = [17, 23];

    let mut packet = WorldPacket::new_empty();
    create.write_player_data(&mut packet, 0x03);

    assert!(
        packet
            .data()
            .windows(4)
            .any(|window| window == [17, 23, 0, create.sex]),
        "PlayerData::PartyType[2] must be serialized before NumBankSlots/NativeSex"
    );
}

#[test]
fn player_create_writes_loaded_player_flags_like_cpp() {
    let mut create = test_player_create_data_with_farsight(ObjectGuid::EMPTY);
    create.player_flags = 0x22;
    create.player_flags_ex = 0x04;

    let mut packet = WorldPacket::new_empty();
    create.write_player_data(&mut packet, 0x03);

    assert!(
        packet
            .data()
            .windows(8)
            .any(|window| window == [0x22, 0, 0, 0, 0x04, 0, 0, 0]),
        "C++ PlayerData::WriteCreate serializes loaded PlayerFlags/PlayerFlagsEx"
    );
}

#[test]
fn player_create_playerdata_self_layout_ends_with_dungeon_score_like_cpp() {
    let create = test_player_create_data_with_farsight(ObjectGuid::EMPTY);

    let mut packet = WorldPacket::new_empty();
    create.write_player_data(&mut packet, 0x03);
    let bytes = packet.data();

    // C++ `UF::PlayerData::WriteCreate` with empty dynamic arrays and
    // self-view PartyMember fields:
    // - fixed header/account/flags/customization/party fields: 50 bytes
    // - QuestLog[25]: 25 * (i64 + i32 + u32 + 24*u16) = 1600 bytes
    // - VisibleItems[19]: 19 * (i32 + u16 + u16) = 152 bytes
    // - fixed tail through Field_3120[19]: 147 bytes
    // - DungeonScoreSummary: f32 + f32 + u32 = 12 bytes
    const EMPTY_SELF_PLAYER_DATA_LEN: usize = 50 + 1600 + 152 + 147 + 12;
    assert_eq!(bytes.len(), EMPTY_SELF_PLAYER_DATA_LEN);

    let dungeon_score = &bytes[bytes.len() - 12..];
    assert_eq!(
        dungeon_score, &[0; 12],
        "empty C++ DungeonScoreSummary is two zero f32 values plus zero Runs count"
    );
}

#[test]
fn player_create_unitdata_owner_layout_matches_cpp_field_count() {
    let mut create = test_player_create_data_with_farsight(ObjectGuid::EMPTY);
    create.visible_items[15] = (0x0102_0304, 0x0506, 0x0708);

    let mut packet = WorldPacket::new_empty();
    create.write_unit_data(&mut packet, 0x01);
    let bytes = packet.data();

    // C++ `UF::UnitData::WriteCreate` with `UpdateFieldFlag::Owner`
    // and empty dynamic arrays/packed GUIDs serializes 823 bytes.
    // This covers the shared player/creature UnitData create layout.
    const OWNER_UNIT_DATA_LEN: usize = 823;
    assert_eq!(bytes.len(), OWNER_UNIT_DATA_LEN);

    assert!(
        bytes
            .windows(8)
            .any(|window| window == [0x04, 0x03, 0x02, 0x01, 0x06, 0x05, 0x08, 0x07]),
        "C++ VisibleItem/VirtualItems order is ItemID(i32), AppearanceMod(u16), ItemVisual(u16)"
    );
}

#[test]
fn player_create_writes_customizations_like_cpp() {
    let without_customizations = test_player_create_data_with_farsight(ObjectGuid::EMPTY);
    let mut with_customizations = test_player_create_data_with_farsight(ObjectGuid::EMPTY);
    with_customizations.customizations = vec![
        ChrCustomizationChoiceValuesUpdate {
            option_id: 110,
            choice_id: 17913,
        },
        ChrCustomizationChoiceValuesUpdate {
            option_id: 111,
            choice_id: 17929,
        },
    ];

    let mut base_packet = WorldPacket::new_empty();
    without_customizations.write_player_data(&mut base_packet, 0x03);
    let base_len = base_packet.data().len();

    let mut packet = WorldPacket::new_empty();
    with_customizations.write_player_data(&mut packet, 0x03);
    let bytes = packet.data();

    assert_eq!(bytes.len(), base_len + 16);
    assert!(
        bytes
            .windows(8)
            .any(|window| window == [110, 0, 0, 0, 249, 69, 0, 0]),
        "PlayerData::Customizations must serialize option/choice uint32 pairs"
    );
    assert!(
        bytes
            .windows(8)
            .any(|window| window == [111, 0, 0, 0, 9, 70, 0, 0]),
        "PlayerData::Customizations must preserve DB order"
    );
}

#[test]
fn player_create_writes_account_guids_like_cpp() {
    let without_account_guids = test_player_create_data_with_farsight(ObjectGuid::EMPTY);
    let mut with_account_guids = test_player_create_data_with_farsight(ObjectGuid::EMPTY);
    with_account_guids.wow_account =
        ObjectGuid::create_global(wow_core::guid::HighGuid::WowAccount, 0, 1);
    with_account_guids.bnet_account =
        ObjectGuid::create_global(wow_core::guid::HighGuid::BNetAccount, 0, 1);

    let mut base_packet = WorldPacket::new_empty();
    without_account_guids.write_player_data(&mut base_packet, 0x03);
    let base_len = base_packet.data().len();

    let mut packet = WorldPacket::new_empty();
    with_account_guids.write_player_data(&mut packet, 0x03);

    assert_eq!(packet.data().len(), base_len + 4);
}

#[test]
fn active_player_create_writes_farsight_after_inventory_slots() {
    let farsight_object = ObjectGuid::new(0x0102_0304_0506_0708, 0x1112_1314_1516_1718);
    let create = test_player_create_data_with_farsight(farsight_object);
    let mut packet = WorldPacket::new_empty();
    create.write_active_player_data(&mut packet);
    let data = packet.data();

    let mut expected_guid = WorldPacket::new_empty();
    expected_guid.write_packed_guid(&farsight_object);
    let expected_guid = expected_guid.into_data();
    let farsight_offset = 141 * 2;
    let summoned_battle_pet_offset = farsight_offset + expected_guid.len();

    assert_ne!(expected_guid, [0, 0]);
    assert_eq!(
        &data[farsight_offset..summoned_battle_pet_offset],
        expected_guid.as_slice()
    );
    assert_eq!(
        &data[summoned_battle_pet_offset..summoned_battle_pet_offset + 2],
        [0, 0]
    );
}

#[test]
fn active_player_create_empty_layout_matches_cpp_trace_offsets() {
    let create = test_player_create_data_with_farsight(ObjectGuid::EMPTY);
    let mut packet = WorldPacket::new_empty();
    create.write_active_player_data(&mut packet);
    let data = packet.data();

    // These offsets mirror the trace labels in C++
    // `UF::ActivePlayerData::WriteCreate` for an empty/default player.
    const INV_SLOTS_END: usize = 141 * 2;
    const FARSIGHT_BATTLEPET_END: usize = INV_SLOTS_END + 2 + 2;
    const SKILL_END: usize = FARSIGHT_BATTLEPET_END + 4 + 8 + 4 + 4 + 4 + (256 * 14);
    const EXPLORED_ZONES_END: usize = 6034;
    const BUYBACK_END: usize = 6268;
    const COMBAT_RATINGS_END: usize = 6444;
    const QUEST_COMPLETED_END: usize = 13561;
    const DYNAMIC_SIZES_END: usize = 13721;
    const PVP_INFO_END: usize = 14183;
    const RESEARCH_HISTORY_END: usize = 14188;
    const FROZEN_PERKS_END: usize = 14229;

    assert_eq!(data.len(), FROZEN_PERKS_END);
    assert_eq!(FARSIGHT_BATTLEPET_END, 286);
    assert_eq!(SKILL_END, 3894);
    assert_eq!(&data[INV_SLOTS_END..FARSIGHT_BATTLEPET_END], &[0, 0, 0, 0]);
    assert_eq!(&data[SKILL_END - 14..SKILL_END], &[0; 14]);
    assert_eq!(&data[EXPLORED_ZONES_END - 8..EXPLORED_ZONES_END], &[0; 8]);
    assert_eq!(&data[BUYBACK_END - 8..BUYBACK_END], &[0; 8]);
    assert_eq!(&data[COMBAT_RATINGS_END - 4..COMBAT_RATINGS_END], &[0; 4]);
    assert_eq!(&data[QUEST_COMPLETED_END - 8..QUEST_COMPLETED_END], &[0; 8]);
    assert_eq!(&data[DYNAMIC_SIZES_END - 1..DYNAMIC_SIZES_END], &[0]);
    assert_eq!(&data[PVP_INFO_END - 1..PVP_INFO_END], &[0]);
    assert_eq!(
        &data[RESEARCH_HISTORY_END - 4..RESEARCH_HISTORY_END],
        &[0; 4]
    );
    assert_eq!(&data[FROZEN_PERKS_END - 9..FROZEN_PERKS_END], &[0; 9]);
}

#[test]
fn active_player_create_writes_collection_dynamic_fields_like_cpp() {
    let mut create = test_player_create_data_with_farsight(ObjectGuid::EMPTY);
    create.heirlooms = vec![44_000, 44_001];
    create.heirloom_flags = vec![0x03, 0x04];
    create.toys = vec![30_000];
    create.transmog = vec![0x2000_0000, 0, 0x01];

    let mut packet = WorldPacket::new_empty();
    create.write_active_player_data(&mut packet);
    let data = packet.data();

    assert!(data.windows(16).any(|window| {
        window
            == [
                2, 0, 0, 0, // Heirlooms.Size
                2, 0, 0, 0, // HeirloomFlags.Size
                1, 0, 0, 0, // Toys.Size
                3, 0, 0, 0, // Transmog.Size
            ]
    }));
    assert!(
        data.windows(12)
            .any(|window| window == [224, 171, 0, 0, 225, 171, 0, 0, 3, 0, 0, 0])
    );
    assert!(
        data.windows(8)
            .any(|window| window == [4, 0, 0, 0, 48, 117, 0, 0])
    );
    assert!(
        data.windows(12)
            .any(|window| window == [0, 0, 0, 32, 0, 0, 0, 0, 1, 0, 0, 0])
    );
}

#[test]
fn active_player_create_writes_cpp_transmog_and_trait_config_dynamic_payloads() {
    let mut baseline = test_player_create_data_with_farsight(ObjectGuid::EMPTY);
    let mut packet = WorldPacket::new_empty();
    baseline.write_active_player_data(&mut packet);
    let baseline_len = packet.data().len();

    baseline.transmog = vec![0; 5528];
    baseline.trait_configs = vec![
        TraitConfigCreateData {
            id: 1,
            config_type: 1,
            skill_line_id: 0,
            chr_specialization_id: 256,
            combat_config_flags: 1,
            local_identifier: 1,
            trait_system_id: 0,
            name: String::new(),
            entries: Vec::new(),
        },
        TraitConfigCreateData {
            id: 2,
            config_type: 1,
            skill_line_id: 0,
            chr_specialization_id: 257,
            combat_config_flags: 1,
            local_identifier: 1,
            trait_system_id: 0,
            name: String::new(),
            entries: Vec::new(),
        },
        TraitConfigCreateData {
            id: 3,
            config_type: 1,
            skill_line_id: 0,
            chr_specialization_id: 258,
            combat_config_flags: 1,
            local_identifier: 1,
            trait_system_id: 0,
            name: String::new(),
            entries: Vec::new(),
        },
    ];

    let mut packet = WorldPacket::new_empty();
    baseline.write_active_player_data(&mut packet);
    let data = packet.data();

    assert_eq!(data.len() - baseline_len, 5528 * 4 + 3 * 26);
}

#[test]
fn create_player_defaults_farsight_object_empty() {
    let guid = ObjectGuid::create_player(1, 42);
    let pos = Position::new(0.0, 0.0, 0.0, 0.0);
    let packet = UpdateObject::create_player(
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

    let UpdateBlock::CreateObject { create_data, .. } = &packet.blocks[0] else {
        panic!("create_player should emit one CreateObject block");
    };
    assert_eq!(create_data.farsight_object, ObjectGuid::EMPTY);
}
