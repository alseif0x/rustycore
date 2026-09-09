//! Spell acquisition model state definitions, part 1 of 3.
//!
//! Separated from the spell_acquisition.rs root under #646. Behaviour is preserved.

use super::*;

pub(super) const DIFFICULTY_NONE_LIKE_CPP: u32 = 0;

pub(super) const SPELL_EFFECT_SUMMON_LIKE_CPP: u32 = 28;

pub(super) const SPELL_EFFECT_LEARN_SPELL_LIKE_CPP: u32 = 36;

pub(super) const SPELL_EFFECT_DUAL_WIELD_LIKE_CPP: u32 = 40;

pub(super) const SPELL_EFFECT_SKILL_STEP_LIKE_CPP: u32 = 44;

pub(super) const SPELL_EFFECT_SKILL_LIKE_CPP: u32 = 118;

pub(super) const MAX_SPELL_EFFECTS_LIKE_CPP: i64 = 32;

pub(super) const TOTAL_SPELL_EFFECTS_LIKE_CPP: i64 = 316;

pub(super) const TOTAL_SPELL_TARGETS_LIKE_CPP: i64 = 153;

pub(super) const TARGET_UNIT_PET_LIKE_CPP: i64 = 5;

pub(super) const TARGET_NONE_LIKE_CPP: i64 = 0;

pub(super) const TARGET_UNIT_CASTER_LIKE_CPP: i64 = 1;

pub(super) const TARGET_UNIT_TARGET_ALLY_LIKE_CPP: i64 = 21;

pub(super) const SPELL_ATTR0_PASSIVE_LIKE_CPP: u32 = 0x0000_0040;

pub(super) const SPELL_ATTR0_NO_IMMUNITIES_LIKE_CPP: u32 = 0x2000_0000;

pub(super) const SPELL_ATTR1_IS_CHANNELLED_LIKE_CPP: u32 = 0x0000_0004;

pub(super) const SPELL_ATTR1_IS_SELF_CHANNELLED_LIKE_CPP: u32 = 0x0000_0040;

pub(super) const SPELL_ATTR1_CAST_WHEN_LEARNED_LIKE_CPP: u32 = 0x8000_0000;

pub(super) const SUMMON_SLOT_MINIPET_LIKE_CPP: i64 = 5;

pub(super) const SUMMON_FROM_BATTLE_PET_JOURNAL_LIKE_CPP: u32 = 0x0020_0000;

// C++ `SpellEffectEntry::EffectBasePoints` is `int32`
// (`DB2Structure.h` / `SpellEffectLoadInfo`), and the hotfix
// `spell_effect.EffectBasePoints` column has the same signed integer domain.
// Do not confuse it with `world.serverside_spell_effect.EffectBasePoints`,
// which is a float source outside this catalog: server-side spell keys are
// explicitly seeded as `ServerSideMetadataUnavailable`.
pub(super) const SPELL_EFFECT_WDC_CHAIN_TARGETS_FIELD: usize = 10;

pub(super) const SPELL_EFFECT_WDC_POINTS_PER_RESOURCE_FIELD: usize = 14;

pub(super) const SPELL_EFFECT_WDC_REAL_POINTS_PER_LEVEL_FIELD: usize = 16;

pub(super) const SPELL_EFFECT_SQL_CHAIN_TARGETS_COLUMN: usize = 14;

pub(super) const SPELL_EFFECT_SQL_POINTS_PER_RESOURCE_COLUMN: usize = 15;

pub(super) const SPELL_EFFECT_SQL_REAL_POINTS_PER_LEVEL_COLUMN: usize = 16;

pub(super) const SPELL_EFFECT_SQL_ITEM_TYPE_COLUMN: usize = 17;

pub(super) const SPELL_EFFECT_SQL_AURA_COLUMN: usize = 18;

pub(super) const SPELL_EFFECT_SQL_MECHANIC_COLUMN: usize = 19;

pub(super) const SPELL_EFFECT_SQL_ATTRIBUTES_COLUMN: usize = 20;

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

/// SQLx-free bridge row used only between the typed persistence adapter and
/// this domain parser. Column positions remain private to the source family;
/// the public persistence port exposes named DTO fields instead.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellAcquisitionSqlOverlayRowLikeCpp {
    pub integer_columns: Vec<Option<i64>>,
    pub float_columns_bits: Vec<Option<u32>>,
}

pub type SpellAcquisitionSqlOverlayFutureLikeCpp<'a> =
    Pin<Box<dyn Future<Output = Result<Vec<SpellAcquisitionSqlOverlayRowLikeCpp>>> + Send + 'a>>;

pub trait SpellAcquisitionSqlOverlaySourceLikeCpp: Send + Sync {
    fn load_overlay_like_cpp(
        &self,
        table: SpellAcquisitionTableLikeCpp,
        official: bool,
    ) -> SpellAcquisitionSqlOverlayFutureLikeCpp<'_>;
}

