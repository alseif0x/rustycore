//! Consume constructor values once; links remain IDs into the canonical Arc.
use super::{Definition, NameSource, POWER_SLOTS, SpellConstructorFields, SpellEffectValues};
use std::collections::BTreeSet;
impl Definition {
    pub(in crate::forever::spells) fn from_constructor_parts(
        spell: u32,
        fields: SpellConstructorFields,
        effects: Vec<SpellEffectValues>,
        cast_time: Option<u32>,
        duration: Option<u32>,
        range: Option<u32>,
        ppm_modifiers: Vec<u32>,
        powers: [Option<u32>; POWER_SLOTS],
        reagent_currencies: Vec<u32>,
        visuals: Vec<u32>,
        labels: BTreeSet<u32>,
        empower_thresholds_ms: Vec<i64>,
    ) -> Self {
        Self {
            name: NameSource::Client(spell),
            fields,
            effects,
            cast_time,
            duration,
            range,
            ppm_modifiers,
            powers,
            reagent_currencies,
            visuals,
            labels,
            empower_thresholds_ms,
            custom_attributes: 0,
            negative_effects: [false; super::super::EFFECT_SLOTS],
            explicit_target_masks: super::ExplicitTargetMasks::default(),
            diminishing: super::DiminishingInfo::default(),
            immunity: super::immunities::ImmunityState::default(),
            target_limit: super::SqrtTargetLimit::default(),
            specific: super::SpellSpecific::default(),
            aura_state: super::SpellAuraState::default(),
        }
    }
}
