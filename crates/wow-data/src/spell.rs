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

use crate::{ConditionEntriesByTypeStore, ConditionsReference};

/// Spell effect types (from SpellEffectType enum)
pub mod spell_effect_types {
    pub const SPELL_EFFECT_SCHOOL_DAMAGE: u32 = 2;
    pub const SPELL_EFFECT_HEAL: u32 = 6;
    pub const SPELL_EFFECT_PERSISTENT_AREA_AURA: u32 = 27;
    pub const SPELL_EFFECT_APPLY_AURA: u32 = 35;
    /// C++ `SPELL_EFFECT_SUMMON_OBJECT_WILD`; see
    /// `Spell::EffectSummonObjectWild` (`SpellEffects.cpp:2937-2986`).
    pub const SPELL_EFFECT_SUMMON_OBJECT_WILD: u32 = 76;
    pub const SPELL_EFFECT_ADD_FARSIGHT: u32 = 72;
    /// First C++ `SPELL_EFFECT_SUMMON_OBJECT_SLOT*` value; see
    /// `Spell::EffectSummonObject` (`SpellEffects.cpp:3541-3597`).
    pub const SPELL_EFFECT_SUMMON_OBJECT_SLOT1: u32 = 104;
    pub const SPELL_EFFECT_APPLY_AREA_AURA_PARTY: u32 = 35;
    pub const SPELL_EFFECT_APPLY_AREA_AURA_RAID: u32 = 65;
    pub const SPELL_EFFECT_APPLY_AREA_AURA_PET: u32 = 119;
    pub const SPELL_EFFECT_APPLY_AREA_AURA_FRIEND: u32 = 128;
    pub const SPELL_EFFECT_APPLY_AREA_AURA_ENEMY: u32 = 129;
    pub const SPELL_EFFECT_APPLY_AREA_AURA_OWNER: u32 = 143;
    pub const SPELL_EFFECT_APPLY_AURA_ON_PET: u32 = 174;
    pub const SPELL_EFFECT_APPLY_AREA_AURA_SUMMONS: u32 = 202;
    pub const SPELL_EFFECT_APPLY_AREA_AURA_PARTY_NONRANDOM: u32 = 271;
}

/// Aura types (from AuraType enum)
pub mod aura_types {
    pub const SPELL_AURA_CONTROL_VEHICLE: i32 = 236;
    pub const SPELL_AURA_DUMMY: i32 = 0;
    pub const SPELL_AURA_SCHOOL_ABSORB: i32 = 1;
    pub const SPELL_AURA_SCHOOL_IMMUNITY: i32 = 2;
    pub const SPELL_AURA_DUMMY_ABSORB: i32 = 3;
    pub const SPELL_AURA_MOD_TAUNT: i32 = 11;
    pub const SPELL_AURA_MODIFY_DAMAGE_PERCENT_TAKEN: i32 = 31;
    pub const SPELL_AURA_HASTE_SPELLS: i32 = 73;
    pub const SPELL_AURA_MOUNTED: i32 = 78;
    pub const SPELL_AURA_PROVIDE_SPELL_FOCUS: i32 = 281;
}

/// Selected `Targets` ids from C++ `SpellImplicitTargetInfo::_data`.
pub mod implicit_targets {
    pub const TARGET_DEST_NEARBY_ENTRY: u32 = 46;
    pub const TARGET_DEST_NEARBY_ENTRY_2: u32 = 107;
    pub const TARGET_DEST_NEARBY_ENTRY_OR_DB: u32 = 142;
}

pub mod attributes {
    pub const SPELL_ATTR4_USE_FACING_FROM_SPELL: u32 = 0x8000_0000;
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
    /// C++ `SpellInfo::RequiresSpellFocus`, hydrated from
    /// `SpellCastingRequirementsEntry::RequiresSpellFocus`.
    pub requires_spell_focus: u32,
    /// Spell effects keyed by C++ `SpellEffectInfo::EffectIndex`.
    pub effects: Vec<SpellEffectInfo>,
}

