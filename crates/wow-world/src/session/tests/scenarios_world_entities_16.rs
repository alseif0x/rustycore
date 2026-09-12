//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn creature_questgiver_single_complete_ender_auto_opens_request_items_can_complete_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let creature_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 21);
    let mut quest = test_quest_template(9_103);
    quest.log_title = "Creature complete ender".into();
    let mut quest_store = wow_data::quest::QuestStore::from_quests_like_cpp([quest]);
    quest_store.ender_quests.insert(777, vec![9_103]);
    session.quests.store = Some(Arc::new(quest_store));
    session.player_quests.insert(
        9_103,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id: 9_103,
            status: crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    assert!(session.use_represented_creature_questgiver_like_cpp(creature_guid, 777));

    let bytes = send_rx.try_recv().unwrap();
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&bytes).server_opcode(),
        Some(ServerOpcodes::QuestGiverRequestItems)
    );
    assert_eq!(
        quest_giver_request_items_summary_like_cpp(&bytes),
        QuestGiverRequestItemsSummaryLikeCpp {
            giver_creature_id: 777,
            quest_id: 9_103,
            status_flags: 0xFF,
            auto_launched: true,
        }
    );
}
#[test]
fn creature_questgiver_ender_relation_precedes_starter_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    session.set_player_level_like_cpp(1);
    let creature_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 22);
    let mut ender = test_quest_template(9_101);
    ender.log_title = "Creature ender".into();
    let mut starter = test_quest_template(9_102);
    starter.quest_type = 2;
    starter.log_title = "Creature starter".into();
    let mut quest_store = wow_data::quest::QuestStore::from_quests_like_cpp([ender, starter]);
    quest_store.ender_quests.insert(777, vec![9_101]);
    quest_store.starter_quests.insert(777, vec![9_102]);
    session.quests.store = Some(Arc::new(quest_store));
    session.player_quests.insert(
        9_101,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id: 9_101,
            status: crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    assert!(session.use_represented_creature_questgiver_like_cpp(creature_guid, 777));

    let bytes = send_rx.try_recv().unwrap();
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&bytes).server_opcode(),
        Some(ServerOpcodes::QuestGiverQuestListMessage)
    );
    assert!(packet_contains_quest_ids_in_order(&bytes, &[9_101, 9_102]));
}
#[test]
fn quest_giver_query_creature_inactive_ender_falls_through_to_same_starter_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    session.set_player_level_like_cpp(1);
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 99);
    let source_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 779, 24);
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, position);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, player_guid, position, 571, 0);
    adopt_live_canonical_test_player_for_interaction_like_cpp(&mut session);
    add_canonical_test_creature(
        &canonical,
        source_guid,
        779,
        position,
        wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
    );
    let mut quest = test_quest_template(9_104);
    quest.quest_type = 2;
    quest.log_title = "Creature same source starter".into();
    let mut quest_store = wow_data::quest::QuestStore::from_quests_like_cpp([quest]);
    quest_store.ender_quests.insert(779, vec![9_104]);
    quest_store.starter_quests.insert(779, vec![9_104]);
    session.quests.store = Some(Arc::new(quest_store));

    assert!(session.send_represented_quest_giver_query_quest_like_cpp(source_guid, 9_104));

    let bytes = send_rx.try_recv().unwrap();
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&bytes).server_opcode(),
        Some(ServerOpcodes::QuestGiverQuestDetails)
    );
    assert!(packet_contains_quest_ids_in_order(&bytes, &[9_104]));
}
#[test]
fn quest_giver_query_creature_inactive_ender_without_starter_rejects_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    session.set_player_level_like_cpp(1);
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 99);
    let source_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 780, 25);
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, position);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, player_guid, position, 571, 0);
    add_canonical_test_creature(
        &canonical,
        source_guid,
        780,
        position,
        wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
    );
    let mut quest_store =
        wow_data::quest::QuestStore::from_quests_like_cpp([test_quest_template(9_105)]);
    quest_store.ender_quests.insert(780, vec![9_105]);
    session.quests.store = Some(Arc::new(quest_store));

    assert!(!session.send_represented_quest_giver_query_quest_like_cpp(source_guid, 9_105));
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn creature_questgiver_without_creature_relations_sends_nothing_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    session.set_player_level_like_cpp(1);
    let creature_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 23);
    let mut quest_store =
        wow_data::quest::QuestStore::from_quests_like_cpp([test_quest_template(9_101)]);
    assert!(quest_store.insert_gameobject_starter_relation_like_cpp(777, 9_101));
    session.quests.store = Some(Arc::new(quest_store));

    assert!(!session.use_represented_creature_questgiver_like_cpp(creature_guid, 777));
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn quest_giver_accept_creature_starter_relation_allows_source_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 99);
    let source_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 782, 29);
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, position);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, player_guid, position, 571, 0);
    adopt_live_canonical_test_player_for_interaction_like_cpp(&mut session);
    add_canonical_test_creature(
        &canonical,
        source_guid,
        782,
        position,
        wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
    );
    let mut quest_store =
        wow_data::quest::QuestStore::from_quests_like_cpp([test_quest_template(9_208)]);
    quest_store.starter_quests.insert(782, vec![9_208]);

    assert!(
        session.represented_quest_giver_accept_source_allows_quest_like_cpp(
            source_guid,
            9_208,
            &quest_store,
        )
    );
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn quest_giver_accept_creature_ender_only_relation_rejects_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 99);
    let source_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 783, 30);
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, position);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, player_guid, position, 571, 0);
    add_canonical_test_creature(
        &canonical,
        source_guid,
        783,
        position,
        wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
    );
    let mut quest_store =
        wow_data::quest::QuestStore::from_quests_like_cpp([test_quest_template(9_209)]);
    quest_store.ender_quests.insert(783, vec![9_209]);

    assert!(
        !session.represented_quest_giver_accept_source_allows_quest_like_cpp(
            source_guid,
            9_209,
            &quest_store,
        )
    );
    assert!(session.player_quests.is_empty());
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn quest_giver_accept_gameobject_starter_relation_allows_source_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let source_guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 784, 31);
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    session.set_player_map_position_like_cpp(571, position);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_gameobject(&canonical, source_guid, 784, position);
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        source_guid,
        784,
        position,
        wow_entities::GAMEOBJECT_TYPE_QUESTGIVER as u8,
    );
    let mut quest_store =
        wow_data::quest::QuestStore::from_quests_like_cpp([test_quest_template(9_210)]);
    assert!(quest_store.insert_gameobject_starter_relation_like_cpp(784, 9_210));

    assert!(
        session.represented_quest_giver_accept_source_allows_quest_like_cpp(
            source_guid,
            9_210,
            &quest_store,
        )
    );
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn quest_giver_accept_gameobject_starter_relation_without_interaction_rejects_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let source_guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 784, 131);
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    session.set_player_map_position_like_cpp(571, position);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_gameobject(&canonical, source_guid, 784, position);
    let mut quest_store =
        wow_data::quest::QuestStore::from_quests_like_cpp([test_quest_template(9_210)]);
    assert!(quest_store.insert_gameobject_starter_relation_like_cpp(784, 9_210));

    assert!(
        !session.represented_quest_giver_accept_source_allows_quest_like_cpp(
            source_guid,
            9_210,
            &quest_store,
        ),
        "C++ CanInteractWithQuestGiver(TYPEID_GAMEOBJECT) requires an interactable questgiver GO, not just a canonical object"
    );
    assert!(session.player_quests.is_empty());
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn quest_giver_accept_gameobject_ender_only_or_unrelated_rejects_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let source_guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 785, 32);
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    session.set_player_map_position_like_cpp(571, position);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_gameobject(&canonical, source_guid, 785, position);
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        source_guid,
        785,
        position,
        wow_entities::GAMEOBJECT_TYPE_QUESTGIVER as u8,
    );
    let mut quest_store = wow_data::quest::QuestStore::from_quests_like_cpp([
        test_quest_template(9_211),
        test_quest_template(9_212),
    ]);
    assert!(quest_store.insert_gameobject_ender_relation_like_cpp(785, 9_211));

    assert!(
        !session.represented_quest_giver_accept_source_allows_quest_like_cpp(
            source_guid,
            9_211,
            &quest_store,
        )
    );
    assert!(
        !session.represented_quest_giver_accept_source_allows_quest_like_cpp(
            source_guid,
            9_212,
            &quest_store,
        )
    );
    assert!(session.player_quests.is_empty());
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn quest_giver_reward_creature_ender_relation_allows_source_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 99);
    let source_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 786, 34);
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, position);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, player_guid, position, 571, 0);
    adopt_live_canonical_test_player_for_interaction_like_cpp(&mut session);
    add_canonical_test_creature(
        &canonical,
        source_guid,
        786,
        position,
        wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
    );
    let mut quest_store =
        wow_data::quest::QuestStore::from_quests_like_cpp([test_quest_template(9_214)]);
    quest_store.ender_quests.insert(786, vec![9_214]);

    assert!(
        session.represented_quest_giver_involved_source_allows_quest_like_cpp(
            source_guid,
            9_214,
            &quest_store,
        )
    );
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn quest_giver_reward_creature_starter_only_relation_rejects_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 99);
    let source_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 787, 35);
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, position);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, player_guid, position, 571, 0);
    add_canonical_test_creature(
        &canonical,
        source_guid,
        787,
        position,
        wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
    );
    let mut quest_store =
        wow_data::quest::QuestStore::from_quests_like_cpp([test_quest_template(9_215)]);
    quest_store.starter_quests.insert(787, vec![9_215]);

    assert!(
        !session.represented_quest_giver_involved_source_allows_quest_like_cpp(
            source_guid,
            9_215,
            &quest_store,
        )
    );
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn quest_giver_reward_gameobject_ender_relation_allows_source_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let source_guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 788, 36);
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    session.set_player_map_position_like_cpp(571, position);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_gameobject(&canonical, source_guid, 788, position);
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        source_guid,
        788,
        position,
        wow_entities::GAMEOBJECT_TYPE_QUESTGIVER as u8,
    );
    let mut quest_store =
        wow_data::quest::QuestStore::from_quests_like_cpp([test_quest_template(9_216)]);
    assert!(quest_store.insert_gameobject_ender_relation_like_cpp(788, 9_216));

    assert!(
        session.represented_quest_giver_involved_source_allows_quest_like_cpp(
            source_guid,
            9_216,
            &quest_store,
        )
    );
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn quest_giver_reward_gameobject_ender_relation_out_of_range_rejects_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let source_guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 790, 38);
    let player_position = Position::new(1.0, 2.0, 3.0, 0.0);
    let go_position = Position::new(100.0, 2.0, 3.0, 0.0);
    session.set_player_map_position_like_cpp(571, player_position);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_gameobject(&canonical, source_guid, 790, go_position);
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        source_guid,
        790,
        go_position,
        wow_entities::GAMEOBJECT_TYPE_QUESTGIVER as u8,
    );
    let mut quest_store =
        wow_data::quest::QuestStore::from_quests_like_cpp([test_quest_template(9_221)]);
    assert!(quest_store.insert_gameobject_ender_relation_like_cpp(790, 9_221));

    assert!(
        !session.represented_quest_giver_involved_source_allows_quest_like_cpp(
            source_guid,
            9_221,
            &quest_store,
        )
    );
    assert!(send_rx.try_recv().is_err());
    assert!(session.player_quests.is_empty());
}
#[test]
fn quest_giver_reward_gameobject_ender_relation_wrong_type_rejects_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let source_guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 791, 39);
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    session.set_player_map_position_like_cpp(571, position);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_gameobject(&canonical, source_guid, 791, position);
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        source_guid,
        791,
        position,
        wow_entities::GAMEOBJECT_TYPE_CHEST as u8,
    );
    let mut quest_store =
        wow_data::quest::QuestStore::from_quests_like_cpp([test_quest_template(9_222)]);
    assert!(quest_store.insert_gameobject_ender_relation_like_cpp(791, 9_222));

    assert!(
        !session.represented_quest_giver_involved_source_allows_quest_like_cpp(
            source_guid,
            9_222,
            &quest_store,
        )
    );
    assert!(send_rx.try_recv().is_err());
    assert!(session.player_quests.is_empty());
}
#[tokio::test]
async fn quest_giver_choose_reward_creature_ender_source_allows_reward_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 99);
    let source_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 792, 40);
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    session.set_player_guid(Some(player_guid));
    session.set_loot_money_persistence_test_result_like_cpp(true);
    session.set_player_map_position_like_cpp(571, position);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, player_guid, position, 571, 0);
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_max_health(100);
            player.unit_mut().set_health(100);
            player.unit_mut().set_faction(1);
        })
        .expect("live canonical Player fixture");
    add_canonical_test_creature(
        &canonical,
        source_guid,
        792,
        position,
        wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
    );
    let mut quest = test_quest_template(9_223);
    quest.reward_money_difficulty = 37;
    let mut quest_store = wow_data::quest::QuestStore::from_quests_like_cpp([quest]);
    quest_store.ender_quests.insert(792, vec![9_223]);
    session.quests.store = Some(Arc::new(quest_store));
    session.set_player_gold_like_cpp(5);
    session
        .mutate_player_quest_gameplay_like_cpp(|quests| {
            quests.insert_status_like_cpp(
                9_223,
                crate::handlers::quest::PlayerQuestStatus {
                    quest_id: 9_223,
                    status: crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
                    explored: false,
                    accept_time_secs: 0,
                    end_time_secs: 0,
                    objective_counts: Vec::new(),
                    slot: 0,
                },
            );
        })
        .expect("canonical Player quest owner");

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            source_guid,
            9_223,
            0,
            0,
        ))
        .await;

    let quests = session
        .player_quest_gameplay_snapshot_like_cpp()
        .expect("canonical Player quest owner after reward");
    assert!(!quests.statuses_like_cpp().contains_key(&9_223));
    assert!(quests.rewarded_quest_ids_like_cpp().contains(&9_223));
    assert_eq!(session.player_gold_like_cpp(), 42);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::UpdateObject,
            ServerOpcodes::QuestGiverQuestComplete,
            ServerOpcodes::QuestUpdateComplete,
        ]
    );
}
#[tokio::test]
async fn quest_giver_choose_reward_gameobject_no_relation_rejects_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let source_guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 794, 42);
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    session.set_player_map_position_like_cpp(571, position);
    session.set_player_gold_like_cpp(5);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_gameobject(&canonical, source_guid, 794, position);
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        source_guid,
        794,
        position,
        wow_entities::GAMEOBJECT_TYPE_QUESTGIVER as u8,
    );
    let mut quest = test_quest_template(9_225);
    quest.reward_money_difficulty = 37;
    session.quests.store = Some(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.player_quests.insert(
        9_225,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id: 9_225,
            status: crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            source_guid,
            9_225,
            0,
            0,
        ))
        .await;

    assert_eq!(
        session.player_quests.get(&9_225).map(|quest| quest.status),
        Some(crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP)
    );
    assert!(!session.rewarded_quests.contains(&9_225));
    assert_eq!(session.player_gold_like_cpp(), 5);
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn quest_giver_query_creature_starter_relation_allows_matching_details_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 99);
    let source_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 24);
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, position);
    session.set_player_level_like_cpp(1);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, player_guid, position, 571, 0);
    adopt_live_canonical_test_player_for_interaction_like_cpp(&mut session);
    add_canonical_test_creature(
        &canonical,
        source_guid,
        777,
        position,
        wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
    );
    let mut quest = test_quest_template(9_201);
    quest.quest_type = 2;
    let other_quest = test_quest_template(9_202);
    let mut quest_store = wow_data::quest::QuestStore::from_quests_like_cpp([quest, other_quest]);
    quest_store.starter_quests.insert(777, vec![9_201]);
    session.quests.store = Some(Arc::new(quest_store));

    assert!(session.send_represented_quest_giver_query_quest_like_cpp(source_guid, 9_201));
    let bytes = send_rx.try_recv().unwrap();
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&bytes).server_opcode(),
        Some(ServerOpcodes::QuestGiverQuestDetails)
    );
    assert!(packet_contains_quest_ids_in_order(&bytes, &[9_201]));

    assert!(!session.send_represented_quest_giver_query_quest_like_cpp(source_guid, 9_202));
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn quest_giver_query_creature_ender_relation_allows_request_items_like_cpp() {
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
        wow_data::quest::QuestStore::from_quests_like_cpp([test_quest_template(9_203)]);
    quest_store.ender_quests.insert(778, vec![9_203]);
    session.quests.store = Some(Arc::new(quest_store));
    assert!(
        session
            .mutate_player_quest_gameplay_like_cpp(|quests| {
                quests.insert_status_like_cpp(
                    9_203,
                    crate::handlers::quest::PlayerQuestStatus {
                        quest_id: 9_203,
                        status: crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
                        explored: false,
                        accept_time_secs: 0,
                        end_time_secs: 0,
                        objective_counts: Vec::new(),
                        slot: 0,
                    },
                );
            })
            .is_some()
    );

    assert!(session.send_represented_quest_giver_query_quest_like_cpp(source_guid, 9_203));
    let bytes = send_rx.try_recv().unwrap();
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&bytes).server_opcode(),
        Some(ServerOpcodes::QuestGiverRequestItems)
    );
    assert!(packet_contains_quest_ids_in_order(&bytes, &[9_203]));
    assert_eq!(
        quest_giver_request_items_summary_like_cpp(&bytes),
        QuestGiverRequestItemsSummaryLikeCpp {
            giver_creature_id: 778,
            quest_id: 9_203,
            status_flags: 0xFF,
            auto_launched: false,
        }
    );
}
#[test]
fn quest_giver_query_gameobject_starter_relation_allows_matching_details_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let source_guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 779, 26);
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    session.set_player_map_position_like_cpp(571, position);
    session.set_player_level_like_cpp(1);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_gameobject(&canonical, source_guid, 779, position);
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        source_guid,
        779,
        position,
        wow_entities::GAMEOBJECT_TYPE_QUESTGIVER as u8,
    );
    let mut quest = test_quest_template(9_204);
    quest.quest_type = 2;
    let other_quest = test_quest_template(9_205);
    let mut quest_store = wow_data::quest::QuestStore::from_quests_like_cpp([quest, other_quest]);
    assert!(quest_store.insert_gameobject_starter_relation_like_cpp(779, 9_204));
    session.quests.store = Some(Arc::new(quest_store));

    assert!(session.send_represented_quest_giver_query_quest_like_cpp(source_guid, 9_204));
    let bytes = send_rx.try_recv().unwrap();
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&bytes).server_opcode(),
        Some(ServerOpcodes::QuestGiverQuestDetails)
    );
    assert!(packet_contains_quest_ids_in_order(&bytes, &[9_204]));

    assert!(!session.send_represented_quest_giver_query_quest_like_cpp(source_guid, 9_205));
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn quest_giver_query_gameobject_without_interaction_rejects_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let source_guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 779, 126);
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    session.set_player_map_position_like_cpp(571, position);
    session.set_player_level_like_cpp(1);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_gameobject(&canonical, source_guid, 779, position);
    let mut quest = test_quest_template(9_204);
    quest.quest_type = 2;
    let mut quest_store = wow_data::quest::QuestStore::from_quests_like_cpp([quest]);
    assert!(quest_store.insert_gameobject_starter_relation_like_cpp(779, 9_204));
    session.quests.store = Some(Arc::new(quest_store));

    assert!(
        !session.send_represented_quest_giver_query_quest_like_cpp(source_guid, 9_204),
        "C++ CanInteractWithQuestGiver(TYPEID_GAMEOBJECT) gates query details before trusting starter relations"
    );
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn quest_giver_query_gameobject_ender_relation_allows_request_items_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let source_guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 780, 27);
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    session.set_player_map_position_like_cpp(571, position);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_gameobject(&canonical, source_guid, 780, position);
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        source_guid,
        780,
        position,
        wow_entities::GAMEOBJECT_TYPE_QUESTGIVER as u8,
    );
    let mut quest_store =
        wow_data::quest::QuestStore::from_quests_like_cpp([test_quest_template(9_206)]);
    assert!(quest_store.insert_gameobject_ender_relation_like_cpp(780, 9_206));
    session.quests.store = Some(Arc::new(quest_store));
    session.player_quests.insert(
        9_206,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id: 9_206,
            status: crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    assert!(session.send_represented_quest_giver_query_quest_like_cpp(source_guid, 9_206));
    let bytes = send_rx.try_recv().unwrap();
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&bytes).server_opcode(),
        Some(ServerOpcodes::QuestGiverRequestItems)
    );
    assert!(packet_contains_quest_ids_in_order(&bytes, &[9_206]));
    assert_eq!(
        quest_giver_request_items_summary_like_cpp(&bytes),
        QuestGiverRequestItemsSummaryLikeCpp {
            giver_creature_id: 0,
            quest_id: 9_206,
            status_flags: 0xFF,
            auto_launched: false,
        }
    );
}
