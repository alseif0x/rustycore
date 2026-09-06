//! Player-name query projections and the read-only capability.
//! Mechanical relocation from lib.rs in #578; public crate-root paths are retained.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerNameQueryRequestLikeCpp {
    pub player_guid_counter: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerNameQueryRowLikeCpp {
    pub name: String,
    pub race: u8,
    pub class: u8,
    pub sex: u8,
    pub level: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerNameQueryOutcomeLikeCpp {
    Found(PlayerNameQueryRowLikeCpp),
    Missing,
    Failed { reason: String },
}

/// Transitional on-demand Character read. C++ serves this identity from
/// `CharacterCache`; #486 owns convergence onto the target-account cache
/// semantics without mixing that behavior correction into #487.
pub trait PlayerNameQueryPersistencePortLikeCpp: Send + Sync {
    fn load_player_name_like_cpp<'a>(
        &'a self,
        request: PlayerNameQueryRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerNameQueryOutcomeLikeCpp>;
}
