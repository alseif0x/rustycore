// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest status persistence and load.

use super::*;

impl WorldSession {
    pub(super) async fn quest_poi_store_like_cpp(&mut self) -> Arc<HashMap<i32, QuestPoiData>> {
        if let Some(store) = &self.quest_poi_store_like_cpp {
            return Arc::clone(store);
        }

        let Some(port) = self.quest_poi_persistence_port_like_cpp() else {
            warn!(
                "QuestPOIQuery: quest POI persistence port unavailable; sending empty C++ response"
            );
            let store = Arc::new(HashMap::new());
            self.quest_poi_store_like_cpp = Some(Arc::clone(&store));
            return store;
        };

        let store = match port.load_quest_poi_rows_like_cpp().await {
            wow_persistence::QuestPoiLoadOutcomeLikeCpp::Loaded { points, blobs } => {
                Arc::new(build_quest_poi_store_like_cpp(points, blobs))
            }
            wow_persistence::QuestPoiLoadOutcomeLikeCpp::Failed { stage, reason } => {
                warn!(?stage, error = %reason, "QuestPOIQuery: failed to load quest POI store like C++");
                Arc::new(HashMap::new())
            }
        };

        self.quest_poi_store_like_cpp = Some(Arc::clone(&store));
        store
    }

    pub(crate) async fn save_represented_quest_status_like_cpp(&self, quest_id: u32) {
        if let Some(status) = self
            .player_quest_gameplay_snapshot_like_cpp()
            .and_then(|state| state.statuses.get(&quest_id).map(|status| status.status))
        {
            self.save_quest_to_db(quest_id, status).await;
        }
    }

    pub(crate) async fn save_changed_represented_quest_statuses_like_cpp(
        &self,
        quest_ids: &mut Vec<u32>,
    ) {
        quest_ids.sort_unstable();
        quest_ids.dedup();
        for quest_id in quest_ids.drain(..) {
            self.save_represented_quest_status_like_cpp(quest_id).await;
        }
    }