/// Minimal `SpellEffectInfo` fields needed by C++ ConditionMgr validation.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpellEffectInfo {
    pub effect_index: u32,
    pub effect: u32,
    pub effect_aura: i32,
    pub effect_base_points: i32,
    pub effect_misc_value_1: i32,
    pub effect_misc_value_2: i32,
    /// C++ `SpellEffectEntry::EffectRadiusIndex[0]` / TargetA radius index.
    pub effect_radius_index_1: u32,
    pub position_facing: f32,
    pub chain_targets: i32,
    pub implicit_target_1: u32,
    pub implicit_target_2: u32,
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

    pub fn effects(&self) -> &[SpellEffectInfo] {
        &self.effects
    }

    pub fn has_aura_like_cpp(&self, aura_type: i32) -> bool {
        self.effects
            .iter()
            .any(|effect| effect.effect_aura == aura_type)
    }

    pub fn requires_spell_focus_like_cpp(&self) -> bool {
        self.requires_spell_focus != 0
    }

    pub fn normalized_implicit_target_effect_mask_like_cpp(&self, mut effect_mask: u32) -> u32 {
        let original_mask = effect_mask;
        for effect in &self.effects {
            let bit = 1u32.checked_shl(effect.effect_index).unwrap_or(0);
            if bit == 0 || (original_mask & bit) == 0 {
                continue;
            }

            if !effect.accepts_implicit_target_conditions_like_cpp() {
                effect_mask &= !bit;
            }
        }
        effect_mask
    }
}

impl SpellEffectInfo {
    pub fn is_mounted_aura_like_cpp(&self) -> bool {
        self.effect == spell_effect_types::SPELL_EFFECT_APPLY_AURA
            && self.effect_aura == aura_types::SPELL_AURA_MOUNTED
    }

    pub fn is_provide_spell_focus_aura_like_cpp(&self) -> bool {
        self.effect == spell_effect_types::SPELL_EFFECT_APPLY_AURA
            && self.effect_aura == aura_types::SPELL_AURA_PROVIDE_SPELL_FOCUS
    }

    pub fn has_focus_destination_implicit_target_like_cpp(&self) -> bool {
        matches!(
            self.implicit_target_1,
            implicit_targets::TARGET_DEST_NEARBY_ENTRY
                | implicit_targets::TARGET_DEST_NEARBY_ENTRY_2
                | implicit_targets::TARGET_DEST_NEARBY_ENTRY_OR_DB
        ) || matches!(
            self.implicit_target_2,
            implicit_targets::TARGET_DEST_NEARBY_ENTRY
                | implicit_targets::TARGET_DEST_NEARBY_ENTRY_2
                | implicit_targets::TARGET_DEST_NEARBY_ENTRY_OR_DB
        )
    }

    pub fn accepts_implicit_target_conditions_like_cpp(&self) -> bool {
        self.chain_targets > 0
            || implicit_target_category_accepts_conditions_like_cpp(self.implicit_target_1)
            || implicit_target_category_accepts_conditions_like_cpp(self.implicit_target_2)
            || spell_effect_accepts_implicit_target_conditions_like_cpp(self.effect)
    }
}

const fn spell_effect_accepts_implicit_target_conditions_like_cpp(effect: u32) -> bool {
    use spell_effect_types::*;
    matches!(
        effect,
        SPELL_EFFECT_PERSISTENT_AREA_AURA
            | SPELL_EFFECT_APPLY_AREA_AURA_PARTY
            | SPELL_EFFECT_APPLY_AREA_AURA_RAID
            | SPELL_EFFECT_APPLY_AREA_AURA_FRIEND
            | SPELL_EFFECT_APPLY_AREA_AURA_ENEMY
            | SPELL_EFFECT_APPLY_AREA_AURA_PET
            | SPELL_EFFECT_APPLY_AREA_AURA_OWNER
            | SPELL_EFFECT_APPLY_AURA_ON_PET
            | SPELL_EFFECT_APPLY_AREA_AURA_SUMMONS
            | SPELL_EFFECT_APPLY_AREA_AURA_PARTY_NONRANDOM
    )
}

