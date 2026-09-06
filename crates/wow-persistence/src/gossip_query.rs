//! Existing gossip query projections and capability; distinct from startup catalog assembly.
//! Mechanical relocation from lib.rs in #578; public crate-root paths are retained.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GossipCreatureMenuRequestLikeCpp {
    pub creature_entry: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GossipMenuCatalogRequestLikeCpp {
    pub menu_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GossipNpcTextCatalogRequestLikeCpp {
    pub npc_text_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GossipBroadcastTextLocaleRequestLikeCpp {
    pub broadcast_text_id: u32,
    pub locale: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GossipMenuOptionCatalogRowLikeCpp {
    pub menu_id: u32,
    pub gossip_option_id: i32,
    pub option_id: u32,
    pub option_npc: u8,
    pub option_text: String,
    pub option_broadcast_text_id: u32,
    pub language: u32,
    pub flags: i32,
    pub action_menu_id: u32,
    pub action_poi_id: u32,
    pub gossip_npc_option_id: Option<i32>,
    pub box_coded: bool,
    pub box_money: u32,
    pub box_text: String,
    pub box_broadcast_text_id: u32,
    pub spell_id: Option<i32>,
    pub override_icon_id: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GossipCatalogReadOutcomeLikeCpp<T> {
    Found(T),
    Missing,
    Failed { reason: String },
}

/// Transitional, SQLx-free view of Rust's on-demand gossip reads. C++ loads
/// these tables into `ObjectMgr` during startup; #491 preserves the current
/// per-interaction deadlines and query order while removing database handles
/// and row decoding from gameplay.
pub trait GossipCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_creature_gossip_menu_id_like_cpp<'a>(
        &'a self,
        request: GossipCreatureMenuRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, GossipCatalogReadOutcomeLikeCpp<u32>>;

    fn load_gossip_menu_text_ids_like_cpp<'a>(
        &'a self,
        request: GossipMenuCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, GossipCatalogReadOutcomeLikeCpp<Vec<u32>>>;

    fn load_npc_text_broadcast_id_like_cpp<'a>(
        &'a self,
        request: GossipNpcTextCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, GossipCatalogReadOutcomeLikeCpp<i32>>;

    fn load_gossip_menu_options_like_cpp<'a>(
        &'a self,
        request: GossipMenuCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<
        'a,
        GossipCatalogReadOutcomeLikeCpp<Vec<GossipMenuOptionCatalogRowLikeCpp>>,
    >;

    fn load_broadcast_text_locale_like_cpp<'a>(
        &'a self,
        request: GossipBroadcastTextLocaleRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, GossipCatalogReadOutcomeLikeCpp<String>>;
}
