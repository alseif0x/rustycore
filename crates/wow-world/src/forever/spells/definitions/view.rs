//! Borrowed immutable semantic definitions; no mutable fields or Player state.
use super::super::{EffectTargetInfo, ImplicitTargetInfo};
use super::{Definition, Key, NameSource, POWER_SLOTS, SpellConstructorFields, SpellEffectValues};
use std::collections::BTreeSet;
use wow_data::forever_spells::*;
#[cfg(test)]
mod tests;

pub struct SpellDefinitionView<'a> {
    pub(super) catalog: &'a SpellCatalog,
    pub(super) definition: &'a Definition,
    pub(super) key: Key,
}
impl<'a> SpellDefinitionView<'a> {
    /// Admit a borrowed semantic view from this exact definition authority.
    /// Equal IDs or a shared raw DB2 catalog do not establish derived identity.
    pub(crate) fn belongs_to(&self, owner: &super::SpellDefinitionSeeds) -> bool {
        owner.definitions.get(&self.key).is_some_and(|definition| {
            std::ptr::eq(definition, self.definition)
                && std::ptr::eq(owner.catalog.as_ref(), self.catalog)
        })
    }
    pub fn spell_specific(&self) -> super::SpellSpecific {
        self.definition.specific
    }
    pub fn aura_state(&self) -> super::SpellAuraState {
        self.definition.aura_state
    }
    pub fn spell_id(&self) -> u32 {
        self.key.0
    }
    pub fn difficulty(&self) -> i16 {
        self.key.1
    }
    pub fn is_server_defined(&self) -> bool {
        matches!(self.definition.name, NameSource::Server(_))
    }
    pub fn fields(&self) -> &'a SpellConstructorFields {
        &self.definition.fields
    }
    /// Server-only correction/custom flags, not a raw DB2 attribute word.
    pub fn custom_attributes(&self) -> u32 {
        self.definition.custom_attributes
    }
    pub fn negative_effects(&self) -> &'a [bool; super::super::EFFECT_SLOTS] {
        &self.definition.negative_effects
    }
    /// Source retained masks; constructor zeros before the custom phase,
    /// then the once-initialized result. Not a fresh derivation/ready marker.
    pub fn explicit_target_masks(&self) -> super::ExplicitTargetMasks {
        self.definition.explicit_target_masks
    }
    /// Source constructor defaults until the once-only diminishing phase.
    pub fn diminishing_info(&self) -> super::DiminishingInfo {
        self.definition.diminishing
    }
    pub fn allowed_mechanic_mask(&self) -> u64 {
        self.definition.immunity.allowed_mechanics
    }
    pub fn sqrt_target_limit(&self) -> super::SqrtTargetLimit {
        self.definition.target_limit
    }
    /// C++ server name pointers alias the same string in every locale slot.
    /// Client missing locales remain missing. This is not a locale fallback.
    pub fn name_bytes(&self, locale: u8) -> Option<&'a [u8]> {
        let bytes = match &self.definition.name {
            NameSource::Client(id) => self
                .catalog
                .spell_name(*id)
                .expect("definition name")
                .name
                .at(locale)?,
            NameSource::Server(bytes) => {
                if locale >= 12 {
                    return None;
                }
                bytes.as_slice()
            }
        };
        Some(
            bytes
                .split(|&byte| byte == 0)
                .next()
                .expect("C-string prefix"),
        )
    }
    pub fn effect_count(&self) -> usize {
        self.definition.effects.len()
    }
    pub fn effect(&self, index: usize) -> Option<SpellEffectView<'a>> {
        self.definition
            .effects
            .get(index)
            .map(|values| SpellEffectView {
                catalog: self.catalog,
                values,
                immunity: self.definition.immunity.effects.get(&index),
            })
    }
    pub fn effects(&self) -> impl Iterator<Item = SpellEffectView<'a>> + 'a {
        let catalog = self.catalog;
        let immunity = &self.definition.immunity;
        self.definition
            .effects
            .iter()
            .enumerate()
            .map(move |(index, values)| SpellEffectView {
                catalog,
                values,
                immunity: immunity.effects.get(&index),
            })
    }
    pub fn has_effect(&self, effect: u32) -> bool {
        self.definition.has_effect(effect)
    }
    /// SpellInfo.cpp:1647-1650, including physical gap slots' kind values.
    pub fn is_loot_crafting(&self) -> bool {
        self.has_effect(59) || self.has_effect(157)
    }
    pub fn cast_time(&self) -> Option<&'a SpellCastTimesRecord> {
        self.definition
            .cast_time
            .and_then(|id| self.catalog.spell_cast_times(id))
    }
    pub fn duration(&self) -> Option<&'a SpellDurationRecord> {
        self.definition
            .duration
            .and_then(|id| self.catalog.spell_duration(id))
    }
    pub fn range(&self) -> Option<&'a SpellRangeRecord> {
        self.definition
            .range
            .and_then(|id| self.catalog.spell_range(id))
    }
    pub fn powers(&self) -> [Option<&'a SpellPowerRecord>; POWER_SLOTS] {
        self.definition
            .powers
            .map(|id| id.and_then(|id| self.catalog.spell_power(id)))
    }
    pub fn ppm_modifiers(&self) -> impl Iterator<Item = &'a SpellProcsPerMinuteModRecord> + 'a {
        let catalog = self.catalog;
        self.definition.ppm_modifiers.iter().map(move |&id| {
            catalog
                .spell_procs_per_minute_mod(id)
                .expect("definition PPM modifier")
        })
    }
    pub fn reagent_currencies(&self) -> impl Iterator<Item = &'a SpellReagentsCurrencyRecord> + 'a {
        let catalog = self.catalog;
        self.definition.reagent_currencies.iter().map(move |&id| {
            catalog
                .spell_reagents_currency(id)
                .expect("definition reagent currency")
        })
    }
    pub fn visuals(&self) -> impl Iterator<Item = &'a SpellXSpellVisualRecord> + 'a {
        let catalog = self.catalog;
        self.definition
            .visuals
            .iter()
            .map(move |&id| catalog.spell_x_spell_visual(id).expect("definition visual"))
    }
    pub fn labels(&self) -> &'a BTreeSet<u32> {
        &self.definition.labels
    }
    pub fn empower_thresholds_ms(&self) -> &'a [i64] {
        &self.definition.empower_thresholds_ms
    }
}

