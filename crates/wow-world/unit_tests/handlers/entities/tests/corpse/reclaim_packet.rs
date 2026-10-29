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
        assert_eq!(
            sent_opcodes(&realm_rx),
            vec![ServerOpcodes::DeathReleaseLoc as u16]
        );
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
    }
}
