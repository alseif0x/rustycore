//! 02245dcd SpellInfo.cpp:516-752,869-963; SpellInfo.h:266-267.
//! Null caster/target startup calculation only (item ID 0, item level -1).
//! Not the live Player/trait/combo/mastery/modifier operation. Random draws
//! are a required caller capability, never an invented mean/legacy RNG.
use super::{
    SpellConstructorFields, SpellDefinitionError, SpellDefinitionSeeds, SpellEffectValues,
};
use wow_data::{
    forever_birth::item_records::ItemCatalog,
    forever_game_tables::SpellValueGameTables,
    forever_spells::{ExpectedStatType, SpellCatalog},
};
#[cfg(test)]
mod tests;

const SCALES_CREATURE_LEVEL: u32 = 0x0008_0000;
const USE_BASE_LEVEL: u32 = 0x0000_1000;
const SCALES_ITEM_LEVEL: u32 = 0x0000_0004;
const FLOAT_AMOUNTS: u32 = 0x0020_0000;
const MIN_VALUE: f64 = -2_000_000_000.0;
const MAX_VALUE: f64 = 2_000_000_000.0;

/// Values/IDs/SQL contents are deliberately absent from startup errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellValueError {
    DefinitionLookup(SpellDefinitionError),
    MissingGameTables,
    MissingScalingRow,
    InvalidCatalogInputs,
    InvalidVarianceRange,
    InvalidVarianceDraw,
    RandomSourceUnavailable,
    UndefinedIntegerCast,
}

/// Transient result, not a cached numeric mirror or initialized-spell marker.
/// None means source did not write the variance out-parameter (variance zero).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StartupSpellValue {
    pub value: f64,
    pub variance: Option<f32>,
}
impl StartupSpellValue {
    /// Source casts after final f64 rounding/clamping, truncating toward zero.
    /// NaN has no defined C++ int32 conversion; never let Rust turn it into 0.
    pub fn as_int(self) -> Result<i32, SpellValueError> {
        let integral = self.value.trunc();
        if !integral.is_finite() || integral < f64::from(i32::MIN) || integral > f64::from(i32::MAX)
        {
            return Err(SpellValueError::UndefinedIntegerCast);
        }
        Ok(integral as i32)
    }
}

impl SpellDefinitionSeeds {
    /// The lookup uses the same exact/difficulty fallback authority as all
    /// other definition consumers. None is no definition/effect, not zero.
    pub fn calculate_startup_base_value(
        &self,
        spell: u32,
        difficulty: i16,
        effect: usize,
        items: &ItemCatalog,
    ) -> Result<Option<f64>, SpellValueError> {
        let Some(definition) = self
            .get(spell, difficulty)
            .map_err(SpellValueError::DefinitionLookup)?
        else {
            return Ok(None);
        };
        let Some(effect) = definition.definition.effects.get(effect) else {
            return Ok(None);
        };
        base_value(
            definition.fields(),
            effect,
            &self.catalog,
            self.value_game_tables(),
            items,
        )
        .map(Some)
    }

    /// Full null-caster/null-target CalcValue, including optional base-point
    /// override. CalcBaseValue still runs first; variance always scales that
    /// calculated base, not the override. The source-compatible random producer
    /// must be supplied before the custom/positivity startup phase is enabled.
    pub fn calculate_startup_value(
        &self,
        spell: u32,
        difficulty: i16,
        effect: usize,
        items: &ItemCatalog,
        base_points: Option<f64>,
        draw: &mut impl FnMut(f32, f32) -> Result<f32, SpellValueError>,
    ) -> Result<Option<StartupSpellValue>, SpellValueError> {
        let Some(definition) = self
            .get(spell, difficulty)
            .map_err(SpellValueError::DefinitionLookup)?
        else {
            return Ok(None);
        };
        let Some(effect) = definition.definition.effects.get(effect) else {
            return Ok(None);
        };
        let base = base_value(
            definition.fields(),
            effect,
            &self.catalog,
            self.value_game_tables(),
            items,
        )?;
        finish_value(effect, base, base_points, draw).map(Some)
    }
}

