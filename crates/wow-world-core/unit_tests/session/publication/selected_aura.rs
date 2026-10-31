//! Prepared selected updates retain their source binding and observer decision.

use super::*;
use crate::session::directory::{
    PlayerDirectoryIdentityLikeCpp, PlayerDirectoryPlacementLikeCpp,
    PlayerSessionRegistrationLikeCpp,
};
use crate::session::mailbox::SessionCommand;
use crate::session::{SessionCore, SessionState};
use std::sync::{Arc, Mutex};
use wow_core::Position;
use wow_entities::Player;
use wow_packet::ServerPacket;

const MAP: MapKey = MapKey::new(571, 0);
type Manager = Arc<Mutex<MapManager>>;

struct Endpoint {
    core: SessionCore,
    output: flume::Receiver<Vec<u8>>,
    realm_output: flume::Receiver<Vec<u8>>,
    _input: flume::Sender<wow_packet::WorldPacket>,
}

fn add_player(manager: &mut MapManager, guid: ObjectGuid, position: Position) -> PlayerHandle {
    let mut player = Box::new(Player::new(Some(1), false));
    player.unit_mut().world_mut().object_mut().create(guid);
    let handle = manager.install_detached_player_like_cpp(player).unwrap();
    manager
        .attach_player_like_cpp(handle, MAP, position)
        .unwrap();
    handle
}

fn endpoint(manager: &Manager, registry: &Arc<PlayerRegistry>, counter: i64) -> Endpoint {
    let guid = ObjectGuid::create_player(1, counter);
    let handle = add_player(&mut manager.lock().unwrap(), guid, Position::ZERO);
    let (input, packet_rx) = flume::bounded(8);
    let (send_tx, output) = flume::unbounded();
    let (realm_send_tx, realm_output) = flume::unbounded();
    let (command_tx, command_rx) = flume::bounded(8);
    let connection = wow_session::SessionConnection::new(send_tx.clone(), packet_rx);
    let mut core = SessionCore::new_for_world_session_like_cpp(
        1,
        "SelectedAura".into(),
        0,
        2,
        2,
        54261,
        "enUS".into(),
        command_tx.clone(),
        command_rx,
        connection,
        Vec::new(),
    );
    core.player_guid = Some(guid);
    core.player_handle_like_cpp = Some(handle);
    core.current_map_id = 571;
    core.state = SessionState::LoggedIn;
    core.canonical_map_manager = Some(Arc::clone(manager));
    core.player_registry = Some(Arc::clone(registry));
    core.install_realm_send_channel_for_test(realm_send_tx.clone());
    registry.register_or_replace(
        guid,
        PlayerSessionRegistrationLikeCpp {
            identity: PlayerDirectoryIdentityLikeCpp::new_with_bnet(
                format!("SelectedAura{counter}"),
                1,
                1,
                0,
                1,
                1,
                0,
                2,
                "enUS",
                Some(handle),
            ),
            placement: PlayerDirectoryPlacementLikeCpp {
                map_id: 571,
                instance_id: 0,
                position: Position::ZERO,
                is_in_world: true,
                level: 80,
                is_alive: true,
            },
            active_loot_rolls: Vec::new(),
            realm_send_tx,
            send_tx,
            command_tx,
            session_phase_tx: crate::session::directory::detached_session_phase_rail_like_cpp(),
            durable_creature_runtime_commands_like_cpp: Arc::clone(
                &core.durable_creature_runtime_commands_like_cpp,
            ),
            client_visible_guids_like_cpp: core.client_visible_guids_like_cpp.clone(),
            client_visible_transports_like_cpp: Default::default(),
            advanced_combat_logging_enabled_like_cpp: Default::default(),
            visibility_refresh_pending_like_cpp: Default::default(),
        },
        Default::default(),
    );
    Endpoint {
        core,
        output,
        realm_output,
        _input: input,
    }
}

fn fixture() -> (Manager, Arc<PlayerRegistry>, Endpoint, Endpoint) {
    let manager = Arc::new(Mutex::new(MapManager::new(wow_map::MIN_GRID_DELAY_MS, 10)));
    manager
        .lock()
        .unwrap()
        .create_world_map(MAP.map_id, MAP.instance_id);
    let registry = Arc::new(PlayerRegistry::new());
    assert!(registry.bind_canonical_map_manager(Arc::clone(&manager)));
    let source = endpoint(&manager, &registry, 84_300_001);
    let observer = endpoint(&manager, &registry, 84_300_002);
    observer
        .core
        .client_visible_guids_like_cpp
        .insert(source.core.player_guid().unwrap());
    (manager, registry, source, observer)
}

