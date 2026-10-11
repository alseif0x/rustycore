//! Represented equipment: equip and unequip over the canonical Player.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    pub(crate) fn apply_initial_equipped_item_equip_auras_like_cpp(&mut self) -> Option<usize> {
        let mut equipped: Vec<_> = self
            .resolved_inventory_item_objects_like_cpp()?
            .values()
            .filter(|item| item.container_guid().is_empty() && item.slot() < INVENTORY_SLOT_BAG_END)
            .map(|item| (item.slot(), item.object().guid()))
            .collect();
        equipped.sort_by_key(|(slot, guid)| (*slot, guid.counter()));
        Some(
            equipped
                .into_iter()
                .map(|(_slot, item_guid)| self.apply_initial_item_equip_auras_like_cpp(item_guid))
                .sum(),
        )
    }
    pub(in crate::session) fn apply_initial_item_equip_auras_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
    ) -> usize {
        self.player_aura_application_cx_like_cpp()
            .apply_initial_item_equip_auras_like_cpp(item_guid)
    }
    pub(crate) fn can_unequip_inventory_item_at_like_cpp(
        &self,
        bag: u8,
        slot: u8,
        swap: bool,
        source_item: Option<&Item>,
        proto: Option<&ItemStorageTemplate>,
        source_is_not_empty_bag: bool,
    ) -> InventoryResult {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.can_unequip_inventory_item_at_like_cpp(
            hub,
            bag,
            slot,
            swap,
            source_item,
            proto,
            source_is_not_empty_bag,
        )
    }
    #[cfg(test)]
    pub(crate) fn creature_equipment_store_like_cpp(
        &self,
    ) -> Option<&Arc<CreatureEquipmentStoreLikeCpp>> {
        self.catalogs.creature_equipment_store_like_cpp.as_ref()
    }
    pub(crate) fn send_equip_error(
        &self,
        result: InventoryResult,
        item1: Option<ObjectGuid>,
        item2: Option<ObjectGuid>,
        required_level: u32,
        limit_category: u32,
    ) {
        self.core
            .send_equip_error(result, item1, item2, required_level, limit_category)
    }
    pub(crate) fn record_represented_avg_equipped_item_level_update_like_cpp(&mut self) {
        #[cfg(test)]
        {
            let Some(avg_equipped_item_level) = self.represented_avg_equipped_item_level_like_cpp()
            else {
                return;
            };
            self.inventory
                .record_represented_avg_equipped_item_level_update_for_test_like_cpp(
                    avg_equipped_item_level,
                );
        }
    }
    /// C++ `Player::CanEquipItem` for a represented runtime item.
    pub(crate) fn plan_equip_existing_inventory_item_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        requested_slot: u8,
        swap: bool,
    ) -> Option<(InventoryResult, u16)> {
        let conditions = self.player_condition_projection_cx_like_cpp();
        wow_world_application::InventoryMovePlanningCxLikeCpp::new(&conditions)
            .plan_equip_existing_inventory_item_like_cpp(
                source_bag,
                source_slot,
                requested_slot,
                swap,
            )
    }
    pub(crate) fn represented_avg_equipped_item_level_like_cpp(&self) -> Option<f32> {
        let inventory_access = self.core.owned_inventory_access_like_cpp();
        let valuation_access = self.core.inventory_valuation_access_like_cpp();
        let modifier_access = self.core.owned_item_modifiers_access_like_cpp();
        let catalogs = self.catalogs.inventory_valuation_catalog_view_like_cpp();
        self.inventory
            .represented_avg_equipped_item_level_with_access_like_cpp(
                &inventory_access,
                &valuation_access,
                &modifier_access,
                &catalogs,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.identity.player_level,
                Self::MIN_ITEM_LEVEL_LIKE_CPP,
                Self::MAX_ITEM_LEVEL_LIKE_CPP,
            )
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/player_items/equipment/f3_shims.rs"]
mod f3_shims;
