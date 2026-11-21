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
            completion_effect: None,
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

fn install_raid_membership_like_cpp(session: &mut WorldSession, player_guid: ObjectGuid) {
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(player_guid);
    group.convert_to_raid_like_cpp();
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    assert!(session.set_owned_player_group_like_cpp(Some((group_guid, 0))));
}

async fn storing_objective_transition_outcome_like_cpp(
    storing_flag: bool,
    initial_status: u8,
    initial_count: i32,
    add_count: i32,
    objective_flags: u32,
) -> (u8, i32, Vec<ServerOpcodes>) {
    storing_objective_transition_outcome_with_proxy_rule_like_cpp(
        storing_flag,
        initial_status,
        initial_count,
        add_count,
        objective_flags,
        0,
        false,
        None,
        0,
        false,
    )
    .await
}

async fn storing_objective_transition_outcome_with_proxy_rule_like_cpp(
    storing_flag: bool,
    initial_status: u8,
    initial_count: i32,
    add_count: i32,
    objective_flags: u32,
    quest_flags_ex: u32,
    empty_credit_guid: bool,
    prerequisite: Option<(bool, u32)>,
    quest_flags: u32,
    in_raid_group: bool,
) -> (u8, i32, Vec<ServerOpcodes>) {
    storing_objective_transition_outcome_with_persistence_like_cpp(
        storing_flag,
        initial_status,
        initial_count,
        add_count,
        objective_flags,
        quest_flags_ex,
        empty_credit_guid,
        prerequisite,
        quest_flags,
        in_raid_group,
        false,
        None,
        None,
        false,
        false,
    )
    .await
}

async fn storing_objective_transition_outcome_with_persistence_like_cpp(
    storing_flag: bool,
    initial_status: u8,
    initial_count: i32,
    add_count: i32,
    objective_flags: u32,
    quest_flags_ex: u32,
    empty_credit_guid: bool,
    prerequisite: Option<(bool, u32)>,
    quest_flags: u32,
    in_raid_group: bool,
    quest_tracker_enabled: bool,
    persistence: Option<
        crate::player::quest_persistence_test_fixture::PlayerQuestPersistencePortFixtureLikeCpp,
    >,
    completion_spell_id: Option<u32>,
    completion_update_zone_auras: bool,
    completion_update_phase_shift: bool,
) -> (u8, i32, Vec<ServerOpcodes>) {
    let (mut session, _, send_rx) = make_session();
    session.set_creature_spawn_catalogs_like_cpp(Arc::new(
        session.creature_spawn_catalogs_for_test_like_cpp(),
    ));
    session.set_quest_enable_quest_tracker_like_cpp(quest_tracker_enabled);
    if let Some(persistence) = persistence {
        session.set_player_quest_persistence_port_like_cpp(Arc::new(persistence));
    }
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
    quest.flags = quest_flags;
    quest.flags_ex = quest_flags_ex;
    if let Some((_, prerequisite_flags)) = prerequisite {
        quest.objectives.push(wow_data::quest::QuestObjective {
            id: TRANSITION_QUEST_ID * 10 - 1,
            quest_id: TRANSITION_QUEST_ID,
            obj_type: objective_type,
            order: 0,
            storage_index: 0,
            object_id: object_id + 1,
            amount: 1,
            flags: prerequisite_flags,
            flags2: 0,
            progress_bar_weight: 0.0,
            completion_effect: None,
            description: String::new(),
        });
    }
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: TRANSITION_QUEST_ID * 10,
        quest_id: TRANSITION_QUEST_ID,
        obj_type: objective_type,
        order: u8::from(prerequisite.is_some()),
        storage_index: i8::from(prerequisite.is_some()),
        object_id,
        amount: 1,
        flags: objective_flags,
        flags2: 0,
        progress_bar_weight: 0.0,
        completion_effect: None,
        description: String::new(),
    });
    if completion_spell_id.is_some()
        || completion_update_zone_auras
        || completion_update_phase_shift
    {
        quest.objectives.last_mut().unwrap().completion_effect =
            Some(wow_data::quest::QuestObjectiveAction {
                game_event_id: None,
                spell_id: completion_spell_id,
                conversation_id: None,
                update_phase_shift: completion_update_phase_shift,
                update_zone_auras: completion_update_zone_auras,
            });
    }
    if let Some(spell_id) = completion_spell_id {
        session.set_spell_store(Arc::new(first_login_noop_spell_store_like_cpp([
            spell_id as i32
        ])));
    }
    if completion_update_zone_auras {
        session.set_spell_store(Arc::new(first_login_noop_spell_store_like_cpp([
            50_101, 50_102,
        ])));
        let spell_area = |spell_id, area_id| wow_data::SpellAreaRowLikeCpp {
            spell_id,
            area_id,
            quest_start: 0,
            quest_start_status: 0,
            quest_end_status: 0,
            quest_end: 0,
            aura_spell: 0,
            race_mask: 0,
            gender: wow_data::GENDER_NONE_LIKE_CPP,
            flags: wow_data::SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP,
        };
        session.set_spell_area_store(Arc::new(
            wow_data::SpellAreaStoreLikeCpp::from_rows_like_cpp(
                [spell_area(50_102, 12), spell_area(50_101, 34)],
                |spell| matches!(spell, 50_101 | 50_102),
                |area| matches!(area, 12 | 34),
                |_| true,
            )
            .store,
        ));
    }
    if completion_update_phase_shift {
        let phase_store = Arc::new(wow_data::PhaseStore::from_entries([]));
        session.set_phase_group_store(Arc::new(wow_data::PhaseGroupStore::from_entries(
            phase_store.as_ref(),
            [],
        )));
        session.set_phase_store(phase_store);
        session.set_terrain_swap_store(Arc::new(wow_data::TerrainSwapStore::default()));
        if completion_spell_id.is_none() && !completion_update_zone_auras {
            session.set_spell_store(Arc::new(first_login_noop_spell_store_like_cpp([])));
        }
        session.set_area_table_store(Arc::new(wow_data::AreaTableStore::from_entries([])));
    }

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
    if completion_update_phase_shift {
        assert!(
            session
                .with_owned_player_mut_like_cpp(|player| {
                    player
                        .unit_mut()
                        .world_mut()
                        .suppressed_phase_shift_mut()
                        .insert(10)
                })
                .is_some()
        );
    }
    if completion_update_zone_auras {
        crate::session::hub_mut(&mut session).set_player_zone_area_like_cpp(12, 34);
        session.set_player_zone_area_authority_complete_like_cpp(true);
        assert!(wow_world_spell::set_player_aura_authority_complete_like_cpp(&mut session, true,));
    }
    if in_raid_group {
        install_raid_membership_like_cpp(&mut session, player_guid);
    }
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
                        objective_counts: if let Some((prerequisite_complete, _)) = prerequisite {
                            vec![i32::from(prerequisite_complete), initial_count]
                        } else {
                            vec![initial_count]
                        },
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
            credit_guid: if empty_credit_guid {
                ObjectGuid::EMPTY
            } else {
                creature_guid
            },
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
        status.objective_counts.last().copied().unwrap_or_default(),
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