fn prepare(manager: &Manager, source: &Endpoint) -> PreparedSelectedAuraUpdate {
    let packet = wow_packet::packets::misc::AuraUpdate {
        unit_guid: source.core.player_guid().unwrap(),
        update_all: false,
        auras: vec![wow_packet::packets::misc::AuraInfoLikeCpp {
            slot: 7,
            aura_data: None,
        }],
    };
    prepare_selected_aura_update(
        &manager.lock().unwrap(),
        source.core.player_handle_like_cpp.unwrap(),
        packet.to_bytes(),
    )
    .unwrap()
}

fn take_commands(endpoint: &Endpoint) -> Vec<SessionCommand> {
    endpoint
        .core
        .durable_creature_runtime_commands_like_cpp
        .lock()
        .unwrap()
        .drain_like_cpp()
}

#[test]
fn selected_aura_self_delivery_is_direct_and_observers_use_the_committed_durable_rail() {
    let (manager, _registry, source, observer) = fixture();
    let prepared = prepare(&manager, &source);
    assert!(
        !source
            .core
            .client_visible_guids_like_cpp
            .contains(&source.core.player_guid().unwrap())
    );
    assert!(
        source
            .core
            .packet_publication_access_like_cpp()
            .publish_prepared_selected_aura_update(&prepared)
    );
    assert_eq!(source.output.try_recv().unwrap(), prepared.packet_bytes());
    assert!(
        take_commands(&source).is_empty(),
        "self delivery needs no World rail drain"
    );
    assert!(observer.output.is_empty());
    assert!(source.core.session_command_rx.is_empty());
    assert!(observer.core.session_command_rx.is_empty());
    let commands = take_commands(&observer);
    assert_eq!(commands.len(), 1);
    let SessionCommand::SendPlayerAuraUpdate(command) = &commands[0] else {
        panic!("observer update must use the established selected aura envelope");
    };
    assert_eq!(
        command.recipient_handle,
        observer.core.player_handle_like_cpp.unwrap()
    );
    assert_eq!(command.map_key, MAP);
    assert_eq!(
        Some((command.map_key, command.residence_revision)),
        manager
            .lock()
            .unwrap()
            .player_active_residence_revision_like_cpp(command.recipient_handle)
    );
    assert!(
        command
            .committed_visibility
            .contains(&source.core.player_guid().unwrap())
    );
    assert_eq!(command.packet_bytes, prepared.packet_bytes());
}

#[test]
fn realm_player_visible_packet_uses_frozen_candidates_and_realm_channels() {
    let (manager, registry, source, observer) = fixture();
    let hidden = endpoint(&manager, &registry, 84_300_005);
    let packet = wow_packet::packets::misc::CrossedInebriationThreshold {
        guid: source.core.player_guid().unwrap(),
        threshold: 2,
        item_id: 0,
    }
    .to_bytes();
    let prepared = prepare_player_visible_packet(
        &manager.lock().unwrap(),
        source.core.player_handle_like_cpp.unwrap(),
        packet.clone(),
        true,
    )
    .unwrap();

    assert!(
        source
            .core
            .packet_publication_access_like_cpp()
            .publish_prepared_player_visible_packet(&prepared)
    );
    assert_eq!(source.realm_output.try_recv().unwrap(), packet);
    assert_eq!(observer.realm_output.try_recv().unwrap(), packet);
    assert!(source.output.is_empty());
    assert!(observer.output.is_empty());
    assert!(
        hidden.realm_output.is_empty(),
        "HaveAtClient gates observer delivery"
    );
    assert!(
        take_commands(&observer).is_empty(),
        "realm bytes bypass AuraUpdate commands"
    );
}

#[test]
fn realm_observer_delivery_rejects_a_stale_frozen_residence_revision() {
    let (manager, registry, source, observer) = fixture();
    let packet = wow_packet::packets::misc::CrossedInebriationThreshold {
        guid: source.core.player_guid().unwrap(),
        threshold: 2,
        item_id: 0,
    }
    .to_bytes();
    let prepared = prepare_player_visible_packet(
        &manager.lock().unwrap(),
        source.core.player_handle_like_cpp.unwrap(),
        packet.clone(),
        true,
    )
    .unwrap();
    let observer_handle = observer.core.player_handle_like_cpp.unwrap();
    {
        let mut manager = manager.lock().unwrap();
        manager.detach_player_like_cpp(observer_handle).unwrap();
        manager
            .attach_player_like_cpp(observer_handle, MAP, Position::ZERO)
            .unwrap();
    }

    assert!(
        source
            .core
            .packet_publication_access_like_cpp()
            .publish_prepared_player_visible_packet(&prepared)
    );
    assert_eq!(source.realm_output.try_recv().unwrap(), packet);
    assert!(observer.realm_output.is_empty());
    assert!(observer.output.is_empty());
    assert!(take_commands(&observer).is_empty());
    let _ = registry;
}

