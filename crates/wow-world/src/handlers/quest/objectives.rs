// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest objective progress and completion.

// Explicit database imports: this module reaches its parent through
// `use super::*`, and the persistence inventory cannot resolve a glob, so
// without these every database access in the file is invisible to the
// ratchet (see #277).
use wow_database::WorldStatements;

use super::*;

impl WorldSession {
    pub(crate) fn represented_quest_objective_complete_like_cpp(
        status: &PlayerQuestStatus,
        quest: &wow_data::quest::QuestTemplate,
        objective: &wow_data::quest::QuestObjective,
    ) -> bool {
        match objective.obj_type {
            QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL
            | QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL
            | QUEST_OBJECTIVE_GAMEOBJECT_LIKE_CPP_LOCAL
            | QUEST_OBJECTIVE_TALKTO_LIKE_CPP_LOCAL
            | QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP_LOCAL
            | QUEST_OBJECTIVE_WINPVPPETBATTLES_LIKE_CPP_LOCAL
            | QUEST_OBJECTIVE_CRITERIA_TREE_LIKE_CPP_LOCAL
            | QUEST_OBJECTIVE_HAVE_CURRENCY_LIKE_CPP_LOCAL
            | QUEST_OBJECTIVE_OBTAIN_CURRENCY_LIKE_CPP_LOCAL
            | QUEST_OBJECTIVE_INCREASE_REPUTATION_LIKE_CPP_LOCAL => {
                let Ok(storage_index) = usize::try_from(objective.storage_index) else {
                    return false;
                };
                status
                    .objective_counts
                    .get(storage_index)
                    .copied()
                    .unwrap_or(0)
                    >= objective.amount
            }
            QUEST_OBJECTIVE_PROGRESS_BAR_LIKE_CPP_LOCAL => {
                Self::represented_quest_objective_progress_bar_complete_like_cpp(status, quest)
            }
            // Other objective completion sources need live runtime data. This helper is only
            // used as a guard before represented item-objective progress, so fail closed.
            _ => false,
        }
    }

    fn represented_quest_objective_progress_bar_complete_like_cpp(
        status: &PlayerQuestStatus,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        let mut progress = 0.0_f32;
        for objective in &quest.objectives {
            if (objective.flags & QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP_LOCAL) == 0 {
                continue;
            }

            let Ok(storage_index) = usize::try_from(objective.storage_index) else {
                continue;
            };
            let count = status
                .objective_counts
                .get(storage_index)
                .copied()
                .unwrap_or(0);
            progress += count as f32 * objective.progress_bar_weight;
            if progress >= 100.0 {
                return true;
            }
        }
        false
    }

