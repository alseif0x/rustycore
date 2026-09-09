//! Session scenarios exercising the represented quest responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn quest_giver_reward_daily_flag_is_not_auto_complete_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    session.set_player_guid(Some(player_guid));
    let mut quest = test_quest_template(9_219);
    quest.flags = 0x0000_1000;
    session.quest_store = Some(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.player_quests.insert(
        9_219,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id: 9_219,
            status: crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_request_reward(quest_giver_request_reward_packet_like_cpp(
            player_guid,
            9_219,
        ))
        .await;

    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        session.player_quests.get(&9_219).map(|quest| quest.status),
        Some(crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP)
    );
}
#[tokio::test]
async fn quest_giver_complete_daily_flag_requires_involved_source_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    session.set_player_guid(Some(player_guid));
    let mut quest = test_quest_template(9_220);
    quest.flags = 0x0000_1000;
    session.quest_store = Some(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.player_quests.insert(
        9_220,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id: 9_220,
            status: crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_complete_quest(quest_giver_complete_packet_like_cpp(
            player_guid,
            9_220,
            true,
        ))
        .await;

    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        session.player_quests.get(&9_220).map(|quest| quest.status),
        Some(crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP)
    );
}
#[tokio::test]
async fn quest_giver_complete_auto_complete_requires_player_guid_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let wrong_guid = ObjectGuid::create_player(1, 100);
    session.set_player_guid(Some(player_guid));
    let mut quest = test_quest_template(9_218);
    quest.flags = crate::handlers::quest::QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    session.quest_store = Some(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.player_quests.insert(
        9_218,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id: 9_218,
            status: crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_complete_quest(quest_giver_complete_packet_like_cpp(
            wrong_guid, 9_218, true,
        ))
        .await;
    assert!(send_rx.try_recv().is_err());

    session
        .handle_quest_giver_complete_quest(quest_giver_complete_packet_like_cpp(
            player_guid,
            9_218,
            false,
        ))
        .await;
    assert!(send_rx.try_recv().is_err());

    session
        .handle_quest_giver_complete_quest(quest_giver_complete_packet_like_cpp(
            player_guid,
            9_218,
            true,
        ))
        .await;
    let bytes = send_rx.try_recv().unwrap();
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&bytes).server_opcode(),
        Some(ServerOpcodes::QuestGiverOfferRewardMessage)
    );
}
#[test]
fn quest_giver_query_rewarded_nonrepeatable_complete_ender_is_not_completable_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 99);
    let source_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 778, 25);
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, position);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, player_guid, position, 571, 0);
    adopt_live_canonical_test_player_for_interaction_like_cpp(&mut session);
    add_canonical_test_creature(
        &canonical,
        source_guid,
        778,
        position,
        wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
    );
    let mut quest_store =
        wow_data::quest::QuestStore::from_quests_like_cpp([test_quest_template(9_204)]);
    quest_store.ender_quests.insert(778, vec![9_204]);
    session.quest_store = Some(Arc::new(quest_store));
    assert!(
        session
            .mutate_player_quest_gameplay_like_cpp(|quests| {
                quests.statuses.insert(
                    9_204,
                    crate::handlers::quest::PlayerQuestStatus {
                        quest_id: 9_204,
                        status: crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
                        explored: false,
                        accept_time_secs: 0,
                        end_time_secs: 0,
                        objective_counts: Vec::new(),
                        slot: 0,
                    },
                );
                quests.rewarded_quest_ids.insert(9_204);
            })
            .is_some()
    );

    assert!(session.send_represented_quest_giver_query_quest_like_cpp(source_guid, 9_204));
    let bytes = send_rx.try_recv().unwrap();
    assert_eq!(
        quest_giver_request_items_summary_like_cpp(&bytes),
        QuestGiverRequestItemsSummaryLikeCpp {
            giver_creature_id: 778,
            quest_id: 9_204,
            status_flags: 0xFD,
            auto_launched: false,
        }
    );
}
#[test]
fn quest_giver_query_unsupported_or_missing_source_sends_no_packet_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(canonical);
    session.quest_store = Some(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [test_quest_template(9_207)],
    )));
    let missing_guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 781, 28);
    let player_guid = ObjectGuid::create_player(1, 123);

    assert!(!session.send_represented_quest_giver_query_quest_like_cpp(missing_guid, 9_207));
    assert!(!session.send_represented_quest_giver_query_quest_like_cpp(player_guid, 9_207));
    assert!(send_rx.try_recv().is_err());
}
// C++ anchor: /home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObject.cpp:2236-2251
#[test]
fn chest_quest_id_incomplete_activates_to_quest_like_cpp() {
    let quest_id: u32 = 1_234;
    let entry: u32 = 55_001;

    // Positive: player has quest in INCOMPLETE — chest with matching chest_quest_id activates.
    let (mut session, _, _send_rx) = make_session();
    session.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![],
            slot: 0,
        },
    );
    let state = RepresentedGameObjectUseState {
        go_type: Some(wow_entities::GAMEOBJECT_TYPE_CHEST as u8),
        loot_state: Some(wow_entities::LootState::Ready),
        chest_loot_source: Some(wow_entities::GameObjectLootSource {
            chest_quest_id: quest_id,
            ..Default::default()
        }),
        ..Default::default()
    };
    assert!(
        session.represented_gameobject_activate_to_quest_like_cpp(entry, &state),
        "chest with INCOMPLETE quest_id must activate"
    );

    // Negative A: chest_quest_id == 0 (no quest link) — no activation.
    let (session_a, _, _) = make_session();
    let state_a = RepresentedGameObjectUseState {
        go_type: Some(wow_entities::GAMEOBJECT_TYPE_CHEST as u8),
        loot_state: Some(wow_entities::LootState::Ready),
        chest_loot_source: Some(wow_entities::GameObjectLootSource {
            chest_quest_id: 0,
            ..Default::default()
        }),
        ..Default::default()
    };
    assert!(
        !session_a.represented_gameobject_activate_to_quest_like_cpp(entry, &state_a),
        "chest_quest_id == 0 must not activate"
    );

    // Negative B: player has the quest but in COMPLETE status — no activation.
    let (mut session_b, _, _) = make_session();
    session_b.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![],
            slot: 0,
        },
    );
    let state_b = RepresentedGameObjectUseState {
        go_type: Some(wow_entities::GAMEOBJECT_TYPE_CHEST as u8),
        loot_state: Some(wow_entities::LootState::Ready),
        chest_loot_source: Some(wow_entities::GameObjectLootSource {
            chest_quest_id: quest_id,
            ..Default::default()
        }),
        ..Default::default()
    };
    assert!(
        !session_b.represented_gameobject_activate_to_quest_like_cpp(entry, &state_b),
        "chest with quest in COMPLETE status must not activate"
    );
}
