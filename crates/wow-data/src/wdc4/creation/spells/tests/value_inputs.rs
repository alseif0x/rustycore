use super::*;
use crate::forever_spells::SpellRecords;
use crate::wdc4::Wdc4Reader;

pub(super) fn prepare(table: SpellTable, reader: &mut Wdc4Reader) {
    match table {
        SpellTable::ExpectedStat => {
            reader.relationship_ids[0] = Some(0x800000c0);
            cell(reader, 0, 0, 32, 0x80000010);
            cell(reader, 1, 0, 32, 0x7fc00020);
            cell(reader, 2, 0, 32, 0x7fc00030);
            cell(reader, 3, 0, 32, 0x7fc00040);
            cell(reader, 4, 0, 32, 0x7fc00050);
            cell(reader, 5, 0, 32, 0x7fc00060);
            cell(reader, 6, 0, 32, 0x7fc00070);
            cell(reader, 7, 0, 32, 0x7fc00080);
            cell(reader, 8, 0, 32, 0x7fc00090);
            cell(reader, 9, 0, 32, 0x7fc000a0);
            cell(reader, 10, 0, 32, 0x800000b0);
        }
        SpellTable::ExpectedStatMod => {
            cell(reader, 0, 0, 32, 0x7fc00010);
            cell(reader, 1, 0, 32, 0x7fc00020);
            cell(reader, 2, 0, 32, 0x7fc00030);
            cell(reader, 3, 0, 32, 0x7fc00040);
            cell(reader, 4, 0, 32, 0x7fc00050);
            cell(reader, 5, 0, 32, 0x7fc00060);
            cell(reader, 6, 0, 32, 0x7fc00070);
            cell(reader, 7, 0, 32, 0x7fc00080);
            cell(reader, 8, 0, 32, 0x7fc00090);
        }
        SpellTable::ContentTuning => {
            cell(reader, 1, 0, 32, 0x80000020);
            cell(reader, 2, 0, 32, 0x80000030);
            cell(reader, 3, 0, 32, 0x80000040);
            cell(reader, 4, 0, 32, 0x80000050);
            cell(reader, 5, 0, 32, 0x80000060);
            cell(reader, 6, 0, 32, 0x80000070);
            cell(reader, 7, 0, 32, 0x80000080);
            cell(reader, 8, 0, 32, 0x7fc00090);
            cell(reader, 9, 0, 32, 0x800000a0);
            cell(reader, 10, 0, 32, 0x800000b0);
            cell(reader, 11, 0, 32, 0x800000c0);
            cell(reader, 12, 0, 32, 0x800000d0);
            cell(reader, 13, 0, 32, 0x800000e0);
            cell(reader, 14, 0, 32, 0x800000f0);
            cell(reader, 15, 0, 32, 0x80000100);
            cell(reader, 16, 0, 32, 0x80000110);
            cell(reader, 17, 0, 32, 0x80000120);
            cell(reader, 18, 0, 32, 0x7fc00130);
        }
        SpellTable::ContentTuningXExpected => {
            reader.relationship_ids[0] = Some(0x80000040);
            cell(reader, 0, 0, 32, 0x80000010);
            cell(reader, 1, 0, 32, 0x80000020);
            cell(reader, 2, 0, 32, 0x80000030);
        }
        SpellTable::RandPropPoints => {
            cell(reader, 0, 0, 32, 0x7fc00010);
            cell(reader, 1, 0, 32, 0x7fc00020);
            cell(reader, 2, 0, 32, 0x80000030);
            cell(reader, 3, 0, 32, 0x80000040);
            cell(reader, 4, 0, 32, 0x7fc00050);
            cell(reader, 4, 1, 32, 0x7fc00051);
            cell(reader, 4, 2, 32, 0x7fc00052);
            cell(reader, 4, 3, 32, 0x7fc00053);
            cell(reader, 4, 4, 32, 0x7fc00054);
            cell(reader, 5, 0, 32, 0x7fc00060);
            cell(reader, 5, 1, 32, 0x7fc00061);
            cell(reader, 5, 2, 32, 0x7fc00062);
            cell(reader, 5, 3, 32, 0x7fc00063);
            cell(reader, 5, 4, 32, 0x7fc00064);
            cell(reader, 6, 0, 32, 0x7fc00070);
            cell(reader, 6, 1, 32, 0x7fc00071);
            cell(reader, 6, 2, 32, 0x7fc00072);
            cell(reader, 6, 3, 32, 0x7fc00073);
            cell(reader, 6, 4, 32, 0x7fc00074);
            cell(reader, 7, 0, 32, 0x80000080);
            cell(reader, 7, 1, 32, 0x80000081);
            cell(reader, 7, 2, 32, 0x80000082);
            cell(reader, 7, 3, 32, 0x80000083);
            cell(reader, 7, 4, 32, 0x80000084);
            cell(reader, 8, 0, 32, 0x80000090);
            cell(reader, 8, 1, 32, 0x80000091);
            cell(reader, 8, 2, 32, 0x80000092);
            cell(reader, 8, 3, 32, 0x80000093);
            cell(reader, 8, 4, 32, 0x80000094);
            cell(reader, 9, 0, 32, 0x800000a0);
            cell(reader, 9, 1, 32, 0x800000a1);
            cell(reader, 9, 2, 32, 0x800000a2);
            cell(reader, 9, 3, 32, 0x800000a3);
            cell(reader, 9, 4, 32, 0x800000a4);
        }
        SpellTable::MythicPlusSeason => {
            cell(reader, 1, 0, 32, 0x80000020);
            cell(reader, 2, 0, 32, 0x80000030);
            cell(reader, 3, 0, 32, 0x80000040);
            cell(reader, 4, 0, 32, 0x80000050);
        }
        _ => {}
    }
}

