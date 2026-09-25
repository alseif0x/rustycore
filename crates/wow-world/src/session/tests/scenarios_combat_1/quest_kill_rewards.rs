use super::*;

#[tokio::test]
async fn player_kill_tracking_event_objective_auto_rewards_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let victim_guid = ObjectGuid::create_player(1, 84);
    let quest_id = 12_503;
    let mut quest = test_quest_template(quest_id);
    quest.flags |= 0x0000_0400; // C++ QUEST_FLAGS_TRACKING_EVENT.
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP,
        order: 0,
        storage_index: 0,
        object_id: 0,
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

    adopt_player_quest_fixture_into_canonical_owner_like_cpp(&mut session);
    session.killed_player_credit_like_cpp(victim_guid).await;

    assert_canonical_quest_status_like_cpp(&session, quest_id, None, true);
    assert_eq!(
        session.represented_quest_complete_status_updates_like_cpp(),
        &[RepresentedQuestCompleteStatusUpdateLikeCpp {
            quest_id,
            old_status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            new_status: crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
            send_quest_update_called: true,
            quest_slot_state_complete_represented: true,
            quest_slot_state_live_update_unrepresented: true,
            visible_gameobjects_or_spellclicks_refresh_unrepresented: true,
            spell_area_runtime_unrepresented: true,
            tracking_event_auto_reward_unrepresented: false,
            quest_tracker_complete_time_unrepresented: true,
            script_status_change_unrepresented: true,
        }]
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::QuestUpdateAddPvpCredit,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::QuestGiverQuestComplete,
            ServerOpcodes::QuestUpdateComplete,
        ]
    );
}

#[tokio::test]
async fn player_kill_same_faction_objective_skips_opposite_team_victim_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let victim_guid = ObjectGuid::create_player(1, 84);
    let (victim_tx, _victim_rx) = flume::bounded(1);
    let registry = Arc::new(PlayerRegistry::default());
    let mut victim_info = broadcast_info(victim_guid, victim_tx);
    victim_info.identity.race = 2; // Orc/Horde; player test race defaults to Human/Alliance.
    registry.register_or_replace(victim_guid, victim_info, Default::default());

    let quest_id = 12_504;
    let mut quest = test_quest_template(quest_id);
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP,
        order: 0,
        storage_index: 0,
        object_id: 0,
        amount: 1,
        flags: QUEST_OBJECTIVE_FLAG_KILL_PLAYERS_SAME_FACTION_LIKE_CPP,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.player_race = 1;
    session.set_player_guid(Some(player_guid));
    session.set_player_registry(registry);
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

    adopt_player_quest_fixture_into_canonical_owner_like_cpp(&mut session);
    session.killed_player_credit_like_cpp(victim_guid).await;

    let state = session
        .player_quest_gameplay_snapshot_like_cpp()
        .expect("canonical Player quest state");
    let status = state
        .statuses_like_cpp()
        .get(&quest_id)
        .expect("quest remains");
    assert_eq!(status.objective_counts, vec![0]);
    assert!(send_rx.try_recv().is_err());
}
