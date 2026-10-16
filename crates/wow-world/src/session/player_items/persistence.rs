//! Save and load plans for represented item and inventory state.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    pub(crate) fn apply_committed_new_inventory_item_at_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        inventory_item: InventoryItem,
        item_object: Item,
    ) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.apply_committed_new_inventory_item_at_like_cpp(
            &mut hub,
            bag,
            slot,
            inventory_item,
            item_object,
        )
    }
    #[cfg(test)]
    pub(crate) fn set_loot_item_store_test_commit_gate_like_cpp(
        &mut self,
        gate: Arc<tokio::sync::Notify>,
    ) {
        self.loot
            .set_loot_item_store_test_commit_gate_like_cpp(gate);
    }
}

impl crate::session::InventoryCxRef<'_> {
    pub(crate) fn item_template_addon_catalog_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::ItemTemplateAddonCatalogPersistencePortLikeCpp>> {
        self.lifecycle
            .item_template_addon_catalog_persistence_port_like_cpp()
            .cloned()
    }

    pub(crate) fn begin_durable_item_loot_persistence_like_cpp(
        &self,
    ) -> DurableItemLootPersistenceGuardLikeCpp {
        self.lifecycle
            .begin_durable_item_loot_persistence_like_cpp()
    }

    pub(crate) async fn wait_for_durable_item_loot_persistence_like_cpp(&self) {
        self.lifecycle
            .wait_for_durable_item_loot_persistence_like_cpp()
            .await;
    }

    pub(crate) fn take_durable_item_loot_completions_like_cpp(
        &self,
    ) -> Vec<DurableItemLootCompletionLikeCpp> {
        self.lifecycle.take_durable_item_loot_completions_like_cpp()
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/player_items/persistence/f3_shims.rs"]
mod f3_shims;
