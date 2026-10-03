//! Numeric fields consumed by Player::Create/GetStartLevel/InitStatsForLevel.
//! 02245dcd DB2Metadata/DB2LoadInfo::{ChrClasses,ChrRaces}; no locale strings
//! or full wire serializer is claimed by this projection.
use crate::wdc4::creation::{CreationDb2, CreationTable};
use anyhow::Result;
use std::path::Path;

#[derive(Clone, Copy)]
pub struct ClassRecord {
    pub id: u32,
    pub flags: i32,
    pub starting_level: i32,
    pub cinematic: u16,
    pub default_spec: u16,
    pub strength_bonus: u8,
    pub primary_stat_priority: i8,
    pub display_power: i8,
    pub ranged_attack_per_agility: u8,
    pub attack_per_agility: u8,
    pub attack_per_strength: u8,
    pub spell_class_set: u8,
}

#[derive(Clone, Copy)]
pub struct RaceRecord {
    pub id: u32,
    pub flags: i32,
    pub faction: i32,
    pub cinematic: i32,
    pub resurrection_sickness_spell: i32,
    pub starting_level: i32,
    pub base_language: i8,
    pub creature_type: u8,
    pub alliance: i8,
    pub neutral_race: i8,
}

impl RaceRecord {
    /// DBCEnums.h::ChrRacesFlag::IsAlliedRace (not a hardcoded race list).
    pub fn is_allied(&self) -> bool {
        self.flags as u32 & 0x0008_0000 != 0
    }
}

pub(super) fn load(directory: &Path) -> Result<(Vec<ClassRecord>, Vec<RaceRecord>)> {
    let classes = CreationDb2::open(directory, CreationTable::Class)?;
    let class_records = classes
        .ids()
        .map(|id| {
            Ok(ClassRecord {
                id,
                flags: classes.bits(id, 14, 0)? as i32,
                starting_level: classes.bits(id, 15, 0)? as i32,
                cinematic: classes.bits(id, 27, 0)? as u16,
                default_spec: classes.bits(id, 28, 0)? as u16,
                strength_bonus: classes.bits(id, 30, 0)? as u8,
                primary_stat_priority: classes.bits(id, 31, 0)? as i8,
                display_power: classes.bits(id, 32, 0)? as i8,
                ranged_attack_per_agility: classes.bits(id, 33, 0)? as u8,
                attack_per_agility: classes.bits(id, 34, 0)? as u8,
                attack_per_strength: classes.bits(id, 35, 0)? as u8,
                spell_class_set: classes.bits(id, 36, 0)? as u8,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let races = CreationDb2::open(directory, CreationTable::Race)?;
    let race_records = races
        .ids()
        .map(|id| {
            Ok(RaceRecord {
                id,
                flags: races.bits(id, 15, 0)? as i32,
                faction: races.bits(id, 16, 0)? as i32,
                cinematic: races.bits(id, 17, 0)? as i32,
                resurrection_sickness_spell: races.bits(id, 18, 0)? as i32,
                starting_level: races.bits(id, 26, 0)? as i32,
                base_language: races.bits(id, 34, 0)? as i8,
                creature_type: races.bits(id, 35, 0)? as u8,
                alliance: races.bits(id, 36, 0)? as i8,
                neutral_race: races.bits(id, 40, 0)? as i8,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok((class_records, race_records))
}
