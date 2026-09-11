//! The C++ `ReputationMgr` rules, operating on Player-owned state.
//!
//! The state lives with the canonical Player; these rules stay here, where the
//! catalogs and packet builders they need are allowed. See `borrowed` (#735).

use std::collections::BTreeMap;

use wow_data::CurrencyTypesStore;
use wow_data::progression_rewards::{
    FactionEntry, FactionStore, FriendshipRepReactionStore, ParagonReputationStore,
};
use wow_data::reputation::{
    MAX_SPILLOVER_FACTIONS_LIKE_CPP, RepSpilloverTemplateLikeCpp, ReputationFlagsLikeCpp,
    ReputationRankLikeCpp,
};
use wow_entities::PlayerReputationStateLikeCpp;
use wow_packet::packets::reputation::{
    FACTION_COUNT_LIKE_CPP, FactionStandingData as FactionStandingDataPacketLikeCpp,
    ForcedReaction as ForcedReactionPacketLikeCpp,
    InitializeFactions as InitializeFactionsPacketLikeCpp,
    SetFactionStanding as SetFactionStandingPacketLikeCpp,
    SetForcedReactions as SetForcedReactionsPacketLikeCpp,
};

mod borrowed;
mod state_1;
mod state_2_ops_1;
mod state_2_ops_2;
mod state_3;

#[allow(unused_imports)]
pub use {borrowed::*, state_1::*, state_2_ops_1::*, state_2_ops_2::*, state_3::*};

#[cfg(test)]
#[path = "mgr/tests/mod.rs"]
mod tests;
