//! Resolve one ordered spell-unlearning operation at the application boundary.
//! Catalog reads, canonical access, skill/equipment effects and packets retain
//! their original points in Player::RemoveSpell's represented Rust sequence.
use super::*;
use wow_entities::{
    SpellUnlearnEdge, SpellUnlearnInput, SpellUnlearnOperation, SpellUnlearnStep,
};

impl WorldSession {
    pub(crate) fn remove_known_spell_like_cpp(&mut self, spell_id: i32) {
        self.remove_known_spell_with_suppress_messaging_like_cpp(spell_id, false);
    }

    pub(crate) fn remove_known_spell_with_suppress_messaging_like_cpp(
        &mut self,
        spell_id: i32,
        suppress_messaging: bool,
    ) {
        if self.known_spells_like_cpp().contains(&spell_id) {
            self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        }
        let mut operation = SpellUnlearnOperation::new(spell_id, suppress_messaging);
        loop {
            let input = match operation.step() {
                SpellUnlearnStep::Known(spell_id) => SpellUnlearnInput::Known(
                    self.known_spells_like_cpp().contains(&spell_id),
                ),
                SpellUnlearnStep::RowsComplete => SpellUnlearnInput::RowsComplete(
                    self.with_player_spell_runtime_like_cpp(|runtime| runtime.rows_complete_like_cpp())
                        .unwrap_or(false),
                ),
                SpellUnlearnStep::InvalidateRows => {
                    self.invalidate_represented_player_spell_rows_like_cpp();
                    SpellUnlearnInput::Applied
                }
                SpellUnlearnStep::NextRank(spell_id) => {
                    SpellUnlearnInput::Rank(self.next_spell_in_chain_like_cpp(spell_id))
                }
                SpellUnlearnStep::Talent(spell_id) => SpellUnlearnInput::Talent(
                    self.spell_custom_attributes_for_difficulty_like_cpp(spell_id, 0)
                        & wow_data::SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP != 0,
                ),
                SpellUnlearnStep::Requiring(spell_id) => {
                    let spells_requiring_removed: Vec<i32> = self
                        .spells_requiring_spell_like_cpp(spell_id)
                        .iter()
                        .filter_map(|spell| i32::try_from(*spell).ok())
                        .collect();
                    SpellUnlearnInput::Requiring(spells_requiring_removed)
                }
                SpellUnlearnStep::Owner(step) => SpellUnlearnInput::Owner(
                    self.mutate_player_spell_runtime_like_cpp(|runtime| {
                        runtime.apply_unlearn_step(step)
                    }),
                ),
                SpellUnlearnStep::DowngradeSkill(current_spell_id) => {
                    if let Some(learned_skill) = self.spell_learn_skill_like_cpp(current_spell_id).copied() {
                        self.downgrade_represented_spell_learn_skill_like_cpp(
                            learned_skill,
                            current_spell_id,
                        );
                    }
                    SpellUnlearnInput::Applied
                }
                SpellUnlearnStep::Learned(current_spell_id) => {
                    let learned_spells: Vec<SpellLearnSpellNodeLikeCpp> = self
                        .spell_learn_spell_map_bounds_like_cpp(current_spell_id)
                        .to_vec();
                    SpellUnlearnInput::Learned(learned_spells.into_iter().map(|node| {
                        SpellUnlearnEdge {
                            spell_id: node.spell,
                            overrides_spell_id: node.overrides_spell,
                        }
                    }))
                }
                SpellUnlearnStep::RemoveOverride { overridden, replacement } => {
                    self.remove_represented_override_spell_like_cpp(overridden, replacement);
                    SpellUnlearnInput::Applied
                }
                SpellUnlearnStep::PreviousRank(spell_id) => {
                    SpellUnlearnInput::Rank(self.prev_spell_in_chain_like_cpp(spell_id))
                }
                SpellUnlearnStep::Ranked(current_spell_id) => SpellUnlearnInput::Ranked(
                    self.spell_catalogs
                        .spell_chain_store()
                        .and_then(|store| store.spell_chain_node_like_cpp(current_spell_id))
                        .is_some(),
                ),
                SpellUnlearnStep::Reactivate { spell_id, dependent } => {
                    if dependent {
                        self.learn_dependent_known_spell_like_cpp(spell_id);
                    } else {
                        self.learn_known_spell_like_cpp(spell_id);
                        let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
                            runtime.set_dependent_like_cpp(spell_id, false);
                        });
                    }
                    SpellUnlearnInput::Applied
                }
                SpellUnlearnStep::Superceded { spell_id, previous_spell_id } => {
                    self.send_packet(
                        &wow_packet::packets::trainer::SupercededSpells::single(
                            spell_id,
                            previous_spell_id,
                        ),
                    );
                    SpellUnlearnInput::Applied
                }
                SpellUnlearnStep::TraitOverride(trait_definition_id) => SpellUnlearnInput::TraitOverride(
                    self.trait_definition_store()
                        .and_then(|store| store.get(trait_definition_id))
                        .map(|definition| definition.overrides_spell_id)
                        .unwrap_or(0),
                ),
                SpellUnlearnStep::TitanGrip(spell_id) => {
                    self.cleanup_removed_spell_titan_grip_like_cpp(spell_id);
                    SpellUnlearnInput::Applied
                }
                SpellUnlearnStep::DualWield(spell_id) => {
                    self.cleanup_removed_spell_dual_wield_like_cpp(spell_id);
                    SpellUnlearnInput::Applied
                }
                SpellUnlearnStep::Offhand => {
                    if self.spell_state.represented_offhand_check_at_spell_unlearn_like_cpp {
                        self.represented_auto_unequip_offhand_if_need_like_cpp(false);
                    }
                    SpellUnlearnInput::Applied
                }
                SpellUnlearnStep::Unlearned { spell_id, suppress_messaging } => {
                    self.send_packet(&wow_packet::packets::trainer::UnlearnedSpells::single(
                        spell_id,
                        suppress_messaging,
                    ));
                    SpellUnlearnInput::Applied
                }
                SpellUnlearnStep::Done => break,
            };
            operation.advance(input);
        }
    }
}