async fn assert_hidden_credit_transition_like_cpp(storing_flag: bool) {
    let (status, count, opcodes) = storing_objective_transition_outcome_like_cpp(
        storing_flag,
        QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        0,
        1,
        wow_constants::quest::QUEST_OBJECTIVE_FLAG_HIDE_CREDIT_MSG_LIKE_CPP,
    )
    .await;
    assert_eq!(status, QUEST_STATUS_COMPLETE_LIKE_CPP);
    assert_eq!(count, 1);
    assert_eq!(
        opcodes
            .iter()
            .filter(|opcode| **opcode == ServerOpcodes::UpdateObject)
            .count(),
        2,
        "hidden credit must not suppress either interaction refresh: {opcodes:?}"
    );
    let hidden_credit = if storing_flag {
        ServerOpcodes::QuestUpdateAddCreditSimple
    } else {
        ServerOpcodes::QuestUpdateAddCredit
    };
    for unexpected in [
        hidden_credit,
        ServerOpcodes::QuestUpdateComplete,
        ServerOpcodes::QuestGiverQuestComplete,
    ] {
        assert!(
            !opcodes.contains(&unexpected),
            "HIDE_CREDIT_MSG must not publish {unexpected:?}: {opcodes:?}"
        );
    }
}

#[tokio::test]
async fn storing_value_hide_credit_msg_completes_without_credit_packet_like_cpp() {
    assert_hidden_credit_transition_like_cpp(false).await;
}

#[tokio::test]
async fn storing_flag_hide_credit_msg_completes_without_credit_packet_like_cpp() {
    assert_hidden_credit_transition_like_cpp(true).await;
}

