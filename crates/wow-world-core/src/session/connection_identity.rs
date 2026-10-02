// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Connection identity data shared with the WorldSession shell.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PacketCounterLikeCpp {
    pub last_receive_time_secs: u64,
    pub amount_counter: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PacketSpoofPendingBanTargetLikeCpp {
    Account { account_id: u32 },
    Ip { address: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketSpoofPendingBanLikeCpp {
    pub target: PacketSpoofPendingBanTargetLikeCpp,
    pub duration_secs: u32,
}

/// Current state of the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Authenticated but no character selected.
    Authed,
    /// Character is logged into the world.
    LoggedIn,
    /// Character is transferring between maps.
    Transfer,
    /// Session is being disconnected.
    Disconnecting,
}
