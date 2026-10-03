//! Exact target storage joins, guarded array admission and difficulty fallback.
use super::{
    EFFECT_SLOTS, Helper, Key, POWER_SLOTS, SpellLoadCounts, SpellLoadError, SpellLoadPlan,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use wow_data::forever_spells::SpellCatalog;

// 02245dcd SharedDefines / SpellAuraDefines. These are reference-supported enum
// ceilings, NOT evidence that every admitted effect has an executable Rust path.
const TOTAL_EFFECTS: u32 = 361;
const TOTAL_AURAS: i16 = 665;
const TOTAL_TARGETS: i16 = 153;
const SUMMON: u32 = 28;
const LANGUAGE: u32 = 39;
const MINIPET: i32 = 5;
const SUMMON_FROM_JOURNAL: u64 = 0x0020_0000;

pub(super) fn build(catalog: Arc<SpellCatalog>) -> Result<SpellLoadPlan, SpellLoadError> {
    let mut helpers = HelperInputs::default();
    let mut counts = SpellLoadCounts::default();
    let mut languages = BTreeMap::<u32, Vec<u32>>::new();
    let mut battle_pets_by_spell = BTreeMap::new();
    let mut species_by_creature = BTreeMap::new();
    for row in catalog.battle_pet_species_records() {
        if row.creature_id != 0 {
            species_by_creature.insert(row.creature_id, row.id);
        }
    }
    for effect in catalog.spell_effect_records() {
        let index = usize::try_from(effect.effect_index)
            .ok()
            .filter(|&index| index < EFFECT_SLOTS)
            .ok_or(SpellLoadError::EffectIndex)?;
        let unknown_effect = effect.effect >= TOTAL_EFFECTS;
        let unknown_aura = effect.effect_aura < 0 || effect.effect_aura >= TOTAL_AURAS;
        let unknown_targets = effect
            .implicit_target
            .iter()
            .filter(|&&target| target >= TOTAL_TARGETS)
            .count();
        if unknown_effect || unknown_aura || unknown_targets != 0 {
            counts.skipped_effects += 1;
            counts.unknown_effect_values += usize::from(unknown_effect);
            counts.unknown_aura_values += usize::from(unknown_aura);
            counts.unknown_target_values += unknown_targets;
            continue;
        }
        // C++ does not reject negative targets before later table indexing.
        // Refuse that undefined admission, rather than invent a target mapping.
        if effect.implicit_target.iter().any(|&target| target < 0) {
            return Err(SpellLoadError::NegativeImplicitTarget);
        }
        helpers
            .entry((effect.spell_id, effect.difficulty_id))
            .or_default()
            .effects[index] = Some(effect.id);
        if effect.effect == SUMMON {
            if let Some(properties) = catalog.summon_properties(effect.effect_misc_value[1] as u32)
            {
                let flags = u64::from(properties.flags[0] as u32)
                    | (u64::from(properties.flags[1] as u32) << 32);
                if properties.slot == MINIPET && flags & SUMMON_FROM_JOURNAL != 0 {
                    if let Some(&species) = species_by_creature.get(&effect.effect_misc_value[0]) {
                        battle_pets_by_spell.insert(effect.spell_id, species);
                    }
                }
            }
        }
        if effect.effect == LANGUAGE {
            languages
                .entry(effect.effect_misc_value[0] as u32)
                .or_default()
                .push(effect.spell_id);
            counts.language_registrations += 1;
        }
        // SpellDefines.h:197 MAX_SPELLMOD=41. The reference reports but does
        // not discard these modifiers; retain the input and count the warning.
        if matches!(effect.effect_aura, 107 | 108 | 218 | 219) && effect.effect_misc_value[0] >= 41
        {
            counts.invalid_modifier_types += 1;
        }
    }
    for row in catalog.spell_aura_options_records() {
        helpers
            .entry((row.spell_id, row.difficulty_id))
            .or_default()
            .aura_options = Some(row.id);
    }
    for row in catalog.spell_aura_restrictions_records() {
        helpers
            .entry((row.spell_id, row.difficulty_id))
            .or_default()
            .aura_restrictions = Some(row.id);
    }
    for row in catalog.spell_casting_requirements_records() {
        helpers
            .entry((row.spell_id as u32, 0))
            .or_default()
            .casting_requirements = Some(row.id);
    }
    for row in catalog.spell_categories_records() {
        helpers
            .entry((row.spell_id, row.difficulty_id))
            .or_default()
            .categories = Some(row.id);
    }
    for row in catalog.spell_class_options_records() {
        helpers
            .entry((row.spell_id as u32, 0))
            .or_default()
            .class_options = Some(row.id);
    }
    for row in catalog.spell_cooldowns_records() {
        helpers
            .entry((row.spell_id, row.difficulty_id))
            .or_default()
            .cooldowns = Some(row.id);
    }
    // Keep the table joins in SpellMgr.cpp source order: the first helper
    // insertion affects the subsequent std::unordered_map traversal.
    for row in catalog.spell_empower_stage_records() {
        if let Some(empower) = catalog.spell_empower(row.spell_empower_id) {
            let stages = &mut helpers
                .entry((empower.spell_id as u32, 0))
                .or_default()
                .empower_stages;
            // lower_bound inserts BEFORE equal Stage: equal stages reverse ID order.
            let at = stages.partition_point(|&id| {
                catalog
                    .spell_empower_stage(id)
                    .expect("selected stage")
                    .stage
                    < row.stage
            });
            stages.insert(at, row.id);
        }
    }
    for row in catalog.spell_equipped_items_records() {
        helpers
            .entry((row.spell_id as u32, 0))
            .or_default()
            .equipped_items = Some(row.id);
    }
    for row in catalog.spell_interrupts_records() {
        helpers
            .entry((row.spell_id, row.difficulty_id))
            .or_default()
            .interrupts = Some(row.id);
    }
    for row in catalog.spell_label_records() {
        helpers
            .entry((row.spell_id, 0))
            .or_default()
            .labels
            .push(row.id);
    }
    for row in catalog.spell_levels_records() {
        helpers
            .entry((row.spell_id, row.difficulty_id))
            .or_default()
            .levels = Some(row.id);
    }
    for row in catalog.spell_misc_records() {
        helpers
            .entry((row.spell_id, row.difficulty_id))
            .or_default()
            .misc = Some(row.id);
    }
    for row in catalog.spell_power_records() {
        let (difficulty, index) = catalog
            .spell_power_difficulty(row.id)
            .map_or((0, row.order_index), |override_row| {
                (override_row.difficulty_id, override_row.order_index)
            });
        let index = usize::from(index);
        if index >= POWER_SLOTS {
            return Err(SpellLoadError::PowerIndex);
        }
        helpers
            .entry((row.spell_id, difficulty))
            .or_default()
            .powers[index] = Some(row.id);
    }
    for row in catalog.spell_reagents_records() {
        helpers
            .entry((row.spell_id as u32, 0))
            .or_default()
            .reagents = Some(row.id);
    }
    for row in catalog.spell_reagents_currency_records() {
        helpers
            .entry((row.spell_id, 0))
            .or_default()
            .reagent_currencies
            .push(row.id);
    }
    for row in catalog.spell_scaling_records() {
        helpers.entry((row.spell_id as u32, 0)).or_default().scaling = Some(row.id);
    }
    for row in catalog.spell_shapeshift_records() {
        helpers
            .entry((row.spell_id as u32, 0))
            .or_default()
            .shapeshift = Some(row.id);
    }
    for row in catalog.spell_target_restrictions_records() {
        helpers
            .entry((row.spell_id, row.difficulty_id))
            .or_default()
            .target_restrictions = Some(row.id);
    }
    for row in catalog.spell_totems_records() {
        helpers.entry((row.spell_id as u32, 0)).or_default().totems = Some(row.id);
    }
    for row in catalog.spell_x_spell_visual_records() {
        let visuals = &mut helpers
            .entry((row.spell_id, row.difficulty_id))
            .or_default()
            .visuals;
        let key = (row.priority, row.caster_player_condition_id);
        // Source condition ID is uint32. Descending pair; before equal pair.
        let at = visuals.partition_point(|&id| {
            let other = catalog.spell_x_spell_visual(id).expect("selected visual");
            (other.priority, other.caster_player_condition_id) > key
        });
        visuals.insert(at, row.id);
    }
    let mut resolved = BTreeMap::new();
    let mut chains = BTreeMap::new();
    for (&key, helper) in &helpers.values {
        if catalog.spell_name(key.0).is_none() {
            counts.unnamed_helpers += 1;
            continue;
        }
        // Resolve against original helpers. For an acyclic chain this produces
        // the same first nonempty value as C++'s unordered in-place traversal.
        if !chains.contains_key(&key.1) {
            chains.insert(key.1, fallback_chain(&catalog, key.1)?);
        }
        let mut helper = helper.clone();
        for &fallback in &chains[&key.1] {
            if let Some(other) = helpers.values.get(&(key.0, fallback)) {
                helper.fill_missing(other);
            }
        }
        resolved.insert(key, helper);
    }
    counts.spells_and_difficulties = resolved.len();
    counts.battle_pet_spell_associations = battle_pets_by_spell.len();
    let mut ppm_modifiers_by_rate = BTreeMap::<u32, Vec<u32>>::new();
    for row in catalog.spell_procs_per_minute_mod_records() {
        ppm_modifiers_by_rate
            .entry(row.spell_procs_per_minute_id)
            .or_default()
            .push(row.id);
    }
    Ok(SpellLoadPlan {
        catalog,
        helpers: resolved,
        helper_insertions: helpers.insertions,
        ppm_modifiers_by_rate,
        languages,
        battle_pets_by_spell,
        counts,
    })
}

// Preserve the source's first operator[] insertion sequence, independently of
// the sorted lookup map. Repeated field joins do not insert a second helper.
#[derive(Default)]
struct HelperInputs {
    values: BTreeMap<Key, Helper>,
    insertions: Vec<Key>,
}
impl HelperInputs {
    fn entry(&mut self, key: Key) -> std::collections::btree_map::Entry<'_, Key, Helper> {
        let entry = self.values.entry(key);
        if matches!(&entry, std::collections::btree_map::Entry::Vacant(_)) {
            self.insertions.push(key);
        }
        entry
    }
}

fn fallback_chain(catalog: &SpellCatalog, difficulty: i16) -> Result<Vec<i16>, SpellLoadError> {
    let mut seen = BTreeSet::from([difficulty]);
    let mut current = difficulty;
    let mut chain = Vec::new();
    while let Some(row) = catalog.difficulty(current as i32 as u32) {
        let fallback = row.fallback_difficulty_id;
        if !seen.insert(fallback) {
            return Err(SpellLoadError::DifficultyCycle);
        }
        chain.push(fallback);
        current = fallback;
    }
    Ok(chain)
}
