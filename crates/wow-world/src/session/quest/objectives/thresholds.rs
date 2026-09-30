//! thresholds operations at the existing Quest application boundary.

use super::*;

impl WorldSession {
    pub(crate) async fn update_represented_money_quest_objective_progress_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        old_money: u64,
        new_money: u64,
    ) {
        self.invalidate_player_quest_status_authority_like_cpp();
        use wow_packet::packets::quest::QuestUpdateComplete;

        let Some(store) = self.quests.store.clone() else {
            return;
        };

        let old_money = old_money.min(i64::MAX as u64) as i64;
        let new_money_i64 = new_money.min(i64::MAX as u64) as i64;
        let Some(quests) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return;
        };
        let matching = wow_entities::plan_threshold_quest_objective_changes_like_cpp(
            quests.statuses_like_cpp(),
            |id| store.get(id).map(|quest| quest.objective_rules_like_cpp()),
            QUEST_OBJECTIVE_MONEY_LIKE_CPP,
            0,
            old_money,
            new_money_i64,
            false,
        );

        let mut quests_to_complete = Vec::new();
        let mut quests_to_save = Vec::new();
        for change in matching {
            let quest_id = change.quest_id;
            let objective_id = change.objective_id;
            let required = change.required;
            let objective_was_complete = change.objective_was_complete;
            let objective_is_now_complete = change.objective_is_now_complete;
            debug!(
                account = self.account_id,
                quest_id,
                old_money,
                new_money = new_money_i64,
                required,
                objective_was_complete,
                objective_is_now_complete,
                "Quest money objective progress"
            );

            if objective_is_now_complete {
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
            } else if objective_was_complete {
                if self
                    .mutate_player_quest_gameplay_like_cpp(|quests| {
                        quests.reopen_quest_after_threshold_loss(quest_id)
                    })
                    .unwrap_or(false)
                {
                    quests_to_save.push(quest_id);
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
                quests_to_save.push(quest_id);
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
    pub(crate) async fn update_represented_currency_quest_objective_progress_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        currency_id: u32,
        change: i32,
    ) {
        self.invalidate_player_quest_status_authority_like_cpp();
        use wow_packet::packets::quest::QuestUpdateComplete;

        let Some(store) = self.quests.store.clone() else {
            return;
        };

        let Some(current_quantity) = self.player_currency_quantity(currency_id).map(i64::from)
        else {
            return;
        };
        let object_id = i32::try_from(currency_id).unwrap_or(i32::MAX);
        let Some(quests) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return;
        };
        let matching = wow_entities::plan_threshold_quest_objective_changes_like_cpp(
            quests.statuses_like_cpp(),
            |id| store.get(id).map(|quest| quest.objective_rules_like_cpp()),
            QUEST_OBJECTIVE_CURRENCY_LIKE_CPP,
            object_id,
            current_quantity,
            current_quantity.saturating_add(i64::from(change)),
            false,
        );

        let mut quests_to_complete = Vec::new();
        let mut quests_to_save = Vec::new();
        for change in matching {
            let quest_id = change.quest_id;
            let objective_id = change.objective_id;
            let required = change.required;
            let objective_was_complete = change.objective_was_complete;
            let objective_is_now_complete = change.objective_is_now_complete;
            debug!(
                account = self.account_id,
                quest_id,
                currency_id,
                current_quantity,
                change = ?change,
                required,
                objective_was_complete,
                objective_is_now_complete,
                "Quest currency objective progress"
            );

            if objective_is_now_complete {
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
            } else if objective_was_complete {
                if self
                    .mutate_player_quest_gameplay_like_cpp(|quests| {
                        quests.reopen_quest_after_threshold_loss(quest_id)
                    })
                    .unwrap_or(false)
                {
                    quests_to_save.push(quest_id);
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
                quests_to_save.push(quest_id);
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
    pub(crate) async fn update_represented_reputation_quest_objective_progress_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        objective_type: u8,
        faction_id: u32,
        change: i32,
    ) {
        self.invalidate_player_quest_status_authority_like_cpp();
        use wow_packet::packets::quest::QuestUpdateComplete;

        let Some(store) = self.quests.store.clone() else {
            return;
        };
        let Some(faction_store) = self.factions.store.as_ref() else {
            return;
        };
        let Some(faction_entry) = faction_store.get(faction_id) else {
            return;
        };

        // Resolve the player identity before taking the canonical manager lock:
        // the session accessors re-enter it and would self-deadlock.
        let player_race = self.player_race_like_cpp();
        let player_class = self.player_class_like_cpp();
        let Some(old_reputation) = self.with_reputation_mgr_like_cpp(|mgr| {
            mgr.reputation_for_faction_like_cpp(faction_entry, player_race, player_class)
        }) else {
            return;
        };
        let new_reputation = old_reputation.saturating_add(change);
        let object_id = i32::try_from(faction_id).unwrap_or(i32::MAX);
        let Some(quests) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return;
        };
        let matching = wow_entities::plan_threshold_quest_objective_changes_like_cpp(
            quests.statuses_like_cpp(),
            |id| store.get(id).map(|quest| quest.objective_rules_like_cpp()),
            objective_type,
            object_id,
            i64::from(old_reputation),
            i64::from(new_reputation),
            objective_type == QUEST_OBJECTIVE_MAX_REPUTATION_LIKE_CPP,
        );

        let mut quests_to_complete = Vec::new();
        let mut quests_to_save = Vec::new();
        for change in matching {
            let quest_id = change.quest_id;
            let objective_id = change.objective_id;
            let required = change.required;
            let objective_was_complete = change.objective_was_complete;
            let objective_is_now_complete = change.objective_is_now_complete;
            debug!(
                account = self.account_id,
                quest_id,
                objective_type,
                faction_id,
                old_reputation,
                new_reputation,
                required,
                objective_was_complete,
                objective_is_now_complete,
                "Quest reputation objective progress"
            );

            if objective_is_now_complete {
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
            } else if objective_was_complete {
                if self
                    .mutate_player_quest_gameplay_like_cpp(|quests| {
                        quests.reopen_quest_after_threshold_loss(quest_id)
                    })
                    .unwrap_or(false)
                {
                    quests_to_save.push(quest_id);
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
                quests_to_save.push(quest_id);
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