const fn implicit_target_category_accepts_conditions_like_cpp(target: u32) -> bool {
    matches!(
        target,
        2 | 3
            | 4
            | 7
            | 8
            | 15
            | 16
            | 20
            | 24
            | 30
            | 31
            | 33
            | 34
            | 37
            | 38
            | 40
            | 46
            | 51
            | 52
            | 54
            | 56
            | 58
            | 59
            | 60
            | 61
            | 89
            | 93
            | 104
            | 105
            | 107
            | 108
            | 109
            | 110
            | 115
            | 116
            | 118
            | 119
            | 120
            | 122
            | 123
            | 128
            | 129
            | 130
            | 133
            | 134
            | 135
            | 136
            | 142
            | 151
    )
}

/// In-memory store of all spells loaded from DB2 or hotfixes database.
#[derive(Default)]
pub struct SpellStore {
    spells: HashMap<i32, SpellInfo>,
    implicit_target_conditions: HashMap<(i32, u32), ConditionsReference>,
}

impl SpellStore {
    /// Create a new empty spell store.
    pub fn new() -> Self {
        Self {
            spells: HashMap::new(),
            implicit_target_conditions: HashMap::new(),
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
    CAST(COALESCE(se.EffectAura, 0) AS SIGNED) as effect_aura,
    CAST(COALESCE(se.EffectMiscValue1, 0) AS SIGNED) as effect_misc_value_1,
    CAST(COALESCE(se.EffectMiscValue2, 0) AS SIGNED) as effect_misc_value_2,
    CAST(COALESCE(se.EffectRadiusIndex1, 0) AS UNSIGNED) as effect_radius_index_1,
    CAST(COALESCE(se.EffectPosFacing, 0.0) AS DECIMAL(10,4)) as position_facing,
    CAST(COALESCE(se.EffectIndex, 0) AS UNSIGNED) as effect_index,
    CAST(COALESCE(se.EffectChainTargets, 0) AS SIGNED) as effect_chain_targets,
    CAST(COALESCE(se.ImplicitTarget1, 0) AS UNSIGNED) as implicit_target_1,
    CAST(COALESCE(se.ImplicitTarget2, 0) AS UNSIGNED) as implicit_target_2,
    CAST(COALESCE(scr.RequiresSpellFocus, 0) AS UNSIGNED) as requires_spell_focus
FROM hotfixes.spell_misc sm
LEFT JOIN hotfixes.spell_effect se 
    ON sm.ID = se.SpellID AND se.DifficultyID = 0
LEFT JOIN hotfixes.spell_casting_requirements scr
    ON sm.ID = scr.SpellID AND scr.DifficultyID = 0
ORDER BY sm.ID, se.EffectIndex
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
                let effect_misc_value_1: i32 = result.try_read(8).unwrap_or(0);
                let effect_misc_value_2: i32 = result.try_read(9).unwrap_or(0);
                let effect_radius_index_1: u32 = result.try_read(10).unwrap_or(0);
                let position_facing: f32 = result.try_read(11).unwrap_or(0.0);
                let effect_index: u32 = result.try_read(12).unwrap_or(0);
                let effect_chain_targets: i32 = result.try_read(13).unwrap_or(0);
                let implicit_target_1: u32 = result.try_read(14).unwrap_or(0);
                let implicit_target_2: u32 = result.try_read(15).unwrap_or(0);
                let requires_spell_focus: u32 = result.try_read(16).unwrap_or(0);

                let spell_info = store.spells.entry(spell_id).or_insert_with(|| SpellInfo {
                    spell_id,
                    cast_time_ms,
                    cooldown_ms,
                    recovery_time_ms,
                    effect_type,
                    effect_base_points,
                    effect_bonus_coefficient,
                    aura_type,
                    display_flags: 0,
                    requires_spell_focus,
                    effects: Vec::new(),
                });

                if effect_type != 0 {
                    spell_info.effects.push(SpellEffectInfo {
                        effect_index,
                        effect: effect_type,
                        effect_aura: aura_type.unwrap_or(0),
                        effect_base_points,
                        effect_misc_value_1,
                        effect_misc_value_2,
                        effect_radius_index_1,
                        position_facing,
                        chain_targets: effect_chain_targets,
                        implicit_target_1,
                        implicit_target_2,
                    });
                }

                if !result.next_row() {
                    break;
                }
            }
        }

