// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! arena capability handler tests.

use super::*;

#[tokio::test]
async fn arena_team_decline_clears_invited_arena_team_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_represented_arena_team_id_invited_like_cpp(12_345);

    session
        .handle_arena_team_decline(WorldPacket::new_empty())
        .await;

    assert_eq!(session.represented_arena_team_id_invited_like_cpp(), 0);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn arena_team_accept_without_manager_preserves_invited_id_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_represented_arena_team_id_invited_like_cpp(12_345);

    session
        .handle_arena_team_accept(WorldPacket::new_empty())
        .await;

    assert_eq!(session.represented_arena_team_id_invited_like_cpp(), 12_345);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn arena_team_leave_without_manager_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_arena_team_leave(WorldPacket::new_empty())
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn arena_team_remove_without_manager_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(77);
    pkt.write_bits(6, 9);
    pkt.write_string("Target");

    session.handle_arena_team_remove(pkt).await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn arena_team_disband_without_manager_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(77);

    session.handle_arena_team_disband(pkt).await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn arena_team_leader_without_manager_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(77);
    pkt.write_bits(6, 9);
    pkt.write_string("Leader");

    session.handle_arena_team_leader(pkt).await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn query_arena_team_without_manager_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(77);

    session.handle_query_arena_team(pkt).await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn arena_team_roster_unknown_team_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    let mut request = WorldPacket::new_empty();
    request.write_uint32(1234);

    session.handle_arena_team_roster(request).await;

    assert!(send_rx.try_recv().is_err());
}
