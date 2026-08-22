// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest status persistence and load.

// Explicit database imports: this module reaches its parent through
// `use super::*`, and the persistence inventory cannot resolve a glob, so
// without these every database access in the file is invisible to the
// ratchet (see #277).
use wow_database::{CharStatements, CharacterDatabase, PreparedStatement, SqlTransaction};

use super::*;

impl WorldSession {
    pub(super) async fn quest_poi_store_like_cpp(&mut self) -> Arc<HashMap<i32, QuestPoiData>> {
        if let Some(store) = &self.quest_poi_store_like_cpp {
            return Arc::clone(store);
        }

        let Some(world_db) = self.world_db().map(Arc::clone) else {
            warn!("QuestPOIQuery: world DB unavailable; sending empty C++ response");
            let store = Arc::new(HashMap::new());
            self.quest_poi_store_like_cpp = Some(Arc::clone(&store));
            return store;
        };

        let store = match load_quest_poi_store_like_cpp(world_db.as_ref()).await {
            Ok(store) => Arc::new(store),
            Err(err) => {
                warn!("QuestPOIQuery: failed to load quest POI store like C++: {err}");
                Arc::new(HashMap::new())
            }
        };

        self.quest_poi_store_like_cpp = Some(Arc::clone(&store));
        store
    }

