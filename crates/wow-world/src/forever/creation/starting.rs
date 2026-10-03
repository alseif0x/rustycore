//! Player.cpp::GetStartLevel/GetStartMoney, pinned Forever 02245dcd.
//! Immutable numeric policy, not a Player/admission/template-data owner.
use super::{CharacterTemplates, SourceError};
use crate::forever::permissions::DefaultAccountPermissions;
use std::sync::Arc;
use wow_data::forever_initialization::InitializationCatalog;

const MAX_LEVEL: u32 = 123;
const MAX_MONEY: u64 = 99_999_999_999;

/// Canonical numeric configuration input; parsing belongs to composition.
/// DB2 StartingLevel fields do not replace these source configuration keys.
pub struct StartingConfig {
    pub normal_level: u32,
    pub death_knight_level: u32,
    pub demon_hunter_level: u32,
    pub allied_level: u32,
    pub gm_level: u32,
    pub normal_money: u64,
    pub death_knight_money: u64,
    pub demon_hunter_money: u64,
    pub evoker_money: u64,
    pub allied_money: u64,
}

pub struct StartingPolicy {
    templates: Arc<CharacterTemplates>,
    normal_level: u8,
    death_knight_level: u8,
    demon_hunter_level: u8,
    allied_level: u8,
    gm_level: u8,
    normal_money: u64,
    death_knight_money: u64,
    demon_hunter_money: u64,
    evoker_money: u64,
    allied_money: u64,
}

/// A transient decision, not a persistent Player or creation success.
pub struct StartingValues {
    level: u8,
    money: u64,
}
impl StartingValues {
    pub fn level(&self) -> u8 {
        self.level
    }
    pub fn money(&self) -> u64 {
        self.money
    }
}

impl StartingPolicy {
    /// World.cpp:752-757,777,899-909,1070-1093; Player.h:1043.
    /// GM is bounded by MAX_LEVEL, NOT MaxPlayerLevel, then raised to the
    /// normal starting level. Above-cap GM/template stats remain a separate path.
    pub fn from_config(
        config: StartingConfig,
        max_level: u8,
        templates: Arc<CharacterTemplates>,
    ) -> Result<Self, SourceError> {
        if !(1..=MAX_LEVEL as u8).contains(&max_level) {
            return Err(SourceError::InvalidLevelCap);
        }
        let level = |value: u32| value.clamp(1, u32::from(max_level)) as u8;
        let normal_level = level(config.normal_level);
        Ok(Self {
            templates,
            normal_level,
            death_knight_level: level(config.death_knight_level),
            demon_hunter_level: level(config.demon_hunter_level),
            allied_level: level(config.allied_level),
            gm_level: (config.gm_level.clamp(1, MAX_LEVEL) as u8).max(normal_level),
            normal_money: config.normal_money.min(MAX_MONEY),
            death_knight_money: config.death_knight_money.min(MAX_MONEY),
            demon_hunter_money: config.demon_hunter_money.min(MAX_MONEY),
            evoker_money: config.evoker_money.min(MAX_MONEY),
            allied_money: config.allied_money.min(MAX_MONEY),
        })
    }

    /// Player.cpp:24649-24716 / RaceMask.h / RBAC.h (10 and 41).
    /// Effective race flags, not old race lists, govern allied initialization.
    /// This does not establish playable combination, map/model, skills or save.
    pub fn select(
        &self,
        race: u8,
        class: u8,
        initialization: &InitializationCatalog,
        permissions: &DefaultAccountPermissions,
        template_id: Option<i32>,
    ) -> Result<StartingValues, SourceError> {
        initialization
            .class(u32::from(class))
            .ok_or(SourceError::MissingClass)?;
        let race_record = initialization
            .race(u32::from(race))
            .ok_or(SourceError::MissingRace)?;
        let allied = race_record.is_allied();
        let pandaren = matches!(race, 25 | 26);
        let mut level = if allied || matches!(race, 52 | 70) {
            self.allied_level
        } else {
            self.normal_level
        };
        if class == 6 {
            level = level.max(if pandaren {
                self.allied_level
            } else {
                self.death_knight_level
            });
        } else if class == 12 {
            level = level.max(self.demon_hunter_level);
        }
        if permissions.use_character_templates() {
            // Source GetCharacterTemplate accepts uint32: preserve conversion
            // of the signed request ID, never mistake that ID for its level.
            if let Some(template) = template_id.and_then(|id| self.templates.get(id as u32)) {
                level = level.max(template.level());
            }
        }
        if permissions.use_start_gm_level() {
            level = level.max(self.gm_level);
        }
        let money = if allied {
            self.allied_money
        } else {
            match class {
                6 => {
                    if pandaren {
                        self.allied_money
                    } else {
                        self.death_knight_money
                    }
                }
                12 => self.demon_hunter_money,
                13 => self.evoker_money,
                _ => self.normal_money,
            }
        };
        Ok(StartingValues { level, money })
    }

    /// Shared immutable owner used by AuthResponse and creation, not a copy.
    pub fn templates(&self) -> &Arc<CharacterTemplates> {
        &self.templates
    }
}

#[cfg(test)]
mod tests;
