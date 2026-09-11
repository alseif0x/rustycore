// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical Player reputation state.
//!
//! C++ `Player` owns `ReputationMgr m_reputationMgr` (`Player.h:3116`,
//! constructed at `Player.cpp:338`): the standings, forced reactions, rank
//! counters and the `_sendFactionIncreased` flag live inside the Player for its
//! whole lifetime. `ReputationMgr` writes those members directly.
//!
//! RustyCore keeps the same ownership. The state and its state-changing
//! invariants live here; the rules that need DB2 catalogs, faction templates or
//! packet builders stay in `wow-world`, which cannot be depended on from this
//! crate. Those rules reach this state through the named operations below —
//! never by rebuilding the aggregate and writing it back through the Player's
//! whole gameplay state (#735).

use std::collections::BTreeMap;

use wow_constants::reputation::{
    REPUTATION_BOTTOM_LIKE_CPP, REPUTATION_CAP_LIKE_CPP, ReputationFlagsLikeCpp,
    ReputationRankLikeCpp,
};

/// C++ `FactionState` (`ReputationMgr.h`), one entry of `_factions`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerFactionStateLikeCpp {
    pub faction_id: u32,
    /// C++ `FactionState::ReputationListID`, the stable client-array key.
    pub reputation_list_id: u32,
    pub standing: i32,
    pub visual_standing_increase: i32,
    pub flags: ReputationFlagsLikeCpp,
    pub need_send: bool,
    pub need_save: bool,
}

/// C++ value-initializes a `FactionState` with no flags before filling it in.
impl Default for PlayerFactionStateLikeCpp {
    fn default() -> Self {
        Self {
            faction_id: 0,
            reputation_list_id: 0,
            standing: 0,
            visual_standing_increase: 0,
            flags: ReputationFlagsLikeCpp::NONE,
            need_send: false,
            need_save: false,
        }
    }
}

impl PlayerFactionStateLikeCpp {
    /// C++ `ReputationMgr::Initialize` seeds a faction at zero standing and
    /// marks it for both the client and the database.
    #[must_use]
    pub fn new_like_cpp(
        faction_id: u32,
        reputation_list_id: u32,
        flags: ReputationFlagsLikeCpp,
    ) -> Self {
        Self {
            faction_id,
            reputation_list_id,
            standing: 0,
            visual_standing_increase: 0,
            flags,
            need_send: true,
            need_save: true,
        }
    }
}

/// C++ `ReputationMgr::_visibleFactionCount` and its honored/revered/exalted
/// siblings.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReputationRankCountersLikeCpp {
    pub visible: u8,
    pub honored: u8,
    pub revered: u8,
    pub exalted: u8,
}

/// The Player-owned reputation state C++ keeps inside `ReputationMgr`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerReputationStateLikeCpp {
    /// C++ `ReputationMgr::_factions`, keyed by `ReputationListID`.
    factions: BTreeMap<u32, PlayerFactionStateLikeCpp>,
    /// C++ `ReputationMgr::_forcedReactions`, keyed by faction id.
    forced_reactions: BTreeMap<u32, ReputationRankLikeCpp>,
    rank_counters: ReputationRankCountersLikeCpp,
    /// C++ `ReputationMgr::_sendFactionIncreased`.
    send_faction_increased: bool,
}

impl PlayerReputationStateLikeCpp {
    /// C++ `ReputationMgr::GetState(RepListID)`.
    #[must_use]
    pub fn faction_like_cpp(&self, reputation_list_id: u32) -> Option<&PlayerFactionStateLikeCpp> {
        self.factions.get(&reputation_list_id)
    }

    /// Mutable access to one identified faction's own `FactionState`.
    ///
    /// C++ writes those members directly through the same lookup
    /// (`_factions.find(id)->second`); the operation names the faction it
    /// changes and can reach no other Player state.
    pub fn faction_mut_like_cpp(
        &mut self,
        reputation_list_id: u32,
    ) -> Option<&mut PlayerFactionStateLikeCpp> {
        self.factions.get_mut(&reputation_list_id)
    }

    #[must_use]
    pub fn contains_faction_like_cpp(&self, reputation_list_id: u32) -> bool {
        self.factions.contains_key(&reputation_list_id)
    }

    /// Every faction state in `ReputationListID` order, which is the order C++
    /// sends and saves them in.
    pub fn factions_like_cpp(&self) -> impl Iterator<Item = &PlayerFactionStateLikeCpp> {
        self.factions.values()
    }

    pub fn factions_mut_like_cpp(
        &mut self,
    ) -> impl Iterator<Item = &mut PlayerFactionStateLikeCpp> {
        self.factions.values_mut()
    }

