use crate::session::state::SessionCore;
use wow_constants::InventoryResult;
use wow_core::ObjectGuid;
use wow_data::ItemChildEquipmentEntry;
use wow_packet::packets::item::InventoryChangeFailure;

impl SessionCore {
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
}

impl crate::session::state::SessionCatalogs {
    /// C++ `DB2Manager::GetItemChildEquipment(parentItemId)`.
    pub fn item_child_equipment_for_parent_like_cpp(
        &self,
        parent_item_id: u32,
    ) -> Option<&ItemChildEquipmentEntry> {
        self.items
            .child_equipment_store
            .as_ref()?
            .values()
            .find(|entry| entry.parent_item_id == parent_item_id)
    }
}
