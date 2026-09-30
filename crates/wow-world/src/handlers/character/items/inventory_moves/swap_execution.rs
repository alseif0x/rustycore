//! swap execution for the existing inventory_moves owner.

use super::*;

impl WorldSession {

    pub(crate) async fn execute_inventory_swap_step_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
        src: u16,
        dst: u16,
    ) -> InventorySwapStepLikeCpp {
        let [src_bag, src_slot] = src.to_be_bytes();
        let [dst_bag, dst_slot] = dst.to_be_bytes();
        let source = self.get_inventory_item_by_pos(src_bag, src_slot);
        let destination = self.get_inventory_item_by_pos(dst_bag, dst_slot);
        let source_guid = source.as_ref().map(|item| item.guid);
        let destination_guid = destination.as_ref().map(|item| item.guid);

        let Some(preflight) = self.plan_inventory_swap_preflight_like_cpp(src, dst) else {
            return InventorySwapStepLikeCpp::Done;
        };
        match preflight.result {
            SwapItemPreflightResult::NoSource => return InventorySwapStepLikeCpp::Done,
            SwapItemPreflightResult::ChildRedirect {
                first_src,
                first_dst,
                second_src,
                second_dst,
            } => {
                let child = source
                    .as_ref()
                    .filter(|item| {
                        is_equipment_pos(src_bag, src_slot)
                            && self
                                .resolved_inventory_item_object_like_cpp(item.guid)
                                .is_some_and(|object| object.has_item_flag(ItemFieldFlags::CHILD))
                    })
                    .map(|item| (src_bag, src_slot, item.guid))
                    .or_else(|| {
                        destination
                            .as_ref()
                            .filter(|item| {
                                is_equipment_pos(dst_bag, dst_slot)
                                    && self
                                        .resolved_inventory_item_object_like_cpp(item.guid)
                                        .is_some_and(|object| {
                                            object.has_item_flag(ItemFieldFlags::CHILD)
                                        })
                            })
                            .map(|item| (dst_bag, dst_slot, item.guid))
                    });
                let Some((child_bag, child_slot, child_guid)) = child else {
                    return InventorySwapStepLikeCpp::Done;
                };
                let hidden_slot = match self.plan_inventory_child_redirect_like_cpp(
                    child_bag, child_slot, first_src, first_dst, second_src, second_dst,
                ) {
                    Ok(hidden_slot) => hidden_slot,
                    Err(result) => {
                        self.send_equip_error(result, source_guid, destination_guid, 0, 0);
                        return InventorySwapStepLikeCpp::Done;
                    }
                };
                if !self
                    .execute_inventory_auto_unequip_child_item_like_cpp(
                        item_guid_generator,
                        creature_spawn_catalogs,
                        child_bag,
                        child_slot,
                        child_guid,
                        hidden_slot,
                    )
                    .await
                {
                    return InventorySwapStepLikeCpp::Done;
                }
                return InventorySwapStepLikeCpp::ChildRedirect {
                    first_src,
                    first_dst,
                    second_src,
                    second_dst,
                };
            }
            SwapItemPreflightResult::Error(result) => {
                self.send_equip_error(result, source_guid, destination_guid, 0, 0);
                return InventorySwapStepLikeCpp::Done;
            }
            SwapItemPreflightResult::Continue => {}
        }

        let Some(source) = source else {
            return InventorySwapStepLikeCpp::Done;
        };
        let source_limit_category = self
            .item_storage_template(source.entry_id)
            .map_or(0, |template| template.item_limit_category);

        let Some(destination) = destination else {
            let Some((result, target)) = self.validate_inventory_swap_target_like_cpp(
                src_bag, src_slot, dst_bag, dst_slot, false, true,
            ) else {
                return InventorySwapStepLikeCpp::Done;
            };
            if result != InventoryResult::Ok {
                self.send_equip_error(result, Some(source.guid), None, 0, source_limit_category);
                return InventorySwapStepLikeCpp::Done;
            }
            match target {
                InventorySwapTargetLikeCpp::Inventory => {
                    self.execute_inventory_storage_move_like_cpp(
                        item_guid_generator,
                        creature_spawn_catalogs,
                        src_bag,
                        src_slot,
                        dst_bag,
                        dst_slot,
                        InventoryStorageTargetLikeCpp::Inventory,
                        InventoryStorageQuestChecksLikeCpp::None,
                        None,
                    )
                    .await;
                }
                InventorySwapTargetLikeCpp::Bank => {
                    self.execute_inventory_storage_move_like_cpp(
                        item_guid_generator,
                        creature_spawn_catalogs,
                        src_bag,
                        src_slot,
                        dst_bag,
                        dst_slot,
                        InventoryStorageTargetLikeCpp::Bank,
                        InventoryStorageQuestChecksLikeCpp::None,
                        None,
                    )
                    .await;
                }
                InventorySwapTargetLikeCpp::Equipment { dest } => {
                    self.execute_inventory_equip_to_empty_like_cpp(
                        item_guid_generator,
                        creature_spawn_catalogs,
                        src_bag,
                        src_slot,
                        dest,
                    )
                    .await;
                }
                InventorySwapTargetLikeCpp::None => {}
            }
            return InventorySwapStepLikeCpp::Done;
        };

        let source_is_bag = self
            .item_storage_template(source.entry_id)
            .is_some_and(|template| template.container_slots > 0);
        let destination_is_bag = self
            .item_storage_template(destination.entry_id)
            .is_some_and(|template| template.container_slots > 0);
        if !source_is_bag && !destination_is_bag && source.entry_id == destination.entry_id {
            let Some((result, target)) = self.validate_inventory_swap_target_like_cpp(
                src_bag, src_slot, dst_bag, dst_slot, false, false,
            ) else {
                return InventorySwapStepLikeCpp::Done;
            };
            if result == InventoryResult::Ok && !matches!(target, InventorySwapTargetLikeCpp::None)
            {
                self.execute_inventory_stack_merge_like_cpp(
                    item_guid_generator,
                    creature_spawn_catalogs,
                    src_bag,
                    src_slot,
                    dst_bag,
                    dst_slot,
                    source,
                    destination,
                )
                .await;
                return InventorySwapStepLikeCpp::Done;
            }
        }

        let Some((source_result, source_target)) = self.validate_inventory_swap_target_like_cpp(
            src_bag, src_slot, dst_bag, dst_slot, true, true,
        ) else {
            return InventorySwapStepLikeCpp::Done;
        };
        if source_result != InventoryResult::Ok {
            self.send_equip_error(
                source_result,
                Some(source.guid),
                Some(destination.guid),
                0,
                source_limit_category,
            );
            return InventorySwapStepLikeCpp::Done;
        }
        let destination_limit_category = self
            .item_storage_template(destination.entry_id)
            .map_or(0, |template| template.item_limit_category);
        let Some((destination_result, destination_target)) = self
            .validate_inventory_swap_target_like_cpp(
                dst_bag, dst_slot, src_bag, src_slot, true, true,
            )
        else {
            return InventorySwapStepLikeCpp::Done;
        };
        if destination_result != InventoryResult::Ok {
            self.send_equip_error(
                destination_result,
                Some(destination.guid),
                Some(source.guid),
                0,
                destination_limit_category,
            );
            return InventorySwapStepLikeCpp::Done;
        }

        self.execute_inventory_real_swap_like_cpp(
            item_guid_generator,
            creature_spawn_catalogs,
            src_bag,
            src_slot,
            dst_bag,
            dst_slot,
            source,
            destination,
            source_target,
            destination_target,
        )
        .await;
        InventorySwapStepLikeCpp::Done
    }

    pub(crate) async fn execute_inventory_equip_to_empty_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
        source_bag: u8,
        source_slot: u8,
        destination: u16,
    ) {
        let Some(source) = self.get_inventory_item_by_pos(source_bag, source_slot) else {
            return;
        };
        let child_plan =
            match self.plan_inventory_equip_child_like_cpp(source_bag, source_slot, source.guid) {
                Ok(plan) => plan,
                Err(result) => {
                    self.send_equip_error(result, Some(source.guid), None, 0, 0);
                    return;
                }
            };
        let source_guid = source.guid;
        self.execute_inventory_equip_to_empty_raw_like_cpp(source_bag, source_slot, destination)
            .await;
        let [destination_bag, destination_slot] = destination.to_be_bytes();
        if !self
            .get_inventory_item_by_guid_like_cpp(source_guid)
            .is_some_and(|(bag, slot, _)| bag == destination_bag && slot == destination_slot)
        {
            return;
        }
        if let Some(plan) = child_plan {
            let _ = self
                .execute_inventory_equip_child_like_cpp(
                    item_guid_generator,
                    creature_spawn_catalogs,
                    plan,
                )
                .await;
        }
        self.execute_inventory_auto_unequip_offhand_if_need_like_cpp(
            item_guid_generator,
            creature_spawn_catalogs,
        )
        .await;
    }
}
