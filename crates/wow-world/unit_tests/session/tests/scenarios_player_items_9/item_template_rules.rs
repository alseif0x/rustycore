use super::*;

#[test]
fn item_storage_template_combines_basic_and_sparse_data_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
        id: 100,
        class_id: ItemClass::Container as u8,
        subclass_id: 0,
        material: 0,
        inventory_type: InventoryType::Weapon as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([(
        100,
        ItemSparseTemplateEntry {
            flags: [
                ItemFlags::IS_BOUND_TO_ACCOUNT.bits() as u32,
                ItemFlags2::UsedInATradeskill as u32,
                0,
                0,
            ],
            bag_family: BagFamilyMask::HERBS.bits(),
            start_quest_id: 0,
            stackable: i32::MAX,
            max_count: 3,
            lock_id: 0,
            required_reputation_rank: 0,
            sell_price: 99,
            buy_price: 123,
            vendor_stack_count: 2,
            price_variance: 1.25,
            price_random_value: 0.75,
            max_durability: 88,
            other_faction_item_id: 0,
            content_tuning_id: 0,
            player_level_to_item_level_curve_id: 0,
            limit_category: 44,
            instance_bound: 7,
            zone_bound: [8, 9],
            required_reputation_faction: 0,
            allowable_class: -1,
            required_expansion: 3,
            bonding: ItemBondingType::OnEquip as u8,
            container_slots: 16,
            inventory_type: InventoryType::Bag as i8,
        },
    )])));

    let template = session.item_storage_template(100).unwrap();

    assert_eq!(template.entry, 100);
    assert_eq!(template.class_id, ItemClass::Container);
    assert_eq!(template.subclass_id, 0);
    assert_eq!(template.inventory_type, InventoryType::Bag);
    assert_eq!(template.bonding, ItemBondingType::OnEquip);
    assert_eq!(template.bag_family, BagFamilyMask::HERBS);
    assert_eq!(template.max_stack_size, 0x7FFF_FFFE);
    assert_eq!(template.max_count, 3);
    assert_eq!(template.item_limit_category, 44);
    assert_eq!(template.container_slots, 16);
    assert_eq!(template.sell_price, 99);
    assert!(template.is_crafting_reagent);
    assert!(template.is_bound_account_wide());
    assert_eq!(
        session.item_template_inventory_type(100),
        Some(InventoryType::Bag as u8)
    );
    assert_eq!(session.item_storage_template(101), None);
}

#[test]
fn item_limit_category_template_applies_conditions_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_player_class_like_cpp(1);
    session.set_item_limit_category_store(Arc::new(ItemLimitCategoryStore::from_entries([
        ItemLimitCategoryEntry {
            id: 44,
            name: "Limited".into(),
            quantity: 1,
            flags: wow_entities::ITEM_LIMIT_CATEGORY_MODE_HAVE,
        },
    ])));
    session.set_item_limit_category_condition_store(Arc::new(
        ItemLimitCategoryConditionStore::from_entries([
            ItemLimitCategoryConditionEntry {
                id: 1,
                add_quantity: 2,
                player_condition_id: 700,
                parent_item_limit_category_id: 44,
            },
            ItemLimitCategoryConditionEntry {
                id: 2,
                add_quantity: 4,
                player_condition_id: 701,
                parent_item_limit_category_id: 44,
            },
            ItemLimitCategoryConditionEntry {
                id: 3,
                add_quantity: -1,
                player_condition_id: 999,
                parent_item_limit_category_id: 44,
            },
            ItemLimitCategoryConditionEntry {
                id: 4,
                add_quantity: 9,
                player_condition_id: 0,
                parent_item_limit_category_id: 45,
            },
        ]),
    ));
    session.set_player_condition_store(Arc::new(PlayerConditionStore::from_entries([
        PlayerConditionEntry {
            id: 700,
            class_mask: 1,
            ..PlayerConditionEntry::default()
        },
        PlayerConditionEntry {
            id: 701,
            class_mask: 2,
            ..PlayerConditionEntry::default()
        },
    ])));

    let limit = session
        .item_limit_category_template_like_cpp(44)
        .expect("limit category should resolve");

    assert_eq!(limit.id, 44);
    assert_eq!(limit.quantity, 2);
    assert_eq!(limit.flags, wow_entities::ITEM_LIMIT_CATEGORY_MODE_HAVE);
}

