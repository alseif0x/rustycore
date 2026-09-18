//! Session scenarios exercising the represented player items responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn repair_all_inventory_item_durability_uses_guild_bank_limit_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let weapon_guid = ObjectGuid::create_item(1, 900);
    let bag_guid = ObjectGuid::create_item(1, 901);
    let armor_guid = ObjectGuid::create_item(1, 902);
    session.set_player_guid(Some(player_guid));
    session.set_player_gold_like_cpp(500);
    session.set_item_store(Arc::new(ItemStore::from_records([
        ItemRecord {
            id: 100,
            class_id: ItemClass::Armor as u8,
            subclass_id: ItemSubClassArmor::Shield as u8,
            material: 0,
            inventory_type: InventoryType::Shield as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
        ItemRecord {
            id: 101,
            class_id: ItemClass::Armor as u8,
            subclass_id: 4,
            material: 0,
            inventory_type: InventoryType::Chest as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
        ItemRecord {
            id: 200,
            class_id: ItemClass::Container as u8,
            subclass_id: 0,
            material: 0,
            inventory_type: InventoryType::Bag as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
    ])));
    let sparse = |inventory_type: InventoryType, max_durability: u32| ItemSparseTemplateEntry {
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
        max_durability,
        other_faction_item_id: 0,
        content_tuning_id: 0,
        player_level_to_item_level_curve_id: 0,
        limit_category: 0,
        instance_bound: 0,
        zone_bound: [0; 2],
        required_reputation_faction: 0,
        allowable_class: 0,
        required_expansion: 0,
        bonding: ItemBondingType::None as u8,
        container_slots: if inventory_type == InventoryType::Bag {
            4
        } else {
            0
        },
        inventory_type: inventory_type as i8,
    };
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_stats_sparse_and_random_property_templates(
            [(
                100,
                ItemStatEntry {
                    stats: [
                        (ItemModType::Strength as i8, 12),
                        (ItemModType::HitRating as i8, 5),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                    ],
                    resistances: [17, 0, 7, 0, 0, 0, 0],
                    armor: 17,
                },
            )],
            [
                (100, sparse(InventoryType::Shield, 50)),
                (101, sparse(InventoryType::Chest, 13)),
                (200, sparse(InventoryType::Bag, 0)),
            ],
            [
                (
                    100,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 57,
                        quality: ItemQuality::Rare as i8,
                        inventory_type: InventoryType::Shield as i8,
                    },
                ),
                (
                    101,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 57,
                        quality: ItemQuality::Rare as i8,
                        inventory_type: InventoryType::Chest as i8,
                    },
                ),
                (
                    200,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 57,
                        quality: ItemQuality::Normal as i8,
                        inventory_type: InventoryType::Bag as i8,
                    },
                ),
            ],
        ),
    ));
    session.set_durability_costs_store(Arc::new(DurabilityCostsStore::from_entries([
        DurabilityCostsEntry {
            id: 57,
            weapon_sub_class_cost: std::array::from_fn(|_| 0),
            armor_sub_class_cost: std::array::from_fn(|i| {
                if i == ItemSubClassArmor::Shield as usize {
                    13
                } else if i == 4 {
                    5
                } else {
                    0
                }
            }),
        },
    ])));
    let mut shield_block_rows = vec![ShieldBlockRegularEntryLikeCpp::default(); 57];
    shield_block_rows[56].superior = 42.0;
    session.set_shield_block_regular_game_table(Arc::new(
        ShieldBlockRegularGameTableLikeCpp::from_rows(shield_block_rows),
    ));
    session.set_durability_quality_store(Arc::new(DurabilityQualityStore::from_entries([
        DurabilityQualityEntry {
            id: (ItemQuality::Rare as u32 + 1) * 2,
            data: 1.25,
        },
    ])));
    session.inventory_items.insert(
        EQUIPMENT_SLOT_OFFHAND,
        InventoryItem {
            guid: weapon_guid,
            entry_id: 100,
            db_guid: weapon_guid.counter() as u64,
            inventory_type: Some(InventoryType::Shield as u8),
        },
    );
    session.inventory_items.insert(
        INVENTORY_SLOT_BAG_START,
        InventoryItem {
            guid: bag_guid,
            entry_id: 200,
            db_guid: bag_guid.counter() as u64,
            inventory_type: Some(InventoryType::Bag as u8),
        },
    );
    let weapon = session.make_inventory_item_object(
        weapon_guid,
        100,
        player_guid,
        1,
        40,
        ItemContext::None,
        EQUIPMENT_SLOT_OFFHAND,
    );
    let bag = session.make_inventory_item_object(
        bag_guid,
        200,
        player_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_BAG_START,
    );
    let mut armor = session.make_inventory_item_object(
        armor_guid,
        101,
        player_guid,
        1,
        10,
        ItemContext::None,
        0,
    );
    armor.set_container_guid_and_slot(bag_guid, 0);
    session.insert_inventory_item_object(weapon);
    session.insert_inventory_item_object(bag);
    session.insert_inventory_item_object(armor);

    session.set_represented_guild_repair_bank_state_like_cpp(Some(
        RepresentedGuildRepairBankStateLikeCpp {
            available_repair_money: 290,
            withdraw_repair_money_allowed: true,
        },
    ));
    assert!(
        session
            .repair_all_inventory_item_durability_with_guild_bank_like_cpp(0.8, 2.0)
            .await
    );
    assert_eq!(session.player_gold_like_cpp(), 500);
    assert_eq!(
        session.inventory_item_objects_like_cpp()[&armor_guid]
            .data()
            .durability,
        13
    );
    assert_eq!(
        session.inventory_item_objects_like_cpp()[&weapon_guid]
            .data()
            .durability,
        50
    );
    assert_eq!(
        session.represented_guild_repair_bank_withdraws_like_cpp(),
        &[RepresentedGuildRepairBankWithdrawLikeCpp {
            amount: 290,
            repair: true,
            success: true,
        }]
    );

    session
        .inventory_item_objects
        .get_mut(&weapon_guid)
        .unwrap()
        .set_durability(0);
    session
        .inventory_item_objects
        .get_mut(&armor_guid)
        .unwrap()
        .set_durability(10);
    session.set_represented_guild_repair_bank_state_like_cpp(Some(
        RepresentedGuildRepairBankStateLikeCpp {
            available_repair_money: 2_000,
            withdraw_repair_money_allowed: false,
        },
    ));
    assert!(
        session
            .repair_all_inventory_item_durability_with_guild_bank_like_cpp(0.8, 2.0)
            .await
    );
    assert_eq!(session.player_gold_like_cpp(), 500);
    assert_eq!(
        session.inventory_item_objects_like_cpp()[&armor_guid]
            .data()
            .durability,
        13
    );
    assert_eq!(
        session.inventory_item_objects_like_cpp()[&weapon_guid]
            .data()
            .durability,
        50
    );
    assert_eq!(
        session
            .represented_guild_repair_bank_withdraws_like_cpp()
            .last(),
        Some(&RepresentedGuildRepairBankWithdrawLikeCpp {
            amount: 1330,
            repair: true,
            success: false,
        })
    );
    assert_eq!(
        session.represented_item_mod_reapply_events_like_cpp(),
        &[RepresentedItemModsReapplyEventLikeCpp {
            item_guid: weapon_guid,
            slot: EQUIPMENT_SLOT_OFFHAND,
            apply: true,
        }],
        "C++ guild-bank repair also calls DurabilityRepair for each selected item before the final guild withdrawal"
    );
    assert_eq!(
        session.represented_item_bonus_actions_like_cpp().len(),
        8,
        "represented guild-bank repair records the same static _ApplyItemBonuses action plan for the broken equipped item"
    );
    assert!(
        session
            .represented_item_bonus_actions_like_cpp()
            .iter()
            .any(|action| matches!(
                action.action,
                ApplyEnchantmentEffectAction::SetShieldBlockValue { amount: 42 }
            )),
        "C++ _ApplyItemBonuses sets ActivePlayerData::ShieldBlock for repaired armor shields"
    );
}
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
#[test]
fn open_item_get_inventory_item_by_pos_resolves_top_level_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));

    let top_guid = ObjectGuid::create_item(1, 900);
    session.inventory_items.insert(
        23,
        InventoryItem {
            guid: top_guid,
            entry_id: 700,
            db_guid: 900,
            inventory_type: None,
        },
    );
    let top_item =
        session.make_inventory_item_object(top_guid, 700, player_guid, 1, 0, ItemContext::None, 23);
    session.insert_inventory_item_object(top_item);

    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, 23)
            .map(|i| i.guid),
        Some(top_guid)
    );
}
#[test]
fn open_item_get_inventory_item_by_pos_excludes_buyback_top_level_like_cpp() {
    let (mut session, _, _) = make_session();
    session.buyback_items.insert(
        BUYBACK_SLOT_START,
        InventoryItem {
            guid: ObjectGuid::create_item(1, 901),
            entry_id: 701,
            db_guid: 901,
            inventory_type: None,
        },
    );

    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, BUYBACK_SLOT_START)
            .is_none()
    );
}
#[test]
fn open_item_get_inventory_item_by_pos_resolves_nested_carried_bag_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    let (_, child_guid) =
        insert_open_item_bag_with_child(&mut session, player_guid, INVENTORY_SLOT_BAG_START, 5);

    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_START, 5)
            .map(|i| i.guid),
        Some(child_guid)
    );
}
#[test]
fn open_item_get_inventory_item_by_pos_resolves_nested_bank_bag_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    let (_, child_guid) =
        insert_open_item_bag_with_child(&mut session, player_guid, BANK_SLOT_BAG_START, 5);

    assert_eq!(
        session
            .get_inventory_item_by_pos(BANK_SLOT_BAG_START, 5)
            .map(|i| i.guid),
        Some(child_guid)
    );
}
#[test]
fn open_item_get_inventory_item_by_pos_resolves_nested_reagent_bag_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    let (_, child_guid) =
        insert_open_item_bag_with_child(&mut session, player_guid, REAGENT_BAG_SLOT_START, 5);

    assert_eq!(
        session
            .get_inventory_item_by_pos(REAGENT_BAG_SLOT_START, 5)
            .map(|i| i.guid),
        Some(child_guid)
    );
}
#[test]
fn open_item_get_inventory_item_by_pos_missing_bag_or_empty_slot_is_missing() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    insert_open_item_bag_with_child(&mut session, player_guid, INVENTORY_SLOT_BAG_START, 5);

    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_START + 1, 0)
            .is_none()
    );
    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_START, 3)
            .is_none()
    );
}
#[test]
fn open_item_nested_item_preserves_top_level_bag_slot_and_inner_slot() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    let (bag_guid, child_guid) =
        insert_open_item_bag_with_child(&mut session, player_guid, INVENTORY_SLOT_BAG_START, 5);

    let child = session.inventory_item_objects.get(&child_guid).unwrap();
    assert_eq!(child.container_guid(), bag_guid);
    assert_eq!(child.bag_slot(), INVENTORY_SLOT_BAG_START);
    assert_eq!(child.slot(), 5);
    assert_eq!(
        child.position(),
        u16::from(INVENTORY_SLOT_BAG_START) << 8 | 5
    );
}
#[tokio::test]
async fn open_item_nested_has_loot_opens_without_internal_bag_error() {
    assert_open_item_nested_has_loot_opens_without_internal_bag_error(INVENTORY_SLOT_BAG_START)
        .await;
}
#[tokio::test]
async fn open_item_nested_bank_bag_has_loot_opens_without_internal_bag_error() {
    assert_open_item_nested_has_loot_opens_without_internal_bag_error(BANK_SLOT_BAG_START).await;
}
#[tokio::test]
async fn open_item_nested_reagent_bag_has_loot_opens_without_internal_bag_error() {
    assert_open_item_nested_has_loot_opens_without_internal_bag_error(REAGENT_BAG_SLOT_START).await;
}
#[tokio::test]
async fn open_item_wrapped_without_has_loot_does_not_generate_loot_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 903);
    session.set_player_guid(Some(player_guid));
    install_open_item_template_with_flags(&mut session, 700, ItemFlags::empty(), 0);
    insert_open_item_top_level(&mut session, player_guid, 23, item_guid, 700, true);
    session
        .inventory_item_objects
        .get_mut(&item_guid)
        .unwrap()
        .set_item_flag(ItemFieldFlags::WRAPPED);

    session
        .handle_open_item(WorldPacket::from_bytes(&[INVENTORY_SLOT_BAG_0, 23]))
        .await;

    assert!(!session.loot_table.contains_key(&item_guid));
    assert!(send_rx.try_recv().is_err());
    assert!(
        session
            .inventory_item_objects
            .get(&item_guid)
            .is_some_and(|item| !item.loot_generated() && item.is_wrapped())
    );
}

