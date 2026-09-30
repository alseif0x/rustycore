//! Lazy catalog and canonical-skill access for the owned skill operation.

use super::*;
use wow_entities::{
    LearnedSkillInput, LearnedSkillLookup, LearnedSkillNode, LearnedSkillOperation,
    LearnedSkillRange, LearnedSkillStep,
};

fn learned_skill_fact(node: SpellLearnSkillNodeLikeCpp) -> LearnedSkillNode {
    LearnedSkillNode {
        skill_id: node.skill,
        step: node.step,
        value: node.value,
        max_value: node.maxvalue,
    }
}

impl WorldSession {
    pub(crate) fn apply_loaded_spell_learn_skills_like_cpp(&mut self, roots: &[i32]) -> bool {
        self.run_learned_skill_operation(LearnedSkillOperation::gain(roots))
    }

    pub(super) fn downgrade_represented_spell_learn_skill_like_cpp(
        &mut self,
        learned_skill: SpellLearnSkillNodeLikeCpp,
        current_spell_id: u32,
    ) {
        let _ = self.run_learned_skill_operation(LearnedSkillOperation::downgrade(
            learned_skill_fact(learned_skill),
            current_spell_id,
        ));
    }

    fn run_learned_skill_operation(&mut self, mut operation: LearnedSkillOperation<'_>) -> bool {
        loop {
            let input = match operation.step() {
                LearnedSkillStep::GainNode(spell_id) => {
                    let lookup = match self.spell_learn_skill_lookup_like_cpp(spell_id) {
                        SpellLearnSkillLookupLikeCpp::Present(node) => {
                            LearnedSkillLookup::Present(learned_skill_fact(*node))
                        }
                        SpellLearnSkillLookupLikeCpp::CoveredWithoutNode => LearnedSkillLookup::Absent,
                        SpellLearnSkillLookupLikeCpp::Indeterminate(_)
                        | SpellLearnSkillLookupLikeCpp::MissingCoverage => LearnedSkillLookup::Unavailable,
                    };
                    LearnedSkillInput::GainNode(lookup)
                }
                LearnedSkillStep::PreviousRank(spell_id) => {
                    LearnedSkillInput::Rank(self.prev_spell_in_chain_like_cpp(spell_id))
                }
                LearnedSkillStep::PreviousNode(spell_id) => LearnedSkillInput::PreviousNode(
                    self.spell_learn_skill_like_cpp(spell_id).copied().map(learned_skill_fact),
                ),
                LearnedSkillStep::FirstRank(spell_id) => {
                    LearnedSkillInput::Rank(self.first_spell_in_chain_like_cpp(spell_id))
                }
                LearnedSkillStep::Value(skill_id) => {
                    LearnedSkillInput::Value(self.resolved_player_skill_value_like_cpp(skill_id))
                }
                LearnedSkillStep::Maximum(skill_id) => {
                    LearnedSkillInput::Maximum(self.resolved_player_skill_max_value_like_cpp(skill_id))
                }
                LearnedSkillStep::Range(skill_id) => {
                    self.resolve_learned_skill_range(&mut operation, skill_id);
                    continue;
                }
                LearnedSkillStep::Write(write) => {
                    self.set_represented_player_skill_like_cpp(
                        write.skill_id, write.step, write.value, write.max_value,
                    );
                    LearnedSkillInput::Applied
                }
                LearnedSkillStep::Done(complete) => return complete,
                LearnedSkillStep::LevelMaximum | LearnedSkillStep::TierMaximum { .. } => {
                    unreachable!("range resolution retains the original borrowed tier store")
                }
            };
            operation.advance(input);
        }
    }

    fn resolve_learned_skill_range(
        &self,
        operation: &mut LearnedSkillOperation<'_>,
        skill_id: u16,
    ) {
        let (Some(skills), Some(lines), Some(tiers)) = (
            self.skill_store(),
            self.skill_line_store(),
            self.skill_tiers_store(),
        ) else {
            operation.advance(LearnedSkillInput::Range(LearnedSkillRange::Unavailable));
            return;
        };
        let Some(rc_info) = skills.skill_race_class_info_like_cpp(
            skill_id,
            self.player_race_like_cpp(),
            self.player_class_like_cpp(),
        ) else {
            operation.advance(LearnedSkillInput::Range(LearnedSkillRange::Unavailable));
            return;
        };
        let range = skills.skill_range_type_like_cpp(rc_info, lines, tiers);
        let always_max = rc_info.flags & wow_data::SKILL_FLAG_ALWAYS_MAX_VALUE_LIKE_CPP != 0;
        let facts = match range {
            SkillRangeTypeLikeCpp::Language => LearnedSkillRange::Language { always_max },
            SkillRangeTypeLikeCpp::Level => LearnedSkillRange::Level { always_max },
            SkillRangeTypeLikeCpp::Mono => LearnedSkillRange::Mono { always_max },
            SkillRangeTypeLikeCpp::Rank => LearnedSkillRange::Rank {
                always_max,
                tier_id: rc_info.skill_tier_id,
            },
            SkillRangeTypeLikeCpp::None => LearnedSkillRange::None { always_max },
        };
        operation.advance(LearnedSkillInput::Range(facts));
        match operation.step() {
            LearnedSkillStep::LevelMaximum => {
                operation.advance(LearnedSkillInput::LevelMaximum(
                    self.max_skill_value_for_level_like_cpp(),
                ));
            }
            LearnedSkillStep::TierMaximum { tier_id, index } => {
                let maximum = u32::try_from(tier_id)
                    .ok()
                    .and_then(|id| tiers.get_skill_tier_like_cpp(id))
                    .map(|tier| tier.get_value_for_tier_index_like_cpp(index));
                operation.advance(LearnedSkillInput::TierMaximum(maximum));
            }
            _ => {}
        }
    }
}
