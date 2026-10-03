//! Owned client + server definitions with explicit intermediate ID corrections.
//! One derived authority consumes the ID-only plan. Global startup correction
//! requires exclusive raw ownership; later readers share an immutable Arc.
mod abilities;
#[cfg(test)]
mod cast_fixture;
mod client;
mod corrections;
mod custom;
mod custom_sql;
mod diminishing;
mod global;
mod immunities;
mod learn_skills;
mod learn_spells;
#[cfg(test)]
mod learning_fixtures;
#[cfg(test)]
mod skill_set_fixture;
#[cfg(test)]
mod spell_book_fixture;
#[cfg(test)]
pub(crate) use cast_fixture::cast_test_definitions;
#[cfg(test)]
pub(crate) use learn_spells::spell_book_test_learn_flags;
#[cfg(test)]
pub(crate) use skill_set_fixture::{skill_set_test_definitions, skill_set_test_spell_fields};
#[cfg(test)]
pub(crate) use spell_book_fixture::spell_book_test_definitions;
mod positivity;
mod ranks;
mod required;
mod server;
mod specific;
mod target_caps;
mod target_masks;
#[cfg(test)]
mod tests;
mod traversal;
mod validity;
mod value_tables;
mod values;
mod view;
use super::{
    EffectTargetInfo, ImplicitTargetInfo, Key, POWER_SLOTS, SpellConstructorFields,
    SpellLoadCounts, SpellLoadPlan,
};
pub use abilities::SkillLineAbilityCounts;
pub use corrections::IdCorrectionCounts;
pub use custom::{CustomAttributeCounts, SpellCustomAttributeError};
pub use custom_sql::SqlCustomAttributeCounts;
pub use diminishing::{
    DiminishingCounts, DiminishingGroup, DiminishingInfo, DiminishingLevel, DiminishingType,
    SpellDiminishingError,
};
pub use global::{GlobalCorrectionCounts, SpellTraversal};
pub use immunities::{
    CreatureImmunityInfo, EffectImmunityInfo, ImmunityCounts, SpellImmunityError,
};
pub use learn_skills::{LearnSkillCounts, SpellLearnSkillError, SpellLearnSkillNode};
pub use learn_spells::{LearnSpellCounts, SpellLearnError, SpellLearnNode};
pub use ranks::{SpellRankCounts, SpellRankError, SpellRankNode};
pub use required::{RequiredSpellCounts, SpellRequiredError};
pub use specific::{SpecificCounts, SpellAuraState, SpellSpecific, SpellSpecificError};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
pub use target_caps::{SpellTargetCapError, SqrtTargetLimit, TargetCapCounts};
pub use target_masks::ExplicitTargetMasks;
pub use traversal::SpellTraversalInputs;
pub use validity::SpellValidityError;
pub use values::{SpellValueError, StartupSpellValue};
pub use view::{SpellDefinitionView, SpellEffectView};
use wow_data::forever_spells::SpellCatalog;
use wow_persistence::forever::spells::server::ServerSpellRows;

#[derive(Default)]
pub struct SpellEffectValues {
    pub index: u32,
    pub effect: u32,
    pub aura: u32,
    pub aura_period: u32,
    pub base_points: f32,
    pub real_points_per_level: f32,
    pub points_per_resource: f32,
    pub amplitude: f32,
    pub chain_amplitude: f32,
    pub bonus_coefficient: f32,
    pub misc_values: [i32; 2],
    pub mechanic: u32,
    pub position_facing: f32,
    pub implicit_targets: [u32; 2],
    pub chain_targets: i32,
    pub item_type: u32,
    pub trigger_spell: u32,
    pub class_mask: [u32; 4],
    pub bonus_coefficient_from_ap: f32,
    pub scaling_class: i32,
    pub scaling_coefficient: f32,
    pub scaling_variance: f32,
    pub scaling_resource_coefficient: f32,
    pub attributes: u32,
    // Checked dependencies, not the raw unvalidated RadiusIndex words.
    pub radius_ids: [Option<u32>; 2],
}

// One canonical effect query implementation for correction and readonly views.
// SpellInfo.cpp:465-497. These methods do not execute an effect or own a Player.
impl SpellEffectValues {
    fn effect_target_info(&self) -> EffectTargetInfo {
        EffectTargetInfo::from_id(self.effect).expect("validated Forever effect kind")
    }
    fn target_metadata(&self) -> [ImplicitTargetInfo; 2] {
        self.implicit_targets
            .map(|id| ImplicitTargetInfo::from_id(id).expect("validated Forever implicit target"))
    }
    fn is_targeting_area(&self) -> bool {
        self.target_metadata().iter().any(|target| target.is_area())
    }
    fn is_area_aura_effect(&self) -> bool {
        matches!(self.effect, 35 | 65 | 119 | 128 | 129 | 143 | 202 | 271)
    }
    fn is_unit_owned_aura_effect(&self) -> bool {
        self.is_area_aura_effect() || matches!(self.effect, 6 | 174)
    }
    fn is_aura(&self) -> bool {
        (self.is_unit_owned_aura_effect() || self.effect == 27) && self.aura != 0
    }
    fn is_aura_kind(&self, aura: u32) -> bool {
        self.is_aura() && self.aura == aura
    }
}

