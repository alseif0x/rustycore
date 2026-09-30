//! validation for the existing inventory_moves owner.

use super::*;

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
}
