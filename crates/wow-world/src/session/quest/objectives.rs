//! Represented quest objective progress and credit.
//!
//! Moved out of the Session root under #605. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(in crate::session) fn represented_current_player_has_incomplete_quest_objective_for_object_id_like_cpp(
        &self,
        object_id: i32,
    ) -> bool {
        let Some(quest_store) = self.quests.store.as_ref() else {
            return false;
        };
        let Some(quests) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return false;
        };
        quests.statuses.values().any(|status| {
            if status.status != crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                return false;
            }
            let Some(quest) = quest_store.get(status.quest_id) else {
                return false;
            };
            quest
                .objectives
                .iter()
                .enumerate()
                .any(|(fallback_index, objective)| {
                    if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP
                        || objective.object_id != object_id
                    {
                        return false;
                    }
                    let storage_index = usize::try_from(objective.storage_index)
                        .ok()
                        .unwrap_or(fallback_index);
                    let current = status
                        .objective_counts
                        .get(storage_index)
                        .copied()
                        .unwrap_or(0);
                    current < objective.amount.max(1)
                })
        })
    }
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
    pub(in crate::session) async fn update_represented_storing_value_quest_objective_progress_like_cpp(
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
        let matching: Vec<(u32, usize, i32, u32)> = quests
            .statuses
            .values()
            .filter(|qs| qs.status == crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP)
            .flat_map(|qs| {
                let quest = store.get(qs.quest_id)?;
                Some(quest.objectives.iter().filter_map(move |obj| {
                    if obj.obj_type != objective_type || obj.object_id != object_id {
                        return None;
                    }
                    if objective_type == QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP
                        && (obj.flags & QUEST_OBJECTIVE_FLAG_KILL_PLAYERS_SAME_FACTION_LIKE_CPP)
                            != 0
                        && victim_team.is_some_and(|team| team != player_team)
                    {
                        return None;
                    }
                    let Ok(idx) = usize::try_from(obj.storage_index) else {
                        return None;
                    };
                    Some((qs.quest_id, idx, obj.amount, obj.id))
                }))
            })
            .flatten()
            .collect();

        let mut quests_to_complete = Vec::new();
        let mut quests_to_save = Vec::new();
        for (quest_id, obj_idx, required, objective_id) in matching {
            let Some(current) = self
                .mutate_player_quest_gameplay_like_cpp(|quests| {
                    let qs = quests.statuses.get_mut(&quest_id)?;
                    if qs.objective_counts.len() <= obj_idx {
                        qs.objective_counts.resize(obj_idx + 1, 0);
                    }
                    if add_count >= 0 && qs.objective_counts[obj_idx] >= required {
                        return None;
                    }
                    qs.objective_counts[obj_idx] = qs.objective_counts[obj_idx]
                        .saturating_add(add_count)
                        .clamp(0, required);
                    Some(qs.objective_counts[obj_idx])
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
                    let quest_already_rewarded = quests.rewarded_quest_ids.contains(&quest_id);
                    if quests.statuses.get(&quest_id).is_some_and(|status| {
                        Self::represented_can_complete_quest_after_objective_like_cpp(
                            status,
                            quest,
                            objective_id,
                            quest_already_rewarded,
                        )
                    }) {
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
                    == Some(Some(crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP))
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
    pub(in crate::session) async fn update_represented_storing_flag_quest_objective_progress_like_cpp(
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
        let matching: Vec<(u32, usize, u32)> = quests
            .statuses
            .values()
            .filter(|qs| qs.status == crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP)
            .flat_map(|qs| {
                let quest = store.get(qs.quest_id)?;
                Some(quest.objectives.iter().filter_map(move |obj| {
                    if obj.obj_type != objective_type
                        || obj.object_id != object_id
                        || !obj.is_storing_flag_like_cpp()
                    {
                        return None;
                    }
                    let Ok(idx) = usize::try_from(obj.storage_index) else {
                        return None;
                    };
                    Some((qs.quest_id, idx, obj.id))
                }))
            })
            .flatten()
            .collect();

        let mut quests_to_complete = Vec::new();
        let mut quests_to_save = Vec::new();
        for (quest_id, obj_idx, objective_id) in matching {
            let Some((objective_was_complete, objective_is_now_complete)) = self
                .mutate_player_quest_gameplay_like_cpp(|quests| {
                    let qs = quests.statuses.get_mut(&quest_id)?;
                    if qs.objective_counts.len() <= obj_idx {
                        qs.objective_counts.resize(obj_idx + 1, 0);
                    }
                    let before = qs.objective_counts[obj_idx] != 0;
                    qs.objective_counts[obj_idx] = i32::from(add_count > 0);
                    Some((before, qs.objective_counts[obj_idx] != 0))
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
                    let quest_already_rewarded = quests.rewarded_quest_ids.contains(&quest_id);
                    if quests.statuses.get(&quest_id).is_some_and(|status| {
                        Self::represented_can_complete_quest_after_objective_like_cpp(
                            status,
                            quest,
                            objective_id,
                            quest_already_rewarded,
                        )
                    }) {
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
                    == Some(Some(crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP))
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
    async fn update_represented_money_quest_objective_progress_like_cpp(
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
        let add_count = new_money_i64.saturating_sub(old_money);
        let Some(quests) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return;
        };
        let matching: Vec<(u32, u32, i32, bool, bool)> = quests
            .statuses
            .values()
            .filter(|qs| {
                qs.status == crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP
                    || qs.status == crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP
            })
            .flat_map(|qs| {
                let quest = store.get(qs.quest_id)?;
                Some(quest.objectives.iter().filter_map(move |obj| {
                    if obj.obj_type != QUEST_OBJECTIVE_MONEY_LIKE_CPP || obj.object_id != 0 {
                        return None;
                    }
                    let objective_was_complete = old_money >= i64::from(obj.amount);
                    if objective_was_complete && add_count >= 0 {
                        return None;
                    }
                    let objective_is_now_complete =
                        old_money.saturating_add(add_count) >= i64::from(obj.amount);
                    Some((
                        qs.quest_id,
                        obj.id,
                        obj.amount,
                        objective_was_complete,
                        objective_is_now_complete,
                    ))
                }))
            })
            .flatten()
            .collect();

        let mut quests_to_complete = Vec::new();
        let mut quests_to_save = Vec::new();
        for (quest_id, objective_id, required, objective_was_complete, objective_is_now_complete) in
            matching
        {
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
                    let quest_already_rewarded = quests.rewarded_quest_ids.contains(&quest_id);
                    if quests.statuses.get(&quest_id).is_some_and(|status| {
                        Self::represented_can_complete_quest_after_objective_like_cpp(
                            status,
                            quest,
                            objective_id,
                            quest_already_rewarded,
                        )
                    }) {
                        quests_to_complete.push((quest_id, objective_id));
                    }
                }
            } else if objective_was_complete {
                if self
                    .mutate_player_quest_gameplay_like_cpp(|quests| {
                        let Some(status) = quests.statuses.get_mut(&quest_id) else {
                            return false;
                        };
                        if status.status != crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP {
                            return false;
                        }
                        status.status = crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP;
                        true
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
                    == Some(Some(crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP))
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
    async fn update_represented_currency_quest_objective_progress_like_cpp(
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
        let add_count = i64::from(change);
        let object_id = i32::try_from(currency_id).unwrap_or(i32::MAX);
        let Some(quests) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return;
        };
        let matching: Vec<(u32, u32, i32, bool, bool)> = quests
            .statuses
            .values()
            .filter(|qs| {
                qs.status == crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP
                    || qs.status == crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP
            })
            .flat_map(|qs| {
                let quest = store.get(qs.quest_id)?;
                Some(quest.objectives.iter().filter_map(move |obj| {
                    if obj.obj_type != QUEST_OBJECTIVE_CURRENCY_LIKE_CPP
                        || obj.object_id != object_id
                    {
                        return None;
                    }
                    let objective_was_complete = current_quantity >= i64::from(obj.amount);
                    if objective_was_complete && change >= 0 {
                        return None;
                    }
                    let objective_is_now_complete =
                        current_quantity.saturating_add(add_count) >= i64::from(obj.amount);
                    Some((
                        qs.quest_id,
                        obj.id,
                        obj.amount,
                        objective_was_complete,
                        objective_is_now_complete,
                    ))
                }))
            })
            .flatten()
            .collect();

        let mut quests_to_complete = Vec::new();
        let mut quests_to_save = Vec::new();
        for (quest_id, objective_id, required, objective_was_complete, objective_is_now_complete) in
            matching
        {
            debug!(
                account = self.account_id,
                quest_id,
                currency_id,
                current_quantity,
                change,
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
                    let quest_already_rewarded = quests.rewarded_quest_ids.contains(&quest_id);
                    if quests.statuses.get(&quest_id).is_some_and(|status| {
                        Self::represented_can_complete_quest_after_objective_like_cpp(
                            status,
                            quest,
                            objective_id,
                            quest_already_rewarded,
                        )
                    }) {
                        quests_to_complete.push((quest_id, objective_id));
                    }
                }
            } else if objective_was_complete {
                if self
                    .mutate_player_quest_gameplay_like_cpp(|quests| {
                        let Some(status) = quests.statuses.get_mut(&quest_id) else {
                            return false;
                        };
                        if status.status != crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP {
                            return false;
                        }
                        status.status = crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP;
                        true
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
                    == Some(Some(crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP))
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
    async fn update_represented_reputation_quest_objective_progress_like_cpp(
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

        let Some(old_reputation) = self.with_reputation_mgr_like_cpp(|mgr| {
            mgr.reputation_for_faction_like_cpp(
                faction_entry,
                self.player_race_like_cpp(),
                self.player_class_like_cpp(),
            )
        }) else {
            return;
        };
        let new_reputation = old_reputation.saturating_add(change);
        let object_id = i32::try_from(faction_id).unwrap_or(i32::MAX);
        let Some(quests) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return;
        };
        let matching: Vec<(u32, u32, i32, bool, bool)> = quests
            .statuses
            .values()
            .filter(|qs| {
                qs.status == crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP
                    || qs.status == crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP
            })
            .flat_map(|qs| {
                let quest = store.get(qs.quest_id)?;
                Some(quest.objectives.iter().filter_map(move |obj| {
                    if obj.obj_type != objective_type || obj.object_id != object_id {
                        return None;
                    }
                    let (objective_was_complete, objective_is_now_complete) = match objective_type {
                        QUEST_OBJECTIVE_MIN_REPUTATION_LIKE_CPP => {
                            (old_reputation >= obj.amount, new_reputation >= obj.amount)
                        }
                        QUEST_OBJECTIVE_MAX_REPUTATION_LIKE_CPP => {
                            (old_reputation <= obj.amount, new_reputation <= obj.amount)
                        }
                        _ => return None,
                    };
                    if objective_was_complete && change >= 0 {
                        return None;
                    }
                    Some((
                        qs.quest_id,
                        obj.id,
                        obj.amount,
                        objective_was_complete,
                        objective_is_now_complete,
                    ))
                }))
            })
            .flatten()
            .collect();

        let mut quests_to_complete = Vec::new();
        let mut quests_to_save = Vec::new();
        for (quest_id, objective_id, required, objective_was_complete, objective_is_now_complete) in
            matching
        {
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
                    let quest_already_rewarded = quests.rewarded_quest_ids.contains(&quest_id);
                    if quests.statuses.get(&quest_id).is_some_and(|status| {
                        Self::represented_can_complete_quest_after_objective_like_cpp(
                            status,
                            quest,
                            objective_id,
                            quest_already_rewarded,
                        )
                    }) {
                        quests_to_complete.push((quest_id, objective_id));
                    }
                }
            } else if objective_was_complete {
                if self
                    .mutate_player_quest_gameplay_like_cpp(|quests| {
                        let Some(status) = quests.statuses.get_mut(&quest_id) else {
                            return false;
                        };
                        if status.status != crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP {
                            return false;
                        }
                        status.status = crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP;
                        true
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
                    == Some(Some(crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP))
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
    pub(crate) fn enqueue_represented_quest_objective_progress_like_cpp(
        &mut self,
        event: RepresentedQuestObjectiveProgressEventLikeCpp,
    ) {
        self.represented_quest_objective_progress_events_like_cpp
            .push_back(event);
    }
    pub(crate) async fn drain_represented_quest_objective_progress_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
    ) {
        if self.represented_quest_objective_progress_draining_like_cpp {
            return;
        }

        self.represented_quest_objective_progress_draining_like_cpp = true;
        while let Some(event) = self
            .represented_quest_objective_progress_events_like_cpp
            .pop_front()
        {
            match event {
                RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                    old_money,
                    new_money,
                } => {
                    self.update_represented_money_quest_objective_progress_like_cpp(
                        item_guid_generator,
                        old_money,
                        new_money,
                    )
                    .await;
                }
                RepresentedQuestObjectiveProgressEventLikeCpp::CurrencyChanged {
                    currency_id,
                    change,
                } => {
                    let object_id = i32::try_from(currency_id).unwrap_or(i32::MAX);
                    self.update_represented_currency_quest_objective_progress_like_cpp(
                        item_guid_generator,
                        currency_id,
                        change,
                    )
                    .await;
                    self.update_represented_storing_value_quest_objective_progress_like_cpp(
                        item_guid_generator,
                        QUEST_OBJECTIVE_HAVE_CURRENCY_LIKE_CPP,
                        object_id,
                        change,
                        wow_core::ObjectGuid::new(0, 0),
                    )
                    .await;
                    self.update_represented_storing_value_quest_objective_progress_like_cpp(
                        item_guid_generator,
                        QUEST_OBJECTIVE_OBTAIN_CURRENCY_LIKE_CPP,
                        object_id,
                        change,
                        wow_core::ObjectGuid::new(0, 0),
                    )
                    .await;
                }
                RepresentedQuestObjectiveProgressEventLikeCpp::ReputationChanged {
                    faction_id,
                    change,
                } => {
                    let object_id = i32::try_from(faction_id).unwrap_or(i32::MAX);
                    self.update_represented_reputation_quest_objective_progress_like_cpp(
                        item_guid_generator,
                        QUEST_OBJECTIVE_MIN_REPUTATION_LIKE_CPP,
                        faction_id,
                        change,
                    )
                    .await;
                    self.update_represented_reputation_quest_objective_progress_like_cpp(
                        item_guid_generator,
                        QUEST_OBJECTIVE_MAX_REPUTATION_LIKE_CPP,
                        faction_id,
                        change,
                    )
                    .await;
                    self.update_represented_storing_value_quest_objective_progress_like_cpp(
                        item_guid_generator,
                        QUEST_OBJECTIVE_INCREASE_REPUTATION_LIKE_CPP,
                        object_id,
                        change,
                        wow_core::ObjectGuid::new(0, 0),
                    )
                    .await;
                }
            }
        }
        self.represented_quest_objective_progress_draining_like_cpp = false;
    }
    #[cfg(test)]
    pub(crate) async fn drain_represented_quest_objective_progress_like_cpp(&mut self) {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return;
        };
        self.drain_represented_quest_objective_progress_with_generator_like_cpp(generator.as_ref())
            .await;
    }
    #[cfg(test)]
    pub(crate) fn represented_quest_push_result_sender_mismatch_count_like_cpp(&self) -> u32 {
        self.represented_quest_push_result_sender_mismatch_count_like_cpp
    }
}