    pub(crate) fn represented_quest_objective_completable_like_cpp(
        status: &PlayerQuestStatus,
        quest: &wow_data::quest::QuestTemplate,
        objective_index: usize,
    ) -> bool {
        let Some(objective) = quest.objectives.get(objective_index) else {
            return false;
        };

        if (objective.flags & QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP_LOCAL) != 0 {
            let Some((progress_bar_index, progress_bar_objective)) =
                quest.objectives.iter().enumerate().find(|(_, other)| {
                    other.obj_type == QUEST_OBJECTIVE_PROGRESS_BAR_LIKE_CPP_LOCAL
                        && (other.flags & QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP_LOCAL)
                            == 0
                })
            else {
                return false;
            };

            return Self::represented_quest_objective_completable_like_cpp(
                status,
                quest,
                progress_bar_index,
            ) && !Self::represented_quest_objective_complete_like_cpp(
                status,
                quest,
                progress_bar_objective,
            );
        }

        if objective_index == 0 {
            return true;
        }

        let mut previous_index = objective_index - 1;
        let mut objective_sequence_satisfied = true;
        let mut previous_sequenced_objective_complete = false;
        let mut previous_sequenced_objective_index = None;

        loop {
            let previous_objective = &quest.objectives[previous_index];
            if (previous_objective.flags & QUEST_OBJECTIVE_FLAG_SEQUENCED_LIKE_CPP_LOCAL) != 0 {
                previous_sequenced_objective_index = Some(previous_index);
                previous_sequenced_objective_complete =
                    Self::represented_quest_objective_complete_like_cpp(
                        status,
                        quest,
                        previous_objective,
                    );
                break;
            }

            if objective_sequence_satisfied {
                objective_sequence_satisfied = Self::represented_quest_objective_complete_like_cpp(
                    status,
                    quest,
                    previous_objective,
                ) || (previous_objective.flags
                    & (QUEST_OBJECTIVE_FLAG_OPTIONAL_LIKE_CPP_LOCAL
                        | QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP_LOCAL))
                    != 0;
            }

            if previous_index == 0 {
                break;
            }
            previous_index -= 1;
        }

        if (objective.flags & QUEST_OBJECTIVE_FLAG_SEQUENCED_LIKE_CPP_LOCAL) != 0 {
            if previous_sequenced_objective_index.is_none() {
                return objective_sequence_satisfied;
            }
            if !previous_sequenced_objective_complete || !objective_sequence_satisfied {
                return false;
            }
        } else if !previous_sequenced_objective_complete {
            if let Some(previous_sequenced_objective_index) = previous_sequenced_objective_index {
                if !Self::represented_quest_objective_completable_like_cpp(
                    status,
                    quest,
                    previous_sequenced_objective_index,
                ) {
                    return false;
                }
            }
        }

        true
    }

    pub(crate) fn represented_can_complete_quest_after_objective_like_cpp(
        status: &PlayerQuestStatus,
        quest: &wow_data::quest::QuestTemplate,
        ignored_objective_id: u32,
        quest_already_rewarded: bool,
    ) -> bool {
        if quest.id == 0 {
            return false;
        }

        if !quest.is_repeatable() && quest_already_rewarded {
            return false;
        }

        if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
            return false;
        }

        for objective in &quest.objectives {
            if ignored_objective_id != 0 && objective.id == ignored_objective_id {
                continue;
            }

            if (objective.flags
                & (QUEST_OBJECTIVE_FLAG_OPTIONAL_LIKE_CPP_LOCAL
                    | QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP_LOCAL))
                != 0
            {
                continue;
            }

            if !Self::represented_quest_objective_complete_like_cpp(status, quest, objective) {
                return false;
            }
        }

        if (quest.flags
            & (QUEST_FLAGS_COMPLETION_EVENT_LIKE_CPP
                | QUEST_FLAGS_COMPLETION_AREA_TRIGGER_LIKE_CPP))
            != 0
            && !status.explored
        {
            return false;
        }

        if quest.limit_time_secs > 0 && status.end_time_secs == 0 {
            return false;
        }

