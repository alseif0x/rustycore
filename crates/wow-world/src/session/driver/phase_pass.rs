// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Phase-filtered packet selection for one Session pass (#787).
//!
//! C++ selects queued packets with `LockedQueue::next(result, check)`
//! (`common/Threading/LockedQueue.h:82-95`): it reads the **front** and, when
//! the filter refuses it, returns without popping. Selection is therefore FIFO,
//! head-only, and an ineligible head ends that pass instead of being skipped.
//!
//! The filter itself is `MapSessionFilter` / `WorldSessionFilter`
//! (`Server/WorldSession.cpp:64-108`), already modelled by
//! `PacketProcessing::allows_phase` over the registration metadata and the
//! Player's residence. This module is the consumer that contract was missing.

use wow_handler::{PacketUpdatePhase, PlayerPacketResidence};

use super::super::{SessionHandlerCatalogsLikeCpp, WorldSession};

/// What one phase-filtered pass did, for the caller that must decide whether
/// the pass finished its queue or stopped at a head it may not process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct SessionPhasePassSummaryLikeCpp {
    /// Packets selected and dispatched in this pass.
    pub(crate) dispatched: usize,
    /// True when the pass ended because the head was ineligible for the phase,
    /// as C++ `LockedQueue::next` leaves it queued.
    pub(crate) stopped_at_ineligible_head: bool,
}

impl WorldSession {
    /// Whether a canonical map tick is driving this session's map phase.
    ///
    /// A session only switches to strict world-phase selection once it has
    /// actually been asked to run a map pass. Until then it keeps processing
    /// its whole queue in its own task, because nothing else would: a session
    /// with no coordinator must not strand the packets C++ would have run
    /// inside `Map::Update`. The flag is set by the first map-phase request and
    /// is the migration boundary of #787, not a permanent mode.
    #[must_use]
    pub(crate) fn is_map_phase_coordinated_like_cpp(&self) -> bool {
        self.map_phase_coordinated_like_cpp
    }

    /// Record that a canonical tick asked this session to run its map phase.
    pub(crate) fn mark_map_phase_coordinated_like_cpp(&mut self) {
        self.map_phase_coordinated_like_cpp = true;
    }

    /// Queue one packet as if it had been ingested, for regressions that must
    /// control the queue head.
    #[cfg(test)]
    pub(crate) fn push_pending_packet_for_test_like_cpp(
        &mut self,
        packet: wow_packet::WorldPacket,
    ) {
        self.pending_packets.push_back(packet);
    }

    /// How many packets are still queued for a later pass.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn pending_packet_count_for_test_like_cpp(&self) -> usize {
        self.pending_packets.len()
    }

    /// The Player's presence at the point of packet selection.
    ///
    /// C++ asks `player->IsInWorld()` inside the filter for every candidate
    /// packet (`WorldSession.cpp:71-78`), so this is observed per selection
    /// rather than cached for the pass: a transfer handled earlier in the same
    /// pass changes which packets the rest of it may process.
    pub(crate) fn player_packet_residence_like_cpp(&self) -> PlayerPacketResidence {
        if self.player_guid().is_none() {
            return PlayerPacketResidence::Missing;
        }
        if self.player_is_strictly_in_world_like_cpp() {
            PlayerPacketResidence::InWorld
        } else {
            PlayerPacketResidence::OutsideWorld
        }
    }

    /// Whether the queued head may be processed in `phase` right now.
    ///
    /// An opcode with no registration is eligible only in the world pass: the
    /// world pass is the one that drains and logs what the map pass must never
    /// consume, and leaving it queued forever would block the head for both.
    fn queued_head_allows_phase_like_cpp(
        &self,
        phase: PacketUpdatePhase,
        residence: PlayerPacketResidence,
    ) -> bool {
        let Some(head) = self.pending_packets.front() else {
            return false;
        };
        let Some(opcode) = head.client_opcode() else {
            return matches!(phase, PacketUpdatePhase::World);
        };
        let Some(entry) = self.dispatch_table.get(&opcode) else {
            return matches!(phase, PacketUpdatePhase::World);
        };
        entry.processing.allows_phase(phase, residence)
    }

    /// Run one phase-filtered dispatch pass over the queued packets.
    ///
    /// Selection follows C++ `LockedQueue::next`: only the head is considered,
    /// and the first head this phase may not process ends the pass with the
    /// packet still queued for the other one.
    /// The world-phase dispatch of one Session driver pass.
    ///
    /// A coordinated session selects with `WorldSessionFilter`, leaving the
    /// map-eligible head for the map pass, exactly as C++ splits the two
    /// (`Server/WorldSession.cpp:85-107`). An uncoordinated session keeps the
    /// pre-#787 behaviour and drains its queue, because no map tick will.
    pub(crate) async fn run_world_phase_dispatch_like_cpp(
        &mut self,
        catalogs: &SessionHandlerCatalogsLikeCpp,
    ) -> SessionPhasePassSummaryLikeCpp {
        if self.is_map_phase_coordinated_like_cpp() {
            return self
                .run_phase_packet_pass_like_cpp(PacketUpdatePhase::World, catalogs)
                .await;
        }

        let mut summary = SessionPhasePassSummaryLikeCpp::default();
        while let Some(pkt) = self.pending_packets.pop_front() {
            self.dispatch_packet(catalogs, pkt).await;
            summary.dispatched = summary.dispatched.saturating_add(1);
        }
        summary
    }

    pub(crate) async fn run_phase_packet_pass_like_cpp(
        &mut self,
        phase: PacketUpdatePhase,
        catalogs: &SessionHandlerCatalogsLikeCpp,
    ) -> SessionPhasePassSummaryLikeCpp {
        let mut summary = SessionPhasePassSummaryLikeCpp::default();
        loop {
            if self.pending_packets.is_empty() {
                return summary;
            }
            let residence = self.player_packet_residence_like_cpp();
            if !self.queued_head_allows_phase_like_cpp(phase, residence) {
                summary.stopped_at_ineligible_head = true;
                return summary;
            }
            let Some(pkt) = self.pending_packets.pop_front() else {
                return summary;
            };
            self.dispatch_packet(catalogs, pkt).await;
            summary.dispatched = summary.dispatched.saturating_add(1);
        }
    }
}

#[cfg(test)]
#[path = "phase_pass_tests.rs"]
mod tests;
