//! Native PlayerData QuestLog ownership scenarios.

use super::*;

/// Source: Player.cpp `_LoadQuestStatus` (18613-18707) filters NONE and
/// missing templates, assigns sequential live slots, writes every row's
/// end time, then clears the unused tail. `_LoadQuestStatusObjectives`
/// (18709-18748) casts objective counts to uint16 and sets storage flags.
#[tokio::test]
async fn native_quest_log_load_filters_invalid_rows_and_clears_stale_tail_like_cpp() {
    let (mut session, _send_rx) = make_session();
    install_native_quest_log_player_like_cpp(&mut session);
    let timed_id = 59_401;
    let second_id = 59_402;
    let none_id = 59_403;
    let missing_id = 59_404;
    let mut timed_quest = quest_template(timed_id);
    timed_quest.limit_time_secs = 600;
    timed_quest.objectives.push(QuestObjective {
        id: timed_id * 10,
        quest_id: timed_id,
        obj_type: QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: 44,
        amount: 5,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([
        timed_quest,
        quest_template(second_id),
        quest_template(none_id),
    ])));

    let fixture = PlayerQuestPersistencePortFixtureLikeCpp {
        active: vec![
            PlayerQuestActivePersistenceRowLikeCpp {
                quest_id: Some(none_id),
                status: Some(wow_conditions::QUEST_STATUS_NONE_LIKE_CPP),
                explored: Some(0),
                accept_time_secs: Some(0),
                end_time_secs: Some(0),
            },
            PlayerQuestActivePersistenceRowLikeCpp {
                quest_id: Some(missing_id),
                status: Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
                explored: Some(0),
                accept_time_secs: Some(0),
                end_time_secs: Some(0),
            },
            PlayerQuestActivePersistenceRowLikeCpp {
                quest_id: Some(timed_id),
                status: Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
                explored: Some(0),
                accept_time_secs: Some(12),
                end_time_secs: Some(34),
            },
            PlayerQuestActivePersistenceRowLikeCpp {
                quest_id: Some(second_id),
                status: Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
                explored: Some(0),
                accept_time_secs: Some(15),
                end_time_secs: Some(0),
            },
        ],
        objectives: vec![PlayerQuestObjectivePersistenceRowLikeCpp {
            quest_id: Some(timed_id),
            storage_index: Some(0),
            count: Some(65_537),
        }],
        ..Default::default()
    };
    session.set_player_quest_persistence_port_like_cpp(Arc::new(fixture));
    wow_world_quest::load_player_quests(&mut session).await;

    let entries = session.quest_log_create_entries_like_cpp();
    assert_eq!(
        entries[0],
        (timed_id, 0, 34, {
            let mut counters = [0; 24];
            counters[0] = 1;
            counters
        })
    );
    assert_eq!(entries[1].0, second_id);
    assert_eq!(entries[1].2, 0);
    assert!(canonical_player_quest_status_like_cpp(&session, none_id).is_none());
    assert!(canonical_player_quest_status_like_cpp(&session, missing_id).is_none());

    // C++ FindQuestSlot and first-free use live QuestID values, not status.Slot.
    session
        .mutate_player_quest_gameplay_like_cpp(|state| {
            state.status_mut_like_cpp(timed_id).unwrap().slot = 10;
            state.status_mut_like_cpp(second_id).unwrap().slot = 11;
        })
        .expect("canonical quest status owner");
    assert_eq!(session.first_free_quest_slot_like_cpp(), Some(2));
    let owner = session.core.quest_objective_access_like_cpp();
    assert_eq!(
        wow_world_quest::find_quest_slot_like_cpp(
            &owner,
            &session.quest_state,
            timed_id,
            cfg!(test),
        ),
        Some(0)
    );

    session.set_player_quest_persistence_port_like_cpp(Arc::new(
        PlayerQuestPersistencePortFixtureLikeCpp {
            active: vec![PlayerQuestActivePersistenceRowLikeCpp {
                quest_id: Some(second_id),
                status: Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
                explored: Some(0),
                accept_time_secs: Some(20),
                end_time_secs: Some(0),
            }],
            ..Default::default()
        },
    ));
    wow_world_quest::load_player_quests(&mut session).await;
    let fewer_entries = session.quest_log_create_entries_like_cpp();
    assert_eq!(fewer_entries[0].0, second_id);
    assert!(
        fewer_entries[1..]
            .iter()
            .all(|entry| *entry == (0, 0, 0, [0; 24]))
    );

    session.set_player_quest_persistence_port_like_cpp(Arc::new(
        PlayerQuestPersistencePortFixtureLikeCpp::default(),
    ));
    wow_world_quest::load_player_quests(&mut session).await;
    assert!(
        session
            .quest_log_create_entries_like_cpp()
            .iter()
            .all(|entry| *entry == (0, 0, 0, [0; 24]))
    );
    assert_eq!(session.first_free_quest_slot_like_cpp(), Some(0));
}

