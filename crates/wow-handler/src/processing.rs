// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Packet-filter contract from `Server/WorldSession.cpp:64-108`.
//!
//! Eligibility is not scheduling or status admission. The driver must select a
//! packet once, preserve packets rejected by this filter for another phase, and
//! apply status admission separately. In particular, `Inplace` passing both
//! filters does not authorize calling the handler twice.

use super::PacketProcessing;

/// The C++ update path asking whether it can consume a queued packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketUpdatePhase {
    /// `WorldSessionFilter`, used by `World::UpdateSessions`.
    World,
    /// `MapSessionFilter`, before respawn and Player/object updates in `Map::Update`.
    Map,
}

/// The canonical Player's presence at the point of packet selection.
///
/// This is a point-in-time observation, not another lifetime authority. A stale
/// handle or inconsistent residence must be resolved by the owner, not mapped to
/// `Missing`. Session login labels alone do not establish `IsInWorld()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerPacketResidence {
    Missing,
    /// A Player exists but `IsInWorld()` is false, including a detached transfer.
    OutsideWorld,
    InWorld,
}

impl PacketProcessing {
    /// Mirror the two C++ packet filters, independently of handler status gates.
    ///
    /// Re-evaluate using canonical residence when selecting a packet: a transfer
    /// can change the eligible phase without changing its registration metadata.
    /// This pure contract does not install or synchronize runtime update phases.
    #[must_use]
    pub const fn allows_phase(
        self,
        phase: PacketUpdatePhase,
        residence: PlayerPacketResidence,
    ) -> bool {
        match self {
            Self::Inplace => true,
            Self::ThreadUnsafe => matches!(phase, PacketUpdatePhase::World),
            Self::ThreadSafe => match phase {
                PacketUpdatePhase::Map => matches!(residence, PlayerPacketResidence::InWorld),
                PacketUpdatePhase::World => !matches!(residence, PlayerPacketResidence::InWorld),
            },
        }
    }
}

#[cfg(test)]
mod tests;