#[tokio::test]
async fn monster_no_credit_for_proxy_rejects_empty_victim_guid_like_cpp() {
    let (status, count, opcodes) = storing_objective_transition_outcome_with_proxy_rule_like_cpp(
        false,
        QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        0,
        1,
        0,
        wow_constants::quest::QUEST_FLAGS_EX_NO_CREDIT_FOR_PROXY_LIKE_CPP,
        true,
        None,
        0,
        false,
    )
    .await;
    assert_eq!(status, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    assert_eq!(count, 0);
    assert!(
        opcodes.is_empty(),
        "a rejected proxy must not mutate or publish quest progress: {opcodes:?}"
    );
}

#[tokio::test]
async fn monster_no_credit_for_proxy_allows_real_or_unflagged_proxy_credit_like_cpp() {
    for (quest_flags_ex, empty_credit_guid) in [
        (
            wow_constants::quest::QUEST_FLAGS_EX_NO_CREDIT_FOR_PROXY_LIKE_CPP,
            false,
        ),
        (0, true),
    ] {
        let outcome = storing_objective_transition_outcome_with_proxy_rule_like_cpp(
            false,
            QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            0,
            1,
            0,
            quest_flags_ex,
            empty_credit_guid,
            None,
            0,
            false,
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
}

async fn sequenced_objective_outcome_like_cpp(
    storing_flag: bool,
    prerequisite_complete: bool,
) -> (u8, i32, Vec<ServerOpcodes>) {
    storing_objective_transition_outcome_with_proxy_rule_like_cpp(
        storing_flag,
        QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        0,
        1,
        wow_constants::quest::QUEST_OBJECTIVE_FLAG_SEQUENCED_LIKE_CPP,
        0,
        false,
        Some((prerequisite_complete, 0)),
        0,
        false,
    )
    .await
}

#[tokio::test]
async fn sequenced_objective_waits_for_prerequisite_in_both_storage_families_like_cpp() {
    for storing_flag in [false, true] {
        let (status, count, opcodes) =
            sequenced_objective_outcome_like_cpp(storing_flag, false).await;
        assert_eq!(status, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
        assert_eq!(count, 0);
        assert!(
            opcodes.is_empty(),
            "an incomplete prerequisite must block progress: {opcodes:?}"
        );
    }
}

#[tokio::test]
async fn sequenced_objective_progresses_after_prerequisite_in_both_storage_families_like_cpp() {
    for storing_flag in [false, true] {
        let expected_credit = if storing_flag {
            ServerOpcodes::QuestUpdateAddCreditSimple
        } else {
            ServerOpcodes::QuestUpdateAddCredit
        };
        let outcome = sequenced_objective_outcome_like_cpp(storing_flag, true).await;
        assert_storing_objective_transition_like_cpp(
            outcome,
            QUEST_STATUS_COMPLETE_LIKE_CPP,
            1,
            2,
            expected_credit,
        );
    }
}

#[tokio::test]
async fn completed_storing_flag_ignores_nonnegative_repeat_events_like_cpp() {
    for add_count in [0, 1] {
        let (status, count, opcodes) = storing_objective_transition_outcome_like_cpp(
            true,
            QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            1,
            add_count,
            0,
        )
        .await;
        assert_eq!(status, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
        assert_eq!(count, 1);
        assert!(
            opcodes.is_empty(),
            "a completed flag must ignore add_count {add_count}: {opcodes:?}"
        );
    }
}

const THRESHOLD_QUEST_ID: u32 = 13_634;
const THRESHOLD_CLICK_NPC_ENTRY: u32 = 714;
const THRESHOLD_CLICK_SPELL_ID: u32 = 914;

async fn threshold_negative_outcome_like_cpp(
    objective_type: u8,
    objective_flags: u32,
    in_raid_group: bool,
    status_requests: Option<
        Arc<std::sync::Mutex<Vec<wow_persistence::PlayerQuestStatusPersistenceRequestLikeCpp>>>,
    >,
    initial_status: u8,
    initial_quantity: i32,
    new_quantity: i32,
    completion_spell_id: Option<u32>,
) -> (
    u8,
    Vec<ServerOpcodes>,
    Option<wow_entities::PlayerQuestStatusSaveTypeLikeCpp>,
) {
    let (mut session, _, send_rx) = make_session();
    session.set_creature_spawn_catalogs_like_cpp(Arc::new(
        session.creature_spawn_catalogs_for_test_like_cpp(),
    ));
    if let Some(status_requests) = status_requests {
        session.set_player_quest_persistence_port_like_cpp(Arc::new(
            crate::player::quest_persistence_test_fixture::PlayerQuestPersistencePortFixtureLikeCpp {
                status_requests,
                ..Default::default()
            },
        ));
    }
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 46);
    let creature_guid = test_creature_guid(134);
    let mut quest = test_quest_template(THRESHOLD_QUEST_ID);
    quest.quest_type = 2;
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: THRESHOLD_QUEST_ID * 10,
        quest_id: THRESHOLD_QUEST_ID,
        obj_type: objective_type,
        order: 0,
        storage_index: -1,
        object_id: if objective_type == QUEST_OBJECTIVE_MONEY_LIKE_CPP {
            0
        } else if objective_type == QUEST_OBJECTIVE_CURRENCY_LIKE_CPP {
            395
        } else {
            7
        },
        amount: 100,
        flags: objective_flags,
        flags2: 0,
        progress_bar_weight: 0.0,
        completion_effect: None,
        description: String::new(),
    });
    if let Some(spell_id) = completion_spell_id {
        quest.objectives.last_mut().unwrap().completion_effect =
            Some(wow_data::quest::QuestObjectiveAction {
                game_event_id: None,
                spell_id: Some(spell_id),
                conversation_id: None,
                update_phase_shift: false,
                update_zone_auras: false,
            });
        session.set_spell_store(Arc::new(first_login_noop_spell_store_like_cpp([
            spell_id as i32
        ])));
    }
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
        .expect("canonical threshold viewer map");
    if in_raid_group {
        install_raid_membership_like_cpp(&mut session, player_guid);
    }
    assert!(
        session
            .mutate_player_quest_gameplay_like_cpp(|state| {
                state.insert_status_like_cpp(
                    THRESHOLD_QUEST_ID,
                    crate::handlers::quest::PlayerQuestStatus {
                        quest_id: THRESHOLD_QUEST_ID,
                        status: initial_status,
                        explored: false,
                        accept_time_secs: 0,
                        end_time_secs: 0,
                        objective_counts: vec![],
                        slot: 0,
                    },
                );
            })
            .is_some(),
        "threshold fixture must use the canonical Player owner"
    );
    session.set_player_gold_like_cpp(initial_quantity as u64);
    assert!(
        session.set_player_currencies_like_cpp(std::collections::HashMap::from([(
            395,
            wow_entities::PlayerCurrency {
                state: wow_entities::PlayerCurrencyState::Unchanged,
                quantity: initial_quantity as u32,
                weekly_quantity: 0,
                tracked_quantity: 0,
                increased_cap_quantity: 0,
                earned_quantity: 0,
                flags: 0,
            },
        )]))
    );
    let mut faction = FactionEntry::for_test_like_cpp(7, 5);
    faction.reputation_flags[0] = ReputationFlagsLikeCpp::VISIBLE.bits();
    session.set_faction_store(Arc::new(FactionStore::from_entries([faction])));
    let mut reputation =
        wow_progression::FactionStateLikeCpp::new_like_cpp(7, 5, ReputationFlagsLikeCpp::VISIBLE);
    reputation.standing = initial_quantity;
    assert!(
        session
            .mutate_reputation_mgr_like_cpp(|manager| {
                manager.insert_state_for_test_like_cpp(reputation);
            })
            .is_some(),
        "threshold reputation must use the canonical Player owner"
    );
    session.set_condition_store(Arc::new(
        ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
            source_type: ConditionSourceType::SpellClickEvent,
            source_group: THRESHOLD_CLICK_NPC_ENTRY,
            source_entry: THRESHOLD_CLICK_SPELL_ID as i32,
            source_id: 0,
            condition_type: ConditionType::Aura,
            condition_value1: 999_999,
            ..Condition::default()
        }]),
    ));
    session.set_npc_spell_click_store(Arc::new(NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
        [wow_data::NpcSpellClickRowLikeCpp {
            npc_entry: THRESHOLD_CLICK_NPC_ENTRY,
            spell_id: THRESHOLD_CLICK_SPELL_ID,
            cast_flags: 0,
            user_type: wow_data::SPELL_CLICK_USER_ANY_LIKE_CPP,
        }],
        |entry| entry == THRESHOLD_CLICK_NPC_ENTRY,
        |spell| spell == THRESHOLD_CLICK_SPELL_ID,
    )));
    add_canonical_test_creature(
        &canonical,
        creature_guid,
        THRESHOLD_CLICK_NPC_ENTRY,
        Position::new(12.0, 0.0, 0.0, 0.0),
        UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32,
    );
    session
        .core
        .client_visible_guids_like_cpp
        .insert(creature_guid);
    let _ = drain_server_opcodes(&send_rx);

    session.set_item_guid_generator_like_cpp(Arc::new(wow_core::ObjectGuidGenerator::new(
        wow_core::guid::HighGuid::Item,
        1,
    )));

    match objective_type {
        QUEST_OBJECTIVE_MONEY_LIKE_CPP => session.money_changed_like_cpp(new_quantity as u64).await,
        QUEST_OBJECTIVE_CURRENCY_LIKE_CPP => {
            session
                .currency_changed_like_cpp(395, new_quantity - initial_quantity)
                .await
        }
        QUEST_OBJECTIVE_MIN_REPUTATION_LIKE_CPP => {
            session
                .reputation_changed_like_cpp(7, new_quantity - initial_quantity)
                .await
        }
        _ => unreachable!("threshold regression objective type"),
    }

    let status = session
        .player_quest_gameplay_snapshot_like_cpp()
        .and_then(|state| {
            state
                .status_like_cpp(THRESHOLD_QUEST_ID)
                .map(|status| status.status)
        })
        .expect("threshold quest status");
    let save_type = session
        .with_owned_player_like_cpp(|player| {
            player
                .gameplay_state()
                .quests
                .status_save_type_like_cpp(THRESHOLD_QUEST_ID)
        })
        .flatten();
    (status, drain_server_opcodes(&send_rx), save_type)
}