pub struct SpellEffectView<'a> {
    catalog: &'a SpellCatalog,
    values: &'a SpellEffectValues,
    immunity: Option<&'a super::EffectImmunityInfo>,
}
impl<'a> SpellEffectView<'a> {
    pub fn immunity_info(&self) -> Option<&'a super::EffectImmunityInfo> {
        self.immunity
    }
    pub fn values(&self) -> &'a SpellEffectValues {
        self.values
    }
    pub fn scaling_expected_stat(&self) -> ExpectedStatType {
        self.values.scaling_expected_stat()
    }
    pub fn target_info(&self) -> EffectTargetInfo {
        self.values.effect_target_info()
    }
    pub fn provided_target_mask(&self) -> u32 {
        self.implicit_targets()
            .iter()
            .fold(0, |mask, target| mask | target.object_type().flag_mask())
    }
    pub fn missing_target_mask(&self, source_set: bool, destination_set: bool, mask: u32) -> u32 {
        self.target_info().missing_target_mask(
            source_set,
            destination_set,
            self.provided_target_mask() | mask,
        )
    }
    /// Constructor admission and pinned corrections retain IDs 0..152.
    /// A blank effect uses the exact target-zero metadata, not a guessed unit.
    pub fn implicit_targets(&self) -> [ImplicitTargetInfo; 2] {
        self.values.target_metadata()
    }
    pub fn is_effect(&self) -> bool {
        self.values.effect != 0
    }
    pub fn is_effect_kind(&self, effect: u32) -> bool {
        self.values.effect == effect
    }
    /// SpellInfo.cpp:475-491. LINE/TRAJ/NYI do not become area targets.
    pub fn is_targeting_area(&self) -> bool {
        self.values.is_targeting_area()
    }
    pub fn is_area_aura_effect(&self) -> bool {
        self.values.is_area_aura_effect()
    }
    pub fn is_unit_owned_aura_effect(&self) -> bool {
        self.values.is_unit_owned_aura_effect()
    }
    /// SpellInfo.cpp:465-472: nonzero aura data alone does not imply an aura.
    pub fn is_aura(&self) -> bool {
        self.values.is_aura()
    }
    pub fn is_aura_kind(&self, aura: u32) -> bool {
        self.values.is_aura_kind(aura)
    }
    pub fn radii(&self) -> [Option<&'a SpellRadiusRecord>; 2] {
        self.values
            .radius_ids
            .map(|id| id.and_then(|id| self.catalog.spell_radius(id)))
    }
}
