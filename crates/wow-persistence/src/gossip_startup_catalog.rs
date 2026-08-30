//! SQLx-free source contract for the represented startup gossip catalogs.

use crate::{GossipMenuOptionCatalogRowLikeCpp, PersistenceFutureLikeCpp};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GossipMenuPersistenceRowLikeCpp {
    pub menu_id: u32,
    pub text_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GossipMenuOptionLocalePersistenceRowLikeCpp {
    pub menu_id: u32,
    pub option_id: u32,
    pub locale: String,
    pub option_text: String,
    pub box_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GossipMenuAddonPersistenceRowLikeCpp {
    pub menu_id: u32,
    pub friendship_faction_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GossipStartupCatalogLoadOutcomeLikeCpp<T> {
    Loaded(Vec<T>),
    Failed { reason: String },
}

/// Four independent startup reads which jointly feed the represented gossip
/// store. Their separation preserves the existing fail-fast query order.
pub trait GossipStartupCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_menu_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        GossipStartupCatalogLoadOutcomeLikeCpp<GossipMenuPersistenceRowLikeCpp>,
    >;

    fn load_menu_option_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        GossipStartupCatalogLoadOutcomeLikeCpp<GossipMenuOptionCatalogRowLikeCpp>,
    >;

    fn load_menu_option_locale_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        GossipStartupCatalogLoadOutcomeLikeCpp<GossipMenuOptionLocalePersistenceRowLikeCpp>,
    >;

    fn load_menu_addon_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        GossipStartupCatalogLoadOutcomeLikeCpp<GossipMenuAddonPersistenceRowLikeCpp>,
    >;
}
