//! Instance scenarios for [`super`].
//!
//! Split out of group_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn minimap_ping_without_player_guid_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();

    session
        .handle_minimap_ping(minimap_ping_packet(10.0, 20.0, None))
        .await;

    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn minimap_ping_sender_not_in_registry_skips_sending_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let sender = ObjectGuid::create_player(1, 42);
    let other = ObjectGuid::create_player(1, 43);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(sender);
    group.add_member(other);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    // Only register 'other' — sender has no PlayerRegistry entry (edge case).
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (other_tx, other_rx) = bounded(8);
    player_registry.register_or_replace(other, broadcast_info(other, other_tx), Default::default());

    session.set_player_guid(Some(sender));
    session.group_guid = Some(group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    session
        .handle_minimap_ping(minimap_ping_packet(1.0, 2.0, None))
        .await;

    // Other should still receive (sender is excluded by guid comparison, not by registry).
    let sent = other_rx
        .try_recv()
        .expect("other should receive even if sender not in registry");
    let mut pkt = WorldPacket::from_bytes(&sent);
    assert_eq!(
        pkt.read_uint16().unwrap(),
        ServerOpcodes::MinimapPing as u16
    );
    assert_eq!(pkt.read_packed_guid().unwrap(), sender);
}
