// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest-related persistence planning for item mutations.

use super::*;

impl WorldSession {
    /// Pure post-move quest snapshot used to persist the item move and its
    /// `ItemAddedQuestCheck` / `ItemRemovedQuestCheck` result atomically.
    pub(crate) fn plan_bank_item_quest_persistence_like_cpp(
        &self,
        entry_id: u32,
        quest_log_item_id: u32,
        moving_to_bank: bool,
        post_move_non_bank_count: u32,
        added_count: u32,
    ) -> Vec<PlayerQuestStatus> {
        let Some(quest_store) = self.quests.store.as_ref() else {
            return Vec::new();
        };
        let Ok(entry_object_id) = i32::try_from(entry_id) else {
            return Vec::new();
        };
        let mut planned = Vec::new();
        let Some(state) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return planned;
        };

        if moving_to_bank {
            let new_item_count = i32::try_from(post_move_non_bank_count).unwrap_or(i32::MAX);
            for current_status in state.statuses_like_cpp().values() {
                let Some(quest) = quest_store.get(current_status.quest_id) else {
                    continue;
                };
                let mut status = current_status.clone();
                let mut changed = false;
                for (objective_index, objective) in quest.objectives.iter().enumerate() {
                    if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL
                        || objective.object_id != entry_object_id
                        || !wow_entities::represented_quest_objective_completable_like_cpp(
                            &status,
                            &quest.objective_rules_like_cpp(),
                            objective_index,
                        )
                    {
                        continue;
                    }
                    let Ok(storage_index) = usize::try_from(objective.storage_index) else {
                        continue;
                    };
                    if new_item_count >= objective.amount {
                        continue;
                    }
                    if status.objective_counts.len() <= storage_index {
                        status.objective_counts.resize(storage_index + 1, 0);
                    }
                    if status.objective_counts[storage_index] != new_item_count.max(0)
                        || status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP
                    {
                        status.objective_counts[storage_index] = new_item_count.max(0);
                        status.status = QUEST_STATUS_INCOMPLETE_LIKE_CPP;
                        changed = true;
                    }
                }
                if changed {
                    planned.push(status);
                }
            }
            return planned;
        }

