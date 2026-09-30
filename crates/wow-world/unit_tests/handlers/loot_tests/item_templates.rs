//! Shared item-template fixtures for loot-handler tests.

use crate::session::WorldSession;
use std::sync::Arc;
use wow_constants::{InventoryType, ItemBondingType, ItemClass, ItemQuality};
use wow_data::{
    ItemDisenchantLootEntry, ItemDisenchantLootStore, ItemRandomPropertyTemplateEntry, ItemRecord,
    ItemSparseTemplateEntry, ItemStatsStore, ItemStore,
};

pub(super) fn install_limited_test_item_template(
    session: &mut WorldSession,
    entry: u32,
    max_count: i32,
) {
    install_limited_test_item_template_with_flags2(session, entry, max_count, 0);
}

pub(super) fn install_limited_test_item_template_with_flags2(
    session: &mut WorldSession,
    entry: u32,
    max_count: i32,
    flags2: u32,
) {
    install_limited_test_item_template_with_flags2_and_bonding(
        session,
        entry,
        max_count,
        flags2,
        ItemBondingType::None,
    );
}

pub(super) fn install_limited_test_item_template_with_flags2_and_bonding(
    session: &mut WorldSession,
    entry: u32,
    max_count: i32,
    flags2: u32,
    bonding: ItemBondingType,
) {
    session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
        id: entry,
        class_id: ItemClass::Consumable as u8,
        subclass_id: 0,
        material: 0,
        inventory_type: InventoryType::NonEquip as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([(
        entry,
        ItemSparseTemplateEntry {
            flags: [0, flags2, 0, 0],
            bag_family: 0,
            start_quest_id: 0,
            stackable: 20,
            max_count,
            lock_id: 0,
            required_reputation_rank: 0,
            sell_price: 0,
            buy_price: 0,
            vendor_stack_count: 1,
            price_variance: 1.0,
            price_random_value: 1.0,
            max_durability: 0,
            other_faction_item_id: 0,
            content_tuning_id: 0,
            player_level_to_item_level_curve_id: 0,
            limit_category: 0,
            instance_bound: 0,
            zone_bound: [0, 0],
            required_reputation_faction: 0,
            allowable_class: -1,
            required_expansion: 0,
            bonding: bonding as u8,
            container_slots: 0,
            inventory_type: InventoryType::NonEquip as i8,
        },
    )])));
}

pub(super) fn install_disenchantable_test_item_template(session: &mut WorldSession, entry: u32) {
    session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
        id: entry,
        class_id: ItemClass::Armor as u8,
        subclass_id: 0,
        material: 0,
        inventory_type: InventoryType::Chest as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }])));
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [(
                entry,
                ItemSparseTemplateEntry {
                    flags: [0, 0, 0, 0],
                    bag_family: 0,
                    start_quest_id: 0,
                    stackable: 1,
                    max_count: 0,
                    lock_id: 0,
                    required_reputation_rank: 0,
                    sell_price: 1,
                    buy_price: 0,
                    vendor_stack_count: 1,
                    price_variance: 1.0,
                    price_random_value: 1.0,
                    max_durability: 0,
                    other_faction_item_id: 0,
                    content_tuning_id: 0,
                    player_level_to_item_level_curve_id: 0,
                    limit_category: 0,
                    instance_bound: 0,
                    zone_bound: [0, 0],
                    required_reputation_faction: 0,
                    allowable_class: -1,
                    required_expansion: 0,
                    bonding: ItemBondingType::None as u8,
                    container_slots: 0,
                    inventory_type: InventoryType::Chest as i8,
                },
            )],
            [(
                entry,
                ItemRandomPropertyTemplateEntry {
                    item_level: 10,
                    quality: ItemQuality::Rare as i8,
                    inventory_type: InventoryType::Chest as i8,
                },
            )],
        ),
    ));
    session.set_item_disenchant_loot_store(Arc::new(ItemDisenchantLootStore::from_entries([
        ItemDisenchantLootEntry {
            id: 901,
            subclass: 0,
            quality: ItemQuality::Rare as u8,
            min_level: 1,
            max_level: 20,
            skill_required: 175,
            expansion_id: -2,
            class_id: ItemClass::Armor as u32,
        },
    ])));
}

pub(super) fn test_item_record(
    item_id: u32,
    random_select: u16,
    random_suffix_group_id: u16,
) -> ItemRecord {
    ItemRecord {
        id: item_id,
        class_id: 2,
        subclass_id: 7,
        material: 0,
        inventory_type: InventoryType::Chest as i8,
        sheathe_type: 0,
        random_select,
        random_suffix_group_id,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }
}
