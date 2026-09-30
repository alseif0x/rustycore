use super::*;

pub(in crate::player::collection_state::tests) fn install_appearance_test_item(
    session: &mut AdmissionFixture,
    item_id: u32,
    class_id: ItemClass,
    subclass_id: u8,
    inventory_type: InventoryType,
    quality: ItemQuality,
    flags: [u32; 4],
    allowable_class: i16,
) {
    install_appearance_test_items(
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

pub(in crate::player::collection_state::tests) fn install_appearance_test_items<const N: usize>(
    session: &mut AdmissionFixture,
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
    session.items = Some(Arc::new(ItemStore::from_records(
        items.iter().copied().map(
            |(item_id, class_id, subclass_id, inventory_type, _, _, _)| ItemRecord {
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
        ),
    )));
    session.search = Some(Arc::new(ItemSearchNameStore::from_entries(
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
    session.stats = Some(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            items.iter().copied().map(
                |(item_id, _, _, inventory_type, _, flags, allowable_class)| {
                    (
                        item_id,
                        ItemSparseTemplateEntry {
                            flags,
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
                            allowable_class,
                            required_expansion: 0,
                            bonding: ItemBondingType::None as u8,
                            container_slots: 0,
                            inventory_type: inventory_type as i8,
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
