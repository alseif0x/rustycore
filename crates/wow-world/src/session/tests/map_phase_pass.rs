//! #787 — the split between the world pass and the map pass, at session level.
//!
//! C++ runs one queued packet in exactly one of the two passes
//! (`Server/WorldSession.cpp:64-108`) and stops a pass at a head that pass may
//! not process (`common/Threading/LockedQueue.h:82-95`).

use super::*;
use crate::session::mailbox::{
    RunMapPhasePassLikeCppCommand, SessionPhasePassOutcomeLikeCpp, SessionPhasePermitLikeCpp,
    SessionPhasePermitStateLikeCpp,
};
use wow_handler::{PacketUpdatePhase, PlayerPacketResidence};

/// Put the session in the directory the way production does before the
/// canonical producer can address its phase rail.
fn register_session_for_phases(session: &mut WorldSession) {
    use crate::session::directory::{
        PlayerDirectoryIdentityLikeCpp, PlayerDirectoryPlacementLikeCpp, PlayerRegistry,
        PlayerSessionRegistrationLikeCpp,
    };

    let guid = session
        .player_guid()
        .expect("the fixture installed a player");
    let registry = std::sync::Arc::new(PlayerRegistry::new());
    let (send_tx, _send_rx) = flume::bounded(8);
    registry.register_or_replace(
        guid,
        PlayerSessionRegistrationLikeCpp {
            identity: PlayerDirectoryIdentityLikeCpp::new("PhasePass", 1, 0, 1, 1, 0, 2),
            placement: PlayerDirectoryPlacementLikeCpp {
                map_id: 571,
                instance_id: 0,
                position: Default::default(),
                is_in_world: true,
                level: 1,
                is_alive: true,
            },
            active_loot_rolls: vec![],
            realm_send_tx: send_tx.clone(),
            send_tx,
            command_tx: session.session_command_tx(),
            // The production rail: the producer addresses this session here.
            session_phase_tx: session.session_phase_sender_like_cpp(),
            durable_creature_runtime_commands_like_cpp: Default::default(),
            client_visible_guids_like_cpp: Default::default(),
            advanced_combat_logging_enabled_like_cpp: Default::default(),
            visibility_refresh_pending_like_cpp: Default::default(),
        },
        Default::default(),
    );
    session.set_player_registry(registry);
    assert!(
        session
            .player_registry()
            .and_then(|registry| registry.session_phase_address_like_cpp(guid))
            .is_some(),
        "the session must be addressable on its phase rail"
    );
}

fn queued(session: &mut WorldSession, opcode: ClientOpcodes) {
    let raw = (opcode as u32) as u16;
    let mut bytes = raw.to_le_bytes().to_vec();
    bytes.extend_from_slice(&[0u8; 4]);
    let packet = WorldPacket::from_bytes(&bytes);
    assert_eq!(packet.client_opcode(), Some(opcode));
    session.push_pending_packet_for_test_like_cpp(packet);
}

#[tokio::test]
async fn an_uncoordinated_session_still_drains_its_own_queue() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    queued(&mut session, ClientOpcodes::MoveInitActiveMoverComplete);

    // No canonical tick has asked this session for a map pass, so nothing else
    // would run these packets: the pre-#787 behaviour is kept deliberately.
    assert!(!session.is_map_phase_coordinated_like_cpp());
    let summary = session
        .run_world_phase_dispatch_like_cpp(&SessionHandlerCatalogsLikeCpp::default())
        .await;

    assert_eq!(summary.dispatched, 1);
    assert_eq!(session.pending_packet_count_for_test_like_cpp(), 0);
}

#[tokio::test]
async fn a_coordinated_world_pass_leaves_the_map_eligible_head_queued() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.mark_map_phase_coordinated_like_cpp();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    queued(&mut session, ClientOpcodes::MoveInitActiveMoverComplete);

    assert_eq!(
        session.player_packet_residence_like_cpp(),
        PlayerPacketResidence::InWorld
    );

    let world = session
        .run_world_phase_dispatch_like_cpp(&SessionHandlerCatalogsLikeCpp::default())
        .await;

    // `MoveInitActiveMoverComplete` is PROCESS_THREADSAFE, so an in-world
    // player's copy belongs to the map pass; the world pass stops at it.
    assert_eq!(world.dispatched, 0);
    assert!(world.stopped_at_ineligible_head);
    assert_eq!(session.pending_packet_count_for_test_like_cpp(), 1);

    let map = session
        .run_phase_packet_pass_like_cpp(
            PacketUpdatePhase::Map,
            &SessionHandlerCatalogsLikeCpp::default(),
        )
        .await;

    assert_eq!(map.dispatched, 1);
    assert_eq!(session.pending_packet_count_for_test_like_cpp(), 0);
}

