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
    /// C++ `CharacterCacheEntry::AccountId` for the target character.
    pub account_id: u32,
    /// Battle.net account owning the target game account. Zero means that the
    /// auth database has no link, matching `AccountMgr::GetIdByGameAccount`.
    pub battlenet_account_id: u32,
    /// C++ `CharacterCacheEntry::IsDeleted`.
    pub is_deleted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerNameQueryOutcomeLikeCpp {
    Found(PlayerNameQueryRowLikeCpp),
    Missing,
    Failed { reason: String },
}

/// Read-only target identity served from the C++-equivalent character cache.
/// The adapter is warmed during world-server startup; packet handlers never
/// issue a full character-row query.
pub trait PlayerNameQueryPersistencePortLikeCpp: Send + Sync {
    fn load_player_name_like_cpp<'a>(
        &'a self,
        request: PlayerNameQueryRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerNameQueryOutcomeLikeCpp>;
}
