use super::LootState;
#[cfg(any(test, feature = "test-fixtures"))]
use std::sync::{Arc, atomic::AtomicUsize};
use wow_constants::ItemFieldFlags;
use wow_entities::{INVENTORY_SLOT_BAG_0, Item, is_bag_pos, make_item_pos};
use wow_world_core::session::HubRef;
#[cfg(any(test, feature = "test-fixtures"))]
use tokio::sync::Notify;

impl LootState {
    /// Apply the item state established by C++ `Player::StoreNewItem` and
    /// `_StoreItem` before the item is persisted or sent to the client.
    pub fn apply_stored_new_item_flags_like_cpp(
        &self,
        hub: HubRef<'_>,
        item_id: u32,
        slot: u8,
        item: &mut Item,
    ) {
        if let Some(template) = hub.catalogs.item_storage_template(item_id) {
            item.set_bonding(template.bonding);
        }
        item.set_item_flag(ItemFieldFlags::NEW_ITEM);
        item.bind_if_stored(is_bag_pos(make_item_pos(INVENTORY_SLOT_BAG_0, slot)));
    }

    pub fn stored_new_item_dynamic_flags_like_cpp(
        &self,
        hub: HubRef<'_>,
        item_id: u32,
        slot: u8,
    ) -> u32 {
        let mut item = Item::new(0);
        self.apply_stored_new_item_flags_like_cpp(hub, item_id, slot, &mut item);
        item.item_flags_bits()
    }

    /// C++ `_StoreItem` binds the destination object before incrementing an
    /// existing stack. Unlike `StoreNewItem`, that historical object must not
    /// acquire `ITEM_FIELD_FLAG_NEW_ITEM` merely because more items arrived.
    pub fn stored_existing_item_dynamic_flags_like_cpp(
        &self,
        hub: HubRef<'_>,
        item_id: u32,
        slot: u8,
        existing: &Item,
    ) -> u32 {
        let mut planned = existing.clone();
        if let Some(template) = hub.catalogs.item_storage_template(item_id) {
            planned.set_bonding(template.bonding);
        }
        planned.bind_if_stored(is_bag_pos(make_item_pos(INVENTORY_SLOT_BAG_0, slot)));
        planned.item_flags_bits()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn loot_item_store_test_grants_like_cpp(&self) -> Option<Arc<AtomicUsize>> {
        self.loot_item_store_test_grants_like_cpp.clone()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_loot_item_store_test_seam_like_cpp(
        &mut self,
        grants: Arc<AtomicUsize>,
        success: bool,
    ) {
        self.loot_item_store_test_grants_like_cpp = Some(grants);
        self.loot_item_store_test_success_like_cpp = success;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn loot_item_store_test_success_like_cpp(&self) -> bool {
        self.loot_item_store_test_success_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn loot_item_store_test_commit_gate_like_cpp(&self) -> Option<Arc<Notify>> {
        self.loot_item_store_test_commit_gate_like_cpp.clone()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_loot_item_store_test_commit_gate_like_cpp(&mut self, gate: Arc<Notify>) {
        self.loot_item_store_test_commit_gate_like_cpp = Some(gate);
    }
}