async fn assert_threshold_negative_transitions_like_cpp(objective_type: u8) {
    for (flags, expected_status) in [
        (0, QUEST_STATUS_INCOMPLETE_LIKE_CPP),
        (
            wow_constants::quest::QUEST_OBJECTIVE_FLAG_OPTIONAL_LIKE_CPP,
            QUEST_STATUS_COMPLETE_LIKE_CPP,
        ),
    ] {
        let (status, opcodes, _) = threshold_negative_outcome_like_cpp(
            objective_type,
            flags,
            false,
            None,
            QUEST_STATUS_COMPLETE_LIKE_CPP,
            100,
            90,
            None,
        )
        .await;
        assert_eq!(status, expected_status, "transition packets: {opcodes:?}");
        assert_eq!(
            opcodes
                .iter()
                .filter(|opcode| **opcode == ServerOpcodes::UpdateObject)
                .count(),
            1,
            "a threshold completion-state change must refresh once: {opcodes:?}"
        );
        for unexpected in [
            ServerOpcodes::QuestUpdateAddCredit,
            ServerOpcodes::QuestUpdateAddCreditSimple,
            ServerOpcodes::QuestUpdateComplete,
            ServerOpcodes::QuestGiverQuestComplete,
        ] {
            assert!(
                !opcodes.contains(&unexpected),
                "negative threshold progress must not publish {unexpected:?}: {opcodes:?}"
            );
        }
    }
}