#[tokio::test]
async fn the_map_pass_stops_at_a_thread_unsafe_head_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.mark_map_phase_coordinated_like_cpp();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    // A world-only head in front of a map-eligible packet: C++ leaves both
    // queued for their own pass rather than reordering the queue.
    queued(&mut session, ClientOpcodes::ArenaTeamRoster);
    queued(&mut session, ClientOpcodes::MoveInitActiveMoverComplete);

    let map = session
        .run_phase_packet_pass_like_cpp(
            PacketUpdatePhase::Map,
            &SessionHandlerCatalogsLikeCpp::default(),
        )
        .await;

    assert_eq!(map.dispatched, 0);
    assert!(map.stopped_at_ineligible_head);
    assert_eq!(session.pending_packet_count_for_test_like_cpp(), 2);
}

#[tokio::test]
async fn an_admitted_map_phase_request_runs_the_pass_and_reports_after_it() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    register_session_for_phases(&mut session);
    queued(&mut session, ClientOpcodes::MoveInitActiveMoverComplete);

    let admission = session
        .current_map_phase_admission_for_test_like_cpp(1, 7, 50)
        .expect("the session can be admitted");
    let permit = SessionPhasePermitLikeCpp::new_like_cpp();
    let (response_tx, response_rx) = flume::bounded(1);
    session
        .run_requested_session_phase_like_cpp(
            crate::session::mailbox::SessionPhaseRequestLikeCpp::Map(
                RunMapPhasePassLikeCppCommand {
                    admission,
                    permit: std::sync::Arc::clone(&permit),
                    response_tx,
                },
            ),
            &SessionHandlerCatalogsLikeCpp::default(),
        )
        .await;

    let result = response_rx.try_recv().expect("completion reported");
    assert_eq!(result.tick_epoch, 7);
    assert_eq!(result.outcome, SessionPhasePassOutcomeLikeCpp::Ran);
    assert_eq!(result.dispatched, 1);
    assert!(session.is_map_phase_coordinated_like_cpp());
    assert_eq!(session.pending_packet_count_for_test_like_cpp(), 0);
    // The completion is reported after the pass, and the permit says so.
    assert_eq!(
        permit.state_like_cpp(),
        SessionPhasePermitStateLikeCpp::Completed
    );
}

#[tokio::test]
async fn a_revoked_request_runs_no_effect_of_the_pass() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    register_session_for_phases(&mut session);
    queued(&mut session, ClientOpcodes::MoveInitActiveMoverComplete);

    let admission = session
        .current_map_phase_admission_for_test_like_cpp(1, 7, 50)
        .expect("the session can be admitted");
    let permit = SessionPhasePermitLikeCpp::new_like_cpp();
    // The coordinator's deadline arrived before this session reached the
    // request. Nothing of the pass may run now.
    permit.revoke_before_start_like_cpp();
    let (response_tx, response_rx) = flume::bounded(1);
    session
        .run_requested_session_phase_like_cpp(
            crate::session::mailbox::SessionPhaseRequestLikeCpp::Map(
                RunMapPhasePassLikeCppCommand {
                    admission,
                    permit,
                    response_tx,
                },
            ),
            &SessionHandlerCatalogsLikeCpp::default(),
        )
        .await;

    let result = response_rx.try_recv().expect("refusal reported");
    assert_eq!(
        result.outcome,
        SessionPhasePassOutcomeLikeCpp::RevokedBeforeStart
    );
    assert_eq!(result.dispatched, 0);
    assert_eq!(session.pending_packet_count_for_test_like_cpp(), 1);
}