/// Source: Player.cpp `IncompleteQuest` (14526-14535) changes status first,
/// then clears COMPLETE only when FindQuestSlot finds a native QuestID.
#[test]
fn native_quest_log_missing_slot_does_not_block_incomplete_status_like_cpp() {
    let (mut session, _send_rx) = make_session();
    install_native_quest_log_player_like_cpp(&mut session);
    let quest_id = 59_405;
    add_active_quest_in_slot_with_status(&mut session, quest_id, 0, QUEST_STATUS_COMPLETE_LIKE_CPP);
    let owner = session.core.quest_objective_access_like_cpp();
    assert_eq!(owner.set_quest_slot_like_cpp(0, 0), Some(true));

    assert_eq!(
        owner.mark_quest_incomplete_if_complete_like_cpp(
            quest_id,
            QUEST_STATUS_COMPLETE_LIKE_CPP,
            QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        ),
        Some(true)
    );
    let snapshot = owner
        .player_quest_gameplay_snapshot_like_cpp()
        .expect("canonical quest status owner");
    let status = snapshot
        .status_like_cpp(quest_id)
        .expect("quest status persists without a live QuestLog row");
    assert_eq!(status.status, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    assert_eq!(session.get_quest_slot_quest_id_like_cpp(0), None);
}

/// An unavailable Rust catalog is not evidence that every persisted quest is
/// unknown. Keep the current canonical state and native rows while the load
/// remains incomplete; known missing templates are filtered by the loader.
#[tokio::test]
async fn native_quest_log_missing_catalog_preserves_existing_owner_state_like_cpp() {
    let (mut session, _send_rx) = make_session();
    install_native_quest_log_player_like_cpp(&mut session);
    let quest_id = 59_408;
    session
        .mutate_player_quest_gameplay_like_cpp(|state| {
            state.insert_status_like_cpp(
                quest_id,
                PlayerQuestStatus {
                    quest_id,
                    status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
                    explored: true,
                    accept_time_secs: 10,
                    end_time_secs: 0,
                    objective_counts: vec![2],
                    slot: 0,
                },
            );
            state.set_status_authority_complete_like_cpp(true);
        })
        .expect("canonical quest status owner");
    {
        let owner = session.core.quest_objective_access_like_cpp();
        assert_eq!(owner.set_quest_slot_like_cpp(0, quest_id), Some(true));
        assert_eq!(owner.set_quest_slot_counter_like_cpp(0, 0, 2), Some(true));
    }
    session.set_player_quest_persistence_port_like_cpp(Arc::new(
        PlayerQuestPersistencePortFixtureLikeCpp {
            active: vec![PlayerQuestActivePersistenceRowLikeCpp {
                quest_id: Some(quest_id + 100),
                status: Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
                explored: Some(0),
                accept_time_secs: Some(20),
                end_time_secs: Some(0),
            }],
            ..Default::default()
        },
    ));

    wow_world_quest::load_player_quests(&mut session).await;

    let owner = session.core.quest_objective_access_like_cpp();
    let retained = owner
        .player_quest_gameplay_snapshot_like_cpp()
        .expect("canonical quest status remains installed");
    assert_eq!(
        retained
            .status_like_cpp(quest_id)
            .map(|status| status.status),
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP)
    );
    assert!(!retained.status_authority_complete_like_cpp());
    let row = owner.quest_log_values_snapshot_like_cpp().unwrap()[0];
    assert_eq!(row.quest_id(), quest_id as i32);
    assert_eq!(row.objective_progress()[0], 2);
}

