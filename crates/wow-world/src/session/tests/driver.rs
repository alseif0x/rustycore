// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Deterministic trace of the real Session driver.
//!
//! Every case here drives the production `update`/`process_pending` pair and
//! asserts on the phase trace they record. Nothing reimplements the order.

use super::*;

use crate::session::driver::MAX_PACKETS_PER_UPDATE;
use crate::session::driver::phases::SessionDriverPhaseLikeCpp as Phase;

mod queued_packets {
    //! Exercise the actual driver and dispatcher with a local test-only thunk.
    //! The thunk records entry, then suspends marker 1; no timers or real DB are used.
    use super::*;
    use std::future::Future;
    use std::task::{Context, Poll, Waker};

    static PROBE: crate::session::registry::PacketHandlerEntry =
        crate::session::registry::PacketHandlerEntry {
            opcode: ClientOpcodes::QueryTime,
            status: SessionStatus::Authed,
            processing: PacketProcessing::Inplace,
            handler_name: "queued_packet_probe",
            handler: |session, _catalogs, mut packet| {
                Box::pin(async move {
                    let marker = packet.read_uint32().unwrap();
                    session
                        .time_sync_clock_delta_queue
                        .push_back((i64::from(marker), 0));
                    if marker == 1 {
                        std::future::pending::<()>().await;
                    }
                })
            },
        };

    fn marked_packet(marker: u32) -> WorldPacket {
        let mut packet = WorldPacket::new_empty();
        packet.write_uint16(ClientOpcodes::QueryTime as u16);
        packet.write_uint32(marker);
        packet.flush_bits();
        packet.reset_read();
        packet
    }

    #[tokio::test]
    async fn cancelled_handler_preserves_unselected_packets_without_replaying_partial_effects() {
        let (mut session, _tx, _rx) = make_session();
        // Replace only this session's entry; the production inventory is untouched.
        session
            .dispatch_table
            .insert(ClientOpcodes::QueryTime, &PROBE);
        for marker in [1, 2, 3] {
            session.pending_packets.push_back(marked_packet(marker));
        }
        let mut pass = Box::pin(session.process_pending());
        assert!(matches!(
            pass.as_mut().poll(&mut Context::from_waker(Waker::noop())),
            Poll::Pending
        ));
        drop(pass);
        assert_eq!(
            session
                .time_sync_clock_delta_queue
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![(1, 0)]
        );
        assert_eq!(session.pending_packets.len(), 2);
        assert_eq!(
            session.pending_packets.front().unwrap().data(),
            marked_packet(2).data()
        );
        assert_eq!(
            session.pending_packets.back().unwrap().data(),
            marked_packet(3).data()
        );

        session.process_pending().await;
        assert!(session.pending_packets.is_empty());
        assert_eq!(
            session
                .time_sync_clock_delta_queue
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![(1, 0), (2, 0), (3, 0)]
        );
    }

    #[tokio::test]
    async fn dropping_an_unpolled_pass_leaves_the_entire_queue_owned_by_session() {
        let (mut session, _tx, _rx) = make_session();
        session.pending_packets.push_back(marked_packet(2));
        drop(session.process_pending());
        assert_eq!(session.pending_packets.len(), 1);
        assert_eq!(
            session.pending_packets.front().unwrap().data(),
            marked_packet(2).data()
        );
    }

    #[test]
    fn retained_packets_share_the_next_ingestion_budget_without_dropping_surplus() {
        let (mut session, _tx, _rx) = make_session();
        session.pending_packets.push_back(marked_packet(2));
        session.pending_packets.push_back(marked_packet(3));
        let (tx, rx) = flume::unbounded();
        session.set_packet_rx(rx);
        for _ in 0..MAX_PACKETS_PER_UPDATE {
            tx.send(benign_packet()).unwrap();
        }
        assert_eq!(session.update(0), MAX_PACKETS_PER_UPDATE - 2);
        assert_eq!(session.pending_packets.len(), MAX_PACKETS_PER_UPDATE);
        assert_eq!(tx.len(), 2);
        assert_eq!(session.update(0), 0);
        assert_eq!(tx.len(), 2);
        assert_eq!(
            session.pending_packets.front().unwrap().data(),
            marked_packet(2).data()
        );
    }
}

/// A cheap packet the AntiDOS gate never throttles.
fn benign_packet() -> WorldPacket {
    let mut packet = WorldPacket::new_empty();
    packet.write_uint16(ClientOpcodes::QueryTime as u16);
    packet.flush_bits();
    packet.reset_read();
    packet
}

async fn one_pass(session: &mut WorldSession, diff_ms: u32) -> usize {
    session.reset_driver_phase_trace_like_cpp();
    let processed = session.update(diff_ms);
    session.process_pending().await;
    processed
}

