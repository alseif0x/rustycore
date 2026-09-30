//! lockouts operations at the existing Quest application boundary.

use super::*;

impl WorldSession {

    pub(super) fn apply_quest_reward_lockout_status_like_cpp(
        &mut self,
        plan: &mut QuestRewardDurablePlanLikeCpp,
        quest: &wow_data::quest::QuestTemplate,
    ) {
        let now = GameTime::now().as_secs() as i64;
        let Some(player_guid) = self.player_guid() else {
            return;
        };

        let mut save_daily = false;
        let mut save_weekly = false;
        let mut save_monthly = false;
        let mut save_seasonal = false;

        if quest.is_daily_like_cpp() || quest.is_df_quest_like_cpp() {
            save_daily = true;
        } else if quest.is_weekly_like_cpp() {
            save_weekly = true;
        } else if quest.is_monthly_like_cpp() {
            save_monthly = true;
        } else if quest.is_seasonal_like_cpp() {
            save_seasonal = true;
        }

        let recurrence = if save_daily {
            Some(RepresentedQuestRecurrenceLikeCpp::Daily {
                quest_id: quest.id,
                now_secs: now,
                is_df_quest: quest.is_df_quest_like_cpp(),
            })
        } else if save_weekly {
            Some(RepresentedQuestRecurrenceLikeCpp::Weekly { quest_id: quest.id })
        } else if save_monthly {
            Some(RepresentedQuestRecurrenceLikeCpp::Monthly { quest_id: quest.id })
        } else if save_seasonal {
            Some(RepresentedQuestRecurrenceLikeCpp::Seasonal {
                event_id: quest.event_id_for_quest_like_cpp(),
                quest_id: quest.id,
                completed_at: now.max(0) as u64,
            })
        } else {
            None
        };
        if let Some(recurrence) = recurrence
            && !self.record_represented_quest_recurrence_like_cpp(recurrence)
        {
            return;
        }

        let owner_guid = player_guid.counter() as u64;
        let Some(recurrence) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return;
        };
        let request = if save_daily {
            let mut quest_ids = recurrence
                .daily_quest_ids_like_cpp()
                .iter()
                .copied()
                .collect::<Vec<_>>();
            quest_ids.extend(recurrence.df_quest_ids_like_cpp().iter().copied());
            wow_persistence::PlayerQuestLockoutPersistenceRequestLikeCpp::Daily {
                owner_guid,
                completed_time: recurrence.last_daily_quest_time_secs_like_cpp(),
                quest_ids,
            }
        } else if save_weekly {
            wow_persistence::PlayerQuestLockoutPersistenceRequestLikeCpp::Weekly {
                owner_guid,
                quest_ids: recurrence
                    .weekly_quest_ids_like_cpp()
                    .iter()
                    .copied()
                    .collect(),
            }
        } else if save_monthly {
            wow_persistence::PlayerQuestLockoutPersistenceRequestLikeCpp::Monthly {
                owner_guid,
                quest_ids: recurrence
                    .monthly_quest_ids_like_cpp()
                    .iter()
                    .copied()
                    .collect(),
            }
        } else if save_seasonal {
            let completions = recurrence
                .seasonal_quests_like_cpp()
                .iter()
                .flat_map(|(event_id, quests)| {
                    quests.iter().filter_map(|(quest_id, completed_time)| {
                        Some(
                            wow_persistence::PlayerQuestSeasonalCompletionPersistenceLikeCpp {
                                quest_id: *quest_id,
                                event_id: *event_id,
                                completed_time: i64::try_from(*completed_time).ok()?,
                            },
                        )
                    })
                })
                .collect();
            wow_persistence::PlayerQuestLockoutPersistenceRequestLikeCpp::Seasonal {
                owner_guid,
                completions,
            }
        } else {
            return;
        };

        // `_SaveDailyQuestStatus` and its siblings belong to the same closing
        // transaction as the rest of the reward (Player.cpp:19634..19638).
        plan.push_lockout(request);
    }
}