        let mut matching_entry_objectives = Vec::new();
        'matching_entry: for status in state.statuses_like_cpp().values() {
            if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                continue;
            }
            let Some(quest) = quest_store.get(status.quest_id) else {
                continue;
            };
            for (objective_index, objective) in quest.objectives.iter().enumerate() {
                let Ok(storage_index) = usize::try_from(objective.storage_index) else {
                    continue;
                };
                let current = status
                    .objective_counts
                    .get(storage_index)
                    .copied()
                    .unwrap_or(0);
                if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL
                    || objective.object_id != entry_object_id
                    || current >= objective.amount
                    || !wow_entities::represented_quest_objective_completable_like_cpp(
                        status,
                        &quest.objective_rules_like_cpp(),
                        objective_index,
                    )
                {
                    continue;
                }
                let is_bound = (objective.flags2
                    & QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL)
                    != 0;
                matching_entry_objectives.push(is_bound);
                if is_bound {
                    break 'matching_entry;
                }
            }
        }
        let mut objective_ids = vec![entry_object_id];
        if quest_log_item_id != 0
            && (matching_entry_objectives.len() != 1 || !matching_entry_objectives[0])
        {
            objective_ids.push(i32::try_from(quest_log_item_id).unwrap_or(i32::MAX));
        }
        let added_count = i32::try_from(added_count).unwrap_or(i32::MAX);

        for current_status in state.statuses_like_cpp().values() {
            if current_status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                continue;
            }
            let Some(quest) = quest_store.get(current_status.quest_id) else {
                continue;
            };
            let mut status = current_status.clone();
            let mut completed_objective_ids = Vec::new();
            let mut changed = false;
            let mut stop_after_status = false;
            for (objective_index, objective) in quest.objectives.iter().enumerate() {
                if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL
                    || !objective_ids.contains(&objective.object_id)
                    || !wow_entities::represented_quest_objective_completable_like_cpp(
                        &status,
                        &quest.objective_rules_like_cpp(),
                        objective_index,
                    )
                {
                    continue;
                }
                let Ok(storage_index) = usize::try_from(objective.storage_index) else {
                    continue;
                };
                if status.objective_counts.len() <= storage_index {
                    status.objective_counts.resize(storage_index + 1, 0);
                }
                let current = status.objective_counts[storage_index];
                if current >= objective.amount {
                    continue;
                }
                let new_count = current
                    .saturating_add(added_count)
                    .clamp(0, objective.amount);
                status.objective_counts[storage_index] = new_count;
                changed = true;
                if new_count >= objective.amount {
                    completed_objective_ids.push(objective.id);
                }
                if (objective.flags2 & QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL) != 0
                {
                    stop_after_status = true;
                    break;
                }
            }
            let quest_already_rewarded = state
                .rewarded_quest_ids_like_cpp()
                .contains(&status.quest_id);
            if completed_objective_ids.iter().any(|objective_id| {
                wow_entities::represented_can_complete_quest_after_objective_like_cpp(
                    &status,
                    &quest.objective_rules_like_cpp(),
                    *objective_id,
                    quest_already_rewarded,
                )
            }) {
                status.status = QUEST_STATUS_COMPLETE_LIKE_CPP;
            }
            if changed {
                planned.push(status);
            }
            if stop_after_status {
                break;
            }
        }
        planned
    }

    /// Pure aggregate form of C++ `Player::ItemRemovedQuestCheck` for a set
    /// of removals that must commit in the same transaction as their items.
    pub(crate) fn begin_item_transfer_quest_persistence_like_cpp(
        &self,
        removed_entries_in_order: &[u32],
        post_removal_non_bank_counts: &[(u32, u32)],
    ) -> ItemTransferQuestPersistencePlanLikeCpp {
        let mut plan = ItemTransferQuestPersistencePlanLikeCpp {
            statuses: self
                .player_quest_gameplay_snapshot_like_cpp()
                .map(|state| state.statuses_snapshot_like_cpp().into_iter().collect())
                .unwrap_or_default(),
            changed_quest_ids: Vec::new(),
        };
        let Some(quest_store) = self.quests.store.as_ref() else {
            return plan;
        };
        let post_removal_counts = post_removal_non_bank_counts
            .iter()
            .copied()
            .collect::<HashMap<_, _>>();
        for &entry_id in removed_entries_in_order {
            let Some(&new_non_bank_item_count) = post_removal_counts.get(&entry_id) else {
                continue;
            };
            plan.changed_quest_ids.extend(
                wow_entities::apply_quest_item_removed_to_statuses_like_cpp(
                    |id| {
                        quest_store
                            .get(id)
                            .map(|quest| quest.objective_rules_like_cpp())
                    },
                    &mut plan.statuses,
                    entry_id,
                    new_non_bank_item_count,
                ),
            );
        }
        plan
    }

    pub(crate) fn plan_item_transfer_withdrawal_quest_persistence_like_cpp(
        &self,
        plan: &mut ItemTransferQuestPersistencePlanLikeCpp,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> bool {
        let Some(quest_store) = self.quests.store.as_ref() else {
            return false;
        };
        let Some(state) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return false;
        };
        let rewarded: HashSet<u32> = state
            .rewarded_quest_ids_like_cpp()
            .iter()
            .copied()
            .collect();
        if let Some((quest_id, _)) = wow_entities::apply_quest_item_added_bound_to_statuses_like_cpp(
            |id| {
                quest_store
                    .get(id)
                    .map(|quest| quest.objective_rules_like_cpp())
            },
            &rewarded,
            &mut plan.statuses,
            entry_id,
            quest_log_item_id,
            count,
        ) {
            plan.changed_quest_ids.push(quest_id);
            return true;
        }
        plan.changed_quest_ids.extend(
            wow_entities::apply_quest_item_added_non_bound_to_statuses_like_cpp(
                |id| {
                    quest_store
                        .get(id)
                        .map(|quest| quest.objective_rules_like_cpp())
                },
                &rewarded,
                &mut plan.statuses,
                entry_id,
                quest_log_item_id,
                count,
            ),
        );
        false
    }

    pub(crate) fn finish_item_transfer_quest_persistence_like_cpp(
        &self,
        mut plan: ItemTransferQuestPersistencePlanLikeCpp,
    ) -> Vec<PlayerQuestStatus> {
        plan.changed_quest_ids.sort_unstable();
        plan.changed_quest_ids.dedup();
        plan.changed_quest_ids
            .into_iter()
            .filter_map(|quest_id| plan.statuses.remove(&quest_id))
            .collect()
    }

    pub(crate) fn void_storage_quest_status_writes_like_cpp(
        &self,
        statuses: &[PlayerQuestStatus],
    ) -> Vec<wow_persistence::VoidStorageQuestStatusWriteLikeCpp> {
        statuses
            .iter()
            .map(|status| {
                let store = self.quests.store.as_ref();
                let objectives = store
                    .and_then(|store| store.get(status.quest_id))
                    .into_iter()
                    .flat_map(|quest| quest.objectives.iter())
                    .filter_map(|objective| {
                        let storage_index = u8::try_from(objective.storage_index).ok()?;
                        let count = status
                            .objective_counts
                            .get(usize::from(storage_index))
                            .copied()
                            .unwrap_or(0);
                        (count != 0).then_some(
                            wow_persistence::VoidStorageQuestObjectiveWriteLikeCpp {
                                storage_index,
                                count,
                            },
                        )
                    })
                    .collect();
                wow_persistence::VoidStorageQuestStatusWriteLikeCpp {
                    quest_id: status.quest_id,
                    status: status.status,
                    explored: status.explored,
                    accept_time_secs: status.accept_time_secs,
                    end_time_secs: status.end_time_secs,
                    objectives,
                }
            })
            .collect()
    }

    pub(crate) fn plan_item_transfer_quest_persistence_like_cpp(
        &self,
        removed_entries_in_order: &[u32],
        post_removal_non_bank_counts: &[(u32, u32)],
        added_items_in_order: &[(u32, u32, u32)],
    ) -> Vec<PlayerQuestStatus> {
        let mut plan = self.begin_item_transfer_quest_persistence_like_cpp(
            removed_entries_in_order,
            post_removal_non_bank_counts,
        );
        for &(entry_id, quest_log_item_id, count) in added_items_in_order {
            let _ = self.plan_item_transfer_withdrawal_quest_persistence_like_cpp(
                &mut plan,
                entry_id,
                quest_log_item_id,
                count,
            );
        }
        self.finish_item_transfer_quest_persistence_like_cpp(plan)
    }

    /// Pure form of the first C++ `Player::StoreNewItem` quest pass:
    /// `ItemAddedQuestCheck(itemId, count, true, &hadBoundItemObjective)`.
    ///
    /// `UpdateQuestObjectiveProgress` stops after the first matching
    /// `QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM` objective. When it changes
    /// one objective, `StoreNewItem` returns `nullptr` and no physical Item is
    /// created. Keeping this as a snapshot lets loot persist the objective and
    /// consume its object-owned claim in one SQL/authority transaction.
    pub(crate) fn plan_quest_source_item_bound_objective_persistence_like_cpp(
        &self,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> Option<QuestSourceItemBoundPersistencePlanLikeCpp> {
        let quest_store = self.quests.store.as_ref()?;
        let count_i32 = i32::try_from(count).unwrap_or(i32::MAX);
        let entry_object_id = i32::try_from(entry_id).unwrap_or(i32::MAX);
        let quest_log_object_id = i32::try_from(quest_log_item_id).unwrap_or(i32::MAX);
        let ordered_quest_ids = self.quest_bound_item_objective_quest_order_like_cpp();
        let state = self.player_quest_gameplay_snapshot_like_cpp()?;

        for object_id in [entry_object_id, quest_log_object_id] {
            if object_id == quest_log_object_id && quest_log_item_id == 0 {
                continue;
            }

            for quest_id in &ordered_quest_ids {
                let Some(current_status) = state.statuses_like_cpp().get(quest_id) else {
                    continue;
                };
                if current_status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                    continue;
                }
                let Some(quest) = quest_store.get(current_status.quest_id) else {
                    continue;
                };

                for (objective_index, objective) in quest.objectives.iter().enumerate() {
                    if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL
                        || (objective.flags2
                            & QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL)
                            == 0
                        || objective.object_id != object_id
                        || !wow_entities::represented_quest_objective_completable_like_cpp(
                            current_status,
                            &quest.objective_rules_like_cpp(),
                            objective_index,
                        )
                    {
                        continue;
                    }

                    let Ok(storage_index) = usize::try_from(objective.storage_index) else {
                        continue;
                    };
                    let current = current_status
                        .objective_counts
                        .get(storage_index)
                        .copied()
                        .unwrap_or(0);
                    if current >= objective.amount {
                        continue;
                    }

                    let mut planned_status = current_status.clone();
                    if planned_status.objective_counts.len() <= storage_index {
                        planned_status.objective_counts.resize(storage_index + 1, 0);
                    }
                    let new_count = current.saturating_add(count_i32).clamp(0, objective.amount);
                    planned_status.objective_counts[storage_index] = new_count;
                    let quest_already_rewarded =
                        state.rewarded_quest_ids_like_cpp().contains(&quest.id);
                    if new_count >= objective.amount
                        && wow_entities::represented_can_complete_quest_after_objective_like_cpp(
                            &planned_status,
                            &quest.objective_rules_like_cpp(),
                            objective.id,
                            quest_already_rewarded,
                        )
                    {
                        planned_status.status = QUEST_STATUS_COMPLETE_LIKE_CPP;
                    }

                    return Some(QuestSourceItemBoundPersistencePlanLikeCpp {
                        statuses: vec![planned_status],
                    });
                }
            }
        }

        None
    }
}