/// The frozen order of a pass with nothing to do and no logged-in player.
#[tokio::test]
async fn empty_pass_records_the_frozen_session_phase_order_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();

    let processed = one_pass(&mut session, 0).await;

    assert_eq!(processed, 0);
    assert_eq!(
        session.driver_phase_trace_like_cpp(),
        &[
            Phase::DrainPrimaryPackets,
            Phase::DrainRealmPackets,
            Phase::ConnectionTimeout,
            Phase::FlushPacketSpoofBan,
            Phase::SessionCommands,
            Phase::CreatureKills,
            Phase::PollInstanceLink,
            Phase::PendingCreatureSpawn,
            Phase::DispatchQueuedPackets,
            Phase::PeriodicPlayerSave,
        ]
    );
}

/// Sustained load keeps the same order and dispatches within the same pass.
#[tokio::test]
async fn sustained_load_keeps_the_phase_order_and_dispatches_in_the_same_pass_like_cpp() {
    let (mut session, pkt_tx, _send_rx) = make_session();
    for _ in 0..3 {
        pkt_tx.send(benign_packet()).unwrap();
    }

    let processed = one_pass(&mut session, 0).await;

    assert_eq!(processed, 3);
    let trace = session.driver_phase_trace_like_cpp();
    assert_eq!(trace.first(), Some(&Phase::DrainPrimaryPackets));
    assert_eq!(trace.last(), Some(&Phase::PeriodicPlayerSave));
    // Ingestion strictly precedes dispatch: a packet read this pass is
    // dispatched this pass, never before it was read.
    let ingest = trace
        .iter()
        .position(|phase| *phase == Phase::DrainPrimaryPackets)
        .expect("ingestion phase");
    let dispatch = trace
        .iter()
        .position(|phase| *phase == Phase::DispatchQueuedPackets)
        .expect("dispatch phase");
    assert!(ingest < dispatch);
    assert!(session.pending_packets.is_empty());
}

/// Bounded progress: one pass never ingests more than the shared budget, and
/// the surplus is left for the next pass rather than dropped.
#[tokio::test]
async fn ingestion_stops_at_the_shared_budget_like_cpp() {
    let (mut session, _bounded_tx, _send_rx) = make_session();
    // `make_session` wires a bounded(100) channel, which is exactly the
    // budget: overfilling it would block the test, not the driver. Install an
    // unbounded primary channel so the bound under test is the driver's.
    let (pkt_tx, pkt_rx) = flume::unbounded();
    session.set_packet_rx(pkt_rx);
    let surplus = 7;
    for _ in 0..MAX_PACKETS_PER_UPDATE + surplus {
        pkt_tx.send(benign_packet()).unwrap();
    }

    session.reset_driver_phase_trace_like_cpp();
    let processed = session.update(0);

    assert_eq!(processed, MAX_PACKETS_PER_UPDATE);
    assert_eq!(session.pending_packets.len(), MAX_PACKETS_PER_UPDATE);
    assert_eq!(pkt_tx.len(), surplus);
}

/// The realm channel does not get its own budget: a saturated primary channel
/// leaves nothing for the realm drain in the same pass.
#[tokio::test]
async fn realm_and_primary_ingestion_share_one_budget_like_cpp() {
    let (mut session, _bounded_tx, _send_rx) = make_session();
    let (pkt_tx, pkt_rx) = flume::unbounded();
    session.set_packet_rx(pkt_rx);
    let (realm_tx, realm_rx) = flume::unbounded();
    session.install_realm_packet_channel(realm_rx);
    for _ in 0..MAX_PACKETS_PER_UPDATE {
        pkt_tx.send(benign_packet()).unwrap();
    }
    realm_tx.send(benign_packet()).unwrap();

    let processed = session.update(0);

    assert_eq!(processed, MAX_PACKETS_PER_UPDATE);
    assert_eq!(realm_tx.len(), 1, "realm packet waits for the next pass");
}

/// Timeout: an expired idle deadline ends the session from inside the pass.
#[tokio::test]
async fn expired_idle_deadline_marks_the_session_disconnecting_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    assert!(!session.is_disconnecting());
    session.socket_timeout_deadline_like_cpp = Instant::now() - Duration::from_secs(1);

    session.reset_driver_phase_trace_like_cpp();
    session.update(0);

    assert!(
        session
            .driver_phase_trace_like_cpp()
            .contains(&Phase::ConnectionTimeout)
    );
    assert!(session.is_disconnecting());
}

/// Routing error / shutdown: a dropped sender ends the pass and the session.
#[tokio::test]
async fn disconnected_primary_channel_marks_the_session_disconnecting_like_cpp() {
    let (mut session, pkt_tx, _send_rx) = make_session();
    drop(pkt_tx);

    let processed = one_pass(&mut session, 0).await;

    assert_eq!(processed, 0);
    assert!(session.is_disconnecting());
}

/// No double tick: each pass records the sequence exactly once, and a second
/// pass repeats it rather than compounding it.
#[tokio::test]
async fn each_pass_records_the_phase_sequence_exactly_once_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();

    one_pass(&mut session, 0).await;
    let first = session.driver_phase_trace_like_cpp().to_vec();
    one_pass(&mut session, 0).await;
    let second = session.driver_phase_trace_like_cpp().to_vec();

    assert_eq!(first, second);
    for phase in &first {
        assert_eq!(
            first.iter().filter(|recorded| *recorded == phase).count(),
            1,
            "{phase:?} ran more than once in a single pass"
        );
    }
}