/// Source: Player.cpp `CompleteQuest` (14496-14505) addresses the live state
/// field through the preserved status.Slot member.
#[test]
fn native_quest_log_completion_sets_complete_on_status_slot_like_cpp() {
    let (mut session, _send_rx) = make_session();
    install_native_quest_log_player_like_cpp(&mut session);
    let quest_id = 59_406;
    session
        .mutate_player_quest_gameplay_like_cpp(|quests| {
            quests.insert_status_like_cpp(
                quest_id,
                PlayerQuestStatus {
                    quest_id,
                    status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
                    explored: false,
                    accept_time_secs: 0,
                    end_time_secs: 0,
                    objective_counts: Vec::new(),
                    slot: 4,
                },
            );
        })
        .expect("canonical quest status owner");
    let owner = session.core.quest_objective_access_like_cpp();
    assert_eq!(owner.set_quest_slot_like_cpp(4, quest_id), Some(true));
    assert_eq!(
        owner.complete_quest_status_like_cpp(
            quest_id,
            QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            QUEST_STATUS_COMPLETE_LIKE_CPP,
        ),
        Some(Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP))
    );
    assert_eq!(
        owner.quest_log_values_snapshot_like_cpp().unwrap()[4].state_flags() & 1,
        1
    );
}

/// Source: `SetQuestObjectiveData` (Player.cpp:16426-16490) writes through
/// `status.Slot`; `FindQuestSlot` and `GetQuestObjectiveData` use QuestID.
#[tokio::test]
async fn native_quest_log_objective_delta_uses_status_slot_while_lookup_uses_quest_id_like_cpp() {
    let (mut session, _send_rx) = make_session();
    install_native_quest_log_player_like_cpp(&mut session);
    let quest_id = 59_409;
    let item_id = 44_901;
    let mut quest = quest_template(quest_id);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: item_id as i32,
        amount: 5,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session
        .mutate_player_quest_gameplay_like_cpp(|state| {
            state.insert_status_like_cpp(
                quest_id,
                PlayerQuestStatus {
                    quest_id,
                    status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
                    explored: false,
                    accept_time_secs: 0,
                    end_time_secs: 0,
                    objective_counts: vec![0],
                    slot: 0,
                },
            );
        })
        .expect("canonical quest status owner");

    {
        let owner = session.core.quest_objective_access_like_cpp();
        assert_eq!(owner.set_quest_slot_like_cpp(4, quest_id), Some(true));
        assert_eq!(
            wow_world_quest::find_quest_slot_like_cpp(
                &owner,
                &session.quest_state,
                quest_id,
                cfg!(test),
            ),
            Some(4)
        );
    }
    let generator = ObjectGuidGenerator::new(HighGuid::Item, 1);
    assert_eq!(
        wow_world_quest::apply_quest_item_added_objective_progress_with_generator_like_cpp(
            &mut session,
            &generator,
            item_id,
            0,
            1,
        )
        .await,
        vec![quest_id]
    );

    let owner = session.core.quest_objective_access_like_cpp();
    let rows = owner.quest_log_values_snapshot_like_cpp().unwrap();
    assert_eq!(rows[0].objective_progress()[0], 1);
    assert_eq!(rows[4].quest_id(), quest_id as i32);
    assert_eq!(rows[4].objective_progress()[0], 0);
    assert_eq!(
        owner
            .player_quest_gameplay_snapshot_like_cpp()
            .unwrap()
            .status_like_cpp(quest_id)
            .unwrap()
            .objective_counts,
        vec![1]
    );
}

#[test]
fn native_quest_log_access_does_not_resolve_a_map_player_without_owned_handle() {
    let (mut session, _send_rx) = make_session();
    // Preserve the map resident while deliberately omitting this session's
    // generation-checked ownership handle.
    session.core.player_handle_like_cpp = None;
    let player_guid = session.player_guid().expect("test player guid");
    let mut manager = wow_map::MapManager::new(60_000, 1);
    insert_player_with_reputation(&mut manager, player_guid, 1, 0);
    session.set_canonical_map_manager(Arc::new(std::sync::Mutex::new(manager)));
    let owner = session.core.quest_objective_access_like_cpp();

    assert!(owner.owner_handle_absent_like_cpp());
    assert!(owner.quest_log_values_snapshot_like_cpp().is_none());
    assert_eq!(owner.set_quest_slot_like_cpp(0, 59_407), None);
    assert_eq!(session.first_free_quest_slot_like_cpp(), None);
    let manager = session
        .core
        .canonical_map_manager
        .as_ref()
        .unwrap()
        .lock()
        .unwrap();
    assert_eq!(
        manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_typed_player(player_guid)
            .unwrap()
            .quest_slot_quest_id_like_cpp(0),
        None,
        "a GUID-resolved Player cannot receive an ownerless QuestLog write"
    );
}