/// Equip one 50-max-durability weapon in the mainhand for durability scenarios.
fn equip_durability_test_weapon_like_cpp(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    weapon_guid: ObjectGuid,
    durability: u32,
) {
    session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
        id: 300,
        class_id: ItemClass::Weapon as u8,
        subclass_id: 0,
        material: 0,
        inventory_type: InventoryType::Weapon as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }])));
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_stats_sparse_and_random_property_templates(
            [(
                300,
                ItemStatEntry {
                    stats: [
                        (ItemModType::Strength as i8, 12),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                    ],
                    resistances: [0; 7],
                    armor: 0,
                },
            )],
            [(
                300,
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
                    max_durability: 50,
                    other_faction_item_id: 0,
                    content_tuning_id: 0,
                    player_level_to_item_level_curve_id: 0,
                    limit_category: 0,
                    instance_bound: 0,
                    zone_bound: [0; 2],
                    required_reputation_faction: 0,
                    allowable_class: 0,
                    required_expansion: 0,
                    bonding: ItemBondingType::None as u8,
                    container_slots: 0,
                    inventory_type: InventoryType::Weapon as i8,
                },
            )],
            [(
                300,
                ItemRandomPropertyTemplateEntry {
                    item_level: 57,
                    quality: ItemQuality::Rare as i8,
                    inventory_type: InventoryType::Weapon as i8,
                },
            )],
        ),
    ));
    session.inventory_items.insert(
        EQUIPMENT_SLOT_MAINHAND,
        InventoryItem {
            guid: weapon_guid,
            entry_id: 300,
            db_guid: weapon_guid.counter() as u64,
            inventory_type: Some(InventoryType::Weapon as u8),
        },
    );
    let weapon = session.make_inventory_item_object(
        weapon_guid,
        300,
        player_guid,
        1,
        durability,
        ItemContext::None,
        EQUIPMENT_SLOT_MAINHAND,
    );
    session.insert_inventory_item_object(weapon);
}

