use super::*;
use crate::item_fixture_rows::{basic_item_record, inventory_sparse_template};

pub(in crate::appearances) fn install_transmog_can_add_test_item(
    session: &mut WorldSession,
    item_id: u32,
    class_id: ItemClass,
    subclass_id: u8,
    inventory_type: InventoryType,
    quality: ItemQuality,
    flags: [u32; 4],
    allowable_class: i16,
) {
    install_transmog_can_add_test_items(
        session,
        [(
            item_id,
            class_id,
            subclass_id,
            inventory_type,
            quality,
            flags,
            allowable_class,
        )],
    );
}

pub(in crate::appearances) fn install_transmog_can_add_test_items<const N: usize>(
    session: &mut WorldSession,
    items: [(
        u32,
        ItemClass,
        u8,
        InventoryType,
        ItemQuality,
        [u32; 4],
        i16,
    ); N],
) {
    session.set_item_store(Arc::new(ItemStore::from_records(
        items.iter().copied().map(
            |(item_id, class_id, subclass_id, inventory_type, _, _, _)| basic_item_record(
                item_id,
                class_id as u8,
                subclass_id,
                inventory_type as i8,
            ),
        ),
    )));
    session.set_item_search_name_store(Arc::new(ItemSearchNameStore::from_entries(
        items
            .iter()
            .copied()
            .map(
                |(item_id, _, _, _, quality, flags, allowable_class)| ItemSearchNameEntry {
                    id: item_id,
                    allowable_race: 0,
                    display: String::new(),
                    overall_quality_id: quality as u8,
                    expansion_id: 0,
                    min_faction_id: 0,
                    min_reputation: 0,
                    allowable_class: i32::from(allowable_class),
                    required_level: 0,
                    required_skill: 0,
                    required_skill_rank: 0,
                    required_ability: 0,
                    item_level: 1,
                    flags: flags.map(|flag| flag as i32),
                },
            ),
    )));
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            items.iter().copied().map(
                |(item_id, _, _, inventory_type, _, flags, allowable_class)| {
                    (
                        item_id,
                        ItemSparseTemplateEntry {
                            flags: flags,
                            price_variance: 0.0,
                            price_random_value: 0.0,
                            zone_bound: [0, 0],
                            allowable_class: allowable_class,
                            ..inventory_sparse_template(inventory_type as i8)
                        },
                    )
                },
            ),
            items
                .iter()
                .copied()
                .map(|(item_id, _, _, inventory_type, quality, _, _)| {
                    (
                        item_id,
                        ItemRandomPropertyTemplateEntry {
                            item_level: 1,
                            quality: quality as i8,
                            inventory_type: inventory_type as i8,
                        },
                    )
                }),
        ),
    ));
}
