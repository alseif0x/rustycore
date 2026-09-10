//! Existing item-addon catalog query capability; adapter ownership remains unchanged.
//! Mechanical relocation from lib.rs in #578; public crate-root paths are retained.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemTemplateAddonCatalogRequestLikeCpp {
    pub item_entry: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemTemplateAddonMoneyRowLikeCpp {
    pub min_money: Option<u32>,
    pub max_money: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemTemplateAddonMoneyOutcomeLikeCpp {
    Found(ItemTemplateAddonMoneyRowLikeCpp),
    Missing,
    Failed { reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ItemTemplateAddonLootMetadataRowLikeCpp {
    pub flags_cu: u32,
    pub quest_log_item_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemTemplateAddonLootMetadataOutcomeLikeCpp {
    Found(ItemTemplateAddonLootMetadataRowLikeCpp),
    Missing,
    Failed { reason: String },
}

/// Transitional on-demand World catalog reads. C++ loads the complete table
/// into `ObjectMgr` during startup; #489 only removes concrete persistence
/// from gameplay while preserving Rust's current query timing and outcomes.
pub trait ItemTemplateAddonCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_item_template_addon_money_like_cpp<'a>(
        &'a self,
        request: ItemTemplateAddonCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, ItemTemplateAddonMoneyOutcomeLikeCpp>;

    fn load_item_template_addon_loot_metadata_like_cpp<'a>(
        &'a self,
        request: ItemTemplateAddonCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, ItemTemplateAddonLootMetadataOutcomeLikeCpp>;
}
