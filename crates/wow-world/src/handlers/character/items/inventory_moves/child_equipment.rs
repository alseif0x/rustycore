//! child equipment for the existing inventory_moves owner.

use super::*;

impl WorldSession {

    /// Current upstream C++ `Player::CanEquipChildItem`. Rust represents the
    /// parent/child link with the child's `CHILD` flag plus creator GUID; the
    /// DB2 row supplies the visible equipment slot. When that slot is busy,
    /// validate where the displaced item can be stored before moving the
    /// parent, preserving C++'s no-partial-parent-move failure contract.
    pub(crate) fn plan_inventory_equip_child_like_cpp(
        &self,
        parent_bag: u8,
        parent_slot: u8,
        parent_guid: ObjectGuid,
    ) -> Result<Option<InventoryEquipChildPlanLikeCpp>, InventoryResult> {
        let Some(parent) = self.resolved_inventory_item_object_like_cpp(parent_guid) else {
            return Ok(None);
        };
        let Some(child_equipment) =
            self.item_child_equipment_for_parent_like_cpp(parent.object().entry())
        else {
            return Ok(None);
        };
        let destination_slot = child_equipment.child_item_equip_slot;
        if !is_equipment_pos(INVENTORY_SLOT_BAG_0, destination_slot) {
            return Err(InventoryResult::NotEquippable);
        }
        let Some(item_objects) = self.resolved_inventory_item_objects_like_cpp() else {
            return Err(InventoryResult::ItemNotFound);
        };
        let Some(child) = item_objects.values().find(|item| {
            item.has_item_flag(ItemFieldFlags::CHILD)
                && item.data().creator == parent_guid
                && (child_equipment.child_item_id <= 0
                    || item.object().entry() == child_equipment.child_item_id as u32)
        }) else {
            return Ok(None);
        };
        let child_guid = child.object().guid();
        if self
            .get_inventory_item_by_guid_like_cpp(child_guid)
            .is_some_and(|(bag, slot, _)| bag == INVENTORY_SLOT_BAG_0 && slot == destination_slot)
        {
            return Ok(None);
        }

        let Some(displaced) =
            self.get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, destination_slot)
        else {
            return Ok(Some(InventoryEquipChildPlanLikeCpp {
                child_guid,
                destination_slot,
                displaced_storage: None,
            }));
        };
        let displaced_object = self.resolved_inventory_item_object_like_cpp(displaced.guid);
        let displaced_proto = self.item_storage_template(displaced.entry_id);
        let child_is_bag = self
            .item_storage_template(child.object().entry())
            .is_some_and(|template| template.container_slots > 0);
        let can_unequip = self.can_unequip_inventory_item_at_like_cpp(
            INVENTORY_SLOT_BAG_0,
            destination_slot,
            !child_is_bag,
            displaced_object.as_ref(),
            displaced_proto.as_ref(),
            self.direct_item_contains_items(displaced.guid),
        );
        if can_unequip != InventoryResult::Ok {
            return Err(can_unequip);
        }

        let displaced_storage = if is_inventory_pos(parent_bag, parent_slot) {
            let mut last_result = InventoryResult::InvFull;
            let mut destination = None;
            for (bag, slot) in [(parent_bag, NULL_SLOT), (NULL_BAG, NULL_SLOT)] {
                let Some((result, _, _)) = self.plan_store_existing_inventory_item_at_like_cpp(
                    INVENTORY_SLOT_BAG_0,
                    destination_slot,
                    bag,
                    slot,
                    true,
                ) else {
                    continue;
                };
                last_result = result;
                if result == InventoryResult::Ok {
                    destination = Some((bag, slot, InventoryStorageTargetLikeCpp::Inventory));
                    break;
                }
            }
            destination.ok_or(last_result)?
        } else if is_bank_pos(parent_bag, parent_slot) {
            let mut last_result = InventoryResult::BankFull;
            let mut destination = None;
            for (bag, slot) in [(parent_bag, NULL_SLOT), (NULL_BAG, NULL_SLOT)] {
                let Some((result, _)) = self.plan_bank_existing_inventory_item_at_like_cpp(
                    INVENTORY_SLOT_BAG_0,
                    destination_slot,
                    bag,
                    slot,
                    true,
                ) else {
                    continue;
                };
                last_result = result;
                if result == InventoryResult::Ok {
                    destination = Some((bag, slot, InventoryStorageTargetLikeCpp::Bank));
                    break;
                }
            }
            destination.ok_or(last_result)?
        } else {
            return Err(InventoryResult::CantSwap);
        };

        Ok(Some(InventoryEquipChildPlanLikeCpp {
            child_guid,
            destination_slot,
            displaced_storage: Some(displaced_storage),
        }))
    }

    /// Current upstream C++ `Player::EquipChildItem`, executed only after the
    /// parent move and its preflight have succeeded.
    pub(crate) async fn execute_inventory_equip_child_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
        plan: InventoryEquipChildPlanLikeCpp,
    ) -> bool {
        if self
            .get_inventory_item_by_guid_like_cpp(plan.child_guid)
            .is_some_and(|(bag, slot, _)| {
                bag == INVENTORY_SLOT_BAG_0 && slot == plan.destination_slot
            })
        {
            return true;
        }

        if let Some((bag, slot, target)) = plan.displaced_storage {
            self.execute_inventory_storage_move_like_cpp(
                item_guid_generator,
                creature_spawn_catalogs,
                INVENTORY_SLOT_BAG_0,
                plan.destination_slot,
                bag,
                slot,
                target,
                InventoryStorageQuestChecksLikeCpp::None,
                None,
            )
            .await;
            if self
                .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, plan.destination_slot)
                .is_some()
            {
                return false;
            }
        }

        let Some((child_bag, child_slot, _)) =
            self.get_inventory_item_by_guid_like_cpp(plan.child_guid)
        else {
            return false;
        };
        self.execute_inventory_equip_to_empty_raw_like_cpp(
            child_bag,
            child_slot,
            wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, plan.destination_slot),
        )
        .await;
        self.get_inventory_item_by_guid_like_cpp(plan.child_guid)
            .is_some_and(|(bag, slot, _)| {
                bag == INVENTORY_SLOT_BAG_0 && slot == plan.destination_slot
            })
    }
}
