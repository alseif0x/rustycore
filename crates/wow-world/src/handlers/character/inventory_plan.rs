//! Inventory and bank storage decisions: swap targets, autostore, equipment cache,
//! item persistence views and the money/item helpers the handlers use.
//!
//! Split out of `character/mod.rs` under #584 (B5); items are unchanged.

use super::*;

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectInventoryPositionUpdateLikeCpp {
    pub slot: u8,
    pub item_db_guid: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventorySwapTargetLikeCpp {
    Inventory,
    Bank,
    Equipment { dest: u16 },
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryStorageTargetLikeCpp {
    Inventory,
    Bank,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryStorageQuestChecksLikeCpp {
    None,
    AutoBankItemRemoved,
    AutoStoreBankItemAdded,
}

pub fn autostore_bank_target_like_cpp(
    source_bag: u8,
    source_slot: u8,
) -> InventoryStorageTargetLikeCpp {
    if is_bank_pos(source_bag, source_slot) {
        InventoryStorageTargetLikeCpp::Inventory
    } else {
        InventoryStorageTargetLikeCpp::Bank
    }
}

pub fn autostore_bank_quest_checks_like_cpp(
    target: InventoryStorageTargetLikeCpp,
) -> InventoryStorageQuestChecksLikeCpp {
    if target == InventoryStorageTargetLikeCpp::Inventory {
        InventoryStorageQuestChecksLikeCpp::AutoStoreBankItemAdded
    } else {
        // C++ HandleAutoStoreBankItemOpcode intentionally does not call
        // ItemRemovedQuestCheck in its inventory-to-bank branch.
        InventoryStorageQuestChecksLikeCpp::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InventoryEquipChildPlanLikeCpp {
    pub child_guid: ObjectGuid,
    pub destination_slot: u8,
    pub displaced_storage: Option<(u8, u8, InventoryStorageTargetLikeCpp)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventorySwapStepLikeCpp {
    Done,
    ChildRedirect {
        first_src: u16,
        first_dst: u16,
        second_src: u16,
        second_dst: u16,
    },
}

#[cfg(test)]
pub fn plan_direct_inventory_swap_persistence_like_cpp(
    src: u8,
    dst: u8,
    src_item: Option<&InventoryItem>,
    dst_item: Option<&InventoryItem>,
) -> Vec<DirectInventoryPositionUpdateLikeCpp> {
    let mut updates = Vec::with_capacity(2);
    if let Some(item) = src_item {
        updates.push(DirectInventoryPositionUpdateLikeCpp {
            slot: dst,
            item_db_guid: item.db_guid,
        });
    }
    if let Some(item) = dst_item {
        updates.push(DirectInventoryPositionUpdateLikeCpp {
            slot: src,
            item_db_guid: item.db_guid,
        });
    }
    updates
}

pub fn loaded_inventory_slot_count_with_legacy_rust_compat(saved_slots: u8) -> u8 {
    // C++ loads the saved value directly, but TrinityCore's schema defaults
    // inventorySlots to the base backpack size. Older RustyCore builds
    // explicitly inserted zero before this field was wired; keep those
    // already-created characters playable without an out-of-band migration.
    if saved_slots == 0 {
        INVENTORY_DEFAULT_SIZE
    } else {
        saved_slots
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadedItemRandomPropertiesLikeCpp {
    pub id: i32,
    pub seed: i32,
}

pub fn bank_store_item_added_quest_count_like_cpp(plan: &InventoryStorageMovePlanLikeCpp) -> u32 {
    // C++ HandleAutoStoreBankItemOpcode passes storedItem->GetCount() after
    // StoreItem. _StoreItem returns the last destination item, so a full merge
    // reports that destination stack's total and a merge+remainder reports the
    // final remainder stack count. This is deliberately not source_count.
    plan.moved_destination
        .map(|(_, _, count)| count)
        .or_else(|| plan.existing_updates.last().map(|update| update.new_count))
        .unwrap_or(0)
}

pub fn bank_store_destination_applies_obtain_spells_like_cpp(bag: u8) -> bool {
    // C++ Player::_StoreItem checks only the bag value. INVENTORY_SLOT_BAG_0
    // therefore includes top-level personal-bank slots as well as carried
    // top-level slots; bank-bag containers remain excluded.
    bag == INVENTORY_SLOT_BAG_0
        || (wow_entities::INVENTORY_SLOT_BAG_START..wow_entities::INVENTORY_SLOT_BAG_END)
            .contains(&bag)
}

pub fn inventory_storage_move_quest_directions_like_cpp(
    source_bag: u8,
    source_slot: u8,
    target: InventoryStorageTargetLikeCpp,
) -> (bool, bool) {
    let moving_to_bank = target == InventoryStorageTargetLikeCpp::Bank;
    let moving_from_bank = !moving_to_bank && is_bank_pos(source_bag, source_slot);
    (moving_to_bank, moving_from_bank)
}

pub type ItemStorageMutablePersistenceLikeCpp =
    wow_persistence::InventoryItemMutablePersistenceLikeCpp;

/// Reverse-map an equipment slot (0-18) to its InventoryType.
///
/// Used as a fallback when Item.db2 store is not available.
pub fn slot_to_inventory_type(slot: u8) -> Option<u8> {
    match slot {
        0 => Some(1),        // Head
        1 => Some(2),        // Neck
        2 => Some(3),        // Shoulders
        3 => Some(4),        // Body (Shirt)
        4 => Some(5),        // Chest
        5 => Some(6),        // Waist
        6 => Some(7),        // Legs
        7 => Some(8),        // Feet
        8 => Some(9),        // Wrists
        9 => Some(10),       // Hands
        10 | 11 => Some(11), // Finger (Ring)
        12 | 13 => Some(12), // Trinket
        14 => Some(16),      // Cloak
        15 => Some(21),      // MainHand (WeaponMainHand)
        16 => Some(22),      // OffHand (WeaponOffHand)
        17 => Some(15),      // Ranged
        18 => Some(19),      // Tabard
        _ => None,
    }
}

/// Parse a space-separated equipment cache string into VisualItemInfo array.
///
/// C++ `EnumCharactersResult::CharacterInfo` parses `equipmentCache` as five
/// fields per slot: InvType, DisplayID, DisplayEnchantID, Subclass, and
/// SecondaryItemModifiedAppearanceID.
pub fn parse_equipment_cache(cache: &str) -> [VisualItemInfo; 34] {
    let mut equipment = [VisualItemInfo::default(); 34];
    if cache.is_empty() {
        return equipment;
    }

    let parts: Vec<&str> = cache.split_whitespace().collect();
    let fields_per_slot = 5;

    for slot in 0..34 {
        let base = slot * fields_per_slot;
        if base + fields_per_slot > parts.len() {
            break;
        }
        equipment[slot] = VisualItemInfo {
            inv_type: parts[base].parse().unwrap_or(0),
            display_id: parts[base + 1].parse().unwrap_or(0),
            display_enchant_id: parts[base + 2].parse().unwrap_or(0),
            subclass: parts[base + 3].parse().unwrap_or(0),
            secondary_item_modified_appearance_id: parts[base + 4].parse().unwrap_or(0),
        };
    }

    equipment
}

pub fn bind_inventory_item_for_destination_like_cpp(
    item: &mut wow_entities::Item,
    destination: u16,
) {
    let [bag, slot] = destination.to_be_bytes();
    if is_equipment_pos(bag, slot) {
        // C++ `Player::EquipItem` calls `VisualizeItem`, which binds
        // BIND_ON_EQUIP as well as the acquire/quest bonding modes.
        item.bind_if_visualized();
    } else {
        // C++ `Player::_StoreItem` has the narrower storage rule: an
        // OnEquip item binds here only when stored in a bag-equipment slot.
        item.bind_if_stored(wow_entities::is_bag_pos(destination));
    }
}

pub fn item_dynamic_flags_changed_like_cpp(
    before: &wow_entities::Item,
    after: &wow_entities::Item,
) -> bool {
    before.item_flags_bits() != after.item_flags_bits()
}

pub fn player_money_gain_like_cpp(current_money: u64, amount: u64) -> Option<u64> {
    if amount == 0 {
        return Some(current_money);
    }

    let max_gain = MAX_MONEY_AMOUNT.checked_sub(amount)?;
    if current_money <= max_gain {
        Some(current_money + amount)
    } else {
        None
    }
}

pub fn item_storage_mutable_persistence_like_cpp(
    db_guid: u64,
    item: &wow_entities::Item,
    count: u32,
    flags: u32,
    enchantments: String,
    effect_count: usize,
) -> ItemStorageMutablePersistenceLikeCpp {
    let data = item.data();
    ItemStorageMutablePersistenceLikeCpp {
        item_guid: db_guid,
        count,
        expiration: data.expiration,
        charges: item_spell_charges_db_string(&data.spell_charges, effect_count),
        flags,
        enchantments,
        durability: data.durability,
        played_time: data.create_played_time,
    }
}

pub fn item_is_currently_looted_like_cpp(item: &wow_entities::Item) -> bool {
    item.loot_generated()
}

pub fn item_is_not_empty_bag_like_cpp(
    inventory_type: Option<InventoryType>,
    contains_items: bool,
) -> bool {
    matches!(inventory_type, Some(InventoryType::Bag)) && contains_items
}
