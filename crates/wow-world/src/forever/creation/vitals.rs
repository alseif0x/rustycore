//! Target Player::Create's unmodified numerical state, before default spells
//! and equipment. This is a transient result, NOT an admitted/saved Player or
//! the complete InitStatsForLevel update-field projection.
use super::{SourceError, WorldSources, progression::unsigned};
use wow_data::{
    forever_game_tables::InitialGameTables, forever_initialization::InitializationCatalog,
};

#[derive(Clone, Copy)]
pub struct InitialPower {
    kind: i8,
    index: u8,
    maximum: i32,
    current: i32,
}
impl InitialPower {
    pub fn kind(&self) -> i8 {
        self.kind
    }
    pub fn index(&self) -> u8 {
        self.index
    }
    pub fn maximum(&self) -> i32 {
        self.maximum
    }
    pub fn current(&self) -> i32 {
        self.current
    }
}

pub struct PreEquipmentVitals {
    stats: [i32; 5],
    base_mana: u32,
    armor: i32,
    health: u64,
    next_level_xp: u32,
    experience: u32,
    display_power: i8,
    powers: Vec<InitialPower>,
}
impl PreEquipmentVitals {
    pub fn stats(&self) -> &[i32; 5] {
        &self.stats
    }
    pub fn base_mana(&self) -> u32 {
        self.base_mana
    }
    pub fn armor(&self) -> i32 {
        self.armor
    }
    /// CreateHealth is source zero. Health/MaxHealth are full before items.
    pub fn full_health(&self) -> u64 {
        self.health
    }
    pub fn next_level_xp(&self) -> u32 {
        self.next_level_xp
    }
    pub fn experience(&self) -> u32 {
        self.experience
    }
    pub fn display_power(&self) -> i8 {
        self.display_power
    }
    pub fn powers(&self) -> &[InitialPower] {
        &self.powers
    }
}

impl WorldSources {
    /// 02245dcd Player.cpp:2394-2580, StatSystem.cpp:90-99,285-332,
    /// Unit.cpp:10081-10185, Player.cpp:504-507/27354-27413.
    /// Missing power metadata cannot become a fabricated zero maximum.
    pub fn pre_equipment_vitals(
        &self,
        race: u8,
        class: u8,
        level: u8,
        initialization: &InitializationCatalog,
        tables: &InitialGameTables,
    ) -> Result<PreEquipmentVitals, SourceError> {
        initialization
            .race(u32::from(race))
            .ok_or(SourceError::MissingRace)?;
        let class_record = initialization
            .class(u32::from(class))
            .ok_or(SourceError::MissingClass)?;
        let display_power = class_record.display_power;
        initialization
            .power(display_power)
            .ok_or(SourceError::MissingPowerType)?;
        initialization
            .class_power_index(u32::from(class), display_power)
            .ok_or(SourceError::MissingDisplayPowerIndex)?;
        let stats = self.primary_stats(race, class, level)?;
        let base_mana = self.base_mana(tables, class, level)?;
        // C++ returns uint32 create mana through an int32 power API. Require
        // representability rather than infer a negative maximum on corrupt GT.
        let mana_maximum =
            i32::try_from(base_mana).map_err(|_| SourceError::InvalidGameTableValue)?;
        let armor = stats[1].checked_mul(2).ok_or(SourceError::StatsOverflow)?;
        // Missing HpPerSta row has a pinned fallback of TEN, not unknown zero.
        let ratio = tables
            .hp_per_sta(u32::from(level))
            .map_or(10.0, |row| row.health);
        let health = u64::from(unsigned(stats[2] as f32 * ratio)?.max(1));
        let next_level_xp = self.experience_for_level(level);
        let experience = if next_level_xp == 0 { u32::MAX } else { 0 };
        let mut powers = Vec::new();
        for (index, kind) in initialization
            .class_power_types(u32::from(class))
            .enumerate()
        {
            let power = initialization
                .power(kind)
                .ok_or(SourceError::MissingPowerType)?;
            let maximum = if kind == 0 {
                mana_maximum
            } else {
                power.max_base_power
            };
            let mut current = 0.min(maximum);
            // SetRuneCooldown runs before InitStats. Only a DK displaying
            // rune power has a nonzero rune maximum at that earlier phase.
            if class == 6 && kind == 5 && display_power == 5 {
                if maximum < 0 {
                    return Err(SourceError::InvalidGameTableValue);
                }
                current = maximum.min(6);
            }
            // InitStatsForLevel fills mana, energy, focus. Then Create applies
            // SetToMaxOnInitialLogIn for every class power. DefaultPower and
            // UnitsUseDefaultPowerOnInit do not select current power here.
            if matches!(kind, 0 | 2 | 3) || power.flags & 0x2000 != 0 {
                current = maximum;
            }
            powers.push(InitialPower {
                kind,
                index: index as u8,
                maximum,
                current,
            });
        }
        Ok(PreEquipmentVitals {
            stats,
            base_mana,
            armor,
            health,
            next_level_xp,
            experience,
            display_power,
            powers,
        })
    }
}

#[cfg(test)]
mod tests;
