//! Composition-only SQL DTO -> target numeric records. No retained raw mirror.
use wow_data::forever_birth::{
    BirthRecords, LoadoutItemRecord, LoadoutRecord, SkillAbilityRecord, SkillLineRecord,
    SkillRaceClassRecord,
};
use wow_persistence::forever::birth::BirthRows;

fn mask(words: [i32; 2]) -> u64 {
    u64::from(words[0] as u32) | (u64::from(words[1] as u32) << 32)
}

pub(super) fn records(rows: BirthRows) -> BirthRecords {
    BirthRecords {
        skill_lines: rows
            .skill_lines
            .into_iter()
            .map(|row| SkillLineRecord {
                id: row.id,
                category: row.category,
                spell_icon_file: row.spell_icon_file,
                can_link: row.can_link,
                parent_skill: row.parent_skill,
                parent_tier_index: row.parent_tier_index,
                flags: row.flags,
                spell_book_spell: row.spell_book_spell,
                expansion_name_shared_string: row.expansion_name_shared_string,
                horde_expansion_name_shared_string: row.horde_expansion_name_shared_string,
            })
            .collect(),
        race_class: rows
            .race_class
            .into_iter()
            .map(|row| SkillRaceClassRecord {
                id: row.id,
                skill: row.skill,
                class_mask: row.class_mask,
                flags: row.flags,
                availability: row.availability,
                min_level: row.min_level,
                tier: row.tier,
                race_mask: mask(row.race_mask),
            })
            .collect(),
        abilities: rows
            .abilities
            .into_iter()
            .map(|row| SkillAbilityRecord {
                id: row.id,
                skill_line: row.skill_line,
                spell: row.spell,
                min_skill_rank: row.min_skill_rank,
                class_mask: row.class_mask,
                supercedes_spell: row.supercedes_spell,
                acquire_method: row.acquire_method,
                trivial_rank_high: row.trivial_rank_high,
                trivial_rank_low: row.trivial_rank_low,
                flags: row.flags,
                num_skill_ups: row.num_skill_ups,
                unique_bit: row.unique_bit,
                trade_skill_category: row.trade_skill_category,
                skillup_skill_line: row.skillup_skill_line,
                field_5_5_4_67090_014: row.field_5_5_4_67090_014,
                race_mask: mask(row.race_mask),
            })
            .collect(),
        loadouts: rows
            .loadouts
            .into_iter()
            .map(|row| LoadoutRecord {
                id: row.id,
                class: row.class,
                purpose: row.purpose,
                item_context: row.item_context,
                field_1_60_1_69876_003: row.field_1_60_1_69876_003,
                race_mask: mask(row.race_mask),
            })
            .collect(),
        loadout_items: rows
            .loadout_items
            .into_iter()
            .map(|row| LoadoutItemRecord {
                id: row.id,
                loadout: row.loadout,
                item: row.item,
            })
            .collect(),
        // SQL overlays cannot declare unavailable baseline sections empty.
        unknown_ability_records: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_persistence::forever::birth::{
        LoadoutItemRow, LoadoutRow, SkillAbilityRow, SkillLineRow, SkillRaceClassRow,
    };

    #[test]
    fn all_numeric_widths_and_signed_mask_words_survive_composition() {
        let rows = records(BirthRows {
            skill_lines: vec![SkillLineRow {
                id: u32::MAX,
                category: i8::MIN,
                spell_icon_file: i32::MIN,
                can_link: -1,
                parent_skill: u32::MAX,
                parent_tier_index: i32::MIN,
                flags: i32::MIN,
                spell_book_spell: -1,
                expansion_name_shared_string: i32::MAX,
                horde_expansion_name_shared_string: -2,
            }],
            race_class: vec![SkillRaceClassRow {
                id: 2,
                skill: u16::MAX,
                class_mask: -1,
                flags: i32::MIN,
                availability: -2,
                min_level: i8::MIN,
                tier: i16::MIN,
                race_mask: [i32::MIN, -1],
            }],
            abilities: vec![SkillAbilityRow {
                id: 3,
                skill_line: u16::MAX,
                spell: i32::MIN,
                min_skill_rank: i16::MIN,
                class_mask: -1,
                supercedes_spell: -2,
                acquire_method: i32::MAX,
                trivial_rank_high: i16::MAX,
                trivial_rank_low: i16::MIN,
                flags: i32::MIN,
                num_skill_ups: i8::MIN,
                unique_bit: -3,
                trade_skill_category: -4,
                skillup_skill_line: -1,
                field_5_5_4_67090_014: [i32::MIN, i32::MAX],
                race_mask: [-1, i32::MIN],
            }],
            loadouts: vec![LoadoutRow {
                id: 4,
                class: i8::MIN,
                purpose: i32::MIN,
                item_context: u8::MAX,
                field_1_60_1_69876_003: i32::MAX,
                race_mask: [-1, -1],
            }],
            loadout_items: vec![LoadoutItemRow {
                id: 5,
                loadout: u16::MAX,
                item: u32::MAX,
            }],
        });
        let line = &rows.skill_lines[0];
        assert_eq!(
            (line.id, line.category, line.spell_icon_file, line.can_link),
            (u32::MAX, i8::MIN, i32::MIN, -1)
        );
        assert_eq!(
            (line.parent_skill, line.parent_tier_index, line.flags),
            (u32::MAX, i32::MIN, i32::MIN)
        );
        assert_eq!(
            (
                line.spell_book_spell,
                line.expansion_name_shared_string,
                line.horde_expansion_name_shared_string
            ),
            (-1, i32::MAX, -2)
        );
        let rc = &rows.race_class[0];
        assert_eq!(
            (
                rc.skill,
                rc.class_mask,
                rc.flags,
                rc.availability,
                rc.min_level,
                rc.tier
            ),
            (u16::MAX, -1, i32::MIN, -2, i8::MIN, i16::MIN)
        );
        assert_eq!(rc.race_mask, 0xffff_ffff_8000_0000);
        let ability = &rows.abilities[0];
        assert_eq!(
            (
                ability.skill_line,
                ability.spell,
                ability.min_skill_rank,
                ability.class_mask,
                ability.supercedes_spell,
                ability.acquire_method
            ),
            (u16::MAX, i32::MIN, i16::MIN, -1, -2, i32::MAX)
        );
        assert_eq!(
            (
                ability.trivial_rank_high,
                ability.trivial_rank_low,
                ability.flags,
                ability.num_skill_ups
            ),
            (i16::MAX, i16::MIN, i32::MIN, i8::MIN)
        );
        assert_eq!(
            (
                ability.unique_bit,
                ability.trade_skill_category,
                ability.skillup_skill_line
            ),
            (-3, -4, -1)
        );
        assert_eq!(ability.field_5_5_4_67090_014, [i32::MIN, i32::MAX]);
        assert_eq!(ability.race_mask, 0x8000_0000_ffff_ffff);
        let loadout = &rows.loadouts[0];
        assert_eq!(
            (
                loadout.class,
                loadout.purpose,
                loadout.item_context,
                loadout.field_1_60_1_69876_003,
                loadout.race_mask
            ),
            (i8::MIN, i32::MIN, u8::MAX, i32::MAX, u64::MAX)
        );
        assert_eq!(
            (rows.loadout_items[0].loadout, rows.loadout_items[0].item),
            (u16::MAX, u32::MAX)
        );
        assert_eq!(rows.unknown_ability_records, 0);
    }
}
