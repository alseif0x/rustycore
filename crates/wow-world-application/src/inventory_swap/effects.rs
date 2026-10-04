// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_constants::item::EnchantmentSlot;
use wow_core::ObjectGuid;
use wow_entities::{INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_END, PROFESSION_SLOT_END, ItemObjectUpdateLikeCpp};
use wow_world_core::session::{OwnedInventoryAccessLikeCpp, OwnedItemModifiersAccessLikeCpp, OwnedItemSetAccessLikeCpp, PacketPublicationAccessLikeCpp};
use wow_world_inventory::{InventoryState, ItemModsCatalogsViewLikeCpp};

/// Post-placement effects; construction retains only borrowed participants.
pub struct InventorySwapEffectsCxLikeCpp<'a> {
    inventory: &'a mut InventoryState,
    inventory_access: OwnedInventoryAccessLikeCpp<'a>,
    modifiers: OwnedItemModifiersAccessLikeCpp<'a>,
    item_sets: OwnedItemSetAccessLikeCpp<'a>,
    publication: PacketPublicationAccessLikeCpp<'a>,
    catalogs: ItemModsCatalogsViewLikeCpp<'a>,
    #[cfg(any(test, feature = "test-fixtures"))]
    player_level: &'a u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    shapeshift_form: &'a u32,
    consumer_test: bool,
}

impl<'a> InventorySwapEffectsCxLikeCpp<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        inventory: &'a mut InventoryState,
        inventory_access: OwnedInventoryAccessLikeCpp<'a>,
        modifiers: OwnedItemModifiersAccessLikeCpp<'a>,
        item_sets: OwnedItemSetAccessLikeCpp<'a>,
        publication: PacketPublicationAccessLikeCpp<'a>,
        catalogs: ItemModsCatalogsViewLikeCpp<'a>,
        #[cfg(any(test, feature = "test-fixtures"))] player_level: &'a u8,
        #[cfg(any(test, feature = "test-fixtures"))] shapeshift_form: &'a u32,
        consumer_test: bool,
    ) -> Self {
        Self { inventory, inventory_access, modifiers, item_sets, publication, catalogs,
            #[cfg(any(test, feature = "test-fixtures"))] player_level,
            #[cfg(any(test, feature = "test-fixtures"))] shapeshift_form,
            consumer_test }
    }

    fn record_mods(&mut self, guid: ObjectGuid, slot: u8, apply: bool) -> bool {
        self.inventory.record_represented_item_mods_with_access_like_cpp(
            &self.inventory_access, &self.modifiers, self.catalogs, guid, slot, apply,
            #[cfg(any(test, feature = "test-fixtures"))] self.player_level,
            #[cfg(any(test, feature = "test-fixtures"))] self.shapeshift_form,
            self.consumer_test,
        ) != 0
    }

    pub fn remove_item_effects_like_cpp(&mut self, bag: u8, slot: u8, guid: ObjectGuid, cleared: &[EnchantmentSlot]) -> bool {
        self.inventory.remove_inventory_item_duration_refs_with_access_like_cpp(&self.inventory_access, guid);
        self.inventory.remove_inventory_tradeable_item_with_access_like_cpp(&self.inventory_access, guid);
        if bag != INVENTORY_SLOT_BAG_0 || slot >= INVENTORY_SLOT_BAG_END { return false; }
        let _ = self.inventory.record_represented_items_set_item_like_cpp(
            &self.inventory_access, &self.modifiers, &self.item_sets, guid, false, self.consumer_test,
        );
        let changed = if self.inventory.resolved_player_inventory_item_object_with_access_like_cpp(&self.inventory_access, guid)
            .is_some_and(|item| item.is_broken()) { false } else { self.record_mods(guid, slot, false) };
        let mut updates = vec![ItemObjectUpdateLikeCpp::SetEquipped(false)];
        updates.extend(cleared.iter().copied().map(ItemObjectUpdateLikeCpp::ClearEnchantment));
        let _ = self.inventory.apply_inventory_item_object_updates_with_access_like_cpp(&self.inventory_access, guid, &updates);
        if slot < PROFESSION_SLOT_END { self.inventory.record_inventory_item_combat_stat_recalculations_like_cpp(slot); }
        changed
    }

    pub fn store_item_effects_like_cpp(&mut self, bag: u8, slot: u8, guid: ObjectGuid) -> bool {
        self.inventory.add_inventory_item_duration_refs_with_access_like_cpp(&self.inventory_access, &self.publication, guid);
        if bag != INVENTORY_SLOT_BAG_0 || slot >= INVENTORY_SLOT_BAG_END { return false; }
        let _ = self.inventory.apply_inventory_item_object_updates_with_access_like_cpp(&self.inventory_access, guid, &[ItemObjectUpdateLikeCpp::SetEquipped(true)]);
        let _ = self.inventory.record_represented_items_set_item_like_cpp(
            &self.inventory_access, &self.modifiers, &self.item_sets, guid, true, self.consumer_test,
        );
        let changed = if self.inventory.resolved_player_inventory_item_object_with_access_like_cpp(&self.inventory_access, guid)
            .is_some_and(|item| !item.is_broken()) { self.record_mods(guid, slot, true) } else { false };
        if slot < PROFESSION_SLOT_END { self.inventory.record_inventory_item_combat_stat_recalculations_like_cpp(slot); }
        changed
    }
}
