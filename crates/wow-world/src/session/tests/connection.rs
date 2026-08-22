// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Routing matrix for the logical realm/instance connection state machine.
//!
//! Each case pins one column of the matrix issue #182 requires: primary-only,
//! attach, switch, restore, stale link, disconnect and failed write.

use super::*;

use wow_network::InstanceLink;

/// Primary-only: before any ConnectTo there is no realm channel, so realm-routed
/// sends fall back to the primary connection instead of being dropped.
#[test]
fn primary_only_realm_sends_fall_back_to_the_primary_connection_like_cpp() {
    let (session, _, primary_rx) = make_session();

    session.send_raw_packet_realm(&[0x11, 0x22]);

    assert_eq!(primary_rx.try_recv().unwrap(), vec![0x11, 0x22]);
    assert!(primary_rx.try_recv().is_err());
}

/// Attach: when the instance link arrives, the instance channel becomes primary
/// and the previous primary is parked as the realm channel. Neither socket is
/// closed — the client drops the session if either TCP connection dies.
#[tokio::test]
async fn instance_link_attach_parks_the_previous_primary_as_realm_like_cpp() {
    let (mut session, _, realm_rx) = make_session();
    let (instance_tx, instance_rx) = flume::unbounded();
    let (link_tx, link_rx) = tokio::sync::oneshot::channel();
    session.set_instance_link_rx(Some(link_rx));
    link_tx
        .send(InstanceLink {
            send_tx: instance_tx,
            send_write_fence_like_cpp: None,
            pkt_rx: None,
        })
        .ok()
        .expect("instance link delivered");

    session.poll_instance_link().await;

    // Switch: the default route now reaches the instance socket, while the
    // realm-routed variant still reaches the parked realm socket.
    session.send_raw_packet(&[0xAA]);
    session.send_raw_packet_realm(&[0xBB]);

    assert_eq!(instance_rx.try_recv().unwrap(), vec![0xAA]);
    assert_eq!(realm_rx.try_recv().unwrap(), vec![0xBB]);
    assert!(instance_rx.try_recv().is_err());
    assert!(realm_rx.try_recv().is_err());
}

/// Restore: logout returns the client to character select on the realm
/// connection, so the parked realm channel becomes primary again and every
/// ConnectTo bit of state is cleared.
#[tokio::test]
async fn restore_realm_channels_reinstates_the_realm_primary_and_clears_connect_to_like_cpp() {
    let (mut session, _, realm_rx) = make_session();
    let (instance_tx, instance_rx) = flume::unbounded();
    let (link_tx, link_rx) = tokio::sync::oneshot::channel();
    session.set_instance_link_rx(Some(link_rx));
    link_tx
        .send(InstanceLink {
            send_tx: instance_tx,
            send_write_fence_like_cpp: None,
            pkt_rx: None,
        })
        .ok()
        .expect("instance link delivered");
    session.poll_instance_link().await;
    session.set_connect_to_key(Some(7));

    session.restore_realm_channels();

    session.send_raw_packet(&[0xCC]);
    assert_eq!(realm_rx.try_recv().unwrap(), vec![0xCC]);
    assert!(instance_rx.try_recv().is_err());
    assert!(session.connect_to_key.is_none());
    assert!(session.connect_to_serial.is_none());
    assert!(session.instance_link_rx.is_none());
    assert!(session.realm_send_tx.is_none());

    // Restoring twice is idempotent: there is no parked realm channel left to
    // promote, so the primary must not be swapped for a dropped one.
    session.restore_realm_channels();
    session.send_raw_packet(&[0xDD]);
    assert_eq!(realm_rx.try_recv().unwrap(), vec![0xDD]);
}

/// Stale link: a closed oneshot means the instance connection never arrived.
/// The pending ConnectTo state is torn down instead of leaking.
#[tokio::test]
async fn closed_instance_link_clears_pending_connect_to_state_like_cpp() {
    let (mut session, _, _primary_rx) = make_session();
    let (link_tx, link_rx) = tokio::sync::oneshot::channel::<InstanceLink>();
    session.set_instance_link_rx(Some(link_rx));
    session.set_connect_to_key(Some(11));
    drop(link_tx);

    session.poll_instance_link().await;

    assert!(session.instance_link_rx.is_none());
    assert!(session.connect_to_key.is_none());
    assert!(session.player_loading().is_none());
}

/// An empty link is not a failure: polling again later must still attach.
#[tokio::test]
async fn pending_instance_link_leaves_the_primary_untouched_like_cpp() {
    let (mut session, _, primary_rx) = make_session();
    let (link_tx, link_rx) = tokio::sync::oneshot::channel::<InstanceLink>();
    session.set_instance_link_rx(Some(link_rx));

    session.poll_instance_link().await;

    assert!(session.instance_link_rx.is_some());
    session.send_raw_packet(&[0xEE]);
    assert_eq!(primary_rx.try_recv().unwrap(), vec![0xEE]);
    drop(link_tx);
}

/// Disconnect and failed write: a closed receiver must be reported, not
/// panicked on, on both the primary and the realm route.
#[test]
fn closed_channels_fail_the_send_without_panicking_like_cpp() {
    let (mut session, _, primary_rx) = make_session();
    let (realm_tx, realm_rx) = flume::unbounded();
    session.install_realm_send_channel_for_test(realm_tx);

    drop(realm_rx);
    session.send_raw_packet_realm(&[0x01]);
    assert!(primary_rx.try_recv().is_err());

    drop(primary_rx);
    session.send_raw_packet(&[0x02]);
}

/// The endpoint accessors round-trip the values the ConnectTo packet carries.
#[test]
fn instance_endpoint_round_trips_like_cpp() {
    let (mut session, _, _rx) = make_session();

    session.set_instance_endpoint([127, 0, 0, 1], 8086);

    assert_eq!(session.instance_address(), [127, 0, 0, 1]);
    assert_eq!(session.instance_port(), 8086);
}