fn durability_spell_store_like_cpp(
    spell_id: i32,
    effect: u32,
    damage: i32,
    slot: i32,
) -> wow_data::SpellStore {
    let mut store = wow_data::SpellStore::new();
    store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: effect,
            effect_base_points: damage,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect,
                effect_base_points: damage,
                effect_misc_value_1: slot,
                ..Default::default()
            }],
        },
    );
    store
}

#[test]
fn durability_points_loss_breaks_and_removes_equipped_item_mods_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 43);
    let weapon_guid = ObjectGuid::create_item(1, 903);
    session.set_player_guid(Some(player_guid));
    // 5 of 50 durability: the minimum one-point loss breaks the item.
    equip_durability_test_weapon_like_cpp(&mut session, player_guid, weapon_guid, 5);

    let affected = session.apply_represented_durability_loss_all_like_cpp(0.1, false);

    assert_eq!(affected, 1);
    assert_eq!(
        session.inventory_item_objects_like_cpp()[&weapon_guid]
            .data()
            .durability,
        0
    );
    assert_eq!(
        session.represented_item_mod_reapply_events_like_cpp(),
        &[RepresentedItemModsReapplyEventLikeCpp {
            item_guid: weapon_guid,
            slot: EQUIPMENT_SLOT_MAINHAND,
            apply: false,
        }],
        "C++ `DurabilityPointsLoss` removes equipped item mods before the durability write"
    );
}

