// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn inventory_items_like_cpp(&self) -> &HashMap<u8, InventoryItem> {
        self.inventory.inventory_items_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn inventory_item_objects_like_cpp(&self) -> &HashMap<ObjectGuid, Item> {
        self.inventory.inventory_item_objects_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn update_inventory_item_object_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        update: impl FnOnce(&mut Item),
    ) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.update_inventory_item_object_like_cpp(&mut hub, item_guid, update)
    }
    pub(crate) fn mutate_player_inventory_runtime_like_cpp<R>(
        &mut self,
        update: impl FnOnce(&mut PlayerInventoryRuntime) -> R,
    ) -> Option<R> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.mutate_player_inventory_runtime_like_cpp(&mut hub, update)
    }
    pub(crate) fn make_inventory_item_object(
        &self,
        item_guid: ObjectGuid,
        entry_id: u32,
        owner_guid: ObjectGuid,
        count: u32,
        durability: u32,
        context: ItemContext,
        slot: u8,
    ) -> Item {
        crate::session::cx_inventory_ref(self).make_inventory_item_object(
            item_guid, entry_id, owner_guid, count, durability, context, slot,
        )
    }
    pub(crate) fn clear_all_inventory_runtime_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.clear_all_inventory_runtime_like_cpp(&mut hub)
    }
}

impl WorldSession {
    pub(crate) fn move_represented_direct_inventory_item_with_item_mods_like_cpp(
        &mut self,
        src: u8,
        dst: u8,
    ) -> Option<bool> {
        let inventory_access = self.core.owned_inventory_access_like_cpp();
        let modifier_access = self.core.owned_item_modifiers_access_like_cpp();
        let item_sets = self.core.owned_item_set_access_like_cpp(
            self.catalogs.items.set_store.as_deref(),
            self.catalogs.spell_catalogs.item_set_spell_store.as_deref(),
            self.catalogs.spell_catalogs.spell_store.as_deref(),
            self.catalogs.heirloom_store.as_deref(),
            self.catalogs.items.stats_store.as_deref(),
            self.catalogs.curve_store.as_deref(),
            self.catalogs.curve_point_store.as_deref(),
            self.catalogs.content_tuning_store.as_deref(),
            #[cfg(any(test, feature = "test-fixtures"))]
            &self
                .fixtures
                .progression
                .player_skill_test_fixture_like_cpp
                .player_skill_records_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self
                .fixtures
                .progression
                .represented_primary_specialization_id_like_cpp,
        );

        self.inventory
            .move_direct_inventory_item_with_item_mods_for_equipment_set_like_cpp(
                &inventory_access,
                &modifier_access,
                &item_sets,
                self.catalogs.items.store.as_ref(),
                self.catalogs.items.stats_store.as_ref(),
                self.catalogs.scaling_stat_distribution_store.as_ref(),
                self.catalogs.scaling_stat_values_store.as_ref(),
                self.catalogs.shield_block_regular_game_table.as_ref(),
                self.catalogs.spell_catalogs.spell_shapeshift_form_store(),
                src,
                dst,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.identity.player_level,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.auras.represented_shapeshift_form_like_cpp,
                cfg!(test),
            )
    }
    pub(crate) fn record_destroyed_inventory_item_mod_remove_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        item_guid: ObjectGuid,
    ) -> bool {
        if bag != INVENTORY_SLOT_BAG_0 || slot >= INVENTORY_SLOT_BAG_END {
            return false;
        }
        if self
            .resolved_inventory_item_object_like_cpp(item_guid)
            .is_some_and(|item| item.is_broken())
        {
            return false;
        }

        self.record_represented_item_mods_like_cpp(item_guid, slot, false) != 0
    }
}
