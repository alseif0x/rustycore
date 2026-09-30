//! spells operations at the existing Quest application boundary.

use super::*;

impl WorldSession {
    /// Represented C++ `Player::LearnQuestRewardedSpells`.
    ///
    /// C++ casts each rewarded quest's `RewardSpell` only when that spell exists,
    /// has a missing `SPELL_EFFECT_LEARN_SPELL` trigger, and the first learned
    /// spell is tied to `SKILL_LINE_ABILITY_REWARDED_FROM_QUEST` when it is not
    /// already known. This represented slice applies only the resulting direct
    /// learned-spell side effect; full `CastSpell` runtime semantics remain in
    /// the spell-system roadmap.
    pub(crate) fn apply_represented_quest_rewarded_spells_like_cpp(&mut self) -> usize {
        let Some(quest_store) = self.quests.store.clone() else {
            return 0;
        };

        let Some(quests) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return 0;
        };
        let mut quest_ids = quests
            .rewarded_quest_ids_like_cpp()
            .iter()
            .copied()
            .collect::<Vec<_>>();
        quest_ids.sort_unstable();

        let mut learned = 0usize;
        for quest_id in quest_ids {
            let Some(quest) = quest_store.get(quest_id) else {
                continue;
            };
            if quest.reward_spell == u32::MAX && quest.source_spell_id != 0 {
                let Ok(source_spell_id) = i32::try_from(quest.source_spell_id) else {
                    continue;
                };
                learned += self.remove_represented_auras_due_to_spell_like_cpp(source_spell_id);
                continue;
            }
            for spell_id in self.represented_quest_rewarded_spell_triggers_like_cpp(quest) {
                self.learn_known_spell_like_cpp(spell_id);
                learned += 1;
            }
        }

        learned
    }
    pub(super) fn represented_quest_rewarded_spell_triggers_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> Vec<i32> {
        if quest.reward_spell == 0 {
            return Vec::new();
        }

        let Ok(reward_spell_id) = i32::try_from(quest.reward_spell) else {
            return Vec::new();
        };

        let Some(spell_info) = self
            .spell_store()
            .and_then(|store| store.get(reward_spell_id))
        else {
            return Vec::new();
        };

        let missing_learn_triggers = spell_info
            .effects()
            .iter()
            .filter(|effect| {
                effect.effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_SPELL
                    && effect.effect_trigger_spell > 0
                    && !self
                        .known_spells_like_cpp()
                        .contains(&effect.effect_trigger_spell)
            })
            .map(|effect| effect.effect_trigger_spell)
            .collect::<Vec<_>>();

        if missing_learn_triggers.is_empty() || spell_info.effects().is_empty() {
            return Vec::new();
        }

        let learned_0 = spell_info.effects()[0].effect_trigger_spell;
        if learned_0 > 0
            && !self.known_spells_like_cpp().contains(&learned_0)
            && !self.skill_store().is_some_and(|store| {
                store
                    .get_skill_line_ability_map_bounds_like_cpp(learned_0)
                    .iter()
                    .any(|ability| {
                        ability.acquire_method
                            == wow_data::skill::SKILL_LINE_ABILITY_REWARDED_FROM_QUEST_LIKE_CPP
                    })
            })
        {
            return Vec::new();
        }

        missing_learn_triggers
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) fn represented_quest_rewarded_talent_points_like_cpp(
        &self,
    ) -> Option<u32> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            player.gameplay_state().quest_rewarded_talent_points
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(
                self.quest_test_fixture_like_cpp
                    .represented_quest_reward_talent_points_like_cpp
                    .iter()
                    .map(|reward| reward.points)
                    .sum(),
            );
        }
        canonical
    }
    pub(crate) fn add_represented_quest_reward_talent_points_like_cpp(
        &mut self,
        quest_id: u32,
        points: u32,
    ) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.add_quest_rewarded_talent_points_like_cpp(points);
            })
            .is_some();
        if canonical {
            return true;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.player_handle_like_cpp.is_none() {
            self.quest_test_fixture_like_cpp
                .represented_quest_reward_talent_points_like_cpp
                .push(RepresentedQuestRewardTalentPointsLikeCpp {
                    quest_id,
                    points,
                    init_talent_for_level_unrepresented: true,
                });
            return true;
        }
        let _ = quest_id;
        false
    }
}