#[tokio::test]
async fn money_negative_transition_honors_optional_and_refreshes_like_cpp() {
    assert_threshold_negative_transitions_like_cpp(QUEST_OBJECTIVE_MONEY_LIKE_CPP).await;
}

#[tokio::test]
async fn currency_negative_transition_honors_optional_and_refreshes_like_cpp() {
    assert_threshold_negative_transitions_like_cpp(QUEST_OBJECTIVE_CURRENCY_LIKE_CPP).await;
}

#[tokio::test]
async fn reputation_negative_transition_honors_optional_and_refreshes_like_cpp() {
    assert_threshold_negative_transitions_like_cpp(QUEST_OBJECTIVE_MIN_REPUTATION_LIKE_CPP).await;
}

#[tokio::test]
async fn raid_objectives_require_quest_admission_in_both_storage_families_like_cpp() {
    for storing_flag in [false, true] {
        let (status, count, opcodes) =
            storing_objective_transition_outcome_with_proxy_rule_like_cpp(
                storing_flag,
                QUEST_STATUS_INCOMPLETE_LIKE_CPP,
                0,
                1,
                0,
                0,
                false,
                None,
                0,
                true,
            )
            .await;
        assert_eq!(status, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
        assert_eq!(count, 0);
        assert!(
            opcodes.is_empty(),
            "an unflagged quest must not progress in raid: {opcodes:?}"
        );

        for in_raid_group in [true, false] {
            let expected_credit = if storing_flag {
                ServerOpcodes::QuestUpdateAddCreditSimple
            } else {
                ServerOpcodes::QuestUpdateAddCredit
            };
            let outcome = storing_objective_transition_outcome_with_proxy_rule_like_cpp(
                storing_flag,
                QUEST_STATUS_INCOMPLETE_LIKE_CPP,
                0,
                1,
                0,
                0,
                false,
                None,
                wow_data::quest::QUEST_FLAGS_RAID_GROUP_OK_LIKE_CPP,
                in_raid_group,
            )
            .await;
            assert_storing_objective_transition_like_cpp(
                outcome,
                QUEST_STATUS_COMPLETE_LIKE_CPP,
                1,
                2,
                expected_credit,
            );
        }
    }
}

#[tokio::test]
async fn raid_money_progresses_without_a_raid_quest_flag_like_cpp() {
    let (status, opcodes, _) = threshold_negative_outcome_like_cpp(
        QUEST_OBJECTIVE_MONEY_LIKE_CPP,
        0,
        true,
        None,
        QUEST_STATUS_COMPLETE_LIKE_CPP,
        100,
        90,
        None,
    )
    .await;
    assert_eq!(status, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    assert_eq!(
        opcodes
            .iter()
            .filter(|opcode| **opcode == ServerOpcodes::UpdateObject)
            .count(),
        1,
        "money remains progressable in raid without a quest raid flag: {opcodes:?}"
    );
    for unexpected in [
        ServerOpcodes::QuestUpdateAddCredit,
        ServerOpcodes::QuestUpdateComplete,
        ServerOpcodes::QuestGiverQuestComplete,
    ] {
        assert!(!opcodes.contains(&unexpected));
    }
}

async fn inline_completion_order_outcome_like_cpp(
    storing_flag: bool,
) -> (Vec<ServerOpcodes>, u8, u8, Vec<i32>) {
    const FIRST_QUEST: u32 = 13_635;
    const SECOND_QUEST: u32 = 13_636;
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 47);
    let creature_guid = test_creature_guid(135);
    let objective_type = if storing_flag {
        wow_constants::quest::QUEST_OBJECTIVE_CRITERIA_TREE_LIKE_CPP
    } else {
        wow_constants::quest::QUEST_OBJECTIVE_MONSTER_LIKE_CPP
    };
    let objective =
        |quest_id, objective_id, object_id, order, storage_index| wow_data::quest::QuestObjective {
            id: objective_id,
            quest_id,
            obj_type: objective_type,
            order,
            storage_index,
            object_id,
            amount: 1,
            flags: 0,
            flags2: 0,
            progress_bar_weight: 0.0,
            completion_effect: None,
            description: String::new(),
        };
    let mut first = test_quest_template(FIRST_QUEST);
    first
        .objectives
        .push(objective(FIRST_QUEST, FIRST_QUEST * 10, 713, 0, 0));
    let mut second = test_quest_template(SECOND_QUEST);
    second
        .objectives
        .push(objective(SECOND_QUEST, SECOND_QUEST * 10, 713, 0, 0));
    second
        .objectives
        .push(objective(SECOND_QUEST, SECOND_QUEST * 10 + 1, 714, 1, 1));
    session.set_player_guid(Some(player_guid));
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [first, second],
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
        .expect("canonical inline-completion map");
    assert!(
        session
            .mutate_player_quest_gameplay_like_cpp(|state| {
                for (quest_id, objective_counts) in
                    [(FIRST_QUEST, vec![0]), (SECOND_QUEST, vec![0, 0])]
                {
                    state.insert_status_like_cpp(
                        quest_id,
                        crate::handlers::quest::PlayerQuestStatus {
                            quest_id,
                            status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
                            explored: false,
                            accept_time_secs: 0,
                            end_time_secs: 0,
                            objective_counts,
                            slot: 0,
                        },
                    );
                }
            })
            .is_some(),
        "inline-completion fixture must use the canonical Player owner"
    );
    session.set_condition_store(Arc::new(
        ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
            source_type: ConditionSourceType::SpellClickEvent,
            source_group: 713,
            source_entry: 913,
            source_id: 0,
            condition_type: ConditionType::Aura,
            condition_value1: 999_999,
            ..Condition::default()
        }]),
    ));
    session.set_npc_spell_click_store(Arc::new(NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
        [wow_data::NpcSpellClickRowLikeCpp {
            npc_entry: 713,
            spell_id: 913,
            cast_flags: 0,
            user_type: wow_data::SPELL_CLICK_USER_ANY_LIKE_CPP,
        }],
        |entry| entry == 713,
        |spell| spell == 913,
    )));
    add_canonical_test_creature(
        &canonical,
        creature_guid,
        713,
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
            object_id: 713,
            add_count: 1,
        }
    } else {
        crate::session::quest::QuestObjectiveOperationLikeCpp::StoringValue {
            objective_type,
            object_id: 713,
            add_count: 1,
            credit_guid: creature_guid,
        }
    };
    session
        .quest_objective_operation_like_cpp(&generator, operation)
        .await;

    let state = session
        .player_quest_gameplay_snapshot_like_cpp()
        .expect("inline-completion quest state");
    (
        drain_server_opcodes(&send_rx),
        state.status_like_cpp(FIRST_QUEST).unwrap().status,
        state.status_like_cpp(SECOND_QUEST).unwrap().status,
        state
            .status_like_cpp(SECOND_QUEST)
            .unwrap()
            .objective_counts
            .clone(),
    )
}

