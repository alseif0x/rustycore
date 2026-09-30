//! Owned, packet-neutral snapshots for reputation publication.
//!
//! These values are built from the canonical Player-owned manager state. The
//! application layer translates them to wire packet DTOs after canonical access
//! ends; they do not own or mutate reputation state.

use wow_constants::reputation::FACTION_COUNT_LIKE_CPP;

#[derive(Debug, Clone, PartialEq)]
pub struct InitializeFactionStateLikeCpp {
    pub faction_standings: [i32; FACTION_COUNT_LIKE_CPP],
    pub faction_has_bonus: [bool; FACTION_COUNT_LIKE_CPP],
    pub faction_flags: [u16; FACTION_COUNT_LIKE_CPP],
}

impl Default for InitializeFactionStateLikeCpp {
    fn default() -> Self {
        Self {
            faction_standings: [0; FACTION_COUNT_LIKE_CPP],
            faction_has_bonus: [false; FACTION_COUNT_LIKE_CPP],
            faction_flags: [0; FACTION_COUNT_LIKE_CPP],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactionStandingStateLikeCpp {
    pub index: i32,
    pub standing: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FactionStandingUpdateLikeCpp {
    pub bonus_from_achievement_system: f32,
    pub faction: Vec<FactionStandingStateLikeCpp>,
    pub show_visual: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForcedReactionStateLikeCpp {
    pub faction: i32,
    pub reaction: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ForcedReactionsStateLikeCpp {
    pub reactions: Vec<ForcedReactionStateLikeCpp>,
}
