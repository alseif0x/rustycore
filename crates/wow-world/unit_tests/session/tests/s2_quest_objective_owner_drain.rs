//! #1363 slice 2 / #1359: focal drain through the App owner of C++
//! `Player::UpdateQuestObjectiveProgress` (`Player.cpp:16980`): events apply in order.

use super::*;
use wow_conditions::{QUEST_STATUS_COMPLETE_LIKE_CPP, QUEST_STATUS_INCOMPLETE_LIKE_CPP};

#[tokio::test]
async fn app_owner_drain_applies_reputation_then_currency_in_recorded_order_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let mut faction = FactionEntry::for_test_like_cpp(7, 5);
    faction.reputation_flags[0] = ReputationFlagsLikeCpp::VISIBLE.bits();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    session.set_faction_store(Arc::new(FactionStore::from_entries([faction])));
    let mut quests = Vec::new();
    for (id, obj_type, object_id) in [
        (13_631, QUEST_OBJECTIVE_OBTAIN_CURRENCY_LIKE_CPP, 395),
        (13_632, QUEST_OBJECTIVE_INCREASE_REPUTATION_LIKE_CPP, 7),
    ] {
        let mut quest = test_quest_template(id);
        quest.objectives.push(wow_data::quest::QuestObjective {
            id: id * 10,
            quest_id: id,
            obj_type,
            order: 0,
            storage_index: 0,
            object_id,
            amount: 100,
            flags: 0,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        });
        quests.push(quest);
        let status = crate::handlers::quest::PlayerQuestStatus {
            quest_id: id,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![0],
            slot: (id % 2) as u8,
        };
        let quest_state = &mut session.quest_state;
        quest_state.fixture_insert_player_quest_status_like_cpp(id, status);
    }
    let store = wow_data::quest::QuestStore::from_quests_like_cpp(quests);
    session.set_quest_store(Arc::new(store));
    adopt_player_quest_fixture_into_canonical_owner_like_cpp(&mut session);
    let _ = drain_server_opcodes(&send_rx);

    session.reputation_changed_like_cpp(7, 30).await;
    session.currency_changed_like_cpp(395, 40).await;

    let state = session.player_quest_gameplay_snapshot_like_cpp().unwrap();
    let progress = |id| {
        state
            .statuses_like_cpp()
            .get(&id)
            .map(|s| (s.status, s.objective_counts[0]))
    };
    let incomplete = crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP;
    assert_eq!(progress(13_632), Some((incomplete, 30)));
    assert_eq!(progress(13_631), Some((incomplete, 40)));
    let packets = drain_server_packet_bytes(&send_rx);
    let credited = packets.iter().map(|bytes| {
        let mut packet = WorldPacket::from_bytes(bytes);
        let opcode = packet.read_uint16().unwrap();
        assert_eq!(opcode, ServerOpcodes::QuestUpdateAddCredit as u16);
        let _victim = packet.read_packed_guid().unwrap();
        packet.read_uint32().unwrap()
    });
    assert_eq!(credited.collect::<Vec<_>>(), vec![13_632, 13_631]);
    let updates = session
        .quest_state
        .represented_quest_complete_status_updates_like_cpp();
    assert!(updates.is_empty());
}

const TRANSITION_QUEST_ID: u32 = 13_633;
const TRANSITION_CLICK_NPC_ENTRY: u32 = 713;
const TRANSITION_CLICK_SPELL_ID: u32 = 913;

