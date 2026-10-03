//! Target SpellMgr::LoadSpellInfoStore join (02245dcd:2496-2735).
//! This is a resolved input plan, NOT finished SpellInfo, a spellbook or Player.
//! Raw records remain in the one immutable SpellCatalog used by hotfix delivery.
mod constructor;
mod definitions;
mod effect_targets;
mod inputs;
mod join;
mod targets;
#[cfg(test)]
pub(crate) use definitions::cast_test_definitions;
#[cfg(test)]
pub(crate) use definitions::{skill_set_test_definitions, skill_set_test_spell_fields};
#[cfg(test)]
pub(crate) use definitions::{spell_book_test_definitions, spell_book_test_learn_flags};
#[cfg(test)]
mod tests;
use std::{collections::BTreeMap, sync::Arc};
use wow_data::forever_spells::SpellCatalog;

pub use constructor::{SpellConstructorFields, SpellConstructorSeed, SpellEffectSeed};
pub use definitions::{
    CreatureImmunityInfo, CustomAttributeCounts, DiminishingCounts, DiminishingGroup,
    DiminishingInfo, DiminishingLevel, DiminishingType, EffectImmunityInfo, ExplicitTargetMasks,
    GlobalCorrectionCounts, IdCorrectionCounts, ImmunityCounts, LearnSkillCounts, LearnSpellCounts,
    RequiredSpellCounts, ServerSpellCounts, SkillLineAbilityCounts, SpecificCounts, SpellAuraState,
    SpellCustomAttributeError, SpellDefinitionError, SpellDefinitionSeeds, SpellDefinitionView,
    SpellDiminishingError, SpellEffectValues, SpellEffectView, SpellImmunityError, SpellLearnError,
    SpellLearnNode, SpellLearnSkillError, SpellLearnSkillNode, SpellRankCounts, SpellRankError,
    SpellRankNode, SpellRequiredError, SpellSpecific, SpellSpecificError, SpellTargetCapError,
    SpellTraversal, SpellTraversalInputs, SpellValidityError, SpellValueError,
    SqlCustomAttributeCounts, SqrtTargetLimit, StartupSpellValue, TargetCapCounts,
};
pub use effect_targets::{EffectTargetInfo, EffectTargetType};
pub use inputs::SpellInputs;
pub use targets::{
    ImplicitTargetInfo, TargetCheck, TargetDirection, TargetObject, TargetReference,
    TargetSelection,
};

pub const EFFECT_SLOTS: usize = 32;
pub const POWER_SLOTS: usize = 5;
type Key = (u32, i16);

#[derive(Clone, Default)]
struct Helper {
    aura_options: Option<u32>,
    aura_restrictions: Option<u32>,
    casting_requirements: Option<u32>,
    categories: Option<u32>,
    class_options: Option<u32>,
    cooldowns: Option<u32>,
    equipped_items: Option<u32>,
    interrupts: Option<u32>,
    levels: Option<u32>,
    misc: Option<u32>,
    reagents: Option<u32>,
    scaling: Option<u32>,
    shapeshift: Option<u32>,
    target_restrictions: Option<u32>,
    totems: Option<u32>,
    effects: [Option<u32>; EFFECT_SLOTS],
    powers: [Option<u32>; POWER_SLOTS],
    empower_stages: Vec<u32>,
    labels: Vec<u32>,
    reagent_currencies: Vec<u32>,
    visuals: Vec<u32>,
}

