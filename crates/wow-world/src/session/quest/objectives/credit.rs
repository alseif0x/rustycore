//! credit operations at the existing Quest application boundary.

use super::*;

impl WorldSession {
    pub(crate) fn represented_quest_can_increase_rewarded_counters_like_cpp(
        &self,
        quest_id: u32,
    ) -> Option<bool> {
        self.quests.store.as_ref()?.get(quest_id).map(|quest| {
            !quest.is_df_quest_like_cpp()
                && !quest.is_daily_like_cpp()
                && (!quest.is_repeatable()
                    || quest.is_weekly_like_cpp()
                    || quest.is_monthly_like_cpp()
                    || quest.is_seasonal_like_cpp())
        })
    }
    pub(crate) async fn update_represented_storing_value_quest_objective_progress_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        objective_type: u8,
        object_id: i32,
        add_count: i32,
        credit_guid: wow_core::ObjectGuid,
    ) {
        self.invalidate_player_quest_status_authority_like_cpp();
        use wow_packet::packets::quest::{
            QuestUpdateAddCredit, QuestUpdateAddPvpCredit, QuestUpdateComplete,
        };

        let Some(store) = self.quests.store.clone() else {
            return;
        };

        let victim_team =
            if objective_type == QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP && !credit_guid.is_empty() {
                self.player_registry
                    .as_ref()
                    .and_then(|registry| registry.quest_credit_race(credit_guid))
                    .map(player_team_for_race_cpp)
            } else {
                None
            };
        let player_team = player_team_for_race_cpp(self.player_race_like_cpp());
        let Some(quests) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return;
        };
        let matching = quests.plan_value_objective_credits(
            |id| store.get(id).map(|quest| quest.objective_rules_like_cpp()),
            objective_type,
            object_id,
            victim_team.map(|team| team == player_team),
        );

        let mut quests_to_complete = Vec::new();
        let mut quests_to_save = Vec::new();
        for (quest_id, obj_idx, required, objective_id) in matching {
            let Some(current) = self
                .mutate_player_quest_gameplay_like_cpp(|quests| {
                    quests.apply_value_objective_credit(quest_id, obj_idx, required, add_count)
                })
                .flatten()
            else {
                continue;
            };
            quests_to_save.push(quest_id);

            debug!(
                account = self.account_id,
                quest_id,
                obj_idx,
                current,
                required,
                objective_type,
                object_id,
                "Quest objective progress"
            );

            if add_count > 0 {
                if objective_type == QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP {
                    self.send_packet(&QuestUpdateAddPvpCredit {
                        quest_id,
                        count: current as u16,
                    });
                } else {
                    self.send_packet(&QuestUpdateAddCredit {
                        victim_guid: credit_guid,
                        quest_id,
                        object_id,
                        count: current as u16,
                        required: required as u16,
                        objective_type,
                    });
                }
            }

            if current >= required {
                if let (Some(quest), Some(quests)) = (
                    store.get(quest_id),
                    self.player_quest_gameplay_snapshot_like_cpp(),
                ) {
                    if quests.can_complete_after_objective(
                        quest_id,
                        objective_id,
                        || quest.objective_rules_like_cpp(),
                    )
                    {
                        quests_to_complete.push(quest_id);
                    }
                }
            }
        }

        for quest_id in quests_to_complete {
            let Some(quest) = store.get(quest_id).cloned() else {
                continue;
            };
            let completed = self
                .complete_represented_quest_after_add_with_generator_like_cpp(
                    item_guid_generator,
                    &quest,
                )
                .await;
            if completed {
                if self.represented_player_quest_status_like_cpp(quest_id)
                    == Some(Some(wow_conditions::QUEST_STATUS_COMPLETE_LIKE_CPP))
                {
                    self.send_packet(&QuestUpdateComplete { quest_id });
                }
                info!(
                    account = self.account_id,
                    quest_id, "Quest objectives complete"
                );
            }
        }
        self.save_changed_represented_quest_statuses_like_cpp(&mut quests_to_save)
            .await;
        self.sync_player_registry_state_like_cpp();
    }
    pub(crate) async fn update_represented_storing_flag_quest_objective_progress_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        objective_type: u8,
        object_id: i32,
        add_count: i32,
    ) {
        self.invalidate_player_quest_status_authority_like_cpp();
        use wow_packet::packets::quest::{QuestUpdateAddCreditSimple, QuestUpdateComplete};

        let Some(store) = self.quests.store.clone() else {
            return;
        };

        let Some(quests) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return;
        };
        let matching = quests.plan_flag_objective_credits(
            |id| store.get(id).map(|quest| quest.objective_rules_like_cpp()),
            objective_type,
            object_id,
        );

        let mut quests_to_complete = Vec::new();
        let mut quests_to_save = Vec::new();
        for (quest_id, obj_idx, objective_id) in matching {
            let Some((objective_was_complete, objective_is_now_complete)) = self
                .mutate_player_quest_gameplay_like_cpp(|quests| {
                    quests.apply_flag_objective_credit(quest_id, obj_idx, add_count)
                })
                .flatten()
            else {
                continue;
            };
            if objective_was_complete != objective_is_now_complete {
                quests_to_save.push(quest_id);
            }

            if add_count > 0 {
                self.send_packet(&QuestUpdateAddCreditSimple {
                    quest_id,
                    object_id,
                    objective_type,
                });
            }

            if !objective_was_complete && objective_is_now_complete {
                if let (Some(quest), Some(quests)) = (
                    store.get(quest_id),
                    self.player_quest_gameplay_snapshot_like_cpp(),
                ) {
                    if quests.can_complete_after_objective(
                        quest_id,
                        objective_id,
                        || quest.objective_rules_like_cpp(),
                    )
                    {
                        quests_to_complete.push((quest_id, objective_id));
                    }
                }
            }
        }

        for (quest_id, objective_id) in quests_to_complete {
            let Some(quest) = store.get(quest_id).cloned() else {
                continue;
            };
            let completed = self
                .complete_represented_quest_after_objective_with_generator_like_cpp(
                    item_guid_generator,
                    &quest,
                    objective_id,
                )
                .await;
            if completed {
                if self.represented_player_quest_status_like_cpp(quest_id)
                    == Some(Some(wow_conditions::QUEST_STATUS_COMPLETE_LIKE_CPP))
                {
                    self.send_packet(&QuestUpdateComplete { quest_id });
                }
                info!(
                    account = self.account_id,
                    quest_id, "Quest objectives complete"
                );
            }
        }
        self.save_changed_represented_quest_statuses_like_cpp(&mut quests_to_save)
            .await;
        self.sync_player_registry_state_like_cpp();
    }
}
