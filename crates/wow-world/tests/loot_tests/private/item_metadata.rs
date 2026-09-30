//! Preserved item catalog generation and binding scenarios.
use super::support::*;
use rand::{SeedableRng, rngs::StdRng};
use wow_constants::{InventoryType, ItemBondingType, ItemClass, ItemContext, ItemFieldFlags, ItemQuality};
use wow_core::ObjectGuidGenerator;
use wow_world::session::InventoryItem;
use wow_persistence::PersistenceOutcomeLikeCpp;
use wow_entities::{Item, INVENTORY_SLOT_ITEM_START};
use wow_data::{ItemRecord, ItemStore, ItemStatsStore, ItemSparseTemplateEntry, ItemRandomEnchantmentTemplateStore, ItemRandomEnchantmentTemplateEntry, ItemRandomPropertiesStore, ItemRandomPropertiesEntry, ItemRandomPropertyTemplateEntry, ItemRandomSuffixStore, ItemRandomSuffixEntry, RandPropPointsStore, RandPropPointsEntry};
use wow_world::test_fixtures::loot::{LootRandomProperties, generate_loot_item_properties_for_test, new_loot_item_flags_for_test, existing_loot_item_flags_for_test, install_loot_inventory_item_for_test, loot_inventory_item_for_test, install_loot_inventory_port_for_test, store_loot_item_for_test};