    #[must_use]
    pub fn faction_count_like_cpp(&self) -> usize {
        self.factions.len()
    }

    /// C++ `ReputationMgr::Initialize` inserting one seeded `FactionState`.
    pub fn insert_faction_like_cpp(&mut self, state: PlayerFactionStateLikeCpp) {
        self.factions.insert(state.reputation_list_id, state);
    }

    /// C++ `ReputationMgr::Initialize` clearing `_factions` and its counters
    /// before reseeding them.
    pub fn clear_factions_like_cpp(&mut self) {
        self.factions.clear();
        self.rank_counters = ReputationRankCountersLikeCpp::default();
        self.send_faction_increased = false;
    }

    /// C++ clamps every standing into `[Reputation_Bottom, Reputation_Cap]`
    /// before storing it.
    pub fn set_standing_like_cpp(&mut self, reputation_list_id: u32, standing: i32) -> bool {
        let Some(faction) = self.factions.get_mut(&reputation_list_id) else {
            return false;
        };
        faction.standing = standing.clamp(REPUTATION_BOTTOM_LIKE_CPP, REPUTATION_CAP_LIKE_CPP);
        true
    }

    /// C++ `ReputationMgr::GetForcedRankIfAny`.
    #[must_use]
    pub fn forced_reaction_like_cpp(&self, faction_id: u32) -> Option<ReputationRankLikeCpp> {
        self.forced_reactions.get(&faction_id).copied()
    }

    /// C++ `ReputationMgr::ApplyForceReaction`.
    pub fn set_forced_reaction_like_cpp(
        &mut self,
        faction_id: u32,
        rank: Option<ReputationRankLikeCpp>,
    ) {
        match rank {
            Some(rank) => {
                self.forced_reactions.insert(faction_id, rank);
            }
            None => {
                self.forced_reactions.remove(&faction_id);
            }
        }
    }

    /// Replace the whole forced-reaction set, as a faction-change or admin
    /// reload does.
    pub fn replace_forced_reactions_like_cpp(
        &mut self,
        reactions: impl IntoIterator<Item = (u32, ReputationRankLikeCpp)>,
    ) {
        self.forced_reactions = reactions.into_iter().collect();
    }

    pub fn forced_reactions_like_cpp(
        &self,
    ) -> impl Iterator<Item = (u32, ReputationRankLikeCpp)> + '_ {
        self.forced_reactions
            .iter()
            .map(|(faction_id, rank)| (*faction_id, *rank))
    }

    #[must_use]
    pub fn rank_counters_like_cpp(&self) -> ReputationRankCountersLikeCpp {
        self.rank_counters
    }

    pub fn set_rank_counters_like_cpp(&mut self, counters: ReputationRankCountersLikeCpp) {
        self.rank_counters = counters;
    }

    /// C++ increments/decrements one cached rank counter as a standing crosses
    /// its threshold; the counters never wrap.
    pub fn adjust_rank_counter_like_cpp(&mut self, rank: ReputationRankCounterLikeCpp, delta: i8) {
        let counter = match rank {
            ReputationRankCounterLikeCpp::Visible => &mut self.rank_counters.visible,
            ReputationRankCounterLikeCpp::Honored => &mut self.rank_counters.honored,
            ReputationRankCounterLikeCpp::Revered => &mut self.rank_counters.revered,
            ReputationRankCounterLikeCpp::Exalted => &mut self.rank_counters.exalted,
        };
        *counter = if delta >= 0 {
            counter.saturating_add(delta.unsigned_abs())
        } else {
            counter.saturating_sub(delta.unsigned_abs())
        };
    }

    #[must_use]
    pub fn send_faction_increased_like_cpp(&self) -> bool {
        self.send_faction_increased
    }

    pub fn set_send_faction_increased_like_cpp(&mut self, value: bool) {
        self.send_faction_increased = value;
    }
}

/// Which cached rank counter one transition moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReputationRankCounterLikeCpp {
    Visible,
    Honored,
    Revered,
    Exalted,
}

impl crate::Player {
    /// C++ `Player::GetReputationMgr() const` (`Player.h:2001`).
    #[must_use]
    pub fn reputation_like_cpp(&self) -> &PlayerReputationStateLikeCpp {
        &self.gameplay_state().reputation
    }

    /// C++ `Player::GetReputationMgr()` (`Player.h:2000`).
    pub fn reputation_mut_like_cpp(&mut self) -> &mut PlayerReputationStateLikeCpp {
        &mut self.gameplay_state_mut().reputation
    }
}