#[tokio::test]
async fn quest_completion_runs_inline_before_the_next_quest_credit_like_cpp() {
    for storing_flag in [false, true] {
        let expected_credit = if storing_flag {
            ServerOpcodes::QuestUpdateAddCreditSimple
        } else {
            ServerOpcodes::QuestUpdateAddCredit
        };
        let (opcodes, first_status, second_status, second_counts) =
            inline_completion_order_outcome_like_cpp(storing_flag).await;
        assert_eq!(
            opcodes,
            vec![
                expected_credit,
                ServerOpcodes::UpdateObject,
                expected_credit,
                ServerOpcodes::UpdateObject,
            ]
        );
        assert_eq!(first_status, QUEST_STATUS_COMPLETE_LIKE_CPP);
        assert_eq!(second_status, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
        assert_eq!(second_counts, vec![1, 0]);
    }
}

#[tokio::test]
async fn negative_objective_marks_default_without_immediate_persistence_like_cpp() {
    let fixture =
        crate::player::quest_persistence_test_fixture::PlayerQuestPersistencePortFixtureLikeCpp::default();
    let status_requests = Arc::clone(&fixture.status_requests);
    let (status, opcodes, save_type) = threshold_negative_outcome_like_cpp(
        QUEST_OBJECTIVE_MONEY_LIKE_CPP,
        0,
        false,
        Some(status_requests.clone()),
        QUEST_STATUS_COMPLETE_LIKE_CPP,
        100,
        90,
        None,
    )
    .await;

    assert_eq!(status, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    assert_eq!(
        save_type,
        Some(wow_entities::PlayerQuestStatusSaveTypeLikeCpp::Default)
    );
    assert!(
        status_requests.lock().unwrap().is_empty(),
        "objective progress must not write quest status immediately"
    );
    assert_eq!(
        opcodes
            .iter()
            .filter(|opcode| **opcode == ServerOpcodes::UpdateObject)
            .count(),
        1,
        "the state transition still refreshes interactions: {opcodes:?}"
    );
}

#[tokio::test]
async fn completion_updates_tracker_only_when_enabled_without_status_save_like_cpp() {
    for enabled in [false, true] {
        let fixture =
            crate::player::quest_persistence_test_fixture::PlayerQuestPersistencePortFixtureLikeCpp::default();
        let status_requests = Arc::clone(&fixture.status_requests);
        let tracker_requests = Arc::clone(&fixture.tracker_completion_requests);
        let (status, count, _) = storing_objective_transition_outcome_with_persistence_like_cpp(
            false,
            QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            0,
            1,
            0,
            0,
            false,
            None,
            0,
            false,
            enabled,
            Some(fixture),
            None,
            false,
            false,
        )
        .await;

        assert_eq!(status, QUEST_STATUS_COMPLETE_LIKE_CPP);
        assert_eq!(count, 1);
        assert!(status_requests.lock().unwrap().is_empty());
        assert_eq!(
            *tracker_requests.lock().unwrap(),
            if enabled {
                vec![(45, TRANSITION_QUEST_ID)]
            } else {
                Vec::new()
            }
        );
    }
}

#[tokio::test]
async fn completion_spell_runs_between_credit_and_quest_completion_like_cpp() {
    let (status, count, opcodes) = storing_objective_transition_outcome_with_persistence_like_cpp(
        false,
        QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        0,
        1,
        0,
        0,
        false,
        None,
        0,
        false,
        false,
        None,
        Some(50_101),
        false,
        false,
    )
    .await;

    assert_eq!(status, QUEST_STATUS_COMPLETE_LIKE_CPP);
    assert_eq!(count, 1);
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::QuestUpdateAddCredit,
            ServerOpcodes::SpellGo,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::UpdateObject,
        ]
    );
}