fn install_limited_test_item_template_with_flags2_and_bonding(
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

fn test_item_record(
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

#[test]
fn stored_new_item_flags_follow_cpp_new_and_binding_rules() {
    let mut session = make_session();
    let item_id = 25;

    install_limited_test_item_template_with_flags2_and_bonding(
        &mut session,
        item_id,
        0,
        0,
        ItemBondingType::OnAcquire,
    );
    assert_eq!(
        new_loot_item_flags_for_test(&session, item_id, INVENTORY_SLOT_ITEM_START),
        (ItemFieldFlags::NEW_ITEM | ItemFieldFlags::SOULBOUND).bits()
    );

    install_limited_test_item_template_with_flags2_and_bonding(
        &mut session,
        item_id,
        0,
        0,
        ItemBondingType::None,
    );
    assert_eq!(
        new_loot_item_flags_for_test(&session, item_id, INVENTORY_SLOT_ITEM_START),
        ItemFieldFlags::NEW_ITEM.bits(),
        "an unbound backpack item must not acquire SOULBOUND"
    );

    install_limited_test_item_template_with_flags2_and_bonding(
        &mut session,
        item_id,
        0,
        0,
        ItemBondingType::OnEquip,
    );
    assert_eq!(
        new_loot_item_flags_for_test(&session, item_id, INVENTORY_SLOT_ITEM_START),
        ItemFieldFlags::NEW_ITEM.bits(),
        "C++ bind-if-stored does not bind OnEquip items in backpack slots"
    );
}

#[test]
fn historical_stack_binding_adds_only_soulbound_like_cpp() {
    let mut session = make_session();
    let item_id = 25;
    let historical = Item::new(0);

    install_limited_test_item_template_with_flags2_and_bonding(
        &mut session,
        item_id,
        0,
        0,
        ItemBondingType::OnAcquire,
    );
    let flags = existing_loot_item_flags_for_test(&session, 
        item_id,
        INVENTORY_SLOT_ITEM_START,
        &historical,
    );
    assert_eq!(flags, ItemFieldFlags::SOULBOUND.bits());
    assert_eq!(flags & ItemFieldFlags::NEW_ITEM.bits(), 0);

    install_limited_test_item_template_with_flags2_and_bonding(
        &mut session,
        item_id,
        0,
        0,
        ItemBondingType::OnEquip,
    );
    assert_eq!(
        existing_loot_item_flags_for_test(&session, 
            item_id,
            wow_entities::INVENTORY_SLOT_BAG_START,
            &historical,
        ),
        ItemFieldFlags::SOULBOUND.bits(),
        "C++ binds an OnEquip item when that item is stored in a bag-equipment position"
    );
    assert_eq!(
        existing_loot_item_flags_for_test(&session, 
            item_id,
            INVENTORY_SLOT_ITEM_START,
            &historical,
        ),
        0,
        "C++ does not bind an OnEquip stack in an ordinary backpack slot"
    );
}

#[test]
fn loot_item_store_random_properties_are_generated_from_cpp_random_select() {
    let entry = LootEntry {
        loot_list_id: 0,
        item_id: 25,
        quantity: 1,
        random_properties_id: -77,
        random_properties_seed: 456,
        item_context: 2,
        flags: LootEntryFlags::default(),
        allowed_looters: Vec::new(),
        roll_winner: ObjectGuid::EMPTY,
        ffa_looted_by: Vec::new(),
        taken: false,
    };
    let mut session = make_session();
    session.set_item_store(Arc::new(ItemStore::from_records([test_item_record(
        entry.item_id,
        77,
        0,
    )])));
    session.set_item_random_enchantment_template_store(Arc::new(
        ItemRandomEnchantmentTemplateStore::from_entries([ItemRandomEnchantmentTemplateEntry {
            group_id: 77,
            enchantment_id: 9001,
            chance: 100.0,
        }]),
    ));
    session.set_item_random_properties_store(Arc::new(ItemRandomPropertiesStore::from_entries([
        ItemRandomPropertiesEntry {
            id: 9001,
            enchantments: [1, 2, 3, 0, 0],
        },
    ])));

    let generated = generate_loot_item_properties_for_test(&session, 
        entry.item_id,
        &mut StdRng::seed_from_u64(1),
    );
    assert_eq!(generated, LootRandomProperties::new(9001, 0));
    assert_ne!(generated.id(), entry.random_properties_id);
    assert_ne!(generated.seed(), entry.random_properties_seed);
}

#[test]
fn loot_item_store_random_suffix_uses_cpp_property_points_seed() {
    let mut session = make_session();
    session.set_item_store(Arc::new(ItemStore::from_records([test_item_record(
        25, 0, 88,
    )])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_random_property_templates([
        (
            25,
            ItemRandomPropertyTemplateEntry {
                item_level: 11,
                quality: ItemQuality::Uncommon as i8,
                inventory_type: InventoryType::Chest as i8,
            },
        ),
    ])));
    session.set_item_random_enchantment_template_store(Arc::new(
        ItemRandomEnchantmentTemplateStore::from_entries([ItemRandomEnchantmentTemplateEntry {
            group_id: 88,
            enchantment_id: 7001,
            chance: 100.0,
        }]),
    ));
    session.set_item_random_suffix_store(Arc::new(ItemRandomSuffixStore::from_entries([
        ItemRandomSuffixEntry {
            id: 7001,
            enchantments: [10, 0, 0, 0, 0],
            allocation_pct: [10000, 0, 0, 0, 0],
        },
    ])));
    session.set_rand_prop_points_store(Arc::new(RandPropPointsStore::from_entries([
        RandPropPointsEntry {
            id: 11,
            damage_replace_stat: 0,
            epic: [900, 0, 0, 0, 0],
            superior: [500, 0, 0, 0, 0],
            good: [123, 0, 0, 0, 0],
        },
    ])));

    let generated = generate_loot_item_properties_for_test(&session, 25, &mut StdRng::seed_from_u64(1));
    assert_eq!(
        generated,
        LootRandomProperties::new(-7001, 123)
    );
}

#[tokio::test]
async fn failed_existing_stack_store_publishes_neither_count_nor_binding() {
    let (mut session, _send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 77);
    let item_id = 25;
    session.set_player_guid(Some(player_guid));
    session.set_item_guid_generator_like_cpp(Arc::new(ObjectGuidGenerator::new(
        HighGuid::Item,
        90_000,
    )));
    install_limited_test_item_template_with_flags2_and_bonding(
        &mut session,
        item_id,
        0,
        0,
        ItemBondingType::OnAcquire,
    );
    wow_world::test_fixtures::enable_ownerless_inventory_snapshots_for_test(&mut session);
    install_loot_inventory_item_for_test(&mut session, INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: item_guid, entry_id: item_id, db_guid: 77, inventory_type: None,
        }, player_guid, 4, 0, ItemContext::None);

    let requests = install_loot_inventory_port_for_test(&mut session,
        PersistenceOutcomeLikeCpp::Failed {
            reason: "fixture rollback".into(),
        },
    );

    let stored = store_loot_item_for_test(&mut session, 
            &LootEntry {
                loot_list_id: 0,
                item_id,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            },
            0,
        )
        .await;

    assert!(!stored);
    let historical = loot_inventory_item_for_test(&session, item_guid)
        .expect("failed transaction keeps the historical stack");
    assert_eq!(historical.count(), 4);
    assert_eq!(historical.item_flags_bits(), 0);
    let requests = requests.lock().unwrap();
    let [wow_persistence::PlayerInventoryPersistenceRequestLikeCpp::LootDirectItemGrant(request)] =
        requests.as_slice()
    else {
        panic!("direct loot must use its semantic inventory persistence variant");
    };
    assert_eq!(request.existing_stacks.len(), 1);
    assert_eq!(request.existing_stacks[0].item_guid, 77);
    assert_eq!(request.existing_stacks[0].new_count, 5);
    assert_eq!(
        request.existing_stacks[0].dynamic_flags,
        Some(ItemFieldFlags::SOULBOUND.bits())
    );
    assert!(request.new_stacks.is_empty());
    assert_eq!(request.stored_item_source, None);
}