#[test]
fn item_buy_and_sell_price_follow_contrasted_cpp_standard_price_shape() {
    let (mut session, _, _) = make_session();
    session.set_item_store(Arc::new(ItemStore::from_records([
        ItemRecord {
            id: 200,
            class_id: ItemClass::Armor as u8,
            subclass_id: 3,
            material: 0,
            inventory_type: InventoryType::Chest as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
        ItemRecord {
            id: 201,
            class_id: ItemClass::Gem as u8,
            subclass_id: 11,
            material: 0,
            inventory_type: InventoryType::Relic as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
    ])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([
        (
            200,
            ItemSparseTemplateEntry {
                flags: [0, 0, 0, 0],
                bag_family: 0,
                start_quest_id: 0,
                stackable: 1,
                max_count: 0,
                lock_id: 0,
                required_reputation_rank: 0,
                sell_price: 77,
                buy_price: 123,
                vendor_stack_count: 2,
                price_variance: 1.5,
                price_random_value: 2.0,
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
                bonding: 0,
                container_slots: 0,
                inventory_type: InventoryType::Chest as i8,
            },
        ),
        (
            201,
            ItemSparseTemplateEntry {
                flags: [0, 0, 0, 0],
                bag_family: 0,
                start_quest_id: 0,
                stackable: 1,
                max_count: 0,
                lock_id: 0,
                required_reputation_rank: 0,
                sell_price: 88,
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
                bonding: 0,
                container_slots: 0,
                inventory_type: InventoryType::Relic as i8,
            },
        ),
    ])));
    session.set_import_price_stores(Arc::new(ImportPriceStores {
        armor: ImportPriceArmorStore::from_entries([ImportPriceArmorEntry {
            id: InventoryType::Chest as u32,
            cloth_modifier: 1.0,
            leather_modifier: 2.0,
            chain_modifier: 3.0,
            plate_modifier: 4.0,
        }]),
        quality: ImportPriceQualityStore::from_entries([ImportPriceQualityEntry {
            id: ItemQuality::Rare as u32 + 1,
            data: 5.0,
        }]),
        shield: ImportPriceShieldStore::from_entries([ImportPriceShieldEntry { id: 2, data: 9.0 }]),
        weapon: ImportPriceWeaponStore::from_entries([
            ImportPriceWeaponEntry { id: 3, data: 7.0 },
            ImportPriceWeaponEntry { id: 5, data: 11.0 },
        ]),
    }));
    session.set_item_price_base_store(Arc::new(ItemPriceBaseStore::from_entries([
        ItemPriceBaseEntry {
            id: 10,
            item_level: 10,
            armor: 100.0,
            weapon: 300.0,
        },
    ])));
    session.set_item_class_store(Arc::new(ItemClassStore::from_entries([ItemClassEntry {
        id: 4,
        class_id: ItemClass::Armor as i8,
        price_modifier: 0.25,
        flags: 0,
    }])));
    session.set_item_disenchant_loot_store(Arc::new(ItemDisenchantLootStore::from_entries([
        ItemDisenchantLootEntry {
            id: 900,
            subclass: -1,
            quality: ItemQuality::Rare as u8,
            min_level: 1,
            max_level: 20,
            skill_required: 175,
            expansion_id: 0,
            class_id: ItemClass::Armor as u32,
        },
    ])));
    session.set_item_currency_cost_store(Arc::new(ItemCurrencyCostStore::from_entries([
        ItemCurrencyCostEntry {
            id: 1,
            item_id: 201,
        },
    ])));

    assert_eq!(
        session.item_buy_price_like_cpp(200, ItemQuality::Rare as u32, 10),
        Some((4500, false))
    );
    assert_eq!(
        session.item_sell_price_like_cpp(200, ItemQuality::Rare as u32, 10),
        Some(77)
    );
    assert_eq!(
        session.item_buy_price_like_cpp(201, ItemQuality::Rare as u32, 10),
        Some((3500, false))
    );
    assert_eq!(
        session.item_disenchant_loot_like_cpp(200, ItemQuality::Rare as u32, 10, true),
        Some((900, 175))
    );
    assert_eq!(
        session.item_disenchant_loot_like_cpp(200, ItemQuality::Rare as u32, 10, false),
        None
    );
}

#[test]
fn item_template_lock_id_uses_item_sparse_lock_id_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([(
        700,
        ItemSparseTemplateEntry {
            flags: [0, 0, 0, 0],
            bag_family: 0,
            start_quest_id: 0,
            stackable: 1,
            max_count: 0,
            lock_id: 99,
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

    assert_eq!(session.item_template_lock_id(700), Some(99));
    assert_eq!(session.item_template_lock_id(701), None);
}