#[tokio::test]
async fn durability_damage_spell_effect_reduces_equipped_items_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 90_200_i32;
    let player_guid = ObjectGuid::create_player(1, 44);
    let weapon_guid = ObjectGuid::create_item(1, 904);
    session.set_player_guid(Some(player_guid));
    equip_durability_test_weapon_like_cpp(&mut session, player_guid, weapon_guid, 50);
    session.set_spell_store(Arc::new(durability_spell_store_like_cpp(
        spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_DURABILITY_DAMAGE,
        7,
        -1,
    )));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented durability-damage spell should execute");

    assert_eq!(
        session.inventory_item_objects_like_cpp()[&weapon_guid]
            .data()
            .durability,
        43,
        "C++ EffectDurabilityDamage slot < 0 calls DurabilityPointsLossAll"
    );
    assert_eq!(
        durability_execute_log_row_like_cpp(&send_rx, player_guid, spell_id),
        (-1, -1),
        "C++ logs ItemID -1 and Amount -1 for the all-items branch"
    );
}

/// C++ `Spell::EffectDurabilityDamage` (`SpellEffects.cpp:4336-4340`) logs
/// `ExecuteLogEffectDurabilityDamage(effect, unitTarget, item->GetEntry(), slot)`.
#[tokio::test]
async fn durability_damage_spell_effect_logs_the_item_entry_and_slot_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 90_202_i32;
    let player_guid = ObjectGuid::create_player(1, 46);
    let weapon_guid = ObjectGuid::create_item(1, 906);
    session.set_player_guid(Some(player_guid));
    equip_durability_test_weapon_like_cpp(&mut session, player_guid, weapon_guid, 50);
    let slot = i32::from(EQUIPMENT_SLOT_MAINHAND);
    session.set_spell_store(Arc::new(durability_spell_store_like_cpp(
        spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_DURABILITY_DAMAGE,
        7,
        slot,
    )));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented durability-damage spell should execute");

    assert_eq!(
        session.inventory_item_objects_like_cpp()[&weapon_guid]
            .data()
            .durability,
        43
    );
    assert_eq!(
        durability_execute_log_row_like_cpp(&send_rx, player_guid, spell_id),
        (300, slot),
        "C++ logs the equipped item entry as ItemID and the slot as Amount"
    );
}

