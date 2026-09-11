// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Reputation value types shared by the canonical Player and its catalogs.
//!
//! C++ declares these in `SharedDefines.h` (`ReputationRank`) and
//! `ReputationMgr.h` (`FactionFlags`, the rank thresholds and the standing
//! caps). They carry no catalog, packet or database dependency, which is why
//! they live beside the other shared enums and flags rather than in the DB2
//! catalog crate: the canonical Player owns reputation state and cannot depend
//! on `wow-data` (#735).
//!
//! `wow_data::reputation` re-exports them, so catalog-side consumers are
//! unchanged.

/// C++ `ReputationMgr::Reputation_Cap`.
pub const REPUTATION_CAP_LIKE_CPP: i32 = 42_000;

/// C++ `ReputationMgr::Reputation_Bottom`.
pub const REPUTATION_BOTTOM_LIKE_CPP: i32 = -42_000;

/// C++ `ReputationMgr::PointsInRank`.
pub const REPUTATION_RANK_THRESHOLDS_LIKE_CPP: [i32; 8] =
    [-42_000, -6_000, -3_000, 0, 3_000, 9_000, 21_000, 42_000];

/// C++ `ReputationRank` (`SharedDefines.h`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ReputationRankLikeCpp {
    Hated = 0,
    Hostile = 1,
    Unfriendly = 2,
    Neutral = 3,
    Friendly = 4,
    Honored = 5,
    Revered = 6,
    Exalted = 7,
}

impl ReputationRankLikeCpp {
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    pub const fn from_u8_like_cpp(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Hated),
            1 => Some(Self::Hostile),
            2 => Some(Self::Unfriendly),
            3 => Some(Self::Neutral),
            4 => Some(Self::Friendly),
            5 => Some(Self::Honored),
            6 => Some(Self::Revered),
            7 => Some(Self::Exalted),
            _ => None,
        }
    }
}

/// C++ `ReputationMgr::ReputationToRank`.
pub fn reputation_rank_from_standing_like_cpp(standing: i32) -> ReputationRankLikeCpp {
    let rank = REPUTATION_RANK_THRESHOLDS_LIKE_CPP
        .iter()
        .position(|threshold| standing < *threshold)
        .map(|idx| idx.saturating_sub(1))
        .unwrap_or(REPUTATION_RANK_THRESHOLDS_LIKE_CPP.len() - 1);

    ReputationRankLikeCpp::from_u8_like_cpp(rank as u8)
        .expect("rank index is bounded by C++ threshold table")
}

bitflags::bitflags! {
    /// C++ `FactionFlags` (`ReputationMgr.h`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ReputationFlagsLikeCpp: u16 {
        const NONE = 0x0000;
        const VISIBLE = 0x0001;
        const AT_WAR = 0x0002;
        const HIDDEN = 0x0004;
        const HEADER = 0x0008;
        const PEACEFUL = 0x0010;
        const INACTIVE = 0x0020;
        const SHOW_PROPAGATED = 0x0040;
        const HEADER_SHOWS_BAR = 0x0080;
        const CAPITAL_CITY_FOR_RACE_CHANGE = 0x0100;
        const GUILD = 0x0200;
        const GARRISON_INVASION = 0x0400;
    }
}
