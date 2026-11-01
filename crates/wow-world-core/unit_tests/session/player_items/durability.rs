use std::sync::Arc;

use crate::session::state::SessionCatalogs;
use wow_data::{
    ItemRandomPropertyTemplateEntry, ItemRecord, ItemSparseTemplateEntry, ItemStatsStore, ItemStore,
};

fn chest_catalogs(basic: bool, random: bool) -> SessionCatalogs {
    let mut catalogs = SessionCatalogs::default();
    if basic {
        catalogs.items.store = Some(Arc::new(ItemStore::from_records([ItemRecord {
            id: 2121,
            class_id: 4,
            subclass_id: 1,
            material: 0,
            inventory_type: 5,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        }])));
    }
    catalogs.items.stats_store = Some(Arc::new(
        ItemStatsStore::from_stats_sparse_and_random_property_templates(
            [],
            [(
                2121,
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
                    price_random_value: 0.0,
                    max_durability: 40,
                    other_faction_item_id: 0,
                    content_tuning_id: 0,
                    player_level_to_item_level_curve_id: 0,
                    limit_category: 0,
                    instance_bound: 0,
                    zone_bound: [0; 2],
                    required_reputation_faction: 0,
                    allowable_class: 0,
                    required_expansion: 0,
                    bonding: 0,
                    container_slots: 0,
                    inventory_type: 5,
                },
            )],
            random.then_some((
                2121,
                ItemRandomPropertyTemplateEntry {
                    item_level: 5,
                    quality: 1,
                    inventory_type: 5,
                },
            )),
        ),
    ));
    catalogs
}

#[test]
fn complete_item_template_uses_cpp_effective_max_without_changing_raw_sparse() {
    let catalogs = chest_catalogs(true, true);
    assert_eq!(catalogs.item_template_max_durability(2121), 60);
    assert_eq!(
        catalogs
            .items
            .stats_store
            .as_ref()
            .unwrap()
            .sparse_template(2121)
            .unwrap()
            .max_durability,
        40
    );
    assert_eq!(catalogs.item_template_max_durability(999), 0);
}

#[test]
fn partial_item_catalogs_retain_raw_sparse_compatibility_only() {
    assert_eq!(
        chest_catalogs(false, true).item_template_max_durability(2121),
        40
    );
    assert_eq!(
        chest_catalogs(true, false).item_template_max_durability(2121),
        40
    );
    assert_eq!(
        SessionCatalogs::default().item_template_max_durability(2121),
        0
    );

    let mut catalogs = chest_catalogs(true, true);
    let mut nondurable = catalogs
        .items
        .store
        .as_ref()
        .unwrap()
        .get(2121)
        .unwrap()
        .clone();
    nondurable.class_id = 0;
    catalogs.items.store = Some(Arc::new(ItemStore::from_records([nondurable])));
    assert_eq!(
        catalogs.item_template_max_durability(2121),
        0,
        "complete metadata reporting a nondurable item must not fall back to raw40"
    );
}
