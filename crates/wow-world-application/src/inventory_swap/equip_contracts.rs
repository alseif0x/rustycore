// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

pub fn bind_inventory_item_for_destination_like_cpp(item: &mut wow_entities::Item, destination: u16) {
    let [bag, slot] = destination.to_be_bytes();
    if wow_entities::is_equipment_pos(bag, slot) {
        // C++ Player::EquipItem/VisualizeItem binds OnEquip and acquire/quest modes.
        item.bind_if_visualized();
    } else {
        item.bind_if_stored(wow_entities::is_bag_pos(destination));
    }
}

pub fn item_dynamic_flags_changed_like_cpp(before: &wow_entities::Item, after: &wow_entities::Item) -> bool {
    before.item_flags_bits() != after.item_flags_bits()
}

pub fn item_spell_charges_db_string(charges: &[i32], effect_count: usize) -> String {
    let mut out = String::new();
    for charge in charges.iter().take(effect_count) {
        out.push_str(&charge.to_string());
        out.push(' ');
    }
    out
}

pub fn item_storage_mutable_persistence_like_cpp(
    db_guid: u64, item: &wow_entities::Item, count: u32, flags: u32,
    enchantments: String, effect_count: usize,
) -> wow_persistence::InventoryItemMutablePersistenceLikeCpp {
    let data = item.data();
    wow_persistence::InventoryItemMutablePersistenceLikeCpp {
        item_guid: db_guid, count, expiration: data.expiration,
        charges: item_spell_charges_db_string(&data.spell_charges, effect_count),
        flags, enchantments, durability: data.durability, played_time: data.create_played_time,
    }
}
