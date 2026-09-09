//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn update_visible_gameobjects_sends_dynamic_flags_for_active_objective_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let go_entry = 8_123;
    let gameobject_guid = test_gameobject_guid(go_entry, 132);
    let quest_id = 12_541;
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
    session.player_quests.insert(
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

    assert_eq!(session.update_visible_gameobjects_like_cpp(), 1);
    assert_eq!(
        send_rx.try_recv().expect("gameobject dynamic flags update"),
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
#[test]
fn update_visible_gameobjects_questgiver_future_status_does_not_activate_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let go_entry = 8_129;
    let gameobject_guid = test_gameobject_guid(go_entry, 139);
    let quest_id = 12_546;
    let mut quest = test_quest_template(quest_id);
    quest.quest_level = 85;
    quest.min_level = 85;
    let mut quest_store = wow_data::quest::QuestStore::from_quests_like_cpp([quest]);
    quest_store
        .gameobject_starter_quests
        .insert(go_entry, vec![quest_id]);

    session.set_player_guid(Some(player_guid));
    session.set_quest_store(Arc::new(quest_store));
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
            go_type: Some(wow_entities::GAMEOBJECT_TYPE_QUESTGIVER as u8),
            ..Default::default()
        },
    );

    assert_eq!(session.update_visible_gameobjects_like_cpp(), 1);
    assert_eq!(
        send_rx
            .try_recv()
            .expect("future questgiver dynamic flags update"),
        expected_gameobject_dynamic_flags_update_like_cpp(gameobject_guid, 571, 0)
    );
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn update_visible_gameobjects_sends_dynamic_flags_for_chest_quest_loot_reference_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let go_entry = 8_126;
    let gameobject_guid = test_gameobject_guid(go_entry, 136);
    let loot_id = 700;
    let reference_id = 701;
    let quest_item_id = 9_001;
    let quest_id = 12_543;
    let mut quest = test_quest_template(quest_id);
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP,
        order: 0,
        storage_index: 0,
        object_id: quest_item_id,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    let mut gameobject_store = wow_loot::LootStore::for_kind_like_cpp(LootStoreKind::Gameobject);
    gameobject_store
        .load_rows_like_cpp(
            [wow_loot::LootTemplateRow {
                entry: loot_id,
                item: wow_loot::LootStoreItem {
                    item_id: reference_id,
                    reference: reference_id,
                    chance: 100.0,
                    needs_quest: false,
                    loot_mode: 1,
                    group_id: 0,
                    min_count: 1,
                    max_count: 1,
                },
            }],
            |_| true,
        )
        .unwrap();
    let mut reference_store = wow_loot::LootStore::for_kind_like_cpp(LootStoreKind::Reference);
    reference_store
        .load_rows_like_cpp(
            [wow_loot::LootTemplateRow {
                entry: reference_id,
                item: wow_loot::LootStoreItem {
                    item_id: quest_item_id as u32,
                    reference: 0,
                    chance: 100.0,
                    needs_quest: true,
                    loot_mode: 1,
                    group_id: 0,
                    min_count: 1,
                    max_count: 1,
                },
            }],
            |_| true,
        )
        .unwrap();
    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Gameobject, gameobject_store);
    stores.insert(LootStoreKind::Reference, reference_store);

    session.set_player_guid(Some(player_guid));
    session.set_loot_stores(Arc::new(stores));
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.player_quests.insert(
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
            chest_loot_source: Some(wow_entities::GameObjectLootSource {
                loot_id,
                ..Default::default()
            }),
            ..Default::default()
        },
    );

    assert_eq!(session.update_visible_gameobjects_like_cpp(), 1);
    assert_eq!(
        send_rx
            .try_recv()
            .expect("chest quest-loot dynamic flags update"),
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
// C++ anchor: /home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Updates/ViewerDependentValues.h:97-101
#[test]
fn update_visible_gameobjects_gm_chest_without_activation_gets_activate_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let go_entry = 8_130;
    let gameobject_guid = test_gameobject_guid(go_entry, 140);
    let quest_id = 12_547;

    session.set_player_guid(Some(player_guid));
    session.set_player_game_master_like_cpp(true);
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
            chest_loot_source: Some(wow_entities::GameObjectLootSource {
                chest_quest_id: quest_id,
                ..Default::default()
            }),
            ..Default::default()
        },
    );

    assert_eq!(session.update_visible_gameobjects_like_cpp(), 1);
    assert_eq!(
        send_rx.try_recv().expect("gm chest dynamic flags update"),
        expected_gameobject_dynamic_flags_update_like_cpp(
            gameobject_guid,
            571,
            wow_entities::GO_DYNFLAG_LO_ACTIVATE
        )
    );
    assert!(send_rx.try_recv().is_err());
}
// C++ anchor: /home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Updates/ViewerDependentValues.h:103-111
#[test]
fn update_visible_gameobjects_gm_goober_without_activation_gets_activate_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let go_entry = 8_131;
    let gameobject_guid = test_gameobject_guid(go_entry, 141);
    let quest_id = 12_548;

    session.set_player_guid(Some(player_guid));
    session.set_player_game_master_like_cpp(true);
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
            go_type: Some(wow_entities::GAMEOBJECT_TYPE_GOOBER as u8),
            goober_use_source: Some(wow_entities::GooberUseSource {
                quest_id,
                ..Default::default()
            }),
            ..Default::default()
        },
    );

    assert_eq!(session.update_visible_gameobjects_like_cpp(), 1);
    assert_eq!(
        send_rx.try_recv().expect("gm goober dynamic flags update"),
        expected_gameobject_dynamic_flags_update_like_cpp(
            gameobject_guid,
            571,
            wow_entities::GO_DYNFLAG_LO_ACTIVATE
        )
    );
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn update_visible_gameobjects_sends_dynamic_flags_for_gathering_node_quest_loot_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let go_entry = 8_127;
    let gameobject_guid = test_gameobject_guid(go_entry, 137);
    let loot_id = 702;
    let quest_item_id = 9_002;
    let quest_id = 12_544;
    let mut quest = test_quest_template(quest_id);
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP,
        order: 0,
        storage_index: 0,
        object_id: quest_item_id,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    let mut gameobject_store = wow_loot::LootStore::for_kind_like_cpp(LootStoreKind::Gameobject);
    gameobject_store
        .load_rows_like_cpp(
            [wow_loot::LootTemplateRow {
                entry: loot_id,
                item: wow_loot::LootStoreItem {
                    item_id: quest_item_id as u32,
                    reference: 0,
                    chance: 100.0,
                    needs_quest: true,
                    loot_mode: 1,
                    group_id: 0,
                    min_count: 1,
                    max_count: 1,
                },
            }],
            |_| true,
        )
        .unwrap();
    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Gameobject, gameobject_store);

    session.set_player_guid(Some(player_guid));
    session.set_loot_stores(Arc::new(stores));
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.player_quests.insert(
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
            go_type: Some(wow_entities::GAMEOBJECT_TYPE_GATHERING_NODE as u8),
            go_state: Some(wow_entities::GoState::Ready),
            gathering_node_loot_id: Some(loot_id),
            ..Default::default()
        },
    );

    assert_eq!(session.update_visible_gameobjects_like_cpp(), 1);
    assert_eq!(
        send_rx
            .try_recv()
            .expect("gathering node quest-loot dynamic flags update"),
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
#[test]
fn update_visible_gameobjects_adds_no_interact_for_failed_player_condition_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let go_entry = 8_128;
    let gameobject_guid = test_gameobject_guid(go_entry, 138);
    let quest_id = 12_545;
    let condition_id = 44;
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

    session.player_class = 1;
    session.set_player_guid(Some(player_guid));
    session.set_player_condition_store(Arc::new(wow_data::PlayerConditionStore::from_entries([
        wow_data::PlayerConditionEntry {
            id: condition_id,
            class_mask: 1 << 1,
            ..Default::default()
        },
    ])));
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.player_quests.insert(
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
            condition_id1: Some(condition_id),
            ..Default::default()
        },
    );

    assert_eq!(session.update_visible_gameobjects_like_cpp(), 1);
    assert_eq!(
        send_rx
            .try_recv()
            .expect("conditioned gameobject dynamic flags update"),
        expected_gameobject_dynamic_flags_update_like_cpp(
            gameobject_guid,
            571,
            wow_entities::GO_DYNFLAG_LO_ACTIVATE
                | wow_entities::GO_DYNFLAG_LO_SPARKLE
                | wow_entities::GO_DYNFLAG_LO_HIGHLIGHT
                | wow_entities::GO_DYNFLAG_LO_NO_INTERACT
        )
    );
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn update_visible_gameobjects_skips_unknown_quest_gameobject_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(8_124, 133);

    session.set_player_guid(Some(player_guid));
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
        8_124,
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

    assert_eq!(session.update_visible_gameobjects_like_cpp(), 0);
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn accept_invite_to_raid_group_triggers_visible_gameobject_refresh_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let inviter_guid = ObjectGuid::create_player(1, 41);
    let player_guid = ObjectGuid::create_player(1, 42);
    let go_entry = 8_125;
    let gameobject_guid = test_gameobject_guid(go_entry, 134);
    let quest_id = 12_542;
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
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(inviter_guid);
    group.convert_to_raid_like_cpp();
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    let player_registry = Arc::new(crate::session::directory::PlayerRegistry::default());
    let (inviter_tx, _inviter_rx) = flume::bounded(8);
    let (player_tx, _player_rx) = flume::bounded(8);
    let (inviter_command_tx, _inviter_command_rx) = flume::unbounded();
    let (player_command_tx, _player_command_rx) = flume::unbounded();
    player_registry.register_or_replace(
        inviter_guid,
        broadcast_info_with_command(inviter_guid, inviter_tx, inviter_command_tx),
        Default::default(),
    );
    player_registry.register_or_replace(
        player_guid,
        broadcast_info_with_command(player_guid, player_tx, player_command_tx),
        Default::default(),
    );
    let pending_invites = Arc::new(PendingInvites::default());
    pending_invites.seed_invite_like_cpp(
        player_guid,
        PendingInviteLikeCpp::new_existing_group(
            inviter_guid,
            group_guid,
            wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP,
        ),
    );

    session.set_player_guid(Some(player_guid));
    session.set_player_registry(Arc::clone(&player_registry));
    session.set_group_registry(Arc::clone(&group_registry), pending_invites);
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.player_quests.insert(
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

    let mut pkt = wow_packet::WorldPacket::new_empty();
    pkt.write_bit(false);
    pkt.write_bit(true);
    pkt.write_bit(false);
    pkt.flush_bits();
    pkt.reset_read();
    session.handle_party_invite_response(pkt).await;

    assert_eq!(session.group_guid, Some(group_guid));
    assert!(
        group_registry
            .get(&group_guid)
            .is_some_and(|group| group.is_raid_group() && group.members.contains(&player_guid))
    );
    let packets = drain_server_packet_bytes(&send_rx);
    assert!(
        packets.iter().any(|bytes| {
            wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
                == Some(ServerOpcodes::UpdateObject)
        }),
        "raid AddMember must send PlayerData::PartyType update"
    );
    assert!(
        packets.iter().any(|bytes| {
            bytes
                == &expected_gameobject_dynamic_flags_update_like_cpp(
                    gameobject_guid,
                    571,
                    wow_entities::GO_DYNFLAG_LO_ACTIVATE
                        | wow_entities::GO_DYNFLAG_LO_SPARKLE
                        | wow_entities::GO_DYNFLAG_LO_HIGHLIGHT,
                )
        }),
        "raid AddMember GO refresh"
    );
}
#[tokio::test]
async fn creature_kill_tracking_event_objective_auto_rewards_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let creature_guid = test_creature_guid(9_900);
    let quest_id = 12_501;
    let creature_entry = 9_901;
    let mut quest = test_quest_template(quest_id);
    quest.flags |= 0x0000_0400; // C++ QUEST_FLAGS_TRACKING_EVENT
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: 0, // C++ QUEST_OBJECTIVE_MONSTER
        order: 0,
        storage_index: 0,
        object_id: creature_entry as i32,
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
    session.player_quests.insert(
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
    session
        .on_creature_killed(creature_entry, creature_guid)
        .await;

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
            ServerOpcodes::QuestUpdateAddCredit,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::QuestGiverQuestComplete,
            ServerOpcodes::QuestUpdateComplete,
        ]
    );
}
