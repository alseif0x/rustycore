//! SQLx-free World catalog contract for C++ vendor interaction.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, PartialEq)]
pub struct VendorCatalogRowLikeCpp {
    pub item_id: i32,
    pub max_count: i32,
    pub extended_cost: u32,
    pub item_type: u8,
    pub buy_price: u64,
    pub max_durability: u32,
    pub buy_count: u32,
    pub do_not_filter: bool,
    pub incr_time: u32,
    pub player_condition_id: u32,
    pub has_vendor_conditions: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VendorCatalogOutcomeLikeCpp<T> {
    Loaded(T),
    Missing,
    Failed { reason: String },
}

pub trait VendorCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_vendor_rows_like_cpp(
        &self,
        root_entry: u32,
        vendor_entry: u32,
    ) -> PersistenceFutureLikeCpp<'_, VendorCatalogOutcomeLikeCpp<Vec<VendorCatalogRowLikeCpp>>>;

    fn load_creature_entry_by_spawn_like_cpp(
        &self,
        spawn_guid: u64,
    ) -> PersistenceFutureLikeCpp<'_, VendorCatalogOutcomeLikeCpp<u32>>;

    fn load_item_sell_price_like_cpp(
        &self,
        item_entry: u32,
    ) -> PersistenceFutureLikeCpp<'_, VendorCatalogOutcomeLikeCpp<u64>>;
}
