//! Save and load plans for represented item and inventory state.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    /// Apply a previously validated and committed C++ `RemoveItem` +
    /// `StoreItem`/`BankItem` relocation to the session-owned inventory.
    ///
    /// `character_inventory.bag` persistence is handled by the caller. This
    /// method deliberately has no fallible database work so memory cannot move
    /// ahead of the transaction.
    pub(crate) fn apply_committed_inventory_item_relocation_like_cpp(
        &mut self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        moved_count: u32,
    ) -> bool {
        wow_world_application::InventoryCommittedRelocationCxLikeCpp::new(
            &mut self.inventory,
            self.core.owned_inventory_access_like_cpp(),
            self.catalogs.items.store.as_ref(),
            self.catalogs.items.stats_store.as_ref(),
        )
        .apply_committed_inventory_item_relocation_like_cpp(
            source_bag,
            source_slot,
            destination_bag,
            destination_slot,
            moved_count,
        )
    }
    /// Publish a committed C++ real swap after both database positions were
    /// replaced in one transaction. Both positions must still contain the
    /// pre-commit items; all runtime mutation is therefore infallible after
    /// this preflight.
    pub(crate) fn apply_committed_inventory_item_swap_like_cpp(
        &mut self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
    ) -> bool {
        wow_world_application::InventoryCommittedSwapCxLikeCpp::new(
            &mut self.inventory,
            self.core.owned_inventory_access_like_cpp(),
            self.catalogs.items.store.as_ref(),
            self.catalogs.items.stats_store.as_ref(),
        )
        .apply_committed_inventory_item_swap_like_cpp(
            source_bag,
            source_slot,
            destination_bag,
            destination_slot,
        )
    }
    /// Remove a source item after its complete stack was merged into existing
    /// destination stacks by a committed storage transaction.
    pub(crate) fn apply_committed_inventory_item_removal_like_cpp(
        &mut self,
        source_bag: u8,
        source_slot: u8,
        item_guid: ObjectGuid,
    ) -> bool {
        wow_world_application::InventoryCommittedRelocationCxLikeCpp::new(
            &mut self.inventory,
            self.core.owned_inventory_access_like_cpp(),
            self.catalogs.items.store.as_ref(),
            self.catalogs.items.stats_store.as_ref(),
        )
        .apply_committed_inventory_item_removal_like_cpp(
            source_bag,
            source_slot,
            item_guid,
        )
    }
    pub fn set_stored_item_money_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::StoredItemMoneyPersistencePortLikeCpp>,
    ) {
        crate::session::cx_inventory(self).set_stored_item_money_persistence_port_like_cpp(port)
    }
    pub fn set_item_template_addon_catalog_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::ItemTemplateAddonCatalogPersistencePortLikeCpp>,
    ) {
        crate::session::cx_inventory(self)
            .set_item_template_addon_catalog_persistence_port_like_cpp(port)
    }
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
    pub(crate) fn plan_inventory_swap_preflight_like_cpp(
        &self,
        src: u16,
        dst: u16,
    ) -> Option<SwapItemPreflightPlan> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.plan_inventory_swap_preflight_like_cpp(hub, src, dst)
    }
    pub fn plan_store_new_direct_inventory_item(
        &self,
        entry_id: u32,
        count: u32,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
        self.plan_store_new_direct_inventory_item_at(entry_id, count, NULL_BAG, NULL_SLOT)
    }
    pub fn plan_store_new_direct_inventory_item_at(
        &self,
        entry_id: u32,
        count: u32,
        bag: u8,
        slot: u8,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
        self.plan_store_direct_inventory_item_like_cpp(
            entry_id,
            count,
            bag,
            slot,
            None,
            false,
            &[],
            &[],
        )
    }
    pub(crate) fn plan_store_new_direct_inventory_item_with_overlays_like_cpp(
        &self,
        entry_id: u32,
        count: u32,
        overlays: &[DirectInventoryStorageOverlayLikeCpp],
        vacated_positions: &[(u8, u8)],
    ) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
        self.plan_store_direct_inventory_item_like_cpp(
            entry_id,
            count,
            NULL_BAG,
            NULL_SLOT,
            None,
            false,
            overlays,
            vacated_positions,
        )
    }
    pub(crate) fn plan_store_existing_direct_inventory_item_like_cpp(
        &self,
        source_slot: u8,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
        self.plan_store_existing_inventory_item_like_cpp(INVENTORY_SLOT_BAG_0, source_slot)
    }
    pub(crate) fn plan_store_existing_inventory_item_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
        self.plan_store_existing_inventory_item_at_like_cpp(
            source_bag,
            source_slot,
            NULL_BAG,
            NULL_SLOT,
            false,
        )
    }
    pub(crate) fn plan_store_existing_inventory_item_at_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        swap: bool,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
        let conditions = self.player_condition_projection_cx_like_cpp();
        wow_world_application::InventoryMovePlanningCxLikeCpp::new(&conditions)
            .plan_store_existing_inventory_item_at_like_cpp(
                source_bag,
                source_slot,
                destination_bag,
                destination_slot,
                swap,
            )
    }
    fn plan_store_direct_inventory_item_like_cpp(
        &self,
        entry_id: u32,
        count: u32,
        bag: u8,
        slot: u8,
        source_item: Option<&Item>,
        swap: bool,
        overlays: &[DirectInventoryStorageOverlayLikeCpp],
        vacated_positions: &[(u8, u8)],
    ) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
        self.player_condition_projection_cx_like_cpp()
            .plan_store_direct_inventory_item_like_cpp(
                self.core
                    .inventory_valuation_access_like_cpp()
                    .realm_id_like_cpp(),
                entry_id,
                count,
                bag,
                slot,
                source_item,
                swap,
                overlays,
                vacated_positions,
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
    pub fn send_new_item_plan(&self, plan: &SendNewItemPlan) {
        let packet = crate::session::item_push_result_from_send_new_item_plan(plan);
        if plan.delivery == SendNewItemDelivery::GroupBroadcast {
            use wow_packet::ServerPacket;

            if self.broadcast_item_push_result_to_group(packet.to_bytes()) {
                return;
            }
        }

        // C++ `Player::SendNewItem` uses `SendDirectMessage`; opcode routing
        // places `SMSG_ITEM_PUSH_RESULT` on CONNECTION_TYPE_REALM even while
        // the inventory object updates remain on the instance connection.
        self.send_packet_realm(&packet);
    }
    pub fn send_item_time_update_plan(&self, update: &PlayerItemTimeUpdate) {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.send_item_time_update_plan(hub, update)
    }
    pub fn send_item_time_update_plans(&self, updates: &[PlayerItemTimeUpdate]) {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.send_item_time_update_plans(hub, updates)
    }
}

impl crate::session::InventoryCxRef<'_> {
    pub(crate) fn stored_item_money_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::StoredItemMoneyPersistencePortLikeCpp>> {
        self.lifecycle
            .stored_item_money_persistence_port_like_cpp()
            .cloned()
    }

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

impl crate::session::InventoryCx<'_> {
    pub fn set_stored_item_money_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::StoredItemMoneyPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_stored_item_money_persistence_port_like_cpp(port);
    }

    pub fn set_item_template_addon_catalog_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::ItemTemplateAddonCatalogPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_item_template_addon_catalog_persistence_port_like_cpp(port);
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/player_items/persistence/f3_shims.rs"]
mod f3_shims;