#[tokio::test]
async fn an_admission_from_before_a_residence_change_is_refused_before_any_effect() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    register_session_for_phases(&mut session);
    queued(&mut session, ClientOpcodes::MoveInitActiveMoverComplete);

    let admission = session
        .current_map_phase_admission_for_test_like_cpp(1, 7, 50)
        .expect("the session can be admitted");

    // Between the split and the delivery the player left the map it was
    // admitted on. C++ could not observe this because the map thread held the
    // session; here the frozen admission is what catches it.
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());

    let permit = SessionPhasePermitLikeCpp::new_like_cpp();
    let (response_tx, response_rx) = flume::bounded(1);
    session
        .run_requested_session_phase_like_cpp(
            crate::session::mailbox::SessionPhaseRequestLikeCpp::Map(
                RunMapPhasePassLikeCppCommand {
                    admission,
                    permit: std::sync::Arc::clone(&permit),
                    response_tx,
                },
            ),
            &SessionHandlerCatalogsLikeCpp::default(),
        )
        .await;

    let result = response_rx.try_recv().expect("refusal reported");
    assert_eq!(
        result.outcome,
        SessionPhasePassOutcomeLikeCpp::RefusedBeforeStart
    );
    assert_eq!(result.dispatched, 0);
    assert_eq!(session.pending_packet_count_for_test_like_cpp(), 1);
    assert_eq!(
        permit.state_like_cpp(),
        SessionPhasePermitStateLikeCpp::RefusedBeforeStart
    );
}

#[tokio::test]
async fn a_thread_safe_head_moves_to_the_world_pass_when_the_player_leaves_the_world() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.mark_map_phase_coordinated_like_cpp();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    queued(&mut session, ClientOpcodes::MoveInitActiveMoverComplete);

    // C++ re-checks `player->IsInWorld()` inside the filter for every candidate
    // packet (`WorldSession.cpp:71-78`). This is the `PROCESS_THREADSAFE` case:
    // leaving the world moves its packets to the world pass. It says nothing
    // about `PROCESS_INPLACE`, which stays eligible in both phases regardless
    // of residence (`wow-handler/src/processing.rs:47`), so this is a statement
    // about one processing class, not about admission in general.
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_ne!(
        session.player_packet_residence_like_cpp(),
        PlayerPacketResidence::InWorld
    );

    let map = session
        .run_phase_packet_pass_like_cpp(
            PacketUpdatePhase::Map,
            &SessionHandlerCatalogsLikeCpp::default(),
        )
        .await;

    assert_eq!(map.dispatched, 0);
    assert!(map.stopped_at_ineligible_head);
    assert_eq!(session.pending_packet_count_for_test_like_cpp(), 1);

    let world = session
        .run_world_phase_dispatch_like_cpp(&SessionHandlerCatalogsLikeCpp::default())
        .await;

    assert_eq!(world.dispatched, 1);
    assert_eq!(session.pending_packet_count_for_test_like_cpp(), 0);
}

#[tokio::test]
async fn the_map_pass_tail_sends_the_periodic_time_sync_with_the_admitted_diff() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    register_session_for_phases(&mut session);
    session.state = SessionState::LoggedIn;
    session.time_sync_timer_ms = 200;

    let admission = session
        .current_map_phase_admission_for_test_like_cpp(1, 7, 50)
        .expect("the session can be admitted");
    let (response_tx, _response_rx) = flume::bounded(1);
    session
        .run_requested_session_phase_like_cpp(
            crate::session::mailbox::SessionPhaseRequestLikeCpp::Map(
                RunMapPhasePassLikeCppCommand {
                    admission,
                    permit: SessionPhasePermitLikeCpp::new_like_cpp(),
                    response_tx,
                },
            ),
            &SessionHandlerCatalogsLikeCpp::default(),
        )
        .await;

    // C++ decrements this timer on the `!ProcessUnsafe()` branch, i.e. in the
    // map filter's pass, with that pass's diff (`WorldSession.cpp:488-497`).
    // It runs even though no packet was queued.
    assert_eq!(session.time_sync_timer_ms, 150);
}

#[tokio::test]
async fn a_coordinated_session_does_not_send_the_time_sync_twice_per_step() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    register_session_for_phases(&mut session);
    session.state = SessionState::LoggedIn;
    session.time_sync_timer_ms = 200;
    session.mark_map_phase_coordinated_like_cpp();

    // The world pass of the same step must leave the timer to the map pass:
    // C++ sends it on exactly one of the two branches.
    session
        .update_with_catalogs_like_cpp(50, &SessionHandlerCatalogsLikeCpp::default())
        .await;

    assert_eq!(session.time_sync_timer_ms, 200);
}

