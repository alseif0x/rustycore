//! Compact effective metadata used to plan player spell acquisition.
//!
//! This is intentionally not another general `SpellInfo` store.  It composes
//! the seven DB2 families needed by acquisition in the same order as
//! `DB2StorageBase::LoadFromDB` and `DB2Manager::LoadHotfixData`:
//!
//! ```text
//! WDC4 -> official SQL -> custom SQL -> final RecordRemoved
//! ```
//!
//! Overlay payload is kept raw until composition is complete.  Consequently
//! an invalid official/custom row still replaces the older row with the same
//! record id and fails closed; it can only be repaired by a later overlay.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result};
use wow_database::{HotfixDatabase, HotfixStatements, SqlResult};

use crate::{Db2HotfixRemovalStoreLikeCpp, wdc4::Wdc4Reader};

const DIFFICULTY_NONE_LIKE_CPP: u32 = 0;
const SPELL_EFFECT_SUMMON_LIKE_CPP: u32 = 28;
const SPELL_EFFECT_LEARN_SPELL_LIKE_CPP: u32 = 36;
const SPELL_EFFECT_DUAL_WIELD_LIKE_CPP: u32 = 40;
const SPELL_EFFECT_SKILL_STEP_LIKE_CPP: u32 = 44;
const SPELL_EFFECT_SKILL_LIKE_CPP: u32 = 118;
const MAX_SPELL_EFFECTS_LIKE_CPP: i64 = 32;
const TOTAL_SPELL_EFFECTS_LIKE_CPP: i64 = 316;
const TOTAL_SPELL_TARGETS_LIKE_CPP: i64 = 153;
const TARGET_UNIT_PET_LIKE_CPP: i64 = 5;
const TARGET_NONE_LIKE_CPP: i64 = 0;
const TARGET_UNIT_CASTER_LIKE_CPP: i64 = 1;
const TARGET_UNIT_TARGET_ALLY_LIKE_CPP: i64 = 21;
const SPELL_ATTR0_PASSIVE_LIKE_CPP: u32 = 0x0000_0040;
const SPELL_ATTR1_CAST_WHEN_LEARNED_LIKE_CPP: u32 = 0x8000_0000;
const SUMMON_SLOT_MINIPET_LIKE_CPP: i64 = 5;
const SUMMON_FROM_BATTLE_PET_JOURNAL_LIKE_CPP: u32 = 0x0020_0000;

// C++ `SpellEffectEntry::EffectBasePoints` is `int32`
// (`DB2Structure.h` / `SpellEffectLoadInfo`), and the hotfix
// `spell_effect.EffectBasePoints` column has the same signed integer domain.
// Do not confuse it with `world.serverside_spell_effect.EffectBasePoints`,
// which is a float source outside this catalog: server-side spell keys are
// explicitly seeded as `ServerSideMetadataUnavailable`.
const SPELL_EFFECT_SQL: &str = concat!(
    "SELECT ID, DifficultyID, EffectIndex, Effect, EffectBasePoints, EffectDieSides, ",
    "EffectTriggerSpell, EffectMiscValue1, EffectMiscValue2, ImplicitTarget1, ",
    "ImplicitTarget2, Coefficient, Variance, SpellID, EffectChainTargets, ",
    "EffectPointsPerResource, EffectRealPointsPerLevel FROM spell_effect ",
    "WHERE (`VerifiedBuild` > 0) = ?"
);
const SPELL_EFFECT_WDC_CHAIN_TARGETS_FIELD: usize = 10;
const SPELL_EFFECT_WDC_POINTS_PER_RESOURCE_FIELD: usize = 14;
const SPELL_EFFECT_WDC_REAL_POINTS_PER_LEVEL_FIELD: usize = 16;
const SPELL_EFFECT_SQL_CHAIN_TARGETS_COLUMN: usize = 14;
const SPELL_EFFECT_SQL_POINTS_PER_RESOURCE_COLUMN: usize = 15;
const SPELL_EFFECT_SQL_REAL_POINTS_PER_LEVEL_COLUMN: usize = 16;
const SPELL_LEARN_SPELL_SQL: &str = concat!(
    "SELECT ID, SpellID, LearnSpellID, OverridesSpellID FROM spell_learn_spell ",
    "WHERE (`VerifiedBuild` > 0) = ?"
);
const SPELL_MISC_SQL: &str = concat!(
    "SELECT ID, Attributes1, Attributes2, DifficultyID, ",
    "ShowFutureSpellPlayerConditionID, SpellID FROM spell_misc ",
    "WHERE (`VerifiedBuild` > 0) = ?"
);
const SPELL_LEVELS_SQL: &str = concat!(
    "SELECT ID, DifficultyID, BaseLevel, SpellLevel, SpellID FROM spell_levels ",
    "WHERE (`VerifiedBuild` > 0) = ?"
);
const TALENT_SQL: &str = concat!(
    "SELECT ID, SpellRank1, SpellRank2, SpellRank3, SpellRank4, SpellRank5, ",
    "SpellRank6, SpellRank7, SpellRank8, SpellRank9 FROM talent ",
    "WHERE (`VerifiedBuild` > 0) = ?"
);
const SUMMON_PROPERTIES_SQL: &str = concat!(
    "SELECT ID, Slot, Flags1 FROM summon_properties ",
    "WHERE (`VerifiedBuild` > 0) = ?"
);
const BATTLE_PET_SPECIES_SQL: &str = concat!(
    "SELECT ID, CreatureID FROM battle_pet_species ",
    "WHERE (`VerifiedBuild` > 0) = ?"
);

/// The source families retained by this specialized projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SpellAcquisitionTableLikeCpp {
    SpellEffect,
    SpellLearnSpell,
    SpellMisc,
    SpellLevels,
    Talent,
    SummonProperties,
    BattlePetSpecies,
}

impl SpellAcquisitionTableLikeCpp {
    const ALL: [Self; 7] = [
        Self::SpellEffect,
        Self::SpellLearnSpell,
        Self::SpellMisc,
        Self::SpellLevels,
        Self::Talent,
        Self::SummonProperties,
        Self::BattlePetSpecies,
    ];

    const fn file_name(self) -> &'static str {
        match self {
            Self::SpellEffect => "SpellEffect.db2",
            Self::SpellLearnSpell => "SpellLearnSpell.db2",
            Self::SpellMisc => "SpellMisc.db2",
            Self::SpellLevels => "SpellLevels.db2",
            Self::Talent => "Talent.db2",
            Self::SummonProperties => "SummonProperties.db2",
            Self::BattlePetSpecies => "BattlePetSpecies.db2",
        }
    }
}

