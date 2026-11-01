// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The inventory move vocabulary and pure item helpers the moved item handlers
//! (`item_session`, #1363 slice 8c) name. Moved verbatim from the application's
//! `inventory_swap/{storage_move,equip_contracts}.rs` and
//! `inventory_move_planning.rs`, which re-export them at their old paths.

use wow_entities::is_bank_pos;

use crate::item_spell_charges_db_string;

/// Which storage the move targets. Moved out of the character handler module
/// with the operation it belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryStorageTargetLikeCpp {
    Inventory,
    Bank,
}

/// Which C++ quest checks `_StoreItem`/`_BankItem` run for this caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryStorageQuestChecksLikeCpp {
    None,
    AutoBankItemRemoved,
    AutoStoreBankItemAdded,
}

/// C++ `HandleAutoStoreBankItemOpcode` targets the opposite storage of the
/// source position.
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventorySwapTargetLikeCpp {
    Inventory,
    Bank,
    Equipment { dest: u16 },
    None,
}

pub fn bind_inventory_item_for_destination_like_cpp(
    item: &mut wow_entities::Item,
    destination: u16,
) {
    let [bag, slot] = destination.to_be_bytes();
    if wow_entities::is_equipment_pos(bag, slot) {
        // C++ Player::EquipItem/VisualizeItem binds OnEquip and acquire/quest modes.
        item.bind_if_visualized();
    } else {
        item.bind_if_stored(wow_entities::is_bag_pos(destination));
    }
}

pub fn item_dynamic_flags_changed_like_cpp(
    before: &wow_entities::Item,
    after: &wow_entities::Item,
) -> bool {
    before.item_flags_bits() != after.item_flags_bits()
}

pub fn item_storage_mutable_persistence_like_cpp(
    db_guid: u64,
    item: &wow_entities::Item,
    count: u32,
    flags: u32,
    enchantments: String,
    effect_count: usize,
) -> wow_persistence::InventoryItemMutablePersistenceLikeCpp {
    let data = item.data();
    wow_persistence::InventoryItemMutablePersistenceLikeCpp {
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
