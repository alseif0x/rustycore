// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! reputation capability handler tests.

use super::*;

#[tokio::test]
async fn request_forced_reactions_sends_cpp_packet_like_cpp() {
    let (mut session, send_rx) = make_session();
    session
        .reputation_mgr_like_cpp_mut()
        .apply_force_reaction_like_cpp(72, ReputationRankLikeCpp::Hostile, true);

    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::RequestForcedReactions as u16);
    session.handle_request_forced_reactions(pkt).await;

    let bytes = send_rx.try_recv().expect("forced reactions packet");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::SetForcedReactions as u16
    );
    assert_eq!(&bytes[2..6], &1u32.to_le_bytes());
    assert_eq!(&bytes[6..10], &72i32.to_le_bytes());
    assert_eq!(
        &bytes[10..14],
        &(ReputationRankLikeCpp::Hostile.as_u8() as i32).to_le_bytes()
    );
}

#[tokio::test]
async fn set_faction_at_war_handlers_mark_reputation_state_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 10, 0);
    let mut faction = FactionEntry::for_test_like_cpp(72, 4);
    faction.reputation_flags[0] = ReputationFlagsLikeCpp::VISIBLE.bits();
    session.set_faction_store(Arc::new(FactionStore::from_entries([faction])));

    let mut at_war = WorldPacket::new_empty();
    at_war.write_uint16(ClientOpcodes::SetFactionAtWar as u16);
    at_war.write_uint16(4);
    session.handle_set_faction_at_war(at_war).await;

    let state = session
        .reputation_mgr_like_cpp()
        .get_state(4)
        .expect("reputation state");
    assert!(state.flags.contains(ReputationFlagsLikeCpp::AT_WAR));
    assert!(state.need_send);
    assert!(state.need_save);
    assert!(send_rx.try_recv().is_err());

    let mut not_at_war = WorldPacket::new_empty();
    not_at_war.write_uint16(ClientOpcodes::SetFactionNotAtWar as u16);
    not_at_war.write_uint16(4);
    session.handle_set_faction_not_at_war(not_at_war).await;

    let state = session
        .reputation_mgr_like_cpp()
        .get_state(4)
        .expect("reputation state");
    assert!(!state.flags.contains(ReputationFlagsLikeCpp::AT_WAR));
    assert!(state.need_send);
    assert!(state.need_save);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn set_faction_inactive_marks_visible_state_like_cpp() {
    let (mut session, send_rx) = make_session();
    session
        .reputation_mgr_like_cpp_mut()
        .insert_state_for_test_like_cpp(crate::reputation::mgr::FactionStateLikeCpp::new_like_cpp(
            72,
            4,
            ReputationFlagsLikeCpp::VISIBLE,
        ));

    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::SetFactionInactive as u16);
    pkt.write_uint32(4);
    pkt.write_bit(true);
    pkt.flush_bits();
    session.handle_set_faction_inactive(pkt).await;

    let state = session
        .reputation_mgr_like_cpp()
        .get_state(4)
        .expect("reputation state");
    assert!(state.flags.contains(ReputationFlagsLikeCpp::INACTIVE));
    assert!(state.need_send);
    assert!(state.need_save);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn set_watched_faction_records_active_player_index_like_cpp() {
    let (mut session, send_rx) = make_session();

    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::SetWatchedFaction as u16);
    pkt.write_uint32(42);
    session.handle_set_watched_faction(pkt).await;

    assert_eq!(session.watched_faction_index_like_cpp(), 42);
    assert!(send_rx.try_recv().is_err());
}