pub(super) fn assert_records(batch: &SpellRecords) {
    let row = &batch.expected_stats[0];
    assert_eq!(row.id, 7);
    assert_eq!(row.expansion_id as u32, 0x80000010);
    assert_eq!(row.creature_health.to_bits(), 0x7fc00020);
    assert_eq!(row.player_health.to_bits(), 0x7fc00030);
    assert_eq!(row.creature_auto_attack_dps.to_bits(), 0x7fc00040);
    assert_eq!(row.creature_armor.to_bits(), 0x7fc00050);
    assert_eq!(row.player_mana.to_bits(), 0x7fc00060);
    assert_eq!(row.player_primary_stat.to_bits(), 0x7fc00070);
    assert_eq!(row.player_secondary_stat.to_bits(), 0x7fc00080);
    assert_eq!(row.armor_constant.to_bits(), 0x7fc00090);
    assert_eq!(row.creature_spell_damage.to_bits(), 0x7fc000a0);
    assert_eq!(row.content_set_id as u32, 0x800000b0);
    assert_eq!(row.lvl, 0x800000c0);
    let row = &batch.expected_stat_mods[0];
    assert_eq!(row.id, 7);
    assert_eq!(row.creature_health_mod.to_bits(), 0x7fc00010);
    assert_eq!(row.player_health_mod.to_bits(), 0x7fc00020);
    assert_eq!(row.creature_auto_attack_dps_mod.to_bits(), 0x7fc00030);
    assert_eq!(row.creature_armor_mod.to_bits(), 0x7fc00040);
    assert_eq!(row.player_mana_mod.to_bits(), 0x7fc00050);
    assert_eq!(row.player_primary_stat_mod.to_bits(), 0x7fc00060);
    assert_eq!(row.player_secondary_stat_mod.to_bits(), 0x7fc00070);
    assert_eq!(row.armor_constant_mod.to_bits(), 0x7fc00080);
    assert_eq!(row.creature_spell_damage_mod.to_bits(), 0x7fc00090);
    let row = &batch.content_tunings[0];
    assert_eq!(row.id, 7);
    assert_eq!(row.flags as u32, 0x80000020);
    assert_eq!(row.expansion_id as u32, 0x80000030);
    assert_eq!(row.health_item_level_curve_id as u32, 0x80000040);
    assert_eq!(row.damage_item_level_curve_id as u32, 0x80000050);
    assert_eq!(row.health_primary_stat_curve_id as u32, 0x80000060);
    assert_eq!(row.damage_primary_stat_curve_id as u32, 0x80000070);
    assert_eq!(
        row.primary_stat_scaling_mod_player_data_element_character_id as u32,
        0x80000080
    );
    assert_eq!(
        row.primary_stat_scaling_mod_player_data_element_character_multiplier
            .to_bits(),
        0x7fc00090
    );
    assert_eq!(row.min_level as u32, 0x800000a0);
    assert_eq!(row.max_level as u32, 0x800000b0);
    assert_eq!(row.min_level_type as u32, 0x800000c0);
    assert_eq!(row.max_level_type as u32, 0x800000d0);
    assert_eq!(row.target_level_delta as u32, 0x800000e0);
    assert_eq!(row.target_level_max_delta as u32, 0x800000f0);
    assert_eq!(row.target_level_min as u32, 0x80000100);
    assert_eq!(row.target_level_max as u32, 0x80000110);
    assert_eq!(row.min_item_level as u32, 0x80000120);
    assert_eq!(row.quest_xp_multiplier.to_bits(), 0x7fc00130);
    let row = &batch.content_tuning_x_expected[0];
    assert_eq!(row.id, 7);
    assert_eq!(row.expected_stat_mod_id as u32, 0x80000010);
    assert_eq!(row.min_mythic_plus_season_id as u32, 0x80000020);
    assert_eq!(row.max_mythic_plus_season_id as u32, 0x80000030);
    assert_eq!(row.content_tuning_id, 0x80000040);
    let row = &batch.rand_prop_points[0];
    assert_eq!(row.id, 7);
    assert_eq!(row.damage_replace_stat_f.to_bits(), 0x7fc00010);
    assert_eq!(row.damage_secondary_f.to_bits(), 0x7fc00020);
    assert_eq!(row.damage_replace_stat as u32, 0x80000030);
    assert_eq!(row.damage_secondary as u32, 0x80000040);
    assert_eq!(row.epic_f[0].to_bits(), 0x7fc00050);
    assert_eq!(row.epic_f[1].to_bits(), 0x7fc00051);
    assert_eq!(row.epic_f[2].to_bits(), 0x7fc00052);
    assert_eq!(row.epic_f[3].to_bits(), 0x7fc00053);
    assert_eq!(row.epic_f[4].to_bits(), 0x7fc00054);
    assert_eq!(row.superior_f[0].to_bits(), 0x7fc00060);
    assert_eq!(row.superior_f[1].to_bits(), 0x7fc00061);
    assert_eq!(row.superior_f[2].to_bits(), 0x7fc00062);
    assert_eq!(row.superior_f[3].to_bits(), 0x7fc00063);
    assert_eq!(row.superior_f[4].to_bits(), 0x7fc00064);
    assert_eq!(row.good_f[0].to_bits(), 0x7fc00070);
    assert_eq!(row.good_f[1].to_bits(), 0x7fc00071);
    assert_eq!(row.good_f[2].to_bits(), 0x7fc00072);
    assert_eq!(row.good_f[3].to_bits(), 0x7fc00073);
    assert_eq!(row.good_f[4].to_bits(), 0x7fc00074);
    assert_eq!(row.epic[0], 0x80000080);
    assert_eq!(row.epic[1], 0x80000081);
    assert_eq!(row.epic[2], 0x80000082);
    assert_eq!(row.epic[3], 0x80000083);
    assert_eq!(row.epic[4], 0x80000084);
    assert_eq!(row.superior[0], 0x80000090);
    assert_eq!(row.superior[1], 0x80000091);
    assert_eq!(row.superior[2], 0x80000092);
    assert_eq!(row.superior[3], 0x80000093);
    assert_eq!(row.superior[4], 0x80000094);
    assert_eq!(row.good[0], 0x800000a0);
    assert_eq!(row.good[1], 0x800000a1);
    assert_eq!(row.good[2], 0x800000a2);
    assert_eq!(row.good[3], 0x800000a3);
    assert_eq!(row.good[4], 0x800000a4);
    let row = &batch.mythic_plus_seasons[0];
    assert_eq!(row.id, 7);
    assert_eq!(row.milestone_season as u32, 0x80000020);
    assert_eq!(row.start_time_event as u32, 0x80000030);
    assert_eq!(row.expansion_level as u32, 0x80000040);
    assert_eq!(row.heroic_lfg_dungeon_min_gear as u32, 0x80000050);
}
