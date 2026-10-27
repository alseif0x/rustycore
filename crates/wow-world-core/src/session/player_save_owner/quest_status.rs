//! Native Player quest-status dirty projection for `_SaveQuestStatus`.

use wow_entities::{
    Player, PlayerQuestNativeSaveProjectionLikeCpp, PlayerQuestObjectiveSaveValueLikeCpp,
    PlayerQuestStatusSaveProjectionLikeCpp, PlayerQuestStatusSaveTypeLikeCpp as SaveType,
};
use wow_persistence::{
    PlayerQuestStatusPersistenceRequestLikeCpp as Request, QuestObjectiveCountPersistenceLikeCpp,
    QuestStatusPersistenceLikeCpp,
};

pub(super) struct QuestStatusCaptureLikeCpp {
    pub requests: Option<Vec<Request>>,
    pub acknowledgement_projection: Vec<PlayerQuestStatusSaveProjectionLikeCpp>,
}

pub(super) fn capture_like_cpp(
    player: &Player,
    quest_store: Option<&wow_data::quest::QuestStore>,
    keep_abandoned: Option<bool>,
) -> Option<QuestStatusCaptureLikeCpp> {
    let intents = player
        .gameplay_state()
        .quests
        .status_save_intents_like_cpp();
    if intents.is_empty() {
        return Some(QuestStatusCaptureLikeCpp {
            requests: None,
            acknowledgement_projection: Vec::new(),
        });
    }

    let owner_guid = player.guid().counter() as u64;
    let rows = player.quest_log_values_like_cpp();
    let mut requests = Vec::new();
    let mut acknowledgement_projection = Vec::with_capacity(intents.len());

    for intent in intents {
        let status = player
            .gameplay_state()
            .quests
            .status_like_cpp(intent.quest_id);
        match intent.save_type {
            SaveType::Delete | SaveType::ForceDelete => {
                requests.push(Request::Delete {
                    owner_guid,
                    quest_id: intent.quest_id,
                });
                acknowledgement_projection.push(PlayerQuestStatusSaveProjectionLikeCpp {
                    quest_id: intent.quest_id,
                    native: None,
                });
            }
            SaveType::Default => {
                let Some(status) = status else {
                    // Source Default with no current m_QuestStatus record emits no SQL.
                    acknowledgement_projection.push(PlayerQuestStatusSaveProjectionLikeCpp {
                        quest_id: intent.quest_id,
                        native: None,
                    });
                    continue;
                };
                if status.status == wow_conditions::QUEST_STATUS_NONE_LIKE_CPP {
                    match keep_abandoned {
                        Some(false) => {
                            // Source consumes this dirty key without SQL when abandoned
                            // rows are not retained.
                            acknowledgement_projection.push(
                                PlayerQuestStatusSaveProjectionLikeCpp {
                                    quest_id: intent.quest_id,
                                    native: None,
                                },
                            );
                            continue;
                        }
                        None => return None,
                        Some(true) => {}
                    }
                }

                let quest = quest_store?.get(intent.quest_id)?;
                let slot = usize::from(status.slot);
                let row = rows.get(slot)?;
                let mut objectives = Vec::new();
                let mut objective_projection = Vec::new();
                for objective in &quest.objectives {
                    let Ok(storage_index) = u8::try_from(objective.storage_index) else {
                        continue;
                    };
                    if usize::from(storage_index) >= 24 {
                        continue;
                    }
                    let is_storing_flag = objective.is_storing_flag_like_cpp();
                    let count =
                        row.objective_data_for_save_like_cpp(storage_index, is_storing_flag)?;
                    objective_projection.push(PlayerQuestObjectiveSaveValueLikeCpp {
                        storage_index,
                        is_storing_flag,
                        value: count,
                    });
                    if count != 0 {
                        objectives.push(QuestObjectiveCountPersistenceLikeCpp {
                            objective_index: storage_index,
                            count: i32::from(count),
                        });
                    }
                }
                requests.push(Request::Save {
                    owner_guid,
                    status: QuestStatusPersistenceLikeCpp {
                        quest_id: intent.quest_id,
                        status: status.status,
                        explored: status.explored,
                        accept_time_secs: status.accept_time_secs,
                        end_time_secs: row.end_time(),
                        objectives,
                    },
                });
                acknowledgement_projection.push(PlayerQuestStatusSaveProjectionLikeCpp {
                    quest_id: intent.quest_id,
                    native: Some(PlayerQuestNativeSaveProjectionLikeCpp {
                        slot: status.slot,
                        end_time_secs: row.end_time(),
                        objectives: objective_projection,
                    }),
                });
            }
        }
    }

    Some(QuestStatusCaptureLikeCpp {
        requests: Some(requests),
        acknowledgement_projection,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_quest_owner_needs_no_catalog_and_selects_no_save_group() {
        let player = Player::new(None, false);
        let capture = capture_like_cpp(&player, None, None).unwrap();
        assert!(capture.requests.is_none());
        assert!(capture.acknowledgement_projection.is_empty());
    }

    #[test]
    fn deleted_and_missing_status_intents_survive_capture_until_commit() {
        let mut player = Player::new(None, false);
        let quests = &mut player.gameplay_state_mut().quests;
        quests.mark_status_save_like_cpp(3, SaveType::ForceDelete);
        quests.mark_status_save_like_cpp(1, SaveType::Default);
        quests.mark_status_save_like_cpp(2, SaveType::Delete);
        let capture = capture_like_cpp(&player, None, None).unwrap();
        assert_eq!(capture.requests.as_ref().unwrap().len(), 2);
        assert_eq!(
            capture
                .acknowledgement_projection
                .iter()
                .map(|row| row.quest_id)
                .collect::<Vec<_>>(),
            vec![1, 2, 3],
        );
        assert_eq!(
            player
                .gameplay_state()
                .quests
                .status_save_intents_like_cpp()
                .len(),
            3
        );
        let receipt = player.capture_save_acknowledgement_with_quest_status_like_cpp(
            &[],
            &[],
            &capture.acknowledgement_projection,
        );
        player.acknowledge_saved_projection_like_cpp(
            receipt,
            wow_entities::PlayerSavedGroupsLikeCpp {
                quest_statuses: true,
                ..Default::default()
            },
        );
        assert!(
            player
                .gameplay_state()
                .quests
                .status_save_intents_like_cpp()
                .is_empty()
        );
    }

    #[test]
    fn abandoned_status_needs_policy_but_known_skip_is_acknowledgeable() {
        let mut player = Player::new(None, false);
        player
            .gameplay_state_mut()
            .quests
            .set_status_like_cpp(7, 0, true);
        assert!(capture_like_cpp(&player, None, None).is_none());
        let capture = capture_like_cpp(&player, None, Some(false)).unwrap();
        assert_eq!(capture.requests.unwrap().len(), 0);
        assert_eq!(capture.acknowledgement_projection.len(), 1);
        assert_eq!(
            player.gameplay_state().quests.status_save_type_like_cpp(7),
            Some(SaveType::Default)
        );
    }

    #[test]
    fn missing_catalog_never_partially_acknowledges_a_dirty_failed_quest() {
        let mut player = Player::new(None, false);
        player
            .gameplay_state_mut()
            .quests
            .mark_status_save_like_cpp(1, SaveType::Delete);
        player.gameplay_state_mut().quests.set_status_like_cpp(
            2,
            wow_conditions::QUEST_STATUS_FAILED_LIKE_CPP,
            true,
        );
        assert!(capture_like_cpp(&player, None, Some(true)).is_none());
        assert_eq!(
            player
                .gameplay_state()
                .quests
                .status_save_intents_like_cpp()
                .len(),
            2
        );
    }
}
