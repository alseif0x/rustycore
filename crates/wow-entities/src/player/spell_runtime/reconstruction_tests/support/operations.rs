//! Resolve facts from real stores; all rules and transitions run in owners.

use super::*;

type Rows = std::iter::Map<
    std::vec::IntoIter<wow_data::SpellLearnSpellNodeLikeCpp>,
    fn(wow_data::SpellLearnSpellNodeLikeCpp) -> LoadedSpellDependency,
>;

fn dependency_fact(row: wow_data::SpellLearnSpellNodeLikeCpp) -> LoadedSpellDependency {
    LoadedSpellDependency {
        spell_id: row.spell,
        overrides_spell_id: row.overrides_spell,
        active: row.active,
        auto_learned: row.auto_learned,
    }
}

impl TestPlayer {
    pub(in crate::player::spell_runtime::reconstruction_tests) fn deactivate_lower_rank_known_spells_for_send_like_cpp(
        &self,
        known: &mut Vec<i32>,
    ) -> usize {
        let Some(chains) = &self.chains else { return 0 };
        PlayerSpellRuntimeState::deactivate_lower_loaded_ranks(known, |id| {
            chains.next_spell_in_chain_like_cpp(id)
        })
    }

    pub(in crate::player::spell_runtime::reconstruction_tests) fn apply_loaded_known_spell_dependencies_like_cpp(
        &mut self,
        known: &mut Vec<i32>,
    ) -> usize {
        let operation = LoadedSpellReconstruction::new(known);
        self.reconstruct(operation, known)
    }

    pub(in crate::player::spell_runtime::reconstruction_tests) fn apply_loaded_spell_dependencies_from_roots_like_cpp(
        &mut self,
        roots: &[i32],
        known: &mut Vec<i32>,
    ) -> usize {
        self.reconstruct(LoadedSpellReconstruction::new(roots), known)
    }

    fn reconstruct(
        &mut self,
        mut operation: LoadedSpellReconstruction<Rows>,
        known: &mut Vec<i32>,
    ) -> usize {
        loop {
            let input = match operation.step() {
                LoadedSpellStep::Dependencies(id) => {
                    let rows = self
                        .learned
                        .as_ref()
                        .map(|store| store.get_spell_learn_spell_map_bounds_like_cpp(id))
                        .unwrap_or(&[])
                        .to_vec();
                    LoadedSpellInput::Dependencies(
                        rows.into_iter().map(dependency_fact as fn(_) -> _),
                    )
                }
                LoadedSpellStep::Flags(id) => {
                    let runtime = &mut self.player.gameplay_state_mut().spells;
                    runtime.set_dependent_like_cpp(id, true);
                    runtime.set_favorite_like_cpp(id, false);
                    LoadedSpellInput::Applied
                }
                LoadedSpellStep::Override {
                    overridden,
                    replacement,
                } => {
                    self.player
                        .gameplay_state_mut()
                        .spells
                        .add_override_spell_like_cpp(overridden, replacement);
                    LoadedSpellInput::Applied
                }
                LoadedSpellStep::Done(added) => return added,
            };
            operation.advance(input, known);
        }
    }

    pub(in crate::player::spell_runtime::reconstruction_tests) fn apply_loaded_spell_dependency_skills_like_cpp(
        &mut self,
        known: &mut Vec<i32>,
        side_effects: &mut Vec<i32>,
    ) -> (usize, bool) {
        let added = self.apply_loaded_known_spell_dependencies_like_cpp(known);
        for &id in known.iter() {
            if !side_effects.contains(&id) {
                side_effects.push(id);
            }
        }
        let complete = self.run_skills(LearnedSkillOperation::gain(side_effects));
        (added, complete)
    }

