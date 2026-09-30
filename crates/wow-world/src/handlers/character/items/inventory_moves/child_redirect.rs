//! child redirect for the existing inventory_moves owner.

use super::*;

impl WorldSession {

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
}
