//! valuation and publication for the existing equipment owner.

use super::*;

impl WorldSession {
    pub fn send_equip_error(
        &self,
        result: InventoryResult,
        item1: Option<ObjectGuid>,
        item2: Option<ObjectGuid>,
        required_level: u32,
        limit_category: u32,
    ) {
        let mut packet = InventoryChangeFailure::new(
            result,
            item1.unwrap_or(ObjectGuid::EMPTY),
            item2.unwrap_or(ObjectGuid::EMPTY),
        );

        if result != InventoryResult::Ok {
            packet.container_b_slot = 0;
            match result {
                InventoryResult::CantEquipLevelI | InventoryResult::PurchaseLevelTooLow => {
                    packet.level = required_level;
                }
                InventoryResult::ItemMaxLimitCategoryCountExceededIs
                | InventoryResult::ItemMaxLimitCategorySocketedExceededIs
                | InventoryResult::ItemMaxLimitCategoryEquippedExceededIs => {
                    packet.limit_category = limit_category;
                }
                _ => {}
            }
        }

        // C++ `Opcodes.cpp` registers `SMSG_INVENTORY_CHANGE_FAILURE` on
        // `CONNECTION_TYPE_REALM`, including errors raised by instance-routed
        // inventory requests after `ConnectTo`.
        self.send_packet_realm(&packet);
    }
    pub(crate) fn record_represented_avg_equipped_item_level_update_like_cpp(&mut self) {
        #[cfg(test)]
        {
            let Some(avg_equipped_item_level) = self.represented_avg_equipped_item_level_like_cpp()
            else {
                return;
            };
            self.player_item_test_fixture_like_cpp
                .represented_avg_equipped_item_level_updates_like_cpp
                .push(avg_equipped_item_level);
        }
    }
    pub(crate) fn represented_avg_equipped_item_level_like_cpp(&self) -> Option<f32> {
        let (_, can_titan_grip) = self.inventory_equip_capabilities_like_cpp()?;
        let inventory_items = self.resolved_inventory_items_like_cpp()?;
        let mut total_item_level = 0u32;
        for slot in 0..EQUIPMENT_SLOT_END {
            let Some(inventory_item) = inventory_items.get(&slot) else {
                continue;
            };
            let runtime_item = self.resolved_inventory_item_object_like_cpp(inventory_item.guid);
            let Some(item_level) = self
                .represented_item_level_like_cpp(inventory_item.entry_id, runtime_item.as_ref())
            else {
                continue;
            };
            total_item_level = total_item_level.saturating_add(item_level);
            let is_mainhand_two_hand = slot == EQUIPMENT_SLOT_MAINHAND
                && !can_titan_grip
                && self
                    .item_storage_template(inventory_item.entry_id)
                    .is_some_and(|item_template| {
                        item_template.inventory_type == InventoryType::Weapon2Hand
                    });
            if is_mainhand_two_hand {
                total_item_level = total_item_level.saturating_add(item_level);
            }
        }

        Some(total_item_level as f32 / 16.0)
    }
    #[cfg(test)]
    pub(crate) fn represented_avg_equipped_item_level_updates_like_cpp(&self) -> &[f32] {
        &self
            .player_item_test_fixture_like_cpp
            .represented_avg_equipped_item_level_updates_like_cpp
    }
}
