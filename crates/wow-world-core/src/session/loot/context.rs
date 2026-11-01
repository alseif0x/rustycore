// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use crate::session::state::hub_support::RepresentedLootPlayerContext;
use wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP;

impl crate::session::SessionCatalogs {
    pub fn remote_has_incomplete_quest_objective_for_object_id_like_cpp(
        &self,
        item_object_id: i32,
        player_context: &RepresentedLootPlayerContext,
    ) -> bool {
        let Some(quest_store) = &self.quests.store else {
            return false;
        };

        player_context
            .active_quest_objective_counts
            .iter()
            .any(|(quest_id, objective_counts)| {
                if player_context.quest_status(*quest_id) != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                    return false;
                }

                let Some(quest) = quest_store.get(*quest_id) else {
                    return false;
                };

                quest
                    .objectives
                    .iter()
                    .enumerate()
                    .any(|(fallback_index, objective)| {
                        if objective.obj_type != 1 || objective.object_id != item_object_id {
                            return false;
                        }

                        let storage_index = usize::try_from(objective.storage_index)
                            .ok()
                            .unwrap_or(fallback_index);
                        let current = objective_counts.get(storage_index).copied().unwrap_or(0);
                        current < objective.amount.max(1)
                    })
            })
    }

    pub fn remote_player_quest_objective_progress_like_cpp(
        &self,
        objective_id: u32,
        player_context: &RepresentedLootPlayerContext,
    ) -> Option<i32> {
        let quest_store = self.quests.store.as_ref()?;

        for (quest_id, objective_counts) in &player_context.active_quest_objective_counts {
            let Some(quest) = quest_store.get(*quest_id) else {
                continue;
            };
            let Some((_, objective)) = quest
                .objectives
                .iter()
                .enumerate()
                .find(|(_, objective)| objective.id == objective_id)
            else {
                continue;
            };
            let objective_index = objective.storage_index.max(0) as usize;
            return Some(objective_counts.get(objective_index).copied().unwrap_or(0));
        }

        None
    }
}
