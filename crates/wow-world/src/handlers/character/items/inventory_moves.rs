// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Inventory move planning and execution for character item handlers.
//!
//! This private module preserves the existing `WorldSession` ownership while
//! isolating the swap/equip/stack transition family from packet adapters.

use super::*;

mod item_mutations;
mod real_swap;

impl WorldSession {
    pub(crate) fn validate_inventory_swap_target_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        swap: bool,
        require_exact_destination: bool,
    ) -> Option<(InventoryResult, InventorySwapTargetLikeCpp)> {
        let source = self.get_inventory_item_by_pos(source_bag, source_slot)?;
        let source_count = self
            .resolved_inventory_item_object_like_cpp(source.guid)?
            .count();
        let destination_pos = wow_entities::make_item_pos(destination_bag, destination_slot);

        if is_inventory_pos(destination_bag, destination_slot) {
            let (mut result, destinations, _) = self
                .plan_store_existing_inventory_item_at_like_cpp(
                    source_bag,
                    source_slot,
                    destination_bag,
                    destination_slot,
                    swap,
                )?;
            if require_exact_destination
                && result == InventoryResult::Ok
                && (destinations.len() != 1
                    || destinations[0].pos != destination_pos
                    || destinations[0].count != source_count)
            {
                result = InventoryResult::InternalBagError;
            }
            return Some((result, InventorySwapTargetLikeCpp::Inventory));
        }
        if is_bank_pos(destination_bag, destination_slot) {
            let (mut result, destinations) = self.plan_bank_existing_inventory_item_at_like_cpp(
                source_bag,
                source_slot,
                destination_bag,
                destination_slot,
                swap,
            )?;
            if require_exact_destination
                && result == InventoryResult::Ok
                && (destinations.len() != 1
                    || destinations[0].pos != destination_pos
                    || destinations[0].count != source_count)
            {
                result = InventoryResult::InternalBagError;
            }
            return Some((result, InventorySwapTargetLikeCpp::Bank));
        }
        if is_equipment_pos(destination_bag, destination_slot) {
            let (mut result, dest) = self.plan_equip_existing_inventory_item_like_cpp(
                source_bag,
                source_slot,
                destination_slot,
                swap,
            )?;
            if result == InventoryResult::Ok && dest != destination_pos {
                result = InventoryResult::InternalBagError;
            }
            return Some((result, InventorySwapTargetLikeCpp::Equipment { dest }));
        }

        Some((InventoryResult::Ok, InventorySwapTargetLikeCpp::None))
    }

    pub(crate) async fn execute_inventory_swap_positions_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
        src: u16,
        dst: u16,
    ) {
        let mut pending = VecDeque::from([(src, dst)]);
        let mut steps = 0usize;
        while let Some((step_src, step_dst)) = pending.pop_front() {
            steps += 1;
            if steps > 4 {
                self.send_equip_error(InventoryResult::InternalBagError, None, None, 0, 0);
                return;
            }
            match self
                .execute_inventory_swap_step_like_cpp(
                    item_guid_generator,
                    creature_spawn_catalogs,
                    step_src,
                    step_dst,
                )
                .await
            {
                InventorySwapStepLikeCpp::Done => {}
                InventorySwapStepLikeCpp::ChildRedirect {
                    first_src,
                    first_dst,
                    second_src,
                    second_dst,
                } => {
                    pending.push_front((second_src, second_dst));
                    pending.push_front((first_src, first_dst));
                }
            }
        }
    }

    pub(crate) fn validate_inventory_redirected_empty_move_like_cpp(
        &self,
        src: u16,
        dst: u16,
    ) -> Result<Option<u32>, InventoryResult> {
        let Some(preflight) = self.plan_inventory_swap_preflight_like_cpp(src, dst) else {
            return Err(InventoryResult::InternalBagError);
        };
        match preflight.result {
            SwapItemPreflightResult::NoSource => return Ok(None),
            SwapItemPreflightResult::Error(result) => return Err(result),
            SwapItemPreflightResult::ChildRedirect { .. } => {
                return Err(InventoryResult::InternalBagError);
            }
            SwapItemPreflightResult::Continue => {}
        }

        let [src_bag, src_slot] = src.to_be_bytes();
        let [dst_bag, dst_slot] = dst.to_be_bytes();
        if self.get_inventory_item_by_pos(dst_bag, dst_slot).is_some() {
            return Err(InventoryResult::InternalBagError);
        }
        let source = self
            .get_inventory_item_by_pos(src_bag, src_slot)
            .ok_or(InventoryResult::ItemNotFound)?;
        let count = self
            .resolved_inventory_item_object_like_cpp(source.guid)
            .map(|item| item.count())
            .ok_or(InventoryResult::ItemNotFound)?;
        let Some((result, target)) = self.validate_inventory_swap_target_like_cpp(
            src_bag, src_slot, dst_bag, dst_slot, false, true,
        ) else {
            return Err(InventoryResult::InternalBagError);
        };
        if result != InventoryResult::Ok {
            return Err(result);
        }
        if matches!(target, InventorySwapTargetLikeCpp::None) {
            return Err(InventoryResult::InternalBagError);
        }
        Ok(Some(count))
    }

    /// Validate the two recursive `Player::SwapItem` moves against a temporary
    /// overlay before `AutoUnequipChildItem` is persisted. C++ performs those
    /// calls synchronously; preserving that observable order in Rust must not
    /// leave the child hidden when a later dead/combat/charmed/unequip/equip
    /// gate rejects either move.
    pub(crate) fn plan_inventory_child_redirect_like_cpp(
        &mut self,
        child_bag: u8,
        child_slot: u8,
        first_src: u16,
        first_dst: u16,
        second_src: u16,
        second_dst: u16,
    ) -> Result<u8, InventoryResult> {
        let child_move = self
            .plan_inventory_storage_move_like_cpp(
                child_bag,
                child_slot,
                INVENTORY_SLOT_BAG_0,
                NULL_SLOT,
                InventoryStorageTargetLikeCpp::Inventory,
            )
            .ok_or(InventoryResult::ItemNotFound)??;
        if !child_move.existing_updates.is_empty() {
            return Err(InventoryResult::InternalBagError);
        }
        let Some((hidden_bag, hidden_slot, hidden_count)) = child_move.moved_destination else {
            return Err(InventoryResult::InternalBagError);
        };
        if hidden_bag != INVENTORY_SLOT_BAG_0
            || !is_child_equipment_pos(hidden_bag, hidden_slot)
            || hidden_count != child_move.source_count
        {
            return Err(InventoryResult::InternalBagError);
        }
        if !self.apply_committed_inventory_item_relocation_like_cpp(
            child_bag,
            child_slot,
            hidden_bag,
            hidden_slot,
            hidden_count,
        ) {
            return Err(InventoryResult::InternalBagError);
        }

        let first_count =
            self.validate_inventory_redirected_empty_move_like_cpp(first_src, first_dst);
        let mut first_applied_count = None;
        let validation = match first_count {
            Ok(Some(count)) => {
                let [first_src_bag, first_src_slot] = first_src.to_be_bytes();
                let [first_dst_bag, first_dst_slot] = first_dst.to_be_bytes();
                if self.apply_committed_inventory_item_relocation_like_cpp(
                    first_src_bag,
                    first_src_slot,
                    first_dst_bag,
                    first_dst_slot,
                    count,
                ) {
                    first_applied_count = Some(count);
                    self.validate_inventory_redirected_empty_move_like_cpp(second_src, second_dst)
                        .map(|_| ())
                } else {
                    Err(InventoryResult::InternalBagError)
                }
            }
            Ok(None) => self
                .validate_inventory_redirected_empty_move_like_cpp(second_src, second_dst)
                .map(|_| ()),
            Err(result) => Err(result),
        };

        let first_rolled_back = if let Some(count) = first_applied_count {
            let [first_src_bag, first_src_slot] = first_src.to_be_bytes();
            let [first_dst_bag, first_dst_slot] = first_dst.to_be_bytes();
            self.apply_committed_inventory_item_relocation_like_cpp(
                first_dst_bag,
                first_dst_slot,
                first_src_bag,
                first_src_slot,
                count,
            )
        } else {
            true
        };
        debug_assert!(first_rolled_back);
        let child_rolled_back = self.apply_committed_inventory_item_relocation_like_cpp(
            hidden_bag,
            hidden_slot,
            child_bag,
            child_slot,
            hidden_count,
        );
        debug_assert!(child_rolled_back);
        if !first_rolled_back || !child_rolled_back {
            return Err(InventoryResult::InternalBagError);
        }

        validation.map(|()| hidden_slot)
    }

    /// Current upstream TrinityCore calls `Player::AutoUnequipChildItem`
    /// before recursively continuing either child redirect in
    /// `Player::SwapItem`. The legacy 3.4.3 snapshot omitted that call and
    /// recurses on the unchanged equipped child forever. Persist the child in
    /// the already validated reserved slot so both queued moves observe its
    /// equipment position as empty.
    pub(crate) async fn execute_inventory_auto_unequip_child_item_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
        child_bag: u8,
        child_slot: u8,
        child_guid: ObjectGuid,
        hidden_slot: u8,
    ) -> bool {
        if is_child_equipment_pos(child_bag, child_slot) {
            return true;
        }

        self.execute_inventory_storage_move_like_cpp(
            item_guid_generator,
            creature_spawn_catalogs,
            child_bag,
            child_slot,
            INVENTORY_SLOT_BAG_0,
            hidden_slot,
            InventoryStorageTargetLikeCpp::Inventory,
            InventoryStorageQuestChecksLikeCpp::None,
            None,
        )
        .await;

        self.get_inventory_item_by_guid_like_cpp(child_guid)
            .is_some_and(|(bag, slot, _)| bag == INVENTORY_SLOT_BAG_0 && slot == hidden_slot)
    }

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

    pub(crate) fn publish_inventory_position_changes_like_cpp(&mut self, positions: &[(u8, u8)]) {
        let mut unique_positions = positions.to_vec();
        unique_positions.sort_unstable();
        unique_positions.dedup();
        let mut top_level_changes = Vec::new();
        let mut visible_item_changes = Vec::new();
        let mut virtual_item_changes = Vec::new();
        let mut gear_changed = false;

        for (bag, slot) in unique_positions {
            if bag != INVENTORY_SLOT_BAG_0 {
                self.send_bag_slot_values_update_like_cpp(bag, slot);
                continue;
            }
            let item = self.get_inventory_item_by_pos(bag, slot);
            top_level_changes.push((
                slot,
                item.as_ref().map_or(ObjectGuid::EMPTY, |item| item.guid),
            ));
            if slot < 19 {
                gear_changed = true;
                visible_item_changes.push((
                    slot,
                    item.as_ref().map_or(0, |item| item.entry_id as i32),
                    0,
                    0,
                ));
            }
            if (15..=17).contains(&slot) {
                virtual_item_changes.push((
                    slot - 15,
                    item.as_ref().map_or(0, |item| item.entry_id as i32),
                    0,
                    0,
                ));
            }
        }
        if !top_level_changes.is_empty() {
            self.send_player_values_update_from_entity_bridge(
                &top_level_changes,
                &visible_item_changes,
                &virtual_item_changes,
                &[],
                None,
            );
        }
        if gear_changed {
            self.send_stat_update();
        }
    }
}