enum NameSource {
    Client(u32),
    Server(Vec<u8>),
}

// Private to the one spells owner. No independent correction/Player mirror.
pub(super) struct Definition {
    name: NameSource,
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
    custom_attributes: u32,
    negative_effects: [bool; super::EFFECT_SLOTS],
    explicit_target_masks: ExplicitTargetMasks,
    diminishing: DiminishingInfo,
    immunity: immunities::ImmunityState,
    target_limit: SqrtTargetLimit,
    specific: SpellSpecific,
    aura_state: SpellAuraState,
}

// Canonical collection indexes after source-container admission. Only IDs,
// not mutable record mirrors or retained native/helper replay containers.
struct DefinitionOrder {
    primary: Vec<Key>,
    by_spell: BTreeMap<u32, Vec<Key>>,
}

impl Definition {
    // SpellInfo.cpp:1557-1563 / :460-463: kind equality, no magnitude,
    // aura or first-effect restriction. Includes existing blank gap slots.
    fn has_effect(&self, effect: u32) -> bool {
        self.effects.iter().any(|values| values.effect == effect)
    }

    fn empty_server(name: Vec<u8>) -> Self {
        Self {
            name: NameSource::Server(name),
            fields: SpellConstructorFields {
                required_areas_id: -1,
                equipped_item_class: -1,
                ..Default::default()
            },
            effects: Vec::new(),
            cast_time: None,
            duration: None,
            range: None,
            ppm_modifiers: Vec::new(),
            powers: [None; POWER_SLOTS],
            reagent_currencies: Vec::new(),
            visuals: Vec::new(),
            labels: BTreeSet::new(),
            empower_thresholds_ms: Vec::new(),
            custom_attributes: 0,
            negative_effects: [false; super::EFFECT_SLOTS],
            explicit_target_masks: ExplicitTargetMasks::default(),
            diminishing: DiminishingInfo::default(),
            immunity: immunities::ImmunityState::default(),
            target_limit: SqrtTargetLimit::default(),
            specific: SpellSpecific::default(),
            aura_state: SpellAuraState::default(),
        }
    }
}

/// Startup admission errors for source-undefined negative indexing, or a
/// lookup chain that cannot terminate, or repeated one-shot corrections.
/// No SQL/record values are disclosed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellDefinitionError {
    EffectIndex,
    NegativeAura,
    NegativeImplicitTarget,
    DifficultyCycle,
    IdCorrectionsAlreadyApplied,
    GlobalCorrectionsRequireIdCorrections,
    GlobalCorrectionsAlreadyApplied,
    InvalidCorrectionTraversal,
    SharedCorrectionCatalog,
    SkillLineAbilitiesRequireGlobalCorrections,
    SkillLineAbilitiesAlreadyLoaded,
    SqlCustomAttributesRequireSkillLineAbilities,
    SqlCustomAttributesAlreadyApplied,
    SpellValueTablesAlreadyLoaded,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ServerSpellCounts {
    pub input_spells: usize,
    pub input_effects: usize,
    pub definitions_added: usize,
    pub duplicate_spell_rows: usize,
    pub rejected_client_names: usize,
    pub skipped_regular_effects: usize,
    pub skipped_missing_difficulty_effects: usize,
    pub skipped_invalid_effects: usize,
    pub missing_radius_warnings: usize,
    pub orphan_effect_groups: usize,
}

/// Startup seed authority; optional ID/global corrections are explicitly counted.
/// NOT fully corrected/executable SpellInfo, learning or Player spell state.
/// Server spell names are internal and never inserted into the DB2 hotfix store.
pub struct SpellDefinitionSeeds {
    catalog: Arc<SpellCatalog>,
    definitions: BTreeMap<Key, Definition>,
    languages: BTreeMap<u32, Vec<u32>>,
    battle_pets_by_spell: BTreeMap<u32, u32>,
    client_counts: SpellLoadCounts,
    server_counts: ServerSpellCounts,
    id_corrections: Option<IdCorrectionCounts>,
    global_corrections: Option<GlobalCorrectionCounts>,
    traversal_inputs: Option<traversal::OwnedInputs>,
    source_order: Option<DefinitionOrder>,
    skill_line_abilities: Option<abilities::SkillLineAbilityIndex>,
    sql_custom_attributes: Option<SqlCustomAttributeCounts>,
    custom_attributes: Option<CustomAttributeCounts>,
    diminishing: Option<DiminishingCounts>,
    immunities: Option<immunities::LoadedImmunities>,
    target_caps: Option<TargetCapCounts>,
    ranks: Option<ranks::LoadedRanks>,
    required: Option<required::LoadedRequired>,
    learn_skills: Option<learn_skills::LoadedLearnSkills>,
    specific: Option<SpecificCounts>,
    learn_spells: Option<learn_spells::LoadedLearnSpells>,
    value_game_tables: Option<Arc<wow_data::forever_game_tables::SpellValueGameTables>>,
}

