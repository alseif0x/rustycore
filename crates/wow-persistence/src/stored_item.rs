//! SQLx-free persistence contract for durable Item-owned state.
//!
//! The capability follows C++ Item persistence boundaries: wrapped gifts,
//! inventory destruction/count changes and item-container loot. Gameplay owns
//! validation and runtime publication; the adapter owns statement expansion,
//! row decoding and transaction outcome classification.

use crate::{PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WrappedGiftPersistenceRowLikeCpp {
    pub entry: u32,
    pub flags: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredItemLootPersistenceRowLikeCpp {
    pub item_id: u32,
    pub count: u32,
    pub item_index: u32,
    pub follow_loot_rules: bool,
    pub free_for_all: bool,
    pub blocked: bool,
    pub counted: bool,
    pub under_threshold: bool,
    pub needs_quest: bool,
    pub random_properties_id: i32,
    pub random_properties_seed: i32,
    pub context: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoredItemLoadOutcomeLikeCpp<T> {
    Loaded(T),
    Missing,
    Failed { reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InventoryItemDestroyPersistenceRequestLikeCpp {
    pub owner_guid: u64,
    pub item_guid: u64,
    pub expire_refund: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InventoryItemCountPersistenceRequestLikeCpp {
    pub item_guid: u64,
    pub count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WrappedGiftOpenPersistenceRequestLikeCpp {
    pub item_guid: u64,
    pub entry: u32,
    pub flags: u32,
    pub durability: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredItemLootSaveRequestLikeCpp {
    pub item_guid: u64,
    pub money: u32,
    /// Already filtered by gameplay using the C++ key/bag-family rules.
    pub items: Vec<StoredItemLootPersistenceRowLikeCpp>,
}

pub trait StoredItemPersistencePortLikeCpp: Send + Sync {
    fn load_wrapped_gift_like_cpp(
        &self,
        item_guid: u64,
    ) -> PersistenceFutureLikeCpp<'_, StoredItemLoadOutcomeLikeCpp<WrappedGiftPersistenceRowLikeCpp>>;

    fn open_wrapped_gift_like_cpp(
        &self,
        request: WrappedGiftOpenPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PersistenceOutcomeLikeCpp>;

    fn update_inventory_item_count_like_cpp(
        &self,
        request: InventoryItemCountPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PersistenceOutcomeLikeCpp>;

    fn destroy_inventory_item_like_cpp(
        &self,
        request: InventoryItemDestroyPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PersistenceOutcomeLikeCpp>;

    fn load_stored_item_money_like_cpp(
        &self,
        item_guid: u64,
    ) -> PersistenceFutureLikeCpp<'_, StoredItemLoadOutcomeLikeCpp<u32>>;

    fn load_stored_item_loot_like_cpp(
        &self,
        item_guid: u64,
    ) -> PersistenceFutureLikeCpp<
        '_,
        StoredItemLoadOutcomeLikeCpp<Vec<StoredItemLootPersistenceRowLikeCpp>>,
    >;

    fn save_stored_item_loot_like_cpp(
        &self,
        request: StoredItemLootSaveRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PersistenceOutcomeLikeCpp>;
}