    #[cfg(test)]
    pub(crate) fn represented_quest_statuses_for_save_like_cpp(&self) -> Vec<(u32, u8)> {
        let Some(state) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return Vec::new();
        };
        let mut quests = state
            .statuses
            .iter()
            .filter_map(|(quest_id, status)| {
                if state.rewarded_quest_ids.contains(quest_id)
                    && self
                        .quest_store
                        .as_ref()
                        .and_then(|store| store.get(*quest_id))
                        .is_some_and(|quest| !quest.is_repeatable())
                {
                    return None;
                }

                Some((*quest_id, status.status))
            })
            .collect::<Vec<_>>();
        quests.sort_by_key(|(quest_id, _)| *quest_id);
        quests
    }

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
        let Some(quest_store) = self.quest_store.as_ref() else {
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
            for current_status in state.statuses.values() {
                let Some(quest) = quest_store.get(current_status.quest_id) else {
                    continue;
                };
                let mut status = current_status.clone();
                let mut changed = false;
                for (objective_index, objective) in quest.objectives.iter().enumerate() {
                    if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL
                        || objective.object_id != entry_object_id
                        || !Self::represented_quest_objective_completable_like_cpp(
                            &status,
                            quest,
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
        'matching_entry: for status in state.statuses.values() {
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
                    || !Self::represented_quest_objective_completable_like_cpp(
                        status,
                        quest,
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

        for current_status in state.statuses.values() {
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
                    || !Self::represented_quest_objective_completable_like_cpp(
                        &status,
                        quest,
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
            let quest_already_rewarded = state.rewarded_quest_ids.contains(&status.quest_id);
            if completed_objective_ids.iter().any(|objective_id| {
                Self::represented_can_complete_quest_after_objective_like_cpp(
                    &status,
                    quest,
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
                .map(|state| state.statuses.into_iter().collect())
                .unwrap_or_default(),
            changed_quest_ids: Vec::new(),
        };
        let Some(quest_store) = self.quest_store.as_ref() else {
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
            plan.changed_quest_ids
                .extend(Self::apply_quest_item_removed_to_statuses_like_cpp(
                    quest_store.as_ref(),
                    &mut plan.statuses,
                    entry_id,
                    new_non_bank_item_count,
                ));
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
        let Some(quest_store) = self.quest_store.as_ref() else {
            return false;
        };
        let Some(state) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return false;
        };
        let rewarded: HashSet<u32> = state.rewarded_quest_ids.into_iter().collect();
        if let Some((quest_id, _)) = Self::apply_quest_item_added_bound_to_statuses_like_cpp(
            quest_store.as_ref(),
            &rewarded,
            &mut plan.statuses,
            entry_id,
            quest_log_item_id,
            count,
        ) {
            plan.changed_quest_ids.push(quest_id);
            return true;
        }
        plan.changed_quest_ids
            .extend(Self::apply_quest_item_added_non_bound_to_statuses_like_cpp(
                quest_store.as_ref(),
                &rewarded,
                &mut plan.statuses,
                entry_id,
                quest_log_item_id,
                count,
            ));
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
                let objectives = self
                    .quest_store
                    .as_ref()
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
        let quest_store = self.quest_store.as_ref()?;
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
                let Some(current_status) = state.statuses.get(quest_id) else {
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
                        || !Self::represented_quest_objective_completable_like_cpp(
                            current_status,
                            quest,
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
                    let quest_already_rewarded = state.rewarded_quest_ids.contains(&quest.id);
                    if new_count >= objective.amount
                        && Self::represented_can_complete_quest_after_objective_like_cpp(
                            &planned_status,
                            quest,
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

    pub(super) async fn save_represented_quest_statuses_completed_after_like_cpp(
        &mut self,
        completion_evidence_start: usize,
    ) {
        let completed_quest_ids: Vec<_> = self.represented_quest_complete_status_updates_like_cpp
            [completion_evidence_start..]
            .iter()
            .filter_map(|evidence| {
                (evidence.new_status == QUEST_STATUS_COMPLETE_LIKE_CPP).then_some(evidence.quest_id)
            })
            .collect();
        for quest_id in completed_quest_ids {
            self.save_represented_quest_status_like_cpp(quest_id).await;
        }
    }

    pub(super) async fn save_quest_to_db(&self, quest_id: u32, status: u8) {
        let owner_guid = match self.player_guid() {
            Some(g) => g.counter() as u64,
            None => return,
        };
        let port = match self.player_quest_persistence_port_like_cpp() {
            Some(port) => port,
            None => return,
        };
        let quest_state = self.player_quest_gameplay_snapshot_like_cpp();
        let mut projection = match quest_state
            .as_ref()
            .and_then(|state| state.statuses.get(&quest_id))
        {
            Some(saved) => self.represented_quest_status_persistence_like_cpp(saved),
            None if status == QUEST_STATUS_REWARDED_LIKE_CPP => {
                wow_persistence::QuestStatusPersistenceLikeCpp {
                    quest_id,
                    status,
                    explored: false,
                    accept_time_secs: 0,
                    end_time_secs: 0,
                    objectives: Vec::new(),
                }
            }
            None => {
                warn!(
                    account = self.account_id,
                    quest_id,
                    "Quest status save skipped because canonical Player quest state is unavailable"
                );
                return;
            }
        };
        projection.status = status;

        match port
            .persist_status_like_cpp(
                wow_persistence::PlayerQuestStatusPersistenceRequestLikeCpp::Save {
                    owner_guid,
                    status: projection,
                },
            )
            .await
        {
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {}
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason } => warn!(
                account = self.account_id,
                quest_id,
                error = %reason,
                "Failed to save quest status"
            ),
            wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => warn!(
                account = self.account_id,
                quest_id,
                error = %reason,
                "Quest status save commit outcome is unknown"
            ),
        }
    }

    /// Delete a quest from the characters database (abandon).
    pub(super) async fn delete_quest_from_db(&self, quest_id: u32) {
        let owner_guid = match self.player_guid() {
            Some(g) => g.counter() as u64,
            None => return,
        };
        let port = match self.player_quest_persistence_port_like_cpp() {
            Some(port) => port,
            None => return,
        };
        match port
            .persist_status_like_cpp(
                wow_persistence::PlayerQuestStatusPersistenceRequestLikeCpp::Delete {
                    owner_guid,
                    quest_id,
                },
            )
            .await
        {
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {}
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason } => warn!(
                account = self.account_id,
                quest_id,
                error = %reason,
                "Failed to delete quest"
            ),
            wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => warn!(
                account = self.account_id,
                quest_id,
                error = %reason,
                "Quest deletion commit outcome is unknown"
            ),
        }
    }

    /// Load all active quests for this player from the characters DB.
    pub(crate) async fn load_player_quests(&mut self) {
        self.begin_player_quest_status_authority_load_like_cpp();

        let owner_guid = match self.player_guid() {
            Some(g) => g.counter() as u64,
            None => return,
        };
        let port = match self.player_quest_persistence_port_like_cpp() {
            Some(port) => port,
            None => return,
        };

        let active_rows = match port.load_active_statuses_like_cpp(owner_guid).await {
            wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Loaded(rows) => rows,
            wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.account_id,
                    error = %reason,
                    "Failed to load quest status"
                );
                return;
            }
        };

        let mut loaded_quests = wow_entities::PlayerQuestGameplayState::default();

        let mut quest_status_rows_coherent_like_cpp = true;
        let mut next_active_slot: u8 = 0;
        let mut stale_rewarded_active_rows = Vec::new();

        for row in active_rows {
            let (
                Some(quest_id),
                Some(status),
                Some(explored),
                Some(accept_time_secs),
                Some(end_time_secs),
            ) = (
                row.quest_id,
                row.status,
                row.explored,
                row.accept_time_secs,
                row.end_time_secs,
            )
            else {
                quest_status_rows_coherent_like_cpp = false;
                continue;
            };
            let status = if status < 7 {
                status
            } else {
                QUEST_STATUS_INCOMPLETE_LIKE_CPP
            };
            let explored = explored != 0;

            if status == QUEST_STATUS_REWARDED_LIKE_CPP {
                // Rewarded (C++ QuestStatus::QUEST_STATUS_REWARDED / m_RewardedQuests).
                // Non-repeatable quests cannot be re-taken once rewarded.
                loaded_quests.rewarded_quest_ids.insert(quest_id);
                stale_rewarded_active_rows.push(quest_id);
            } else if next_active_slot < MAX_QUEST_LOG_SIZE_LIKE_CPP {
                // Active or complete-but-not-turned-in.
                // C++ _LoadQuestStatus assigns sequential visible slots in DB row order
                // because the character DB status row has no persisted quest-log slot.
                let slot = next_active_slot;
                next_active_slot = next_active_slot.saturating_add(1);
                let obj_count = self
                    .quest_store
                    .as_ref()
                    .and_then(|s| s.get(quest_id))
                    .map_or(0, |q| q.objectives.len());
                if loaded_quests.statuses.contains_key(&quest_id) {
                    quest_status_rows_coherent_like_cpp = false;
                }
                loaded_quests.statuses.insert(
                    quest_id,
                    PlayerQuestStatus {
                        quest_id,
                        status,
                        explored,
                        accept_time_secs,
                        end_time_secs,
                        objective_counts: vec![0; obj_count],
                        slot,
                    },
                );
            }
        }

        match port.load_objectives_like_cpp(owner_guid).await {
            wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Loaded(objective_rows) => {
                for row in objective_rows {
                    let quest_id = row.quest_id.unwrap_or(0);
                    let storage_index = row.storage_index.unwrap_or(0);
                    let data = row.count.unwrap_or(0);
                    if let (Some(status), Some(quest)) = (
                        loaded_quests.statuses.get_mut(&quest_id),
                        self.quest_store
                            .as_ref()
                            .and_then(|store| store.get(quest_id)),
                    ) {
                        if let Some(objective) = quest.objectives.iter().find(|objective| {
                            u8::try_from(objective.storage_index).ok() == Some(storage_index)
                        }) {
                            let index = usize::from(storage_index);
                            if status.objective_counts.len() <= index {
                                status.objective_counts.resize(index + 1, 0);
                            }
                            status.objective_counts[index] = if objective.is_storing_flag_like_cpp()
                            {
                                i32::from(data != 0)
                            } else {
                                data
                            };
                        }
                    }
                }
            }
            wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.account_id,
                    error = %reason,
                    "Failed to load quest objective status"
                );
            }
        }

        let mut rewarded_rows_coherent_like_cpp = false;
        match port.load_rewarded_like_cpp(owner_guid).await {
            wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Loaded(rewarded_rows) => {
                rewarded_rows_coherent_like_cpp = true;
                for row in rewarded_rows {
                    let Some(quest_id) = row.quest_id else {
                        rewarded_rows_coherent_like_cpp = false;
                        continue;
                    };
                    loaded_quests.rewarded_quest_rows.insert(quest_id);
                    if self
                        .represented_quest_can_increase_rewarded_counters_like_cpp(quest_id)
                        .is_some_and(|can_increase| can_increase)
                    {
                        loaded_quests.rewarded_quest_ids.insert(quest_id);
                    }
                }
            }
            wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.account_id,
                    error = %reason,
                    "Failed to load rewarded quest status"
                );
            }
        }

        loaded_quests.status_authority_complete =
            quest_status_rows_coherent_like_cpp && rewarded_rows_coherent_like_cpp;
        if self
            .mutate_player_quest_gameplay_like_cpp(|state| {
                state.statuses = loaded_quests.statuses;
                state.rewarded_quest_ids = loaded_quests.rewarded_quest_ids;
                state.rewarded_quest_rows = loaded_quests.rewarded_quest_rows;
                state.status_authority_complete = loaded_quests.status_authority_complete;
            })
            .is_none()
        {
            warn!(
                account = self.account_id,
                "Failed to install loaded quest status into canonical Player owner"
            );
            return;
        }

        stale_rewarded_active_rows
            .extend(self.remove_represented_active_rewarded_duplicates_like_cpp());
        stale_rewarded_active_rows.sort_unstable();
        stale_rewarded_active_rows.dedup();
        for quest_id in stale_rewarded_active_rows {
            info!(
                account = self.account_id,
                quest_id,
                "QuestLoad: migrating stale active rewarded quest status before deleting active row like C++"
            );
            self.save_quest_to_db(quest_id, QUEST_STATUS_REWARDED_LIKE_CPP)
                .await;
        }

        let mut loaded_df = std::collections::BTreeSet::new();
        let mut loaded_daily = std::collections::BTreeSet::new();
        let mut loaded_last_daily_time = 0;
        match port.load_daily_like_cpp(owner_guid).await {
            wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Loaded(daily_rows) => {
                for row in daily_rows {
                    let quest_id = row.quest_id.unwrap_or(0);
                    let completed_time = row.completed_time.unwrap_or(0);
                    if let Some(quest) = self
                        .quest_store
                        .as_ref()
                        .and_then(|store| store.get(quest_id))
                    {
                        loaded_last_daily_time = completed_time;
                        if quest.is_df_quest_like_cpp() {
                            loaded_df.insert(quest_id);
                        } else {
                            loaded_daily.insert(quest_id);
                        }
                    }
                }
            }
            wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.account_id,
                    error = %reason,
                    "Failed to load daily quest status"
                );
            }
        }
        let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.df_quest_ids = loaded_df;
            state.daily_quest_ids = loaded_daily;
            state.last_daily_quest_time_secs = loaded_last_daily_time;
        });

        let mut loaded_weekly = std::collections::BTreeSet::new();
        match port.load_weekly_like_cpp(owner_guid).await {
            wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Loaded(weekly_rows) => {
                for row in weekly_rows {
                    let quest_id = row.quest_id.unwrap_or(0);
                    if self
                        .quest_store
                        .as_ref()
                        .and_then(|store| store.get(quest_id))
                        .is_some()
                    {
                        loaded_weekly.insert(quest_id);
                    }
                }
            }
            wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.account_id,
                    error = %reason,
                    "Failed to load weekly quest status"
                );
            }
        }
        let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.weekly_quest_ids = loaded_weekly;
        });

        let mut loaded_monthly = std::collections::BTreeSet::new();
        match port.load_monthly_like_cpp(owner_guid).await {
            wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Loaded(monthly_rows) => {
                for row in monthly_rows {
                    let quest_id = row.quest_id.unwrap_or(0);
                    if self
                        .quest_store
                        .as_ref()
                        .and_then(|store| store.get(quest_id))
                        .is_some()
                    {
                        loaded_monthly.insert(quest_id);
                    }
                }
            }
            wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.account_id,
                    error = %reason,
                    "Failed to load monthly quest status"
                );
            }
        }
        let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.monthly_quest_ids = loaded_monthly;
        });

        let seasonal_rows = match port.load_seasonal_like_cpp(owner_guid).await {
            wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Loaded(rows) => rows
                .into_iter()
                .map(|row| {
                    let quest_id = row.quest_id.unwrap_or_else(|| {
                        warn!(
                            account = self.account_id,
                            "Failed to read seasonal quest id"
                        );
                        0
                    });
                    let event_id = row.event_id.unwrap_or_else(|| {
                        warn!(
                            account = self.account_id,
                            quest_id, "Failed to read seasonal quest event id"
                        );
                        u32::MAX
                    });
                    let completed_time = row.completed_time.unwrap_or_else(|| {
                        warn!(
                            account = self.account_id,
                            quest_id, event_id, "Failed to read seasonal quest completedTime"
                        );
                        -1
                    });
                    SeasonalQuestStatusDbRowLikeCpp {
                        quest_id,
                        event_id,
                        completed_time,
                    }
                })
                .collect(),
            wow_persistence::PlayerQuestLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.account_id,
                    error = %reason,
                    "Failed to load seasonal quest status"
                );
                Vec::new()
            }
        };

        let quest_store = self.quest_store.as_ref().map(Arc::clone);
        let quest_v2_store = self.quest_v2_store.as_ref().map(Arc::clone);
        let seasonal_outcome = self.load_seasonal_quest_status_like_cpp(
            seasonal_rows,
            quest_store.as_deref(),
            quest_v2_store.as_deref(),
        );

        if seasonal_outcome.skipped_no_quest_store > 0
            || seasonal_outcome.skipped_missing_quest > 0
            || seasonal_outcome.skipped_event_out_of_range > 0
            || seasonal_outcome.skipped_negative_completed_time > 0
            || seasonal_outcome.completed_bit_skipped_no_quest_v2_store > 0
            || seasonal_outcome.completed_bit_skipped_zero_unique_bit > 0
            || seasonal_outcome.completed_bit_no_change_or_noop > 0
        {
            warn!(
                account = self.account_id,
                rows_seen = seasonal_outcome.rows_seen,
                skipped_no_quest_store = seasonal_outcome.skipped_no_quest_store,
                skipped_missing_quest = seasonal_outcome.skipped_missing_quest,
                skipped_event_out_of_range = seasonal_outcome.skipped_event_out_of_range,
                skipped_negative_completed_time = seasonal_outcome.skipped_negative_completed_time,
                completed_bit_skipped_no_quest_v2_store =
                    seasonal_outcome.completed_bit_skipped_no_quest_v2_store,
                completed_bit_skipped_zero_unique_bit =
                    seasonal_outcome.completed_bit_skipped_zero_unique_bit,
                completed_bit_no_change_or_noop = seasonal_outcome.completed_bit_no_change_or_noop,
                "Skipped seasonal quest status rows during login load"
            );
        }

        let recurrence = self.player_quest_gameplay_snapshot_like_cpp();
        info!(
            account = self.account_id,
            active = recurrence.as_ref().map_or(0, |state| state.statuses.len()),
            rewarded = recurrence
                .as_ref()
                .map_or(0, |state| state.rewarded_quest_ids.len()),
            df = recurrence
                .as_ref()
                .map_or(0, |state| state.df_quest_ids.len()),
            daily = recurrence
                .as_ref()
                .map_or(0, |state| state.daily_quest_ids.len()),
            weekly = recurrence
                .as_ref()
                .map_or(0, |state| state.weekly_quest_ids.len()),
            monthly = recurrence
                .as_ref()
                .map_or(0, |state| state.monthly_quest_ids.len()),
            seasonal_inserted = seasonal_outcome.inserted,
            seasonal_replaced = seasonal_outcome.replaced,
            seasonal_completed_bit_set = seasonal_outcome.completed_bit_set,
            seasonal_completed_bit_skipped_no_quest_v2_store =
                seasonal_outcome.completed_bit_skipped_no_quest_v2_store,
            seasonal_completed_bit_skipped_zero_unique_bit =
                seasonal_outcome.completed_bit_skipped_zero_unique_bit,
            seasonal_completed_bit_no_change_or_noop =
                seasonal_outcome.completed_bit_no_change_or_noop,
            "Loaded player quests"
        );
    }
}
