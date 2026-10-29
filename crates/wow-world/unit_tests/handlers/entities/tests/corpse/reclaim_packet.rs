use super::*;

#[tokio::test]
async fn reclaim_corpse_dispatch_ignores_captured_guid_hint_but_requires_native_range() {
    for (counter, body) in [
        (4_491, &[0x00, 0x00][..]),
        (4_492, &[0x01, 0xa0, 0x14, 0x04, 0x08][..]),
    ] {
        let (mut session, send_rx, realm_rx, port) =
            released_ghost_without_reclaim_delay_like_cpp(counter).await;
        let player_guid = session.player_guid().unwrap();
        let owner = session.core.player_handle_like_cpp.expect("native owner");
        let corpse_guid = current_corpse_guid_like_cpp(&session, player_guid).unwrap();
        let catalogs = session.session_handler_catalogs_for_test_like_cpp();
        session.set_state(crate::session::SessionState::LoggedIn);

        // C++ MiscHandler.cpp:416 ignores the packet hint, but its native
        // corpse owner, reclaim delay and distance checks remain authoritative.
        session.set_player_position_like_cpp(Position::new(61.0, 2.0, 3.0, 0.0));
        let mut packet = WorldPacket::new_empty();
        packet.write_uint16(ClientOpcodes::ReclaimCorpse as u16);
        packet.write_bytes(body);
        session.dispatch_packet(&catalogs, packet.clone()).await;
        assert!(!session.player_is_alive_like_cpp());
        assert!(crate::session::hub_ref(&session).player_has_ghost_flag_like_cpp());
        assert_eq!(
            current_corpse_guid_like_cpp(&session, player_guid),
            Some(corpse_guid)
        );
        assert!(port.deletes.lock().unwrap().is_empty());
        assert!(sent_opcodes(&send_rx).is_empty());
        assert!(sent_opcodes(&realm_rx).is_empty());

        session
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player
                    .unit_mut()
                    .set_power_index(wow_constants::PowerType::Mana, Some(0));
                player
                    .unit_mut()
                    .set_max_power(wow_constants::PowerType::Mana, 26_910);
                player
                    .unit_mut()
                    .set_power(wow_constants::PowerType::Mana, 0);
                player
                    .unit_mut()
                    .set_power_index(wow_constants::PowerType::Rage, Some(1));
                player
                    .unit_mut()
                    .set_max_power(wow_constants::PowerType::Rage, 100);
                player
                    .unit_mut()
                    .set_power(wow_constants::PowerType::Rage, 0);
                player
                    .unit_mut()
                    .set_power_index(wow_constants::PowerType::Energy, None);
                player
                    .unit_mut()
                    .set_power_index(wow_constants::PowerType::Focus, None);
            })
            .unwrap();
        let canonical = Arc::clone(session.core.canonical_map_manager.as_ref().unwrap());
        let peer_guid = ObjectGuid::create_player(1, counter + 10);
        add_canonical_test_player_on_map_for_misc_test(
            &canonical,
            peer_guid,
            Position::new(10.0, 2.0, 3.0, 0.0),
            571,
            0,
        );
        canonical
            .lock()
            .unwrap()
            .adopt_active_player_like_cpp(peer_guid)
            .unwrap();
        let registry = Arc::new(wow_world_core::player_directory::PlayerRegistry::new());
        assert!(registry.bind_canonical_map_manager(Arc::clone(&canonical)));
        let (peer_tx, peer_rx) = flume::unbounded();
        let mut registration = broadcast_info_with_command_tx(peer_tx);
        registration.placement.position = Position::new(10.0, 2.0, 3.0, 0.0);
        registration
            .client_visible_guids_like_cpp
            .insert(player_guid);
        registry.register_or_replace(peer_guid, registration, Default::default());
        session.set_player_registry(registry);
        session.set_player_position_like_cpp(Position::new(10.0, 2.0, 3.0, 0.0));
        session.dispatch_packet(&catalogs, packet.clone()).await;
        assert!(session.player_is_alive_like_cpp());
        assert!(!crate::session::hub_ref(&session).player_has_ghost_flag_like_cpp());
        assert_eq!(
            session.canonical_player_health_snapshot_like_cpp(),
            Some((500, 1_000))
        );
        assert_eq!(session.core.player_handle_like_cpp, Some(owner));
        assert_eq!(port.delete_requests(), vec![counter as u64]);
        assert_eq!(current_corpse_guid_like_cpp(&session, player_guid), None);
        let realm_packets = realm_rx.try_iter().collect::<Vec<_>>();
        assert_eq!(realm_packets.len(), 3);
        assert_eq!(
            u16::from_le_bytes([realm_packets[0][0], realm_packets[0][1]]),
            ServerOpcodes::DeathReleaseLoc as u16
        );
        let expected_power = [
            (wow_constants::PowerType::Mana, 13_455),
            (wow_constants::PowerType::Rage, 0),
        ]
        .map(|(power, value)| {
            wow_packet::packets::combat::PowerUpdate {
                guid: player_guid,
                powers: vec![(value, power as u8)],
            }
            .to_bytes()
        });
        assert_eq!(&realm_packets[1..], &expected_power);
        let peer_power = peer_rx
            .try_iter()
            .filter_map(|command| match command {
                crate::session::mailbox::SessionCommand::SendRealmIfVisibleLikeCpp(command) => {
                    assert_eq!(command.source_guid, player_guid);
                    Some(command.packet_bytes)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(peer_power, expected_power);
        let movement = sent_opcodes(&send_rx);
        let land = movement
            .iter()
            .position(|opcode| *opcode == ServerOpcodes::MoveSetLandWalk as u16)
            .unwrap();
        let unroot = movement
            .iter()
            .position(|opcode| *opcode == ServerOpcodes::MoveUnroot as u16)
            .unwrap();
        assert!(land < unroot);

        session.dispatch_packet(&catalogs, packet).await;
        assert_eq!(port.delete_requests(), vec![counter as u64]);
        assert!(sent_opcodes(&send_rx).is_empty());
        assert!(sent_opcodes(&realm_rx).is_empty());
        assert!(peer_rx.try_iter().all(|command| !matches!(
            command,
            crate::session::mailbox::SessionCommand::SendRealmIfVisibleLikeCpp(_)
        )));
    }
}
