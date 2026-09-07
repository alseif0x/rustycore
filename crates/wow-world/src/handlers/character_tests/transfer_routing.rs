//! Live #585 regression: TransferPending/NewWorld are realm, SuspendToken instance.
//! C++ Opcodes.cpp:1811/2150/2173. No ACK is sent before the disconnect boundary.
use super::*;

#[tokio::test]
async fn pending_worldport_uses_separate_cpp_connections() {
    let (mut session, instance) = make_session_with_send_capacity(100);
    let (realm_tx, realm) = flume::unbounded();
    session.install_realm_send_channel_for_test(realm_tx);
    session.set_map_store(crate::teleport_test_fixtures::world_maps([0, 571]));
    let guid = ObjectGuid::create_player(1, 585_2173);
    assert!(session.ensure_login_player_controller_like_cpp(
        guid,
        "TransferRoute".into(),
        Position::ZERO,
        0,
        1,
        1,
        80,
        0,
    ));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);
    let destination = Position::new(100.0, 200.0, 40.0, 2.0);
    session.teleport_to(571, destination).await;
    let realm_packets: Vec<_> = realm.try_iter().collect();
    assert_eq!(realm_packets.len(), 1);
    assert_eq!(
        u16::from_le_bytes(realm_packets[0][..2].try_into().unwrap()),
        ServerOpcodes::TransferPending as u16
    );
    let instance_opcodes: Vec<_> = instance
        .try_iter()
        .map(|packet| u16::from_le_bytes(packet[..2].try_into().unwrap()))
        .collect();
    assert!(instance_opcodes.contains(&(ServerOpcodes::SuspendToken as u16)));
    assert!(!instance_opcodes.contains(&(ServerOpcodes::TransferPending as u16)));
    session
        .handle_suspend_token_response(wow_packet::WorldPacket::new_empty())
        .await;
    let new_world = realm.try_recv().expect("NewWorld on realm");
    assert_eq!(
        u16::from_le_bytes(new_world[..2].try_into().unwrap()),
        ServerOpcodes::NewWorld as u16
    );
    assert_eq!(
        u32::from_le_bytes(new_world[30..34].try_into().unwrap()),
        16
    );
    assert!(instance.is_empty());
    assert!(session.represented_far_teleport_pending_like_cpp());
    drop(realm);
    session
        .handle_suspend_token_response(wow_packet::WorldPacket::new_empty())
        .await;
    assert_eq!(session.state(), crate::session::SessionState::Disconnecting);
}
