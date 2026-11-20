// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::sync::Arc;

use super::*;
use wow_data::{
    CharacterLoadoutEntry, CharacterLoadoutItemEntry, CharacterLoadoutItemStore,
    CharacterLoadoutStore, ItemAppearanceEntry, ItemAppearanceStore, ItemModifiedAppearanceEntry,
    ItemModifiedAppearanceStore, ItemRecord, ItemSparseTemplateEntry, ItemStatsStore, ItemStore,
};
use wow_persistence::forever::creation::ItemOverride;

fn sparse(inventory_type: i8, durability: u32) -> ItemSparseTemplateEntry {
    ItemSparseTemplateEntry {
        flags: [0; 4],
        bag_family: 0,
        start_quest_id: 0,
        stackable: 1,
        max_count: 0,
        lock_id: 0,
        required_reputation_rank: 0,
        sell_price: 0,
        buy_price: 0,
        vendor_stack_count: 1,
        price_variance: 1.0,
        price_random_value: 1.0,
        max_durability: durability,
        other_faction_item_id: 0,
        content_tuning_id: 0,
        player_level_to_item_level_curve_id: 0,
        limit_category: 0,
        instance_bound: 0,
        zone_bound: [0; 2],
        required_reputation_faction: 0,
        allowable_class: -1,
        required_expansion: 0,
        bonding: 0,
        container_slots: 0,
        inventory_type,
    }
}

fn item(id: u32, class_id: u8, subclass_id: u8) -> ItemRecord {
    ItemRecord {
        id,
        class_id,
        subclass_id,
        material: 0,
        inventory_type: 0,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }
}

fn modified(id: u32, item_id: u32, appearance_id: u32) -> ItemModifiedAppearanceEntry {
    ItemModifiedAppearanceEntry {
        id,
        item_id: item_id as i32,
        item_appearance_modifier_id: 0,
        item_appearance_id: appearance_id as i32,
        order_index: 0,
        transmog_source_type_enum: 0,
    }
}

fn appearance(id: u32, display_id: u32) -> ItemAppearanceEntry {
    ItemAppearanceEntry {
        id,
        display_type: 0,
        item_display_info_id: display_id as i32,
        default_icon_file_data_id: 0,
        ui_order: 0,
    }
}

#[test]
fn starting_items_combine_loadout_backpack_bag_and_overrides_like_cpp() {
    let mut catalogs = SessionCatalogs::default();
    catalogs.items.character_loadout_store = Some(Arc::new(CharacterLoadoutStore::from_entries([
        CharacterLoadoutEntry {
            id: 1,
            race_mask: 1,
            chr_class_id: 1,
            purpose: 9,
            item_context: 0,
        },
    ])));
    catalogs.items.character_loadout_item_store =
        Some(Arc::new(CharacterLoadoutItemStore::from_entries([
            CharacterLoadoutItemEntry {
                id: 1,
                character_loadout_id: 1,
                item_id: 38,
            },
            CharacterLoadoutItemEntry {
                id: 2,
                character_loadout_id: 1,
                item_id: 25,
            },
        ])));
    catalogs.items.store = Some(Arc::new(ItemStore::from_records([
        item(38, 4, 0),
        item(25, 2, 7),
        item(100, 1, 1),
        item(200, 0, 2),
    ])));
    catalogs.items.stats_store = Some(Arc::new(ItemStatsStore::from_sparse_templates([
        (38, sparse(4, 60)),
        (25, sparse(13, 55)),
        (100, sparse(18, 30)),
        (200, sparse(0, 0)),
    ])));
    catalogs.items.modified_appearance_store =
        Some(Arc::new(ItemModifiedAppearanceStore::from_entries([
            modified(38, 38, 380),
            modified(25, 25, 250),
            modified(100, 100, 1_000),
            modified(200, 200, 2_000),
        ])));
    catalogs.items.appearance_store = Some(Arc::new(ItemAppearanceStore::from_entries([
        appearance(380, 3_800),
        appearance(250, 2_500),
        appearance(1_000, 10_000),
        appearance(2_000, 20_000),
    ])));
    catalogs.items.starting_item_overrides = vec![
        ItemOverride {
            race: 0,
            class: 0,
            item: 200,
            amount: 1,
        },
        ItemOverride {
            race: 0,
            class: 0,
            item: 100,
            amount: 4,
        },
        ItemOverride {
            race: 0,
            class: 0,
            item: 25,
            amount: -1,
        },
        ItemOverride {
            race: 0,
            class: 0,
            item: 25,
            amount: 2,
        },
    ];
    let generator = ObjectGuidGenerator::new(wow_core::guid::HighGuid::Item, 1);

    let (items, cache) = project_starting_items_like_cpp(&catalogs, 1, 1, &generator);

    let mut actual: Vec<_> = items
        .iter()
        .map(|item| (item.item_id, item.slot, item.count, item.durability))
        .collect();
    actual.sort_by_key(|entry| entry.0);
    assert_eq!(
        actual,
        vec![
            (25, 15, 2, 55),
            (38, 3, 1, 60),
            (100, 30, 4, 30),
            (200, 19, 1, 0)
        ]
    );
    let guids: std::collections::HashSet<_> = items.iter().map(|item| item.guid).collect();
    assert_eq!(guids.len(), items.len());
    assert!(guids.iter().all(|guid| *guid > 0));
    let fields: Vec<i32> = cache
        .split_whitespace()
        .map(|field| field.parse().unwrap())
        .collect();
    assert_eq!(fields.len(), 34 * 5);
    assert_eq!(&fields[15..20], &[4, 3_800, 0, 0, 0]);
    assert_eq!(&fields[75..80], &[13, 2_500, 0, 7, 0]);
    assert_eq!(&fields[95..100], &[0, 0, 0, 0, 0]);
    assert_eq!(&fields[150..155], &[18, 10_000, 0, 1, 0]);
}