/// Runtime WDC4 table hashes used for the final removal pass.
///
/// No production hash is compiled into this module.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SpellAcquisitionTableHashesLikeCpp {
    pub spell_effect: u32,
    pub spell_learn_spell: u32,
    pub spell_misc: u32,
    pub spell_levels: u32,
    pub talent: u32,
    pub summon_properties: u32,
    pub battle_pet_species: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellAcquisitionDiagnosticSeverityLikeCpp {
    Warning,
    Indeterminate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellAcquisitionDiagnosticKindLikeCpp {
    UnreadableSqlField {
        field: &'static str,
    },
    InvalidField {
        field: &'static str,
        raw: i64,
        expected: &'static str,
    },
    EffectSlotCollisionResolved {
        spell_id: u32,
        difficulty_id: u32,
        effect_index: u8,
        replaced_record_id: u32,
        winning_record_id: u32,
    },
    MetadataCollisionResolved {
        spell_id: u32,
        difficulty_id: u32,
        replaced_record_id: u32,
        winning_record_id: u32,
    },
    ConflictingSpeciesForCreature {
        creature_id: u32,
        species_ids: Vec<u32>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellAcquisitionDiagnosticLikeCpp {
    pub severity: SpellAcquisitionDiagnosticSeverityLikeCpp,
    pub table: SpellAcquisitionTableLikeCpp,
    pub record_id: Option<u32>,
    pub kind: SpellAcquisitionDiagnosticKindLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidAcquisitionValueLikeCpp {
    pub field: &'static str,
    pub raw: i64,
    pub expected: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellAcquisitionIndeterminateReasonLikeCpp {
    ServerSideMetadataUnavailable,
    EffectivePayloadUnavailable,
    InvalidEffectiveRow {
        table: SpellAcquisitionTableLikeCpp,
        record_id: u32,
        field: &'static str,
        raw: i64,
    },
    EffectiveTableIncomplete {
        table: SpellAcquisitionTableLikeCpp,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellAcquisitionSourceCoverageLikeCpp {
    Covered,
    Indeterminate(SpellAcquisitionIndeterminateReasonLikeCpp),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellAcquisitionCoverageSeedLikeCpp {
    pub spell_id: u32,
    pub difficulty_id: u32,
    pub source: SpellAcquisitionSourceCoverageLikeCpp,
}

impl SpellAcquisitionCoverageSeedLikeCpp {
    pub const fn covered(spell_id: u32, difficulty_id: u32) -> Self {
        Self {
            spell_id,
            difficulty_id,
            source: SpellAcquisitionSourceCoverageLikeCpp::Covered,
        }
    }

    pub const fn indeterminate(
        spell_id: u32,
        difficulty_id: u32,
        reason: SpellAcquisitionIndeterminateReasonLikeCpp,
    ) -> Self {
        Self {
            spell_id,
            difficulty_id,
            source: SpellAcquisitionSourceCoverageLikeCpp::Indeterminate(reason),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellAcquisitionEffectsLookupLikeCpp<'a> {
    MissingCoverage,
    Indeterminate(&'a [SpellAcquisitionIndeterminateReasonLikeCpp]),
    /// An empty slice means covered-with-zero-acquisition-effects.
    Covered(&'a [SpellAcquisitionEffectLikeCpp]),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellAcquisitionResolvedEffectsLookupLikeCpp<'a> {
    MissingCoverage {
        difficulty_id: u32,
    },
    Indeterminate(Vec<SpellAcquisitionIndeterminateReasonLikeCpp>),
    /// Slots are ordered by `EffectIndex`. An empty vector is covered with
    /// zero effects across the complete requested fallback chain.
    Covered(Vec<&'a SpellAcquisitionEffectLikeCpp>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellAcquisitionMetadataLookupLikeCpp<'a, T> {
    MissingCoverage,
    Indeterminate(&'a [SpellAcquisitionIndeterminateReasonLikeCpp]),
    CoveredWithoutRow,
    Present(&'a T),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellAcquisitionDependenciesLookupLikeCpp<'a> {
    MissingCoverage,
    Indeterminate(&'a [SpellAcquisitionIndeterminateReasonLikeCpp]),
    Covered(&'a [SpellAcquisitionDependencyLikeCpp]),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellAcquisitionResolvedMetadataLookupLikeCpp<'a, T> {
    MissingCoverage { difficulty_id: u32 },
    Indeterminate(Vec<SpellAcquisitionIndeterminateReasonLikeCpp>),
    CoveredWithoutRow,
    Present(&'a T),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellAcquisitionTalentLookupLikeCpp<'a> {
    Indeterminate(&'a [SpellAcquisitionIndeterminateReasonLikeCpp]),
    NotTalent,
    Talent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcquisitionValueDomainLikeCpp {
    pub minimum: i32,
    pub maximum: i32,
}

impl AcquisitionValueDomainLikeCpp {
    pub const fn deterministic_value(self) -> Option<i32> {
        if self.minimum == self.maximum {
            Some(self.minimum)
        } else {
            None
        }
    }
}

/// Final compact `SpellEffect` payload. Signed source values remain raw until
/// a consumer requests the corresponding checked domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellAcquisitionEffectLikeCpp {
    pub record_id: u32,
    pub spell_id_raw: i64,
    pub difficulty_id_raw: i64,
    pub effect_index_raw: i64,
    pub effect_type_raw: i64,
    /// Raw regular DB2/hotfix `SpellEffectEntry::EffectBasePoints` (`int32`).
    ///
    /// C++ promotes this integer into `SpellEffectInfo::BasePoints` only
    /// after loading; `base_points_die_sides_domain_checked` mirrors that
    /// promotion and the subsequent `CalcBaseValue(nullptr)` rounding.
    pub effect_base_points_raw: i64,
    pub effect_die_sides_raw: i64,
    pub effect_chain_targets_raw: i64,
    pub effect_points_per_resource_bits: u32,
    pub effect_real_points_per_level_bits: u32,
    /// Exact IEEE-754 payloads. Keeping bits avoids normalizing NaNs while
    /// retaining `Eq` for deterministic plans and fixtures.
    pub effect_coefficient_bits: u32,
    pub effect_variance_bits: u32,
    pub effect_trigger_spell_raw: i64,
    pub effect_misc_value_raw: [i64; 2],
    pub implicit_target_raw: [i64; 2],
}

impl SpellAcquisitionEffectLikeCpp {
    pub fn spell_id_checked(&self) -> Result<u32, InvalidAcquisitionValueLikeCpp> {
        positive_u32(self.spell_id_raw, "SpellEffect.SpellID")
    }

    pub fn difficulty_id_checked(&self) -> Result<u32, InvalidAcquisitionValueLikeCpp> {
        source_i32(self.difficulty_id_raw, "SpellEffect.DifficultyID")?;
        checked_u8(self.difficulty_id_raw, "SpellEffect.DifficultyID").map(u32::from)
    }

    pub fn effect_index_checked(&self) -> Result<u8, InvalidAcquisitionValueLikeCpp> {
        let index = checked_u8(self.effect_index_raw, "SpellEffect.EffectIndex")?;
        if i64::from(index) >= MAX_SPELL_EFFECTS_LIKE_CPP {
            return Err(invalid(
                "SpellEffect.EffectIndex",
                self.effect_index_raw,
                "0..MAX_SPELL_EFFECTS",
            ));
        }
        Ok(index)
    }

    pub fn effect_type_checked(&self) -> Result<u32, InvalidAcquisitionValueLikeCpp> {
        let effect = nonnegative_u32(self.effect_type_raw, "SpellEffect.Effect")?;
        if i64::from(effect) >= TOTAL_SPELL_EFFECTS_LIKE_CPP {
            return Err(invalid(
                "SpellEffect.Effect",
                self.effect_type_raw,
                "0..TOTAL_SPELL_EFFECTS",
            ));
        }
        Ok(effect)
    }

    pub fn trigger_spell_id_checked(&self) -> Result<u32, InvalidAcquisitionValueLikeCpp> {
        source_i32(
            self.effect_trigger_spell_raw,
            "SpellEffect.EffectTriggerSpell",
        )?;
        positive_u32(
            self.effect_trigger_spell_raw,
            "SpellEffect.EffectTriggerSpell",
        )
    }

    pub fn misc_value_id_checked(
        &self,
        index: usize,
    ) -> Result<u32, InvalidAcquisitionValueLikeCpp> {
        let raw = self.effect_misc_value_raw[index];
        source_i32(raw, "SpellEffect.EffectMiscValue")?;
        positive_u32(raw, "SpellEffect.EffectMiscValue")
    }

    pub fn targets_unit_pet_like_cpp(&self) -> bool {
        self.implicit_target_raw[0] == TARGET_UNIT_PET_LIKE_CPP
    }

    pub fn targets_player_like_cpp(&self) -> bool {
        self.implicit_target_raw.iter().all(|target| {
            matches!(
                *target,
                TARGET_NONE_LIKE_CPP
                    | TARGET_UNIT_CASTER_LIKE_CPP
                    | TARGET_UNIT_TARGET_ALLY_LIKE_CPP
            )
        })
    }

    pub fn coefficient_checked(&self) -> Result<f32, InvalidAcquisitionValueLikeCpp> {
        finite_f32_from_bits(self.effect_coefficient_bits, "SpellEffect.Coefficient")
    }

    pub fn variance_checked(&self) -> Result<f32, InvalidAcquisitionValueLikeCpp> {
        finite_f32_from_bits(self.effect_variance_bits, "SpellEffect.Variance")
    }

    /// Unscaled `BasePoints + DieSides` domain used by startup learn-skill
    /// projection. A non-singleton domain is deliberately not guessed.
    pub fn base_points_die_sides_domain_checked(
        &self,
    ) -> Result<AcquisitionValueDomainLikeCpp, InvalidAcquisitionValueLikeCpp> {
        let source_base = source_i32(self.effect_base_points_raw, "SpellEffect.EffectBasePoints")?;
        let coefficient = self.coefficient_checked()?;
        let variance = self.variance_checked()?;
        // `SpellEffectInfo::Scaling.Class` is always zero in this legacy.
        // Therefore `CalcBaseValue(nullptr, ...)` returns zero whenever the
        // effective coefficient is nonzero; otherwise it rounds BasePoints
        // after the source i32 was converted to f32.
        let base = if coefficient != 0.0 {
            0
        } else {
            let rounded = f64::from((source_base as f32).round());
            if !(f64::from(i32::MIN)..=f64::from(i32::MAX)).contains(&rounded) {
                return Err(invalid(
                    "SpellEffect.EffectBasePoints",
                    self.effect_base_points_raw,
                    "f32-rounded i32 result",
                ));
            }
            rounded as i32
        };
        let die = source_i32(self.effect_die_sides_raw, "SpellEffect.EffectDieSides")?;
        let (minimum_die, maximum_die) = match die {
            0 => (0.0_f64, 0.0_f64),
            1 => (1.0, 1.0),
            value if value > 1 => (1.0, f64::from(value)),
            value => (f64::from(value), 1.0),
        };
        // C++ computes `delta` in f32 and `frand(-delta, delta)` uses
        // `uniform_real_distribution<float>`, whose upper bound is exclusive.
        // Promote the actual reachable f32 endpoints to double, apply
        // DieSides, and only then round the final value like CalcValue().
        let variance_delta = (variance * 0.5).abs();
        let (minimum_with_variance, maximum_with_variance) = if variance_delta == 0.0 {
            (f64::from(base), f64::from(base))
        } else {
            let lower_sample = -variance_delta;
            let upper_sample = f32::from_bits(variance_delta.to_bits() - 1);
            let lower_value = f64::from(base) + f64::from(base) * f64::from(lower_sample);
            let upper_value = f64::from(base) + f64::from(base) * f64::from(upper_sample);
            (lower_value.min(upper_value), lower_value.max(upper_value))
        };
        let minimum = checked_rounded_i32(
            minimum_with_variance + minimum_die,
            self.effect_base_points_raw,
        )?;
        let maximum = checked_rounded_i32(
            maximum_with_variance + maximum_die,
            self.effect_base_points_raw,
        )?;
        Ok(AcquisitionValueDomainLikeCpp { minimum, maximum })
    }
}

fn checked_rounded_i32(
    value: f64,
    raw_base_points: i64,
) -> Result<i32, InvalidAcquisitionValueLikeCpp> {
    let rounded = value.round();
    if rounded.is_finite() && (f64::from(i32::MIN)..=f64::from(i32::MAX)).contains(&rounded) {
        return Ok(rounded as i32);
    }
    Err(invalid(
        "SpellEffect.EffectBasePoints+EffectDieSides+Variance",
        raw_base_points,
        "i32 result",
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellAcquisitionDependencyLikeCpp {
    pub record_id: u32,
    pub spell_id_raw: i64,
    pub learn_spell_id_raw: i64,
    pub overrides_spell_id_raw: i64,
}

impl SpellAcquisitionDependencyLikeCpp {
    pub fn spell_id_checked(&self) -> Result<u32, InvalidAcquisitionValueLikeCpp> {
        source_i32(self.spell_id_raw, "SpellLearnSpell.SpellID")?;
        positive_u32(self.spell_id_raw, "SpellLearnSpell.SpellID")
    }

    pub fn learned_spell_id_checked(&self) -> Result<u32, InvalidAcquisitionValueLikeCpp> {
        source_i32(self.learn_spell_id_raw, "SpellLearnSpell.LearnSpellID")?;
        positive_u32(self.learn_spell_id_raw, "SpellLearnSpell.LearnSpellID")
    }

    pub fn overrides_spell_id_checked(
        &self,
    ) -> Result<Option<u32>, InvalidAcquisitionValueLikeCpp> {
        source_i32(
            self.overrides_spell_id_raw,
            "SpellLearnSpell.OverridesSpellID",
        )?;
        optional_positive_u32(
            self.overrides_spell_id_raw,
            "SpellLearnSpell.OverridesSpellID",
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellAcquisitionMiscLikeCpp {
    pub record_id: u32,
    pub spell_id_raw: i64,
    pub difficulty_id_raw: i64,
    pub attributes_raw: [i64; 2],
    pub show_future_spell_player_condition_id_raw: i64,
}

impl SpellAcquisitionMiscLikeCpp {
    pub fn is_passive_checked(&self) -> Result<bool, InvalidAcquisitionValueLikeCpp> {
        Ok(
            checked_u32_bits(self.attributes_raw[0], "SpellMisc.Attributes1")?
                & SPELL_ATTR0_PASSIVE_LIKE_CPP
                != 0,
        )
    }

    pub fn cast_when_learned_checked(&self) -> Result<bool, InvalidAcquisitionValueLikeCpp> {
        Ok(
            checked_u32_bits(self.attributes_raw[1], "SpellMisc.Attributes2")?
                & SPELL_ATTR1_CAST_WHEN_LEARNED_LIKE_CPP
                != 0,
        )
    }

    pub fn future_player_condition_id_checked(
        &self,
    ) -> Result<Option<u32>, InvalidAcquisitionValueLikeCpp> {
        source_i32(
            self.show_future_spell_player_condition_id_raw,
            "SpellMisc.ShowFutureSpellPlayerConditionID",
        )?;
        optional_positive_u32(
            self.show_future_spell_player_condition_id_raw,
            "SpellMisc.ShowFutureSpellPlayerConditionID",
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellAcquisitionLevelsLikeCpp {
    pub record_id: u32,
    pub spell_id_raw: i64,
    pub difficulty_id_raw: i64,
    pub base_level_raw: i64,
    pub spell_level_raw: i64,
}

impl SpellAcquisitionLevelsLikeCpp {
    pub fn base_level_checked(&self) -> Result<i16, InvalidAcquisitionValueLikeCpp> {
        i16::try_from(self.base_level_raw)
            .map_err(|_| invalid("SpellLevels.BaseLevel", self.base_level_raw, "i16"))
    }

    pub fn spell_level_checked(&self) -> Result<i16, InvalidAcquisitionValueLikeCpp> {
        i16::try_from(self.spell_level_raw)
            .map_err(|_| invalid("SpellLevels.SpellLevel", self.spell_level_raw, "i16"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellAcquisitionTalentLikeCpp {
    pub record_id: u32,
    pub spell_rank_raw: [i64; 9],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellAcquisitionSummonPropertiesLikeCpp {
    pub record_id: u32,
    pub slot_raw: i64,
    pub flags_1_raw: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellAcquisitionBattlePetSpeciesLikeCpp {
    /// The DB2 record id is the canonical species id.
    pub species_id: u32,
    pub creature_id_raw: i64,
}

/// Final `RecordRemoved` evidence retained after effective composition.
///
/// A typed payload means the row existed before the final removal pass and
/// preserves enough relation data for diagnostics. `Unknown` records a
/// tombstone whose ID had no WDC4/SQL payload; it is evidence only and must
/// never be associated with an arbitrary spell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellAcquisitionRemovedRowLikeCpp {
    SpellEffect(SpellAcquisitionEffectLikeCpp),
    SpellLearnSpell(SpellAcquisitionDependencyLikeCpp),
    SpellMisc(SpellAcquisitionMiscLikeCpp),
    SpellLevels(SpellAcquisitionLevelsLikeCpp),
    Talent(SpellAcquisitionTalentLikeCpp),
    SummonProperties(SpellAcquisitionSummonPropertiesLikeCpp),
    BattlePetSpecies(SpellAcquisitionBattlePetSpeciesLikeCpp),
    Unknown {
        table: SpellAcquisitionTableLikeCpp,
        record_id: i32,
    },
}

impl SpellAcquisitionRemovedRowLikeCpp {
    pub const fn table_like_cpp(&self) -> SpellAcquisitionTableLikeCpp {
        match self {
            Self::SpellEffect(_) => SpellAcquisitionTableLikeCpp::SpellEffect,
            Self::SpellLearnSpell(_) => SpellAcquisitionTableLikeCpp::SpellLearnSpell,
            Self::SpellMisc(_) => SpellAcquisitionTableLikeCpp::SpellMisc,
            Self::SpellLevels(_) => SpellAcquisitionTableLikeCpp::SpellLevels,
            Self::Talent(_) => SpellAcquisitionTableLikeCpp::Talent,
            Self::SummonProperties(_) => SpellAcquisitionTableLikeCpp::SummonProperties,
            Self::BattlePetSpecies(_) => SpellAcquisitionTableLikeCpp::BattlePetSpecies,
            Self::Unknown { table, .. } => *table,
        }
    }

    pub const fn record_id_like_cpp(&self) -> i64 {
        match self {
            Self::SpellEffect(row) => row.record_id as i64,
            Self::SpellLearnSpell(row) => row.record_id as i64,
            Self::SpellMisc(row) => row.record_id as i64,
            Self::SpellLevels(row) => row.record_id as i64,
            Self::Talent(row) => row.record_id as i64,
            Self::SummonProperties(row) => row.record_id as i64,
            Self::BattlePetSpecies(row) => row.species_id as i64,
            Self::Unknown { record_id, .. } => *record_id as i64,
        }
    }

    const fn hotfix_record_id_like_cpp(&self) -> i32 {
        match self {
            Self::SpellEffect(row) => row.record_id as i32,
            Self::SpellLearnSpell(row) => row.record_id as i32,
            Self::SpellMisc(row) => row.record_id as i32,
            Self::SpellLevels(row) => row.record_id as i32,
            Self::Talent(row) => row.record_id as i32,
            Self::SummonProperties(row) => row.record_id as i32,
            Self::BattlePetSpecies(row) => row.species_id as i32,
            Self::Unknown { record_id, .. } => *record_id,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ComposedEffectiveTableLikeCpp<T> {
    pub effective_rows: BTreeMap<u32, T>,
    pub removed_rows: BTreeMap<u32, T>,
}

/// Pure generic implementation of DB2 record replacement and final removals.
///
/// Values are not validated here by design: an invalid overlay must replace,
/// not accidentally reveal, an older valid payload.
pub fn compose_effective_table_like_cpp<T>(
    base_rows: impl IntoIterator<Item = (u32, T)>,
    official_rows: impl IntoIterator<Item = (u32, T)>,
    custom_rows: impl IntoIterator<Item = (u32, T)>,
    table_hash: u32,
    removed_records: &Db2HotfixRemovalStoreLikeCpp,
) -> BTreeMap<u32, T> {
    compose_effective_table_with_removed_like_cpp(
        base_rows,
        official_rows,
        custom_rows,
        table_hash,
        removed_records,
    )
    .effective_rows
}

pub fn compose_effective_table_with_removed_like_cpp<T>(
    base_rows: impl IntoIterator<Item = (u32, T)>,
    official_rows: impl IntoIterator<Item = (u32, T)>,
    custom_rows: impl IntoIterator<Item = (u32, T)>,
    table_hash: u32,
    removed_records: &Db2HotfixRemovalStoreLikeCpp,
) -> ComposedEffectiveTableLikeCpp<T> {
    let mut effective: BTreeMap<_, _> = base_rows.into_iter().collect();
    effective.extend(official_rows);
    effective.extend(custom_rows);
    let removed_ids = effective
        .keys()
        .copied()
        .filter(|record_id| removed_records.contains_like_cpp(table_hash, *record_id as i32))
        .collect::<Vec<_>>();
    let mut removed_rows = BTreeMap::new();
    for record_id in removed_ids {
        if let Some(row) = effective.remove(&record_id) {
            removed_rows.insert(record_id, row);
        }
    }
    ComposedEffectiveTableLikeCpp {
        effective_rows: effective,
        removed_rows,
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EffectiveSpellAcquisitionRowsLikeCpp {
    pub spell_effects: Vec<SpellAcquisitionEffectLikeCpp>,
    pub spell_learn_spells: Vec<SpellAcquisitionDependencyLikeCpp>,
    pub spell_misc: Vec<SpellAcquisitionMiscLikeCpp>,
    pub spell_levels: Vec<SpellAcquisitionLevelsLikeCpp>,
    pub talents: Vec<SpellAcquisitionTalentLikeCpp>,
    pub summon_properties: Vec<SpellAcquisitionSummonPropertiesLikeCpp>,
    pub battle_pet_species: Vec<SpellAcquisitionBattlePetSpeciesLikeCpp>,
}

#[derive(Debug, Clone, Default)]
struct CoverageRecordLikeCpp {
    reasons_by_table:
        BTreeMap<SpellAcquisitionTableLikeCpp, Vec<SpellAcquisitionIndeterminateReasonLikeCpp>>,
}

impl CoverageRecordLikeCpp {
    fn add_source_reason_like_cpp(&mut self, reason: SpellAcquisitionIndeterminateReasonLikeCpp) {
        for table in SpellAcquisitionTableLikeCpp::ALL {
            self.add_table_reason_like_cpp(table, reason.clone());
        }
    }

    fn add_table_reason_like_cpp(
        &mut self,
        table: SpellAcquisitionTableLikeCpp,
        reason: SpellAcquisitionIndeterminateReasonLikeCpp,
    ) {
        let reasons = self.reasons_by_table.entry(table).or_default();
        if !reasons.contains(&reason) {
            reasons.push(reason);
        }
    }

    fn reasons_for_table_like_cpp(
        &self,
        table: SpellAcquisitionTableLikeCpp,
    ) -> &[SpellAcquisitionIndeterminateReasonLikeCpp] {
        self.reasons_by_table
            .get(&table)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}

#[derive(Debug, Clone)]
pub struct SpellAcquisitionCatalogLikeCpp {
    table_hashes: SpellAcquisitionTableHashesLikeCpp,
    coverage_by_key: BTreeMap<(u32, u32), CoverageRecordLikeCpp>,
    effects_by_key: BTreeMap<(u32, u32), Vec<SpellAcquisitionEffectLikeCpp>>,
    acquisition_effects_by_key: BTreeMap<(u32, u32), Vec<SpellAcquisitionEffectLikeCpp>>,
    summon_effects_by_spell: BTreeMap<u32, Vec<SpellAcquisitionEffectLikeCpp>>,
    dependencies_by_spell: BTreeMap<u32, Vec<SpellAcquisitionDependencyLikeCpp>>,
    dependency_rows: Vec<SpellAcquisitionDependencyLikeCpp>,
    misc_by_key: BTreeMap<(u32, u32), SpellAcquisitionMiscLikeCpp>,
    levels_by_key: BTreeMap<(u32, u32), SpellAcquisitionLevelsLikeCpp>,
    talent_spell_ids: BTreeSet<u32>,
    summon_properties_by_id: BTreeMap<u32, SpellAcquisitionSummonPropertiesLikeCpp>,
    species_by_creature: BTreeMap<u32, BTreeSet<u32>>,
    removed_rows: Vec<SpellAcquisitionRemovedRowLikeCpp>,
    removed_summon_properties_ids: BTreeSet<u32>,
    removed_species_by_creature: BTreeMap<u32, BTreeSet<u32>>,
    global_indeterminate_by_table:
        BTreeMap<SpellAcquisitionTableLikeCpp, Vec<SpellAcquisitionIndeterminateReasonLikeCpp>>,
    diagnostics: Vec<SpellAcquisitionDiagnosticLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BattlePetClassificationLikeCpp {
    NotBattlePet,
    Species(u32),
    Indeterminate(Vec<BattlePetIndeterminateReasonLikeCpp>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BattlePetIndeterminateReasonLikeCpp {
    MissingSpellCoverage {
        spell_id: u32,
    },
    MissingSpellDifficultyCoverage {
        spell_id: u32,
        difficulty_id: u32,
        effect_record_id: u32,
    },
    SpellCoverage {
        spell_id: u32,
        reason: SpellAcquisitionIndeterminateReasonLikeCpp,
    },
    EffectiveTableIncomplete {
        table: SpellAcquisitionTableLikeCpp,
        reason: SpellAcquisitionIndeterminateReasonLikeCpp,
    },
    InvalidSummonEffect {
        record_id: u32,
        field: &'static str,
        raw: i64,
    },
    MissingSummonProperties {
        effect_record_id: u32,
        properties_id: u32,
    },
    RemovedSummonProperties {
        effect_record_id: u32,
        properties_id: u32,
    },
    InvalidSummonProperties {
        effect_record_id: u32,
        properties_id: u32,
        field: &'static str,
        raw: i64,
    },
    MissingSpeciesForCreature {
        effect_record_id: u32,
        creature_id: u32,
    },
    RemovedSpeciesForCreature {
        effect_record_id: u32,
        creature_id: u32,
        species_ids: Vec<u32>,
    },
    ConflictingSpeciesForCreature {
        effect_record_id: u32,
        creature_id: u32,
        species_ids: Vec<u32>,
    },
    ConflictingSpeciesForSpell {
        spell_id: u32,
        species_ids: Vec<u32>,
    },
}

impl SpellAcquisitionCatalogLikeCpp {
    /// Whether the canonical spell-info key exists, independently of whether
    /// every acquisition table for that key is hydrated.  This is the narrow
    /// equivalent needed for C++ `GetSpellInfo` short-circuits before later
    /// gates decide whether full metadata is required.
    pub fn contains_spell_difficulty_key_like_cpp(
        &self,
        spell_id: u32,
        difficulty_id: u32,
    ) -> bool {
        self.coverage_by_key
            .contains_key(&(spell_id, difficulty_id))
    }

    /// Build every derived index from already-final effective rows.
    ///
    /// Rows are sorted by record id before projection. For duplicate
    /// `(spell, difficulty, effect index)` slots this reproduces the C++ DB2
    /// iteration result: the higher final record id wins. Every SUMMON row is
    /// retained separately because C++ builds its battle-pet map while
    /// iterating the store, before assigning the effect slot.
    pub fn from_effective_rows_like_cpp(
        coverage: impl IntoIterator<Item = SpellAcquisitionCoverageSeedLikeCpp>,
        rows: EffectiveSpellAcquisitionRowsLikeCpp,
        table_hashes: SpellAcquisitionTableHashesLikeCpp,
        diagnostics: Vec<SpellAcquisitionDiagnosticLikeCpp>,
    ) -> Self {
        Self::from_effective_rows_and_removed_like_cpp(
            coverage,
            rows,
            Vec::new(),
            table_hashes,
            diagnostics,
        )
    }

    pub fn from_effective_rows_and_removed_like_cpp(
        coverage: impl IntoIterator<Item = SpellAcquisitionCoverageSeedLikeCpp>,
        mut rows: EffectiveSpellAcquisitionRowsLikeCpp,
        mut removed_rows: Vec<SpellAcquisitionRemovedRowLikeCpp>,
        table_hashes: SpellAcquisitionTableHashesLikeCpp,
        mut diagnostics: Vec<SpellAcquisitionDiagnosticLikeCpp>,
    ) -> Self {
        removed_rows.sort_by_key(|row| (row.table_like_cpp(), row.record_id_like_cpp()));
        let mut catalog = Self {
            table_hashes,
            coverage_by_key: BTreeMap::new(),
            effects_by_key: BTreeMap::new(),
            acquisition_effects_by_key: BTreeMap::new(),
            summon_effects_by_spell: BTreeMap::new(),
            dependencies_by_spell: BTreeMap::new(),
            dependency_rows: Vec::new(),
            misc_by_key: BTreeMap::new(),
            levels_by_key: BTreeMap::new(),
            talent_spell_ids: BTreeSet::new(),
            summon_properties_by_id: BTreeMap::new(),
            species_by_creature: BTreeMap::new(),
            removed_rows,
            removed_summon_properties_ids: BTreeSet::new(),
            removed_species_by_creature: BTreeMap::new(),
            global_indeterminate_by_table: BTreeMap::new(),
            diagnostics: Vec::new(),
        };

        for removed in &catalog.removed_rows {
            match removed {
                SpellAcquisitionRemovedRowLikeCpp::SummonProperties(row) => {
                    catalog.removed_summon_properties_ids.insert(row.record_id);
                }
                SpellAcquisitionRemovedRowLikeCpp::BattlePetSpecies(row) => {
                    if let Ok(creature_id) =
                        source_i32(row.creature_id_raw, "BattlePetSpecies.CreatureID")
                        && creature_id > 0
                    {
                        catalog
                            .removed_species_by_creature
                            .entry(creature_id as u32)
                            .or_default()
                            .insert(row.species_id);
                    }
                }
                SpellAcquisitionRemovedRowLikeCpp::Unknown {
                    table: SpellAcquisitionTableLikeCpp::SummonProperties,
                    record_id,
                } if *record_id > 0 => {
                    catalog
                        .removed_summon_properties_ids
                        .insert(*record_id as u32);
                }
                _ => {}
            }
        }

        for diagnostic in &diagnostics {
            if diagnostic.severity == SpellAcquisitionDiagnosticSeverityLikeCpp::Indeterminate
                && diagnostic.record_id.is_none()
                && matches!(
                    &diagnostic.kind,
                    SpellAcquisitionDiagnosticKindLikeCpp::UnreadableSqlField { .. }
                )
            {
                catalog.push_global_reason_like_cpp(
                    diagnostic.table,
                    SpellAcquisitionIndeterminateReasonLikeCpp::EffectiveTableIncomplete {
                        table: diagnostic.table,
                    },
                );
            }
        }

        for seed in coverage {
            let entry = catalog
                .coverage_by_key
                .entry((seed.spell_id, seed.difficulty_id))
                .or_default();
            if let SpellAcquisitionSourceCoverageLikeCpp::Indeterminate(reason) = seed.source {
                entry.add_source_reason_like_cpp(reason);
            }
        }

        rows.spell_effects.sort_by_key(|row| row.record_id);
        let mut effect_slots =
            BTreeMap::<(u32, u32), BTreeMap<u8, SpellAcquisitionEffectLikeCpp>>::new();
        for row in rows.spell_effects {
            let spell_id = match row.spell_id_checked() {
                Ok(value) => value,
                Err(error) => {
                    catalog.mark_global_invalid_like_cpp(
                        SpellAcquisitionTableLikeCpp::SpellEffect,
                        row.record_id,
                        error,
                        &mut diagnostics,
                    );
                    continue;
                }
            };
            let difficulty_id = match row.difficulty_id_checked() {
                Ok(value) => value,
                Err(error) => {
                    catalog.mark_invalid_spell_like_cpp(
                        spell_id,
                        SpellAcquisitionTableLikeCpp::SpellEffect,
                        row.record_id,
                        error,
                        &mut diagnostics,
                    );
                    continue;
                }
            };
            let key = (spell_id, difficulty_id);
            let effect_type = match row.effect_type_checked() {
                Ok(value) => value,
                Err(error) => {
                    catalog.mark_invalid_key_like_cpp(
                        key,
                        SpellAcquisitionTableLikeCpp::SpellEffect,
                        row.record_id,
                        error,
                        &mut diagnostics,
                    );
                    continue;
                }
            };
            if effect_type == SPELL_EFFECT_SUMMON_LIKE_CPP {
                catalog
                    .summon_effects_by_spell
                    .entry(spell_id)
                    .or_default()
                    .push(row.clone());
            }
            let effect_index = match row.effect_index_checked() {
                Ok(value) => value,
                Err(error) => {
                    catalog.mark_invalid_key_like_cpp(
                        key,
                        SpellAcquisitionTableLikeCpp::SpellEffect,
                        row.record_id,
                        error,
                        &mut diagnostics,
                    );
                    continue;
                }
            };

            // C++ ASSERTs these structural fields while iterating every
            // effective SpellEffect row, before slot replacement. Preserve
            // that fail-closed boundary even when a later RecordID shadows
            // this slot. Non-structural payload is validated only after the
            // final slot winner is known.
            for (field, raw) in [
                ("SpellEffect.ImplicitTarget1", row.implicit_target_raw[0]),
                ("SpellEffect.ImplicitTarget2", row.implicit_target_raw[1]),
            ] {
                if i16::try_from(raw).is_err() || !(0..TOTAL_SPELL_TARGETS_LIKE_CPP).contains(&raw)
                {
                    catalog.mark_invalid_key_like_cpp(
                        key,
                        SpellAcquisitionTableLikeCpp::SpellEffect,
                        row.record_id,
                        invalid(field, raw, "0..TOTAL_SPELL_TARGETS"),
                        &mut diagnostics,
                    );
                }
            }

            let slots = effect_slots.entry(key).or_default();
            if let Some(replaced) = slots.insert(effect_index, row.clone()) {
                diagnostics.push(SpellAcquisitionDiagnosticLikeCpp {
                    severity: SpellAcquisitionDiagnosticSeverityLikeCpp::Warning,
                    table: SpellAcquisitionTableLikeCpp::SpellEffect,
                    record_id: Some(row.record_id),
                    kind: SpellAcquisitionDiagnosticKindLikeCpp::EffectSlotCollisionResolved {
                        spell_id,
                        difficulty_id,
                        effect_index,
                        replaced_record_id: replaced.record_id,
                        winning_record_id: row.record_id,
                    },
                });
            }
        }
        catalog.effects_by_key = effect_slots
            .into_iter()
            .map(|(key, slots)| (key, slots.into_values().collect()))
            .collect();
        let final_effect_rows = catalog
            .effects_by_key
            .iter()
            .flat_map(|(key, effects)| {
                effects
                    .iter()
                    .cloned()
                    .map(|effect| (*key, effect))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        for (_key, row) in final_effect_rows {
            for error in acquisition_effect_payload_errors_like_cpp(&row) {
                diagnostics.push(diagnostic_from_invalid(
                    SpellAcquisitionTableLikeCpp::SpellEffect,
                    row.record_id,
                    error,
                ));
            }
        }
        catalog.acquisition_effects_by_key = catalog
            .effects_by_key
            .iter()
            .filter_map(|(key, effects)| {
                let acquisition_effects = effects
                    .iter()
                    .filter(|effect| {
                        effect
                            .effect_type_checked()
                            .is_ok_and(is_acquisition_effect_like_cpp)
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                (!acquisition_effects.is_empty()).then_some((*key, acquisition_effects))
            })
            .collect();
        for effects in catalog.summon_effects_by_spell.values_mut() {
            effects.sort_by_key(|effect| {
                (
                    effect.difficulty_id_raw,
                    effect.effect_index_raw,
                    effect.record_id,
                )
            });
        }

        rows.spell_learn_spells.sort_by_key(|row| row.record_id);
        for row in rows.spell_learn_spells {
            let source_spell = match row.spell_id_checked() {
                Ok(value) => value,
                Err(error) => {
                    catalog.mark_global_invalid_like_cpp(
                        SpellAcquisitionTableLikeCpp::SpellLearnSpell,
                        row.record_id,
                        error,
                        &mut diagnostics,
                    );
                    catalog.dependency_rows.push(row);
                    continue;
                }
            };
            let key = (source_spell, DIFFICULTY_NONE_LIKE_CPP);
            if let Err(error) = row.learned_spell_id_checked() {
                catalog.mark_invalid_key_like_cpp(
                    key,
                    SpellAcquisitionTableLikeCpp::SpellLearnSpell,
                    row.record_id,
                    error,
                    &mut diagnostics,
                );
            }
            if let Err(error) = row.overrides_spell_id_checked() {
                catalog.mark_invalid_key_like_cpp(
                    key,
                    SpellAcquisitionTableLikeCpp::SpellLearnSpell,
                    row.record_id,
                    error,
                    &mut diagnostics,
                );
            }
            catalog
                .dependencies_by_spell
                .entry(source_spell)
                .or_default()
                .push(row.clone());
            catalog.dependency_rows.push(row);
        }

        rows.spell_misc.sort_by_key(|row| row.record_id);
        for row in rows.spell_misc {
            let spell_id = match positive_u32(row.spell_id_raw, "SpellMisc.SpellID") {
                Ok(value) => value,
                Err(error) => {
                    catalog.mark_global_invalid_like_cpp(
                        SpellAcquisitionTableLikeCpp::SpellMisc,
                        row.record_id,
                        error,
                        &mut diagnostics,
                    );
                    continue;
                }
            };
            let difficulty_id = match checked_u8(row.difficulty_id_raw, "SpellMisc.DifficultyID") {
                Ok(value) => u32::from(value),
                Err(error) => {
                    catalog.mark_invalid_spell_like_cpp(
                        spell_id,
                        SpellAcquisitionTableLikeCpp::SpellMisc,
                        row.record_id,
                        error,
                        &mut diagnostics,
                    );
                    continue;
                }
            };
            let key = (spell_id, difficulty_id);
            if let Some(replaced) = catalog.misc_by_key.insert(key, row.clone()) {
                diagnostics.push(metadata_collision_diagnostic(
                    SpellAcquisitionTableLikeCpp::SpellMisc,
                    key,
                    replaced.record_id,
                    row.record_id,
                ));
            }
        }
        let final_misc_rows = catalog
            .misc_by_key
            .iter()
            .map(|(key, row)| (*key, row.clone()))
            .collect::<Vec<_>>();
        for (_key, row) in final_misc_rows {
            for result in [
                checked_u32_bits(row.attributes_raw[0], "SpellMisc.Attributes1").map(|_| ()),
                checked_u32_bits(row.attributes_raw[1], "SpellMisc.Attributes2").map(|_| ()),
                row.future_player_condition_id_checked().map(|_| ()),
            ] {
                if let Err(error) = result {
                    diagnostics.push(diagnostic_from_invalid(
                        SpellAcquisitionTableLikeCpp::SpellMisc,
                        row.record_id,
                        error,
                    ));
                }
            }
        }

        rows.spell_levels.sort_by_key(|row| row.record_id);
        for row in rows.spell_levels {
            let spell_id = match positive_u32(row.spell_id_raw, "SpellLevels.SpellID") {
                Ok(value) => value,
                Err(error) => {
                    catalog.mark_global_invalid_like_cpp(
                        SpellAcquisitionTableLikeCpp::SpellLevels,
                        row.record_id,
                        error,
                        &mut diagnostics,
                    );
                    continue;
                }
            };
            let difficulty_id = match checked_u8(row.difficulty_id_raw, "SpellLevels.DifficultyID")
            {
                Ok(value) => u32::from(value),
                Err(error) => {
                    catalog.mark_invalid_spell_like_cpp(
                        spell_id,
                        SpellAcquisitionTableLikeCpp::SpellLevels,
                        row.record_id,
                        error,
                        &mut diagnostics,
                    );
                    continue;
                }
            };
            let key = (spell_id, difficulty_id);
            if let Some(replaced) = catalog.levels_by_key.insert(key, row.clone()) {
                diagnostics.push(metadata_collision_diagnostic(
                    SpellAcquisitionTableLikeCpp::SpellLevels,
                    key,
                    replaced.record_id,
                    row.record_id,
                ));
            }
        }
        let final_levels_rows = catalog
            .levels_by_key
            .iter()
            .map(|(key, row)| (*key, row.clone()))
            .collect::<Vec<_>>();
        for (_key, row) in final_levels_rows {
            for result in [
                row.base_level_checked().map(|_| ()),
                row.spell_level_checked().map(|_| ()),
            ] {
                if let Err(error) = result {
                    diagnostics.push(diagnostic_from_invalid(
                        SpellAcquisitionTableLikeCpp::SpellLevels,
                        row.record_id,
                        error,
                    ));
                }
            }
        }

        rows.talents.sort_by_key(|row| row.record_id);
        for row in rows.talents {
            for raw in row.spell_rank_raw {
                if raw == 0 {
                    continue;
                }
                match source_i32(raw, "Talent.SpellRank")
                    .and_then(|_| positive_u32(raw, "Talent.SpellRank"))
                {
                    Ok(spell_id) => {
                        catalog.talent_spell_ids.insert(spell_id);
                    }
                    Err(error) => catalog.mark_global_invalid_like_cpp(
                        SpellAcquisitionTableLikeCpp::Talent,
                        row.record_id,
                        error,
                        &mut diagnostics,
                    ),
                }
            }
        }

        rows.summon_properties.sort_by_key(|row| row.record_id);
        for row in rows.summon_properties {
            if let Err(error) = source_i32(row.slot_raw, "SummonProperties.Slot") {
                diagnostics.push(diagnostic_from_invalid(
                    SpellAcquisitionTableLikeCpp::SummonProperties,
                    row.record_id,
                    error,
                ));
            }
            if let Err(error) = checked_u32_bits(row.flags_1_raw, "SummonProperties.Flags1") {
                diagnostics.push(diagnostic_from_invalid(
                    SpellAcquisitionTableLikeCpp::SummonProperties,
                    row.record_id,
                    error,
                ));
            }
            catalog.summon_properties_by_id.insert(row.record_id, row);
        }

        rows.battle_pet_species.sort_by_key(|row| row.species_id);
        for row in rows.battle_pet_species {
            match source_i32(row.creature_id_raw, "BattlePetSpecies.CreatureID") {
                Ok(0) => {}
                Ok(value) if value > 0 => {
                    catalog
                        .species_by_creature
                        .entry(value as u32)
                        .or_default()
                        .insert(row.species_id);
                }
                Err(_) if row.creature_id_raw == UNREADABLE_SQL_RAW_LIKE_CPP => {
                    catalog.mark_global_invalid_like_cpp(
                        SpellAcquisitionTableLikeCpp::BattlePetSpecies,
                        row.species_id,
                        invalid(
                            "BattlePetSpecies.CreatureID",
                            row.creature_id_raw,
                            "readable zero or positive i32",
                        ),
                        &mut diagnostics,
                    );
                }
                Ok(_) | Err(_) => diagnostics.push(diagnostic_from_invalid(
                    SpellAcquisitionTableLikeCpp::BattlePetSpecies,
                    row.species_id,
                    invalid(
                        "BattlePetSpecies.CreatureID",
                        row.creature_id_raw,
                        "zero or positive i32",
                    ),
                )),
            }
        }
        for (creature_id, species_ids) in &catalog.species_by_creature {
            if species_ids.len() > 1 {
                diagnostics.push(SpellAcquisitionDiagnosticLikeCpp {
                    severity: SpellAcquisitionDiagnosticSeverityLikeCpp::Indeterminate,
                    table: SpellAcquisitionTableLikeCpp::BattlePetSpecies,
                    record_id: None,
                    kind: SpellAcquisitionDiagnosticKindLikeCpp::ConflictingSpeciesForCreature {
                        creature_id: *creature_id,
                        species_ids: species_ids.iter().copied().collect(),
                    },
                });
            }
        }

        let global_reasons_by_table = catalog.global_indeterminate_by_table.clone();
        for coverage in catalog.coverage_by_key.values_mut() {
            for (table, reasons) in &global_reasons_by_table {
                for reason in reasons {
                    coverage.add_table_reason_like_cpp(*table, reason.clone());
                }
            }
        }

        catalog.diagnostics = diagnostics;
        catalog
    }

    fn mark_invalid_key_like_cpp(
        &mut self,
        key: (u32, u32),
        table: SpellAcquisitionTableLikeCpp,
        record_id: u32,
        error: InvalidAcquisitionValueLikeCpp,
        diagnostics: &mut Vec<SpellAcquisitionDiagnosticLikeCpp>,
    ) {
        diagnostics.push(diagnostic_from_invalid(table, record_id, error));
        if let Some(coverage) = self.coverage_by_key.get_mut(&key) {
            let reason = SpellAcquisitionIndeterminateReasonLikeCpp::InvalidEffectiveRow {
                table,
                record_id,
                field: error.field,
                raw: error.raw,
            };
            coverage.add_table_reason_like_cpp(table, reason);
        }
    }

    fn mark_invalid_spell_like_cpp(
        &mut self,
        spell_id: u32,
        table: SpellAcquisitionTableLikeCpp,
        record_id: u32,
        error: InvalidAcquisitionValueLikeCpp,
        diagnostics: &mut Vec<SpellAcquisitionDiagnosticLikeCpp>,
    ) {
        diagnostics.push(diagnostic_from_invalid(table, record_id, error));
        let reason = SpellAcquisitionIndeterminateReasonLikeCpp::InvalidEffectiveRow {
            table,
            record_id,
            field: error.field,
            raw: error.raw,
        };
        for (_, coverage) in self
            .coverage_by_key
            .range_mut((spell_id, u32::MIN)..=(spell_id, u32::MAX))
        {
            coverage.add_table_reason_like_cpp(table, reason.clone());
        }
    }

    fn mark_global_invalid_like_cpp(
        &mut self,
        table: SpellAcquisitionTableLikeCpp,
        record_id: u32,
        error: InvalidAcquisitionValueLikeCpp,
        diagnostics: &mut Vec<SpellAcquisitionDiagnosticLikeCpp>,
    ) {
        diagnostics.push(diagnostic_from_invalid(table, record_id, error));
        self.push_global_reason_like_cpp(
            table,
            SpellAcquisitionIndeterminateReasonLikeCpp::InvalidEffectiveRow {
                table,
                record_id,
                field: error.field,
                raw: error.raw,
            },
        );
    }

    fn push_global_reason_like_cpp(
        &mut self,
        table: SpellAcquisitionTableLikeCpp,
        reason: SpellAcquisitionIndeterminateReasonLikeCpp,
    ) {
        let reasons = self.global_indeterminate_by_table.entry(table).or_default();
        if !reasons.contains(&reason) {
            reasons.push(reason);
        }
    }

    pub const fn table_hashes_like_cpp(&self) -> SpellAcquisitionTableHashesLikeCpp {
        self.table_hashes
    }

    pub fn diagnostics_like_cpp(&self) -> &[SpellAcquisitionDiagnosticLikeCpp] {
        &self.diagnostics
    }

    pub fn removed_rows_like_cpp(&self) -> &[SpellAcquisitionRemovedRowLikeCpp] {
        &self.removed_rows
    }

    pub fn effects_for_spell_difficulty_like_cpp(
        &self,
        spell_id: u32,
        difficulty_id: u32,
    ) -> SpellAcquisitionEffectsLookupLikeCpp<'_> {
        self.effects_lookup_from_map_like_cpp((spell_id, difficulty_id), &self.effects_by_key)
    }

    fn effects_lookup_from_map_like_cpp<'a>(
        &'a self,
        key: (u32, u32),
        effects_by_key: &'a BTreeMap<(u32, u32), Vec<SpellAcquisitionEffectLikeCpp>>,
    ) -> SpellAcquisitionEffectsLookupLikeCpp<'a> {
        let Some(coverage) = self.coverage_by_key.get(&key) else {
            return SpellAcquisitionEffectsLookupLikeCpp::MissingCoverage;
        };
        let reasons =
            coverage.reasons_for_table_like_cpp(SpellAcquisitionTableLikeCpp::SpellEffect);
        if !reasons.is_empty() {
            return SpellAcquisitionEffectsLookupLikeCpp::Indeterminate(reasons);
        }
        SpellAcquisitionEffectsLookupLikeCpp::Covered(
            effects_by_key.get(&key).map(Vec::as_slice).unwrap_or(&[]),
        )
    }

    /// Ordered final acquisition effects for `DIFFICULTY_NONE`.
    pub fn acquisition_effects_like_cpp(
        &self,
        spell_id: u32,
    ) -> SpellAcquisitionEffectsLookupLikeCpp<'_> {
        self.effects_lookup_from_map_like_cpp(
            (spell_id, DIFFICULTY_NONE_LIKE_CPP),
            &self.acquisition_effects_by_key,
        )
    }

    /// Every final `DIFFICULTY_NONE` effect slot, including effects that the
    /// acquisition planner must explicitly classify as unsupported or
    /// runtime-dependent.
    pub fn difficulty_none_effects_like_cpp(
        &self,
        spell_id: u32,
    ) -> SpellAcquisitionEffectsLookupLikeCpp<'_> {
        self.effects_for_spell_difficulty_like_cpp(spell_id, DIFFICULTY_NONE_LIKE_CPP)
    }

    /// Reproduce C++ `SpellInfoLoadHelper` fallback filling without requiring
    /// this specialized catalog to own the general Difficulty graph.
    ///
    /// The caller supplies `[requested, fallback, fallback-of-fallback, ...]`.
    /// Earlier difficulties win per effect slot; later rows fill only blanks.
    pub fn resolved_effects_for_difficulty_chain_like_cpp(
        &self,
        spell_id: u32,
        difficulty_chain: impl IntoIterator<Item = u32>,
    ) -> SpellAcquisitionResolvedEffectsLookupLikeCpp<'_> {
        let mut slots = BTreeMap::<u8, &SpellAcquisitionEffectLikeCpp>::new();
        for (chain_index, difficulty_id) in difficulty_chain.into_iter().enumerate() {
            let key = (spell_id, difficulty_id);
            let Some(coverage) = self.coverage_by_key.get(&key) else {
                if chain_index == 0 {
                    return SpellAcquisitionResolvedEffectsLookupLikeCpp::MissingCoverage {
                        difficulty_id,
                    };
                }
                // C++ `SpellInfoLoadHelper` skips absent fallback rows and
                // continues walking the remainder of the Difficulty chain.
                continue;
            };
            let reasons =
                coverage.reasons_for_table_like_cpp(SpellAcquisitionTableLikeCpp::SpellEffect);
            if !reasons.is_empty() {
                return SpellAcquisitionResolvedEffectsLookupLikeCpp::Indeterminate(
                    reasons.to_vec(),
                );
            }
            for effect in self.effects_by_key.get(&key).into_iter().flatten() {
                // Every retained winner already passed the structural index
                // check during construction.
                if let Ok(effect_index) = effect.effect_index_checked() {
                    slots.entry(effect_index).or_insert(effect);
                }
            }
        }
        SpellAcquisitionResolvedEffectsLookupLikeCpp::Covered(slots.into_values().collect())
    }

    /// Every final SUMMON row at every difficulty, before effect-slot
    /// collision reduction, in deterministic order.
    pub fn summon_effects_all_difficulties_like_cpp(
        &self,
        spell_id: u32,
    ) -> impl Iterator<Item = &SpellAcquisitionEffectLikeCpp> {
        self.summon_effects_by_spell
            .get(&spell_id)
            .into_iter()
            .flatten()
    }

    pub fn dependency_rows_from_spell_like_cpp(
        &self,
        spell_id: u32,
    ) -> &[SpellAcquisitionDependencyLikeCpp] {
        self.dependencies_by_spell
            .get(&spell_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn dependency_rows_lookup_like_cpp(
        &self,
        spell_id: u32,
    ) -> SpellAcquisitionDependenciesLookupLikeCpp<'_> {
        let key = (spell_id, DIFFICULTY_NONE_LIKE_CPP);
        let Some(coverage) = self.coverage_by_key.get(&key) else {
            return SpellAcquisitionDependenciesLookupLikeCpp::MissingCoverage;
        };
        let reasons =
            coverage.reasons_for_table_like_cpp(SpellAcquisitionTableLikeCpp::SpellLearnSpell);
        if !reasons.is_empty() {
            return SpellAcquisitionDependenciesLookupLikeCpp::Indeterminate(reasons);
        }
        SpellAcquisitionDependenciesLookupLikeCpp::Covered(
            self.dependency_rows_from_spell_like_cpp(spell_id),
        )
    }

    pub fn effective_dependency_rows_like_cpp(
        &self,
    ) -> impl Iterator<Item = &SpellAcquisitionDependencyLikeCpp> {
        self.dependency_rows.iter()
    }

    pub fn misc_for_spell_like_cpp(
        &self,
        spell_id: u32,
        difficulty_id: u32,
    ) -> SpellAcquisitionMetadataLookupLikeCpp<'_, SpellAcquisitionMiscLikeCpp> {
        self.metadata_lookup_like_cpp(
            (spell_id, difficulty_id),
            SpellAcquisitionTableLikeCpp::SpellMisc,
            &self.misc_by_key,
        )
    }

    pub fn levels_for_spell_like_cpp(
        &self,
        spell_id: u32,
        difficulty_id: u32,
    ) -> SpellAcquisitionMetadataLookupLikeCpp<'_, SpellAcquisitionLevelsLikeCpp> {
        self.metadata_lookup_like_cpp(
            (spell_id, difficulty_id),
            SpellAcquisitionTableLikeCpp::SpellLevels,
            &self.levels_by_key,
        )
    }

    pub fn resolved_misc_for_difficulty_chain_like_cpp(
        &self,
        spell_id: u32,
        difficulty_chain: impl IntoIterator<Item = u32>,
    ) -> SpellAcquisitionResolvedMetadataLookupLikeCpp<'_, SpellAcquisitionMiscLikeCpp> {
        self.resolved_metadata_for_difficulty_chain_like_cpp(
            spell_id,
            difficulty_chain,
            SpellAcquisitionTableLikeCpp::SpellMisc,
            &self.misc_by_key,
        )
    }

    pub fn resolved_levels_for_difficulty_chain_like_cpp(
        &self,
        spell_id: u32,
        difficulty_chain: impl IntoIterator<Item = u32>,
    ) -> SpellAcquisitionResolvedMetadataLookupLikeCpp<'_, SpellAcquisitionLevelsLikeCpp> {
        self.resolved_metadata_for_difficulty_chain_like_cpp(
            spell_id,
            difficulty_chain,
            SpellAcquisitionTableLikeCpp::SpellLevels,
            &self.levels_by_key,
        )
    }

    fn resolved_metadata_for_difficulty_chain_like_cpp<'a, T>(
        &'a self,
        spell_id: u32,
        difficulty_chain: impl IntoIterator<Item = u32>,
        table: SpellAcquisitionTableLikeCpp,
        rows: &'a BTreeMap<(u32, u32), T>,
    ) -> SpellAcquisitionResolvedMetadataLookupLikeCpp<'a, T> {
        for (chain_index, difficulty_id) in difficulty_chain.into_iter().enumerate() {
            let key = (spell_id, difficulty_id);
            let Some(coverage) = self.coverage_by_key.get(&key) else {
                if chain_index == 0 {
                    return SpellAcquisitionResolvedMetadataLookupLikeCpp::MissingCoverage {
                        difficulty_id,
                    };
                }
                // Missing fallback data is not fatal in C++; continue to the
                // next fallback difficulty.
                continue;
            };
            let reasons = coverage.reasons_for_table_like_cpp(table);
            if !reasons.is_empty() {
                return SpellAcquisitionResolvedMetadataLookupLikeCpp::Indeterminate(
                    reasons.to_vec(),
                );
            }
            if let Some(row) = rows.get(&key) {
                return SpellAcquisitionResolvedMetadataLookupLikeCpp::Present(row);
            }
        }
        SpellAcquisitionResolvedMetadataLookupLikeCpp::CoveredWithoutRow
    }

    fn metadata_lookup_like_cpp<'a, T>(
        &'a self,
        key: (u32, u32),
        table: SpellAcquisitionTableLikeCpp,
        rows: &'a BTreeMap<(u32, u32), T>,
    ) -> SpellAcquisitionMetadataLookupLikeCpp<'a, T> {
        let Some(coverage) = self.coverage_by_key.get(&key) else {
            return SpellAcquisitionMetadataLookupLikeCpp::MissingCoverage;
        };
        let reasons = coverage.reasons_for_table_like_cpp(table);
        if !reasons.is_empty() {
            return SpellAcquisitionMetadataLookupLikeCpp::Indeterminate(reasons);
        }
        rows.get(&key).map_or(
            SpellAcquisitionMetadataLookupLikeCpp::CoveredWithoutRow,
            SpellAcquisitionMetadataLookupLikeCpp::Present,
        )
    }

    pub fn talent_membership_like_cpp(
        &self,
        spell_id: u32,
    ) -> SpellAcquisitionTalentLookupLikeCpp<'_> {
        // Membership is monotonic: one valid final rank proves talent status
        // even if another unrelated Talent row was unreadable.
        if self.talent_spell_ids.contains(&spell_id) {
            return SpellAcquisitionTalentLookupLikeCpp::Talent;
        }
        if let Some(reasons) = self
            .global_indeterminate_by_table
            .get(&SpellAcquisitionTableLikeCpp::Talent)
            && !reasons.is_empty()
        {
            return SpellAcquisitionTalentLookupLikeCpp::Indeterminate(reasons);
        }
        SpellAcquisitionTalentLookupLikeCpp::NotTalent
    }

    pub fn talent_spell_ids_like_cpp(&self) -> impl Iterator<Item = u32> + '_ {
        self.talent_spell_ids.iter().copied()
    }

    pub fn summon_properties_like_cpp(
        &self,
        properties_id: u32,
    ) -> Option<&SpellAcquisitionSummonPropertiesLikeCpp> {
        self.summon_properties_by_id.get(&properties_id)
    }

    pub fn battle_pet_classification_like_cpp(
        &self,
        spell_id: u32,
    ) -> BattlePetClassificationLikeCpp {
        let mut species_for_spell = BTreeSet::new();
        let mut reasons = Vec::new();

        let mut found_spell_coverage = false;
        for ((covered_spell_id, _difficulty_id), coverage) in self
            .coverage_by_key
            .range((spell_id, u32::MIN)..=(spell_id, u32::MAX))
        {
            debug_assert_eq!(*covered_spell_id, spell_id);
            found_spell_coverage = true;
            for reason in
                coverage.reasons_for_table_like_cpp(SpellAcquisitionTableLikeCpp::SpellEffect)
            {
                let mapped = BattlePetIndeterminateReasonLikeCpp::SpellCoverage {
                    spell_id,
                    reason: reason.clone(),
                };
                if !reasons.contains(&mapped) {
                    reasons.push(mapped);
                }
            }
        }
        if !found_spell_coverage {
            reasons.push(BattlePetIndeterminateReasonLikeCpp::MissingSpellCoverage { spell_id });
        }
        // An incomplete SpellEffect source can hide a SUMMON and therefore
        // prevents a negative classification. The referenced properties and
        // species tables matter only after an effective SUMMON reaches them.
        for table in [SpellAcquisitionTableLikeCpp::SpellEffect] {
            if let Some(table_reasons) = self.global_indeterminate_by_table.get(&table) {
                for reason in table_reasons {
                    let mapped = BattlePetIndeterminateReasonLikeCpp::EffectiveTableIncomplete {
                        table,
                        reason: reason.clone(),
                    };
                    if !reasons.contains(&mapped) {
                        reasons.push(mapped);
                    }
                }
            }
        }

        for effect in self.summon_effects_all_difficulties_like_cpp(spell_id) {
            let difficulty_id = match effect.difficulty_id_checked() {
                Ok(difficulty_id) => difficulty_id,
                Err(error) => {
                    reasons.push(BattlePetIndeterminateReasonLikeCpp::InvalidSummonEffect {
                        record_id: effect.record_id,
                        field: error.field,
                        raw: error.raw,
                    });
                    continue;
                }
            };
            if let Err(error) = effect.effect_index_checked() {
                reasons.push(BattlePetIndeterminateReasonLikeCpp::InvalidSummonEffect {
                    record_id: effect.record_id,
                    field: error.field,
                    raw: error.raw,
                });
                continue;
            }
            if !self
                .coverage_by_key
                .contains_key(&(spell_id, difficulty_id))
            {
                reasons.push(
                    BattlePetIndeterminateReasonLikeCpp::MissingSpellDifficultyCoverage {
                        spell_id,
                        difficulty_id,
                        effect_record_id: effect.record_id,
                    },
                );
                continue;
            }

            let properties_id = match source_i32(
                effect.effect_misc_value_raw[1],
                "SpellEffect.EffectMiscValue",
            ) {
                Ok(0) => continue,
                Ok(value) if value > 0 => value as u32,
                Err(error) => {
                    reasons.push(BattlePetIndeterminateReasonLikeCpp::InvalidSummonEffect {
                        record_id: effect.record_id,
                        field: error.field,
                        raw: error.raw,
                    });
                    continue;
                }
                Ok(_) => {
                    reasons.push(BattlePetIndeterminateReasonLikeCpp::InvalidSummonEffect {
                        record_id: effect.record_id,
                        field: "SpellEffect.EffectMiscValue",
                        raw: effect.effect_misc_value_raw[1],
                    });
                    continue;
                }
            };
            if let Some(table_reasons) = self
                .global_indeterminate_by_table
                .get(&SpellAcquisitionTableLikeCpp::SummonProperties)
            {
                for reason in table_reasons {
                    let mapped = BattlePetIndeterminateReasonLikeCpp::EffectiveTableIncomplete {
                        table: SpellAcquisitionTableLikeCpp::SummonProperties,
                        reason: reason.clone(),
                    };
                    if !reasons.contains(&mapped) {
                        reasons.push(mapped);
                    }
                }
            }
            let Some(properties) = self.summon_properties_by_id.get(&properties_id) else {
                reasons.push(
                    if self.removed_summon_properties_ids.contains(&properties_id) {
                        BattlePetIndeterminateReasonLikeCpp::RemovedSummonProperties {
                            effect_record_id: effect.record_id,
                            properties_id,
                        }
                    } else {
                        BattlePetIndeterminateReasonLikeCpp::MissingSummonProperties {
                            effect_record_id: effect.record_id,
                            properties_id,
                        }
                    },
                );
                continue;
            };
            let slot = match source_i32(properties.slot_raw, "SummonProperties.Slot") {
                Ok(value) => i64::from(value),
                Err(error) => {
                    reasons.push(
                        BattlePetIndeterminateReasonLikeCpp::InvalidSummonProperties {
                            effect_record_id: effect.record_id,
                            properties_id,
                            field: error.field,
                            raw: error.raw,
                        },
                    );
                    continue;
                }
            };
            let flags = match checked_u32_bits(properties.flags_1_raw, "SummonProperties.Flags1") {
                Ok(value) => value,
                Err(error) => {
                    reasons.push(
                        BattlePetIndeterminateReasonLikeCpp::InvalidSummonProperties {
                            effect_record_id: effect.record_id,
                            properties_id,
                            field: error.field,
                            raw: error.raw,
                        },
                    );
                    continue;
                }
            };
            if slot != SUMMON_SLOT_MINIPET_LIKE_CPP
                || flags & SUMMON_FROM_BATTLE_PET_JOURNAL_LIKE_CPP == 0
            {
                continue;
            }

            if let Some(table_reasons) = self
                .global_indeterminate_by_table
                .get(&SpellAcquisitionTableLikeCpp::BattlePetSpecies)
            {
                for reason in table_reasons {
                    let mapped = BattlePetIndeterminateReasonLikeCpp::EffectiveTableIncomplete {
                        table: SpellAcquisitionTableLikeCpp::BattlePetSpecies,
                        reason: reason.clone(),
                    };
                    if !reasons.contains(&mapped) {
                        reasons.push(mapped);
                    }
                }
            }
            let creature_id = match effect.misc_value_id_checked(0) {
                Ok(value) => value,
                Err(error) => {
                    reasons.push(BattlePetIndeterminateReasonLikeCpp::InvalidSummonEffect {
                        record_id: effect.record_id,
                        field: error.field,
                        raw: error.raw,
                    });
                    continue;
                }
            };
            let Some(species) = self.species_by_creature.get(&creature_id) else {
                reasons.push(
                    if let Some(removed_species) =
                        self.removed_species_by_creature.get(&creature_id)
                    {
                        BattlePetIndeterminateReasonLikeCpp::RemovedSpeciesForCreature {
                            effect_record_id: effect.record_id,
                            creature_id,
                            species_ids: removed_species.iter().copied().collect(),
                        }
                    } else {
                        BattlePetIndeterminateReasonLikeCpp::MissingSpeciesForCreature {
                            effect_record_id: effect.record_id,
                            creature_id,
                        }
                    },
                );
                continue;
            };
            if species.len() > 1 {
                reasons.push(
                    BattlePetIndeterminateReasonLikeCpp::ConflictingSpeciesForCreature {
                        effect_record_id: effect.record_id,
                        creature_id,
                        species_ids: species.iter().copied().collect(),
                    },
                );
                continue;
            }
            species_for_spell.extend(species);
        }

        if species_for_spell.len() > 1 {
            reasons.push(
                BattlePetIndeterminateReasonLikeCpp::ConflictingSpeciesForSpell {
                    spell_id,
                    species_ids: species_for_spell.iter().copied().collect(),
                },
            );
        }
        if !reasons.is_empty() {
            return BattlePetClassificationLikeCpp::Indeterminate(reasons);
        }
        match species_for_spell.iter().next().copied() {
            Some(species_id) => BattlePetClassificationLikeCpp::Species(species_id),
            None => BattlePetClassificationLikeCpp::NotBattlePet,
        }
    }
}

fn is_acquisition_effect_like_cpp(effect_type: u32) -> bool {
    matches!(
        effect_type,
        SPELL_EFFECT_SUMMON_LIKE_CPP
            | SPELL_EFFECT_LEARN_SPELL_LIKE_CPP
            | SPELL_EFFECT_DUAL_WIELD_LIKE_CPP
            | SPELL_EFFECT_SKILL_STEP_LIKE_CPP
            | SPELL_EFFECT_SKILL_LIKE_CPP
    )
}

fn acquisition_effect_payload_errors_like_cpp(
    row: &SpellAcquisitionEffectLikeCpp,
) -> Vec<InvalidAcquisitionValueLikeCpp> {
    let mut errors = Vec::new();
    let mut push = |error| {
        if !errors.contains(&error) {
            errors.push(error);
        }
    };
    let Ok(effect_type) = row.effect_type_checked() else {
        return errors;
    };
    match effect_type {
        SPELL_EFFECT_LEARN_SPELL_LIKE_CPP => {
            if let Err(error) = row.trigger_spell_id_checked() {
                push(error);
            }
        }
        SPELL_EFFECT_SKILL_LIKE_CPP | SPELL_EFFECT_SKILL_STEP_LIKE_CPP => {
            if let Err(error) = row.misc_value_id_checked(0) {
                push(error);
            }
            if let Err(error) = row.base_points_die_sides_domain_checked() {
                push(error);
            }
        }
        _ => {}
    }
    errors
}

fn metadata_collision_diagnostic(
    table: SpellAcquisitionTableLikeCpp,
    (spell_id, difficulty_id): (u32, u32),
    replaced_record_id: u32,
    winning_record_id: u32,
) -> SpellAcquisitionDiagnosticLikeCpp {
    SpellAcquisitionDiagnosticLikeCpp {
        severity: SpellAcquisitionDiagnosticSeverityLikeCpp::Warning,
        table,
        record_id: Some(winning_record_id),
        kind: SpellAcquisitionDiagnosticKindLikeCpp::MetadataCollisionResolved {
            spell_id,
            difficulty_id,
            replaced_record_id,
            winning_record_id,
        },
    }
}

fn diagnostic_from_invalid(
    table: SpellAcquisitionTableLikeCpp,
    record_id: u32,
    error: InvalidAcquisitionValueLikeCpp,
) -> SpellAcquisitionDiagnosticLikeCpp {
    SpellAcquisitionDiagnosticLikeCpp {
        severity: SpellAcquisitionDiagnosticSeverityLikeCpp::Indeterminate,
        table,
        record_id: Some(record_id),
        kind: SpellAcquisitionDiagnosticKindLikeCpp::InvalidField {
            field: error.field,
            raw: error.raw,
            expected: error.expected,
        },
    }
}

fn spell_acquisition_effect_from_wdc_like_cpp(
    record_id: u32,
    index: usize,
    reader: &Wdc4Reader,
) -> SpellAcquisitionEffectLikeCpp {
    SpellAcquisitionEffectLikeCpp {
        record_id,
        spell_id_raw: i64::from(reader.get_relationship_id(index).unwrap_or(0)),
        difficulty_id_raw: i64::from(reader.get_field_i32(index, 0)),
        effect_index_raw: i64::from(reader.get_field_i32(index, 1)),
        effect_type_raw: i64::from(reader.get_field_u32(index, 2)),
        effect_base_points_raw: i64::from(reader.get_field_i32(index, 7)),
        effect_die_sides_raw: i64::from(reader.get_field_i32(index, 11)),
        effect_chain_targets_raw: i64::from(
            reader.get_field_i32(index, SPELL_EFFECT_WDC_CHAIN_TARGETS_FIELD),
        ),
        effect_points_per_resource_bits: reader
            .get_field_f32(index, SPELL_EFFECT_WDC_POINTS_PER_RESOURCE_FIELD)
            .to_bits(),
        effect_real_points_per_level_bits: reader
            .get_field_f32(index, SPELL_EFFECT_WDC_REAL_POINTS_PER_LEVEL_FIELD)
            .to_bits(),
        effect_coefficient_bits: reader.get_field_f32(index, 20).to_bits(),
        effect_variance_bits: reader.get_field_f32(index, 21).to_bits(),
        effect_trigger_spell_raw: i64::from(reader.get_field_i32(index, 17)),
        effect_misc_value_raw: std::array::from_fn(|array_index| {
            i64::from(reader.get_array_element(index, 24, array_index, 32) as i32)
        }),
        implicit_target_raw: std::array::from_fn(|array_index| {
            i64::from(reader.get_array_element(index, 27, array_index, 16) as i16)
        }),
    }
}

trait SpellEffectSqlFieldSourceLikeCpp {
    fn raw(&mut self, column: usize, field: &'static str) -> i64;
    fn f32_bits(&mut self, column: usize, field: &'static str) -> u32;
}

struct SpellEffectSqlResultSourceLikeCpp<'a> {
    result: &'a SqlResult,
    diagnostics: &'a mut Vec<SpellAcquisitionDiagnosticLikeCpp>,
    record_id: u32,
}

impl SpellEffectSqlFieldSourceLikeCpp for SpellEffectSqlResultSourceLikeCpp<'_> {
    fn raw(&mut self, column: usize, field: &'static str) -> i64 {
        sql_raw_or_invalid(
            self.result,
            column,
            field,
            SpellAcquisitionTableLikeCpp::SpellEffect,
            self.record_id,
            self.diagnostics,
        )
    }

    fn f32_bits(&mut self, column: usize, field: &'static str) -> u32 {
        sql_f32_bits_or_invalid(
            self.result,
            column,
            field,
            SpellAcquisitionTableLikeCpp::SpellEffect,
            self.record_id,
            self.diagnostics,
        )
    }
}

fn spell_acquisition_effect_from_sql_source_like_cpp(
    record_id: u32,
    source: &mut impl SpellEffectSqlFieldSourceLikeCpp,
) -> SpellAcquisitionEffectLikeCpp {
    SpellAcquisitionEffectLikeCpp {
        record_id,
        difficulty_id_raw: source.raw(1, "SpellEffect.DifficultyID"),
        effect_index_raw: source.raw(2, "SpellEffect.EffectIndex"),
        effect_type_raw: source.raw(3, "SpellEffect.Effect"),
        effect_base_points_raw: source.raw(4, "SpellEffect.EffectBasePoints"),
        effect_die_sides_raw: source.raw(5, "SpellEffect.EffectDieSides"),
        effect_chain_targets_raw: source.raw(
            SPELL_EFFECT_SQL_CHAIN_TARGETS_COLUMN,
            "SpellEffect.EffectChainTargets",
        ),
        effect_points_per_resource_bits: source.f32_bits(
            SPELL_EFFECT_SQL_POINTS_PER_RESOURCE_COLUMN,
            "SpellEffect.EffectPointsPerResource",
        ),
        effect_real_points_per_level_bits: source.f32_bits(
            SPELL_EFFECT_SQL_REAL_POINTS_PER_LEVEL_COLUMN,
            "SpellEffect.EffectRealPointsPerLevel",
        ),
        effect_coefficient_bits: source.f32_bits(11, "SpellEffect.Coefficient"),
        effect_variance_bits: source.f32_bits(12, "SpellEffect.Variance"),
        effect_trigger_spell_raw: source.raw(6, "SpellEffect.EffectTriggerSpell"),
        effect_misc_value_raw: [
            source.raw(7, "SpellEffect.EffectMiscValue1"),
            source.raw(8, "SpellEffect.EffectMiscValue2"),
        ],
        implicit_target_raw: [
            source.raw(9, "SpellEffect.ImplicitTarget1"),
            source.raw(10, "SpellEffect.ImplicitTarget2"),
        ],
        spell_id_raw: source.raw(13, "SpellEffect.SpellID"),
    }
}

fn spell_acquisition_effect_from_sql_like_cpp(
    record_id: u32,
    result: &SqlResult,
    diagnostics: &mut Vec<SpellAcquisitionDiagnosticLikeCpp>,
) -> SpellAcquisitionEffectLikeCpp {
    let mut source = SpellEffectSqlResultSourceLikeCpp {
        result,
        diagnostics,
        record_id,
    };
    spell_acquisition_effect_from_sql_source_like_cpp(record_id, &mut source)
}

impl SpellAcquisitionCatalogLikeCpp {
    /// Load and compose the seven acquisition source families.
    ///
    /// `coverage` must contain the exact regular `(SpellID, DifficultyID)`
    /// keys and any explicitly represented server-side keys. This lets a
    /// caller distinguish an existing spell with zero acquisition effects
    /// from a key for which no authoritative payload exists.
    pub async fn load_effective_like_cpp(
        data_dir: &str,
        locale: &str,
        hotfix_db: &HotfixDatabase,
        removed_records: &Db2HotfixRemovalStoreLikeCpp,
        coverage: impl IntoIterator<Item = SpellAcquisitionCoverageSeedLikeCpp>,
    ) -> Result<Self> {
        let coverage = coverage.into_iter().collect::<Vec<_>>();
        let mut diagnostics = Vec::new();
        let mut effective = EffectiveSpellAcquisitionRowsLikeCpp::default();
        let mut removed_rows = Vec::new();

        let (spell_effect_hash, spell_effect_base) = load_wdc_rows_like_cpp(
            data_dir,
            locale,
            SpellAcquisitionTableLikeCpp::SpellEffect,
            spell_acquisition_effect_from_wdc_like_cpp,
        )?;
        let [spell_effect_official, spell_effect_custom] = load_sql_overlays_like_cpp(
            hotfix_db,
            SpellAcquisitionTableLikeCpp::SpellEffect,
            SPELL_EFFECT_SQL,
            &mut diagnostics,
            spell_acquisition_effect_from_sql_like_cpp,
        )
        .await?;
        let composed_spell_effects = compose_effective_table_with_removed_like_cpp(
            spell_effect_base,
            spell_effect_official,
            spell_effect_custom,
            spell_effect_hash,
            removed_records,
        );
        effective.spell_effects = composed_spell_effects
            .effective_rows
            .into_values()
            .collect();
        removed_rows.extend(
            composed_spell_effects
                .removed_rows
                .into_values()
                .map(SpellAcquisitionRemovedRowLikeCpp::SpellEffect),
        );

        let (spell_learn_spell_hash, spell_learn_spell_base) = load_wdc_rows_like_cpp(
            data_dir,
            locale,
            SpellAcquisitionTableLikeCpp::SpellLearnSpell,
            |record_id, index, reader| SpellAcquisitionDependencyLikeCpp {
                record_id,
                spell_id_raw: i64::from(reader.get_field_i32(index, 0)),
                learn_spell_id_raw: i64::from(reader.get_field_i32(index, 1)),
                overrides_spell_id_raw: i64::from(reader.get_field_i32(index, 2)),
            },
        )?;
        let [spell_learn_spell_official, spell_learn_spell_custom] = load_sql_overlays_like_cpp(
            hotfix_db,
            SpellAcquisitionTableLikeCpp::SpellLearnSpell,
            SPELL_LEARN_SPELL_SQL,
            &mut diagnostics,
            |record_id, result, diagnostics| SpellAcquisitionDependencyLikeCpp {
                record_id,
                spell_id_raw: sql_raw_or_invalid(
                    result,
                    1,
                    "SpellLearnSpell.SpellID",
                    SpellAcquisitionTableLikeCpp::SpellLearnSpell,
                    record_id,
                    diagnostics,
                ),
                learn_spell_id_raw: sql_raw_or_invalid(
                    result,
                    2,
                    "SpellLearnSpell.LearnSpellID",
                    SpellAcquisitionTableLikeCpp::SpellLearnSpell,
                    record_id,
                    diagnostics,
                ),
                overrides_spell_id_raw: sql_raw_or_invalid(
                    result,
                    3,
                    "SpellLearnSpell.OverridesSpellID",
                    SpellAcquisitionTableLikeCpp::SpellLearnSpell,
                    record_id,
                    diagnostics,
                ),
            },
        )
        .await?;
        let composed_spell_learn_spells = compose_effective_table_with_removed_like_cpp(
            spell_learn_spell_base,
            spell_learn_spell_official,
            spell_learn_spell_custom,
            spell_learn_spell_hash,
            removed_records,
        );
        effective.spell_learn_spells = composed_spell_learn_spells
            .effective_rows
            .into_values()
            .collect();
        removed_rows.extend(
            composed_spell_learn_spells
                .removed_rows
                .into_values()
                .map(SpellAcquisitionRemovedRowLikeCpp::SpellLearnSpell),
        );

        let (spell_misc_hash, spell_misc_base) = load_wdc_rows_like_cpp(
            data_dir,
            locale,
            SpellAcquisitionTableLikeCpp::SpellMisc,
            |record_id, index, reader| SpellAcquisitionMiscLikeCpp {
                record_id,
                attributes_raw: std::array::from_fn(|array_index| {
                    i64::from(reader.get_array_element(index, 0, array_index, 32))
                }),
                difficulty_id_raw: i64::from(reader.get_field_u8(index, 1)),
                show_future_spell_player_condition_id_raw: i64::from(
                    reader.get_field_i32(index, 12),
                ),
                spell_id_raw: i64::from(reader.get_relationship_id(index).unwrap_or(0)),
            },
        )?;
        let [spell_misc_official, spell_misc_custom] = load_sql_overlays_like_cpp(
            hotfix_db,
            SpellAcquisitionTableLikeCpp::SpellMisc,
            SPELL_MISC_SQL,
            &mut diagnostics,
            |record_id, result, diagnostics| SpellAcquisitionMiscLikeCpp {
                record_id,
                attributes_raw: [
                    sql_raw_or_invalid(
                        result,
                        1,
                        "SpellMisc.Attributes1",
                        SpellAcquisitionTableLikeCpp::SpellMisc,
                        record_id,
                        diagnostics,
                    ),
                    sql_raw_or_invalid(
                        result,
                        2,
                        "SpellMisc.Attributes2",
                        SpellAcquisitionTableLikeCpp::SpellMisc,
                        record_id,
                        diagnostics,
                    ),
                ],
                difficulty_id_raw: sql_raw_or_invalid(
                    result,
                    3,
                    "SpellMisc.DifficultyID",
                    SpellAcquisitionTableLikeCpp::SpellMisc,
                    record_id,
                    diagnostics,
                ),
                show_future_spell_player_condition_id_raw: sql_raw_or_invalid(
                    result,
                    4,
                    "SpellMisc.ShowFutureSpellPlayerConditionID",
                    SpellAcquisitionTableLikeCpp::SpellMisc,
                    record_id,
                    diagnostics,
                ),
                spell_id_raw: sql_raw_or_invalid(
                    result,
                    5,
                    "SpellMisc.SpellID",
                    SpellAcquisitionTableLikeCpp::SpellMisc,
                    record_id,
                    diagnostics,
                ),
            },
        )
        .await?;
        let composed_spell_misc = compose_effective_table_with_removed_like_cpp(
            spell_misc_base,
            spell_misc_official,
            spell_misc_custom,
            spell_misc_hash,
            removed_records,
        );
        effective.spell_misc = composed_spell_misc.effective_rows.into_values().collect();
        removed_rows.extend(
            composed_spell_misc
                .removed_rows
                .into_values()
                .map(SpellAcquisitionRemovedRowLikeCpp::SpellMisc),
        );

        let (spell_levels_hash, spell_levels_base) = load_wdc_rows_like_cpp(
            data_dir,
            locale,
            SpellAcquisitionTableLikeCpp::SpellLevels,
            |record_id, index, reader| SpellAcquisitionLevelsLikeCpp {
                record_id,
                difficulty_id_raw: i64::from(reader.get_field_u8(index, 0)),
                base_level_raw: i64::from(reader.get_field_i16(index, 1)),
                spell_level_raw: i64::from(reader.get_field_i16(index, 3)),
                spell_id_raw: i64::from(reader.get_relationship_id(index).unwrap_or(0)),
            },
        )?;
        let [spell_levels_official, spell_levels_custom] = load_sql_overlays_like_cpp(
            hotfix_db,
            SpellAcquisitionTableLikeCpp::SpellLevels,
            SPELL_LEVELS_SQL,
            &mut diagnostics,
            |record_id, result, diagnostics| SpellAcquisitionLevelsLikeCpp {
                record_id,
                difficulty_id_raw: sql_raw_or_invalid(
                    result,
                    1,
                    "SpellLevels.DifficultyID",
                    SpellAcquisitionTableLikeCpp::SpellLevels,
                    record_id,
                    diagnostics,
                ),
                base_level_raw: sql_raw_or_invalid(
                    result,
                    2,
                    "SpellLevels.BaseLevel",
                    SpellAcquisitionTableLikeCpp::SpellLevels,
                    record_id,
                    diagnostics,
                ),
                spell_level_raw: sql_raw_or_invalid(
                    result,
                    3,
                    "SpellLevels.SpellLevel",
                    SpellAcquisitionTableLikeCpp::SpellLevels,
                    record_id,
                    diagnostics,
                ),
                spell_id_raw: sql_raw_or_invalid(
                    result,
                    4,
                    "SpellLevels.SpellID",
                    SpellAcquisitionTableLikeCpp::SpellLevels,
                    record_id,
                    diagnostics,
                ),
            },
        )
        .await?;
        let composed_spell_levels = compose_effective_table_with_removed_like_cpp(
            spell_levels_base,
            spell_levels_official,
            spell_levels_custom,
            spell_levels_hash,
            removed_records,
        );
        effective.spell_levels = composed_spell_levels.effective_rows.into_values().collect();
        removed_rows.extend(
            composed_spell_levels
                .removed_rows
                .into_values()
                .map(SpellAcquisitionRemovedRowLikeCpp::SpellLevels),
        );

        let (talent_hash, talent_base) = load_wdc_rows_like_cpp(
            data_dir,
            locale,
            SpellAcquisitionTableLikeCpp::Talent,
            |record_id, index, reader| SpellAcquisitionTalentLikeCpp {
                record_id,
                spell_rank_raw: std::array::from_fn(|array_index| {
                    i64::from(reader.get_array_element(index, 11, array_index, 32) as i32)
                }),
            },
        )?;
        let [talent_official, talent_custom] = load_sql_overlays_like_cpp(
            hotfix_db,
            SpellAcquisitionTableLikeCpp::Talent,
            TALENT_SQL,
            &mut diagnostics,
            |record_id, result, diagnostics| SpellAcquisitionTalentLikeCpp {
                record_id,
                spell_rank_raw: std::array::from_fn(|array_index| {
                    sql_raw_or_invalid(
                        result,
                        1 + array_index,
                        "Talent.SpellRank",
                        SpellAcquisitionTableLikeCpp::Talent,
                        record_id,
                        diagnostics,
                    )
                }),
            },
        )
        .await?;
        let composed_talents = compose_effective_table_with_removed_like_cpp(
            talent_base,
            talent_official,
            talent_custom,
            talent_hash,
            removed_records,
        );
        effective.talents = composed_talents.effective_rows.into_values().collect();
        removed_rows.extend(
            composed_talents
                .removed_rows
                .into_values()
                .map(SpellAcquisitionRemovedRowLikeCpp::Talent),
        );

        let (summon_properties_hash, summon_properties_base) = load_wdc_rows_like_cpp(
            data_dir,
            locale,
            SpellAcquisitionTableLikeCpp::SummonProperties,
            |record_id, index, reader| SpellAcquisitionSummonPropertiesLikeCpp {
                record_id,
                slot_raw: i64::from(reader.get_field_i32(index, 3)),
                flags_1_raw: i64::from(reader.get_array_element(index, 4, 0, 32) as i32),
            },
        )?;
        let [summon_properties_official, summon_properties_custom] = load_sql_overlays_like_cpp(
            hotfix_db,
            SpellAcquisitionTableLikeCpp::SummonProperties,
            SUMMON_PROPERTIES_SQL,
            &mut diagnostics,
            |record_id, result, diagnostics| SpellAcquisitionSummonPropertiesLikeCpp {
                record_id,
                slot_raw: sql_raw_or_invalid(
                    result,
                    1,
                    "SummonProperties.Slot",
                    SpellAcquisitionTableLikeCpp::SummonProperties,
                    record_id,
                    diagnostics,
                ),
                flags_1_raw: sql_raw_or_invalid(
                    result,
                    2,
                    "SummonProperties.Flags1",
                    SpellAcquisitionTableLikeCpp::SummonProperties,
                    record_id,
                    diagnostics,
                ),
            },
        )
        .await?;
        let composed_summon_properties = compose_effective_table_with_removed_like_cpp(
            summon_properties_base,
            summon_properties_official,
            summon_properties_custom,
            summon_properties_hash,
            removed_records,
        );
        effective.summon_properties = composed_summon_properties
            .effective_rows
            .into_values()
            .collect();
        removed_rows.extend(
            composed_summon_properties
                .removed_rows
                .into_values()
                .map(SpellAcquisitionRemovedRowLikeCpp::SummonProperties),
        );

        let (battle_pet_species_hash, battle_pet_species_base) = load_wdc_rows_like_cpp(
            data_dir,
            locale,
            SpellAcquisitionTableLikeCpp::BattlePetSpecies,
            |record_id, index, reader| SpellAcquisitionBattlePetSpeciesLikeCpp {
                species_id: record_id,
                creature_id_raw: i64::from(reader.get_field_i32(index, 3)),
            },
        )?;
        let [battle_pet_species_official, battle_pet_species_custom] = load_sql_overlays_like_cpp(
            hotfix_db,
            SpellAcquisitionTableLikeCpp::BattlePetSpecies,
            BATTLE_PET_SPECIES_SQL,
            &mut diagnostics,
            |record_id, result, diagnostics| SpellAcquisitionBattlePetSpeciesLikeCpp {
                species_id: record_id,
                creature_id_raw: sql_raw_or_invalid(
                    result,
                    1,
                    "BattlePetSpecies.CreatureID",
                    SpellAcquisitionTableLikeCpp::BattlePetSpecies,
                    record_id,
                    diagnostics,
                ),
            },
        )
        .await?;
        let composed_battle_pet_species = compose_effective_table_with_removed_like_cpp(
            battle_pet_species_base,
            battle_pet_species_official,
            battle_pet_species_custom,
            battle_pet_species_hash,
            removed_records,
        );
        effective.battle_pet_species = composed_battle_pet_species
            .effective_rows
            .into_values()
            .collect();
        removed_rows.extend(
            composed_battle_pet_species
                .removed_rows
                .into_values()
                .map(SpellAcquisitionRemovedRowLikeCpp::BattlePetSpecies),
        );

        let table_hashes = SpellAcquisitionTableHashesLikeCpp {
            spell_effect: spell_effect_hash,
            spell_learn_spell: spell_learn_spell_hash,
            spell_misc: spell_misc_hash,
            spell_levels: spell_levels_hash,
            talent: talent_hash,
            summon_properties: summon_properties_hash,
            battle_pet_species: battle_pet_species_hash,
        };
        let table_by_hash = [
            (
                table_hashes.spell_effect,
                SpellAcquisitionTableLikeCpp::SpellEffect,
            ),
            (
                table_hashes.spell_learn_spell,
                SpellAcquisitionTableLikeCpp::SpellLearnSpell,
            ),
            (
                table_hashes.spell_misc,
                SpellAcquisitionTableLikeCpp::SpellMisc,
            ),
            (
                table_hashes.spell_levels,
                SpellAcquisitionTableLikeCpp::SpellLevels,
            ),
            (table_hashes.talent, SpellAcquisitionTableLikeCpp::Talent),
            (
                table_hashes.summon_properties,
                SpellAcquisitionTableLikeCpp::SummonProperties,
            ),
            (
                table_hashes.battle_pet_species,
                SpellAcquisitionTableLikeCpp::BattlePetSpecies,
            ),
        ];
        for (table_hash, record_id) in removed_records.removed_records_in_order_like_cpp() {
            for (_, table) in table_by_hash
                .iter()
                .filter(|(candidate_hash, _)| *candidate_hash == table_hash)
            {
                if !removed_rows.iter().any(|row| {
                    row.table_like_cpp() == *table && row.hotfix_record_id_like_cpp() == record_id
                }) {
                    removed_rows.push(SpellAcquisitionRemovedRowLikeCpp::Unknown {
                        table: *table,
                        record_id,
                    });
                }
            }
        }

        Ok(Self::from_effective_rows_and_removed_like_cpp(
            coverage,
            effective,
            removed_rows,
            table_hashes,
            diagnostics,
        ))
    }
}

fn load_wdc_rows_like_cpp<T>(
    data_dir: &str,
    locale: &str,
    table: SpellAcquisitionTableLikeCpp,
    mut read: impl FnMut(u32, usize, &Wdc4Reader) -> T,
) -> Result<(u32, Vec<(u32, T)>)> {
    let path = Path::new(data_dir)
        .join("dbc")
        .join(locale)
        .join(table.file_name());
    let reader =
        Wdc4Reader::open(&path).with_context(|| format!("failed to open {}", path.display()))?;
    let table_hash = reader.table_hash();
    let rows = reader
        .iter_records()
        .map(|(record_id, index)| (record_id, read(record_id, index, &reader)))
        .collect();
    Ok((table_hash, rows))
}

async fn load_sql_overlays_like_cpp<T>(
    hotfix_db: &HotfixDatabase,
    table: SpellAcquisitionTableLikeCpp,
    sql: &'static str,
    diagnostics: &mut Vec<SpellAcquisitionDiagnosticLikeCpp>,
    mut read: impl FnMut(u32, &SqlResult, &mut Vec<SpellAcquisitionDiagnosticLikeCpp>) -> T,
) -> Result<[Vec<(u32, T)>; 2]> {
    let mut batches = [Vec::new(), Vec::new()];
    for (batch_index, official) in [true, false].into_iter().enumerate() {
        let mut statement = hotfix_db.prepare(HotfixStatements::base(sql));
        statement.set_bool(0, official);
        let mut result = hotfix_db
            .query(&statement)
            .await
            .with_context(|| format!("failed to load {} SQL overlay", table.file_name()))?;
        if result.is_empty() {
            continue;
        }
        loop {
            match sql_raw_i64(&result, 0).and_then(|raw| u32::try_from(raw).ok()) {
                Some(record_id) => {
                    let row = read(record_id, &result, diagnostics);
                    batches[batch_index].push((record_id, row));
                }
                None => diagnostics.push(SpellAcquisitionDiagnosticLikeCpp {
                    severity: SpellAcquisitionDiagnosticSeverityLikeCpp::Indeterminate,
                    table,
                    record_id: None,
                    kind: SpellAcquisitionDiagnosticKindLikeCpp::UnreadableSqlField { field: "ID" },
                }),
            }
            if !result.next_row() {
                break;
            }
        }
    }
    Ok(batches)
}

const UNREADABLE_SQL_RAW_LIKE_CPP: i64 = i64::MIN;

fn sql_raw_or_invalid(
    result: &SqlResult,
    column: usize,
    field: &'static str,
    table: SpellAcquisitionTableLikeCpp,
    record_id: u32,
    diagnostics: &mut Vec<SpellAcquisitionDiagnosticLikeCpp>,
) -> i64 {
    sql_raw_i64(result, column).unwrap_or_else(|| {
        diagnostics.push(SpellAcquisitionDiagnosticLikeCpp {
            severity: SpellAcquisitionDiagnosticSeverityLikeCpp::Indeterminate,
            table,
            record_id: Some(record_id),
            kind: SpellAcquisitionDiagnosticKindLikeCpp::UnreadableSqlField { field },
        });
        UNREADABLE_SQL_RAW_LIKE_CPP
    })
}

fn sql_f32_bits_or_invalid(
    result: &SqlResult,
    column: usize,
    field: &'static str,
    table: SpellAcquisitionTableLikeCpp,
    record_id: u32,
    diagnostics: &mut Vec<SpellAcquisitionDiagnosticLikeCpp>,
) -> u32 {
    result
        .try_read::<f32>(column)
        .or_else(|| result.try_read::<f64>(column).map(|value| value as f32))
        .map(f32::to_bits)
        .unwrap_or_else(|| {
            diagnostics.push(SpellAcquisitionDiagnosticLikeCpp {
                severity: SpellAcquisitionDiagnosticSeverityLikeCpp::Indeterminate,
                table,
                record_id: Some(record_id),
                kind: SpellAcquisitionDiagnosticKindLikeCpp::UnreadableSqlField { field },
            });
            f32::NAN.to_bits()
        })
}

fn sql_raw_i64(result: &SqlResult, column: usize) -> Option<i64> {
    result
        .try_read::<i64>(column)
        .or_else(|| result.try_read::<i32>(column).map(i64::from))
        .or_else(|| result.try_read::<i16>(column).map(i64::from))
        .or_else(|| result.try_read::<i8>(column).map(i64::from))
        .or_else(|| {
            result
                .try_read::<u64>(column)
                .and_then(|value| i64::try_from(value).ok())
        })
        .or_else(|| result.try_read::<u32>(column).map(i64::from))
        .or_else(|| result.try_read::<u16>(column).map(i64::from))
        .or_else(|| result.try_read::<u8>(column).map(i64::from))
}

const fn invalid(
    field: &'static str,
    raw: i64,
    expected: &'static str,
) -> InvalidAcquisitionValueLikeCpp {
    InvalidAcquisitionValueLikeCpp {
        field,
        raw,
        expected,
    }
}

fn source_i32(raw: i64, field: &'static str) -> Result<i32, InvalidAcquisitionValueLikeCpp> {
    i32::try_from(raw).map_err(|_| invalid(field, raw, "i32"))
}

fn positive_u32(raw: i64, field: &'static str) -> Result<u32, InvalidAcquisitionValueLikeCpp> {
    u32::try_from(raw)
        .ok()
        .filter(|value| *value != 0)
        .ok_or_else(|| invalid(field, raw, "positive u32"))
}

fn optional_positive_u32(
    raw: i64,
    field: &'static str,
) -> Result<Option<u32>, InvalidAcquisitionValueLikeCpp> {
    if raw == 0 {
        return Ok(None);
    }
    positive_u32(raw, field).map(Some)
}

fn nonnegative_u32(raw: i64, field: &'static str) -> Result<u32, InvalidAcquisitionValueLikeCpp> {
    u32::try_from(raw).map_err(|_| invalid(field, raw, "u32"))
}

fn checked_u8(raw: i64, field: &'static str) -> Result<u8, InvalidAcquisitionValueLikeCpp> {
    u8::try_from(raw).map_err(|_| invalid(field, raw, "u8"))
}

fn checked_u32_bits(raw: i64, field: &'static str) -> Result<u32, InvalidAcquisitionValueLikeCpp> {
    if let Ok(value) = u32::try_from(raw) {
        return Ok(value);
    }
    i32::try_from(raw)
        .map(|value| value as u32)
        .map_err(|_| invalid(field, raw, "u32/i32 bit field"))
}

fn finite_f32_from_bits(
    bits: u32,
    field: &'static str,
) -> Result<f32, InvalidAcquisitionValueLikeCpp> {
    let value = f32::from_bits(bits);
    if value.is_finite() {
        Ok(value)
    } else {
        Err(invalid(field, i64::from(bits), "finite f32"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push_u16_le(bytes: &mut Vec<u8>, value: u16) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u32_le(bytes: &mut Vec<u8>, value: u32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u64_le(bytes: &mut Vec<u8>, value: u64) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn minimal_spell_effect_wdc4() -> Vec<u8> {
        const FIELD_COUNT: u32 = 29;
        const RECORD_SIZE: u32 = FIELD_COUNT * 4;
        const HEADER_SIZE: u32 = 72;
        const SECTION_HEADER_SIZE: u32 = 40;
        const FIELD_META_SIZE: u32 = 4;
        const FIELD_STORAGE_INFO_SIZE: u32 = 24;
        const RECORD_OFFSET: u32 = HEADER_SIZE
            + SECTION_HEADER_SIZE
            + FIELD_COUNT * FIELD_META_SIZE
            + FIELD_COUNT * FIELD_STORAGE_INFO_SIZE;

        let mut bytes = Vec::new();
        push_u32_le(&mut bytes, 0x3443_4457); // WDC4
        push_u32_le(&mut bytes, 1); // record_count
        push_u32_le(&mut bytes, FIELD_COUNT);
        push_u32_le(&mut bytes, RECORD_SIZE);
        push_u32_le(&mut bytes, 0); // string_table_size
        push_u32_le(&mut bytes, 0); // table_hash
        push_u32_le(&mut bytes, 0x6B64_DD7A); // C++ SpellEffectMeta layout
        push_u32_le(&mut bytes, 77);
        push_u32_le(&mut bytes, 77);
        push_u32_le(&mut bytes, 0); // locale
        push_u16_le(&mut bytes, 0x04); // external ID list
        push_u16_le(&mut bytes, u16::MAX); // no inline ID field
        push_u32_le(&mut bytes, FIELD_COUNT);
        push_u32_le(&mut bytes, 0); // packed_data_offset
        push_u32_le(&mut bytes, 0); // lookup_column_count
        push_u32_le(&mut bytes, FIELD_COUNT * FIELD_STORAGE_INFO_SIZE);
        push_u32_le(&mut bytes, 0); // common_data_size
        push_u32_le(&mut bytes, 0); // pallet_data_size
        push_u32_le(&mut bytes, 1); // section_count

        push_u64_le(&mut bytes, 0); // tact_key_hash
        push_u32_le(&mut bytes, RECORD_OFFSET);
        push_u32_le(&mut bytes, 1); // record_count
        push_u32_le(&mut bytes, 0); // string_table_size
        push_u32_le(&mut bytes, RECORD_OFFSET + RECORD_SIZE);
        push_u32_le(&mut bytes, 4); // id_list_size
        push_u32_le(&mut bytes, 0); // relationship_data_size
        push_u32_le(&mut bytes, 0); // offset_map_id_count
        push_u32_le(&mut bytes, 0); // copy_table_count

        bytes.resize(
            bytes.len() + FIELD_COUNT as usize * FIELD_META_SIZE as usize,
            0,
        );
        for field in 0..FIELD_COUNT {
            push_u16_le(&mut bytes, (field * 32) as u16);
            push_u16_le(&mut bytes, 32);
            push_u32_le(&mut bytes, 0); // additional_data_size
            push_u32_le(&mut bytes, 0); // CompressionType::None
            push_u32_le(&mut bytes, 0);
            push_u32_le(&mut bytes, 0);
            push_u32_le(&mut bytes, 0);
        }
        assert_eq!(bytes.len(), RECORD_OFFSET as usize);

        let mut fields = [0_u32; FIELD_COUNT as usize];
        fields[9] = 9;
        fields[10] = 17; // EffectChainTargets
        fields[11] = 11;
        fields[13] = 13.0_f32.to_bits();
        fields[14] = 1.75_f32.to_bits(); // EffectPointsPerResource
        fields[15] = 15.0_f32.to_bits();
        fields[16] = (-2.5_f32).to_bits(); // EffectRealPointsPerLevel
        fields[17] = 17.0_f32.to_bits();
        for field in fields {
            push_u32_le(&mut bytes, field);
        }
        push_u32_le(&mut bytes, 77); // external record ID
        bytes
    }

    struct SentinelSpellEffectSqlSource {
        raw: [i64; 17],
        f32_bits: [u32; 17],
    }

    impl SpellEffectSqlFieldSourceLikeCpp for SentinelSpellEffectSqlSource {
        fn raw(&mut self, column: usize, _field: &'static str) -> i64 {
            self.raw[column]
        }

        fn f32_bits(&mut self, column: usize, _field: &'static str) -> u32 {
            self.f32_bits[column]
        }
    }

    fn effect(
        record_id: u32,
        spell_id: i64,
        difficulty_id: i64,
        effect_index: i64,
        effect_type: i64,
    ) -> SpellAcquisitionEffectLikeCpp {
        SpellAcquisitionEffectLikeCpp {
            record_id,
            spell_id_raw: spell_id,
            difficulty_id_raw: difficulty_id,
            effect_index_raw: effect_index,
            effect_type_raw: effect_type,
            effect_base_points_raw: 0,
            effect_die_sides_raw: 0,
            effect_chain_targets_raw: 0,
            effect_points_per_resource_bits: 0.0_f32.to_bits(),
            effect_real_points_per_level_bits: 0.0_f32.to_bits(),
            effect_coefficient_bits: 0.0_f32.to_bits(),
            effect_variance_bits: 0.0_f32.to_bits(),
            effect_trigger_spell_raw: 0,
            effect_misc_value_raw: [0, 0],
            implicit_target_raw: [0, 0],
        }
    }

    #[test]
    fn player_effect_targets_share_the_cpp_none_caster_and_ally_set() {
        let mut row = effect(1, 100, 0, 0, 36);
        for targets in [[0, 0], [1, 0], [21, 0], [1, 21]] {
            row.implicit_target_raw = targets;
            assert!(row.targets_player_like_cpp());
        }
        row.implicit_target_raw = [1, 5];
        assert!(!row.targets_player_like_cpp());
        row.implicit_target_raw = [6, 0];
        assert!(!row.targets_player_like_cpp());
    }

    fn summon(
        record_id: u32,
        spell_id: u32,
        difficulty_id: u8,
        effect_index: u8,
        creature_id: i64,
        properties_id: i64,
    ) -> SpellAcquisitionEffectLikeCpp {
        let mut row = effect(
            record_id,
            i64::from(spell_id),
            i64::from(difficulty_id),
            i64::from(effect_index),
            i64::from(SPELL_EFFECT_SUMMON_LIKE_CPP),
        );
        row.effect_misc_value_raw = [creature_id, properties_id];
        row
    }

    fn catalog(
        coverage: impl IntoIterator<Item = SpellAcquisitionCoverageSeedLikeCpp>,
        rows: EffectiveSpellAcquisitionRowsLikeCpp,
    ) -> SpellAcquisitionCatalogLikeCpp {
        SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
            coverage,
            rows,
            SpellAcquisitionTableHashesLikeCpp::default(),
            Vec::new(),
        )
    }

    fn catalog_with_removed(
        coverage: impl IntoIterator<Item = SpellAcquisitionCoverageSeedLikeCpp>,
        rows: EffectiveSpellAcquisitionRowsLikeCpp,
        removed_rows: Vec<SpellAcquisitionRemovedRowLikeCpp>,
    ) -> SpellAcquisitionCatalogLikeCpp {
        SpellAcquisitionCatalogLikeCpp::from_effective_rows_and_removed_like_cpp(
            coverage,
            rows,
            removed_rows,
            SpellAcquisitionTableHashesLikeCpp::default(),
            Vec::new(),
        )
    }

    #[test]
    fn spell_effect_wdc_hydrates_planner_fields_from_cpp_physical_indices() {
        assert_eq!(
            [
                SPELL_EFFECT_WDC_CHAIN_TARGETS_FIELD,
                SPELL_EFFECT_WDC_POINTS_PER_RESOURCE_FIELD,
                SPELL_EFFECT_WDC_REAL_POINTS_PER_LEVEL_FIELD,
            ],
            [10, 14, 16]
        );

        let path = std::env::temp_dir().join(format!(
            "rustycore-spell-effect-planner-fields-{}.db2",
            std::process::id()
        ));
        std::fs::write(&path, minimal_spell_effect_wdc4()).expect("write SpellEffect WDC4 fixture");
        let reader = Wdc4Reader::open(&path).expect("open SpellEffect WDC4 fixture");
        let row = spell_acquisition_effect_from_wdc_like_cpp(77, 0, &reader);
        std::fs::remove_file(path).expect("remove SpellEffect WDC4 fixture");

        assert_eq!(row.effect_chain_targets_raw, 17);
        assert_eq!(row.effect_points_per_resource_bits, 1.75_f32.to_bits());
        assert_eq!(row.effect_real_points_per_level_bits, (-2.5_f32).to_bits());
    }

    #[test]
    fn spell_effect_sql_hydrates_planner_fields_from_projection_columns() {
        let projection = SPELL_EFFECT_SQL
            .strip_prefix("SELECT ")
            .and_then(|sql| sql.split_once(" FROM spell_effect "))
            .map(|(columns, _)| columns.split(", ").collect::<Vec<_>>())
            .expect("SpellEffect SQL projection");
        assert_eq!(projection.len(), 17);
        assert_eq!(
            [
                SPELL_EFFECT_SQL_CHAIN_TARGETS_COLUMN,
                SPELL_EFFECT_SQL_POINTS_PER_RESOURCE_COLUMN,
                SPELL_EFFECT_SQL_REAL_POINTS_PER_LEVEL_COLUMN,
            ],
            [14, 15, 16]
        );
        assert_eq!(projection[14], "EffectChainTargets");
        assert_eq!(projection[15], "EffectPointsPerResource");
        assert_eq!(projection[16], "EffectRealPointsPerLevel");

        let mut source = SentinelSpellEffectSqlSource {
            raw: [0; 17],
            f32_bits: [0; 17],
        };
        source.raw[14] = 23;
        source.f32_bits[15] = 3.25_f32.to_bits();
        source.f32_bits[16] = (-4.5_f32).to_bits();
        let row = spell_acquisition_effect_from_sql_source_like_cpp(88, &mut source);

        assert_eq!(row.effect_chain_targets_raw, 23);
        assert_eq!(row.effect_points_per_resource_bits, 3.25_f32.to_bits());
        assert_eq!(row.effect_real_points_per_level_bits, (-4.5_f32).to_bits());
    }

    #[test]
    fn composition_is_base_then_official_then_custom_then_removal() {
        let table_hash = 0xAABB_CCDD;
        let removals = Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([
            (table_hash, 2, 2),
            (table_hash, 3, 2),
            (table_hash, 3, 1),
            (table_hash, 4, 2),
        ]);
        let composed = compose_effective_table_with_removed_like_cpp(
            [(1, "base-1"), (2, "base-2")],
            [(1, "official-1"), (3, "official-sql-only")],
            [(1, "custom-1"), (4, "custom-sql-only")],
            table_hash,
            &removals,
        );
        let effective = &composed.effective_rows;

        assert_eq!(effective.get(&1), Some(&"custom-1"));
        assert!(!effective.contains_key(&2));
        assert_eq!(effective.get(&3), Some(&"official-sql-only"));
        assert!(!effective.contains_key(&4));
        assert_eq!(composed.removed_rows.get(&2), Some(&"base-2"));
        assert_eq!(composed.removed_rows.get(&4), Some(&"custom-sql-only"));
        assert!(!composed.removed_rows.contains_key(&3));
    }

    #[test]
    fn every_source_family_uses_the_complete_overlay_and_removal_lifecycle() {
        fn assert_family<T, Make>(mut make: Make)
        where
            T: Clone + std::fmt::Debug + PartialEq,
            Make: FnMut(u32, i64) -> T,
        {
            let table_hash = 0xAABB_CCDD;
            let base_collision = make(1, 10);
            let removed_base = make(2, 20);
            let official_collision = make(1, 30);
            let official_sql_only = make(3, 40);
            let custom_collision = make(1, 50);
            let removed_custom_sql_only = make(4, 60);
            let removals = Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([
                (table_hash, 2, 2),
                (table_hash, 3, 2),
                (table_hash, 3, 1),
                (table_hash, 4, 2),
            ]);

            let composed = compose_effective_table_with_removed_like_cpp(
                [(1, base_collision), (2, removed_base.clone())],
                [(1, official_collision), (3, official_sql_only.clone())],
                [
                    (1, custom_collision.clone()),
                    (4, removed_custom_sql_only.clone()),
                ],
                table_hash,
                &removals,
            );

            assert_eq!(composed.effective_rows.get(&1), Some(&custom_collision));
            assert_eq!(
                composed.effective_rows.get(&3),
                Some(&official_sql_only),
                "a later non-removal status must restore the SQL-only row"
            );
            assert_eq!(composed.removed_rows.get(&2), Some(&removed_base));
            assert_eq!(
                composed.removed_rows.get(&4),
                Some(&removed_custom_sql_only)
            );
        }

        assert_family(|record_id, marker| {
            let mut row = effect(record_id, 100, 0, 0, i64::from(SPELL_EFFECT_SKILL_LIKE_CPP));
            row.effect_base_points_raw = marker;
            row
        });
        assert_family(|record_id, marker| SpellAcquisitionDependencyLikeCpp {
            record_id,
            spell_id_raw: 100,
            learn_spell_id_raw: marker,
            overrides_spell_id_raw: 0,
        });
        assert_family(|record_id, marker| SpellAcquisitionMiscLikeCpp {
            record_id,
            spell_id_raw: 100,
            difficulty_id_raw: 0,
            attributes_raw: [marker, 0],
            show_future_spell_player_condition_id_raw: 0,
        });
        assert_family(|record_id, marker| SpellAcquisitionLevelsLikeCpp {
            record_id,
            spell_id_raw: 100,
            difficulty_id_raw: 0,
            base_level_raw: marker,
            spell_level_raw: 1,
        });
        assert_family(|record_id, marker| SpellAcquisitionTalentLikeCpp {
            record_id,
            spell_rank_raw: [marker, 0, 0, 0, 0, 0, 0, 0, 0],
        });
        assert_family(
            |record_id, marker| SpellAcquisitionSummonPropertiesLikeCpp {
                record_id,
                slot_raw: marker,
                flags_1_raw: 0,
            },
        );
        assert_family(
            |record_id, marker| SpellAcquisitionBattlePetSpeciesLikeCpp {
                species_id: record_id,
                creature_id_raw: marker,
            },
        );
    }

    #[test]
    fn invalid_overlay_replaces_stale_base_and_custom_can_repair_it() {
        let base = effect(7, 100, 0, 0, i64::from(SPELL_EFFECT_SKILL_LIKE_CPP));
        let invalid_official = effect(7, 100, 0, -1, i64::from(SPELL_EFFECT_SKILL_LIKE_CPP));
        let custom = effect(7, 100, 0, 1, i64::from(SPELL_EFFECT_DUAL_WIELD_LIKE_CPP));
        let removals = Db2HotfixRemovalStoreLikeCpp::default();

        let invalid_final = compose_effective_table_like_cpp(
            [(7, base.clone())],
            [(7, invalid_official)],
            [],
            0xAABB,
            &removals,
        );
        let invalid_catalog = catalog(
            [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects: invalid_final.into_values().collect(),
                ..Default::default()
            },
        );
        assert!(matches!(
            invalid_catalog.acquisition_effects_like_cpp(100),
            SpellAcquisitionEffectsLookupLikeCpp::Indeterminate(_)
        ));

        let repaired_final = compose_effective_table_like_cpp(
            [(7, base)],
            [(7, effect(7, 100, 0, -1, 118))],
            [(7, custom)],
            0xAABB,
            &removals,
        );
        let repaired_catalog = catalog(
            [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects: repaired_final.into_values().collect(),
                ..Default::default()
            },
        );
        let SpellAcquisitionEffectsLookupLikeCpp::Covered(effects) =
            repaired_catalog.acquisition_effects_like_cpp(100)
        else {
            panic!("custom repair must restore determinate coverage");
        };
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0].record_id, 7);
        assert_eq!(
            effects[0].effect_type_checked(),
            Ok(SPELL_EFFECT_DUAL_WIELD_LIKE_CPP)
        );
    }

    #[test]
    fn typed_tombstones_cover_all_source_families_without_changing_final_coverage() {
        let removed_rows = vec![
            SpellAcquisitionRemovedRowLikeCpp::SpellEffect(effect(
                1,
                100,
                0,
                0,
                i64::from(SPELL_EFFECT_DUAL_WIELD_LIKE_CPP),
            )),
            SpellAcquisitionRemovedRowLikeCpp::SpellLearnSpell(SpellAcquisitionDependencyLikeCpp {
                record_id: 2,
                spell_id_raw: 100,
                learn_spell_id_raw: 200,
                overrides_spell_id_raw: 0,
            }),
            SpellAcquisitionRemovedRowLikeCpp::SpellMisc(SpellAcquisitionMiscLikeCpp {
                record_id: 3,
                spell_id_raw: 100,
                difficulty_id_raw: 0,
                attributes_raw: [0, 0],
                show_future_spell_player_condition_id_raw: 0,
            }),
            SpellAcquisitionRemovedRowLikeCpp::SpellLevels(SpellAcquisitionLevelsLikeCpp {
                record_id: 4,
                spell_id_raw: 100,
                difficulty_id_raw: 0,
                base_level_raw: 1,
                spell_level_raw: 1,
            }),
            SpellAcquisitionRemovedRowLikeCpp::Talent(SpellAcquisitionTalentLikeCpp {
                record_id: 5,
                spell_rank_raw: [100, 0, 0, 0, 0, 0, 0, 0, 0],
            }),
            SpellAcquisitionRemovedRowLikeCpp::SummonProperties(
                SpellAcquisitionSummonPropertiesLikeCpp {
                    record_id: 6,
                    slot_raw: 0,
                    flags_1_raw: 0,
                },
            ),
            SpellAcquisitionRemovedRowLikeCpp::BattlePetSpecies(
                SpellAcquisitionBattlePetSpeciesLikeCpp {
                    species_id: 7,
                    creature_id_raw: 0,
                },
            ),
        ];
        let catalog = catalog_with_removed(
            [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
            EffectiveSpellAcquisitionRowsLikeCpp::default(),
            removed_rows,
        );

        assert_eq!(catalog.removed_rows_like_cpp().len(), 7);
        assert_eq!(
            catalog
                .removed_rows_like_cpp()
                .iter()
                .map(SpellAcquisitionRemovedRowLikeCpp::table_like_cpp)
                .collect::<BTreeSet<_>>(),
            SpellAcquisitionTableLikeCpp::ALL.into_iter().collect()
        );
        assert_eq!(
            catalog.effects_for_spell_difficulty_like_cpp(100, 0),
            SpellAcquisitionEffectsLookupLikeCpp::Covered(&[]),
            "a final removal is evidence, not an implicit indeterminate result"
        );
    }

    #[test]
    fn coverage_distinguishes_zero_effects_missing_and_source_unavailable() {
        let catalog = catalog(
            [
                SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0),
                SpellAcquisitionCoverageSeedLikeCpp::indeterminate(
                    200,
                    0,
                    SpellAcquisitionIndeterminateReasonLikeCpp::ServerSideMetadataUnavailable,
                ),
            ],
            EffectiveSpellAcquisitionRowsLikeCpp::default(),
        );

        assert_eq!(
            catalog.acquisition_effects_like_cpp(100),
            SpellAcquisitionEffectsLookupLikeCpp::Covered(&[])
        );
        assert_eq!(
            catalog.acquisition_effects_like_cpp(300),
            SpellAcquisitionEffectsLookupLikeCpp::MissingCoverage
        );
        assert!(matches!(
            catalog.acquisition_effects_like_cpp(200),
            SpellAcquisitionEffectsLookupLikeCpp::Indeterminate(_)
        ));
    }

    #[test]
    fn difficulty_none_slots_are_ordered_and_highest_record_id_wins() {
        let mut lower_slot = effect(10, 100, 0, 1, i64::from(SPELL_EFFECT_SKILL_LIKE_CPP));
        lower_slot.effect_misc_value_raw[0] = 777;
        let mut winner = effect(30, 100, 0, 1, i64::from(SPELL_EFFECT_LEARN_SPELL_LIKE_CPP));
        winner.effect_trigger_spell_raw = 900;
        let first = effect(20, 100, 0, 0, i64::from(SPELL_EFFECT_DUAL_WIELD_LIKE_CPP));
        let mut other_difficulty =
            effect(40, 100, 2, 0, i64::from(SPELL_EFFECT_SKILL_STEP_LIKE_CPP));
        other_difficulty.effect_misc_value_raw[0] = 777;
        let catalog = catalog(
            [
                SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0),
                SpellAcquisitionCoverageSeedLikeCpp::covered(100, 2),
            ],
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects: vec![winner, other_difficulty, lower_slot, first],
                ..Default::default()
            },
        );

        let SpellAcquisitionEffectsLookupLikeCpp::Covered(effects) =
            catalog.acquisition_effects_like_cpp(100)
        else {
            panic!("difficulty-none effects must be covered");
        };
        assert_eq!(
            effects.iter().map(|row| row.record_id).collect::<Vec<_>>(),
            vec![20, 30]
        );
        assert!(catalog.diagnostics_like_cpp().iter().any(|diagnostic| {
            matches!(
                diagnostic.kind,
                SpellAcquisitionDiagnosticKindLikeCpp::EffectSlotCollisionResolved {
                    replaced_record_id: 10,
                    winning_record_id: 30,
                    ..
                }
            )
        }));
        let SpellAcquisitionEffectsLookupLikeCpp::Covered(other) =
            catalog.effects_for_spell_difficulty_like_cpp(100, 2)
        else {
            panic!("other difficulty must remain independently covered");
        };
        assert_eq!(other[0].record_id, 40);
    }

    #[test]
    fn checked_values_do_not_narrow_skill_ids_and_expose_die_domain() {
        let mut row = effect(1, 100, 0, 0, i64::from(SPELL_EFFECT_SKILL_LIKE_CPP));
        row.effect_misc_value_raw[0] = 70_000;
        row.effect_base_points_raw = 4;
        row.effect_die_sides_raw = 1;

        assert_eq!(row.misc_value_id_checked(0), Ok(70_000));
        assert_eq!(
            row.base_points_die_sides_domain_checked(),
            Ok(AcquisitionValueDomainLikeCpp {
                minimum: 5,
                maximum: 5,
            })
        );
        row.effect_die_sides_raw = 3;
        assert_eq!(
            row.base_points_die_sides_domain_checked(),
            Ok(AcquisitionValueDomainLikeCpp {
                minimum: 5,
                maximum: 7,
            })
        );
        row.effect_die_sides_raw = 0;
        row.effect_base_points_raw = i64::from(i32::MAX);
        assert!(
            row.base_points_die_sides_domain_checked().is_err(),
            "i32::MAX rounds to 2^31 in f32 and must not use Rust's saturating float cast"
        );
        row.difficulty_id_raw = 256;
        assert!(
            row.difficulty_id_checked().is_err(),
            "C++ Difficulty has an explicit uint8 source domain"
        );
    }

    #[test]
    fn learn_skill_value_domain_honors_legacy_coefficient_and_variance() {
        let mut row = effect(1, 100, 0, 0, i64::from(SPELL_EFFECT_SKILL_LIKE_CPP));
        row.effect_misc_value_raw[0] = 777;
        row.effect_base_points_raw = 4;
        row.effect_die_sides_raw = 1;

        row.effect_coefficient_bits = 1.0_f32.to_bits();
        assert_eq!(
            row.base_points_die_sides_domain_checked(),
            Ok(AcquisitionValueDomainLikeCpp {
                minimum: 1,
                maximum: 1,
            }),
            "legacy Scaling.Class=0 makes a nonzero coefficient ignore BasePoints"
        );

        row.effect_coefficient_bits = 0.0_f32.to_bits();
        row.effect_variance_bits = 0.25_f32.to_bits();
        assert_eq!(
            row.base_points_die_sides_domain_checked(),
            Ok(AcquisitionValueDomainLikeCpp {
                minimum: 5,
                maximum: 5,
            }),
            "frand's exclusive upper endpoint keeps the final value singleton"
        );

        row.effect_base_points_raw = 8;
        row.effect_die_sides_raw = 0;
        row.effect_variance_bits = 0.5_f32.to_bits();
        assert_eq!(
            row.base_points_die_sides_domain_checked(),
            Ok(AcquisitionValueDomainLikeCpp {
                minimum: 6,
                maximum: 10,
            }),
            "a variance whose rounded outcomes differ remains explicitly ranged"
        );

        row.effect_base_points_raw = 4;
        row.effect_die_sides_raw = 1;
        row.effect_variance_bits = 0.25_f32.to_bits();
        row.effect_coefficient_bits = 1.0_f32.to_bits();
        assert_eq!(
            row.base_points_die_sides_domain_checked()
                .and_then(|domain| domain
                    .deterministic_value()
                    .ok_or_else(|| { invalid("test", 0, "deterministic") })),
            Ok(1),
            "variance over the coefficient-forced zero base remains inert"
        );

        row.effect_coefficient_bits = 0.0_f32.to_bits();
        row.effect_base_points_raw = -4;
        assert_eq!(
            row.base_points_die_sides_domain_checked(),
            Ok(AcquisitionValueDomainLikeCpp {
                minimum: -3,
                maximum: -3,
            }),
            "a negative base reverses which variance endpoint is open"
        );

        row.effect_base_points_raw = 4;
        row.effect_coefficient_bits = 0.0_f32.to_bits();
        row.effect_variance_bits = 0.0_f32.to_bits();
        row.effect_die_sides_raw = -3;
        assert_eq!(
            row.base_points_die_sides_domain_checked(),
            Ok(AcquisitionValueDomainLikeCpp {
                minimum: 1,
                maximum: 5,
            }),
            "negative DieSides uses C++'s inclusive [DieSides, 1] range"
        );

        row.effect_coefficient_bits = f32::NAN.to_bits();
        assert!(row.base_points_die_sides_domain_checked().is_err());
    }

    #[test]
    fn dependencies_metadata_and_talent_membership_use_final_rows() {
        let mut attributes = [0_i64; 2];
        attributes[0] = i64::from(SPELL_ATTR0_PASSIVE_LIKE_CPP);
        attributes[1] = i64::from(SPELL_ATTR1_CAST_WHEN_LEARNED_LIKE_CPP);
        let rows = EffectiveSpellAcquisitionRowsLikeCpp {
            spell_learn_spells: vec![SpellAcquisitionDependencyLikeCpp {
                record_id: 10,
                spell_id_raw: 100,
                learn_spell_id_raw: 200,
                overrides_spell_id_raw: 300,
            }],
            spell_misc: vec![SpellAcquisitionMiscLikeCpp {
                record_id: 11,
                spell_id_raw: 100,
                difficulty_id_raw: 0,
                attributes_raw: attributes,
                show_future_spell_player_condition_id_raw: 44,
            }],
            spell_levels: vec![SpellAcquisitionLevelsLikeCpp {
                record_id: 12,
                spell_id_raw: 100,
                difficulty_id_raw: 0,
                base_level_raw: 10,
                spell_level_raw: 20,
            }],
            talents: vec![SpellAcquisitionTalentLikeCpp {
                record_id: 13,
                spell_rank_raw: [100, 200, 0, 0, 0, 0, 0, 0, 0],
            }],
            ..Default::default()
        };
        let catalog = catalog([SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)], rows);

        let dependency = &catalog.dependency_rows_from_spell_like_cpp(100)[0];
        assert_eq!(dependency.learned_spell_id_checked(), Ok(200));
        assert_eq!(dependency.overrides_spell_id_checked(), Ok(Some(300)));
        let SpellAcquisitionMetadataLookupLikeCpp::Present(misc) =
            catalog.misc_for_spell_like_cpp(100, 0)
        else {
            panic!("misc metadata missing");
        };
        assert_eq!(misc.is_passive_checked(), Ok(true));
        assert_eq!(misc.cast_when_learned_checked(), Ok(true));
        assert_eq!(misc.future_player_condition_id_checked(), Ok(Some(44)));
        let SpellAcquisitionMetadataLookupLikeCpp::Present(levels) =
            catalog.levels_for_spell_like_cpp(100, 0)
        else {
            panic!("levels metadata missing");
        };
        assert_eq!(levels.base_level_checked(), Ok(10));
        assert_eq!(levels.spell_level_checked(), Ok(20));
        assert_eq!(
            catalog.talent_membership_like_cpp(100),
            SpellAcquisitionTalentLookupLikeCpp::Talent
        );
        assert_eq!(
            catalog.talent_membership_like_cpp(200),
            SpellAcquisitionTalentLookupLikeCpp::Talent
        );
        assert_eq!(
            catalog.talent_membership_like_cpp(300),
            SpellAcquisitionTalentLookupLikeCpp::NotTalent
        );
    }

    #[test]
    fn invalidity_is_scoped_to_its_metadata_family() {
        let rows = EffectiveSpellAcquisitionRowsLikeCpp {
            spell_effects: vec![effect(
                1,
                100,
                0,
                0,
                i64::from(SPELL_EFFECT_DUAL_WIELD_LIKE_CPP),
            )],
            spell_misc: vec![SpellAcquisitionMiscLikeCpp {
                record_id: 2,
                spell_id_raw: 100,
                difficulty_id_raw: 0,
                attributes_raw: [0, 0],
                show_future_spell_player_condition_id_raw: 0,
            }],
            spell_levels: vec![SpellAcquisitionLevelsLikeCpp {
                record_id: 3,
                spell_id_raw: 100,
                difficulty_id_raw: 0,
                base_level_raw: i64::from(i16::MAX) + 1,
                spell_level_raw: 1,
            }],
            battle_pet_species: vec![SpellAcquisitionBattlePetSpeciesLikeCpp {
                species_id: 4,
                creature_id_raw: -1,
            }],
            ..Default::default()
        };
        let scoped_catalog = catalog([SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)], rows);

        assert!(matches!(
            scoped_catalog.effects_for_spell_difficulty_like_cpp(100, 0),
            SpellAcquisitionEffectsLookupLikeCpp::Covered(_)
        ));
        assert!(matches!(
            scoped_catalog.misc_for_spell_like_cpp(100, 0),
            SpellAcquisitionMetadataLookupLikeCpp::Present(_)
        ));
        let SpellAcquisitionMetadataLookupLikeCpp::Present(levels) =
            scoped_catalog.levels_for_spell_like_cpp(100, 0)
        else {
            panic!("semantic payload invalidity must not erase final metadata");
        };
        assert!(levels.base_level_checked().is_err());
        assert_eq!(levels.spell_level_checked(), Ok(1));

        let mut invalid_effect = effect(5, 200, 0, 0, i64::from(SPELL_EFFECT_SKILL_LIKE_CPP));
        invalid_effect.effect_misc_value_raw[0] = -1;
        let effect_catalog = catalog(
            [SpellAcquisitionCoverageSeedLikeCpp::covered(200, 0)],
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects: vec![invalid_effect],
                spell_misc: vec![SpellAcquisitionMiscLikeCpp {
                    record_id: 6,
                    spell_id_raw: 200,
                    difficulty_id_raw: 0,
                    attributes_raw: [0, 0],
                    show_future_spell_player_condition_id_raw: 0,
                }],
                ..Default::default()
            },
        );
        let SpellAcquisitionEffectsLookupLikeCpp::Covered(effects) =
            effect_catalog.effects_for_spell_difficulty_like_cpp(200, 0)
        else {
            panic!("semantic payload invalidity must remain consumer-scoped");
        };
        assert!(effects[0].misc_value_id_checked(0).is_err());
        assert!(matches!(
            effect_catalog.misc_for_spell_like_cpp(200, 0),
            SpellAcquisitionMetadataLookupLikeCpp::Present(_)
        ));
    }

    #[test]
    fn irrelevant_effect_payload_does_not_hide_valid_acquisition_effects() {
        let mut runtime_only = effect(1, 100, 0, 0, 1);
        runtime_only.effect_base_points_raw = i64::from(i32::MAX) + 1;
        runtime_only.effect_die_sides_raw = i64::from(i32::MAX) + 1;
        runtime_only.effect_trigger_spell_raw = -1;
        runtime_only.effect_misc_value_raw = [-1, i64::from(i32::MAX) + 1];
        runtime_only.effect_coefficient_bits = f32::NAN.to_bits();
        runtime_only.effect_variance_bits = f32::INFINITY.to_bits();
        let mut dual_wield = effect(2, 100, 0, 1, i64::from(SPELL_EFFECT_DUAL_WIELD_LIKE_CPP));
        dual_wield.effect_base_points_raw = i64::from(i32::MAX) + 1;
        dual_wield.effect_coefficient_bits = f32::NAN.to_bits();
        let mut skill = effect(3, 100, 0, 2, i64::from(SPELL_EFFECT_SKILL_LIKE_CPP));
        skill.effect_misc_value_raw[0] = 777;
        skill.effect_base_points_raw = 1;

        let catalog = catalog(
            [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects: vec![runtime_only, dual_wield, skill],
                ..Default::default()
            },
        );

        let SpellAcquisitionEffectsLookupLikeCpp::Covered(effects) =
            catalog.acquisition_effects_like_cpp(100)
        else {
            panic!("payload unused by acquisition must not poison the whole spell");
        };
        assert_eq!(
            effects
                .iter()
                .map(|effect| effect.record_id)
                .collect::<Vec<_>>(),
            vec![2, 3]
        );
    }

    #[test]
    fn metadata_payload_validity_is_scoped_to_each_consumed_field() {
        let catalog = catalog(
            [
                SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0),
                SpellAcquisitionCoverageSeedLikeCpp::covered(200, 0),
            ],
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_misc: vec![
                    SpellAcquisitionMiscLikeCpp {
                        record_id: 1,
                        spell_id_raw: 100,
                        difficulty_id_raw: 0,
                        attributes_raw: [i64::from(SPELL_ATTR0_PASSIVE_LIKE_CPP), 0],
                        show_future_spell_player_condition_id_raw: i64::from(i32::MAX) + 1,
                    },
                    SpellAcquisitionMiscLikeCpp {
                        record_id: 2,
                        spell_id_raw: 200,
                        difficulty_id_raw: 0,
                        attributes_raw: [i64::MAX, 0],
                        show_future_spell_player_condition_id_raw: 44,
                    },
                ],
                spell_levels: vec![SpellAcquisitionLevelsLikeCpp {
                    record_id: 3,
                    spell_id_raw: 100,
                    difficulty_id_raw: 0,
                    base_level_raw: i64::from(i16::MAX) + 1,
                    spell_level_raw: 20,
                }],
                ..Default::default()
            },
        );

        let SpellAcquisitionMetadataLookupLikeCpp::Present(first_misc) =
            catalog.misc_for_spell_like_cpp(100, 0)
        else {
            panic!("final Misc row must remain present");
        };
        assert_eq!(first_misc.is_passive_checked(), Ok(true));
        assert!(first_misc.future_player_condition_id_checked().is_err());

        let SpellAcquisitionMetadataLookupLikeCpp::Present(second_misc) =
            catalog.misc_for_spell_like_cpp(200, 0)
        else {
            panic!("final Misc row must remain present");
        };
        assert!(second_misc.is_passive_checked().is_err());
        assert_eq!(
            second_misc.future_player_condition_id_checked(),
            Ok(Some(44))
        );

        let SpellAcquisitionMetadataLookupLikeCpp::Present(levels) =
            catalog.levels_for_spell_like_cpp(100, 0)
        else {
            panic!("final Levels row must remain present");
        };
        assert!(levels.base_level_checked().is_err());
        assert_eq!(levels.spell_level_checked(), Ok(20));
    }

    #[test]
    fn invalid_difficulty_is_scoped_to_the_related_spell() {
        let invalid_effect = effect(1, 100, 700, 0, i64::from(SPELL_EFFECT_SKILL_LIKE_CPP));
        let catalog = catalog(
            [
                SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0),
                SpellAcquisitionCoverageSeedLikeCpp::covered(200, 0),
            ],
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects: vec![invalid_effect],
                spell_misc: vec![SpellAcquisitionMiscLikeCpp {
                    record_id: 2,
                    spell_id_raw: 100,
                    difficulty_id_raw: 700,
                    attributes_raw: [0, 0],
                    show_future_spell_player_condition_id_raw: 0,
                }],
                spell_levels: vec![SpellAcquisitionLevelsLikeCpp {
                    record_id: 3,
                    spell_id_raw: 100,
                    difficulty_id_raw: 700,
                    base_level_raw: 1,
                    spell_level_raw: 1,
                }],
                ..Default::default()
            },
        );

        assert!(matches!(
            catalog.effects_for_spell_difficulty_like_cpp(100, 0),
            SpellAcquisitionEffectsLookupLikeCpp::Indeterminate(_)
        ));
        assert!(matches!(
            catalog.misc_for_spell_like_cpp(100, 0),
            SpellAcquisitionMetadataLookupLikeCpp::Indeterminate(_)
        ));
        assert!(matches!(
            catalog.levels_for_spell_like_cpp(100, 0),
            SpellAcquisitionMetadataLookupLikeCpp::Indeterminate(_)
        ));
        assert_eq!(
            catalog.effects_for_spell_difficulty_like_cpp(200, 0),
            SpellAcquisitionEffectsLookupLikeCpp::Covered(&[])
        );
        assert_eq!(
            catalog.misc_for_spell_like_cpp(200, 0),
            SpellAcquisitionMetadataLookupLikeCpp::CoveredWithoutRow
        );
        assert_eq!(
            catalog.levels_for_spell_like_cpp(200, 0),
            SpellAcquisitionMetadataLookupLikeCpp::CoveredWithoutRow
        );
    }

    #[test]
    fn shadowed_invalid_payload_does_not_poison_the_final_winner() {
        let mut invalid_lower = effect(10, 100, 0, 0, i64::from(SPELL_EFFECT_SKILL_LIKE_CPP));
        invalid_lower.effect_misc_value_raw[0] = -1;
        let winner = effect(20, 100, 0, 0, i64::from(SPELL_EFFECT_DUAL_WIELD_LIKE_CPP));
        let catalog = catalog(
            [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects: vec![winner, invalid_lower],
                spell_misc: vec![
                    SpellAcquisitionMiscLikeCpp {
                        record_id: 10,
                        spell_id_raw: 100,
                        difficulty_id_raw: 0,
                        attributes_raw: [i64::MAX, 0],
                        show_future_spell_player_condition_id_raw: 0,
                    },
                    SpellAcquisitionMiscLikeCpp {
                        record_id: 20,
                        spell_id_raw: 100,
                        difficulty_id_raw: 0,
                        attributes_raw: [0, 0],
                        show_future_spell_player_condition_id_raw: 0,
                    },
                ],
                spell_levels: vec![
                    SpellAcquisitionLevelsLikeCpp {
                        record_id: 10,
                        spell_id_raw: 100,
                        difficulty_id_raw: 0,
                        base_level_raw: i64::MAX,
                        spell_level_raw: 1,
                    },
                    SpellAcquisitionLevelsLikeCpp {
                        record_id: 20,
                        spell_id_raw: 100,
                        difficulty_id_raw: 0,
                        base_level_raw: 1,
                        spell_level_raw: 1,
                    },
                ],
                ..Default::default()
            },
        );

        let SpellAcquisitionEffectsLookupLikeCpp::Covered(effects) =
            catalog.effects_for_spell_difficulty_like_cpp(100, 0)
        else {
            panic!("valid higher RecordID must determine the slot");
        };
        assert_eq!(effects[0].record_id, 20);
        assert!(matches!(
            catalog.misc_for_spell_like_cpp(100, 0),
            SpellAcquisitionMetadataLookupLikeCpp::Present(row) if row.record_id == 20
        ));
        assert!(matches!(
            catalog.levels_for_spell_like_cpp(100, 0),
            SpellAcquisitionMetadataLookupLikeCpp::Present(row) if row.record_id == 20
        ));
    }

    #[test]
    fn difficulty_fallback_merges_effect_slots_and_uses_first_metadata() {
        let requested_slot = effect(30, 100, 2, 0, i64::from(SPELL_EFFECT_DUAL_WIELD_LIKE_CPP));
        let shadowed_fallback_slot =
            effect(20, 100, 1, 0, i64::from(SPELL_EFFECT_DUAL_WIELD_LIKE_CPP));
        let fallback_slot = effect(21, 100, 1, 1, i64::from(SPELL_EFFECT_DUAL_WIELD_LIKE_CPP));
        let final_slot = effect(10, 100, 0, 2, i64::from(SPELL_EFFECT_DUAL_WIELD_LIKE_CPP));
        let catalog = catalog(
            [
                SpellAcquisitionCoverageSeedLikeCpp::covered(100, 2),
                SpellAcquisitionCoverageSeedLikeCpp::covered(100, 1),
                SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0),
            ],
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects: vec![
                    final_slot,
                    fallback_slot,
                    shadowed_fallback_slot,
                    requested_slot,
                ],
                spell_misc: vec![
                    SpellAcquisitionMiscLikeCpp {
                        record_id: 1,
                        spell_id_raw: 100,
                        difficulty_id_raw: 1,
                        attributes_raw: [0, 0],
                        show_future_spell_player_condition_id_raw: 11,
                    },
                    SpellAcquisitionMiscLikeCpp {
                        record_id: 2,
                        spell_id_raw: 100,
                        difficulty_id_raw: 0,
                        attributes_raw: [0, 0],
                        show_future_spell_player_condition_id_raw: 22,
                    },
                ],
                ..Default::default()
            },
        );

        let SpellAcquisitionResolvedEffectsLookupLikeCpp::Covered(effects) =
            catalog.resolved_effects_for_difficulty_chain_like_cpp(100, [2, 1, 0])
        else {
            panic!("complete fallback chain must resolve");
        };
        assert_eq!(
            effects
                .iter()
                .map(|effect| effect.record_id)
                .collect::<Vec<_>>(),
            vec![30, 21, 10]
        );
        assert!(matches!(
            catalog.resolved_misc_for_difficulty_chain_like_cpp(100, [2, 1, 0]),
            SpellAcquisitionResolvedMetadataLookupLikeCpp::Present(row)
                if row.record_id == 1
        ));
        let SpellAcquisitionResolvedEffectsLookupLikeCpp::Covered(effects) =
            catalog.resolved_effects_for_difficulty_chain_like_cpp(100, [2, 9, 1, 0])
        else {
            panic!("absent intermediate fallback must be skipped");
        };
        assert_eq!(
            effects
                .iter()
                .map(|effect| effect.record_id)
                .collect::<Vec<_>>(),
            vec![30, 21, 10]
        );
        assert!(matches!(
            catalog.resolved_misc_for_difficulty_chain_like_cpp(100, [2, 9, 1, 0]),
            SpellAcquisitionResolvedMetadataLookupLikeCpp::Present(row)
                if row.record_id == 1
        ));
        assert!(matches!(
            catalog.resolved_effects_for_difficulty_chain_like_cpp(100, [9, 2]),
            SpellAcquisitionResolvedEffectsLookupLikeCpp::MissingCoverage { difficulty_id: 9 }
        ));
    }

    #[test]
    fn all_final_effects_remain_visible_to_the_planner() {
        let unsupported_runtime_effect = effect(1, 100, 0, 0, 3);
        let learn_effect = {
            let mut row = effect(2, 100, 0, 1, i64::from(SPELL_EFFECT_LEARN_SPELL_LIKE_CPP));
            row.effect_trigger_spell_raw = 200;
            row
        };
        let catalog = catalog(
            [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects: vec![unsupported_runtime_effect, learn_effect],
                ..Default::default()
            },
        );

        let SpellAcquisitionEffectsLookupLikeCpp::Covered(all_effects) =
            catalog.difficulty_none_effects_like_cpp(100)
        else {
            panic!("all effects must be available");
        };
        assert_eq!(all_effects.len(), 2);
        assert_eq!(all_effects[0].effect_type_checked(), Ok(3));

        let SpellAcquisitionEffectsLookupLikeCpp::Covered(acquisition_effects) =
            catalog.acquisition_effects_like_cpp(100)
        else {
            panic!("filtered acquisition effects must be available");
        };
        assert_eq!(acquisition_effects.len(), 1);
        assert_eq!(acquisition_effects[0].record_id, 2);
    }

    #[test]
    fn invalid_dependency_remains_visible_and_fails_source_closed() {
        let catalog = catalog(
            [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_learn_spells: vec![SpellAcquisitionDependencyLikeCpp {
                    record_id: 1,
                    spell_id_raw: 100,
                    learn_spell_id_raw: -1,
                    overrides_spell_id_raw: 0,
                }],
                ..Default::default()
            },
        );

        assert_eq!(
            catalog.effective_dependency_rows_like_cpp().count(),
            1,
            "invalid final rows remain inspectable"
        );
        assert!(matches!(
            catalog.dependency_rows_lookup_like_cpp(100),
            SpellAcquisitionDependenciesLookupLikeCpp::Indeterminate(_)
        ));
        assert!(matches!(
            catalog.acquisition_effects_like_cpp(100),
            SpellAcquisitionEffectsLookupLikeCpp::Covered(_)
        ));
    }

    #[test]
    fn unrepresentable_final_effect_relation_marks_all_coverage_indeterminate() {
        let composed = compose_effective_table_like_cpp(
            [(
                1,
                effect(1, 100, 0, 0, i64::from(SPELL_EFFECT_SKILL_LIKE_CPP)),
            )],
            [(
                1,
                effect(1, -1, 0, 0, i64::from(SPELL_EFFECT_SKILL_LIKE_CPP)),
            )],
            [],
            0xAABB,
            &Db2HotfixRemovalStoreLikeCpp::default(),
        );
        let catalog = catalog(
            [
                SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0),
                SpellAcquisitionCoverageSeedLikeCpp::covered(200, 0),
            ],
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects: composed.into_values().collect(),
                ..Default::default()
            },
        );

        assert!(matches!(
            catalog.difficulty_none_effects_like_cpp(100),
            SpellAcquisitionEffectsLookupLikeCpp::Indeterminate(_)
        ));
        assert!(matches!(
            catalog.difficulty_none_effects_like_cpp(200),
            SpellAcquisitionEffectsLookupLikeCpp::Indeterminate(_)
        ));
    }

    #[test]
    fn invalid_final_talent_rank_is_not_misclassified_as_not_talent() {
        let catalog = catalog(
            [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
            EffectiveSpellAcquisitionRowsLikeCpp {
                talents: vec![SpellAcquisitionTalentLikeCpp {
                    record_id: 1,
                    spell_rank_raw: [-1, 0, 0, 0, 0, 0, 0, 0, 0],
                }],
                ..Default::default()
            },
        );

        assert!(matches!(
            catalog.talent_membership_like_cpp(100),
            SpellAcquisitionTalentLookupLikeCpp::Indeterminate(_)
        ));
        assert!(matches!(
            catalog.difficulty_none_effects_like_cpp(100),
            SpellAcquisitionEffectsLookupLikeCpp::Covered(_)
        ));
    }

    #[test]
    fn valid_talent_membership_wins_over_unrelated_invalid_rows() {
        let catalog = catalog(
            [
                SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0),
                SpellAcquisitionCoverageSeedLikeCpp::covered(200, 0),
            ],
            EffectiveSpellAcquisitionRowsLikeCpp {
                talents: vec![
                    SpellAcquisitionTalentLikeCpp {
                        record_id: 1,
                        spell_rank_raw: [100, 0, 0, 0, 0, 0, 0, 0, 0],
                    },
                    SpellAcquisitionTalentLikeCpp {
                        record_id: 2,
                        spell_rank_raw: [-1, 0, 0, 0, 0, 0, 0, 0, 0],
                    },
                ],
                ..Default::default()
            },
        );

        assert_eq!(
            catalog.talent_membership_like_cpp(100),
            SpellAcquisitionTalentLookupLikeCpp::Talent
        );
        assert!(matches!(
            catalog.talent_membership_like_cpp(200),
            SpellAcquisitionTalentLookupLikeCpp::Indeterminate(_)
        ));
    }

    #[test]
    fn battle_pet_uses_all_difficulties_and_coalesces_same_species() {
        let catalog = catalog(
            [
                SpellAcquisitionCoverageSeedLikeCpp::covered(100, 2),
                SpellAcquisitionCoverageSeedLikeCpp::covered(100, 3),
            ],
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects: vec![
                    summon(1, 100, 2, 0, 900, 700),
                    summon(2, 100, 3, 1, 900, 700),
                ],
                summon_properties: vec![SpellAcquisitionSummonPropertiesLikeCpp {
                    record_id: 700,
                    slot_raw: SUMMON_SLOT_MINIPET_LIKE_CPP,
                    flags_1_raw: i64::from(SUMMON_FROM_BATTLE_PET_JOURNAL_LIKE_CPP),
                }],
                battle_pet_species: vec![SpellAcquisitionBattlePetSpeciesLikeCpp {
                    species_id: 50,
                    creature_id_raw: 900,
                }],
                ..Default::default()
            },
        );

        assert_eq!(
            catalog
                .summon_effects_all_difficulties_like_cpp(100)
                .count(),
            2
        );
        assert_eq!(
            catalog.battle_pet_classification_like_cpp(100),
            BattlePetClassificationLikeCpp::Species(50)
        );
    }

    #[test]
    fn battle_pet_requires_exact_coverage_for_each_summon_difficulty() {
        let catalog = catalog(
            [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects: vec![summon(1, 100, 2, 0, 900, 700)],
                summon_properties: vec![SpellAcquisitionSummonPropertiesLikeCpp {
                    record_id: 700,
                    slot_raw: SUMMON_SLOT_MINIPET_LIKE_CPP,
                    flags_1_raw: i64::from(SUMMON_FROM_BATTLE_PET_JOURNAL_LIKE_CPP),
                }],
                battle_pet_species: vec![SpellAcquisitionBattlePetSpeciesLikeCpp {
                    species_id: 50,
                    creature_id_raw: 900,
                }],
                ..Default::default()
            },
        );

        assert!(matches!(
            catalog.battle_pet_classification_like_cpp(100),
            BattlePetClassificationLikeCpp::Indeterminate(ref reasons)
                if reasons.iter().any(|reason| matches!(
                    reason,
                    BattlePetIndeterminateReasonLikeCpp::MissingSpellDifficultyCoverage {
                        spell_id: 100,
                        difficulty_id: 2,
                        effect_record_id: 1,
                    }
                ))
        ));
    }

    #[test]
    fn battle_pet_requires_spell_coverage_and_treats_null_properties_as_nonqualifying() {
        let rows = EffectiveSpellAcquisitionRowsLikeCpp {
            spell_effects: vec![summon(1, 100, 0, 0, 900, 0)],
            ..Default::default()
        };
        let missing = catalog([], rows.clone());
        assert!(matches!(
            missing.battle_pet_classification_like_cpp(100),
            BattlePetClassificationLikeCpp::Indeterminate(ref reasons)
                if reasons.iter().any(|reason| matches!(
                    reason,
                    BattlePetIndeterminateReasonLikeCpp::MissingSpellCoverage {
                        spell_id: 100
                    }
                ))
        ));

        let unavailable = catalog(
            [SpellAcquisitionCoverageSeedLikeCpp::indeterminate(
                100,
                700,
                SpellAcquisitionIndeterminateReasonLikeCpp::ServerSideMetadataUnavailable,
            )],
            rows.clone(),
        );
        assert!(matches!(
            unavailable.battle_pet_classification_like_cpp(100),
            BattlePetClassificationLikeCpp::Indeterminate(ref reasons)
                if reasons.iter().any(|reason| matches!(
                    reason,
                    BattlePetIndeterminateReasonLikeCpp::SpellCoverage {
                        reason: SpellAcquisitionIndeterminateReasonLikeCpp::ServerSideMetadataUnavailable,
                        ..
                    }
                ))
        ));

        let covered = catalog([SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)], rows);
        assert_eq!(
            covered.battle_pet_classification_like_cpp(100),
            BattlePetClassificationLikeCpp::NotBattlePet
        );
    }

    #[test]
    fn battle_pet_distinguishes_removed_properties_and_species() {
        let qualifying_properties = SpellAcquisitionSummonPropertiesLikeCpp {
            record_id: 700,
            slot_raw: SUMMON_SLOT_MINIPET_LIKE_CPP,
            flags_1_raw: i64::from(SUMMON_FROM_BATTLE_PET_JOURNAL_LIKE_CPP),
        };
        let removed_properties = catalog_with_removed(
            [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects: vec![summon(1, 100, 0, 0, 900, 700)],
                ..Default::default()
            },
            vec![SpellAcquisitionRemovedRowLikeCpp::SummonProperties(
                qualifying_properties.clone(),
            )],
        );
        assert!(matches!(
            removed_properties.battle_pet_classification_like_cpp(100),
            BattlePetClassificationLikeCpp::Indeterminate(ref reasons)
                if reasons.iter().any(|reason| matches!(
                    reason,
                    BattlePetIndeterminateReasonLikeCpp::RemovedSummonProperties {
                        properties_id: 700,
                        ..
                    }
                ))
        ));

        let removed_species = catalog_with_removed(
            [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects: vec![summon(1, 100, 0, 0, 900, 700)],
                summon_properties: vec![qualifying_properties],
                ..Default::default()
            },
            vec![SpellAcquisitionRemovedRowLikeCpp::BattlePetSpecies(
                SpellAcquisitionBattlePetSpeciesLikeCpp {
                    species_id: 50,
                    creature_id_raw: 900,
                },
            )],
        );
        assert!(matches!(
            removed_species.battle_pet_classification_like_cpp(100),
            BattlePetClassificationLikeCpp::Indeterminate(ref reasons)
                if reasons.iter().any(|reason| matches!(
                    reason,
                    BattlePetIndeterminateReasonLikeCpp::RemovedSpeciesForCreature {
                        creature_id: 900,
                        ..
                    }
                ))
        ));

        let unknown_removed_properties = catalog_with_removed(
            [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects: vec![summon(1, 100, 0, 0, 900, 700)],
                ..Default::default()
            },
            vec![SpellAcquisitionRemovedRowLikeCpp::Unknown {
                table: SpellAcquisitionTableLikeCpp::SummonProperties,
                record_id: 700,
            }],
        );
        assert!(matches!(
            unknown_removed_properties.battle_pet_classification_like_cpp(100),
            BattlePetClassificationLikeCpp::Indeterminate(ref reasons)
                if reasons.iter().any(|reason| matches!(
                    reason,
                    BattlePetIndeterminateReasonLikeCpp::RemovedSummonProperties {
                        properties_id: 700,
                        ..
                    }
                ))
        ));
    }

    #[test]
    fn battle_pet_conflicts_and_missing_references_are_indeterminate() {
        let conflicting = catalog(
            [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects: vec![summon(1, 100, 0, 0, 900, 700)],
                summon_properties: vec![SpellAcquisitionSummonPropertiesLikeCpp {
                    record_id: 700,
                    slot_raw: SUMMON_SLOT_MINIPET_LIKE_CPP,
                    flags_1_raw: i64::from(SUMMON_FROM_BATTLE_PET_JOURNAL_LIKE_CPP),
                }],
                battle_pet_species: vec![
                    SpellAcquisitionBattlePetSpeciesLikeCpp {
                        species_id: 50,
                        creature_id_raw: 900,
                    },
                    SpellAcquisitionBattlePetSpeciesLikeCpp {
                        species_id: 51,
                        creature_id_raw: 900,
                    },
                ],
                ..Default::default()
            },
        );
        assert!(matches!(
            conflicting.battle_pet_classification_like_cpp(100),
            BattlePetClassificationLikeCpp::Indeterminate(ref reasons)
                if reasons.iter().any(|reason| matches!(
                    reason,
                    BattlePetIndeterminateReasonLikeCpp::ConflictingSpeciesForCreature { .. }
                ))
        ));

        let missing_properties = catalog(
            [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects: vec![summon(1, 100, 0, 0, 900, 700)],
                ..Default::default()
            },
        );
        assert!(matches!(
            missing_properties.battle_pet_classification_like_cpp(100),
            BattlePetClassificationLikeCpp::Indeterminate(_)
        ));

        let missing_species = catalog(
            [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects: vec![summon(1, 100, 0, 0, 900, 700)],
                summon_properties: vec![SpellAcquisitionSummonPropertiesLikeCpp {
                    record_id: 700,
                    slot_raw: SUMMON_SLOT_MINIPET_LIKE_CPP,
                    flags_1_raw: i64::from(SUMMON_FROM_BATTLE_PET_JOURNAL_LIKE_CPP),
                }],
                ..Default::default()
            },
        );
        assert!(matches!(
            missing_species.battle_pet_classification_like_cpp(100),
            BattlePetClassificationLikeCpp::Indeterminate(_)
        ));

        // Mutate a raw final row to exercise corrupt difficulty metadata
        // without narrowing it first.
        let mut invalid_rows = EffectiveSpellAcquisitionRowsLikeCpp::default();
        let mut invalid_summon = summon(2, 101, 0, 0, 900, 700);
        invalid_summon.difficulty_id_raw = -1;
        invalid_rows.spell_effects.push(invalid_summon);
        invalid_rows.summon_properties = vec![SpellAcquisitionSummonPropertiesLikeCpp {
            record_id: 700,
            slot_raw: SUMMON_SLOT_MINIPET_LIKE_CPP,
            flags_1_raw: i64::from(SUMMON_FROM_BATTLE_PET_JOURNAL_LIKE_CPP),
        }];
        invalid_rows.battle_pet_species = vec![SpellAcquisitionBattlePetSpeciesLikeCpp {
            species_id: 50,
            creature_id_raw: 900,
        }];
        let invalid_catalog = catalog(
            [SpellAcquisitionCoverageSeedLikeCpp::covered(101, 0)],
            invalid_rows,
        );
        assert!(matches!(
            invalid_catalog.battle_pet_classification_like_cpp(101),
            BattlePetClassificationLikeCpp::Indeterminate(_)
        ));
    }

    #[test]
    fn species_data_without_qualifying_summon_is_not_authority() {
        let no_summon_catalog = catalog(
            [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
            EffectiveSpellAcquisitionRowsLikeCpp {
                // Deliberately no SUMMON effect. BattlePetSpecies.SummonSpellID
                // is not retained or consulted by this catalog.
                battle_pet_species: vec![SpellAcquisitionBattlePetSpeciesLikeCpp {
                    species_id: 50,
                    creature_id_raw: 900,
                }],
                ..Default::default()
            },
        );
        assert_eq!(
            no_summon_catalog.battle_pet_classification_like_cpp(100),
            BattlePetClassificationLikeCpp::NotBattlePet
        );

        let invalid_species = catalog(
            [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
            EffectiveSpellAcquisitionRowsLikeCpp {
                battle_pet_species: vec![SpellAcquisitionBattlePetSpeciesLikeCpp {
                    species_id: 51,
                    creature_id_raw: -1,
                }],
                ..Default::default()
            },
        );
        assert_eq!(
            invalid_species.battle_pet_classification_like_cpp(100),
            BattlePetClassificationLikeCpp::NotBattlePet,
            "unreferenced species corruption cannot turn a covered zero-SUMMON spell indeterminate"
        );

        let unreadable_species = catalog(
            [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects: vec![summon(1, 100, 0, 0, 900, 700)],
                summon_properties: vec![SpellAcquisitionSummonPropertiesLikeCpp {
                    record_id: 700,
                    slot_raw: SUMMON_SLOT_MINIPET_LIKE_CPP,
                    flags_1_raw: i64::from(SUMMON_FROM_BATTLE_PET_JOURNAL_LIKE_CPP),
                }],
                battle_pet_species: vec![
                    SpellAcquisitionBattlePetSpeciesLikeCpp {
                        species_id: 50,
                        creature_id_raw: 900,
                    },
                    SpellAcquisitionBattlePetSpeciesLikeCpp {
                        species_id: 51,
                        creature_id_raw: UNREADABLE_SQL_RAW_LIKE_CPP,
                    },
                ],
                ..Default::default()
            },
        );
        assert!(matches!(
            unreadable_species.battle_pet_classification_like_cpp(100),
            BattlePetClassificationLikeCpp::Indeterminate(ref reasons)
                if reasons.iter().any(|reason| matches!(
                    reason,
                    BattlePetIndeterminateReasonLikeCpp::EffectiveTableIncomplete {
                        table: SpellAcquisitionTableLikeCpp::BattlePetSpecies,
                        ..
                    }
                ))
        ));
    }

    #[test]
    fn runtime_hash_bundle_is_preserved_without_constants() {
        let hashes = SpellAcquisitionTableHashesLikeCpp {
            spell_effect: 1,
            spell_learn_spell: 2,
            spell_misc: 3,
            spell_levels: 4,
            talent: 5,
            summon_properties: 6,
            battle_pet_species: 7,
        };
        let catalog = SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
            [],
            EffectiveSpellAcquisitionRowsLikeCpp::default(),
            hashes,
            Vec::new(),
        );
        assert_eq!(catalog.table_hashes_like_cpp(), hashes);
    }
}
