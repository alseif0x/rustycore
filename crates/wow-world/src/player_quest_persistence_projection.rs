// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! SQLx-free projection of represented Player quest state for atomic workflows.

use crate::{WorldSession, handlers::quest::PlayerQuestStatus};

impl WorldSession {
    pub(crate) fn represented_quest_status_persistence_like_cpp(
        &self,
        status: &PlayerQuestStatus,
    ) -> wow_persistence::QuestStatusPersistenceLikeCpp {
        let objectives = self
            .quest_store
            .as_ref()
            .and_then(|store| store.get(status.quest_id))
            .map(|quest| {
                quest
                    .objectives
                    .iter()
                    .filter_map(|objective| {
                        let objective_index = u8::try_from(objective.storage_index).ok()?;
                        let count = status
                            .objective_counts
                            .get(usize::from(objective_index))
                            .copied()
                            .unwrap_or(0);
                        (count != 0).then_some(
                            wow_persistence::QuestObjectiveCountPersistenceLikeCpp {
                                objective_index,
                                count,
                            },
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        wow_persistence::QuestStatusPersistenceLikeCpp {
            quest_id: status.quest_id,
            status: status.status,
            explored: status.explored,
            accept_time_secs: status.accept_time_secs,
            end_time_secs: status.end_time_secs,
            objectives,
        }
    }

    pub(crate) fn represented_quest_status_persistence_rows_like_cpp(
        &self,
        statuses: &[PlayerQuestStatus],
    ) -> Vec<wow_persistence::QuestStatusPersistenceLikeCpp> {
        statuses
            .iter()
            .map(|status| self.represented_quest_status_persistence_like_cpp(status))
            .collect()
    }
}
