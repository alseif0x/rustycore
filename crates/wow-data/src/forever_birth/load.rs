use super::{
    AbilityBaseline, BirthRecords, LoadoutItemRecord, LoadoutRecord, SkillAbilityRecord,
    SkillLineRecord, SkillRaceClassRecord,
};
use crate::wdc4::creation::{CreationDb2, CreationTable};
use anyhow::Result;
use std::path::Path;

fn mask(table: &CreationDb2, id: u32, field: usize) -> Result<u64> {
    Ok(u64::from(table.bits(id, field, 0)?) | (u64::from(table.bits(id, field, 1)?) << 32))
}

pub(super) fn records(directory: &Path, baseline: AbilityBaseline) -> Result<BirthRecords> {
    let lines = CreationDb2::open(directory, CreationTable::SkillLine)?;
    let skill_lines = lines
        .ids()
        .map(|id| {
            Ok(SkillLineRecord {
                id,
                category: lines.bits(id, 6, 0)? as i8,
                spell_icon_file: lines.bits(id, 7, 0)? as i32,
                can_link: lines.bits(id, 8, 0)? as i8,
                parent_skill: lines.bits(id, 9, 0)?,
                parent_tier_index: lines.bits(id, 10, 0)? as i32,
                flags: lines.bits(id, 11, 0)? as i32,
                spell_book_spell: lines.bits(id, 12, 0)? as i32,
                expansion_name_shared_string: lines.bits(id, 13, 0)? as i32,
                horde_expansion_name_shared_string: lines.bits(id, 14, 0)? as i32,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let rc = CreationDb2::open(directory, CreationTable::SkillRaceClass)?;
    let race_class = rc
        .ids()
        .map(|id| {
            Ok(SkillRaceClassRecord {
                id,
                skill: rc.bits(id, 0, 0)? as u16,
                class_mask: rc.bits(id, 1, 0)? as i32,
                flags: rc.bits(id, 2, 0)? as i32,
                availability: rc.bits(id, 3, 0)? as i32,
                min_level: rc.bits(id, 4, 0)? as i8,
                tier: rc.bits(id, 5, 0)? as i16,
                race_mask: mask(&rc, id, 6)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let (ability, unknown_ability_records) = match baseline {
        AbilityBaseline::Complete => (
            CreationDb2::open(directory, CreationTable::SkillAbility)?,
            0,
        ),
        AbilityBaseline::AvailablePrefix => (CreationDb2::open_birth_ability_prefix(directory)?, 5),
    };
    let abilities = ability
        .ids()
        .map(|id| {
            Ok(SkillAbilityRecord {
                id,
                skill_line: ability.bits(id, 3, 0)? as u16,
                spell: ability.bits(id, 4, 0)? as i32,
                min_skill_rank: ability.bits(id, 5, 0)? as i16,
                class_mask: ability.bits(id, 6, 0)? as i32,
                supercedes_spell: ability.bits(id, 7, 0)? as i32,
                acquire_method: ability.bits(id, 8, 0)? as i32,
                trivial_rank_high: ability.bits(id, 9, 0)? as i16,
                trivial_rank_low: ability.bits(id, 10, 0)? as i16,
                flags: ability.bits(id, 11, 0)? as i32,
                num_skill_ups: ability.bits(id, 12, 0)? as i8,
                unique_bit: ability.bits(id, 13, 0)? as i16,
                trade_skill_category: ability.bits(id, 14, 0)? as i16,
                skillup_skill_line: ability.bits(id, 15, 0)? as i16,
                field_5_5_4_67090_014: [
                    ability.bits(id, 16, 0)? as i32,
                    ability.bits(id, 16, 1)? as i32,
                ],
                race_mask: mask(&ability, id, 17)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let loadout = CreationDb2::open(directory, CreationTable::Loadout)?;
    let loadouts = loadout
        .ids()
        .map(|id| {
            Ok(LoadoutRecord {
                id,
                class: loadout.bits(id, 0, 0)? as i8,
                purpose: loadout.bits(id, 1, 0)? as i32,
                item_context: loadout.bits(id, 2, 0)? as u8,
                field_1_60_1_69876_003: loadout.bits(id, 3, 0)? as i32,
                race_mask: mask(&loadout, id, 4)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let items = CreationDb2::open(directory, CreationTable::LoadoutItem)?;
    let loadout_items = items
        .ids()
        .map(|id| {
            Ok(LoadoutItemRecord {
                id,
                loadout: items.bits(id, 0, 0)? as u16,
                item: items.bits(id, 1, 0)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(BirthRecords {
        skill_lines,
        race_class,
        abilities,
        loadouts,
        loadout_items,
        unknown_ability_records,
    })
}