fn base_value(
    fields: &SpellConstructorFields,
    effect: &SpellEffectValues,
    catalog: &SpellCatalog,
    tables: Option<&SpellValueGameTables>,
    items: &ItemCatalog,
) -> Result<f64, SpellValueError> {
    let float_amounts = fields.attributes[12] & FLOAT_AMOUNTS != 0;
    if effect.scaling_coefficient != 0.0 {
        let item_scaled = fields.attributes[11] & SCALES_ITEM_LEVEL != 0;
        let mut level = fields.spell_level;
        if fields.base_level != 0 && !item_scaled && fields.attributes[10] & USE_BASE_LEVEL != 0 {
            level = fields.base_level;
        }
        if fields.min_scaling_level != 0 && fields.min_scaling_level > level {
            level = fields.min_scaling_level;
        }
        if fields.max_scaling_level != 0 && fields.max_scaling_level < level {
            level = fields.max_scaling_level;
        }
        let mut value = 0.0f32;
        if level > 0 {
            if effect.scaling_class == 0 {
                // Source early return precedes even NaN coefficient arithmetic.
                return Ok(0.0);
            }
            // Default itemLevel=-1 maps to 1, independently of the spell level.
            if item_scaled {
                if matches!(effect.scaling_class, -8 | -9) {
                    let row = catalog
                        .rand_prop_points_or_last(1)
                        .map_err(|_| SpellValueError::InvalidCatalogInputs)?;
                    value = if effect.scaling_class == -8 {
                        row.damage_replace_stat_f
                    } else {
                        row.damage_secondary_f
                    };
                } else {
                    value = catalog
                        .random_property_points(1, 3, 5, 0)
                        .map_err(|_| SpellValueError::InvalidCatalogInputs)?;
                }
            } else if matches!(effect.scaling_class, -10..=-1 | 1..=15) {
                let tables = tables.ok_or(SpellValueError::MissingGameTables)?;
                let row = tables
                    .scaling(level)
                    .ok_or(SpellValueError::MissingScalingRow)?;
                value = row.for_class(effect.scaling_class);
            }
            // GameTables.h's default selector returns zero without touching
            // its row pointer; an unknown selector need not dereference a
            // missing row. This is source zero, not missing-input zero fill.
            // These LookupEntry/GetRow checks are genuinely optional on the
            // admitted stores. Query ID 0; do not assume it is always absent.
            // Missing encrypted ItemSparse records remain unknown, not proof
            // that the full native client has no such record.
            let multiplier = match effect.scaling_class {
                -7 => tables.ok_or(SpellValueError::MissingGameTables)?.ratings(1),
                -6 => tables.ok_or(SpellValueError::MissingGameTables)?.stamina(1),
                _ => None,
            };
            if let Some(multiplier) = multiplier {
                if let Some(item) = items.sparse(0) {
                    value *= multiplier.for_inventory_type(item.inventory_type as u8);
                }
            }
        }
        value *= effect.scaling_coefficient;
        if value > 0.0 && value < 1.0 {
            value = 1.0;
        }
        if !float_amounts {
            value = value.round();
        }
        Ok(f64::from(value))
    } else {
        let mut value = effect.base_points;
        let mut stat = effect.scaling_expected_stat();
        if stat != ExpectedStatType::None {
            if fields.attributes[0] & SCALES_CREATURE_LEVEL != 0 {
                stat = ExpectedStatType::CreatureAutoAttackDps;
            }
            let expansion = catalog
                .content_tuning_expansion(fields.content_tuning_id)
                .map_err(|_| SpellValueError::InvalidCatalogInputs)?;
            // Source deliberately passes tuning/class/season 0, not the
            // SpellInfo ContentTuningId used to select expansion above.
            value = catalog
                .evaluate_expected_stat(stat, 1, expansion, 0, 0, 0)
                .map_err(|_| SpellValueError::InvalidCatalogInputs)?
                * value
                / 100.0f32;
            if !float_amounts {
                value = value.round();
            }
        }
        // No round at all for an unscaled, non-expected-stat base.
        Ok(f64::from(value))
    }
}

fn finish_value(
    effect: &SpellEffectValues,
    base: f64,
    override_points: Option<f64>,
    draw: &mut impl FnMut(f32, f32) -> Result<f32, SpellValueError>,
) -> Result<StartupSpellValue, SpellValueError> {
    let mut value = override_points.unwrap_or(base);
    let mut variance = None;
    if effect.scaling_variance != 0.0 {
        // Multiplication/narrowing is f32 before promotion into the f64 value.
        let delta = (effect.scaling_variance * 0.5f32).abs();
        if !delta.is_finite() {
            return Err(SpellValueError::InvalidVarianceRange);
        }
        let sample = draw(-delta, delta)?;
        if !sample.is_finite() || sample < -delta || sample > delta {
            return Err(SpellValueError::InvalidVarianceDraw);
        }
        value += base * f64::from(sample);
        variance = Some(sample);
    }
    // Per-level, combo, traits, mastery and ApplyEffectModifiers require a
    // non-null caster and therefore do not run in this startup operation.
    if effect.rounds_value() {
        value = value.round();
    }
    Ok(StartupSpellValue {
        value: value.clamp(MIN_VALUE, MAX_VALUE),
        variance,
    })
}

impl SpellEffectValues {
    pub(super) fn scaling_expected_stat(&self) -> ExpectedStatType {
        use ExpectedStatType::*;
        match self.effect {
            2 | 7 | 9 | 17 | 58 => CreatureSpellDamage,
            10 | 75 => PlayerHealth,
            30 | 62 if self.misc_values[0] == 0 => PlayerMana,
            8 => PlayerMana,
            6 | 27 | 35 | 65 | 119 | 128 | 129 | 143 | 174 | 202 | 271 => match self.aura {
                3 | 13 | 15 | 43 | 53 | 59 | 62 | 102 | 131 | 180 => CreatureSpellDamage,
                8 | 14 | 34 | 69 | 84 | 97 | 115 | 135 | 161 | 230 | 250 | 301 => PlayerHealth,
                64 => PlayerMana,
                29 | 99 | 124 => PlayerPrimaryStat,
                189 => PlayerSecondaryStat,
                22 | 83 | 123 | 465 => ArmorConstant,
                24 | 35 | 73 | 85 | 162 | 418 if self.misc_values[0] == 0 => PlayerMana,
                _ => None,
            },
            _ => None,
        }
    }
    fn rounds_value(&self) -> bool {
        match self.effect {
            2 | 7 | 9 | 10 | 17 | 31 | 58 | 67 | 75 | 121 | 8 | 30 | 62 => true,
            // Party nonrandom (271) is deliberately NOT in CalcValue's list,
            // even though GetScalingExpectedStat includes it above.
            6 | 27 | 35 | 65 | 119 | 128 | 129 | 143 | 174 | 202 => matches!(
                self.aura,
                3 | 8 | 53 | 62 | 70 | 15 | 43 | 20 | 21 | 24 | 64 | 89 | 162
            ),
            _ => false,
        }
    }
}