#[tokio::test]
async fn storing_flag_completion_spell_runs_before_quest_completion_like_cpp() {
    let (status, count, opcodes) = storing_objective_transition_outcome_with_persistence_like_cpp(
        true,
        QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        0,
        1,
        0,
        0,
        false,
        None,
        0,
        false,
        false,
        None,
        Some(50_101),
        false,
        false,
    )
    .await;

    assert_eq!(status, QUEST_STATUS_COMPLETE_LIKE_CPP);
    assert_eq!(count, 1);
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::QuestUpdateAddCreditSimple,
            ServerOpcodes::SpellGo,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::UpdateObject,
        ]
    );
}

async fn assert_threshold_completion_spell_like_cpp(objective_type: u8) {
    let (status, opcodes, _) = threshold_negative_outcome_like_cpp(
        objective_type,
        0,
        false,
        None,
        QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        0,
        100,
        Some(50_101),
    )
    .await;
    assert_eq!(status, QUEST_STATUS_COMPLETE_LIKE_CPP);
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::UpdateObject,
        ]
    );
}

#[tokio::test]
async fn money_completion_spell_runs_before_quest_completion_like_cpp() {
    assert_threshold_completion_spell_like_cpp(QUEST_OBJECTIVE_MONEY_LIKE_CPP).await;
}

