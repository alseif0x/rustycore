//! First correction phase: all 191 ID-specific groups, before global rules.
//! 02245dcd SpellMgr.cpp:3378-3403 / 3405-5257. This is not the full
//! LoadSpellInfoCorrections operation: trajectory/implicit-target/global rules
//! and SummonProperties changes must still follow before executable publication.
mod campaigns;
mod constants;
mod encounters;
mod general;
#[cfg(test)]
mod tests;
use super::{Definition, Key, SpellDefinitionError, SpellDefinitionSeeds, SpellEffectValues};
use std::collections::BTreeMap;
use wow_data::forever_spells::SpellCatalog;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IdCorrectionCounts {
    pub patch_groups: usize,
    pub requested_spell_ids: usize,
    pub missing_spell_requests: usize,
    pub spell_patch_applications: usize,
    pub effect_patch_applications: usize,
    pub missing_effect_requests: usize,
}

impl SpellDefinitionSeeds {
    /// Consume the startup owner before its Arc publication. All difficulties
    /// are corrected; repeating the additive rules is explicitly rejected.
    /// No corrected/ready SpellInfo claim is made by this intermediate phase.
    pub fn with_id_corrections(mut self) -> Result<Self, SpellDefinitionError> {
        if self.id_corrections.is_some() {
            return Err(SpellDefinitionError::IdCorrectionsAlreadyApplied);
        }
        let mut counts = IdCorrectionCounts::default();
        general::run(&self.catalog, &mut self.definitions, &mut counts);
        encounters::run(&self.catalog, &mut self.definitions, &mut counts);
        campaigns::run(&self.catalog, &mut self.definitions, &mut counts);
        self.id_corrections = Some(counts);
        Ok(self)
    }
    pub fn id_correction_counts(&self) -> Option<IdCorrectionCounts> {
        self.id_corrections
    }
}

// _GetSpellInfo returns every existing difficulty, without fallback construction.
fn apply(
    ids: &[u32],
    catalog: &SpellCatalog,
    definitions: &mut BTreeMap<Key, Definition>,
    counts: &mut IdCorrectionCounts,
    fix: fn(&mut Definition, &SpellCatalog, &mut IdCorrectionCounts),
) {
    counts.patch_groups += 1;
    counts.requested_spell_ids += ids.len();
    for &id in ids {
        let mut found = false;
        for (_, definition) in definitions.range_mut((id, i16::MIN)..=(id, i16::MAX)) {
            found = true;
            counts.spell_patch_applications += 1;
            fix(definition, catalog, counts);
        }
        if !found {
            counts.missing_spell_requests += 1;
        }
    }
}

// Source checks vector length, NOT effect != NONE. An existing blank gap is
// eligible; a missing slot is logged/skipped and never appended by a correction.
fn effect_slot<'a>(
    definition: &'a mut Definition,
    index: usize,
    counts: &mut IdCorrectionCounts,
) -> Option<&'a mut SpellEffectValues> {
    match definition.effects.get_mut(index) {
        Some(effect) => {
            counts.effect_patch_applications += 1;
            Some(effect)
        }
        None => {
            counts.missing_effect_requests += 1;
            None
        }
    }
}
