//! bound items operations at the existing Quest application boundary.

use super::*;

impl WorldSession {

    /// C++ walks one objective-status index and stops at the first quest-bound
    /// item objective. Durable plans use a `HashMap`, while live Player status
    /// uses a `BTreeMap`. Use the explicit quest-log slot
    /// (then quest id as a deterministic duplicate-slot fallback) for both the
    /// durable plan and its post-commit application.
    pub(in crate::handlers::quest) fn quest_bound_item_objective_quest_order_like_cpp(&self) -> Vec<u32> {
        let mut quests = self
            .player_quest_gameplay_snapshot_like_cpp()
            .into_iter()
            .flat_map(|state| state.statuses_snapshot_like_cpp().into_values())
            .map(|status| (status.slot, status.quest_id))
            .collect::<Vec<_>>();
        quests.sort_unstable();
        quests.into_iter().map(|(_, quest_id)| quest_id).collect()
    }

    pub(in crate::handlers::quest) async fn apply_quest_source_item_bound_objective_progress_for_object_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        quest_store: &QuestStore,
        object_id: i32,
        count_i32: i32,
    ) -> Vec<(u32, i32)> {
        self.invalidate_player_quest_status_authority_like_cpp();
        let ordered_quest_ids = self.quest_bound_item_objective_quest_order_like_cpp();
        let Some(wow_entities::QuestBoundItemObjectiveProgressLikeCpp {
            updated_counts,
            quests_to_complete,
        }) = self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.apply_bound_item_objective_progress_like_cpp(
                |id| {
                    quest_store
                        .get(id)
                        .map(|quest| quest.objective_rules_like_cpp())
                },
                ordered_quest_ids,
                object_id,
                count_i32,
            )
        })
        else {
            return Vec::new();
        };

        for quest_id in quests_to_complete {
            if let Some(quest) = quest_store.get(quest_id).cloned() {
                self.complete_represented_quest_after_add_with_generator_like_cpp(
                    item_guid_generator,
                    &quest,
                )
                .await;
            }
        }

        updated_counts
    }

    #[cfg(feature = "test-fixtures")]
    pub(crate) async fn apply_quest_source_item_bound_objective_progress_for_object_for_test(
        &mut self,
        quest_store: &QuestStore,
        object_id: i32,
        count_i32: i32,
    ) -> Vec<(u32, i32)> {
        let generators = self.id_generators_for_test_like_cpp();
        self.apply_quest_source_item_bound_objective_progress_for_object_with_generator_like_cpp(
            generators.item.as_ref(),
            quest_store,
            object_id,
            count_i32,
        )
        .await
    }

    #[cfg(test)]
    pub(in crate::handlers::quest) async fn apply_quest_source_item_bound_objective_progress_for_object_like_cpp(
        &mut self,
        quest_store: &QuestStore,
        object_id: i32,
        count_i32: i32,
    ) -> Vec<(u32, i32)> {
        let generators = self.id_generators_for_test_like_cpp();
        self.apply_quest_source_item_bound_objective_progress_for_object_with_generator_like_cpp(
            generators.item.as_ref(),
            quest_store,
            object_id,
            count_i32,
        )
        .await
    }

    pub(crate) async fn apply_quest_source_item_bound_objective_preflight_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> Option<QuestSourceItemBoundPreflightLikeCpp> {
        let Some(_player_guid) = self.player_guid() else {
            return None;
        };
        let Some(quest_store) = self.quests.store.clone() else {
            return None;
        };
        let count_i32 = i32::try_from(count).unwrap_or(i32::MAX);
        let entry_object_id = i32::try_from(entry_id).unwrap_or(i32::MAX);
        let mut updated_counts = self
            .apply_quest_source_item_bound_objective_progress_for_object_with_generator_like_cpp(
                item_guid_generator,
                quest_store.as_ref(),
                entry_object_id,
                count_i32,
            )
            .await;

        if quest_log_item_id != 0 && updated_counts.len() != 1 {
            let quest_log_object_id = i32::try_from(quest_log_item_id).unwrap_or(i32::MAX);
            updated_counts.extend(
                self.apply_quest_source_item_bound_objective_progress_for_object_with_generator_like_cpp(
                    item_guid_generator,
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

    #[cfg(test)]
    pub(crate) async fn apply_quest_source_item_bound_objective_preflight_like_cpp(
        &mut self,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> Option<QuestSourceItemBoundPreflightLikeCpp> {
        let generators = self.id_generators_for_test_like_cpp();
        self.apply_quest_source_item_bound_objective_preflight_with_generator_like_cpp(
            generators.item.as_ref(),
            entry_id,
            quest_log_item_id,
            count,
        )
        .await
    }
}