#[tokio::test]
async fn an_admission_whose_residence_revision_moved_is_refused_like_an_away_and_back_transfer() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    register_session_for_phases(&mut session);
    queued(&mut session, ClientOpcodes::MoveInitActiveMoverComplete);

    let mut admission = session
        .current_map_phase_admission_for_test_like_cpp(1, 7, 50)
        .expect("the session can be admitted");
    // The player left the map and came back to it while no guard was held. The
    // key matches again, so only the revision distinguishes the two residences.
    admission.residence_revision = admission.residence_revision.wrapping_add(1);

    let permit = SessionPhasePermitLikeCpp::new_like_cpp();
    let (response_tx, response_rx) = flume::bounded(1);
    session
        .run_requested_session_phase_like_cpp(
            crate::session::mailbox::SessionPhaseRequestLikeCpp::Map(
                RunMapPhasePassLikeCppCommand {
                    admission,
                    permit,
                    response_tx,
                },
            ),
            &SessionHandlerCatalogsLikeCpp::default(),
        )
        .await;

    let result = response_rx.try_recv().expect("refusal reported");
    assert_eq!(
        result.outcome,
        SessionPhasePassOutcomeLikeCpp::RefusedBeforeStart
    );
    assert_eq!(session.pending_packet_count_for_test_like_cpp(), 1);
}

#[tokio::test]
async fn an_admission_for_another_incarnation_of_the_same_map_is_refused() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    register_session_for_phases(&mut session);
    queued(&mut session, ClientOpcodes::MoveInitActiveMoverComplete);

    let mut admission = session
        .current_map_phase_admission_for_test_like_cpp(1, 7, 50)
        .expect("the session can be admitted");
    // `MapKey` is reusable: this request was admitted by a tick of the map that
    // held the key before the current one.
    admission.map_incarnation = admission.map_incarnation.wrapping_add(1);

    let permit = SessionPhasePermitLikeCpp::new_like_cpp();
    let (response_tx, response_rx) = flume::bounded(1);
    session
        .run_requested_session_phase_like_cpp(
            crate::session::mailbox::SessionPhaseRequestLikeCpp::Map(
                RunMapPhasePassLikeCppCommand {
                    admission,
                    permit,
                    response_tx,
                },
            ),
            &SessionHandlerCatalogsLikeCpp::default(),
        )
        .await;

    let result = response_rx.try_recv().expect("refusal reported");
    assert_eq!(
        result.outcome,
        SessionPhasePassOutcomeLikeCpp::RefusedBeforeStart
    );
    assert_eq!(session.pending_packet_count_for_test_like_cpp(), 1);
}

#[tokio::test]
async fn control_traffic_behind_a_map_eligible_head_still_advances_in_the_world_phase() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    register_session_for_phases(&mut session);
    session.state = SessionState::LoggedIn;
    // A head this phase may not process, which C++ leaves queued rather than
    // skipping (`LockedQueue.h:82-95`).
    queued(&mut session, ClientOpcodes::MoveInitActiveMoverComplete);

    // Control does not travel on the packet queue: it has its own rail, so an
    // ineligible head cannot stall it. Without that separation the world pass
    // would be unable to answer while the map phase owns the head.
    let (ack_tx, ack_rx) = flume::bounded(1);
    session
        .session_command_tx()
        .try_send(
            crate::session::mailbox::SessionCommand::WorldSessionShutdownFlushLikeCpp(
                crate::session::mailbox::WorldSessionShutdownFlushLikeCppCommand {
                    diff_ms: 1,
                    response_tx: ack_tx,
                },
            ),
        )
        .expect("the control channel accepts the command");

    let permit = SessionPhasePermitLikeCpp::new_like_cpp();
    let (response_tx, response_rx) = flume::bounded(1);
    session
        .run_requested_session_phase_like_cpp(
            crate::session::mailbox::SessionPhaseRequestLikeCpp::World(
                crate::session::mailbox::RunWorldPhasePassLikeCppRequest {
                    coordinator_id: 1,
                    tick_epoch: 7,
                    diff_ms: 50,
                    permit,
                    response_tx,
                },
            ),
            &SessionHandlerCatalogsLikeCpp::default(),
        )
        .await;

    let world = response_rx.try_recv().expect("the world pass reported");
    assert_eq!(world.outcome, SessionPhasePassOutcomeLikeCpp::Ran);
    assert!(
        ack_rx.try_recv().is_ok(),
        "the control command must be observed even though the packet head is not this phase's"
    );
    assert_eq!(
        session.pending_packet_count_for_test_like_cpp(),
        1,
        "the map-eligible head stays queued for the map pass"
    );
}
