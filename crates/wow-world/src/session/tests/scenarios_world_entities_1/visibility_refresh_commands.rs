use super::*;

/// Future global creature CREATE/DESTROY work must not use
/// `SendIfVisibleLikeCpp`: a not-yet-visible creature needs the session's
/// visibility pass to build CREATE bytes and update HaveAtClient.
#[tokio::test]
async fn refresh_visible_world_creatures_command_forces_creature_visibility_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 90_009);
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);
    let creature_guid = test_creature_guid(90_010);
    let creature_position = Position::new(12.0, 10.0, 0.0, 0.0);
    let (grid_x, grid_y) =
        crate::map_manager::world_to_grid_coords(creature_position.x, creature_position.y);
    manager.write().unwrap().add_creature(
        571,
        0,
        grid_x,
        grid_y,
        crate::map_manager::WorldCreature::new(
            creature_guid,
            901,
            creature_position,
            100,
            80,
            1,
            2,
            0.0,
            1,
            35,
            0,
            0,
        ),
    );

    session.state = SessionState::LoggedIn;
    session.set_map_manager(manager);
    session.set_canonical_map_manager(canonical);
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "RefreshVisibility".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical viewer map");
    // Prove the command bypasses the 50-yard visibility throttle.
    session.last_visibility_pos = Some(player_position);

    session
        .session_command_tx()
        .try_send(SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(
            RefreshVisibleWorldCreaturesLikeCppCommand {
                map_id: 571,
                instance_id: 0,
            },
        ))
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&creature_guid),
        "forced creature visibility must create the unseen creature"
    );
    let packet = send_rx
        .try_recv()
        .expect("creature CREATE visibility packet");
    let opcode = u16::from_le_bytes([packet[0], packet[1]]);
    assert_eq!(opcode, ServerOpcodes::UpdateObject as u16);
    assert!(send_rx.try_recv().is_err(), "no extra packets");
}

#[tokio::test]
async fn refresh_visible_world_creatures_command_rejects_wrong_map_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.state = SessionState::LoggedIn;
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.last_visibility_pos = Some(Position::ZERO);

    session
        .session_command_tx()
        .try_send(SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(
            RefreshVisibleWorldCreaturesLikeCppCommand {
                map_id: 530,
                instance_id: 0,
            },
        ))
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(
        session.last_visibility_pos,
        Some(Position::ZERO),
        "wrong-map command must not force visibility"
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn refresh_visible_gameobjects_or_spellclicks_command_sends_gameobject_delta_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let go_entry = 8_126;
    let gameobject_guid = test_gameobject_guid(go_entry, 135);
    let quest_id = 12_543;
    let mut quest = test_quest_template(quest_id);
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: 2,
        order: 0,
        storage_index: 0,
        object_id: go_entry as i32,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });

    session.set_player_guid(Some(player_guid));
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.quest_test_fixture_like_cpp.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![0],
            slot: 0,
        },
    );
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_gameobject(
        &canonical,
        gameobject_guid,
        go_entry,
        Position::new(12.0, 0.0, 0.0, 0.0),
    );
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);
    session.represented_gameobject_use_states.insert(
        gameobject_guid,
        RepresentedGameObjectUseState {
            go_type: Some(wow_entities::GAMEOBJECT_TYPE_CHEST as u8),
            loot_state: Some(wow_entities::LootState::Ready),
            ..Default::default()
        },
    );

    session
        .session_command_tx()
        .try_send(SessionCommand::RefreshVisibleGameobjectsOrSpellClicksLikeCpp)
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(
        send_rx.try_recv().expect("remote GO refresh update"),
        expected_gameobject_dynamic_flags_update_like_cpp(
            gameobject_guid,
            571,
            wow_entities::GO_DYNFLAG_LO_ACTIVATE
                | wow_entities::GO_DYNFLAG_LO_SPARKLE
                | wow_entities::GO_DYNFLAG_LO_HIGHLIGHT
        )
    );
    assert!(send_rx.try_recv().is_err());
}
