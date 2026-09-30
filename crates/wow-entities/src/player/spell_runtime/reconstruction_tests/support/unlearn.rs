//! Full removal reaches the productive downgrade operation at Downstep.

use super::*;

impl TestPlayer {
    pub(in crate::player::spell_runtime::reconstruction_tests) fn remove_known_spell_like_cpp(
        &mut self,
        id: i32,
    ) {
        let mut operation = SpellUnlearnOperation::new(id, false);
        loop {
            let input = match operation.step() {
                SpellUnlearnStep::Known(id) => {
                    SpellUnlearnInput::Known(self.known_spells_like_cpp().contains(&id))
                }
                SpellUnlearnStep::RowsComplete => SpellUnlearnInput::RowsComplete(
                    self.player
                        .spell_runtime_like_cpp()
                        .rows_complete_like_cpp(),
                ),
                SpellUnlearnStep::InvalidateRows => {
                    let runtime = &mut self.player.gameplay_state_mut().spells;
                    runtime.clear_rows_like_cpp();
                    runtime.set_acquisition_snapshot_completeness_like_cpp(false, false);
                    SpellUnlearnInput::Applied
                }
                SpellUnlearnStep::NextRank(id) => SpellUnlearnInput::Rank(
                    self.chains
                        .as_ref()
                        .map(|store| store.next_spell_in_chain_like_cpp(id))
                        .unwrap_or(0),
                ),
                SpellUnlearnStep::Talent(_) => SpellUnlearnInput::Talent(false),
                SpellUnlearnStep::Requiring(_) => SpellUnlearnInput::Requiring(Vec::new()),
                SpellUnlearnStep::Owner(step) => SpellUnlearnInput::Owner(Some(
                    self.player
                        .gameplay_state_mut()
                        .spells
                        .apply_unlearn_step(step),
                )),
                SpellUnlearnStep::DowngradeSkill(id) => {
                    if let Some(node) = self.learn_skills.as_ref().and_then(|store| {
                        match store.spell_learn_skill_lookup_like_cpp(id) {
                            wow_data::SpellLearnSkillLookupLikeCpp::Present(node) => Some(*node),
                            _ => None,
                        }
                    }) {
                        let _ =
                            self.run_skills(LearnedSkillOperation::downgrade(node_fact(node), id));
                    }
                    SpellUnlearnInput::Applied
                }
                SpellUnlearnStep::Learned(id) => {
                    let rows = self
                        .learned
                        .as_ref()
                        .map(|store| store.get_spell_learn_spell_map_bounds_like_cpp(id))
                        .unwrap_or(&[])
                        .to_vec();
                    SpellUnlearnInput::Learned(rows.into_iter().map(|row| SpellUnlearnEdge {
                        spell_id: row.spell,
                        overrides_spell_id: row.overrides_spell,
                    }))
                }
                SpellUnlearnStep::RemoveOverride {
                    overridden,
                    replacement,
                } => {
                    self.player
                        .gameplay_state_mut()
                        .spells
                        .remove_override_spell_like_cpp(overridden, replacement);
                    SpellUnlearnInput::Applied
                }
                SpellUnlearnStep::PreviousRank(id) => SpellUnlearnInput::Rank(
                    self.chains
                        .as_ref()
                        .map(|store| store.prev_spell_in_chain_like_cpp(id))
                        .unwrap_or(0),
                ),
                SpellUnlearnStep::Ranked(id) => SpellUnlearnInput::Ranked(
                    self.chains
                        .as_ref()
                        .and_then(|store| store.spell_chain_node_like_cpp(id))
                        .is_some(),
                ),
                SpellUnlearnStep::Reactivate {
                    spell_id,
                    dependent,
                } => {
                    {
                        let runtime = &mut self.player.gameplay_state_mut().spells;
                        runtime.clear_rows_like_cpp();
                        runtime.set_acquisition_snapshot_completeness_like_cpp(false, false);
                    }
                    self.player
                        .gameplay_state_mut()
                        .spells
                        .learn_known_spell_id_like_cpp(spell_id);
                    if dependent {
                        self.player
                            .gameplay_state_mut()
                            .spells
                            .mark_known_spell_dependent_like_cpp(spell_id);
                    } else {
                        self.player
                            .gameplay_state_mut()
                            .spells
                            .set_dependent_like_cpp(spell_id, false);
                    }
                    SpellUnlearnInput::Applied
                }
                SpellUnlearnStep::TraitOverride(_) => SpellUnlearnInput::TraitOverride(0),
                SpellUnlearnStep::Done => return,
                SpellUnlearnStep::Superceded { .. }
                | SpellUnlearnStep::TitanGrip(_)
                | SpellUnlearnStep::DualWield(_)
                | SpellUnlearnStep::Offhand
                | SpellUnlearnStep::Unlearned { .. } => SpellUnlearnInput::Applied,
            };
            operation.advance(input);
        }
    }
}
