//! Session scenarios exercising the represented player items responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn repair_inventory_item_durability_spends_money_and_restores_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 900);
    session.set_player_guid(Some(player_guid));
    session.set_player_gold_like_cpp(2_000);
    session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
        id: 100,
        class_id: ItemClass::Weapon as u8,
        subclass_id: 7,
        material: 0,
        inventory_type: InventoryType::Weapon as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }])));
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [(
                100,
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
                100,
                ItemRandomPropertyTemplateEntry {
                    item_level: 57,
                    quality: ItemQuality::Rare as i8,
                    inventory_type: InventoryType::Weapon as i8,
                },
            )],
        ),
    ));
    session.set_durability_costs_store(Arc::new(DurabilityCostsStore::from_entries([
        DurabilityCostsEntry {
            id: 57,
            weapon_sub_class_cost: std::array::from_fn(|i| if i == 7 { 13 } else { 0 }),
            armor_sub_class_cost: [0; 8],
        },
    ])));
    session.set_durability_quality_store(Arc::new(DurabilityQualityStore::from_entries([
        DurabilityQualityEntry {
            id: (ItemQuality::Rare as u32 + 1) * 2,
            data: 1.25,
        },
    ])));
    session.inventory_items.insert(
        EQUIPMENT_SLOT_MAINHAND,
        InventoryItem {
            guid: item_guid,
            entry_id: 100,
            db_guid: item_guid.counter() as u64,
            inventory_type: Some(InventoryType::Weapon as u8),
        },
    );
    let item = session.make_inventory_item_object(
        item_guid,
        100,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_MAINHAND,
    );
    session.insert_inventory_item_object(item);

    assert!(
        session
            .repair_inventory_item_durability_like_cpp(item_guid, true, 0.8, 2.0)
            .await
    );
    assert_eq!(session.player_gold_like_cpp(), 700);
    assert_eq!(
        session.inventory_item_objects_like_cpp()[&item_guid]
            .data()
            .durability,
        50
    );
    assert_eq!(
        session.represented_item_mod_reapply_events_like_cpp(),
        &[RepresentedItemModsReapplyEventLikeCpp {
            item_guid,
            slot: EQUIPMENT_SLOT_MAINHAND,
            apply: true,
        }]
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::UpdateObject, ServerOpcodes::UpdateObject],
        "C++ DurabilityRepair sets item durability and reapplies equipped broken-item mods, both visible through update fields"
    );

    session.set_player_gold_like_cpp(10);
    let item = session.inventory_item_objects.get_mut(&item_guid).unwrap();
    item.set_durability(40);
    assert!(
        !session
            .repair_inventory_item_durability_like_cpp(item_guid, true, 0.8, 2.0)
            .await
    );
    assert_eq!(session.player_gold_like_cpp(), 10);
    assert_eq!(
        session.inventory_item_objects_like_cpp()[&item_guid]
            .data()
            .durability,
        40
    );
    assert!(drain_server_opcodes(&send_rx).is_empty());
}
#[test]
fn represented_item_mods_records_weapon_damage_without_stat_entry_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 900);
    session.set_player_guid(Some(player_guid));
    session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
        id: 100,
        class_id: ItemClass::Weapon as u8,
        subclass_id: 7,
        material: 0,
        inventory_type: InventoryType::Weapon as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_weapon_templates([(
        100,
        ItemWeaponTemplateEntry {
            dmg_variance: 1.0,
            item_delay: 2600,
            min_damage: [12, 0, 0, 0, 0],
            max_damage: [18, 0, 0, 0, 0],
            damage_damage_type: 0,
        },
    )])));
    session.inventory_items.insert(
        EQUIPMENT_SLOT_MAINHAND,
        InventoryItem {
            guid: item_guid,
            entry_id: 100,
            db_guid: item_guid.counter() as u64,
            inventory_type: Some(InventoryType::Weapon as u8),
        },
    );
    let item = session.make_inventory_item_object(
        item_guid,
        100,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_MAINHAND,
    );
    session.insert_inventory_item_object(item);

    session.record_represented_item_mods_like_cpp(item_guid, EQUIPMENT_SLOT_MAINHAND, true);

    assert_eq!(
        session.represented_item_bonus_actions_like_cpp(),
        &[
            RepresentedItemBonusActionLikeCpp {
                item_guid,
                slot: EQUIPMENT_SLOT_MAINHAND,
                action: ApplyEnchantmentEffectAction::SetBaseWeaponDamage {
                    attack_type: wow_constants::WeaponAttackType::BaseAttack,
                    bound: wow_entities::WeaponDamageBoundLikeCpp::Min,
                    amount_bits: 12.0f32.to_bits(),
                },
            },
            RepresentedItemBonusActionLikeCpp {
                item_guid,
                slot: EQUIPMENT_SLOT_MAINHAND,
                action: ApplyEnchantmentEffectAction::SetBaseWeaponDamage {
                    attack_type: wow_constants::WeaponAttackType::BaseAttack,
                    bound: wow_entities::WeaponDamageBoundLikeCpp::Max,
                    amount_bits: 18.0f32.to_bits(),
                },
            },
            RepresentedItemBonusActionLikeCpp {
                item_guid,
                slot: EQUIPMENT_SLOT_MAINHAND,
                action: ApplyEnchantmentEffectAction::SetBaseAttackTime {
                    attack_type: wow_constants::WeaponAttackType::BaseAttack,
                    time_ms: 2600,
                },
            },
            RepresentedItemBonusActionLikeCpp {
                item_guid,
                slot: EQUIPMENT_SLOT_MAINHAND,
                action: ApplyEnchantmentEffectAction::UpdateDamagePhysical {
                    attack_type: wow_constants::WeaponAttackType::BaseAttack,
                },
            },
        ],
        "C++ Player::_ApplyItemBonuses reaches _ApplyWeaponDamage even when ItemSparse has no stat modifiers"
    );
}
#[test]
fn represented_item_mods_apply_scaling_weapon_dps_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 901);
    session.set_player_guid(Some(player_guid));
    session.set_player_level_like_cpp(80);
    session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
        id: 101,
        class_id: ItemClass::Weapon as u8,
        subclass_id: 7,
        material: 0,
        inventory_type: InventoryType::Weapon as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 77,
        scaling_stat_value: 0x0000_0200,
    }])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_weapon_templates([(
        101,
        ItemWeaponTemplateEntry {
            dmg_variance: 1.0,
            item_delay: 2000,
            min_damage: [1, 0, 0, 0, 0],
            max_damage: [2, 0, 0, 0, 0],
            damage_damage_type: 0,
        },
    )])));
    session.set_scaling_stat_distribution_store(Arc::new(
        ScalingStatDistributionStore::from_entries([ScalingStatDistributionEntry {
            id: 77,
            player_level_to_item_level_curve_id: 0,
            min_level: 10,
            max_level: 20,
            bonus: [0; 10],
            stat_id: [0; 10],
        }]),
    ));
    session.set_scaling_stat_values_store(Arc::new(ScalingStatValuesStore::from_entries([
        ScalingStatValuesEntry {
            id: 20,
            char_level: 20,
            weapon_dps_1h: 100,
            weapon_dps_2h: 0,
            spellcaster_dps_1h: 0,
            spellcaster_dps_2h: 0,
            ranged_dps: 0,
            wand_dps: 0,
            spell_power: 0,
            shoulder_budget: 0,
            trinket_budget: 0,
            weapon_budget_1h: 0,
            primary_budget: 0,
            ranged_budget: 0,
            tertiary_budget: 0,
            cloth_shoulder_armor: 0,
            leather_shoulder_armor: 0,
            mail_shoulder_armor: 0,
            plate_shoulder_armor: 0,
            cloth_cloak_armor: 0,
            cloth_chest_armor: 0,
            leather_chest_armor: 0,
            mail_chest_armor: 0,
            plate_chest_armor: 0,
        },
    ])));
    session.inventory_items.insert(
        EQUIPMENT_SLOT_MAINHAND,
        InventoryItem {
            guid: item_guid,
            entry_id: 101,
            db_guid: item_guid.counter() as u64,
            inventory_type: Some(InventoryType::Weapon as u8),
        },
    );
    let item = session.make_inventory_item_object(
        item_guid,
        101,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_MAINHAND,
    );
    session.insert_inventory_item_object(item);

    session.record_represented_item_mods_like_cpp(item_guid, EQUIPMENT_SLOT_MAINHAND, true);

    assert_eq!(
        session.represented_item_bonus_actions_like_cpp(),
        &[
            RepresentedItemBonusActionLikeCpp {
                item_guid,
                slot: EQUIPMENT_SLOT_MAINHAND,
                action: ApplyEnchantmentEffectAction::SetBaseWeaponDamage {
                    attack_type: wow_constants::WeaponAttackType::BaseAttack,
                    bound: wow_entities::WeaponDamageBoundLikeCpp::Min,
                    amount_bits: 140.0f32.to_bits(),
                },
            },
            RepresentedItemBonusActionLikeCpp {
                item_guid,
                slot: EQUIPMENT_SLOT_MAINHAND,
                action: ApplyEnchantmentEffectAction::SetBaseWeaponDamage {
                    attack_type: wow_constants::WeaponAttackType::BaseAttack,
                    bound: wow_entities::WeaponDamageBoundLikeCpp::Max,
                    amount_bits: 260.0f32.to_bits(),
                },
            },
            RepresentedItemBonusActionLikeCpp {
                item_guid,
                slot: EQUIPMENT_SLOT_MAINHAND,
                action: ApplyEnchantmentEffectAction::SetBaseAttackTime {
                    attack_type: wow_constants::WeaponAttackType::BaseAttack,
                    time_ms: 2000,
                },
            },
            RepresentedItemBonusActionLikeCpp {
                item_guid,
                slot: EQUIPMENT_SLOT_MAINHAND,
                action: ApplyEnchantmentEffectAction::UpdateDamagePhysical {
                    attack_type: wow_constants::WeaponAttackType::BaseAttack,
                },
            },
        ],
        "C++ clamps player level to ScalingStatDistribution range and replaces weapon min/max from ScalingStatValues::getDPSMod"
    );
    assert_eq!(
        session
            .represented_item_bonus_state_like_cpp()
            .weapon_damage[wow_constants::WeaponAttackType::BaseAttack as usize],
        [140.0, 260.0],
        "represented runtime state now applies the planned SetBaseWeaponDamage actions"
    );
    assert_eq!(
        session
            .represented_item_bonus_state_like_cpp()
            .base_attack_time[wow_constants::WeaponAttackType::BaseAttack as usize],
        2000
    );
    assert_eq!(
        session
            .represented_item_bonus_state_like_cpp()
            .damage_physical_updates,
        &[wow_constants::WeaponAttackType::BaseAttack]
    );
    let stat_changes =
        represented_player_stat_changes_like_cpp(&session.represented_item_bonus_state_like_cpp());
    assert_eq!(stat_changes.min_damage, 140.0);
    assert_eq!(stat_changes.max_damage, 260.0);
    assert_eq!(
        stat_changes.min_ranged_damage, 0.0,
        "the represented packet projection does not invent ranged/offhand values when C++ only changed BASE_ATTACK"
    );
}
#[test]
fn destroyed_inventory_item_mod_remove_matches_cpp_destroy_item_equipment_branch() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 903);
    session.set_player_guid(Some(player_guid));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_parts(
        [(
            103,
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
        [],
    )));
    session.inventory_items.insert(
        EQUIPMENT_SLOT_CHEST,
        InventoryItem {
            guid: item_guid,
            entry_id: 103,
            db_guid: item_guid.counter() as u64,
            inventory_type: Some(InventoryType::Chest as u8),
        },
    );
    let item = session.make_inventory_item_object(
        item_guid,
        103,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_CHEST,
    );
    session.insert_inventory_item_object(item);

    session.record_represented_item_mods_like_cpp(item_guid, EQUIPMENT_SLOT_CHEST, true);
    assert_eq!(
        session.represented_item_bonus_state_like_cpp().stats_base
            [wow_constants::Stats::Strength as usize],
        12
    );
    let actions_before_destroy = session.represented_item_bonus_actions_like_cpp().len();

    assert!(session.record_destroyed_inventory_item_mod_remove_like_cpp(
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_CHEST,
        item_guid,
    ));

    assert_eq!(
        session.represented_item_bonus_state_like_cpp().stats_base
            [wow_constants::Stats::Strength as usize],
        0,
        "C++ Player::DestroyItem calls _ApplyItemMods(pItem, slot, false) for bag 0 slots below INVENTORY_SLOT_BAG_END"
    );
    assert!(session.represented_item_bonus_actions_like_cpp().len() > actions_before_destroy);
}
#[test]
fn destroyed_inventory_item_mod_remove_skips_backpack_and_broken_items_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let backpack_item_guid = ObjectGuid::create_item(1, 904);
    let broken_item_guid = ObjectGuid::create_item(1, 905);
    session.set_player_guid(Some(player_guid));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_parts(
        [(
            104,
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
        [],
    )));

    let backpack_item = session.make_inventory_item_object(
        backpack_item_guid,
        104,
        player_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    session.insert_inventory_item_object(backpack_item);

    let mut broken_item = session.make_inventory_item_object(
        broken_item_guid,
        104,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_CHEST,
    );
    broken_item.set_max_durability(10);
    broken_item.set_durability(0);
    session.insert_inventory_item_object(broken_item);

    let actions_before = session.represented_item_bonus_actions_like_cpp().len();
    assert!(
        !session.record_destroyed_inventory_item_mod_remove_like_cpp(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            backpack_item_guid,
        )
    );
    assert!(
        !session.record_destroyed_inventory_item_mod_remove_like_cpp(
            INVENTORY_SLOT_BAG_0,
            EQUIPMENT_SLOT_CHEST,
            broken_item_guid,
        )
    );
    assert_eq!(
        session.represented_item_bonus_actions_like_cpp().len(),
        actions_before,
        "C++ _ApplyItemMods skips non-applied inventory slots and broken equipped items"
    );
}
#[test]
fn destroyed_inventory_item_set_remove_matches_cpp_even_for_broken_equipped_item() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let chest_guid = ObjectGuid::create_item(1, 912);
    let hands_guid = ObjectGuid::create_item(1, 913);
    let backpack_guid = ObjectGuid::create_item(1, 914);
    session.set_player_guid(Some(player_guid));
    session.set_item_set_store(Arc::new(ItemSetStore::from_entries([ItemSetEntry {
        id: 704,
        name: "Destroy Set".to_string(),
        set_flags: 0,
        required_skill: 0,
        required_skill_rank: 0,
        item_id: std::array::from_fn(|i| match i {
            0 => 106,
            1 => 107,
            2 => 108,
            _ => 0,
        }),
    }])));
    session.set_item_set_spell_store(Arc::new(ItemSetSpellStore::from_entries([
        ItemSetSpellEntry {
            id: 20,
            chr_spec_id: 0,
            spell_id: 9020,
            threshold: 2,
            item_set_id: 704,
        },
    ])));

    let mut broken_chest = session.make_inventory_item_object(
        chest_guid,
        106,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_CHEST,
    );
    broken_chest.set_max_durability(10);
    broken_chest.set_durability(0);
    session.insert_inventory_item_object(broken_chest);
    session.insert_inventory_item_like_cpp(
        EQUIPMENT_SLOT_CHEST,
        InventoryItem {
            guid: chest_guid,
            entry_id: 106,
            db_guid: chest_guid.counter() as u64,
            inventory_type: Some(InventoryType::Chest as u8),
        },
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_HANDS,
        hands_guid,
        107,
        InventoryType::Hands,
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        INVENTORY_SLOT_ITEM_START,
        backpack_guid,
        108,
        InventoryType::Chest,
    );

    assert!(!session.record_represented_items_set_item_like_cpp(chest_guid, true));
    assert!(session.record_represented_items_set_item_like_cpp(hands_guid, true));
    assert_eq!(
        session.represented_item_set_spell_events_like_cpp(),
        &[RepresentedItemSetSpellEventLikeCpp {
            item_set_id: 704,
            spell_entry_id: 20,
            spell_id: 9020,
            threshold: 2,
            apply: true,
        }]
    );

    assert!(!session.record_direct_inventory_item_set_remove_like_cpp(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        backpack_guid,
    ));
    assert_eq!(
        session.represented_item_set_spell_events_like_cpp().len(),
        1
    );

    assert!(session.record_direct_inventory_item_set_remove_like_cpp(
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_CHEST,
        chest_guid,
    ));
    assert_eq!(
        session.represented_item_set_spell_events_like_cpp()[1],
        RepresentedItemSetSpellEventLikeCpp {
            item_set_id: 704,
            spell_entry_id: 20,
            spell_id: 9020,
            threshold: 2,
            apply: false,
        },
        "C++ DestroyItem removes item-set bonuses for equipped/equipped-bag slots, and item-set bonuses still count broken items"
    );
}
#[test]
fn represented_item_set_guards_skill_legacy_flag_spec_and_broken_items_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let chest_guid = ObjectGuid::create_item(1, 908);
    let hands_guid = ObjectGuid::create_item(1, 909);
    session.set_player_guid(Some(player_guid));
    session.set_represented_primary_specialization_id_like_cpp(66);
    session.set_item_set_store(Arc::new(ItemSetStore::from_entries([
        ItemSetEntry {
            id: 701,
            name: "Skill Set".to_string(),
            set_flags: 0,
            required_skill: 333,
            required_skill_rank: 80,
            item_id: std::array::from_fn(|i| if i == 0 { 102 } else { 0 }),
        },
        ItemSetEntry {
            id: 702,
            name: "Inactive Set".to_string(),
            set_flags: ITEM_SET_FLAG_LEGACY_INACTIVE_LIKE_CPP,
            required_skill: 0,
            required_skill_rank: 0,
            item_id: std::array::from_fn(|i| if i == 0 { 103 } else { 0 }),
        },
        ItemSetEntry {
            id: 703,
            name: "Spec Set".to_string(),
            set_flags: 0,
            required_skill: 0,
            required_skill_rank: 0,
            item_id: std::array::from_fn(|i| match i {
                0 => 104,
                1 => 105,
                _ => 0,
            }),
        },
    ])));
    session.set_item_set_spell_store(Arc::new(ItemSetSpellStore::from_entries([
        ItemSetSpellEntry {
            id: 10,
            chr_spec_id: 0,
            spell_id: 9010,
            threshold: 1,
            item_set_id: 701,
        },
        ItemSetSpellEntry {
            id: 11,
            chr_spec_id: 0,
            spell_id: 9011,
            threshold: 1,
            item_set_id: 702,
        },
        ItemSetSpellEntry {
            id: 12,
            chr_spec_id: 65,
            spell_id: 9012,
            threshold: 2,
            item_set_id: 703,
        },
        ItemSetSpellEntry {
            id: 13,
            chr_spec_id: 66,
            spell_id: 9013,
            threshold: 2,
            item_set_id: 703,
        },
    ])));

    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        chest_guid,
        102,
        InventoryType::Chest,
    );
    assert!(!session.record_represented_items_set_item_like_cpp(chest_guid, true));
    session.set_player_skill_values_like_cpp(HashMap::from([(333, 80)]));
    assert!(session.record_represented_items_set_item_like_cpp(chest_guid, true));

    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_HANDS,
        hands_guid,
        103,
        InventoryType::Hands,
    );
    assert!(!session.record_represented_items_set_item_like_cpp(hands_guid, true));

    let first_spec_guid = ObjectGuid::create_item(1, 910);
    let second_spec_guid = ObjectGuid::create_item(1, 911);
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        first_spec_guid,
        104,
        InventoryType::Chest,
    );
    let mut broken_second = session.make_inventory_item_object(
        second_spec_guid,
        105,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_HANDS,
    );
    broken_second.set_max_durability(10);
    broken_second.set_durability(0);
    session.insert_inventory_item_object(broken_second);
    session.insert_inventory_item_like_cpp(
        EQUIPMENT_SLOT_HANDS,
        InventoryItem {
            guid: second_spec_guid,
            entry_id: 105,
            db_guid: second_spec_guid.counter() as u64,
            inventory_type: Some(InventoryType::Hands as u8),
        },
    );

    assert!(!session.record_represented_items_set_item_like_cpp(first_spec_guid, true));
    assert!(session.record_represented_items_set_item_like_cpp(second_spec_guid, true));
    assert!(
        !session
            .represented_item_set_spell_events_like_cpp()
            .iter()
            .any(|event| event.spell_id == 9012),
        "C++ AddItemsSetItem does not cast set spells for a non-primary ChrSpecID"
    );
    assert!(
        session
            .represented_item_set_spell_events_like_cpp()
            .iter()
            .any(|event| event.spell_id == 9013 && event.apply),
        "C++ item set bonuses are not dependent on item broken state"
    );
}
