//! Complete source global correction rules with explicit producer traversal.
//! SpellMgr.cpp:5259-5336, 02245dcd. No BTree-order or fixed-point substitute.
//! Shape validation is not evidence of C++ traversal provenance: composition
//! must supply both actual first-index order and second-index lookup order.
#[cfg(test)]
mod tests;
use super::super::TargetSelection;
use super::{Definition, DefinitionOrder, Key, SpellDefinitionError, SpellDefinitionSeeds};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use wow_data::forever_spells::{SpellCatalog, SummonPropertiesPatch};

/// Unverified producer output. The startup owner validates the exact key set
/// before mutation. Hash/toolchain/source provenance remains the producer's gate.
pub struct SpellTraversal {
    primary: Vec<Key>,
    secondary: Vec<Key>,
}
impl SpellTraversal {
    pub fn new(primary: Vec<(u32, i16)>, secondary: Vec<(u32, i16)>) -> Self {
        Self { primary, secondary }
    }
    fn validate(
        &self,
        definitions: &BTreeMap<Key, Definition>,
    ) -> Result<(), SpellDefinitionError> {
        let expected: BTreeSet<_> = definitions.keys().copied().collect();
        for order in [&self.primary, &self.secondary] {
            if order.len() != expected.len()
                || order.iter().copied().collect::<BTreeSet<_>>() != expected
            {
                return Err(SpellDefinitionError::InvalidCorrectionTraversal);
            }
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GlobalCorrectionCounts {
    pub definitions: usize,
    pub effect_slots: usize,
    pub trajectory_range_assignments: usize,
    pub movement_speeds: usize,
    pub cone_angles: usize,
    pub redirected_area_aura_effects: usize,
    pub magnet_proc_clears: usize,
    pub vehicle_facing_flags: usize,
    pub passive_flight_flags: usize,
    pub single_target_limits: usize,
    pub summon_properties_applied: usize,
    pub summon_properties_missing: usize,
}

impl SpellDefinitionSeeds {
    /// Complete the global phase only with explicit traversal and exclusive raw
    /// ownership. No failure can return an admitted/partially corrected owner.
    /// This still does not supply custom attributes/learning or ready SpellInfo.
    pub fn with_global_corrections(
        mut self,
        traversal: SpellTraversal,
    ) -> Result<Self, SpellDefinitionError> {
        if self.id_corrections.is_none() {
            return Err(SpellDefinitionError::GlobalCorrectionsRequireIdCorrections);
        }
        if self.global_corrections.is_some() {
            return Err(SpellDefinitionError::GlobalCorrectionsAlreadyApplied);
        }
        traversal.validate(&self.definitions)?;
        // Check exclusivity before changing derived fields. No COW/raw clone.
        if Arc::get_mut(&mut self.catalog).is_none() {
            return Err(SpellDefinitionError::SharedCorrectionCatalog);
        }
        let mut lookup = BTreeMap::<u32, Vec<Key>>::new();
        for &key in &traversal.secondary {
            lookup.entry(key.0).or_default().push(key);
        }
        let mut counts = GlobalCorrectionCounts::default();
        for &key in &traversal.primary {
            counts.definitions += 1;
            let slots = self.definitions[&key].effects.len();
            for index in 0..slots {
                counts.effect_slots += 1;
                let effect = &self.definitions[&key].effects[index];
                let trigger = effect.trigger_spell;
                if effect.effect != 0 && effect.implicit_targets.contains(&89) {
                    if let Some(children) = lookup.get(&trigger) {
                        for child in children {
                            // Source reads parent max inside each child iteration.
                            // A self-trigger may change another difficulty's range.
                            let main_range = self.definitions[&key].range;
                            let main_max = max_range(&self.catalog, main_range);
                            let child_max = max_range(&self.catalog, self.definitions[child].range);
                            if child_max < main_max {
                                self.definitions
                                    .get_mut(child)
                                    .expect("validated lookup key")
                                    .range = main_range;
                                counts.trajectory_range_assignments += 1;
                            }
                        }
                    }
                }
                let spell = self
                    .definitions
                    .get_mut(&key)
                    .expect("validated primary key");
                local_effect(spell, index, &mut counts);
            }
            local_spell(
                self.definitions
                    .get_mut(&key)
                    .expect("validated primary key"),
                &mut counts,
            );
        }
        // SharedDefines.h:6664-6685: PET control=2, Totem title=4, NOT slot IDs.
        // DB2HotfixGenerator's default notifyClient=false: no new metadata.
        let raw = Arc::get_mut(&mut self.catalog).expect("exclusive startup catalog");
        let raw_counts = raw.apply_summon_properties_patches([
            SummonPropertiesPatch {
                id: 121,
                title: Some(4),
                control: None,
            },
            SummonPropertiesPatch {
                id: 647,
                title: Some(4),
                control: None,
            },
            SummonPropertiesPatch {
                id: 628,
                title: None,
                control: Some(2),
            },
        ]);
        counts.summon_properties_applied = raw_counts.applied;
        counts.summon_properties_missing = raw_counts.missing;
        self.global_corrections = Some(counts);
        // Promote admitted key order to canonical readonly collection indexes
        // for later source-ordered passes. No native/helper/history survives.
        self.source_order = Some(DefinitionOrder {
            primary: traversal.primary,
            by_spell: lookup,
        });
        self.traversal_inputs = None;
        Ok(self)
    }
    pub fn global_correction_counts(&self) -> Option<GlobalCorrectionCounts> {
        self.global_corrections
    }
}
fn max_range(catalog: &SpellCatalog, range: Option<u32>) -> f32 {
    range
        .and_then(|id| catalog.spell_range(id))
        .map_or(0.0, |row| row.range_max[0])
}
fn local_effect(spell: &mut Definition, index: usize, counts: &mut GlobalCorrectionCounts) {
    let effect = &mut spell.effects[index];
    if matches!(effect.effect, 96 | 149 | 41 | 42 | 138)
        && spell.fields.speed == 0.0
        && spell.fields.spell_family_name == 0
        && spell.fields.attributes[9] & 0x10 == 0
    {
        spell.fields.speed = 42.0;
        counts.movement_speeds += 1;
    }
    let targets = effect.target_metadata();
    if targets
        .iter()
        .any(|t| t.selection_category() == TargetSelection::Cone)
        && fuzzy_zero(spell.fields.cone_angle)
    {
        spell.fields.cone_angle = 90.0;
        counts.cone_angles += 1;
    }
    if effect.is_area_aura_effect() && targets.iter().any(|target| target.is_area()) {
        effect.implicit_targets = [1, 0];
        counts.redirected_area_aura_effects += 1;
    }
}
fn local_spell(spell: &mut Definition, counts: &mut GlobalCorrectionCounts) {
    let has_aura = |aura| spell.effects.iter().any(|effect| effect.is_aura_kind(aura));
    if has_aura(96) {
        spell.fields.proc_flags = [0; 2];
        counts.magnet_proc_clears += 1;
    }
    if has_aura(236) {
        spell.fields.attributes[5] |= 0x0008_0000;
        counts.vehicle_facing_flags += 1;
    }
    if spell.fields.active_icon_file_data_id == 135754 {
        spell.fields.attributes[0] |= 0x40;
        counts.passive_flight_flags += 1;
    }
    if spell.fields.attributes[5] & 0x20 != 0 && spell.fields.max_affected_targets == 0 {
        spell.fields.max_affected_targets = 1;
        counts.single_target_limits += 1;
    }
}
fn fuzzy_zero(angle: f32) -> bool {
    // g3dmath.h:133,839-856. The source adds/multiplies in float precision.
    let magnitude = angle.abs() + 1.0;
    let epsilon = if magnitude == f32::INFINITY {
        0.00001
    } else {
        0.00001 * magnitude
    };
    angle == 0.0 || angle.abs() <= epsilon
}