#[test]
fn selected_aura_delivery_uses_frozen_candidates_and_publication_time_have_at_client() {
    let (manager, registry, source, observer) = fixture();
    let hidden = endpoint(&manager, &registry, 84_300_003);
    let prepared = prepare(&manager, &source);
    manager
        .lock()
        .unwrap()
        .with_player_mut_like_cpp(observer.core.player_handle_like_cpp.unwrap(), |player| {
            player
                .unit_mut()
                .world_mut()
                .relocate(Position::xyz(10_000.0, 10_000.0, 0.0));
            player.unit_mut().world_mut().phase_shift_mut().insert(99);
        })
        .unwrap();
    let later = endpoint(&manager, &registry, 84_300_004);
    later
        .core
        .client_visible_guids_like_cpp
        .insert(source.core.player_guid().unwrap());

    assert!(
        source
            .core
            .packet_publication_access_like_cpp()
            .publish_prepared_selected_aura_update(&prepared)
    );
    assert_eq!(source.output.try_recv().unwrap(), prepared.packet_bytes());
    assert_eq!(
        take_commands(&observer).len(),
        1,
        "range and phase are frozen at preparation"
    );
    assert!(
        take_commands(&hidden).is_empty(),
        "HaveAtClient still gates observer handoff"
    );
    assert!(
        take_commands(&later).is_empty(),
        "new candidates are not selected after preparation"
    );
}

#[test]
fn selected_aura_stale_source_identity_or_residence_sends_nothing() {
    for mode in 0..8 {
        let (manager, _registry, mut source, observer) = fixture();
        let prepared = prepare(&manager, &source);
        let handle = source.core.player_handle_like_cpp.unwrap();
        match mode {
            0 => source.core.player_guid = Some(ObjectGuid::create_player(1, 84_300_099)),
            1 => source.core.player_handle_like_cpp = observer.core.player_handle_like_cpp,
            2 => {
                let mut manager = manager.lock().unwrap();
                manager.detach_player_like_cpp(handle).unwrap();
                manager
                    .attach_player_like_cpp(handle, MAP, Position::ZERO)
                    .unwrap();
            }
            3 => {
                add_player(&mut manager.lock().unwrap(), handle.guid(), Position::ZERO);
            }
            _ => {
                manager
                    .lock()
                    .unwrap()
                    .with_player_mut_like_cpp(handle, |player| {
                        if mode == 4 {
                            player
                                .unit_mut()
                                .world_mut()
                                .object_mut()
                                .remove_from_world();
                        } else if mode == 5 || mode == 6 {
                            let world = player.unit_mut().world_mut();
                            world.object_mut().remove_from_world();
                            world.reset_map().unwrap();
                            if mode == 5 {
                                world.set_map(1, 0).unwrap();
                            }
                            world.object_mut().add_to_world();
                        } else {
                            player
                                .unit_mut()
                                .world_mut()
                                .object_mut()
                                .create(ObjectGuid::create_player(1, 84_300_099));
                        }
                    })
                    .unwrap();
            }
        }
        assert!(
            !source
                .core
                .packet_publication_access_like_cpp()
                .publish_prepared_selected_aura_update(&prepared)
        );
        assert!(source.output.is_empty());
        assert!(observer.output.is_empty());
        assert!(take_commands(&source).is_empty());
        assert!(take_commands(&observer).is_empty());
    }
}

#[test]
fn selected_aura_valid_detached_source_keeps_direct_self_only_publication() {
    let (manager, _registry, source, observer) = fixture();
    let handle = source.core.player_handle_like_cpp.unwrap();
    manager
        .lock()
        .unwrap()
        .detach_player_like_cpp(handle)
        .unwrap();
    let prepared = prepare(&manager, &source);
    assert!(
        source
            .core
            .packet_publication_access_like_cpp()
            .publish_prepared_selected_aura_update(&prepared)
    );
    assert_eq!(source.output.try_recv().unwrap(), prepared.packet_bytes());
    assert!(take_commands(&observer).is_empty());
    assert!(
        source
            .core
            .packet_publication_access_like_cpp()
            .publish_selected_aura_update(handle.guid(), prepared.packet_bytes())
    );
    assert_eq!(source.output.try_recv().unwrap(), prepared.packet_bytes());
}