    pub(crate) async fn save_represented_quest_status_like_cpp(&self, quest_id: u32) {
        if let Some(status) = self
            .player_quests
            .get(&quest_id)
            .map(|status| status.status)
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
        let mut quests = self
            .player_quests
            .iter()
            .filter_map(|(quest_id, status)| {
                if self.rewarded_quests.contains(quest_id)
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

        if moving_to_bank {
            let new_item_count = i32::try_from(post_move_non_bank_count).unwrap_or(i32::MAX);
            for current_status in self.player_quests.values() {
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
        'matching_entry: for status in self.player_quests.values() {
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

        for current_status in self.player_quests.values() {
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
            let quest_already_rewarded = self.rewarded_quests.contains(&status.quest_id);
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
            statuses: self.player_quests.clone(),
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
        if let Some((quest_id, _)) = Self::apply_quest_item_added_bound_to_statuses_like_cpp(
            quest_store.as_ref(),
            &self.rewarded_quests,
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
                &self.rewarded_quests,
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

    pub(crate) fn append_planned_quest_statuses_to_transaction_like_cpp(
        &self,
        transaction: &mut SqlTransaction,
        char_db: &CharacterDatabase,
        player_guid: u64,
        planned_statuses: &[PlayerQuestStatus],
    ) {
        for status in planned_statuses {
            for statement in self.represented_quest_status_save_statements_like_cpp(
                player_guid,
                status.quest_id,
                status.status,
                Some(status),
                |statement| char_db.prepare(statement),
            ) {
                transaction.append(statement);
            }
        }
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

        for object_id in [entry_object_id, quest_log_object_id] {
            if object_id == quest_log_object_id && quest_log_item_id == 0 {
                continue;
            }

            for quest_id in &ordered_quest_ids {
                let Some(current_status) = self.player_quests.get(quest_id) else {
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
                    let quest_already_rewarded = self.rewarded_quests.contains(&quest.id);
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

    /// Save quest status and represented objective counters to the characters database.
    ///
    /// C++ anchor: `Player::_SaveQuestStatus`, `Player.cpp:20160-20191`.
    /// The represented path keeps Rust's existing direct save timing, but mirrors the
    /// C++ objective persistence order for a saved quest: status row first, then delete
    /// stale objective rows for the quest, then replace nonzero objective counters.
    /// For Rust's combined rewarded migration path, preserve the rewarded row before
    /// deleting the stale active row.
    pub(super) fn represented_quest_status_save_statements_like_cpp(
        &self,
        guid: u64,
        quest_id: u32,
        status: u8,
        status_snapshot: Option<&PlayerQuestStatus>,
        mut prepare: impl FnMut(CharStatements) -> PreparedStatement,
    ) -> Vec<PreparedStatement> {
        let mut statements = Vec::new();

        if status == QUEST_STATUS_REWARDED_LIKE_CPP {
            let mut rewarded = prepare(CharStatements::INS_CHAR_QUESTSTATUS_REWARDED);
            rewarded.set_u64(0, guid);
            rewarded.set_u32(1, quest_id);
            statements.push(rewarded);

            let mut del_status = prepare(CharStatements::DEL_CHAR_QUEST_STATUS);
            del_status.set_u64(0, guid);
            del_status.set_u32(1, quest_id);
            statements.push(del_status);

            let mut del_objectives =
                prepare(CharStatements::DEL_CHAR_QUEST_STATUS_OBJECTIVES_BY_QUEST);
            del_objectives.set_u64(0, guid);
            del_objectives.set_u32(1, quest_id);
            statements.push(del_objectives);

            return statements;
        }

        let saved_status = status_snapshot.or_else(|| self.player_quests.get(&quest_id));
        let represented_explored = saved_status.map(|status| status.explored).unwrap_or(false);
        let represented_accept_time = saved_status
            .map(|status| status.accept_time_secs)
            .unwrap_or(0);
        let represented_end_time = saved_status.map(|status| status.end_time_secs).unwrap_or(0);
        let mut stmt = prepare(CharStatements::INS_CHAR_QUEST_STATUS);
        stmt.set_u64(0, guid);
        stmt.set_u32(1, quest_id);
        stmt.set_u8(2, status);
        stmt.set_u8(3, u8::from(represented_explored));
        stmt.set_i64(4, represented_accept_time);
        stmt.set_i64(5, represented_end_time);
        statements.push(stmt);

        let mut del_objectives = prepare(CharStatements::DEL_CHAR_QUEST_STATUS_OBJECTIVES_BY_QUEST);
        del_objectives.set_u64(0, guid);
        del_objectives.set_u32(1, quest_id);
        statements.push(del_objectives);

        if let (Some(quest_store), Some(saved_status)) = (self.quest_store.as_ref(), saved_status)
            && let Some(quest) = quest_store.get(quest_id)
        {
            for objective in &quest.objectives {
                if objective.storage_index < 0 {
                    continue;
                }
                let storage_index = objective.storage_index as usize;
                let count = saved_status
                    .objective_counts
                    .get(storage_index)
                    .copied()
                    .unwrap_or(0);
                if count == 0 {
                    continue;
                }

                let Ok(objective_index) = u8::try_from(objective.storage_index) else {
                    continue;
                };
                let mut rep_objective = prepare(CharStatements::REP_CHAR_QUEST_STATUS_OBJECTIVES);
                rep_objective.set_u64(0, guid);
                rep_objective.set_u32(1, quest_id);
                rep_objective.set_u8(2, objective_index);
                rep_objective.set_i32(3, count);
                statements.push(rep_objective);
            }
        }

        statements
    }

    pub(super) async fn save_quest_to_db(&self, quest_id: u32, status: u8) {
        let guid = match self.player_guid() {
            Some(g) => g.counter() as u64,
            None => return,
        };
        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        let mut tx = SqlTransaction::new();
        for stmt in self.represented_quest_status_save_statements_like_cpp(
            guid,
            quest_id,
            status,
            None,
            |statement| char_db.prepare(statement),
        ) {
            tx.append(stmt);
        }

        if let Err(e) = char_db.commit_transaction(tx).await {
            warn!(
                account = self.account_id,
                quest_id, "Failed to save quest status: {e}"
            );
        }
    }

    /// Delete a quest from the characters database (abandon).
    pub(super) async fn delete_quest_from_db(&self, quest_id: u32) {
        use wow_database::CharStatements;

        let guid = match self.player_guid() {
            Some(g) => g.counter() as u64,
            None => return,
        };
        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        let mut tx = SqlTransaction::new();
        let mut stmt = char_db.prepare(CharStatements::DEL_CHAR_QUEST_STATUS);
        stmt.set_u64(0, guid);
        stmt.set_u32(1, quest_id);
        tx.append(stmt);

        let mut del_objectives =
            char_db.prepare(CharStatements::DEL_CHAR_QUEST_STATUS_OBJECTIVES_BY_QUEST);
        del_objectives.set_u64(0, guid);
        del_objectives.set_u32(1, quest_id);
        tx.append(del_objectives);

        if let Err(e) = char_db.commit_transaction(tx).await {
            warn!(
                account = self.account_id,
                quest_id, "Failed to delete quest: {e}"
            );
        }
    }

    /// Load all active quests for this player from the characters DB.
    pub(crate) async fn load_player_quests(&mut self) {
        use wow_database::CharStatements;

        self.begin_player_quest_status_authority_load_like_cpp();

        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };
        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        let mut stmt = char_db.prepare(CharStatements::SEL_CHAR_QUEST_STATUS);
        Self::bind_player_quest_status_load_guid_like_cpp(&mut stmt, player_guid);

        let result = match char_db.query(&stmt).await {
            Ok(r) => r,
            Err(e) => {
                warn!(
                    account = self.account_id,
                    "Failed to load quest status: {e}"
                );
                return;
            }
        };

        self.player_quests.clear();
        self.rewarded_quests.clear();

        let mut quest_status_rows_coherent_like_cpp = true;
        let mut next_active_slot: u8 = 0;
        let mut stale_rewarded_active_rows = Vec::new();

        if !result.is_empty() {
            let mut result = result;
            loop {
                let row = (
                    result.try_read::<u32>(0),
                    result.try_read::<u8>(1),
                    result.try_read::<u8>(2),
                    result.try_read::<i64>(3),
                    result.try_read::<i64>(4),
                );
                let (
                    Some(quest_id),
                    Some(status),
                    Some(explored),
                    Some(accept_time_secs),
                    Some(end_time_secs),
                ) = row
                else {
                    quest_status_rows_coherent_like_cpp = false;
                    if !result.next_row() {
                        break;
                    }
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
                    self.rewarded_quests.insert(quest_id);
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
                    if self.player_quests.contains_key(&quest_id) {
                        quest_status_rows_coherent_like_cpp = false;
                    }
                    self.player_quests.insert(
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

                if !result.next_row() {
                    break;
                }
            }
        }

        let mut objective_stmt = char_db.prepare(CharStatements::SEL_CHAR_QUEST_STATUS_OBJECTIVES);
        Self::bind_player_quest_status_load_guid_like_cpp(&mut objective_stmt, player_guid);

        match char_db.query(&objective_stmt).await {
            Ok(objective_rows) if !objective_rows.is_empty() => {
                let mut objective_rows = objective_rows;
                loop {
                    let quest_id: u32 = objective_rows.try_read::<u32>(0).unwrap_or(0);
                    let storage_index: u8 = objective_rows.try_read::<u8>(1).unwrap_or(0);
                    let data: i32 = objective_rows.try_read::<i32>(2).unwrap_or(0);

                    if let (Some(status), Some(quest)) = (
                        self.player_quests.get_mut(&quest_id),
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

                    if !objective_rows.next_row() {
                        break;
                    }
                }
            }
            Ok(_) => {}
            Err(e) => {
                warn!(
                    account = self.account_id,
                    "Failed to load quest objective status: {e}"
                );
            }
        }

        let mut rewarded_rows_coherent_like_cpp = false;
        let mut rewarded_stmt = char_db.prepare(CharStatements::SEL_CHARACTER_QUESTSTATUSREW);
        Self::bind_player_quest_status_load_guid_like_cpp(&mut rewarded_stmt, player_guid);
        match char_db.query(&rewarded_stmt).await {
            Ok(rewarded_rows) if !rewarded_rows.is_empty() => {
                rewarded_rows_coherent_like_cpp = true;
                let mut rewarded_rows = rewarded_rows;
                loop {
                    let Some(quest_id) = rewarded_rows.try_read::<u32>(0) else {
                        rewarded_rows_coherent_like_cpp = false;
                        if !rewarded_rows.next_row() {
                            break;
                        }
                        continue;
                    };
                    self.record_represented_rewarded_quest_row_like_cpp(quest_id);
                    if self
                        .represented_quest_can_increase_rewarded_counters_like_cpp(quest_id)
                        .is_some_and(|can_increase| can_increase)
                    {
                        self.rewarded_quests.insert(quest_id);
                    }

                    if !rewarded_rows.next_row() {
                        break;
                    }
                }
            }
            Ok(_) => rewarded_rows_coherent_like_cpp = true,
            Err(e) => {
                warn!(
                    account = self.account_id,
                    "Failed to load rewarded quest status: {e}"
                );
            }
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

        if quest_status_rows_coherent_like_cpp && rewarded_rows_coherent_like_cpp {
            self.complete_player_quest_status_authority_load_like_cpp();
        }

        self.df_quests_like_cpp.clear();
        self.daily_quests_completed_like_cpp.clear();
        self.last_daily_quest_time_like_cpp = 0;
        let mut daily_stmt = char_db.prepare(CharStatements::SEL_CHARACTER_QUESTSTATUS_DAILY);
        Self::bind_player_quest_status_load_guid_like_cpp(&mut daily_stmt, player_guid);
        match char_db.query(&daily_stmt).await {
            Ok(daily_rows) if !daily_rows.is_empty() => {
                let mut daily_rows = daily_rows;
                loop {
                    let quest_id = daily_rows.try_read::<u32>(0).unwrap_or(0);
                    let completed_time = daily_rows.try_read::<i64>(1).unwrap_or(0);
                    if let Some(quest) = self
                        .quest_store
                        .as_ref()
                        .and_then(|store| store.get(quest_id))
                    {
                        self.last_daily_quest_time_like_cpp = completed_time;
                        if quest.is_df_quest_like_cpp() {
                            self.df_quests_like_cpp.insert(quest_id);
                        } else {
                            self.daily_quests_completed_like_cpp.insert(quest_id);
                        }
                    }

                    if !daily_rows.next_row() {
                        break;
                    }
                }
            }
            Ok(_) => {}
            Err(e) => {
                warn!(
                    account = self.account_id,
                    "Failed to load daily quest status: {e}"
                );
            }
        }

        self.weekly_quests_completed_like_cpp.clear();
        let mut weekly_stmt = char_db.prepare(CharStatements::SEL_CHARACTER_QUESTSTATUS_WEEKLY);
        Self::bind_player_quest_status_load_guid_like_cpp(&mut weekly_stmt, player_guid);
        match char_db.query(&weekly_stmt).await {
            Ok(weekly_rows) if !weekly_rows.is_empty() => {
                let mut weekly_rows = weekly_rows;
                loop {
                    let quest_id = weekly_rows.try_read::<u32>(0).unwrap_or(0);
                    if self
                        .quest_store
                        .as_ref()
                        .and_then(|store| store.get(quest_id))
                        .is_some()
                    {
                        self.weekly_quests_completed_like_cpp.insert(quest_id);
                    }

                    if !weekly_rows.next_row() {
                        break;
                    }
                }
            }
            Ok(_) => {}
            Err(e) => {
                warn!(
                    account = self.account_id,
                    "Failed to load weekly quest status: {e}"
                );
            }
        }

        self.monthly_quests_completed_like_cpp.clear();
        let mut monthly_stmt = char_db.prepare(CharStatements::SEL_CHARACTER_QUESTSTATUS_MONTHLY);
        Self::bind_player_quest_status_load_guid_like_cpp(&mut monthly_stmt, player_guid);
        match char_db.query(&monthly_stmt).await {
            Ok(monthly_rows) if !monthly_rows.is_empty() => {
                let mut monthly_rows = monthly_rows;
                loop {
                    let quest_id = monthly_rows.try_read::<u32>(0).unwrap_or(0);
                    if self
                        .quest_store
                        .as_ref()
                        .and_then(|store| store.get(quest_id))
                        .is_some()
                    {
                        self.monthly_quests_completed_like_cpp.insert(quest_id);
                    }

                    if !monthly_rows.next_row() {
                        break;
                    }
                }
            }
            Ok(_) => {}
            Err(e) => {
                warn!(
                    account = self.account_id,
                    "Failed to load monthly quest status: {e}"
                );
            }
        }

        let mut seasonal_stmt = char_db.prepare(CharStatements::SEL_CHAR_QUEST_STATUS_SEASONAL);
        Self::bind_player_quest_status_load_guid_like_cpp(&mut seasonal_stmt, player_guid);

        let seasonal_rows = match char_db.query(&seasonal_stmt).await {
            Ok(result) => {
                let mut rows = Vec::new();
                if !result.is_empty() {
                    let mut result = result;
                    loop {
                        let quest_id = result.try_read::<u32>(0).unwrap_or_else(|| {
                            warn!(
                                account = self.account_id,
                                "Failed to read seasonal quest id"
                            );
                            0
                        });
                        let event_id = result.try_read::<u32>(1).unwrap_or_else(|| {
                            warn!(
                                account = self.account_id,
                                quest_id, "Failed to read seasonal quest event id"
                            );
                            u32::MAX
                        });
                        let completed_time = result.try_read::<i64>(2).unwrap_or_else(|| {
                            warn!(
                                account = self.account_id,
                                quest_id, event_id, "Failed to read seasonal quest completedTime"
                            );
                            -1
                        });
                        rows.push(SeasonalQuestStatusDbRowLikeCpp {
                            quest_id,
                            event_id,
                            completed_time,
                        });

                        if !result.next_row() {
                            break;
                        }
                    }
                }
                rows
            }
            Err(e) => {
                warn!(
                    account = self.account_id,
                    "Failed to load seasonal quest status: {e}"
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

        info!(
            account = self.account_id,
            active = self.player_quests.len(),
            rewarded = self.rewarded_quests.len(),
            df = self.df_quests_like_cpp.len(),
            daily = self.daily_quests_completed_like_cpp.len(),
            weekly = self.weekly_quests_completed_like_cpp.len(),
            monthly = self.monthly_quests_completed_like_cpp.len(),
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