async fn storing_objective_transition_outcome_like_cpp(
    storing_flag: bool,
    initial_status: u8,
    initial_count: i32,
    add_count: i32,
    objective_flags: u32,
) -> (u8, i32, Vec<ServerOpcodes>) {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 45);
    let creature_guid = test_creature_guid(133);
    let objective_type = if storing_flag {
        wow_constants::quest::QUEST_OBJECTIVE_CRITERIA_TREE_LIKE_CPP
    } else {
        wow_constants::quest::QUEST_OBJECTIVE_MONSTER_LIKE_CPP
    };
    let object_id = TRANSITION_CLICK_NPC_ENTRY as i32;
    let mut quest = test_quest_template(TRANSITION_QUEST_ID);
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: TRANSITION_QUEST_ID * 10,
        quest_id: TRANSITION_QUEST_ID,
        obj_type: objective_type,
        order: 0,
        storage_index: 0,
        object_id,
        amount: 1,
        flags: objective_flags,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });

    session.set_player_guid(Some(player_guid));
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
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
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical objective-transition viewer map");
    assert!(
        session
            .mutate_player_quest_gameplay_like_cpp(|state| {
                state.insert_status_like_cpp(
                    TRANSITION_QUEST_ID,
                    crate::handlers::quest::PlayerQuestStatus {
                        quest_id: TRANSITION_QUEST_ID,
                        status: initial_status,
                        explored: false,
                        accept_time_secs: 0,
                        end_time_secs: 0,
                        objective_counts: vec![initial_count],
                        slot: 0,
                    },
                );
            })
            .is_some(),
        "objective-transition fixture must use the canonical Player owner"
    );
    session.set_condition_store(Arc::new(
        ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
            source_type: ConditionSourceType::SpellClickEvent,
            source_group: TRANSITION_CLICK_NPC_ENTRY,
            source_entry: TRANSITION_CLICK_SPELL_ID as i32,
            source_id: 0,
            condition_type: ConditionType::Aura,
            condition_value1: 999_999,
            ..Condition::default()
        }]),
    ));
    session.set_npc_spell_click_store(Arc::new(NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
        [wow_data::NpcSpellClickRowLikeCpp {
            npc_entry: TRANSITION_CLICK_NPC_ENTRY,
            spell_id: TRANSITION_CLICK_SPELL_ID,
            cast_flags: 0,
            user_type: wow_data::SPELL_CLICK_USER_ANY_LIKE_CPP,
        }],
        |entry| entry == TRANSITION_CLICK_NPC_ENTRY,
        |spell| spell == TRANSITION_CLICK_SPELL_ID,
    )));
    add_canonical_test_creature(
        &canonical,
        creature_guid,
        TRANSITION_CLICK_NPC_ENTRY,
        Position::new(12.0, 0.0, 0.0, 0.0),
        UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32,
    );
    session
        .core
        .client_visible_guids_like_cpp
        .insert(creature_guid);
    let _ = drain_server_opcodes(&send_rx);

    let generator = wow_core::ObjectGuidGenerator::new(wow_core::guid::HighGuid::Item, 1);
    let operation = if storing_flag {
        crate::session::quest::QuestObjectiveOperationLikeCpp::StoringFlag {
            objective_type,
            object_id,
            add_count,
        }
    } else {
        crate::session::quest::QuestObjectiveOperationLikeCpp::StoringValue {
            objective_type,
            object_id,
            add_count,
            credit_guid: creature_guid,
        }
    };
    session
        .quest_objective_operation_like_cpp(&generator, operation)
        .await;

    let status = session
        .player_quest_gameplay_snapshot_like_cpp()
        .and_then(|state| state.status_like_cpp(TRANSITION_QUEST_ID).cloned())
        .expect("objective-transition quest status");
    (
        status.status,
        status.objective_counts[0],
        drain_server_opcodes(&send_rx),
    )
}