impl SpellAcquisitionTableLikeCpp {
    pub(super) const ALL: [Self; 7] = [
        Self::SpellEffect,
        Self::SpellLearnSpell,
        Self::SpellMisc,
        Self::SpellLevels,
        Self::Talent,
        Self::SummonProperties,
        Self::BattlePetSpecies,
    ];

    pub(super) const fn file_name(self) -> &'static str {
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
    /// Raw `SpellEffectEntry::EffectAura` (`int16`).
    pub effect_aura_raw: i64,
    /// Raw `SpellEffectEntry::EffectMechanic` (`int32`).
    pub effect_mechanic_raw: i64,
    /// Raw `SpellEffectEntry::EffectAttributes` (`int32` bitmask).
    pub effect_attributes_raw: i64,
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
    /// Raw regular DB2/hotfix `SpellEffectEntry::EffectItemType` (`int32`).
    pub effect_item_type_raw: i64,
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

    pub fn item_type_checked(&self) -> Result<u32, InvalidAcquisitionValueLikeCpp> {
        source_i32(self.effect_item_type_raw, "SpellEffect.EffectItemType")?;
        nonnegative_u32(self.effect_item_type_raw, "SpellEffect.EffectItemType")
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

pub(super) fn checked_rounded_i32(
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
    pub fn no_immunities_checked(&self) -> Result<bool, InvalidAcquisitionValueLikeCpp> {
        Ok(
            checked_u32_bits(self.attributes_raw[0], "SpellMisc.Attributes1")?
                & SPELL_ATTR0_NO_IMMUNITIES_LIKE_CPP
                != 0,
        )
    }

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

    pub fn is_channeled_checked(&self) -> Result<bool, InvalidAcquisitionValueLikeCpp> {
        Ok(
            checked_u32_bits(self.attributes_raw[1], "SpellMisc.Attributes2")?
                & (SPELL_ATTR1_IS_CHANNELLED_LIKE_CPP | SPELL_ATTR1_IS_SELF_CHANNELLED_LIKE_CPP)
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

    pub(super) const fn hotfix_record_id_like_cpp(&self) -> i32 {
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
pub(super) struct CoverageRecordLikeCpp {
    pub(super) reasons_by_table:
        BTreeMap<SpellAcquisitionTableLikeCpp, Vec<SpellAcquisitionIndeterminateReasonLikeCpp>>,
}

impl CoverageRecordLikeCpp {
    pub(super) fn add_source_reason_like_cpp(
        &mut self,
        reason: SpellAcquisitionIndeterminateReasonLikeCpp,
    ) {
        for table in SpellAcquisitionTableLikeCpp::ALL {
            self.add_table_reason_like_cpp(table, reason.clone());
        }
    }

    pub(super) fn add_table_reason_like_cpp(
        &mut self,
        table: SpellAcquisitionTableLikeCpp,
        reason: SpellAcquisitionIndeterminateReasonLikeCpp,
    ) {
        let reasons = self.reasons_by_table.entry(table).or_default();
        if !reasons.contains(&reason) {
            reasons.push(reason);
        }
    }

    pub(super) fn reasons_for_table_like_cpp(
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
    pub(super) table_hashes: SpellAcquisitionTableHashesLikeCpp,
    pub(super) coverage_by_key: BTreeMap<(u32, u32), CoverageRecordLikeCpp>,
    pub(super) effects_by_key: BTreeMap<(u32, u32), Vec<SpellAcquisitionEffectLikeCpp>>,
    pub(super) acquisition_effects_by_key: BTreeMap<(u32, u32), Vec<SpellAcquisitionEffectLikeCpp>>,
    pub(super) summon_effects_by_spell: BTreeMap<u32, Vec<SpellAcquisitionEffectLikeCpp>>,
    pub(super) dependencies_by_spell: BTreeMap<u32, Vec<SpellAcquisitionDependencyLikeCpp>>,
    pub(super) dependency_rows: Vec<SpellAcquisitionDependencyLikeCpp>,
    pub(super) misc_by_key: BTreeMap<(u32, u32), SpellAcquisitionMiscLikeCpp>,
    pub(super) levels_by_key: BTreeMap<(u32, u32), SpellAcquisitionLevelsLikeCpp>,
    pub(super) talent_spell_ids: BTreeSet<u32>,
    pub(super) summon_properties_by_id: BTreeMap<u32, SpellAcquisitionSummonPropertiesLikeCpp>,
    pub(super) species_by_creature: BTreeMap<u32, BTreeSet<u32>>,
    pub(super) removed_rows: Vec<SpellAcquisitionRemovedRowLikeCpp>,
    pub(super) removed_summon_properties_ids: BTreeSet<u32>,
    pub(super) removed_species_by_creature: BTreeMap<u32, BTreeSet<u32>>,
    pub(super) global_indeterminate_by_table:
        BTreeMap<SpellAcquisitionTableLikeCpp, Vec<SpellAcquisitionIndeterminateReasonLikeCpp>>,
    pub(super) diagnostics: Vec<SpellAcquisitionDiagnosticLikeCpp>,
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
