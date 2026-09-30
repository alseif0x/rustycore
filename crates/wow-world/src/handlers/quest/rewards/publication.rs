//! publication operations at the existing Quest application boundary.

use super::*;

impl WorldSession {

    #[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_variables))]
    pub(super) fn apply_represented_quest_reward_skill_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        if quest.reward_skill_line_id != 0 {
            self.quest_test_fixture_like_cpp
                .represented_quest_reward_skill_updates_like_cpp
                .push((quest.reward_skill_line_id, quest.reward_skill_points));
        }
    }

    pub(super) fn record_represented_quest_reward_spell_casts_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            let caster_selection_unrepresented =
                (quest.flags & QUEST_FLAGS_PLAYER_CAST_COMPLETE_LIKE_CPP) == 0;
            if quest.reward_spell > 0 {
                self.quest_test_fixture_like_cpp
                    .represented_quest_reward_spell_casts_like_cpp
                    .push(RepresentedQuestRewardSpellCastLikeCpp {
                        quest_id: quest.id,
                        spell_id: quest.reward_spell,
                        kind: RepresentedQuestRewardSpellKindLikeCpp::RewardSpell,
                        can_delay_teleport_like_cpp: self.represented_can_delay_teleport_like_cpp(),
                        spell_info_lookup_unrepresented: true,
                        caster_selection_unrepresented,
                        cast_spell_runtime_unrepresented: true,
                    });
                return;
            }

            let display_spells = quest.reward_display_spell;
            for (index, spell_id) in display_spells.into_iter().enumerate() {
                if spell_id == 0 {
                    continue;
                }
                self.quest_test_fixture_like_cpp
                    .represented_quest_reward_spell_casts_like_cpp
                    .push(RepresentedQuestRewardSpellCastLikeCpp {
                        quest_id: quest.id,
                        spell_id,
                        kind: RepresentedQuestRewardSpellKindLikeCpp::RewardDisplaySpell {
                            index: index as u8,
                        },
                        can_delay_teleport_like_cpp: self.represented_can_delay_teleport_like_cpp(),
                        spell_info_lookup_unrepresented: true,
                        caster_selection_unrepresented,
                        cast_spell_runtime_unrepresented: true,
                    });
            }
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = quest;
    }

    pub(super) fn apply_represented_quest_title_and_talent_rewards_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        if quest.reward_title_id != 0 {
            self.quest_test_fixture_like_cpp
                .represented_quest_reward_titles_like_cpp
                .push(RepresentedQuestRewardTitleLikeCpp {
                    quest_id: quest.id,
                    title_id: quest.reward_title_id,
                    char_title_lookup_unrepresented: true,
                    set_title_runtime_unrepresented: true,
                });
        }
        if quest.reward_skill_points != 0 {
            let _ = self.add_represented_quest_reward_talent_points_like_cpp(
                quest.id,
                quest.reward_skill_points,
            );
        }
    }

    pub(super) fn record_represented_quest_reward_mail_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
        quest_giver_guid: ObjectGuid,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            if quest.reward_mail_template_id == 0 {
                return;
            }

            self.quest_test_fixture_like_cpp
                .represented_quest_reward_mails_like_cpp
                .push(RepresentedQuestRewardMailLikeCpp {
                    quest_id: quest.id,
                    mail_template_id: quest.reward_mail_template_id,
                    delay_secs: quest.reward_mail_delay_secs,
                    sender_entry: (quest.reward_mail_sender_entry != 0)
                        .then_some(quest.reward_mail_sender_entry),
                    quest_giver_guid: (quest.reward_mail_sender_entry == 0)
                        .then_some(quest_giver_guid),
                    mail_template_lookup_unrepresented: true,
                    mail_draft_runtime_unrepresented: true,
                    character_db_transaction_unrepresented: true,
                });
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = (quest, quest_giver_guid);
    }
}