        true
    }

    pub(crate) async fn quest_source_item_quest_log_item_id_like_cpp(
        &mut self,
        entry_id: u32,
    ) -> u32 {
        if let Some(quest_log_item_id) =
            self.item_template_addon_quest_log_item_id_like_cpp(entry_id)
        {
            return quest_log_item_id;
        }

        let Some(world_db) = self.world_db().map(Arc::clone) else {
            return 0;
        };

        let mut stmt = world_db.prepare(WorldStatements::SEL_ITEM_TEMPLATE_ADDON_LOOT_METADATA);
        stmt.set_u32(0, entry_id);

        let quest_log_item_id = match world_db.query(&stmt).await {
            Ok(result) if !result.is_empty() => result
                .try_read::<i32>(1)
                .unwrap_or(0)
                .try_into()
                .unwrap_or(0),
            Ok(_) => 0,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    entry_id,
                    ?error,
                    "QuestConfirmAccept: failed to load item_template_addon QuestLogItemId"
                );
                0
            }
        };
        self.cache_item_template_addon_quest_log_item_id_like_cpp(entry_id, quest_log_item_id);
        quest_log_item_id
    }

    pub(crate) async fn apply_quest_source_item_added_non_bound_objective_progress_like_cpp(
        &mut self,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> Vec<u32> {
        self.apply_quest_item_added_objective_progress_filtered_like_cpp(
            entry_id,
            quest_log_item_id,
            count,
            Some(false),
        )
        .await
    }

    /// C++ `Player::ItemAddedQuestCheck(entry, count)` without a bound-item
    /// filter, as used by bank withdrawals after `StoreItem`.
    pub(crate) async fn apply_quest_item_added_objective_progress_like_cpp(
        &mut self,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> Vec<u32> {
        self.apply_quest_item_added_objective_progress_filtered_like_cpp(
            entry_id,
            quest_log_item_id,
            count,
            None,
        )
        .await
    }

    async fn apply_quest_item_added_objective_progress_filtered_like_cpp(
        &mut self,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
        bound_item_requirement: Option<bool>,
    ) -> Vec<u32> {
        use wow_packet::packets::quest::QuestUpdateComplete;

        let Some(quest_store) = self.quest_store.clone() else {
            return Vec::new();
        };
        let added_count = count;
        let count = i32::try_from(count).unwrap_or(i32::MAX);
        let entry_object_id = i32::try_from(entry_id).unwrap_or(i32::MAX);
        let mut objective_ids = vec![entry_object_id];
        let mut matching_entry_objectives = Vec::new();
        'matching_entry: for status in self.player_quests.values() {
            if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                continue;
            }
            let Some(quest) = quest_store.get(status.quest_id) else {
                continue;
            };
            for (objective_index, objective) in quest.objectives.iter().enumerate() {
                let is_bound = (objective.flags2
                    & QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL)
                    != 0;
                let passes_filter =
                    bound_item_requirement.is_none_or(|required_bound| required_bound == is_bound);
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
                    || !passes_filter
                    || current >= objective.amount
                    || !Self::represented_quest_objective_completable_like_cpp(
                        status,
                        quest,
                        objective_index,
                    )
                {
                    continue;
                }
                matching_entry_objectives.push(is_bound);
                if is_bound {
                    break 'matching_entry;
                }
            }
        }
        let should_update_quest_log_item = quest_log_item_id != 0
            && (matching_entry_objectives.len() != 1 || !matching_entry_objectives[0]);
        if should_update_quest_log_item {
            objective_ids.push(i32::try_from(quest_log_item_id).unwrap_or(i32::MAX));
        }

        let mut changed_quest_ids = Vec::new();
        let mut quests_to_complete = Vec::new();
        let mut objective_updates = Vec::new();
        'quests: for status in self.player_quests.values_mut() {
            if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                continue;
            }

            let Some(quest) = quest_store.get(status.quest_id) else {
                continue;
            };

            for (objective_index, objective) in quest.objectives.iter().enumerate() {
                if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL {
                    continue;
                }
                let is_bound = (objective.flags2
                    & QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL)
                    != 0;
                if bound_item_requirement.is_some_and(|required_bound| required_bound != is_bound) {
                    continue;
                }
                if !objective_ids.contains(&objective.object_id) {
                    continue;
                }
                if !Self::represented_quest_objective_completable_like_cpp(
                    status,
                    quest,
                    objective_index,
                ) {
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
                status.objective_counts[storage_index] =
                    current.saturating_add(count).clamp(0, objective.amount);
                let new_count = status.objective_counts[storage_index];
                if !changed_quest_ids.contains(&status.quest_id) {
                    changed_quest_ids.push(status.quest_id);
                }
                if count > 0 {
                    objective_updates.push((new_count, is_bound));
                }
                let quest_already_rewarded = self.rewarded_quests.contains(&status.quest_id);
                if new_count >= objective.amount
                    && Self::represented_can_complete_quest_after_objective_like_cpp(
                        status,
                        quest,
                        objective.id,
                        quest_already_rewarded,
                    )
                {
                    quests_to_complete.push(status.quest_id);
                }
                if is_bound {
                    break 'quests;
                }
            }
        }
        for quest_id in quests_to_complete {
            if let Some(quest) = quest_store.get(quest_id).cloned() {
                let completed = self
                    .complete_represented_quest_after_add_if_ready_like_cpp(&quest)
                    .await;
                if completed
                    && self
                        .player_quests
                        .get(&quest_id)
                        .is_some_and(|status| status.status == QUEST_STATUS_COMPLETE_LIKE_CPP)
                {
                    self.send_packet(&QuestUpdateComplete { quest_id });
                }
            }
        }
        if objective_updates.len() == 1 && objective_updates[0].1 {
            self.send_quest_bound_item_update_like_cpp(
                entry_id,
                quest_log_item_id,
                added_count,
                u32::try_from(objective_updates[0].0.max(0)).unwrap_or(u32::MAX),
            );
        }
        self.sync_player_registry_state_like_cpp();
        changed_quest_ids
    }

    /// C++ `Player::SendQuestUpdateAddItem`: ITEM objectives never use
    /// `SMSG_QUEST_UPDATE_ADD_CREDIT`; a single quest-bound objective uses
    /// `SMSG_ITEM_PUSH_RESULT` display type 3 instead.
    fn send_quest_bound_item_update_like_cpp(
        &self,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
        quantity_in_inventory: u32,
    ) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        let delivery = if self
            .item_template_flags3(entry_id)
            .is_some_and(|flags| (flags & ItemFlags3::DontReportLootLogToParty as u32) != 0)
        {
            SendNewItemDelivery::Direct
        } else {
            SendNewItemDelivery::GroupBroadcast
        };

        self.send_new_item_plan(&SendNewItemPlan {
            player_guid,
            item_guid: ObjectGuid::EMPTY,
            item_entry: entry_id,
            item_instance: SendNewItemInstancePlan {
                item_id: entry_id,
                random_properties_seed: 0,
                random_properties_id: 0,
                modifications: Vec::new(),
            },
            slot: u8::from(wow_entities::INVENTORY_SLOT_BAG_0),
            slot_in_bag: 0,
            quest_log_item_id,
            quantity: count,
            quantity_in_inventory,
            dungeon_encounter_id: 0,
            battle_pet_species_id: 0,
            battle_pet_breed_id: 0,
            battle_pet_breed_quality: 0,
            battle_pet_level: 0,
            pushed: false,
            created: false,
            is_encounter_loot: false,
            display_text: SendNewItemDisplayText::QuestUpdateAddItem,
            delivery,
        });
    }

    pub(super) fn apply_quest_item_removed_to_statuses_like_cpp(
        quest_store: &QuestStore,
        player_quests: &mut HashMap<u32, PlayerQuestStatus>,
        entry_id: u32,
        new_non_bank_item_count: u32,
    ) -> Vec<u32> {
        let Ok(object_id) = i32::try_from(entry_id) else {
            return Vec::new();
        };
        let new_item_count = i32::try_from(new_non_bank_item_count).unwrap_or(i32::MAX);
        let mut changed_quest_ids = Vec::new();

        for status in player_quests.values_mut() {
            let Some(quest) = quest_store.get(status.quest_id) else {
                continue;
            };
            for (objective_index, objective) in quest.objectives.iter().enumerate() {
                if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL
                    || objective.object_id != object_id
                    || !Self::represented_quest_objective_completable_like_cpp(
                        status,
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
                if status.objective_counts[storage_index] == new_item_count
                    && status.status == QUEST_STATUS_INCOMPLETE_LIKE_CPP
                {
                    continue;
                }
                status.objective_counts[storage_index] = new_item_count.max(0);
                status.status = QUEST_STATUS_INCOMPLETE_LIKE_CPP;
                changed_quest_ids.push(status.quest_id);
            }
        }
        changed_quest_ids.sort_unstable();
        changed_quest_ids.dedup();
        changed_quest_ids
    }

    pub(super) fn apply_quest_item_added_non_bound_to_statuses_like_cpp(
        quest_store: &QuestStore,
        rewarded_quests: &HashSet<u32>,
        player_quests: &mut HashMap<u32, PlayerQuestStatus>,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> Vec<u32> {
        let entry_object_id = i32::try_from(entry_id).unwrap_or(i32::MAX);
        let mut objective_ids = vec![entry_object_id];
        if quest_log_item_id != 0 {
            objective_ids.push(i32::try_from(quest_log_item_id).unwrap_or(i32::MAX));
        }
        let count = i32::try_from(count).unwrap_or(i32::MAX);
        let mut changed_quest_ids = Vec::new();
        let mut quests_to_complete = Vec::new();

        for status in player_quests.values_mut() {
            if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                continue;
            }
            let Some(quest) = quest_store.get(status.quest_id) else {
                continue;
            };
            for (objective_index, objective) in quest.objectives.iter().enumerate() {
                if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL
                    || (objective.flags2 & QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL)
                        != 0
                    || !objective_ids.contains(&objective.object_id)
                    || !Self::represented_quest_objective_completable_like_cpp(
                        status,
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
                let new_count = current.saturating_add(count).clamp(0, objective.amount);
                status.objective_counts[storage_index] = new_count;
                changed_quest_ids.push(status.quest_id);
                if new_count >= objective.amount
                    && Self::represented_can_complete_quest_after_objective_like_cpp(
                        status,
                        quest,
                        objective.id,
                        rewarded_quests.contains(&status.quest_id),
                    )
                {
                    quests_to_complete.push(status.quest_id);
                }
            }
        }
        for quest_id in quests_to_complete {
            if let Some(status) = player_quests.get_mut(&quest_id) {
                status.status = QUEST_STATUS_COMPLETE_LIKE_CPP;
            }
        }
        changed_quest_ids.sort_unstable();
        changed_quest_ids.dedup();
        changed_quest_ids
    }

    pub(super) fn apply_quest_item_added_bound_to_statuses_like_cpp(
        quest_store: &QuestStore,
        rewarded_quests: &HashSet<u32>,
        player_quests: &mut HashMap<u32, PlayerQuestStatus>,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> Option<(u32, i32)> {
        let count = i32::try_from(count).unwrap_or(i32::MAX);
        let mut ordered_quest_ids = player_quests
            .values()
            .map(|status| (status.slot, status.quest_id))
            .collect::<Vec<_>>();
        ordered_quest_ids.sort_unstable();

        for object_id in [entry_id, quest_log_item_id] {
            if object_id == 0 {
                continue;
            }
            let object_id = i32::try_from(object_id).unwrap_or(i32::MAX);
            for &(_, quest_id) in &ordered_quest_ids {
                let Some(status) = player_quests.get_mut(&quest_id) else {
                    continue;
                };
                if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                    continue;
                }
                let Some(quest) = quest_store.get(status.quest_id) else {
                    continue;
                };
                for (objective_index, objective) in quest.objectives.iter().enumerate() {
                    if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL
                        || (objective.flags2
                            & QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL)
                            == 0
                        || objective.object_id != object_id
                        || !Self::represented_quest_objective_completable_like_cpp(
                            status,
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
                    let new_count = current.saturating_add(count).clamp(0, objective.amount);
                    status.objective_counts[storage_index] = new_count;
                    if new_count >= objective.amount
                        && Self::represented_can_complete_quest_after_objective_like_cpp(
                            status,
                            quest,
                            objective.id,
                            rewarded_quests.contains(&status.quest_id),
                        )
                    {
                        status.status = QUEST_STATUS_COMPLETE_LIKE_CPP;
                    }
                    return Some((status.quest_id, new_count));
                }
            }
        }
        None
    }

    /// C++ `Player::ItemRemovedQuestCheck`: after the inventory mutation,
    /// recompute matching item objectives from carried (non-bank) contents and
    /// move completed quests back to incomplete when the requirement is lost.
    pub(crate) fn apply_quest_item_removed_like_cpp(&mut self, entry_id: u32) -> Vec<u32> {
        self.invalidate_player_quest_status_authority_like_cpp();
        let Some(quest_store) = self.quest_store.clone() else {
            return Vec::new();
        };
        let new_non_bank_item_count = self.represented_non_bank_item_count_like_cpp(entry_id);
        let changed_quest_ids = Self::apply_quest_item_removed_to_statuses_like_cpp(
            quest_store.as_ref(),
            &mut self.player_quests,
            entry_id,
            new_non_bank_item_count,
        );
        let changed_slots = changed_quest_ids
            .iter()
            .filter_map(|quest_id| self.player_quests.get(quest_id).map(|status| status.slot))
            .collect::<Vec<_>>();
        for slot in changed_slots {
            self.send_represented_quest_log_slot_update_like_cpp(slot);
        }
        let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
        self.sync_player_registry_state_like_cpp();
        changed_quest_ids
    }

    pub(crate) fn apply_quest_item_added_non_bound_state_like_cpp(
        &mut self,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> Vec<u32> {
        self.invalidate_player_quest_status_authority_like_cpp();
        let Some(quest_store) = self.quest_store.clone() else {
            return Vec::new();
        };
        Self::apply_quest_item_added_non_bound_to_statuses_like_cpp(
            quest_store.as_ref(),
            &self.rewarded_quests,
            &mut self.player_quests,
            entry_id,
            quest_log_item_id,
            count,
        )
    }

    pub(crate) fn apply_quest_item_added_bound_state_like_cpp(
        &mut self,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> Vec<u32> {
        self.invalidate_player_quest_status_authority_like_cpp();
        let Some(quest_store) = self.quest_store.clone() else {
            return Vec::new();
        };
        let Some((quest_id, new_count)) = Self::apply_quest_item_added_bound_to_statuses_like_cpp(
            quest_store.as_ref(),
            &self.rewarded_quests,
            &mut self.player_quests,
            entry_id,
            quest_log_item_id,
            count,
        ) else {
            return Vec::new();
        };
        self.send_quest_bound_item_update_like_cpp(
            entry_id,
            quest_log_item_id,
            count,
            u32::try_from(new_count.max(0)).unwrap_or(u32::MAX),
        );
        vec![quest_id]
    }

    pub(crate) fn publish_quest_item_added_status_changes_like_cpp(
        &mut self,
        changed_quest_ids: &[u32],
    ) {
        use wow_packet::packets::quest::QuestUpdateComplete;

        let mut changed_slots = changed_quest_ids
            .iter()
            .filter_map(|quest_id| self.player_quests.get(quest_id).map(|status| status.slot))
            .collect::<Vec<_>>();
        changed_slots.sort_unstable();
        changed_slots.dedup();
        for slot in changed_slots {
            self.send_represented_quest_log_slot_update_like_cpp(slot);
        }
        for &quest_id in changed_quest_ids {
            if self
                .player_quests
                .get(&quest_id)
                .is_some_and(|status| status.status == QUEST_STATUS_COMPLETE_LIKE_CPP)
            {
                self.send_packet(&QuestUpdateComplete { quest_id });
            }
        }
        self.sync_player_registry_state_like_cpp();
    }

    /// C++ walks one objective-status index and stops at the first quest-bound
    /// item objective. Rust stores statuses in a `HashMap`, so two independent
    /// scans could select different quests. Use the explicit quest-log slot
    /// (then quest id as a deterministic duplicate-slot fallback) for both the
    /// durable plan and its post-commit application.
    pub(super) fn quest_bound_item_objective_quest_order_like_cpp(&self) -> Vec<u32> {
        let mut quests = self
            .player_quests
            .values()
            .map(|status| (status.slot, status.quest_id))
            .collect::<Vec<_>>();
        quests.sort_unstable();
        quests.into_iter().map(|(_, quest_id)| quest_id).collect()
    }

    pub(super) async fn apply_quest_source_item_bound_objective_progress_for_object_like_cpp(
        &mut self,
        quest_store: &QuestStore,
        object_id: i32,
        count_i32: i32,
    ) -> Vec<(u32, i32)> {
        self.invalidate_player_quest_status_authority_like_cpp();
        let mut updated_counts = Vec::new();
        let mut quests_to_complete = Vec::new();
        let ordered_quest_ids = self.quest_bound_item_objective_quest_order_like_cpp();

        'quests: for quest_id in ordered_quest_ids {
            let Some(status) = self.player_quests.get_mut(&quest_id) else {
                continue;
            };
            if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                continue;
            }

            let Some(quest) = quest_store.get(status.quest_id) else {
                continue;
            };

            for (objective_index, objective) in quest.objectives.iter().enumerate() {
                if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL {
                    continue;
                }
                if (objective.flags2 & QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL) == 0
                {
                    continue;
                }
                if objective.object_id != object_id {
                    continue;
                }
                if !Self::represented_quest_objective_completable_like_cpp(
                    status,
                    quest,
                    objective_index,
                ) {
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
                let new_count = current.saturating_add(count_i32).clamp(0, objective.amount);
                status.objective_counts[storage_index] = new_count;
                updated_counts.push((status.quest_id, new_count));
                let quest_already_rewarded = self.rewarded_quests.contains(&status.quest_id);
                if new_count >= objective.amount
                    && Self::represented_can_complete_quest_after_objective_like_cpp(
                        status,
                        quest,
                        objective.id,
                        quest_already_rewarded,
                    )
                {
                    quests_to_complete.push(status.quest_id);
                }
                // C++ `UpdateQuestObjectiveProgress` stops after the first
                // credited quest-bound Item objective.
                break 'quests;
            }
        }

        for quest_id in quests_to_complete {
            if let Some(quest) = quest_store.get(quest_id).cloned() {
                self.complete_represented_quest_after_add_if_ready_like_cpp(&quest)
                    .await;
            }
        }

        updated_counts
    }

    pub(crate) async fn apply_quest_source_item_bound_objective_preflight_like_cpp(
        &mut self,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> Option<QuestSourceItemBoundPreflightLikeCpp> {
        let Some(_player_guid) = self.player_guid() else {
            return None;
        };
        let Some(quest_store) = self.quest_store.clone() else {
            return None;
        };
        let count_i32 = i32::try_from(count).unwrap_or(i32::MAX);
        let entry_object_id = i32::try_from(entry_id).unwrap_or(i32::MAX);
        let mut updated_counts = self
            .apply_quest_source_item_bound_objective_progress_for_object_like_cpp(
                quest_store.as_ref(),
                entry_object_id,
                count_i32,
            )
            .await;

        if quest_log_item_id != 0 && updated_counts.len() != 1 {
            let quest_log_object_id = i32::try_from(quest_log_item_id).unwrap_or(i32::MAX);
            updated_counts.extend(
                self.apply_quest_source_item_bound_objective_progress_for_object_like_cpp(
                    quest_store.as_ref(),
                    quest_log_object_id,
                    count_i32,
                )
                .await,
            );
        }

        if updated_counts.is_empty() {
            return None;
        }

        self.sync_player_registry_state_like_cpp();
        let mut changed_quest_ids = Vec::new();
        for &(quest_id, _) in &updated_counts {
            if !changed_quest_ids.contains(&quest_id) {
                changed_quest_ids.push(quest_id);
            }
        }

        if updated_counts.len() != 1 {
            return Some(QuestSourceItemBoundPreflightLikeCpp {
                no_grant: false,
                changed_quest_ids,
            });
        }

        self.send_quest_bound_item_update_like_cpp(
            entry_id,
            quest_log_item_id,
            count,
            u32::try_from(updated_counts[0].1.max(0)).unwrap_or(u32::MAX),
        );
        Some(QuestSourceItemBoundPreflightLikeCpp {
            no_grant: true,
            changed_quest_ids,
        })
    }
}
