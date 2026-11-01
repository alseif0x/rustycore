//! C++ `game/Reputation` runtime state.

pub mod mgr;

pub use mgr::{
    FactionStateLikeCpp, ForcedReactionsLikeCpp, RepListIdLikeCpp, ReputationMgrLikeCpp,
    ReputationMgrMutLikeCpp, ReputationMgrRefLikeCpp, ReputationRankCounterLikeCpp,
    ReputationRankCountersLikeCpp, reputation_to_rank_like_cpp,
};
