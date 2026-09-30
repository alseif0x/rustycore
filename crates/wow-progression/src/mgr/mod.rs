//! The C++ `ReputationMgr` rules, operating on Player-owned state.
//!
//! The state lives with the canonical Player; this module owns reputation rules
//! and packet-neutral publication values. Application code owns wire presentation.

use std::collections::BTreeMap;

use wow_constants::reputation::{
    FACTION_COUNT_LIKE_CPP, REPUTATION_BOTTOM_LIKE_CPP, REPUTATION_CAP_LIKE_CPP,
    ReputationFlagsLikeCpp, ReputationRankLikeCpp, reputation_rank_from_standing_like_cpp,
};
use wow_data_model::reputation::{
    FactionEntry, MAX_SPILLOVER_FACTIONS_LIKE_CPP, RepSpilloverTemplateLikeCpp,
};
use wow_entities::PlayerReputationStateLikeCpp;

mod borrowed;
mod catalog;
mod publication;
mod state_1;
mod state_2_ops_1;
mod state_2_ops_2;
mod state_3;

#[allow(unused_imports)]
pub use {
    borrowed::*, catalog::*, publication::*, state_1::*, state_2_ops_1::*, state_2_ops_2::*,
    state_3::*,
};

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
