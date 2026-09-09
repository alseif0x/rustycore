//! Session scenarios exercising the represented player items responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn direct_inventory_store_plan_counts_represented_bag_contents_for_limit_category_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let bag_guid = ObjectGuid::create_item(1, 800);
    let child_guid = ObjectGuid::create_item(1, 801);
    session.set_player_guid(Some(player_guid));
    session.set_item_store(Arc::new(ItemStore::from_records([
        ItemRecord {
            id: 600,
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
        ItemRecord {
            id: 700,
            class_id: ItemClass::Consumable as u8,
            subclass_id: 0,
            material: 0,
            inventory_type: InventoryType::NonEquip as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
    ])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([
        (
            600,
            ItemSparseTemplateEntry {
                flags: [0, 0, 0, 0],
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
                container_slots: 4,
                inventory_type: InventoryType::Bag as i8,
            },
        ),
        (
            700,
            ItemSparseTemplateEntry {
                flags: [0, 0, 0, 0],
                bag_family: 0,
                start_quest_id: 0,
                stackable: 20,
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
                limit_category: 44,
                instance_bound: 0,
                zone_bound: [0, 0],
                required_reputation_faction: 0,
                allowable_class: -1,
                required_expansion: 0,
                bonding: ItemBondingType::None as u8,
                container_slots: 0,
                inventory_type: InventoryType::NonEquip as i8,
            },
        ),
    ])));
    session.set_item_limit_category_store(Arc::new(ItemLimitCategoryStore::from_entries([
        ItemLimitCategoryEntry {
            id: 44,
            name: "Have one".into(),
            quantity: 1,
            flags: wow_entities::ITEM_LIMIT_CATEGORY_MODE_HAVE,
        },
    ])));

    session.inventory_items.insert(
        INVENTORY_SLOT_BAG_START,
        InventoryItem {
            guid: bag_guid,
            entry_id: 600,
            db_guid: 800,
            inventory_type: Some(InventoryType::Bag as u8),
        },
    );
    let bag = session.make_inventory_item_object(
        bag_guid,
        600,
        player_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_BAG_START,
    );
    session.insert_inventory_item_object(bag);
    let mut child = session.make_inventory_item_object(
        child_guid,
        700,
        player_guid,
        1,
        0,
        ItemContext::None,
        0,
    );
    child.set_container_guid_and_slot(bag_guid, 0);
    session.insert_inventory_item_object(child);

    let (result, dest, no_space) = session
        .plan_store_new_direct_inventory_item(700, 1)
        .expect("player snapshot should exist");

    assert_eq!(result, InventoryResult::ItemMaxLimitCategoryCountExceededIs);
    assert!(dest.is_empty());
    assert_eq!(no_space, Some(1));
}
#[test]
fn direct_inventory_store_plan_allocates_represented_bag_slot_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let bag_guid = ObjectGuid::create_item(1, 850);
    session.set_player_guid(Some(player_guid));
    session.set_item_store(Arc::new(ItemStore::from_records([
        ItemRecord {
            id: 600,
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
        ItemRecord {
            id: 700,
            class_id: ItemClass::Consumable as u8,
            subclass_id: 0,
            material: 0,
            inventory_type: InventoryType::NonEquip as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
        ItemRecord {
            id: 701,
            class_id: ItemClass::Consumable as u8,
            subclass_id: 0,
            material: 0,
            inventory_type: InventoryType::NonEquip as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
    ])));
    let sparse = |inventory_type: InventoryType,
                  stackable: i32,
                  container_slots: u8|
     -> ItemSparseTemplateEntry {
        ItemSparseTemplateEntry {
            flags: [0, 0, 0, 0],
            bag_family: 0,
            start_quest_id: 0,
            stackable,
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
            container_slots,
            inventory_type: inventory_type as i8,
        }
    };
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([
        (600, sparse(InventoryType::Bag, 1, 4)),
        (700, sparse(InventoryType::NonEquip, 20, 0)),
        (701, sparse(InventoryType::NonEquip, 20, 0)),
    ])));

    session.inventory_items.insert(
        INVENTORY_SLOT_BAG_START,
        InventoryItem {
            guid: bag_guid,
            entry_id: 600,
            db_guid: 850,
            inventory_type: Some(InventoryType::Bag as u8),
        },
    );
    let bag = session.make_inventory_item_object(
        bag_guid,
        600,
        player_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_BAG_START,
    );
    session.insert_inventory_item_object(bag);

    for slot_offset in 0..INVENTORY_DEFAULT_SIZE {
        let slot = INVENTORY_SLOT_ITEM_START + slot_offset;
        let db_guid = 900 + u64::from(slot_offset);
        let guid = ObjectGuid::create_item(1, db_guid as i64);
        session.inventory_items.insert(
            slot,
            InventoryItem {
                guid,
                entry_id: 701,
                db_guid,
                inventory_type: None,
            },
        );
        let item = session.make_inventory_item_object(
            guid,
            701,
            player_guid,
            1,
            0,
            ItemContext::None,
            slot,
        );
        session.insert_inventory_item_object(item);
    }

    let (result, dest, no_space) = session
        .plan_store_new_direct_inventory_item(700, 3)
        .expect("player snapshot should exist");

    assert_eq!(result, InventoryResult::Ok);
    assert_eq!(no_space, None);
    assert_eq!(
        dest,
        vec![ItemPosCount::new(
            (u16::from(INVENTORY_SLOT_BAG_START) << 8) | 0,
            3,
        )]
    );
}
#[test]
fn current_player_item_enchantment_plan_removes_canonical_duration_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 90_510);
    let player_position = Position::new(1.0, 2.0, 3.0, 0.0);
    let item_guid = ObjectGuid::create_item(1, 90_511);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_skill_values_like_cpp(HashMap::from([(333, 80)]));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    session
        .mutate_canonical_player_like_cpp(|player| player.unit_mut().set_level(80))
        .unwrap();
    session.set_spell_item_enchantment_store(Arc::new(SpellItemEnchantmentStore::from_entries([
        SpellItemEnchantmentEntry {
            id: 905,
            effect_arg: [0; 3],
            effect_points_min: [0; 3],
            item_visual: 0,
            flags: SpellItemEnchantmentFlags::empty(),
            required_skill_id: 333,
            required_skill_rank: 75,
            item_level: 1,
            charges: 0,
            effect: [ItemEnchantmentType::None as u8; 3],
            condition_id: 0,
            min_level: 1,
            max_level: 0,
        },
    ])));

    session.insert_inventory_item_like_cpp(
        EQUIPMENT_SLOT_MAINHAND,
        InventoryItem {
            guid: item_guid,
            entry_id: 700,
            db_guid: item_guid.counter() as u64,
            inventory_type: Some(InventoryType::Weapon as u8),
        },
    );
    let mut item = session.make_inventory_item_object(
        item_guid,
        700,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_MAINHAND,
    );
    item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 905, 12_000, 0);
    session.insert_inventory_item_object(item.clone());
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.add_enchantment_duration(
                &mut item,
                EnchantmentSlot::EnhancementTemporary,
                12_000,
            );
            assert_eq!(player.enchant_durations().len(), 1);
        })
        .unwrap();

    let plan = session
        .apply_current_player_item_enchantment_plan_like_cpp(
            item_guid,
            EnchantmentSlot::EnhancementTemporary,
            ApplyEnchantmentArgs::remove(),
        )
        .expect("canonical player should receive enchantment remove plan");

    assert!(
        matches!(
        plan.result,
        ApplyEnchantmentResult::Applied {
            apply: false,
            duration_action: Some(ApplyEnchantmentDurationAction::Removed {
                item_guid: removed_guid,
                slot: EnchantmentSlot::EnhancementTemporary,
            }),
            ..
        } if removed_guid == item_guid
        ),
        "unexpected plan: {plan:?}"
    );
    assert!(
        session
            .mutate_canonical_player_like_cpp(|player| player.enchant_durations().is_empty())
            .unwrap()
    );
    assert_eq!(
        session.inventory_item_objects_like_cpp()[&item_guid]
            .data()
            .enchantments[EnchantmentSlot::EnhancementTemporary as usize]
            .id,
        905
    );
}
#[test]
fn loaded_equipped_item_enchantments_apply_and_send_durations_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 90_520);
    let player_position = Position::new(1.0, 2.0, 3.0, 0.0);
    let item_guid = ObjectGuid::create_item(1, 90_521);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    session
        .mutate_canonical_player_like_cpp(|player| player.unit_mut().set_level(80))
        .unwrap();
    session.set_spell_item_enchantment_store(Arc::new(SpellItemEnchantmentStore::from_entries([
        SpellItemEnchantmentEntry {
            id: 903,
            effect_arg: [0; 3],
            effect_points_min: [0; 3],
            item_visual: 0,
            flags: SpellItemEnchantmentFlags::empty(),
            required_skill_id: 0,
            required_skill_rank: 0,
            item_level: 1,
            charges: 0,
            effect: [ItemEnchantmentType::None as u8; 3],
            condition_id: 0,
            min_level: 1,
            max_level: 0,
        },
        SpellItemEnchantmentEntry {
            id: 904,
            effect_arg: [0; 3],
            effect_points_min: [0; 3],
            item_visual: 44,
            flags: SpellItemEnchantmentFlags::empty(),
            required_skill_id: 0,
            required_skill_rank: 0,
            item_level: 1,
            charges: 0,
            effect: [ItemEnchantmentType::None as u8; 3],
            condition_id: 0,
            min_level: 1,
            max_level: 0,
        },
    ])));

    let mut item = session.make_inventory_item_object(
        item_guid,
        700,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_MAINHAND,
    );
    item.set_enchantment(EnchantmentSlot::EnhancementPermanent, 904, 0, 0);
    item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 903, 6_000, 0);
    session.insert_inventory_item_object(item);

    let outcome = session.apply_loaded_equipped_item_enchantments_like_cpp(item_guid);

    assert_eq!(outcome.plans.len(), 2);
    assert!(outcome.plans.iter().any(|plan| matches!(
        plan.result,
        ApplyEnchantmentResult::Applied {
            item_guid: applied_item_guid,
            slot: EnchantmentSlot::EnhancementPermanent,
            enchantment_id: 904,
            apply: true,
            update_permanent_visible_item: true,
            ..
        } if applied_item_guid == item_guid
    )));
    assert!(outcome.plans.iter().any(|plan| matches!(
        plan.result,
        ApplyEnchantmentResult::Applied {
            item_guid: applied_item_guid,
            slot: EnchantmentSlot::EnhancementTemporary,
            enchantment_id: 903,
            apply: true,
            duration_action: Some(ApplyEnchantmentDurationAction::Added(
                PlayerEnchantTimeUpdate {
                    item_guid: duration_item_guid,
                    slot: EnchantmentSlot::EnhancementTemporary,
                    duration_secs: 6,
                }
            )),
            ..
        } if applied_item_guid == item_guid && duration_item_guid == item_guid
    )));
    assert_eq!(
        outcome.visible_item_changes,
        vec![(EQUIPMENT_SLOT_MAINHAND, 700, 0, 44)]
    );
    assert_eq!(
        outcome.duration_updates,
        vec![PlayerEnchantTimeUpdate {
            item_guid,
            slot: EnchantmentSlot::EnhancementTemporary,
            duration_secs: 6,
        }]
    );
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| player.enchant_durations().to_vec()),
        Some(vec![PlayerEnchantDuration {
            item_guid,
            slot: EnchantmentSlot::EnhancementTemporary,
            left_duration_ms: 6_000,
        }])
    );
    assert!(
        send_rx.try_recv().is_err(),
        "loaded enchant replay queues packets until after login CREATE"
    );
    session.send_loaded_equipped_item_enchantment_updates_like_cpp(&outcome);
    assert_eq!(
        send_rx.try_recv().unwrap(),
        ItemEnchantTimeUpdate {
            owner_guid: player_guid,
            item_guid,
            duration_left: 6,
            slot: EnchantmentSlot::EnhancementTemporary as u32,
        }
        .to_bytes()
    );
    assert!(
        drain_server_packet_bytes(&send_rx)
            .iter()
            .any(|bytes| WorldPacket::from_bytes(bytes).server_opcode()
                == Some(ServerOpcodes::UpdateObject)),
        "permanent enchant visual update is emitted after login CREATE"
    );
}
#[test]
fn loaded_socket_enchantment_replay_enforces_prismatic_and_gem_skills_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 90_530);
    let item_guid = ObjectGuid::create_item(1, 90_531);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(
        &canonical,
        player_guid,
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        0,
    );
    session
        .mutate_canonical_player_like_cpp(|player| player.unit_mut().set_level(80))
        .unwrap();
    let enchantment = |id, required_skill_id, required_skill_rank| SpellItemEnchantmentEntry {
        id,
        effect_arg: [0; 3],
        effect_points_min: [0; 3],
        item_visual: 0,
        flags: SpellItemEnchantmentFlags::empty(),
        required_skill_id,
        required_skill_rank,
        item_level: 1,
        charges: 0,
        effect: [ItemEnchantmentType::None as u8; 3],
        condition_id: 0,
        min_level: 1,
        max_level: 0,
    };
    session.set_spell_item_enchantment_store(Arc::new(SpellItemEnchantmentStore::from_entries([
        enchantment(913, 0, 0),
        enchantment(914, 755, 350),
    ])));
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_templates(std::iter::empty::<(u32, ItemSparseTemplateEntry)>())
            .with_socket_templates([
                (
                    700,
                    ItemSocketTemplateEntry {
                        socket_types: [0, 2, 2],
                        required_skill_id: 0,
                        required_skill_rank: 0,
                    },
                ),
                (
                    800,
                    ItemSocketTemplateEntry {
                        socket_types: [0; 3],
                        required_skill_id: 202,
                        required_skill_rank: 300,
                    },
                ),
            ]),
    ));
    session.insert_inventory_item_like_cpp(
        EQUIPMENT_SLOT_CHEST,
        InventoryItem {
            guid: item_guid,
            entry_id: 700,
            db_guid: item_guid.counter() as u64,
            inventory_type: Some(InventoryType::Chest as u8),
        },
    );
    let mut item = session.make_inventory_item_object(
        item_guid,
        700,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_CHEST,
    );
    item.set_gems(vec![SocketedGem {
        item_id: 800,
        context: 0,
        bonus_list_ids: Vec::new(),
    }]);
    item.set_enchantment(EnchantmentSlot::EnhancementSocket, 913, 0, 0);
    item.set_enchantment(EnchantmentSlot::EnhancementSocketPrismatic, 914, 0, 0);
    session.insert_inventory_item_object(item);

    session.set_player_skill_values_like_cpp(HashMap::from([(755, 349), (202, 299)]));
    let prismatic_blocked = session
        .apply_current_player_item_enchantment_plan_like_cpp(
            item_guid,
            EnchantmentSlot::EnhancementSocket,
            ApplyEnchantmentArgs::apply(),
        )
        .unwrap();
    assert!(matches!(
        prismatic_blocked.result,
        ApplyEnchantmentResult::Skipped(ApplyEnchantmentSkipReason::PrismaticRequiredSkillTooLow)
    ));

    session.set_player_skill_values_like_cpp(HashMap::from([(755, 350), (202, 299)]));
    let gem_blocked = session
        .apply_current_player_item_enchantment_plan_like_cpp(
            item_guid,
            EnchantmentSlot::EnhancementSocket,
            ApplyEnchantmentArgs::apply(),
        )
        .unwrap();
    assert!(matches!(
        gem_blocked.result,
        ApplyEnchantmentResult::Skipped(ApplyEnchantmentSkipReason::GemRequiredSkillTooLow)
    ));

    session.set_player_skill_values_like_cpp(HashMap::from([(755, 350), (202, 300)]));
    let allowed = session
        .apply_current_player_item_enchantment_plan_like_cpp(
            item_guid,
            EnchantmentSlot::EnhancementSocket,
            ApplyEnchantmentArgs::apply(),
        )
        .unwrap();
    assert!(matches!(
        allowed.result,
        ApplyEnchantmentResult::Applied { .. }
    ));
}
#[test]
fn loaded_broken_equipped_item_skips_enchantment_replay_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 90_526);
    let item_guid = ObjectGuid::create_item(1, 90_527);
    session.set_player_guid(Some(player_guid));

    let mut item = session.make_inventory_item_object(
        item_guid,
        703,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_MAINHAND,
    );
    item.set_max_durability(100);
    item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 903, 6_000, 0);
    assert!(item.is_broken());
    session.insert_inventory_item_object(item);

    let outcome = session.apply_loaded_equipped_item_enchantments_like_cpp(item_guid);

    assert!(outcome.plans.is_empty());
    assert!(outcome.duration_updates.is_empty());
    assert!(!outcome.send_stat_update);
    assert!(outcome.visible_item_changes.is_empty());
    assert!(outcome.effect_actions.is_empty());
    assert!(outcome.unrepresented_effect_actions.is_empty());
    session.send_loaded_equipped_item_enchantment_updates_like_cpp(&outcome);
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn loaded_top_level_bag_skips_enchantment_replay_like_cpp_item_is_equipped() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 90_532);
    let item_guid = ObjectGuid::create_item(1, 90_533);
    session.set_player_guid(Some(player_guid));

    let mut item = session.make_inventory_item_object(
        item_guid,
        700,
        player_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_BAG_START,
    );
    item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 903, 6_000, 0);
    assert!(!item.is_equipped());
    session.insert_inventory_item_object(item);

    let outcome = session.apply_loaded_equipped_item_enchantments_like_cpp(item_guid);

    assert!(outcome.plans.is_empty());
    assert!(outcome.duration_updates.is_empty());
    assert!(!outcome.send_stat_update);
    assert!(outcome.visible_item_changes.is_empty());
    assert!(outcome.effect_actions.is_empty());
    assert!(outcome.unrepresented_effect_actions.is_empty());
    session.send_loaded_equipped_item_enchantment_updates_like_cpp(&outcome);
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn loaded_disarmed_mainhand_skips_enchantment_replay_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 90_528);
    let player_position = Position::new(1.0, 2.0, 3.0, 0.0);
    let item_guid = ObjectGuid::create_item(1, 90_529);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    session
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .set_unit_flags_like_cpp(UnitFlags::DISARMED);
        })
        .unwrap();

    let mut item = session.make_inventory_item_object(
        item_guid,
        704,
        player_guid,
        1,
        1,
        ItemContext::None,
        EQUIPMENT_SLOT_MAINHAND,
    );
    item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 903, 6_000, 0);
    session.insert_inventory_item_object(item);

    let outcome = session.apply_loaded_equipped_item_enchantments_like_cpp(item_guid);

    assert!(outcome.plans.is_empty());
    assert!(outcome.duration_updates.is_empty());
    session.send_loaded_equipped_item_enchantment_updates_like_cpp(&outcome);
    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| player.enchant_durations().len()),
        Some(0)
    );
}
#[test]
fn loaded_equipped_item_enchantments_apply_effect_actions_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 90_522);
    let player_position = Position::new(1.0, 2.0, 3.0, 0.0);
    let item_guid = ObjectGuid::create_item(1, 90_523);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    session
        .mutate_canonical_player_like_cpp(|player| player.unit_mut().set_level(80))
        .unwrap();
    session.set_spell_item_enchantment_store(Arc::new(SpellItemEnchantmentStore::from_entries([
        SpellItemEnchantmentEntry {
            id: 906,
            effect_arg: [ItemModType::Health as u32, 0, 0],
            effect_points_min: [17, 0, 0],
            item_visual: 0,
            flags: SpellItemEnchantmentFlags::empty(),
            required_skill_id: 0,
            required_skill_rank: 0,
            item_level: 1,
            charges: 0,
            effect: [
                ItemEnchantmentType::Stat as u8,
                ItemEnchantmentType::None as u8,
                ItemEnchantmentType::None as u8,
            ],
            condition_id: 0,
            min_level: 1,
            max_level: 0,
        },
    ])));

    let mut item = session.make_inventory_item_object(
        item_guid,
        701,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_CHEST,
    );
    item.set_enchantment(EnchantmentSlot::EnhancementPermanent, 906, 0, 0);
    session.insert_inventory_item_object(item.clone());

    let outcome = session.apply_loaded_equipped_item_enchantments_like_cpp(item_guid);

    assert!(matches!(
        outcome.plans.first().map(|plan| plan.result),
        Some(ApplyEnchantmentResult::Applied {
            enchantment_id: 906,
            apply: true,
            effects_allowed: true,
            ..
        })
    ));
    assert_eq!(
        session.represented_item_bonus_state_like_cpp().health_base,
        17
    );
    assert_eq!(outcome.effect_actions.len(), 1);
    assert!(matches!(
        outcome.effect_actions[0].action,
        ApplyEnchantmentEffectAction::UnitModifier { .. }
    ));
    assert!(outcome.unrepresented_effect_actions.is_empty());
    assert!(outcome.send_stat_update);
    assert!(
        send_rx.try_recv().is_err(),
        "loaded enchant stat update is queued until after login CREATE"
    );
    session.send_loaded_equipped_item_enchantment_updates_like_cpp(&outcome);
    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(
        packets.len(),
        1,
        "only the permanent-enchant visual delta remains; stat bonuses are folded into the full login snapshot"
    );
    assert_eq!(
        WorldPacket::from_bytes(&packets[0]).server_opcode(),
        Some(ServerOpcodes::UpdateObject)
    );

    session.clear_all_inventory_runtime_like_cpp();
    assert!(session.represented_item_bonus_actions_like_cpp().is_empty());
    assert_eq!(
        session.represented_item_bonus_state_like_cpp().health_base,
        0,
        "logout discards the old Player's item modifier state"
    );

    session.insert_inventory_item_object(item);
    let relogin = session.apply_initial_loaded_item_mods_like_cpp(&[item_guid]);
    assert!(relogin.enchantments.send_stat_update);
    assert_eq!(
        session.represented_item_bonus_state_like_cpp().health_base,
        17,
        "relogin replays the new Player's enchantment once instead of carrying or doubling the previous bonus"
    );
}
#[test]
fn send_new_item_plan_maps_entity_fields_to_item_push_result_like_cpp() {
    let plan = send_new_item_plan(SendNewItemDelivery::Direct);
    let packet = WorldSession::item_push_result_from_send_new_item_plan(&plan);

    assert_eq!(packet.player_guid, plan.player_guid);
    assert_eq!(packet.item_guid, plan.item_guid);
    assert_eq!(packet.slot, 4);
    assert_eq!(packet.slot_in_bag, 7);
    assert_eq!(packet.quest_log_item_id, 777);
    assert_eq!(packet.quantity, 3);
    assert_eq!(packet.quantity_in_inventory, 9);
    assert_eq!(packet.dungeon_encounter_id, 615);
    assert_eq!(
        packet.display_text,
        ItemPushResultDisplayType::EncounterLoot
    );
    assert!(packet.pushed);
    assert!(!packet.created);
    assert!(!packet.is_bonus_roll);
    assert!(packet.is_encounter_loot);
    assert_eq!(packet.item.item_id, 9001);
    assert_eq!(packet.item.random_properties_seed, 456);
    assert_eq!(packet.item.random_properties_id, -77);
    assert!(packet.item.item_bonus.is_none());
    assert_eq!(
        packet.item.modifications.values,
        vec![ItemMod::new(123, 3), ItemMod::new(25, 5)]
    );
}
