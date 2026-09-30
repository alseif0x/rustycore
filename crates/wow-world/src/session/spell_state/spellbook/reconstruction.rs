//! Resolve loaded reconstruction steps at their original application points.

use super::*;
use wow_entities::{
    LoadedSpellDependency, LoadedSpellInput, LoadedSpellReconstruction, LoadedSpellStep,
    PlayerSpellRuntimeState,
};

type DependencyRows = std::iter::Map<
    std::vec::IntoIter<SpellLearnSpellNodeLikeCpp>,
    fn(SpellLearnSpellNodeLikeCpp) -> LoadedSpellDependency,
>;

fn dependency_fact(row: SpellLearnSpellNodeLikeCpp) -> LoadedSpellDependency {
    LoadedSpellDependency {
        spell_id: row.spell,
        overrides_spell_id: row.overrides_spell,
        active: row.active,
        auto_learned: row.auto_learned,
    }
}

impl WorldSession {
    pub(crate) fn apply_loaded_known_spell_dependencies_like_cpp(
        &mut self,
        known_spells: &mut Vec<i32>,
    ) -> usize {
        let operation = LoadedSpellReconstruction::new(known_spells);
        self.run_loaded_spell_reconstruction(operation, known_spells)
    }

    pub(crate) fn apply_loaded_spell_dependencies_from_roots_like_cpp(
        &mut self,
        roots: &[i32],
        known_spells: &mut Vec<i32>,
    ) -> usize {
        let operation = LoadedSpellReconstruction::new(roots);
        self.run_loaded_spell_reconstruction(operation, known_spells)
    }

    fn run_loaded_spell_reconstruction(
        &mut self,
        mut operation: LoadedSpellReconstruction<DependencyRows>,
        known_spells: &mut Vec<i32>,
    ) -> usize {
        loop {
            let input = match operation.step() {
                LoadedSpellStep::Dependencies(spell_id) => {
                    let learned_spells = self
                        .spell_learn_spell_map_bounds_like_cpp(spell_id)
                        .to_vec();
                    LoadedSpellInput::Dependencies(
                        learned_spells.into_iter().map(dependency_fact as fn(_) -> _),
                    )
                }
                LoadedSpellStep::Flags(spell_id) => {
                    let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
                        runtime.set_dependent_like_cpp(spell_id, true);
                        runtime.set_favorite_like_cpp(spell_id, false);
                    });
                    LoadedSpellInput::Applied
                }
                LoadedSpellStep::Override { overridden, replacement } => {
                    self.add_represented_override_spell_like_cpp(overridden, replacement);
                    LoadedSpellInput::Applied
                }
                LoadedSpellStep::Done(added) => return added,
            };
            operation.advance(input, known_spells);
        }
    }

    pub(crate) fn deactivate_lower_rank_known_spells_for_send_like_cpp(
        &self,
        known_spells: &mut Vec<i32>,
    ) -> usize {
        let Some(spell_chains) = self.spell_catalogs.spell_chain_store() else {
            return 0;
        };
        PlayerSpellRuntimeState::deactivate_lower_loaded_ranks(known_spells, |spell_id| {
            spell_chains.next_spell_in_chain_like_cpp(spell_id)
        })
    }
}
