use super::*;
use std::sync::Arc;

fn recovery_phase(session: &WorldSession) -> Option<wow_entities::PlayerResurrectionPhase> {
    session
        .core
        .with_owned_player_like_cpp(|player| {
            player
                .resurrection_state_like_cpp()
                .delayed_after_teleport
                .map(|continuation| continuation.phase)
        })
        .flatten()
}

fn add_recovery_healer(session: &WorldSession, guid_counter: i64) -> ObjectGuid {
    let position = session.player_position_like_cpp().expect("ghost position");
    insert_healer(
        session,
        guid_counter,
        position,
        NPCFlags1::SPIRIT_HEALER.bits(),
    )
}

fn install_applied_save(
    session: &mut WorldSession,
) -> Arc<crate::session::RecordingPlayerLifecyclePort> {
    let port = crate::session::RecordingPlayerLifecyclePort::new(
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.set_player_lifecycle_port_like_cpp(port.clone());
    port
}

fn add_durability_probe(session: &mut WorldSession, guid_counter: i64) -> ObjectGuid {
    let owner = session.player_guid().expect("canonical Player GUID");
    session.set_player_inventory_slot_count_like_cpp(wow_entities::INVENTORY_DEFAULT_SIZE);
    let guid = ObjectGuid::create_item(1, guid_counter);
    add_top_level_item(
        session,
        owner,
        guid,
        u32::try_from(guid_counter + 1).expect("fixture item entry fits u32"),
        EQUIPMENT_SLOT_CHEST,
        100,
        100,
        ItemUpdateState::Unchanged,
    );
    guid
}

fn install_distinct_recovery_graveyards(
    session: &mut WorldSession,
    corpse_guid: ObjectGuid,
) -> Arc<wow_data::GraveyardStore> {
    let graveyards = super::native_outcome::install_distinct_graveyards(session, corpse_guid);
    let (corpse_map, corpse_position) =
        wow_world_entities::player_corpse_location_like_cpp(session)
            .expect("cached corpse location");
    let corpse_grave = wow_world_entities::closest_graveyard_like_cpp(
        session,
        graveyards.as_ref(),
        corpse_map,
        corpse_position,
    );
    let ghost_position = session.player_position_like_cpp().expect("ghost position");
    let ghost_grave = wow_world_entities::closest_graveyard_like_cpp(
        session,
        graveyards.as_ref(),
        571,
        ghost_position,
    );
    assert_eq!(corpse_grave, Some(5));
    assert_eq!(
        ghost_grave,
        Some(wow_data::ALLIANCE_GRAVEYARD_SAFE_LOC_ID_LIKE_CPP)
    );
    assert_ne!(corpse_grave, ghost_grave);
    graveyards
}

#[tokio::test]
async fn recovery_fences_healer_failed_delete_keeps_native_owner_and_skips_save_teleport() {
    let (mut session, send_rx, _realm_rx, corpse_port) = logged_in_released_ghost(43_231).await;
    let _packet_peer = keep_recovery_input_open(&mut session);
    let healer = add_recovery_healer(&session, 43_232);
    let player_guid = session.player_guid().unwrap();
    let corpse_guid = corpse::current_corpse_guid_like_cpp(&session, player_guid).unwrap();
    let item_guid = add_durability_probe(&mut session, 43_239);
    let _graveyards = install_distinct_recovery_graveyards(&mut session, corpse_guid);
    corpse_port.fail_next_delete();
    let save_port = install_applied_save(&mut session);

    session
        .handle_spirit_healer_activate(healer_packet(healer))
        .await;

    assert_eq!(session.state(), crate::session::SessionState::Disconnecting);
    assert_eq!(corpse_port.delete_requests(), vec![43_231]);
    assert_eq!(
        corpse::current_corpse_guid_like_cpp(&session, player_guid),
        None
    );
    assert_eq!(save_port.character_save_count(), 0);
    assert_eq!(item_durability(&session, item_guid), 75);
    assert_eq!(
        recovery_phase(&session),
        Some(wow_entities::PlayerResurrectionPhase::RecoveryCorpseBonesInFlight)
    );
    assert!(session.core.player_handle_like_cpp.is_some());
    let opcodes = drain_packets(&send_rx)
        .iter()
        .map(|packet| packet_opcode(packet))
        .collect::<Vec<_>>();
    assert!(!opcodes.contains(&(ServerOpcodes::MoveTeleport as u16)));
}

#[tokio::test]
async fn recovery_fences_healer_cancelled_delete_is_held_and_never_reentered() {
    let (mut session, _send_rx, _realm_rx, corpse_port) = logged_in_released_ghost(43_233).await;
    let _packet_peer = keep_recovery_input_open(&mut session);
    let healer = add_recovery_healer(&session, 43_234);
    let player_guid = session.player_guid().unwrap();
    let corpse_guid = corpse::current_corpse_guid_like_cpp(&session, player_guid).unwrap();
    let item_guid = add_durability_probe(&mut session, 43_240);
    let _graveyards = install_distinct_recovery_graveyards(&mut session, corpse_guid);
    corpse_port.hold_delete();
    let save_port = install_applied_save(&mut session);
    let mut activation = Box::pin(session.handle_spirit_healer_activate(healer_packet(healer)));
    tokio::select! {
        result = tokio::time::timeout(std::time::Duration::from_secs(5), corpse_port.wait_for_delete()) => {
            assert!(result.is_ok(), "spirit healer did not reach corpse deletion");
        }
        _ = &mut activation => panic!("spirit healer returned before the held delete resolved"),
    }
    drop(activation);

    assert_eq!(corpse_port.delete_requests(), vec![43_233]);
    assert_eq!(
        corpse::current_corpse_guid_like_cpp(&session, player_guid),
        None
    );
    assert_eq!(save_port.character_save_count(), 0);
    assert_eq!(item_durability(&session, item_guid), 75);
    assert_eq!(
        recovery_phase(&session),
        Some(wow_entities::PlayerResurrectionPhase::RecoveryCorpseBonesInFlight)
    );
    session
        .handle_spirit_healer_activate(healer_packet(healer))
        .await;
    assert_eq!(item_durability(&session, item_guid), 75);
    assert_eq!(corpse_port.delete_requests(), vec![43_233]);
    assert_eq!(save_port.character_save_count(), 0);
    assert_eq!(
        recovery_phase(&session),
        Some(wow_entities::PlayerResurrectionPhase::RecoveryCorpseBonesInFlight)
    );
}

#[tokio::test]
async fn recovery_fences_healer_cancelled_save_keeps_durability_and_owner_once() {
    let (mut session, send_rx, _realm_rx, corpse_port) = logged_in_released_ghost(43_251).await;
    let _packet_peer = keep_recovery_input_open(&mut session);
    let healer = add_recovery_healer(&session, 43_252);
    let player_guid = session.player_guid().unwrap();
    let corpse_guid = corpse::current_corpse_guid_like_cpp(&session, player_guid).unwrap();
    let item_guid = add_durability_probe(&mut session, 43_253);
    let _graveyards = install_distinct_recovery_graveyards(&mut session, corpse_guid);
    let save_port = crate::session::RecordingPlayerLifecyclePort::new(
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    save_port.hold_character_save();
    session.set_player_lifecycle_port_like_cpp(save_port.clone());
    let handle = session.core.player_handle_like_cpp.unwrap();
    let manager = session.core.canonical_map_manager.as_ref().unwrap().clone();

    let mut activation = Box::pin(session.handle_spirit_healer_activate(healer_packet(healer)));
    tokio::select! {
        result = tokio::time::timeout(std::time::Duration::from_secs(5), save_port.wait_for_character_save()) => {
            assert!(result.is_ok(), "the healer did not reach its held save");
        }
        _ = &mut activation => panic!("the healer returned before its held save resolved"),
    }
    assert_eq!(
        manager
            .lock()
            .unwrap()
            .with_player_like_cpp(handle, |player| {
                player
                    .resurrection_state_like_cpp()
                    .delayed_after_teleport
                    .map(|continuation| continuation.phase)
            })
            .unwrap(),
        Some(wow_entities::PlayerResurrectionPhase::SavingPlayer)
    );
    drop(activation);

    assert_eq!(corpse_port.delete_requests(), vec![43_251]);
    assert_eq!(
        corpse::current_corpse_guid_like_cpp(&session, player_guid),
        None
    );
    assert_eq!(item_durability(&session, item_guid), 75);
    assert_eq!(save_port.character_save_count(), 1);
    session
        .handle_spirit_healer_activate(healer_packet(healer))
        .await;
    assert_eq!(corpse_port.delete_requests(), vec![43_251]);
    assert_eq!(item_durability(&session, item_guid), 75);
    assert_eq!(save_port.character_save_count(), 1);
    assert_eq!(
        recovery_phase(&session),
        Some(wow_entities::PlayerResurrectionPhase::SavingPlayer)
    );
    assert!(
        !drain_packets(&send_rx)
            .iter()
            .any(|packet| { packet_opcode(packet) == ServerOpcodes::MoveTeleport as u16 })
    );
}

#[tokio::test]
async fn recovery_fences_healer_save_failed_or_unknown_blocks_grave_teleport() {
    for outcome in [
        wow_persistence::PersistenceOutcomeLikeCpp::Failed {
            reason: "healer save failed".into(),
        },
        wow_persistence::PersistenceOutcomeLikeCpp::Unknown {
            reason: "healer save result unknown".into(),
        },
    ] {
        let (mut session, send_rx, _realm_rx, corpse_port) = logged_in_released_ghost(43_235).await;
        let _packet_peer = keep_recovery_input_open(&mut session);
        let healer = add_recovery_healer(&session, 43_236);
        let player_guid = session.player_guid().unwrap();
        let corpse_guid = corpse::current_corpse_guid_like_cpp(&session, player_guid).unwrap();
        let item_guid = add_durability_probe(&mut session, 43_243);
        let _graveyards = install_distinct_recovery_graveyards(&mut session, corpse_guid);
        let save_port = crate::session::RecordingPlayerLifecyclePort::new(outcome);
        session.set_player_lifecycle_port_like_cpp(save_port.clone());

        session
            .handle_spirit_healer_activate(healer_packet(healer))
            .await;

        assert_eq!(corpse_port.delete_requests(), vec![43_235]);
        assert_eq!(
            corpse::current_corpse_guid_like_cpp(&session, player_guid),
            None
        );
        assert_eq!(save_port.character_save_count(), 1);
        assert_eq!(item_durability(&session, item_guid), 75);
        assert_eq!(
            recovery_phase(&session),
            Some(wow_entities::PlayerResurrectionPhase::SavingPlayer)
        );
        assert_eq!(session.state(), crate::session::SessionState::Disconnecting);
        let opcodes = drain_packets(&send_rx)
            .iter()
            .map(|packet| packet_opcode(packet))
            .collect::<Vec<_>>();
        assert!(!opcodes.contains(&(ServerOpcodes::MoveTeleport as u16)));
        session
            .handle_spirit_healer_activate(healer_packet(healer))
            .await;
        assert_eq!(item_durability(&session, item_guid), 75);
        assert_eq!(corpse_port.delete_requests(), vec![43_235]);
        assert_eq!(save_port.character_save_count(), 1);
        assert_eq!(
            recovery_phase(&session),
            Some(wow_entities::PlayerResurrectionPhase::SavingPlayer)
        );
    }
}

#[tokio::test]
async fn recovery_fences_healer_no_corpse_keeps_source_no_save_behavior() {
    let (mut session, _send_rx, _realm_rx, corpse_port) = logged_in_released_ghost(43_237).await;
    let _packet_peer = keep_recovery_input_open(&mut session);
    let healer = add_recovery_healer(&session, 43_238);
    let item_guid = add_durability_probe(&mut session, 43_244);
    let generators = session.id_generators_for_test_like_cpp();
    assert!(
        wow_world_entities::spawn_corpse_bones_like_cpp(
            &mut session,
            generators.item.as_ref(),
            false,
        )
        .await
    );
    let deletes_before = corpse_port.delete_requests();
    let save_port = install_applied_save(&mut session);

    session
        .handle_spirit_healer_activate(healer_packet(healer))
        .await;

    assert!(session.player_is_alive_like_cpp());
    assert_eq!(corpse_port.delete_requests(), deletes_before);
    assert_eq!(save_port.character_save_count(), 0);
    assert_eq!(item_durability(&session, item_guid), 75);
    assert_eq!(recovery_phase(&session), None);
}

#[tokio::test]
async fn recovery_fences_healer_confirmed_save_allows_cached_distinct_grave_teleport() {
    let (mut session, send_rx, _realm_rx, corpse_port) = logged_in_released_ghost(43_245).await;
    let _packet_peer = keep_recovery_input_open(&mut session);
    let healer = add_recovery_healer(&session, 43_246);
    let player_guid = session.player_guid().unwrap();
    let corpse_guid = corpse::current_corpse_guid_like_cpp(&session, player_guid).unwrap();
    let item_guid = add_durability_probe(&mut session, 43_247);
    let _graveyards = install_distinct_recovery_graveyards(&mut session, corpse_guid);
    let save_port = install_applied_save(&mut session);

    session
        .handle_spirit_healer_activate(healer_packet(healer))
        .await;

    assert_eq!(corpse_port.delete_requests(), vec![43_245]);
    assert_eq!(save_port.character_save_count(), 1);
    assert_eq!(item_durability(&session, item_guid), 75);
    assert_eq!(recovery_phase(&session), None);
    assert!(
        drain_packets(&send_rx)
            .iter()
            .any(|packet| packet_opcode(packet) == ServerOpcodes::MoveTeleport as u16),
        "a confirmed save permits the cached, reachable corpse-graveyard teleport"
    );
}

#[tokio::test]
async fn recovery_fences_healer_deferred_save_resumes_once_with_cached_grave_and_durability() {
    let (mut session, send_rx, _realm_rx, corpse_port) = logged_in_released_ghost(43_248).await;
    let _packet_peer = keep_recovery_input_open(&mut session);
    let healer = add_recovery_healer(&session, 43_249);
    let player_guid = session.player_guid().unwrap();
    let corpse_guid = corpse::current_corpse_guid_like_cpp(&session, player_guid).unwrap();
    let item_guid = add_durability_probe(&mut session, 43_250);
    let _graveyards = install_distinct_recovery_graveyards(&mut session, corpse_guid);
    let save_port = install_applied_save(&mut session);
    let old_destination = session.player_position_like_cpp().unwrap();
    assert!(session.set_pending_teleport_like_cpp(Some((571, old_destination))));
    assert!(session.set_represented_far_teleport_pending_like_cpp(true));

    session
        .handle_spirit_healer_activate(healer_packet(healer))
        .await;

    assert_eq!(corpse_port.delete_requests(), vec![43_248]);
    assert_eq!(save_port.character_save_count(), 0);
    assert_eq!(item_durability(&session, item_guid), 75);
    assert_eq!(
        recovery_phase(&session),
        Some(wow_entities::PlayerResurrectionPhase::PlayerSaveDeferred)
    );
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| {
            player
                .resurrection_state_like_cpp()
                .delayed_after_teleport
                .map(|continuation| continuation.operation)
        }),
        Some(Some(
            wow_entities::PlayerResurrectionOperation::SpiritHealer {
                corpse_graveyard: Some(5),
            }
        ))
    );

    let _activation_packets = drain_packets(&send_rx);

    let ghost_position = session.player_position_like_cpp().unwrap();
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player.resurrection_state_mut_like_cpp().corpse_location = Some((571, ghost_position));
        })
        .unwrap();
    crate::session::install_worldport_ack_inputs(&mut session);
    session.set_state(crate::session::SessionState::Transfer);
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    let catalogs = session.session_handler_catalogs_for_test_like_cpp();
    let response = crate::session::registry::get_handler(ClientOpcodes::WorldPortResponse)
        .expect("registered WorldPortResponse handler");

    let capture_finished = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let capture_finished_on_thread = std::sync::Arc::clone(&capture_finished);
    let packet_capture = std::thread::spawn(move || {
        let mut packets = Vec::new();
        while !capture_finished_on_thread.load(std::sync::atomic::Ordering::Acquire) {
            match send_rx.recv_timeout(std::time::Duration::from_millis(10)) {
                Ok(packet) => packets.push(packet),
                Err(flume::RecvTimeoutError::Timeout) => {}
                Err(flume::RecvTimeoutError::Disconnected) => break,
            }
        }
        packets.extend(drain_packets(&send_rx));
        packets
    });
    (response.handler)(&mut session, &catalogs, WorldPacket::new_empty()).await;
    capture_finished.store(true, std::sync::atomic::Ordering::Release);
    let acknowledged_packets = packet_capture
        .join()
        .expect("registered ACK packet collector thread");

    assert_eq!(save_port.character_save_count(), 1);
    assert_eq!(corpse_port.delete_requests(), vec![43_248]);
    assert_eq!(item_durability(&session, item_guid), 75);
    assert_eq!(recovery_phase(&session), None);
    assert_eq!(session.state(), crate::session::SessionState::LoggedIn);
    let teleport = crate::session::hub_ref(&session)
        .player_teleport_state_snapshot_like_cpp()
        .expect("canonical teleport state after healer resumption");
    assert!(teleport.near_pending);
    assert!(!teleport.far_pending);
    assert!(
        acknowledged_packets
            .iter()
            .any(|packet| packet_opcode(packet) == ServerOpcodes::MoveTeleport as u16),
        "confirmed healer save issues its post-save teleport during the registered ACK"
    );
}
