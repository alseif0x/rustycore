use super::InventoryPlayerSnapshotLikeCpp;
use crate::ItemStorageTemplate;
use crate::player::items::storage::ItemStorageView;
use crate::player::{
    CanBankItemArgs, CanEquipItemArgs, CanEquipItemOutcome, CanEquipUniqueItemArgs,
    CanStoreItemArgs, CanStoreItemOutcome, CanUnequipItemArgs, CanUseItemArgs, EQUIPMENT_SLOT_END,
    ItemPosCount, ItemSearchCallbackResult, ItemSearchLocation, PlayerStorageError,
    SwapItemPreflightItem, SwapItemPreflightPlan, is_bag_storage_slot,
};
use wow_constants::InventoryResult;
use wow_core::ObjectGuid;

impl InventoryPlayerSnapshotLikeCpp {
    fn item_storage_view(&self) -> ItemStorageView<'_> {
        ItemStorageView::new(
            self.owner_guid,
            self.inventory.as_ref(),
            self.active_player_data.num_backpack_slots,
        )
    }

    pub fn is_valid_pos(&self, bag: u8, slot: u8, explicit_pos: bool) -> bool {
        self.item_storage_view()
            .is_valid_pos(bag, slot, explicit_pos)
    }

    pub fn for_each_item_guid(
        &self,
        location: ItemSearchLocation,
        callback: impl FnMut(ObjectGuid) -> ItemSearchCallbackResult,
    ) -> bool {
        self.item_storage_view()
            .for_each_item_guid(location, callback)
    }

    pub fn can_store_item(
        &self,
        dest: &mut Vec<ItemPosCount>,
        args: CanStoreItemArgs<'_>,
    ) -> CanStoreItemOutcome {
        self.item_storage_view().can_store_item(dest, args)
    }

    pub fn can_bank_item(
        &self,
        dest: &mut Vec<ItemPosCount>,
        args: CanBankItemArgs<'_>,
    ) -> InventoryResult {
        self.item_storage_view()
            .can_bank_item(dest, args, self.player_data.num_bank_slots)
    }

    pub fn can_use_item(&self, args: CanUseItemArgs<'_>) -> InventoryResult {
        self.item_storage_view().can_use_item(args)
    }

    pub fn can_unequip_item(&self, args: CanUnequipItemArgs<'_>) -> InventoryResult {
        self.item_storage_view().can_unequip_item(args)
    }

    pub fn can_equip_item(&self, args: CanEquipItemArgs<'_>) -> CanEquipItemOutcome {
        self.item_storage_view().can_equip_item(args)
    }

    pub fn can_equip_unique_item(&self, args: CanEquipUniqueItemArgs<'_>) -> InventoryResult {
        self.item_storage_view().can_equip_unique_item(args)
    }

    pub fn is_two_hand_used_template(&self, main_template: Option<&ItemStorageTemplate>) -> bool {
        self.item_storage_view()
            .is_two_hand_used_template(main_template, self.can_titan_grip)
    }

    pub fn swap_item_preflight_plan(
        &self,
        src: u16,
        dst: u16,
        is_alive: bool,
        src_item: Option<SwapItemPreflightItem>,
        dst_item: Option<SwapItemPreflightItem>,
    ) -> SwapItemPreflightPlan {
        self.item_storage_view()
            .swap_item_preflight_plan(src, dst, is_alive, src_item, dst_item)
    }

    pub fn primary_specialization_id_like_cpp(&self) -> u32 {
        self.player_data.current_spec_id
    }

    pub fn set_can_titan_grip_like_cpp(&mut self, value: bool) {
        self.can_titan_grip = value;
    }

    pub fn register_bag_storage(
        &mut self,
        bag_slot: u8,
        bag_guid: ObjectGuid,
        bag_size: u8,
    ) -> Result<(), PlayerStorageError> {
        self.inventory
            .register_bag_storage(bag_slot, bag_guid, bag_size)
    }

    pub fn store_bag_item(
        &mut self,
        bag: u8,
        slot: u8,
        guid: ObjectGuid,
    ) -> Result<(), PlayerStorageError> {
        self.inventory.store_bag_item(bag, slot, guid)
    }

    pub fn remove_bag_item(
        &mut self,
        bag: u8,
        slot: u8,
    ) -> Result<Option<ObjectGuid>, PlayerStorageError> {
        self.inventory.remove_bag_item(bag, slot)
    }

    pub fn store_top_level_item(
        &mut self,
        slot: u8,
        guid: ObjectGuid,
    ) -> Result<(), PlayerStorageError> {
        self.inventory
            .store_top_level_item_native_like_cpp(slot, guid)?;
        self.set_inv_slot(slot as usize, guid);
        Ok(())
    }

    pub fn remove_top_level_item(
        &mut self,
        slot: u8,
    ) -> Result<Option<ObjectGuid>, PlayerStorageError> {
        let removed = self.inventory.take_top_level_item_native_like_cpp(slot)?;
        self.set_inv_slot(slot as usize, ObjectGuid::EMPTY);
        if slot < EQUIPMENT_SLOT_END {
            self.set_visible_item_slot(slot, None);
        }
        if is_bag_storage_slot(slot) {
            self.inventory.bags[slot as usize] = None;
        }
        Ok(removed)
    }
}
