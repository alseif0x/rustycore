// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

impl crate::session::SessionCatalogs {
    pub fn void_storage_quest_status_writes_like_cpp(
        &self,
        statuses: &[wow_entities::PlayerQuestStatusRecord],
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
}

impl crate::session::HubRef<'_> {
    pub fn represented_quest_status_persistence_rows_like_cpp(
        &self,
        statuses: &[wow_entities::PlayerQuestStatusRecord],
    ) -> Vec<wow_persistence::QuestStatusPersistenceLikeCpp> {
        statuses
            .iter()
            .map(|status| {
                self.catalogs
                    .represented_quest_status_persistence_like_cpp(status)
            })
            .collect()
    }
}

impl crate::session::SessionCatalogs {
    pub fn represented_quest_status_persistence_like_cpp(
        &self,
        status: &wow_entities::PlayerQuestStatusRecord,
    ) -> wow_persistence::QuestStatusPersistenceLikeCpp {
        let objectives = self
            .quests
            .store
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
}