/// Decode the single `DurabilityDamageTargets` row of a cast's execute log.
fn durability_execute_log_row_like_cpp(
    send_rx: &flume::Receiver<Vec<u8>>,
    player_guid: ObjectGuid,
    spell_id: i32,
) -> (i32, i32) {
    let packets = crate::session::tests::drain_server_packet_bytes(send_rx);
    assert_eq!(
        packets
            .iter()
            .map(|bytes| {
                wow_packet::WorldPacket::from_bytes(bytes)
                    .server_opcode()
                    .expect("server opcode")
            })
            .collect::<Vec<_>>(),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::SpellExecuteLog,
            ServerOpcodes::CooldownEvent
        ]
    );
    let log_bytes = packets
        .iter()
        .find(|bytes| {
            wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
                == Some(ServerOpcodes::SpellExecuteLog)
        })
        .expect("execute log packet");
    let mut log = wow_packet::WorldPacket::from_bytes(log_bytes);
    log.read_uint16().expect("opcode");
    assert_eq!(log.read_packed_guid().expect("caster"), player_guid);
    assert_eq!(log.read_int32().expect("spell id"), spell_id);
    assert_eq!(log.read_uint32().expect("effect count"), 1);
    assert_eq!(
        log.read_int32().expect("effect"),
        i32::try_from(wow_data::spell::spell_effect_types::SPELL_EFFECT_DURABILITY_DAMAGE).unwrap()
    );
    assert_eq!(log.read_uint32().expect("power drain count"), 0);
    assert_eq!(log.read_uint32().expect("extra attacks count"), 0);
    assert_eq!(log.read_uint32().expect("durability count"), 1);
    for _ in 0..3 {
        assert_eq!(log.read_uint32().expect("empty list count"), 0);
    }
    let victim = log.read_packed_guid().expect("victim");
    assert_eq!(victim, player_guid);
    (
        log.read_int32().expect("item id"),
        log.read_int32().expect("amount"),
    )
}

#[tokio::test]
async fn durability_damage_pct_spell_effect_reduces_the_targeted_slot_like_cpp() {
    let (mut session, _, _) = make_session();
    let spell_id = 90_201_i32;
    let player_guid = ObjectGuid::create_player(1, 45);
    let weapon_guid = ObjectGuid::create_item(1, 905);
    session.set_player_guid(Some(player_guid));
    equip_durability_test_weapon_like_cpp(&mut session, player_guid, weapon_guid, 50);
    session.set_spell_store(Arc::new(durability_spell_store_like_cpp(
        spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_DURABILITY_DAMAGE_PCT,
        20,
        i32::from(EQUIPMENT_SLOT_MAINHAND),
    )));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented durability-damage-pct spell should execute");

    assert_eq!(
        session.inventory_item_objects_like_cpp()[&weapon_guid]
            .data()
            .durability,
        40,
        "C++ EffectDurabilityDamagePCT slot >= 0 calls DurabilityLoss on that slot"
    );
}
