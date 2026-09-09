//! Per-player C++ `ReputationMgr` state foundation.
//!
//! This module owns only the direct `ReputationMgr.h` state shape. Initialization,
//! DB load/save, spillover and packet fanout are ported in later slices.

use std::collections::BTreeMap;

use wow_data::CurrencyTypesStore;
use wow_data::progression_rewards::{
    FactionEntry, FactionStore, FriendshipRepReactionStore, ParagonReputationStore,
};
use wow_data::reputation::{
    MAX_SPILLOVER_FACTIONS_LIKE_CPP, RepSpilloverTemplateLikeCpp, ReputationFlagsLikeCpp,
    ReputationRankLikeCpp,
};
use wow_entities::{PlayerGameplayState, PlayerReputationRecord};
use wow_packet::packets::reputation::{
    FACTION_COUNT_LIKE_CPP, FactionStandingData as FactionStandingDataPacketLikeCpp,
    ForcedReaction as ForcedReactionPacketLikeCpp,
    InitializeFactions as InitializeFactionsPacketLikeCpp,
    SetFactionStanding as SetFactionStandingPacketLikeCpp,
    SetForcedReactions as SetForcedReactionsPacketLikeCpp,
};

mod state_1;
mod state_3;
#[allow(unused_imports)]
pub use state_1::*;
pub use state_2_ops_1::*;
pub use state_2_ops_2::*;
#[allow(unused_imports)]
#[allow(unused_imports)]
pub use state_3::*;

mod state_2_ops_1;
mod state_2_ops_2;
#[cfg(test)]
#[path = "mgr/tests/mod.rs"]
mod tests;
