// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Recurrence projection for the closing quest-reward transaction.

use wow_world_core::session::QuestRewardPlayerAccessLikeCpp;

use super::super::QuestRewardDurablePlanLikeCpp;
use super::super::SessionQuestState;
use super::QuestRewardCx;

#[derive(Clone, Copy)]
enum RepresentedQuestRecurrenceLikeCpp {
    Daily {
        quest_id: u32,
        now_secs: i64,
        is_df_quest: bool,
    },
    Weekly {
        quest_id: u32,
    },
    Monthly {
        quest_id: u32,
    },
    Seasonal {
        event_id: u16,
        quest_id: u32,
        completed_at: u64,
    },
}

impl QuestRewardCx<'_> {
    pub fn apply_quest_reward_lockout_status_like_cpp(
        quest_state: &mut SessionQuestState,
        player: &mut QuestRewardPlayerAccessLikeCpp<'_>,
        plan: &mut QuestRewardDurablePlanLikeCpp,
        quest: &wow_data::quest::QuestTemplate,
        world_test_consumer: bool,
    ) {
        let now = wow_core::GameTime::now().as_secs() as i64;
        let Some(player_guid) = player.player_guid_like_cpp() else {
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
            && !Self::record_quest_recurrence_like_cpp(
                quest_state,
                player,
                recurrence,
                world_test_consumer,
            )
        {
            return;
        }

        let owner_guid = player_guid.counter() as u64;
        #[cfg(any(test, feature = "test-fixtures"))]
        let quest_gameplay = if world_test_consumer && player.owner_handle_absent_like_cpp() {
            Some(quest_state.player_quest_gameplay_fixture_like_cpp())
        } else {
            player.quest_gameplay_snapshot_like_cpp()
        };
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let quest_gameplay = player.quest_gameplay_snapshot_like_cpp();
        let Some(recurrence) = quest_gameplay else {
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

    fn record_quest_recurrence_like_cpp(
        quest_state: &mut SessionQuestState,
        player: &mut QuestRewardPlayerAccessLikeCpp<'_>,
        recurrence: RepresentedQuestRecurrenceLikeCpp,
        world_test_consumer: bool,
    ) -> bool {
        #[cfg(any(test, feature = "test-fixtures"))]
        if world_test_consumer && player.owner_handle_absent_like_cpp() {
            let mut state = quest_state.player_quest_gameplay_fixture_like_cpp();
            match recurrence {
                RepresentedQuestRecurrenceLikeCpp::Daily {
                    quest_id,
                    now_secs,
                    is_df_quest,
                } => {
                    state.set_last_daily_quest_time_secs_like_cpp(now_secs);
                    if is_df_quest {
                        state.set_df_quest_like_cpp(quest_id, true);
                    } else {
                        state.set_daily_like_cpp(quest_id, true);
                    }
                }
                RepresentedQuestRecurrenceLikeCpp::Weekly { quest_id } => {
                    state.set_weekly_like_cpp(quest_id, true);
                }
                RepresentedQuestRecurrenceLikeCpp::Monthly { quest_id } => {
                    state.set_monthly_like_cpp(quest_id, true);
                }
                RepresentedQuestRecurrenceLikeCpp::Seasonal {
                    event_id,
                    quest_id,
                    completed_at,
                } => state.set_seasonal_like_cpp(event_id, quest_id, completed_at),
            }
            quest_state.apply_player_quest_gameplay_fixture_like_cpp(state);
            return true;
        }

        let recorded = match recurrence {
            RepresentedQuestRecurrenceLikeCpp::Daily {
                quest_id,
                now_secs,
                is_df_quest,
            } => player.record_daily_quest_reward_recurrence_like_cpp(
                quest_id,
                now_secs,
                is_df_quest,
            ),
            RepresentedQuestRecurrenceLikeCpp::Weekly { quest_id } => {
                player.record_weekly_quest_reward_recurrence_like_cpp(quest_id)
            }
            RepresentedQuestRecurrenceLikeCpp::Monthly { quest_id } => {
                player.record_monthly_quest_reward_recurrence_like_cpp(quest_id)
            }
            RepresentedQuestRecurrenceLikeCpp::Seasonal {
                event_id,
                quest_id,
                completed_at,
            } => player.record_seasonal_quest_reward_recurrence_like_cpp(
                event_id,
                quest_id,
                completed_at,
            ),
        };
        if recorded {
            #[cfg(any(test, feature = "test-fixtures"))]
            if world_test_consumer
                && !player.owner_handle_absent_like_cpp()
                && let Some(state) = player.quest_gameplay_snapshot_like_cpp()
            {
                quest_state.apply_player_quest_core_compatibility_like_cpp(&state);
            }
            return true;
        }

        let _ = (quest_state, world_test_consumer);
        false
    }
}