#[tokio::test]
async fn currency_completion_spell_runs_before_quest_completion_like_cpp() {
    assert_threshold_completion_spell_like_cpp(QUEST_OBJECTIVE_CURRENCY_LIKE_CPP).await;
}

#[tokio::test]
async fn reputation_completion_spell_runs_before_quest_completion_like_cpp() {
    assert_threshold_completion_spell_like_cpp(QUEST_OBJECTIVE_MIN_REPUTATION_LIKE_CPP).await;
}

#[tokio::test]
async fn zone_and_area_aura_completion_effects_cast_after_final_refresh_like_cpp() {
    let (status, count, opcodes) = storing_objective_transition_outcome_with_persistence_like_cpp(
        false,
        QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        0,
        1,
        0,
        0,
        false,
        None,
        0,
        false,
        false,
        None,
        None,
        true,
        false,
    )
    .await;

    assert_eq!(status, QUEST_STATUS_COMPLETE_LIKE_CPP);
    assert_eq!(count, 1);
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::QuestUpdateAddCredit,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::SpellGo,
            ServerOpcodes::SpellGo,
        ]
    );
}

#[tokio::test]
async fn completion_phase_shift_runs_after_final_interaction_refresh_like_cpp() {
    let (status, count, opcodes) = storing_objective_transition_outcome_with_persistence_like_cpp(
        false,
        QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        0,
        1,
        0,
        0,
        false,
        None,
        0,
        false,
        false,
        None,
        None,
        false,
        true,
    )
    .await;

    assert_eq!(status, QUEST_STATUS_COMPLETE_LIKE_CPP);
    assert_eq!(count, 1);
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::QuestUpdateAddCredit,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::PhaseShiftChange,
            ServerOpcodes::UpdateObject,
        ]
    );
}

#[tokio::test]
async fn completion_phase_shift_precedes_zone_and_area_aura_casts_like_cpp() {
    let (status, count, opcodes) = storing_objective_transition_outcome_with_persistence_like_cpp(
        false,
        QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        0,
        1,
        0,
        0,
        false,
        None,
        0,
        false,
        false,
        None,
        None,
        true,
        true,
    )
    .await;

    assert_eq!(status, QUEST_STATUS_COMPLETE_LIKE_CPP);
    assert_eq!(count, 1);
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::QuestUpdateAddCredit,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::PhaseShiftChange,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::SpellGo,
            ServerOpcodes::SpellGo,
        ]
    );
}
