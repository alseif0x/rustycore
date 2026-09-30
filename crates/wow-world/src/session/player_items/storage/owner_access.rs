//! owner access for the existing storage owner.

use super::*;

impl WorldSession {
    pub(in crate::session) fn direct_inventory_player_snapshot(&self) -> Option<Player> {
        if let Some(player) = self.with_owned_player_like_cpp(Clone::clone) {
            return Some(player);
        }

        #[cfg(any(test, feature = "test-fixtures"))]
        if self.ownerless_inventory_fallback_enabled_for_test() {
            let player_guid = self.player_guid()?;
            let mut player = Player::new(None, false);
            player
                .unit_mut()
                .world_mut()
                .object_mut()
                .create(player_guid);
            player.set_inventory_slot_count(self.resolved_player_inventory_slot_count_like_cpp()?);
            player.set_bank_bag_slot_count(self.resolved_player_bank_bag_slot_count_like_cpp()?);

            let item_objects = self.resolved_inventory_item_objects_like_cpp()?;
            for (&slot, item) in &self.resolved_inventory_items_like_cpp()? {
                if (slot as usize) < PLAYER_SLOT_END && !wow_entities::is_buyback_slot(slot) {
                    let _ = player.store_top_level_item(slot, item.guid);
                    if is_represented_bag_slot(slot)
                        && item_objects.contains_key(&item.guid)
                        && let Some(template) = self.item_storage_template(item.entry_id)
                        && template.container_slots > 0
                    {
                        let _ =
                            player.register_bag_storage(slot, item.guid, template.container_slots);
                    }
                }
            }
            return Some(player);
        }

        None
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    fn mirror_player_inventory_runtime_to_legacy_like_cpp(
        &mut self,
        inventory: &PlayerInventoryRuntime,
    ) {
        self.player_item_test_fixture_like_cpp.inventory_items =
            inventory.inventory_items().clone();
        self.player_item_test_fixture_like_cpp.buyback_items = inventory.buyback_items().clone();
        self.player_item_test_fixture_like_cpp.buyback_price = *inventory.buyback_price();
        self.player_item_test_fixture_like_cpp.buyback_timestamp = *inventory.buyback_timestamp();
        self.player_item_test_fixture_like_cpp.current_buyback_slot =
            inventory.current_buyback_slot();
        self.inventory_item_objects = inventory.item_objects().clone();
    }
    pub(crate) fn mutate_player_inventory_runtime_like_cpp<R>(
        &mut self,
        update: impl FnOnce(&mut PlayerInventoryRuntime) -> R,
    ) -> Option<R> {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        let mut update = Some(update);
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.player_handle_like_cpp.is_none() {
            let mut inventory = PlayerInventoryRuntime::default();
            inventory.inventory_items_mut().extend(
                self.player_item_test_fixture_like_cpp
                    .inventory_items
                    .clone(),
            );
            inventory
                .buyback_items_mut()
                .extend(self.player_item_test_fixture_like_cpp.buyback_items.clone());
            *inventory.buyback_price_mut() = self.player_item_test_fixture_like_cpp.buyback_price;
            *inventory.buyback_timestamp_mut() =
                self.player_item_test_fixture_like_cpp.buyback_timestamp;
            inventory.set_current_buyback_slot(
                self.player_item_test_fixture_like_cpp.current_buyback_slot,
            );
            inventory
                .item_objects_mut()
                .extend(self.inventory_item_objects.clone());
            let result =
                update.take().expect("inventory mutation closure runs once")(&mut inventory);
            self.mirror_player_inventory_runtime_to_legacy_like_cpp(&inventory);
            return Some(result);
        }
        let result = self.with_owned_player_mut_like_cpp(|player| {
            update.take().expect("inventory mutation closure runs once")(
                player.inventory_runtime_mut_like_cpp(),
            )
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if result.is_some()
            && let Some(inventory) = self
                .with_owned_player_like_cpp(|player| player.inventory_runtime_like_cpp().clone())
        {
            self.mirror_player_inventory_runtime_to_legacy_like_cpp(&inventory);
        }
        result
    }
    pub(in crate::session) fn resolved_player_inventory_runtime_like_cpp(
        &self,
    ) -> Option<PlayerInventoryRuntime> {
        if let Some(inventory) =
            self.with_owned_player_like_cpp(|player| player.inventory_runtime_like_cpp().clone())
        {
            return Some(inventory);
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.player_handle_like_cpp.is_none() {
            let mut inventory = PlayerInventoryRuntime::default();
            inventory.inventory_items_mut().extend(
                self.player_item_test_fixture_like_cpp
                    .inventory_items
                    .clone(),
            );
            inventory
                .buyback_items_mut()
                .extend(self.player_item_test_fixture_like_cpp.buyback_items.clone());
            *inventory.buyback_price_mut() = self.player_item_test_fixture_like_cpp.buyback_price;
            *inventory.buyback_timestamp_mut() =
                self.player_item_test_fixture_like_cpp.buyback_timestamp;
            inventory.set_current_buyback_slot(
                self.player_item_test_fixture_like_cpp.current_buyback_slot,
            );
            inventory
                .item_objects_mut()
                .extend(self.inventory_item_objects.clone());
            return Some(inventory);
        }
        None
    }
    pub(crate) fn resolved_inventory_items_like_cpp(&self) -> Option<HashMap<u8, InventoryItem>> {
        self.resolved_player_inventory_runtime_like_cpp()
            .map(|inventory| inventory.inventory_items().clone())
    }
    pub(crate) fn resolved_inventory_item_objects_like_cpp(
        &self,
    ) -> Option<HashMap<ObjectGuid, Item>> {
        self.resolved_player_inventory_runtime_like_cpp()
            .map(|inventory| inventory.item_objects().clone())
    }
    pub(crate) fn resolved_inventory_item_like_cpp(&self, slot: u8) -> Option<InventoryItem> {
        self.resolved_player_inventory_runtime_like_cpp()?
            .inventory_items()
            .get(&slot)
            .cloned()
    }
    pub(crate) fn resolved_inventory_item_object_like_cpp(&self, guid: ObjectGuid) -> Option<Item> {
        self.resolved_player_inventory_runtime_like_cpp()?
            .item_objects()
            .get(&guid)
            .cloned()
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn inventory_items_like_cpp(&self) -> &HashMap<u8, InventoryItem> {
        &self.player_item_test_fixture_like_cpp.inventory_items
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn inventory_item_objects_like_cpp(&self) -> &HashMap<ObjectGuid, Item> {
        &self.inventory_item_objects
    }
}
