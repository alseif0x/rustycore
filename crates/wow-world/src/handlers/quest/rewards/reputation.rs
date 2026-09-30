//! reputation operations at the existing Quest application boundary.

use super::*;

impl WorldSession {

    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_quest_reward_reputation_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) {
        let source = wow_progression::QuestReputationSource::from_flags(
            quest.is_daily_like_cpp(),
            quest.is_weekly_like_cpp(),
            quest.is_monthly_like_cpp(),
            quest.is_repeatable(),
        );
        let gain_source = match source {
            RepresentedQuestRewardReputationSourceLikeCpp::Quest => {
                ReputationGainSourceLikeCpp::Quest
            }
            RepresentedQuestRewardReputationSourceLikeCpp::DailyQuest => {
                ReputationGainSourceLikeCpp::DailyQuest
            }
            RepresentedQuestRewardReputationSourceLikeCpp::WeeklyQuest => {
                ReputationGainSourceLikeCpp::WeeklyQuest
            }
            RepresentedQuestRewardReputationSourceLikeCpp::MonthlyQuest => {
                ReputationGainSourceLikeCpp::MonthlyQuest
            }
            RepresentedQuestRewardReputationSourceLikeCpp::RepeatableQuest => {
                ReputationGainSourceLikeCpp::RepeatableQuest
            }
        };
        let faction_store = self.faction_store().map(Arc::clone);
        let quest_faction_reward_store = self.quests.faction_reward_store.as_ref().map(Arc::clone);
        let reputation_reward_rate_store = self.reputation_reward_rate_store().map(Arc::clone);
        let reputation_spillover_template_store =
            self.reputation_spillover_template_store().map(Arc::clone);
        let friendship_rep_reaction_store = self.friendship_rep_reaction_store().map(Arc::clone);
        let paragon_reputation_store = self.paragon_reputation_store().map(Arc::clone);
        let currency_types_store = self.currency_types_store().map(Arc::clone);

        for slot in 0..wow_data::quest::QUEST_REWARD_REPUTATIONS_COUNT {
            let faction_id = quest.reward_faction_ids[slot];
            if faction_id == 0 {
                continue;
            }
            let faction_entry = match faction_store.as_deref() {
                Some(store) => match store.get(faction_id).cloned() {
                    Some(entry) => Some(entry),
                    None => continue,
                },
                None => None,
            };
            let faction_lookup_missing = faction_entry.is_none();

            let reward_faction_override = quest.reward_faction_overrides[slot];
            let Some(reward) = wow_progression::calculate_quest_reputation_reward(
                wow_progression::QuestReputationSlotRules {
                    reward_value: quest.reward_faction_values[slot],
                    reward_override: reward_faction_override,
                    rank_cap: quest.reward_faction_cap_in[slot],
                    spillover_mask: quest.reward_faction_flags,
                    slot,
                },
                |row| quest_faction_reward_store.as_deref().map(|store| {
                    store.get(row).map(|entry| &entry.difficulty)
                }),
                |base_reputation_before_gain, no_quest_bonus| {
                    let quest_level_for_gain =
                        player_quest_level_like_cpp(quest, self.player_level_like_cpp()).max(0) as u32;
                    let reputation_rates = self.reputation_rates_like_cpp();
                    self.reputation_gain_percent_before_reward_rate_like_cpp(
                        gain_source,
                        quest_level_for_gain,
                        base_reputation_before_gain,
                        faction_id,
                        no_quest_bonus,
                    ).map(|percent| (percent, reputation_rates.gain))
                },
                || reputation_reward_rate_store.as_ref().map(|_| {
                    self.reputation_reward_rate_for_source_like_cpp(gain_source, faction_id)
                }),
                |percent| self.apply_recruit_a_friend_reputation_bonus_like_cpp(gain_source, percent),
                || self.canonical_player_reputation_standing_like_cpp(faction_id)
                    .map(reputation_rank_from_standing_like_cpp),
            ) else {
                continue;
            };

            let wow_progression::QuestReputationReward {
                base_reputation: base_reputation_before_gain,
                after_low_level: reputation_after_low_level_rate_like_cpp,
                after_reward_rate: reputation_after_reward_rate_like_cpp,
                gain: reputation_after_recruit_a_friend_bonus_like_cpp,
                gain_rate,
                no_quest_bonus,
                no_spillover,
                reward_table_unavailable: quest_faction_reward_lookup,
                reward_rate_unavailable: reputation_reward_rate_lookup,
                rank_cap_unresolved,
            } = reward;
            let modify_reputation_runtime_unrepresented =
                if let (Some(faction_entry), Some(faction_store)) =
                    (faction_entry.as_ref(), faction_store.as_deref())
                {
                    let options = wow_progression::mgr::SetReputationOptionsLikeCpp {
                        incremental: true,
                        spillover_only: false,
                        no_spillover,
                        reputation_gain_rate: gain_rate,
                        paragon_reward_quest_status_none_like_cpp: true,
                        renown_current_level_like_cpp: 0,
                        renown_currency_increased_cap_quantity_like_cpp: 0,
                        player_race: self.player_race_like_cpp(),
                        player_class: self.player_class_like_cpp(),
                    };
                    let db_spillover_template = reputation_spillover_template_store
                        .as_deref()
                        .and_then(|store| store.get(faction_id));
                    let catalogs =
                        crate::reputation_catalog_adapter::ReputationCatalogViewLikeCpp::new(
                            Some(faction_store.as_ref()),
                            friendship_rep_reaction_store.as_deref(),
                            paragon_reputation_store.as_deref(),
                            currency_types_store.as_deref(),
                        );
                    let mutation = self.mutate_reputation_mgr_like_cpp(|mgr| {
                        let outcome = mgr.set_reputation_like_cpp(
                            faction_entry,
                            reputation_after_recruit_a_friend_bonus_like_cpp,
                            options,
                            &catalogs,
                            db_spillover_template,
                        );
                        let update = outcome.send_state_rep_list_id.map(|rep_list_id| {
                            mgr.faction_standing_update_like_cpp(Some(rep_list_id))
                        });
                        (outcome, update)
                    });
                    let owner_unavailable = mutation.is_none();
                    if let Some((_outcome, Some(update))) = mutation {
                        let packet =
                            crate::handlers::progression::presentation::set_faction_standing_packet_like_cpp(
                                update,
                            );
                        self.send_packet(&packet);
                    }
                    owner_unavailable
                } else {
                    true
                };

            #[cfg(any(test, feature = "test-fixtures"))]
            {
                self.quest_test_fixture_like_cpp
                    .represented_quest_reward_reputations_like_cpp
                    .push(RepresentedQuestRewardReputationLikeCpp {
                        quest_id: quest.id,
                        slot: slot as u8,
                        faction_id,
                        reward_faction_value: quest.reward_faction_values[slot],
                        reward_faction_override,
                        reward_faction_cap_in: quest.reward_faction_cap_in[slot],
                        base_reputation_before_gain,
                        reputation_after_low_level_rate_like_cpp,
                        reputation_after_reward_rate_like_cpp,
                        no_quest_bonus,
                        no_spillover,
                        source,
                        faction_store_lookup_unrepresented: faction_lookup_missing,
                        quest_faction_reward_store_lookup_unrepresented:
                            quest_faction_reward_lookup,
                        reputation_reward_rate_lookup_unrepresented: reputation_reward_rate_lookup,
                        gray_level_script_hook_unrepresented: true,
                        reputation_rank_cap_check_unrepresented: rank_cap_unresolved,
                        calculate_reputation_gain_unrepresented: true,
                        modify_reputation_runtime_unrepresented,
                    });
            }
        }
    }
}
