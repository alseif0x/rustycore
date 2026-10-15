//! The quest kill credit of a represented creature kill (C++
//! `QUEST_OBJECTIVE_MONSTER`). The kill reward queue, the reputation it awards
//! and the death-state completion moved to `wow_world_entities` (#1363 slice
//! 10b); this quest-bound step stays on the session as their host method.

use super::*;

impl WorldSession {
    /// Called when the player kills a creature. Checks all active kill-objective quests
    /// and updates progress. Sends SMSG_QUEST_UPDATE_ADD_CREDIT if progress was made.
    pub(crate) async fn on_creature_killed_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_entry: u32,
        creature_guid: wow_core::ObjectGuid,
    ) {
        // C++ QUEST_OBJECTIVE_MONSTER.
        self.quest_objective_operation_like_cpp(
            item_guid_generator,
            crate::session::quest::QuestObjectiveOperationLikeCpp::StoringValue {
                objective_type: 0,
                object_id: creature_entry as i32,
                add_count: 1,
                credit_guid: creature_guid,
            },
        )
        .await;
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/world_entities/creature_kill/f3_shims.rs"]
mod f3_shims;
