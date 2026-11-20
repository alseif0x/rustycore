use super::*;
use wow_constants::quest::{
    QUEST_OBJECTIVE_FLAG_KILL_PLAYERS_SAME_FACTION_LIKE_CPP, QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP,
};

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
    session
        .quest_state
        .fixture_insert_player_quest_status_like_cpp(
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
    let _ = publish_quest_log_values_after_map_tick_like_cpp(&mut session);
    let _ = drain_server_opcodes(&send_rx);
    session.killed_player_credit_like_cpp(victim_guid).await;
    assert_eq!(
        publish_quest_log_values_after_map_tick_like_cpp(&mut session),
        1
    );

    assert_canonical_quest_status_like_cpp(&session, quest_id, None, true);
    assert_eq!(
        session
            .quest_state
            .represented_quest_complete_status_updates_like_cpp(),
        &[RepresentedQuestCompleteStatusUpdateLikeCpp {
            quest_id,
            old_status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            new_status: crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
            send_quest_update_called: true,
            quest_slot_state_complete_represented: true,
            quest_slot_state_live_update_unrepresented: false,
            visible_gameobjects_or_spellclicks_refresh_unrepresented: true,
            spell_area_runtime_unrepresented: true,
            tracking_event_auto_reward_unrepresented: false,
            quest_tracker_complete_time_unrepresented: false,
            script_status_change_unrepresented: true,
        }]
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::QuestUpdateAddPvpCredit,
            ServerOpcodes::QuestGiverQuestComplete,
            ServerOpcodes::UpdateObject,
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
    session.fixtures.identity.player_race = 1;
    session.set_player_guid(Some(player_guid));
    session.set_player_registry(registry);
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session
        .quest_state
        .fixture_insert_player_quest_status_like_cpp(
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

async fn mercenary_same_faction_outcome_like_cpp(
    actor_mercenary: bool,
    victim_race: u8,
    victim_mercenary: bool,
) -> (u8, Vec<i32>, Vec<ServerOpcodes>) {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let victim_guid = ObjectGuid::create_player(1, 84);
    let (victim_tx, _victim_rx) = flume::bounded(1);
    let canonical = shared_canonical_map_manager();
    let registry = Arc::new(PlayerRegistry::default());
    assert!(registry.bind_canonical_map_manager(Arc::clone(&canonical)));
    let mut victim_info = broadcast_info(victim_guid, victim_tx);
    victim_info.identity.race = victim_race;
    victim_info.placement.map_id = 571;
    registry.register_or_replace(victim_guid, victim_info, Default::default());
    let mut victim_player = Player::new(Some(8), false);
    victim_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(victim_guid);
    victim_player
        .unit_mut()
        .world_mut()
        .set_map(571, 0)
        .unwrap();
    victim_player
        .unit_mut()
        .world_mut()
        .relocate(Position::ZERO);
    victim_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    victim_player.set_race_class_gender(victim_race, 1, wow_constants::Gender::Male);
    if victim_mercenary {
        victim_player.set_player_flag_ex(
            wow_world_core::session::state::hub_support::PLAYER_FLAGS_EX_MERCENARY_MODE_LIKE_CPP,
        );
    }
    canonical.lock().unwrap().create_world_map(571, 0);
    canonical
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(victim_player).unwrap())
        .unwrap();

    let quest_id = 12_505;
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
    session.fixtures.identity.player_race = 1;
    session.set_player_guid(Some(player_guid));
    session.set_player_registry(registry);
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session
        .quest_state
        .fixture_insert_player_quest_status_like_cpp(
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
    if actor_mercenary {
        assert!(
            session
                .mutate_canonical_player_like_cpp(|player| {
                    player.set_player_flag_ex(
                        wow_world_core::session::state::hub_support::PLAYER_FLAGS_EX_MERCENARY_MODE_LIKE_CPP,
                    );
                })
                .is_some(),
            "actor mercenary flag must use the canonical Player"
        );
    }
    session.killed_player_credit_like_cpp(victim_guid).await;

    let status = session
        .player_quest_gameplay_snapshot_like_cpp()
        .and_then(|state| state.status_like_cpp(quest_id).cloned())
        .expect("mercenary quest status");
    (
        status.status,
        status.objective_counts,
        drain_server_opcodes(&send_rx),
    )
}

#[tokio::test]
async fn same_faction_objective_uses_effective_team_with_mercenary_on_both_sides_like_cpp() {
    for (actor_mercenary, victim_race, victim_mercenary, expected_complete) in [
        (false, 2, true, true),
        (true, 2, false, true),
        (true, 1, false, false),
    ] {
        let (status, counts, opcodes) =
            mercenary_same_faction_outcome_like_cpp(actor_mercenary, victim_race, victim_mercenary)
                .await;
        if expected_complete {
            assert_eq!(status, crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP);
            assert_eq!(counts, vec![1]);
            assert!(opcodes.contains(&ServerOpcodes::QuestUpdateAddPvpCredit));
        } else {
            assert_eq!(status, crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP);
            assert_eq!(counts, vec![0]);
            assert!(opcodes.is_empty());
        }
        assert!(!opcodes.contains(&ServerOpcodes::QuestUpdateComplete));
    }
}
