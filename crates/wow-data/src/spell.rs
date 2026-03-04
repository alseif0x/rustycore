// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spell.db2 and related spell data loading.
//!
//! Loads spell metadata from hotfixes database or DB2 files:
//! - Cast time (milliseconds)
//! - Global cooldown
//! - Per-spell cooldown
//! - Effect type (heal, damage, apply aura, etc.)
//! - Effect parameters (base points, bonus coefficients)

use std::collections::HashMap;

use anyhow::Result;
use tracing::info;
use wow_database::HotfixDatabase;

/// Spell effect types (from SpellEffectType enum)
pub mod spell_effect_types {
    pub const SPELL_EFFECT_SCHOOL_DAMAGE: u32 = 2;
    pub const SPELL_EFFECT_HEAL: u32 = 6;
    pub const SPELL_EFFECT_APPLY_AURA: u32 = 35;
}

/// Aura types (from AuraType enum)
pub mod aura_types {
    pub const SPELL_AURA_DUMMY: i32 = 0;
    pub const SPELL_AURA_SCHOOL_ABSORB: i32 = 1;
    pub const SPELL_AURA_SCHOOL_IMMUNITY: i32 = 2;
    pub const SPELL_AURA_DUMMY_ABSORB: i32 = 3;
    pub const SPELL_AURA_MODIFY_DAMAGE_PERCENT_TAKEN: i32 = 31;
    pub const SPELL_AURA_HASTE_SPELLS: i32 = 73;
}

/// Metadata for a spell from Spell.db2 and related tables.
#[derive(Debug, Clone)]
pub struct SpellInfo {
    /// Spell ID
    pub spell_id: i32,
    /// Cast time in milliseconds (0 = instant)
    pub cast_time_ms: u32,
    /// Global cooldown in milliseconds
    pub cooldown_ms: u32,
    /// Per-spell cooldown in milliseconds (0 = no per-spell cooldown)
    pub recovery_time_ms: u32,
    /// First effect type (primary effect) — e.g., 2 (damage), 6 (heal), 35 (aura)
    pub effect_type: u32,
    /// Base damage/healing before bonuses
    pub effect_base_points: i32,
    /// Spell power / attack power coefficient (0.0 = no scaling)
    pub effect_bonus_coefficient: f32,
    /// Aura type if effect_type == SPELL_EFFECT_APPLY_AURA
    pub aura_type: Option<i32>,
    /// Display flags (channelled, etc.)
    pub display_flags: u32,
}

impl SpellInfo {
    /// Convenience: returns the effective cooldown (per-spell or global, whichever is larger).
    pub fn effective_cooldown_ms(&self) -> u32 {
        self.recovery_time_ms.max(self.cooldown_ms)
    }

    /// Returns true if this spell has a cast time (not instant).
    pub fn has_cast_time(&self) -> bool {
        self.cast_time_ms > 0
    }
}

/// In-memory store of all spells loaded from DB2 or hotfixes database.
#[derive(Default)]
pub struct SpellStore {
    spells: HashMap<i32, SpellInfo>,
}

impl SpellStore {
    /// Create a new empty spell store.
    pub fn new() -> Self {
        Self {
            spells: HashMap::new(),
        }
    }

    /// Load spell data from hotfixes database.
    ///
    /// Queries `hotfixes.spell_misc` (cast time, cooldowns) and
    /// `hotfixes.spell_effect` (effect type, damage/healing parameters).
    ///
    /// # Arguments
    ///
    /// * `db` - HotfixDatabase connection pool
    ///
    /// # Returns
    ///
    /// A populated SpellStore on success, or a database error on failure.
    pub async fn load(db: &HotfixDatabase) -> Result<Self> {
        let mut store = Self::new();

        // Query spell_misc and spell_effect from hotfixes database
        // NOTE: Phase 1 — cast_time_ms and cooldown_ms are hardcoded to 0 (instant).
        // Phase 2+ will load from SpellCastTimes.dbc and SpellDuration.dbc using
        // CastingTimeIndex and DurationIndex respectively.
        let sql = r#"
SELECT 
    CAST(sm.ID AS SIGNED) as spell_id,
    CAST(0 AS UNSIGNED) as cast_time_ms,
    CAST(0 AS UNSIGNED) as cooldown_ms,
    CAST(0 AS UNSIGNED) as recovery_time_ms,
    CAST(COALESCE(se.Effect, 0) AS UNSIGNED) as effect_type,
    CAST(COALESCE(se.EffectBasePoints, 0) AS SIGNED) as effect_base_points,
    CAST(COALESCE(se.EffectBonusCoefficient, 0.0) AS DECIMAL(10,2)) as effect_bonus_coeff,
    CAST(COALESCE(se.EffectAura, 0) AS SIGNED) as effect_aura
FROM hotfixes.spell_misc sm
LEFT JOIN hotfixes.spell_effect se 
    ON sm.ID = se.SpellID AND se.DifficultyID = 0
LIMIT 10000
        "#;

        let mut result = db.direct_query(sql).await?;

        if !result.is_empty() {
            loop {
                let spell_id: i32 = result.read(0);
                let cast_time_ms: u32 = result.read(1);
                let cooldown_ms: u32 = result.read(2);
                let recovery_time_ms: u32 = result.read(3);
                let effect_type: u32 = result.try_read(4).unwrap_or(0);
                let effect_base_points: i32 = result.try_read(5).unwrap_or(0);
                let effect_bonus_coefficient: f32 = result.try_read(6).unwrap_or(0.0);
                let aura_type: Option<i32> = result.try_read(7);

                let spell_info = SpellInfo {
                    spell_id,
                    cast_time_ms,
                    cooldown_ms,
                    recovery_time_ms,
                    effect_type,
                    effect_base_points,
                    effect_bonus_coefficient,
                    aura_type,
                    display_flags: 0,
                };

                store.spells.insert(spell_id, spell_info);

                if !result.next_row() {
                    break;
                }
            }
        }

        info!("Loaded {} spells from hotfixes database", store.spells.len());
        Ok(store)
    }



    /// Look up a spell by ID.
    pub fn get(&self, spell_id: i32) -> Option<&SpellInfo> {
        self.spells.get(&spell_id)
    }

    /// Insert a spell into the store (for testing or dynamic registration).
    #[allow(dead_code)]
    pub fn insert(&mut self, spell_id: i32, info: SpellInfo) {
        self.spells.insert(spell_id, info);
    }

    /// Get the total number of loaded spells.
    pub fn len(&self) -> usize {
        self.spells.len()
    }

    /// Check if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.spells.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spell_store_creation() {
        let store = SpellStore::new();
        assert!(store.is_empty(), "new store should be empty");
    }

    #[test]
    fn test_spell_info_effective_cooldown() {
        let spell = SpellInfo {
            spell_id: 100,
            cast_time_ms: 0,
            cooldown_ms: 1500,
            recovery_time_ms: 8000,
            effect_type: 2,
            effect_base_points: 50,
            effect_bonus_coefficient: 0.5,
            aura_type: None,
            display_flags: 0,
        };

        // recovery_time_ms is larger
        assert_eq!(spell.effective_cooldown_ms(), 8000);

        let instant = SpellInfo {
            spell_id: 100,
            cast_time_ms: 0,
            cooldown_ms: 1500,
            recovery_time_ms: 0,
            effect_type: 2,
            effect_base_points: 50,
            effect_bonus_coefficient: 0.5,
            aura_type: None,
            display_flags: 0,
        };

        // GCD is the limit
        assert_eq!(instant.effective_cooldown_ms(), 1500);
    }
}
