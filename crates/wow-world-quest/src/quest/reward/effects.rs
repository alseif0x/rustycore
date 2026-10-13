// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest reward side effects whose selected runtime owners remain external.

use super::QuestRewardCx;

impl QuestRewardCx<'_> {
    pub fn apply_represented_quest_reward_skill_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.world_test_consumer && quest.reward_skill_line_id != 0 {
            self.quest_state
                .fixture_record_quest_reward_skill_update_like_cpp(
                    quest.reward_skill_line_id,
                    quest.reward_skill_points,
                );
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = quest;
    }

    pub fn apply_represented_quest_title_and_talent_rewards_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.world_test_consumer && quest.reward_title_id != 0 {
            self.quest_state.fixture_record_quest_reward_title_like_cpp(
                super::super::RepresentedQuestRewardTitleLikeCpp {
                    quest_id: quest.id,
                    title_id: quest.reward_title_id,
                    char_title_lookup_unrepresented: true,
                    set_title_runtime_unrepresented: true,
                },
            );
        }

        if quest.reward_skill_points == 0 {
            return;
        }
        if self
            .player
            .add_quest_rewarded_talent_points_like_cpp(quest.reward_skill_points)
        {
            return;
        }

        #[cfg(any(test, feature = "test-fixtures"))]
        if self.world_test_consumer && self.player.owner_handle_absent_like_cpp() {
            self.quest_state
                .fixture_record_quest_reward_talent_points_like_cpp(
                    super::super::RepresentedQuestRewardTalentPointsLikeCpp {
                        quest_id: quest.id,
                        points: quest.reward_skill_points,
                        init_talent_for_level_unrepresented: true,
                    },
                );
        }
    }

    pub fn record_represented_quest_reward_mail_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
        quest_giver_guid: wow_core::ObjectGuid,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.world_test_consumer && quest.reward_mail_template_id != 0 {
            self.quest_state.fixture_record_quest_reward_mail_like_cpp(
                super::super::RepresentedQuestRewardMailLikeCpp {
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
                },
            );
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = (quest, quest_giver_guid);
    }

    pub fn record_represented_quest_reward_spell_casts_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
        #[cfg(any(test, feature = "test-fixtures"))]
        teleport_fixture: &wow_world_core::session::state::TeleportState,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.world_test_consumer {
            let caster_selection_unrepresented = (quest.flags
                & wow_constants::quest::QUEST_FLAGS_PLAYER_CAST_COMPLETE_LIKE_CPP)
                == 0;
            if quest.reward_spell > 0 {
                let can_delay_teleport_like_cpp = self
                    .player
                    .represented_can_delay_teleport_like_cpp(teleport_fixture);
                self.quest_state
                    .fixture_record_quest_reward_spell_cast_like_cpp(
                        super::super::RepresentedQuestRewardSpellCastLikeCpp {
                            quest_id: quest.id,
                            spell_id: quest.reward_spell,
                            kind: super::super::RepresentedQuestRewardSpellKindLikeCpp::RewardSpell,
                            can_delay_teleport_like_cpp,
                            spell_info_lookup_unrepresented: true,
                            caster_selection_unrepresented,
                            cast_spell_runtime_unrepresented: true,
                        },
                    );
                return;
            }

            for (index, spell_id) in quest.reward_display_spell.into_iter().enumerate() {
                if spell_id == 0 {
                    continue;
                }
                let can_delay_teleport_like_cpp = self
                    .player
                    .represented_can_delay_teleport_like_cpp(teleport_fixture);
                self.quest_state
                    .fixture_record_quest_reward_spell_cast_like_cpp(
                        super::super::RepresentedQuestRewardSpellCastLikeCpp {
                            quest_id: quest.id,
                            spell_id,
                            kind: super::super::RepresentedQuestRewardSpellKindLikeCpp::RewardDisplaySpell {
                                index: index as u8,
                            },
                            can_delay_teleport_like_cpp,
                            spell_info_lookup_unrepresented: true,
                            caster_selection_unrepresented,
                            cast_spell_runtime_unrepresented: true,
                        },
                    );
            }
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = quest;
    }
}
