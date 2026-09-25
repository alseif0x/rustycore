//! Player item and equipment fixtures.
//!
//! These builders retain the original session-test behavior and are
//! visible only within the parent `session::tests` subtree.

use super::*;

pub(in crate::session::tests) fn install_remove_spell_offhand_templates_like_cpp(
    session: &mut WorldSession,
    items: &[(u32, InventoryType, u32, ItemClass, u8)],
) {
    session.set_item_store(Arc::new(ItemStore::from_records(items.iter().map(
        |&(item_id, inventory_type, _, class_id, subclass_id)| ItemRecord {
            id: item_id,
            class_id: class_id as u8,
            subclass_id,
            material: 0,
            inventory_type: inventory_type as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
    ))));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates(
        items
            .iter()
            .map(|&(item_id, inventory_type, flags3, _, _)| {
                (
                    item_id,
                    ItemSparseTemplateEntry {
                        flags: [0, 0, flags3, 0],
                        bag_family: 0,
                        start_quest_id: 0,
                        stackable: 1,
                        max_count: 0,
                        lock_id: 0,
                        required_reputation_rank: 0,
                        sell_price: 0,
                        buy_price: 0,
                        vendor_stack_count: 1,
                        price_variance: 0.0,
                        price_random_value: 0.0,
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
                        container_slots: if inventory_type == InventoryType::Bag {
                            4
                        } else {
                            0
                        },
                        inventory_type: inventory_type as i8,
                    },
                )
            }),
    )));
}

pub(in crate::session::tests) fn sparse_template_for_inventory_type_like_cpp(
    inventory_type: InventoryType,
    flags3: u32,
) -> ItemSparseTemplateEntry {
    sparse_template_with_scaling_like_cpp(inventory_type, flags3, 0, 0)
}

pub(in crate::session::tests) fn sparse_template_with_scaling_like_cpp(
    inventory_type: InventoryType,
    flags3: u32,
    content_tuning_id: i32,
    player_level_to_item_level_curve_id: i32,
) -> ItemSparseTemplateEntry {
    ItemSparseTemplateEntry {
        flags: [0, 0, flags3, 0],
        bag_family: 0,
        start_quest_id: 0,
        stackable: 1,
        max_count: 0,
        lock_id: 0,
        required_reputation_rank: 0,
        sell_price: 0,
        buy_price: 0,
        vendor_stack_count: 1,
        price_variance: 0.0,
        price_random_value: 0.0,
        max_durability: 0,
        other_faction_item_id: 0,
        content_tuning_id,
        player_level_to_item_level_curve_id,
        limit_category: 0,
        instance_bound: 0,
        zone_bound: [0, 0],
        required_reputation_faction: 0,
        allowable_class: -1,
        required_expansion: 0,
        bonding: ItemBondingType::None as u8,
        container_slots: if inventory_type == InventoryType::Bag {
            4
        } else {
            0
        },
        inventory_type: inventory_type as i8,
    }
}

pub(in crate::session::tests) fn equip_represented_test_item_like_cpp(
    session: &mut WorldSession,
    slot: u8,
    item_guid: ObjectGuid,
    item_id: u32,
    inventory_type: InventoryType,
) {
    let owner = session.player_guid().unwrap_or(ObjectGuid::EMPTY);
    let item = session.make_inventory_item_object(
        item_guid,
        item_id,
        owner,
        1,
        0,
        ItemContext::None,
        slot,
    );
    session.insert_inventory_item_object(item);
    session.insert_inventory_item_like_cpp(
        slot,
        InventoryItem {
            guid: item_guid,
            entry_id: item_id,
            db_guid: item_guid.counter() as u64,
            inventory_type: Some(inventory_type as u8),
        },
    );
}

pub(in crate::session::tests) fn install_represented_item_level_curve_fixture_like_cpp(
    session: &mut WorldSession,
    item_id: u32,
    content_tuning_id: i32,
    curve_id: i32,
) {
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [(
                item_id,
                sparse_template_with_scaling_like_cpp(
                    InventoryType::Chest,
                    0,
                    content_tuning_id,
                    curve_id,
                ),
            )],
            [(
                item_id,
                ItemRandomPropertyTemplateEntry {
                    item_level: 10,
                    quality: ItemQuality::Epic as i8,
                    inventory_type: InventoryType::Chest as i8,
                },
            )],
        ),
    ));
    session.set_curve_store(Arc::new(CurveStore::from_entries([CurveEntry {
        id: curve_id as u32,
        curve_type: 0,
        flags: 0,
    }])));
    session.set_curve_point_store(Arc::new(CurvePointStore::from_entries([
        CurvePointEntry {
            id: 1,
            pos: [1.0, 101.0],
            pre_sl_squish_pos: [0.0, 0.0],
            curve_id: curve_id as u32,
            order_index: 0,
        },
        CurvePointEntry {
            id: 2,
            pos: [60.0, 160.0],
            pre_sl_squish_pos: [0.0, 0.0],
            curve_id: curve_id as u32,
            order_index: 1,
        },
    ])));
}

pub(in crate::session::tests) fn represented_test_item_record_like_cpp(
    item_id: u32,
    inventory_type: InventoryType,
    class_id: ItemClass,
    subclass_id: u8,
) -> ItemRecord {
    ItemRecord {
        id: item_id,
        class_id: class_id as u8,
        subclass_id,
        material: 0,
        inventory_type: inventory_type as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }
}

pub(in crate::session::tests) fn install_represented_pvp_item_level_fixture_like_cpp(
    session: &mut WorldSession,
    item_id: u32,
    pvp_delta: u8,
) {
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [(
                item_id,
                sparse_template_for_inventory_type_like_cpp(InventoryType::Chest, 0),
            )],
            [(
                item_id,
                ItemRandomPropertyTemplateEntry {
                    item_level: 100,
                    quality: ItemQuality::Epic as i8,
                    inventory_type: InventoryType::Chest as i8,
                },
            )],
        ),
    ));
    session.set_pvp_item_store(Arc::new(PvpItemStore::from_entries([PvpItemEntry {
        id: 1,
        item_id: item_id as i32,
        item_level_delta: pvp_delta,
    }])));
}

pub(in crate::session::tests) fn represented_item_level_area_map_like_cpp(
    map_id: u32,
    instance_type: i8,
    flags2: u32,
) -> wow_data::map::MapEntry {
    wow_data::map::MapEntry {
        id: map_id,
        instance_type,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2,
    }
}

pub(in crate::session::tests) fn install_stackable_test_item_template(
    session: &mut WorldSession,
    entry: u32,
    max_stack_size: i32,
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
            flags: [0, 0, 0, 0],
            bag_family: 0,
            start_quest_id: 0,
            stackable: max_stack_size,
            max_count: 0,
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
            bonding: ItemBondingType::None as u8,
            container_slots: 0,
            inventory_type: InventoryType::NonEquip as i8,
        },
    )])));
}