impl Helper {
    fn fill_missing(&mut self, fallback: &Self) {
        self.aura_options = self.aura_options.or(fallback.aura_options);
        self.aura_restrictions = self.aura_restrictions.or(fallback.aura_restrictions);
        self.casting_requirements = self.casting_requirements.or(fallback.casting_requirements);
        self.categories = self.categories.or(fallback.categories);
        self.class_options = self.class_options.or(fallback.class_options);
        self.cooldowns = self.cooldowns.or(fallback.cooldowns);
        self.equipped_items = self.equipped_items.or(fallback.equipped_items);
        self.interrupts = self.interrupts.or(fallback.interrupts);
        self.levels = self.levels.or(fallback.levels);
        self.misc = self.misc.or(fallback.misc);
        self.reagents = self.reagents.or(fallback.reagents);
        self.scaling = self.scaling.or(fallback.scaling);
        self.shapeshift = self.shapeshift.or(fallback.shapeshift);
        self.target_restrictions = self.target_restrictions.or(fallback.target_restrictions);
        self.totems = self.totems.or(fallback.totems);
        for (own, other) in self.effects.iter_mut().zip(&fallback.effects) {
            *own = own.or(*other);
        }
        for (own, other) in self.powers.iter_mut().zip(&fallback.powers) {
            *own = own.or(*other);
        }
        // Entire vectors use the first nonempty fallback, never concatenation.
        if self.empower_stages.is_empty() {
            self.empower_stages.clone_from(&fallback.empower_stages);
        }
        if self.labels.is_empty() {
            self.labels.clone_from(&fallback.labels);
        }
        if self.reagent_currencies.is_empty() {
            self.reagent_currencies
                .clone_from(&fallback.reagent_currencies);
        }
        if self.visuals.is_empty() {
            self.visuals.clone_from(&fallback.visuals);
        }
    }
}

/// Bounds/cycles that the reference asserts or indexes unchecked. No record
/// values/IDs are included in errors or startup diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellLoadError {
    EffectIndex,
    PowerIndex,
    NegativeImplicitTarget,
    DifficultyCycle,
}

/// Counts only. Unknown enum effects are skipped as in the Classic guard, not
/// normalized into known effects; encrypted baseline rows remain unknown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpellLoadCounts {
    pub spells_and_difficulties: usize,
    pub unnamed_helpers: usize,
    pub skipped_effects: usize,
    pub unknown_effect_values: usize,
    pub unknown_aura_values: usize,
    pub unknown_target_values: usize,
    pub invalid_modifier_types: usize,
    pub language_registrations: usize,
    pub battle_pet_spell_associations: usize,
}

/// Startup-only semantic join owner. IDs select immutable records, not cloned
/// DB2 payloads. No SQL, locks, Session or mutable Player state is retained.
pub struct SpellLoadPlan {
    catalog: Arc<SpellCatalog>,
    helpers: BTreeMap<Key, Helper>,
    helper_insertions: Vec<Key>,
    ppm_modifiers_by_rate: BTreeMap<u32, Vec<u32>>,
    // Pre-skill language registrations only; LanguageMgr::LoadLanguages is later.
    languages: BTreeMap<u32, Vec<u32>>,
    battle_pets_by_spell: BTreeMap<u32, u32>,
    counts: SpellLoadCounts,
}

impl SpellLoadPlan {
    /// Source order is ascending effective DB2 storage ID, not SQL row order.
    /// Publish only after every join and named difficulty chain succeeds.
    pub fn build(catalog: Arc<SpellCatalog>) -> Result<Self, SpellLoadError> {
        join::build(catalog)
    }

    pub fn counts(&self) -> SpellLoadCounts {
        self.counts
    }

    pub fn get(&self, spell: u32, difficulty: i16) -> Option<SpellInputs<'_>> {
        self.helpers
            .get(&(spell, difficulty))
            .map(|helper| SpellInputs {
                catalog: &self.catalog,
                ppm_modifiers_by_rate: &self.ppm_modifiers_by_rate,
                helper,
                spell,
                difficulty,
            })
    }

    pub fn records(&self) -> impl Iterator<Item = SpellInputs<'_>> {
        self.helpers
            .iter()
            .map(|(&(spell, difficulty), helper)| SpellInputs {
                catalog: &self.catalog,
                ppm_modifiers_by_rate: &self.ppm_modifiers_by_rate,
                helper,
                spell,
                difficulty,
            })
    }

    /// Source LanguageMgr::LoadSpellEffectLanguage emits one registration per
    /// accepted effect, even a later-overwritten or unnamed helper's effect.
    /// No skill ID or complete language support is implied by this precursor.
    pub fn language_spell_registrations(&self, language: u32) -> impl Iterator<Item = u32> + '_ {
        self.languages.get(&language).into_iter().flatten().copied()
    }

    pub fn battle_pet_species_for_spell(
        &self,
        spell: u32,
    ) -> Option<&wow_data::forever_spells::BattlePetSpeciesRecord> {
        self.battle_pets_by_spell
            .get(&spell)
            .and_then(|&id| self.catalog.battle_pet_species(id))
    }
}