        info!(
            "Loaded {} spells from hotfixes database",
            store.spells.len()
        );
        Ok(store)
    }

    /// Look up a spell by ID.
    pub fn get(&self, spell_id: i32) -> Option<&SpellInfo> {
        self.spells.get(&spell_id)
    }

    pub fn implicit_target_conditions_like_cpp(
        &self,
        spell_id: i32,
        effect_index: u32,
    ) -> Option<&ConditionsReference> {
        self.implicit_target_conditions
            .get(&(spell_id, effect_index))
    }

    pub fn attach_spell_implicit_target_conditions_like_cpp(
        &mut self,
        conditions: &ConditionEntriesByTypeStore,
    ) -> usize {
        let mut attached = 0;
        let Some(entries) = conditions.entries_for_source_type_like_cpp(
            wow_constants::ConditionSourceType::SpellImplicitTarget,
        ) else {
            return attached;
        };

        self.implicit_target_conditions.clear();
        for (id, bucket) in entries {
            let Some(spell) = self.spells.get(&id.source_entry) else {
                continue;
            };

            for effect in &spell.effects {
                let bit = 1_u32.checked_shl(effect.effect_index).unwrap_or(0);
                if bit == 0 || (id.source_group & bit) == 0 {
                    continue;
                }

                self.implicit_target_conditions.insert(
                    (id.source_entry, effect.effect_index),
                    ConditionsReference::new(bucket),
                );
                attached += bucket.len();
            }
        }

        attached
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
    use crate::{Condition, ConditionEntriesByTypeStore};
    use wow_constants::{ConditionSourceType, ConditionType};

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
            requires_spell_focus: 0,
            effects: Vec::new(),
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
            requires_spell_focus: 0,
            effects: Vec::new(),
        };

        // GCD is the limit
        assert_eq!(instant.effective_cooldown_ms(), 1500);
    }

    #[test]
    fn spell_info_requires_spell_focus_matches_cpp_field() {
        let mut spell = SpellInfo {
            spell_id: 100,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            effects: Vec::new(),
        };

        assert!(!spell.requires_spell_focus_like_cpp());
        spell.requires_spell_focus = 181;
        assert!(spell.requires_spell_focus_like_cpp());
    }

    #[test]
    fn spell_implicit_target_effect_mask_normalizes_like_cpp_conditionmgr() {
        let spell = SpellInfo {
            spell_id: 100,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            effects: vec![
                SpellEffectInfo {
                    effect_index: 0,
                    effect: 0,
                    chain_targets: 0,
                    implicit_target_1: 6,
                    implicit_target_2: 0,
                    ..Default::default()
                },
                SpellEffectInfo {
                    effect_index: 1,
                    effect: 0,
                    chain_targets: 0,
                    implicit_target_1: 7,
                    implicit_target_2: 0,
                    ..Default::default()
                },
                SpellEffectInfo {
                    effect_index: 2,
                    effect: spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_RAID,
                    chain_targets: 0,
                    implicit_target_1: 0,
                    implicit_target_2: 0,
                    ..Default::default()
                },
                SpellEffectInfo {
                    effect_index: 3,
                    effect: 0,
                    chain_targets: 2,
                    implicit_target_1: 0,
                    implicit_target_2: 0,
                    ..Default::default()
                },
            ],
        };

        assert_eq!(
            spell.normalized_implicit_target_effect_mask_like_cpp(0b1111),
            0b1110
        );
        assert_eq!(
            spell.normalized_implicit_target_effect_mask_like_cpp(0b0001),
            0
        );
    }

    #[test]
    fn spell_effect_detects_mounted_aura_like_cpp() {
        let mounted = SpellEffectInfo {
            effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_aura: aura_types::SPELL_AURA_MOUNTED,
            effect_base_points: 11,
            effect_misc_value_1: 22,
            effect_misc_value_2: 33,
            ..Default::default()
        };
        let other_aura = SpellEffectInfo {
            effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_aura: aura_types::SPELL_AURA_HASTE_SPELLS,
            ..Default::default()
        };

        assert!(mounted.is_mounted_aura_like_cpp());
        assert!(!other_aura.is_mounted_aura_like_cpp());
        assert_eq!(mounted.effect_base_points, 11);
        assert_eq!(mounted.effect_misc_value_1, 22);
        assert_eq!(mounted.effect_misc_value_2, 33);
    }

    #[test]
    fn spell_effect_detects_provide_spell_focus_aura_like_cpp() {
        let focus = SpellEffectInfo {
            effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_aura: aura_types::SPELL_AURA_PROVIDE_SPELL_FOCUS,
            effect_misc_value_1: 181,
            ..Default::default()
        };
        let other_effect = SpellEffectInfo {
            effect: spell_effect_types::SPELL_EFFECT_HEAL,
            effect_aura: aura_types::SPELL_AURA_PROVIDE_SPELL_FOCUS,
            ..Default::default()
        };

        assert!(focus.is_provide_spell_focus_aura_like_cpp());
        assert!(!other_effect.is_provide_spell_focus_aura_like_cpp());
        assert_eq!(focus.effect_misc_value_1, 181);
    }

    #[test]
    fn spell_effect_detects_focus_destination_implicit_targets_like_cpp() {
        let mut effect = SpellEffectInfo {
            implicit_target_1: implicit_targets::TARGET_DEST_NEARBY_ENTRY,
            ..Default::default()
        };
        assert!(effect.has_focus_destination_implicit_target_like_cpp());

        effect.implicit_target_1 = 0;
        effect.implicit_target_2 = implicit_targets::TARGET_DEST_NEARBY_ENTRY_2;
        assert!(effect.has_focus_destination_implicit_target_like_cpp());

        effect.implicit_target_2 = implicit_targets::TARGET_DEST_NEARBY_ENTRY_OR_DB;
        assert!(effect.has_focus_destination_implicit_target_like_cpp());

        effect.implicit_target_2 = 40;
        assert!(!effect.has_focus_destination_implicit_target_like_cpp());
    }

    #[test]
    fn spell_implicit_target_conditions_attach_to_effects_like_cpp() {
        let mut store = SpellStore::new();
        store.insert(
            100,
            SpellInfo {
                spell_id: 100,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: 0,
                effect_base_points: 0,
                effect_bonus_coefficient: 0.0,
                aura_type: None,
                display_flags: 0,
                requires_spell_focus: 0,
                effects: vec![
                    SpellEffectInfo {
                        effect_index: 0,
                        effect: 0,
                        chain_targets: 0,
                        implicit_target_1: 6,
                        implicit_target_2: 0,
                        ..Default::default()
                    },
                    SpellEffectInfo {
                        effect_index: 1,
                        effect: 0,
                        chain_targets: 0,
                        implicit_target_1: 7,
                        implicit_target_2: 0,
                        ..Default::default()
                    },
                ],
            },
        );
        let conditions = ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
            source_type: ConditionSourceType::SpellImplicitTarget,
            source_group: 0b11,
            source_entry: 100,
            condition_type: ConditionType::Aura,
            ..Condition::default()
        }]);

        assert_eq!(
            store.attach_spell_implicit_target_conditions_like_cpp(&conditions),
            2
        );
        assert!(
            store
                .implicit_target_conditions_like_cpp(100, 0)
                .and_then(|reference| reference.upgrade())
                .is_some_and(|conditions| conditions.len() == 1)
        );
        assert!(
            store
                .implicit_target_conditions_like_cpp(100, 1)
                .and_then(|reference| reference.upgrade())
                .is_some_and(|conditions| conditions.len() == 1)
        );
    }
}
