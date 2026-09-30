//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn gameobject_use_goober_multi_interact_matches_cpp_per_player_branch() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 14);
    session.set_player_guid(Some(player_guid));

    assert!(session.use_represented_gameobject_goober_state_like_cpp(
        gameobject_guid,
        player_guid,
        777,
        wow_entities::GooberUseSource {
            allow_multi_interact: true,
            ..Default::default()
        },
    ));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![RepresentedGameObjectUseEffect::GooberSetGoStateForPlayer {
            gameobject_guid,
            player_guid,
            go_state: wow_entities::GoState::Active,
        }]
    );
    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .unwrap();
    assert_eq!(state.per_player_state_player_guid, Some(player_guid));
    assert_eq!(
        state.per_player_go_state,
        Some(wow_entities::GoState::Active)
    );
    assert!(state.per_player_go_state_until.is_some());
    assert_eq!(
        send_rx.try_recv().expect("SetGoStateFor direct packet"),
        wow_packet::packets::misc::GameObjectSetStateLocal {
            object_guid: gameobject_guid,
            state: wow_entities::GoState::Active as u8,
        }
        .to_bytes()
    );

    session.represented_gameobject_use_effects.clear();
    session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default()
        .despawn_delay_secs = Some(45);
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);
    assert!(session.use_represented_gameobject_goober_state_like_cpp(
        gameobject_guid,
        player_guid,
        777,
        wow_entities::GooberUseSource {
            allow_multi_interact: true,
            consumable: true,
            ..Default::default()
        },
    ));
    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .unwrap();
    assert_eq!(state.per_player_despawn_secs, Some(45));
    assert!(state.per_player_despawn_until.is_some());
    assert_eq!(state.per_player_state_player_guid, Some(player_guid));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![RepresentedGameObjectUseEffect::GooberDespawnForPlayer {
            gameobject_guid,
            player_guid,
            despawn_secs: 45,
        }]
    );
    assert!(
        !session
            .client_visible_guids_like_cpp
            .contains(&gameobject_guid)
    );
    assert_eq!(
        send_rx
            .try_recv()
            .expect("DespawnForPlayer out-of-range update"),
        wow_packet::packets::update::UpdateObject::out_of_range_objects(
            vec![gameobject_guid],
            session.player_map_id_like_cpp(),
        )
        .to_bytes()
    );
}
#[tokio::test]
async fn process_pending_expires_per_player_gameobject_state_like_cpp_update() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.set_state(SessionState::LoggedIn);
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 141);
    {
        let state = session
            .represented_gameobject_use_states
            .entry(gameobject_guid)
            .or_default();
        state.go_state = Some(wow_entities::GoState::Ready);
        state.per_player_state_player_guid = Some(player_guid);
        state.per_player_go_state = Some(wow_entities::GoState::Active);
        state.per_player_go_state_until = Some(Instant::now() - Duration::from_secs(1));
        state.per_player_despawn_secs = Some(45);
        state.per_player_despawn_until = Some(Instant::now() - Duration::from_secs(1));
    }

    session.process_pending().await;

    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .unwrap();
    assert_eq!(state.per_player_state_player_guid, None);
    assert_eq!(state.per_player_go_state, None);
    assert_eq!(state.per_player_go_state_until, None);
    assert_eq!(state.per_player_despawn_secs, None);
    assert_eq!(state.per_player_despawn_until, None);
    assert!(
        session
            .represented_gameobject_use_effects
            .iter()
            .any(|effect| matches!(
                effect,
                RepresentedGameObjectUseEffect::GameObjectPerPlayerStateExpired {
                    gameobject_guid: expired_guid,
                    player_guid: expired_player_guid,
                    despawned: true,
                    needs_state_update: true,
                } if *expired_guid == gameobject_guid
                    && *expired_player_guid == player_guid
            ))
    );
}
#[tokio::test]
async fn process_pending_expires_gameobject_despawn_delay_like_cpp_update() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    session.set_state(SessionState::LoggedIn);
    let player_guid = ObjectGuid::create_player(1, 99);
    let same_map_guid = ObjectGuid::create_player(1, 77);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 142);
    let linked_trap_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 146);
    let (same_command_tx, same_command_rx) = flume::bounded(2);
    let (same_send_tx, _same_send_rx) = flume::bounded::<Vec<u8>>(1);
    let player_registry = Arc::new(PlayerRegistry::default());
    let mut same_info = broadcast_info(same_map_guid, same_send_tx);
    same_info.placement.map_id = 571;
    same_info.command_tx = same_command_tx;
    player_registry.register_or_replace(same_map_guid, same_info, Default::default());
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.set_player_registry(player_registry);
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);
    {
        let state = session
            .represented_gameobject_use_states
            .entry(gameobject_guid)
            .or_default();
        state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_GATHERING_NODE as u8);
        state.go_state = Some(wow_entities::GoState::Active);
        state.loot_state = Some(wow_entities::LootState::Activated);
        state.despawn_delay_secs = Some(15);
        state.despawn_delay_until = Some(Instant::now() - Duration::from_secs(1));
        state.linked_trap_guid = Some(linked_trap_guid);
    }
    let linked_state = session
        .represented_gameobject_use_states
        .entry(linked_trap_guid)
        .or_default();
    linked_state.map_id = Some(571);
    linked_state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_TRAP as u8);
    linked_state.loot_state = Some(wow_entities::LootState::Ready);
    session
        .client_visible_guids_like_cpp
        .insert(linked_trap_guid);

    session.process_pending().await;

    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .unwrap();
    assert_eq!(state.despawn_delay_until, None);
    assert_eq!(state.loot_state, Some(wow_entities::LootState::NotReady));
    assert_eq!(state.go_state, Some(wow_entities::GoState::Ready));
    assert!(
        !session
            .client_visible_guids_like_cpp
            .contains(&gameobject_guid)
    );
    let linked_state = session
        .represented_gameobject_use_states
        .get(&linked_trap_guid)
        .unwrap();
    assert_eq!(
        linked_state.loot_state,
        Some(wow_entities::LootState::NotReady)
    );
    assert!(
        !session
            .client_visible_guids_like_cpp
            .contains(&linked_trap_guid)
    );
    let opcodes = drain_server_opcodes(&send_rx);
    assert_eq!(
        opcodes
            .iter()
            .filter(|opcode| **opcode == ServerOpcodes::GameObjectDespawn)
            .count(),
        2
    );
    assert_eq!(opcodes.len(), 4);
    let linked_command = match same_command_rx.try_recv() {
        Ok(SessionCommand::SendIfVisibleLikeCpp(command)) => command,
        other => {
            panic!("expected represented linked trap despawn fanout command, got {other:?}")
        }
    };
    let mut linked_expected = (ServerOpcodes::GameObjectDespawn as u16)
        .to_le_bytes()
        .to_vec();
    linked_expected.extend_from_slice(&linked_trap_guid.to_raw_bytes());
    assert_eq!(linked_command.source_guid, linked_trap_guid);
    assert_eq!(linked_command.map_id, 571);
    assert_eq!(linked_command.instance_id, 0);
    assert_eq!(linked_command.packet_bytes, linked_expected);
    let command = match same_command_rx.try_recv() {
        Ok(SessionCommand::SendIfVisibleLikeCpp(command)) => command,
        other => panic!("expected represented delete despawn fanout command, got {other:?}"),
    };
    let mut expected = (ServerOpcodes::GameObjectDespawn as u16)
        .to_le_bytes()
        .to_vec();
    expected.extend_from_slice(&gameobject_guid.to_raw_bytes());
    assert_eq!(command.source_guid, gameobject_guid);
    assert_eq!(command.map_id, 571);
    assert_eq!(command.instance_id, 0);
    assert_eq!(command.packet_bytes, expected);
}
#[tokio::test]
async fn process_pending_clears_generic_just_deactivated_gameobject_like_cpp_update() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    session.set_state(SessionState::LoggedIn);
    let static_trap_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 143);
    let linked_trap_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 145);
    let owned_trap_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 144);
    let owner_guid = ObjectGuid::create_player(1, 99);
    for (guid, owner, linked_trap_entry) in [
        (static_trap_guid, None, Some(999)),
        (owned_trap_guid, Some(owner_guid), None),
    ] {
        let state = session
            .represented_gameobject_use_states
            .entry(guid)
            .or_default();
        state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_TRAP as u8);
        state.loot_state = Some(wow_entities::LootState::JustDeactivated);
        state.loot_state_unit_guid = owner_guid;
        state.use_count = 3;
        state.owner_guid = owner;
        state.linked_trap_entry = linked_trap_entry;
        if guid == static_trap_guid {
            state.linked_trap_guid = Some(linked_trap_guid);
        }
        session.client_visible_guids_like_cpp.insert(guid);
        session.loot_table.insert(
            guid,
            CreatureLoot {
                loot_guid: guid,
                coins: 7,
                unlooted_count: 1,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: Vec::new(),
                looted_by_player: false,
            },
        );
    }
    let linked_state = session
        .represented_gameobject_use_states
        .entry(linked_trap_guid)
        .or_default();
    linked_state.map_id = Some(571);
    linked_state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_TRAP as u8);
    linked_state.loot_state = Some(wow_entities::LootState::Ready);
    session
        .client_visible_guids_like_cpp
        .insert(linked_trap_guid);

    session.process_pending().await;

    let static_state = session
        .represented_gameobject_use_states
        .get(&static_trap_guid)
        .unwrap();
    assert_eq!(
        static_state.loot_state,
        Some(wow_entities::LootState::NotReady)
    );
    assert_eq!(static_state.loot_state_unit_guid, ObjectGuid::EMPTY);
    assert_eq!(static_state.use_count, 0);
    assert_eq!(static_state.linked_trap_entry, Some(999));
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&static_trap_guid)
    );
    assert!(!session.loot_table.contains_key(&static_trap_guid));

    let owned_state = session
        .represented_gameobject_use_states
        .get(&owned_trap_guid)
        .unwrap();
    assert_eq!(
        owned_state.loot_state,
        Some(wow_entities::LootState::NotReady)
    );
    assert!(
        !session
            .client_visible_guids_like_cpp
            .contains(&owned_trap_guid)
    );
    assert!(!session.loot_table.contains_key(&owned_trap_guid));
    let linked_state = session
        .represented_gameobject_use_states
        .get(&linked_trap_guid)
        .unwrap();
    assert_eq!(
        linked_state.loot_state,
        Some(wow_entities::LootState::NotReady)
    );
    assert!(
        !session
            .client_visible_guids_like_cpp
            .contains(&linked_trap_guid)
    );
    let opcodes = drain_server_opcodes(&send_rx);
    assert_eq!(
        opcodes
            .iter()
            .filter(|opcode| **opcode == ServerOpcodes::GameObjectDespawn)
            .count(),
        3
    );
    assert_eq!(opcodes.len(), 5);
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::GameObjectLinkedTrapDespawn {
                gameobject_guid: static_trap_guid,
                trap_entry: 999,
            },
            RepresentedGameObjectUseEffect::GameObjectJustDeactivatedCleared {
                gameobject_guid: static_trap_guid,
                deleted: false,
            },
            RepresentedGameObjectUseEffect::GameObjectJustDeactivatedCleared {
                gameobject_guid: owned_trap_guid,
                deleted: true,
            },
        ]
    );
}
#[tokio::test]
async fn represented_gameobject_chest_just_deactivated_consumable_sends_despawn_without_destroy_like_cpp()
 {
    let (mut session, _pkt_tx, send_rx) = make_session();
    session.set_state(SessionState::LoggedIn);
    let player_guid = ObjectGuid::create_player(1, 99);
    let same_map_guid = ObjectGuid::create_player(1, 77);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 245);
    let (same_command_tx, same_command_rx) = flume::bounded(2);
    let (same_send_tx, _same_send_rx) = flume::bounded::<Vec<u8>>(1);
    let player_registry = Arc::new(PlayerRegistry::default());
    let mut same_info = broadcast_info(same_map_guid, same_send_tx);
    same_info.placement.map_id = 571;
    same_info.command_tx = same_command_tx;
    player_registry.register_or_replace(same_map_guid, same_info, Default::default());
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.set_player_registry(player_registry);
    let state = session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default();
    state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_CHEST as u8);
    state.loot_state = Some(wow_entities::LootState::JustDeactivated);
    state.despawn_at_action = true;
    state.go_anim_progress = 0;
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    session.process_pending().await;

    let opcodes = drain_server_opcodes(&send_rx);
    assert_eq!(opcodes, vec![ServerOpcodes::GameObjectDespawn]);
    let command = match same_command_rx.try_recv() {
        Ok(SessionCommand::SendIfVisibleLikeCpp(command)) => command,
        other => panic!("expected represented chest despawn fanout command, got {other:?}"),
    };
    let mut expected = (ServerOpcodes::GameObjectDespawn as u16)
        .to_le_bytes()
        .to_vec();
    expected.extend_from_slice(&gameobject_guid.to_raw_bytes());
    assert_eq!(command.source_guid, gameobject_guid);
    assert_eq!(command.map_id, 571);
    assert_eq!(command.instance_id, 0);
    assert_eq!(command.packet_bytes, expected);
}
#[tokio::test]
async fn represented_gameobject_chest_just_deactivated_anim_progress_non_consumable_returns_ready_like_cpp()
 {
    let (mut session, _pkt_tx, send_rx) = make_session();
    session.set_state(SessionState::LoggedIn);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 246);
    let state = session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default();
    state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_CHEST as u8);
    state.loot_state = Some(wow_entities::LootState::JustDeactivated);
    state.despawn_at_action = false;
    state.go_anim_progress = 1;
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    session.process_pending().await;

    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .unwrap();
    assert_eq!(state.loot_state, Some(wow_entities::LootState::Ready));
    assert!(state.chest_restock_until.is_none());
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&gameobject_guid)
    );
    assert!(drain_server_opcodes(&send_rx).is_empty());
}
#[tokio::test]
async fn represented_gameobject_chest_just_deactivated_anim_progress_non_consumable_restock_not_ready_like_cpp()
 {
    let (mut session, _pkt_tx, send_rx) = make_session();
    session.set_state(SessionState::LoggedIn);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 248);
    let state = session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default();
    state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_CHEST as u8);
    state.loot_state = Some(wow_entities::LootState::JustDeactivated);
    state.despawn_at_action = false;
    state.go_anim_progress = 1;
    state.chest_restock_time_secs = Some(30);
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    session.process_pending().await;

    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .unwrap();
    assert_eq!(state.loot_state, Some(wow_entities::LootState::NotReady));
    assert!(state.chest_restock_until.is_some());
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&gameobject_guid)
    );
    assert!(drain_server_opcodes(&send_rx).is_empty());
}
#[tokio::test]
async fn generic_just_deactivated_gameobject_anim_progress_sends_despawn_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    session.set_state(SessionState::LoggedIn);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 249);
    let state = session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default();
    state.go_type = Some(5);
    state.loot_state = Some(wow_entities::LootState::JustDeactivated);
    state.despawn_at_action = false;
    state.go_anim_progress = 1;
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    session.process_pending().await;

    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .unwrap();
    assert_eq!(state.loot_state, Some(wow_entities::LootState::NotReady));
    let opcodes = drain_server_opcodes(&send_rx);
    assert_eq!(opcodes, vec![ServerOpcodes::GameObjectDespawn]);
}
#[tokio::test]
async fn gameobject_override_flags_restore_after_generic_just_deactivated_despawn_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    session.set_state(SessionState::LoggedIn);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 250);
    session.record_represented_gameobject_override_like_cpp(gameobject_guid, 0, 0, true);
    let state = session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default();
    state.go_type = Some(5);
    state.loot_state = Some(wow_entities::LootState::JustDeactivated);
    state.despawn_at_action = false;
    state.go_anim_progress = 1;
    state.gameobject_flags = wow_entities::GO_FLAG_IN_USE;
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    session.process_pending().await;

    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .unwrap();
    assert_eq!(state.gameobject_flags, 0);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::GameObjectDespawn]
    );
}
#[tokio::test]
async fn gameobject_without_override_source_does_not_restore_false_zero_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    session.set_state(SessionState::LoggedIn);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 251);
    session.record_represented_gameobject_override_like_cpp(gameobject_guid, 0, 0, false);
    let state = session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default();
    state.go_type = Some(5);
    state.loot_state = Some(wow_entities::LootState::JustDeactivated);
    state.go_anim_progress = 1;
    state.gameobject_flags = wow_entities::GO_FLAG_IN_USE;
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    session.process_pending().await;

    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .unwrap();
    assert_eq!(state.gameobject_flags, wow_entities::GO_FLAG_IN_USE);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::GameObjectDespawn]
    );
}
#[tokio::test]
async fn represented_gameobject_chest_just_deactivated_without_conditions_sends_no_extra_despawn_like_cpp()
 {
    let (mut session, _pkt_tx, send_rx) = make_session();
    session.set_state(SessionState::LoggedIn);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 247);
    let state = session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default();
    state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_CHEST as u8);
    state.loot_state = Some(wow_entities::LootState::JustDeactivated);
    state.despawn_at_action = false;
    state.go_anim_progress = 0;
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    session.process_pending().await;

    assert!(drain_server_opcodes(&send_rx).is_empty());
}
#[tokio::test]
async fn process_pending_schedules_gameobject_respawn_delay_like_cpp_update() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    session.set_state(SessionState::LoggedIn);
    let spawned_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 145);
    let temporary_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 146);
    for (guid, spawned_by_default) in [(spawned_guid, true), (temporary_guid, false)] {
        let state = session
            .represented_gameobject_use_states
            .entry(guid)
            .or_default();
        state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_TRAP as u8);
        state.loot_state = Some(wow_entities::LootState::JustDeactivated);
        state.respawn_delay_secs = Some(30);
        state.spawned_by_default = Some(spawned_by_default);
        session.client_visible_guids_like_cpp.insert(guid);
    }

    session.process_pending().await;

    let spawned_state = session
        .represented_gameobject_use_states
        .get(&spawned_guid)
        .unwrap();
    assert_eq!(
        spawned_state.loot_state,
        Some(wow_entities::LootState::NotReady)
    );
    assert!(spawned_state.respawn_until.is_some());
    assert!(
        !session
            .client_visible_guids_like_cpp
            .contains(&spawned_guid)
    );

    let temporary_state = session
        .represented_gameobject_use_states
        .get(&temporary_guid)
        .unwrap();
    assert_eq!(
        temporary_state.loot_state,
        Some(wow_entities::LootState::NotReady)
    );
    assert!(temporary_state.respawn_until.is_none());
    assert!(
        !session
            .client_visible_guids_like_cpp
            .contains(&temporary_guid)
    );
    let opcodes = drain_server_opcodes(&send_rx);
    assert_eq!(
        opcodes
            .iter()
            .filter(|opcode| **opcode == ServerOpcodes::GameObjectDespawn)
            .count(),
        2
    );
    assert_eq!(opcodes.len(), 4);

    session
        .represented_gameobject_use_states
        .get_mut(&spawned_guid)
        .unwrap()
        .respawn_until = Some(Instant::now() - Duration::from_secs(1));
    session.process_pending().await;

    let spawned_state = session
        .represented_gameobject_use_states
        .get(&spawned_guid)
        .unwrap();
    assert_eq!(
        spawned_state.loot_state,
        Some(wow_entities::LootState::Ready)
    );
    assert_eq!(spawned_state.go_state, Some(wow_entities::GoState::Ready));
    assert!(spawned_state.respawn_until.is_none());
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&spawned_guid)
    );
}
#[test]
fn gameobject_goober_just_deactivated_resets_and_clears_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let other_player_guid = ObjectGuid::create_player(1, 100);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 15);
    {
        let state = session
            .represented_gameobject_use_states
            .entry(gameobject_guid)
            .or_default();
        state.loot_state = Some(wow_entities::LootState::JustDeactivated);
        state.loot_state_unit_guid = player_guid;
        state.gameobject_flags = wow_entities::GO_FLAG_IN_USE;
        state.go_state = Some(wow_entities::GoState::Active);
        state.unique_users = vec![player_guid, other_player_guid];
        state.cooldown_until = Some(Instant::now() + Duration::from_secs(30));
    }

    assert!(
        session.apply_represented_gameobject_goober_just_deactivated_like_cpp(
            gameobject_guid,
            wow_entities::GooberUseSource {
                lock_id: 12,
                auto_close_ms: 3_000,
                spell_id: 7777,
                linked_trap_entry: 999,
                ..Default::default()
            },
        )
    );

    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .unwrap();
    assert_eq!(state.loot_state, Some(wow_entities::LootState::Ready));
    assert_eq!(state.loot_state_unit_guid, ObjectGuid::EMPTY);
    assert_eq!(state.gameobject_flags & wow_entities::GO_FLAG_IN_USE, 0);
    assert_eq!(state.go_state, Some(wow_entities::GoState::Ready));
    assert!(state.unique_users.is_empty());
    assert!(state.cooldown_until.is_none());
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::GooberLinkedTrapDespawn {
                gameobject_guid,
                trap_entry: 999,
            },
            RepresentedGameObjectUseEffect::GooberUniqueUserSpell {
                gameobject_guid,
                player_guid,
                spell_id: 7777,
            },
            RepresentedGameObjectUseEffect::GooberUniqueUserSpell {
                gameobject_guid,
                player_guid: other_player_guid,
                spell_id: 7777,
            },
            RepresentedGameObjectUseEffect::GooberCleared {
                gameobject_guid,
                loot_state: wow_entities::LootState::Ready,
                go_state: Some(wow_entities::GoState::Ready),
            },
        ]
    );
}
