//! Immutable target world sources for the ongoing Player::Create port.
//! This is deliberately NOT PlayerInfo: map/transport/movie/scene validation,
//! effective DB2 skills/loadouts/spells/XP/powers and initialization must still
//! succeed before a creation or persistence operation can use these inputs.

mod item_templates;
mod items;
mod progression;
mod skill_tiers;
mod skills;
mod starting;
mod templates;
mod vitals;
pub use item_templates::{
    ItemTemplateError, ItemTemplateMetadata, ItemTemplateNumericView, NumericItemTemplates,
};
pub use items::{InitialItem, InitialItems};
pub use skills::{DefaultSkillRequest, InitialSkillFields, SkillFieldSeed, SkillLookupCounts};
pub use starting::{StartingConfig, StartingPolicy, StartingValues};
pub use templates::{CharacterTemplate, CharacterTemplates, TemplateClass};
pub use vitals::{InitialPower, PreEquipmentVitals};
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use wow_core::Position;
use wow_data::forever_appearance::AppearanceCatalog;
use wow_data::forever_initialization::InitializationCatalog;
use wow_persistence::forever::creation::{
    CastSpell, CreationWorldRows, CustomSpell, ItemOverride, StartAction, StartDefinition,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceError {
    EmptyRequiredSource,
    DuplicateIdentity,
    InvalidLevel,
    MissingDefinition,
    MissingRaceStats,
    MissingLevelOne,
    StatsOverflow,
    MissingMap,
    InstanceableStart,
    InvalidStartPosition,
    MissingGenderModel,
    InvalidLevelCap,
    InsufficientXpCoverage,
    InvalidGameTableValue,
    MissingManaRow,
    InvalidClass,
    MissingRace,
    MissingClass,
    MissingPowerType,
    MissingDisplayPowerIndex,
    MissingBirthSkillSources,
    MissingBirthSkillLookup,
    BirthSkillLookupAlreadyLoaded,
    InvalidBirthSkillLookup,
}

/// The normal-location portion of ObjectMgr::LoadPlayerInfo, not complete
/// PlayerInfo. NPE/transport/intros/skills/items/spells still need their own
/// source validation before Player::Create or persistence may consume them.
#[derive(Clone, Copy)]
pub struct NormalStart {
    pub map: u32,
    pub position: Position,
    pub gender_displays: [u32; 2],
}

/// Sole immutable owner of the admitted *source batch*, not of a Player.
/// Raw SQL rows are consumed here; no Session or legacy Player mirror exists.
pub struct WorldSources {
    definitions: BTreeMap<(u8, u8), StartDefinition>,
    race_stats: BTreeMap<u8, [i16; 5]>,
    class_stats: BTreeMap<(u8, u8), [i32; 5]>,
    remaining: SupplementarySources,
    progression: progression::Progression,
    skill_tiers: skill_tiers::SkillTiers,
    skill_sources: Option<skills::SkillSources>,
}

struct SupplementarySources {
    actions: Vec<StartAction>,
    items: Vec<ItemOverride>,
    custom_spells: Vec<CustomSpell>,
    cast_spells: Vec<CastSpell>,
}

impl WorldSources {
    pub fn load(
        rows: CreationWorldRows,
        initialization: &InitializationCatalog,
        game_tables: &wow_data::forever_game_tables::InitialGameTables,
        max_level: u8,
    ) -> Result<Self, SourceError> {
        Self::from_rows(
            rows,
            |race| initialization.race(u32::from(race)).is_some(),
            |class| initialization.class(u32::from(class)).is_some(),
            game_tables,
            max_level,
        )
    }

    fn from_rows(
        rows: CreationWorldRows,
        race_exists: impl Fn(u8) -> bool,
        class_exists: impl Fn(u8) -> bool,
        game_tables: &wow_data::forever_game_tables::InitialGameTables,
        max_level: u8,
    ) -> Result<Self, SourceError> {
        if rows.definitions.is_empty() || rows.race_stats.is_empty() || rows.class_stats.is_empty()
        {
            return Err(SourceError::EmptyRequiredSource);
        }
        let mut definitions = BTreeMap::new();
        for row in rows.definitions {
            if race_exists(row.race)
                && class_exists(row.class)
                && definitions.insert((row.race, row.class), row).is_some()
            {
                return Err(SourceError::DuplicateIdentity);
            }
        }
        let mut race_stats = BTreeMap::new();
        for row in rows.race_stats {
            if race_exists(row.race) && race_stats.insert(row.race, row.modifiers).is_some() {
                return Err(SourceError::DuplicateIdentity);
            }
        }
        let mut class_stats = BTreeMap::new();
        for row in rows.class_stats {
            if !class_exists(row.class) {
                continue;
            }
            // Source indexes level-1; corrupt zero cannot become index 255.
            if row.level == 0 {
                return Err(SourceError::InvalidLevel);
            }
            if row.level > max_level {
                continue;
            }
            if class_stats
                .insert((row.class, row.level), row.stats)
                .is_some()
            {
                return Err(SourceError::DuplicateIdentity);
            }
        }
        Ok(Self {
            definitions,
            race_stats,
            class_stats,
            remaining: SupplementarySources {
                actions: rows.actions,
                items: rows.item_overrides,
                custom_spells: rows.custom_spells,
                cast_spells: rows.cast_spells,
            },
            progression: progression::Progression::load(rows.xp_overrides, game_tables, max_level)?,
            skill_tiers: skill_tiers::SkillTiers::load(rows.skill_tiers)?,
            skill_sources: None,
        })
    }

    /// Unvalidated world definition. Presence never proves a usable map,
    /// model, transport or intro. Nullable source fields remain untouched.
    pub fn definition(&self, race: u8, class: u8) -> Option<&StartDefinition> {
        self.definitions.get(&(race, class))
    }

    /// Source SkillTiersEntry::GetValueForTierIndex: clamp to 15 then
    /// backtrack zero columns. Zero is legitimate; None means missing tier.
    /// SetSkill's uint16 narrowing is not performed by this source lookup.
    pub fn skill_tier_value(&self, tier: u32, index: u32) -> Option<u32> {
        self.skill_tiers.value(tier, index)
    }

    /// ObjectMgr.cpp:3881-3928, MapManager.h:93-95/MapManager.cpp:377,
    /// GridDefines.h:199-216 and Position.cpp:207-214 at target 02245dcd.
    /// Reuses only source-contrasted pure coordinate/orientation math, not
    /// legacy map DB2 layouts or a fabricated fallback position.
    pub fn normal_start(
        &self,
        race: u8,
        class: u8,
        initialization: &InitializationCatalog,
        appearance: &AppearanceCatalog,
    ) -> Result<NormalStart, SourceError> {
        let row = self
            .definition(race, class)
            .ok_or(SourceError::MissingDefinition)?;
        let map = initialization
            .map(u32::from(row.map))
            .ok_or(SourceError::MissingMap)?;
        let [x, y, z, orientation] = row.position;
        let mut position = Position::new(x, y, z, orientation);
        if !position.is_valid_map_coord_like_cpp() {
            return Err(SourceError::InvalidStartPosition);
        }
        if map.instanceable() {
            return Err(SourceError::InstanceableStart);
        }
        let male = appearance
            .model(race, 0)
            .ok_or(SourceError::MissingGenderModel)?;
        let female = appearance
            .model(race, 1)
            .ok_or(SourceError::MissingGenderModel)?;
        position.orientation = wow_movement::normalize_orientation_like_cpp(orientation);
        Ok(NormalStart {
            map: map.id,
            position,
            gender_displays: [male.display, female.display],
        })
    }

    /// ObjectMgr.cpp:4303-4395 / ObjectMgr.h::PlayerLevelInfo (signed int32).
    /// A missing race-stat row is unknown/fatal, not zero racial modifiers.
    /// Source fills gaps *after* adding racial stats, using strength == 0.
    /// Caller must still validate PlayerInfo/map/DB2 and configured level cap.
    pub fn primary_stats(&self, race: u8, class: u8, level: u8) -> Result<[i32; 5], SourceError> {
        if level == 0 {
            return Err(SourceError::InvalidLevel);
        }
        if self.definition(race, class).is_none() {
            return Err(SourceError::MissingDefinition);
        }
        let modifiers = self
            .race_stats
            .get(&race)
            .ok_or(SourceError::MissingRaceStats)?;
        let mut previous = None;
        let cap = self.progression.max_level();
        for current in 1..=level.min(cap) {
            let combined = self
                .class_stats
                .get(&(class, current))
                .map(|stats| {
                    let mut result = [0; 5];
                    for index in 0..5 {
                        result[index] = stats[index]
                            .checked_add(i32::from(modifiers[index]))
                            .ok_or(SourceError::StatsOverflow)?;
                    }
                    Ok::<_, SourceError>(result)
                })
                .transpose()?;
            if let Some(stats) = combined.filter(|stats| stats[0] != 0) {
                previous = Some(stats);
            } else if current == 1 {
                return Err(SourceError::MissingLevelOne);
            }
        }
        let stats = previous.ok_or(SourceError::MissingLevelOne)?;
        if level > cap {
            progression::extrapolate_stats(class, cap, level, stats)
        } else {
            Ok(stats)
        }
    }

    /// Source XP table lookup; out-of-range returns zero. This does not admit
    /// the requested Player level or replace the starting-level policy gate.
    pub fn experience_for_level(&self, level: u8) -> u32 {
        self.progression.xp(level)
    }

    /// Source GetPlayerClassLevelInfo clamps the lookup to configured cap.
    /// Numeric asset/coverage errors are not silently returned as zero mana.
    pub fn base_mana(
        &self,
        tables: &wow_data::forever_game_tables::InitialGameTables,
        class: u8,
        level: u8,
    ) -> Result<u32, SourceError> {
        self.progression.base_mana(tables, class, level)
    }

    pub fn progression_counts(&self) -> [usize; 2] {
        self.progression.counts()
    }

    /// Metadata only; no position, name, spell or private client record dumped.
    pub fn counts(&self) -> [usize; 9] {
        [
            self.definitions.len(),
            self.remaining.items.len(),
            self.remaining.custom_spells.len(),
            self.remaining.cast_spells.len(),
            self.remaining.actions.len(),
            self.race_stats.len(),
            self.class_stats.len(),
            self.progression.override_count(),
            self.skill_tiers.len(),
        ]
    }
}