fn assert_storing_objective_transition_like_cpp(
    outcome: (u8, i32, Vec<ServerOpcodes>),
    expected_status: u8,
    expected_count: i32,
    expected_visibility_refreshes: usize,
    expected_credit: ServerOpcodes,
) {
    let (status, count, opcodes) = outcome;
    assert_eq!(status, expected_status);
    assert_eq!(count, expected_count);
    assert_eq!(
        opcodes
            .iter()
            .filter(|opcode| **opcode == ServerOpcodes::UpdateObject)
            .count(),
        expected_visibility_refreshes,
        "unexpected interaction refresh count: {opcodes:?}"
    );
    assert_eq!(
        opcodes.contains(&expected_credit),
        expected_count == 1 && expected_status == QUEST_STATUS_COMPLETE_LIKE_CPP,
        "credit must only be published for the positive transition: {opcodes:?}"
    );
    for unexpected in [
        ServerOpcodes::QuestUpdateComplete,
        ServerOpcodes::QuestGiverQuestComplete,
    ] {
        assert!(
            !opcodes.contains(&unexpected),
            "objective progress must not publish {unexpected:?}: {opcodes:?}"
        );
    }
}

#[tokio::test]
async fn storing_value_positive_transition_completes_and_refreshes_twice_like_cpp() {
    let outcome = storing_objective_transition_outcome_like_cpp(
        false,
        QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        0,
        1,
        0,
    )
    .await;
    assert_storing_objective_transition_like_cpp(
        outcome,
        QUEST_STATUS_COMPLETE_LIKE_CPP,
        1,
        2,
        ServerOpcodes::QuestUpdateAddCredit,
    );
}

#[tokio::test]
async fn storing_value_negative_transition_marks_quest_incomplete_and_refreshes_like_cpp() {
    let outcome = storing_objective_transition_outcome_like_cpp(
        false,
        QUEST_STATUS_COMPLETE_LIKE_CPP,
        1,
        -1,
        0,
    )
    .await;
    assert_storing_objective_transition_like_cpp(
        outcome,
        QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        0,
        1,
        ServerOpcodes::QuestUpdateAddCredit,
    );
}

#[tokio::test]
async fn storing_value_optional_negative_transition_keeps_quest_complete_and_refreshes_like_cpp() {
    let outcome = storing_objective_transition_outcome_like_cpp(
        false,
        QUEST_STATUS_COMPLETE_LIKE_CPP,
        1,
        -1,
        wow_constants::quest::QUEST_OBJECTIVE_FLAG_OPTIONAL_LIKE_CPP,
    )
    .await;
    assert_storing_objective_transition_like_cpp(
        outcome,
        QUEST_STATUS_COMPLETE_LIKE_CPP,
        0,
        1,
        ServerOpcodes::QuestUpdateAddCredit,
    );
}

#[tokio::test]
async fn storing_flag_positive_transition_completes_and_refreshes_twice_like_cpp() {
    let outcome = storing_objective_transition_outcome_like_cpp(
        true,
        QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        0,
        1,
        0,
    )
    .await;
    assert_storing_objective_transition_like_cpp(
        outcome,
        QUEST_STATUS_COMPLETE_LIKE_CPP,
        1,
        2,
        ServerOpcodes::QuestUpdateAddCreditSimple,
    );
}

#[tokio::test]
async fn storing_flag_negative_transition_marks_quest_incomplete_and_refreshes_like_cpp() {
    let outcome = storing_objective_transition_outcome_like_cpp(
        true,
        QUEST_STATUS_COMPLETE_LIKE_CPP,
        1,
        -1,
        0,
    )
    .await;
    assert_storing_objective_transition_like_cpp(
        outcome,
        QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        0,
        1,
        ServerOpcodes::QuestUpdateAddCreditSimple,
    );
}

#[tokio::test]
async fn storing_flag_optional_negative_transition_keeps_quest_complete_and_refreshes_like_cpp() {
    let outcome = storing_objective_transition_outcome_like_cpp(
        true,
        QUEST_STATUS_COMPLETE_LIKE_CPP,
        1,
        -1,
        wow_constants::quest::QUEST_OBJECTIVE_FLAG_OPTIONAL_LIKE_CPP,
    )
    .await;
    assert_storing_objective_transition_like_cpp(
        outcome,
        QUEST_STATUS_COMPLETE_LIKE_CPP,
        0,
        1,
        ServerOpcodes::QuestUpdateAddCreditSimple,
    );
}
