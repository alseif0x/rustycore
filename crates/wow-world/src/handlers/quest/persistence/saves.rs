//! Quest saves operation at the application boundary.
use super::*;

impl WorldSession {


    pub(crate) async fn save_represented_quest_status_like_cpp(&self, quest_id: u32) {
        if let Some(status) = self
            .player_quest_gameplay_snapshot_like_cpp()
            .and_then(|state| {
                state
                    .statuses_like_cpp()
                    .get(&quest_id)
                    .map(|status| status.status)
            })
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

    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn represented_quest_statuses_for_save_like_cpp(&self) -> Vec<(u32, u8)> {
        let Some(state) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return Vec::new();
        };
        let mut quests = state
            .statuses_like_cpp()
            .iter()
            .filter_map(|(quest_id, status)| {
                let store = self.quests.store.as_ref();
                if state.rewarded_quest_ids_like_cpp().contains(quest_id)
                    && store
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

    pub(in crate::handlers::quest) async fn save_represented_quest_statuses_completed_after_like_cpp(
        &mut self,
        completion_evidence_start: usize,
    ) {
        let completed_quest_ids: Vec<_> = self
            .quest_state
            .represented_quest_complete_status_updates_like_cpp[completion_evidence_start..]
            .iter()
            .filter_map(|evidence| {
                (evidence.new_status == QUEST_STATUS_COMPLETE_LIKE_CPP).then_some(evidence.quest_id)
            })
            .collect();
        for quest_id in completed_quest_ids {
            self.save_represented_quest_status_like_cpp(quest_id).await;
        }
    }

    /// Project the quest's durable status without writing it.
    ///
    /// Shared by the standalone save below and by the quest-reward operation,
    /// which carries the same row inside its own closing transaction rather
    /// than committing it separately.
    pub(crate) fn plan_quest_status_save_like_cpp(
        &self,
        quest_id: u32,
        status: u8,
    ) -> Option<wow_persistence::PlayerQuestStatusPersistenceRequestLikeCpp> {
        let owner_guid = self.player_guid()?.counter() as u64;
        let quest_state = self.player_quest_gameplay_snapshot_like_cpp();
        let mut projection = match quest_state
            .as_ref()
            .and_then(|state| state.statuses_like_cpp().get(&quest_id))
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
                return None;
            }
        };
        projection.status = status;
        Some(
            wow_persistence::PlayerQuestStatusPersistenceRequestLikeCpp::Save {
                owner_guid,
                status: projection,
            },
        )
    }

    pub(in crate::handlers::quest) async fn save_quest_to_db(&self, quest_id: u32, status: u8) {
        let port = match self.player_quest_persistence_port_like_cpp() {
            Some(port) => port,
            None => return,
        };
        let Some(request) = self.plan_quest_status_save_like_cpp(quest_id, status) else {
            return;
        };

        match port.persist_status_like_cpp(request).await {
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
    pub(in crate::handlers::quest) async fn delete_quest_from_db(&self, quest_id: u32) {
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
}
