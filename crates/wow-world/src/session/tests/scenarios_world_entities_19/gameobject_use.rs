//! GameObject-use session scenarios.

use super::*;

#[test]
fn gameobject_goober_just_deactivated_consumable_stays_not_ready_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let same_map_guid = ObjectGuid::create_player(1, 77);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 16);
    let linked_trap_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 19);
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
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default()
        .loot_state = Some(wow_entities::LootState::JustDeactivated);
    session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default()
        .linked_trap_guid = Some(linked_trap_guid);
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

    assert!(
        session.apply_represented_gameobject_goober_just_deactivated_like_cpp(
            gameobject_guid,
            wow_entities::GooberUseSource {
                consumable: true,
                linked_trap_entry: 999,
                ..Default::default()
            },
        )
    );

    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .unwrap();
    assert_eq!(state.loot_state, Some(wow_entities::LootState::NotReady));
    assert_eq!(state.go_state, None);
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
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::GooberLinkedTrapDespawn {
                gameobject_guid,
                trap_entry: 999,
            },
            RepresentedGameObjectUseEffect::GooberCleared {
                gameobject_guid,
                loot_state: wow_entities::LootState::NotReady,
                go_state: None,
            },
        ]
    );
    let opcodes = drain_server_opcodes(&send_rx);
    assert_eq!(
        opcodes
            .iter()
            .filter(|opcode| **opcode == ServerOpcodes::GameObjectDespawn)
            .count(),
        2
    );
    assert_eq!(opcodes.len(), 3);
    let linked_command = match same_command_rx.try_recv() {
        Ok(SessionCommand::SendIfVisibleLikeCpp(command)) => command,
        other => {
            panic!("expected represented linked trap despawn fanout command, got {other:?}")
        }
    };
    assert_eq!(linked_command.source_guid, linked_trap_guid);
    assert_eq!(linked_command.map_id, 571);
    assert_eq!(linked_command.instance_id, 0);
    let command = match same_command_rx.try_recv() {
        Ok(SessionCommand::SendIfVisibleLikeCpp(command)) => command,
        other => panic!("expected represented goober despawn fanout command, got {other:?}"),
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
async fn gameobject_goober_just_deactivated_non_consumable_anim_progress_sends_no_despawn_like_cpp()
{
    let (mut session, _pkt_tx, send_rx) = make_session();
    session.set_state(SessionState::LoggedIn);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 18);
    let state = session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default();
    state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_GOOBER as u8);
    state.loot_state = Some(wow_entities::LootState::JustDeactivated);
    state.go_anim_progress = 1;
    state.goober_use_source = Some(wow_entities::GooberUseSource {
        consumable: false,
        spell_id: 7777,
        ..Default::default()
    });
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    session.process_pending().await;

    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .unwrap();
    assert_eq!(state.loot_state, Some(wow_entities::LootState::Ready));
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&gameobject_guid)
    );
    assert!(drain_server_opcodes(&send_rx).is_empty());
}