    pub(in crate::player::spell_runtime::reconstruction_tests) fn run_skills(
        &mut self,
        mut operation: LearnedSkillOperation<'_>,
    ) -> bool {
        loop {
            let step = operation.step();
            self.trace.push(step);
            let input = match step {
                LearnedSkillStep::GainNode(id) => LearnedSkillInput::GainNode(
                    match self
                        .learn_skills
                        .as_ref()
                        .map(|store| store.spell_learn_skill_lookup_like_cpp(id))
                    {
                        Some(wow_data::SpellLearnSkillLookupLikeCpp::Present(node)) => {
                            LearnedSkillLookup::Present(node_fact(*node))
                        }
                        Some(wow_data::SpellLearnSkillLookupLikeCpp::CoveredWithoutNode) => {
                            LearnedSkillLookup::Absent
                        }
                        _ => LearnedSkillLookup::Unavailable,
                    },
                ),
                LearnedSkillStep::PreviousRank(id) => LearnedSkillInput::Rank(
                    self.chains
                        .as_ref()
                        .map(|store| store.prev_spell_in_chain_like_cpp(id))
                        .unwrap_or(0),
                ),
                LearnedSkillStep::PreviousNode(id) => LearnedSkillInput::PreviousNode(
                    self.learn_skills
                        .as_ref()
                        .and_then(|store| match store.spell_learn_skill_lookup_like_cpp(id) {
                            wow_data::SpellLearnSkillLookupLikeCpp::Present(node) => Some(*node),
                            _ => None,
                        })
                        .map(node_fact),
                ),
                LearnedSkillStep::FirstRank(id) => LearnedSkillInput::Rank(
                    self.chains
                        .as_ref()
                        .map(|store| store.first_spell_in_chain_like_cpp(id))
                        .unwrap_or(id),
                ),
                LearnedSkillStep::Value(id) => {
                    LearnedSkillInput::Value(Some(self.player_skill_value_like_cpp(id)))
                }
                LearnedSkillStep::Maximum(id) => {
                    LearnedSkillInput::Maximum(Some(self.player_skill_max_value_like_cpp(id)))
                }
                LearnedSkillStep::Range(id) => {
                    let facts = if let (Some(skills), Some(lines), Some(tiers)) =
                        (&self.skills, &self.lines, &self.tiers)
                    {
                        if let Some(rc) = skills.skill_race_class_info_like_cpp(
                            id,
                            self.player.race_like_cpp(),
                            self.player.class_like_cpp(),
                        ) {
                            let always_max =
                                rc.flags & wow_data::SKILL_FLAG_ALWAYS_MAX_VALUE_LIKE_CPP != 0;
                            match skills.skill_range_type_like_cpp(rc, lines, tiers) {
                                wow_data::SkillRangeTypeLikeCpp::Language => {
                                    LearnedSkillRange::Language { always_max }
                                }
                                wow_data::SkillRangeTypeLikeCpp::Level => {
                                    LearnedSkillRange::Level { always_max }
                                }
                                wow_data::SkillRangeTypeLikeCpp::Mono => {
                                    LearnedSkillRange::Mono { always_max }
                                }
                                wow_data::SkillRangeTypeLikeCpp::Rank => LearnedSkillRange::Rank {
                                    always_max,
                                    tier_id: rc.skill_tier_id,
                                },
                                wow_data::SkillRangeTypeLikeCpp::None => {
                                    LearnedSkillRange::None { always_max }
                                }
                            }
                        } else {
                            LearnedSkillRange::Unavailable
                        }
                    } else {
                        LearnedSkillRange::Unavailable
                    };
                    LearnedSkillInput::Range(facts)
                }
                LearnedSkillStep::LevelMaximum => LearnedSkillInput::LevelMaximum(
                    PlayerGameplayState::skill_maximum_for_level(self.player.level_like_cpp()),
                ),
                LearnedSkillStep::TierMaximum { tier_id, index } => LearnedSkillInput::TierMaximum(
                    u32::try_from(tier_id)
                        .ok()
                        .and_then(|id| self.tiers.as_ref()?.get_skill_tier_like_cpp(id))
                        .map(|tier| tier.get_value_for_tier_index_like_cpp(index)),
                ),
                LearnedSkillStep::Write(write) => {
                    self.write_skill(write);
                    LearnedSkillInput::Applied
                }
                LearnedSkillStep::Done(complete) => return complete,
            };
            operation.advance(input);
        }
    }
}