impl SpellLoadPlan {
    pub fn with_server_spells(
        self,
        rows: ServerSpellRows,
    ) -> Result<SpellDefinitionSeeds, SpellDefinitionError> {
        let mut definitions = BTreeMap::new();
        for inputs in self.records() {
            let key = (inputs.spell_id(), inputs.difficulty());
            definitions.insert(key, inputs.constructor_seed().into_owned_definition());
        }
        let mut server_requests = Vec::new();
        let counts = server::load(&self.catalog, &mut definitions, rows, &mut server_requests)?;
        // ID-only load helpers/PPM index retire here; definitions now own the
        // derived constructor state. Raw catalog payloads are not cloned.
        Ok(SpellDefinitionSeeds {
            catalog: self.catalog,
            definitions,
            languages: self.languages,
            battle_pets_by_spell: self.battle_pets_by_spell,
            client_counts: self.counts,
            server_counts: counts,
            id_corrections: None,
            global_corrections: None,
            source_order: None,
            skill_line_abilities: None,
            sql_custom_attributes: None,
            custom_attributes: None,
            diminishing: None,
            immunities: None,
            target_caps: None,
            ranks: None,
            required: None,
            learn_skills: None,
            specific: None,
            learn_spells: None,
            value_game_tables: None,
            traversal_inputs: Some(traversal::OwnedInputs::new(
                self.helper_insertions,
                server_requests,
            )),
        })
    }
}

impl SpellDefinitionSeeds {
    /// Share final/raw inputs only after the caller's startup phases finish.
    /// Cloning now prevents a later exclusive global-correction transition.
    pub fn raw_catalog(&self) -> Arc<SpellCatalog> {
        self.catalog.clone()
    }
    pub fn client_counts(&self) -> SpellLoadCounts {
        self.client_counts
    }
    pub fn server_counts(&self) -> ServerSpellCounts {
        self.server_counts
    }
    pub fn len(&self) -> usize {
        self.definitions.len()
    }
    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }
    pub fn records(&self) -> impl Iterator<Item = SpellDefinitionView<'_>> {
        let mut ordered = self.source_order.as_ref().map(|order| order.primary.iter());
        let mut sorted = self.definitions.iter();
        std::iter::from_fn(move || {
            let (&key, definition) = if let Some(keys) = &mut ordered {
                let key = keys.next()?;
                (key, &self.definitions[key])
            } else {
                sorted.next()?
            };
            Some(SpellDefinitionView {
                catalog: &self.catalog,
                definition,
                key,
            })
        })
    }
    /// None before global/source-index admission; Some(empty) for an absent
    /// spell afterwards. Existing difficulties retain source equal_range order,
    /// not a guessed sorted-difficulty or manufactured fallback ordering.
    pub fn corrected_difficulties(
        &self,
        spell: u32,
    ) -> Option<impl Iterator<Item = SpellDefinitionView<'_>>> {
        let order = self.source_order.as_ref()?;
        Some(
            order
                .by_spell
                .get(&spell)
                .into_iter()
                .flatten()
                .map(move |&key| SpellDefinitionView {
                    catalog: &self.catalog,
                    definition: &self.definitions[&key],
                    key,
                }),
        )
    }
    pub fn get_exact(&self, spell: u32, difficulty: i16) -> Option<SpellDefinitionView<'_>> {
        self.definitions
            .get(&(spell, difficulty))
            .map(|definition| SpellDefinitionView {
                catalog: &self.catalog,
                definition,
                key: (spell, difficulty),
            })
    }
    /// SpellMgr.cpp:692-710: exact key first, then the first present fallback.
    /// No manufactured difficulty or implicit fallback to DIFFICULTY_NONE.
    pub fn get(
        &self,
        spell: u32,
        difficulty: i16,
    ) -> Result<Option<SpellDefinitionView<'_>>, SpellDefinitionError> {
        if let Some(definition) = self.get_exact(spell, difficulty) {
            return Ok(Some(definition));
        }
        let mut seen = BTreeSet::from([difficulty]);
        let mut current = difficulty;
        while let Some(row) = self.catalog.difficulty(current as i32 as u32) {
            current = row.fallback_difficulty_id;
            if let Some(definition) = self.get_exact(spell, current) {
                return Ok(Some(definition));
            }
            if !seen.insert(current) {
                return Err(SpellDefinitionError::DifficultyCycle);
            }
        }
        Ok(None)
    }
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
