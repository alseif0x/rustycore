use super::*;
use crate::PlayerSpellRuntimeState;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(super) struct Catalog {
    pub next: BTreeMap<u32, u32>,
    pub previous: BTreeMap<u32, u32>,
    pub ranked: BTreeSet<u32>,
    pub talents: BTreeSet<u32>,
    pub requiring: BTreeMap<u32, Vec<i32>>,
    pub learned: BTreeMap<u32, Vec<SpellUnlearnEdge>>,
    pub traits: BTreeMap<u32, i32>,
}

pub(super) fn runtime(spells: Vec<i32>) -> PlayerSpellRuntimeState {
    let mut runtime = PlayerSpellRuntimeState::default();
    runtime.replace_known_spell_ids_like_cpp(spells);
    runtime
}

pub(super) fn remove(
    runtime: &mut PlayerSpellRuntimeState,
    catalog: &Catalog,
    spell_id: i32,
    suppress: bool,
) -> Vec<SpellUnlearnStep> {
    drive(runtime, catalog, spell_id, suppress, &BTreeSet::new(), &BTreeSet::new())
}

pub(super) fn drive(
    runtime: &mut PlayerSpellRuntimeState,
    catalog: &Catalog,
    spell_id: i32,
    suppress: bool,
    failed_forget: &BTreeSet<i32>,
    failed_trait: &BTreeSet<i32>,
) -> Vec<SpellUnlearnStep> {
    let mut operation = SpellUnlearnOperation::new(spell_id, suppress);
    let mut trace = Vec::new();
    loop {
        let step = operation.step();
        trace.push(step);
        let input = match step {
            SpellUnlearnStep::Known(id) => SpellUnlearnInput::Known(
                runtime.known_spells_like_cpp().contains(&id),
            ),
            SpellUnlearnStep::RowsComplete => SpellUnlearnInput::RowsComplete(
                runtime.rows_complete_like_cpp(),
            ),
            SpellUnlearnStep::InvalidateRows => {
                runtime.clear_rows_like_cpp();
                runtime.set_acquisition_snapshot_completeness_like_cpp(false, false);
                SpellUnlearnInput::Applied
            }
            SpellUnlearnStep::NextRank(id) => {
                SpellUnlearnInput::Rank(catalog.next.get(&id).copied().unwrap_or(0))
            }
            SpellUnlearnStep::Talent(id) => SpellUnlearnInput::Talent(catalog.talents.contains(&id)),
            SpellUnlearnStep::Requiring(id) => SpellUnlearnInput::Requiring(
                catalog.requiring.get(&id).cloned().unwrap_or_default(),
            ),
            SpellUnlearnStep::Owner(owner_step) => {
                let fail = match owner_step {
                    SpellUnlearnOwnerStep::Forget { spell_id, .. } => failed_forget.contains(&spell_id),
                    SpellUnlearnOwnerStep::DropOverridesAndTrait { spell_id } => failed_trait.contains(&spell_id),
                };
                SpellUnlearnInput::Owner((!fail).then(|| runtime.apply_unlearn_step(owner_step)))
            }
            SpellUnlearnStep::Learned(id) => SpellUnlearnInput::Learned(
                catalog.learned.get(&id).cloned().unwrap_or_default().into_iter(),
            ),
            SpellUnlearnStep::RemoveOverride { overridden, replacement } => {
                runtime.remove_override_spell_like_cpp(overridden, replacement);
                SpellUnlearnInput::Applied
            }
            SpellUnlearnStep::PreviousRank(id) => {
                SpellUnlearnInput::Rank(catalog.previous.get(&id).copied().unwrap_or(0))
            }
            SpellUnlearnStep::Ranked(id) => SpellUnlearnInput::Ranked(catalog.ranked.contains(&id)),
            SpellUnlearnStep::Reactivate { spell_id, dependent } => {
                // Existing low-level LearnSpell invalidation precedes its writer.
                runtime.clear_rows_like_cpp();
                runtime.set_acquisition_snapshot_completeness_like_cpp(false, false);
                runtime.learn_known_spell_id_like_cpp(spell_id);
                if dependent {
                    runtime.mark_known_spell_dependent_like_cpp(spell_id);
                } else {
                    runtime.set_dependent_like_cpp(spell_id, false);
                }
                SpellUnlearnInput::Applied
            }
            SpellUnlearnStep::TraitOverride(id) => {
                SpellUnlearnInput::TraitOverride(catalog.traits.get(&id).copied().unwrap_or(0))
            }
            SpellUnlearnStep::Done => break,
            // Skill, equipment and wire delivery are external application effects.
            SpellUnlearnStep::DowngradeSkill(_)
            | SpellUnlearnStep::Superceded { .. }
            | SpellUnlearnStep::TitanGrip(_)
            | SpellUnlearnStep::DualWield(_)
            | SpellUnlearnStep::Offhand
            | SpellUnlearnStep::Unlearned { .. } => SpellUnlearnInput::Applied,
        };
        operation.advance(input);
    }
    trace
}
