//! Player inventory and equipment position definitions.

use super::{
    is_bag_storage_slot, EQUIPMENT_SLOT_END, INVENTORY_SLOT_BAG_0, NULL_SLOT,
    PROFESSION_SLOT_END, PROFESSION_SLOT_START,
};

pub const PLAYER_SLOT_END: usize = 141;
pub const INVENTORY_DEFAULT_SIZE: u8 = 16;
pub const INVENTORY_SLOT_BAG_START: u8 = 30;
pub const INVENTORY_SLOT_BAG_END: u8 = 34;
pub const REAGENT_BAG_SLOT_START: u8 = 34;
pub const REAGENT_BAG_SLOT_END: u8 = 35;
pub const INVENTORY_SLOT_ITEM_START: u8 = 35;
pub const INVENTORY_SLOT_ITEM_END: u8 = 59;
pub const BANK_SLOT_ITEM_START: u8 = 59;
pub const BANK_SLOT_ITEM_END: u8 = 87;
pub const BANK_SLOT_BAG_START: u8 = 87;
pub const BANK_SLOT_BAG_END: u8 = 94;
pub const BUYBACK_SLOT_START: u8 = 94;
pub const BUYBACK_SLOT_END: u8 = 106;
pub const BUYBACK_SLOT_COUNT: usize = (BUYBACK_SLOT_END - BUYBACK_SLOT_START) as usize;
pub const KEYRING_SLOT_START: u8 = 106;
pub const KEYRING_SLOT_END: u8 = 138;
pub const CHILD_EQUIPMENT_SLOT_START: u8 = 138;
pub const CHILD_EQUIPMENT_SLOT_END: u8 = 141;
pub const ITEM_LIMIT_CATEGORY_MODE_HAVE: u8 = 0;
pub const ITEM_LIMIT_CATEGORY_MODE_EQUIP: u8 = 1;

pub const fn make_item_pos(bag: u8, slot: u8) -> u16 {
    u16::from_be_bytes([bag, slot])
}

pub fn is_inventory_pos(bag: u8, slot: u8) -> bool {
    if bag == INVENTORY_SLOT_BAG_0 && slot == NULL_SLOT {
        return true;
    }
    if bag == INVENTORY_SLOT_BAG_0
        && (INVENTORY_SLOT_ITEM_START..INVENTORY_SLOT_ITEM_END).contains(&slot)
    {
        return true;
    }
    if (INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END).contains(&bag) {
        return true;
    }
    if bag == INVENTORY_SLOT_BAG_0 && (KEYRING_SLOT_START..KEYRING_SLOT_END).contains(&slot) {
        return true;
    }
    if is_child_equipment_pos(bag, slot) {
        return true;
    }
    false
}

pub fn is_inventory_packed_pos(pos: u16) -> bool {
    let [bag, slot] = pos.to_be_bytes();
    is_inventory_pos(bag, slot)
}

pub fn is_equipment_pos(bag: u8, slot: u8) -> bool {
    if bag == INVENTORY_SLOT_BAG_0 && slot < EQUIPMENT_SLOT_END {
        return true;
    }
    if bag == INVENTORY_SLOT_BAG_0 && (PROFESSION_SLOT_START..PROFESSION_SLOT_END).contains(&slot) {
        return true;
    }
    if bag == INVENTORY_SLOT_BAG_0
        && (INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END).contains(&slot)
    {
        return true;
    }
    if bag == INVENTORY_SLOT_BAG_0 && (REAGENT_BAG_SLOT_START..REAGENT_BAG_SLOT_END).contains(&slot)
    {
        return true;
    }
    false
}

pub fn is_equipment_packed_pos(pos: u16) -> bool {
    let [bag, slot] = pos.to_be_bytes();
    is_equipment_pos(bag, slot)
}

pub fn is_bank_pos(bag: u8, slot: u8) -> bool {
    if bag == INVENTORY_SLOT_BAG_0 && (BANK_SLOT_ITEM_START..BANK_SLOT_ITEM_END).contains(&slot) {
        return true;
    }
    if bag == INVENTORY_SLOT_BAG_0 && (BANK_SLOT_BAG_START..BANK_SLOT_BAG_END).contains(&slot) {
        return true;
    }
    if (BANK_SLOT_BAG_START..BANK_SLOT_BAG_END).contains(&bag) {
        return true;
    }
    false
}

pub fn is_bank_packed_pos(pos: u16) -> bool {
    let [bag, slot] = pos.to_be_bytes();
    is_bank_pos(bag, slot)
}

pub fn is_bag_pos(pos: u16) -> bool {
    let [bag, slot] = pos.to_be_bytes();
    bag == INVENTORY_SLOT_BAG_0 && is_bag_storage_slot(slot)
}

pub fn is_child_equipment_pos(bag: u8, slot: u8) -> bool {
    bag == INVENTORY_SLOT_BAG_0
        && (CHILD_EQUIPMENT_SLOT_START..CHILD_EQUIPMENT_SLOT_END).contains(&slot)
}

pub fn is_child_equipment_packed_pos(pos: u16) -> bool {
    let [bag, slot] = pos.to_be_bytes();
    is_child_equipment_pos(bag, slot)
}
