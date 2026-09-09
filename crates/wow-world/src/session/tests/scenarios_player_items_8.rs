//! Session scenarios exercising the represented player items responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn represented_item_set_heirloom_max_level_guard_matches_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let chest_guid = ObjectGuid::create_item(1, 918);
    let hands_guid = ObjectGuid::create_item(1, 919);
    session.set_player_guid(Some(player_guid));
    session.set_player_level_like_cpp(19);
    session.set_item_set_store(Arc::new(ItemSetStore::from_entries([ItemSetEntry {
        id: 709,
        name: "Heirloom Curve Set".to_string(),
        set_flags: 0,
        required_skill: 0,
        required_skill_rank: 0,
        item_id: std::array::from_fn(|i| match i {
            0 => 112,
            1 => 113,
            _ => 0,
        }),
    }])));
    session.set_item_set_spell_store(Arc::new(ItemSetSpellStore::from_entries([
        ItemSetSpellEntry {
            id: 33,
            chr_spec_id: 0,
            spell_id: 9033,
            threshold: 2,
            item_set_id: 709,
        },
    ])));
    session.set_heirloom_store(Arc::new(HeirloomStore::from_entries([
        HeirloomEntry {
            id: 112,
            source_text: "test".to_string(),
            item_id: 112,
            legacy_upgraded_item_id: 0,
            static_upgraded_item_id: 0,
            source_type_enum: 0,
            flags: 0,
            legacy_item_id: 0,
            upgrade_item_id: [0; 6],
            upgrade_item_bonus_list_id: [0; 6],
        },
        HeirloomEntry {
            id: 113,
            source_text: "test".to_string(),
            item_id: 113,
            legacy_upgraded_item_id: 0,
            static_upgraded_item_id: 0,
            source_type_enum: 0,
            flags: 0,
            legacy_item_id: 0,
            upgrade_item_id: [0; 6],
            upgrade_item_bonus_list_id: [0; 6],
        },
    ])));
    let sparse = ItemSparseTemplateEntry {
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
        max_durability: 0,
        other_faction_item_id: 0,
        content_tuning_id: 55,
        player_level_to_item_level_curve_id: 77,
        limit_category: 0,
        instance_bound: 0,
        zone_bound: [0; 2],
        required_reputation_faction: 0,
        allowable_class: 0,
        required_expansion: 0,
        bonding: ItemBondingType::None as u8,
        container_slots: 0,
        inventory_type: InventoryType::Chest as i8,
    };
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [(112, sparse), (113, sparse)],
            [],
        ),
    ));
    session.set_curve_store(Arc::new(CurveStore::from_entries([CurveEntry {
        id: 77,
        curve_type: 0,
        flags: 0,
    }])));
    session.set_curve_point_store(Arc::new(CurvePointStore::from_entries([
        CurvePointEntry {
            id: 1,
            pos: [1.0, 10.0],
            pre_sl_squish_pos: [0.0, 0.0],
            curve_id: 77,
            order_index: 0,
        },
        CurvePointEntry {
            id: 2,
            pos: [20.0, 20.0],
            pre_sl_squish_pos: [0.0, 0.0],
            curve_id: 77,
            order_index: 1,
        },
    ])));
    session.set_content_tuning_store(Arc::new(ContentTuningStore::from_entries([
        ContentTuningEntry {
            id: 55,
            min_level: 1,
            max_level: 18,
            flags: 0,
            expected_stat_mod_id: 0,
            difficulty_esm_id: 0,
        },
    ])));
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        chest_guid,
        112,
        InventoryType::Chest,
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_HANDS,
        hands_guid,
        113,
        InventoryType::Hands,
    );

    assert!(
        !session.record_represented_items_set_item_like_cpp(chest_guid, true),
        "C++ AddItemsSetItem returns before creating the set effect when player level exceeds heirloom max level"
    );
    assert!(session.represented_item_set_effect_like_cpp(709).is_none());

    session.set_player_level_like_cpp(18);
    assert!(!session.record_represented_items_set_item_like_cpp(chest_guid, true));
    assert!(session.record_represented_items_set_item_like_cpp(hands_guid, true));
    assert_eq!(
        session.represented_item_set_spell_events_like_cpp(),
        &[RepresentedItemSetSpellEventLikeCpp {
            item_set_id: 709,
            spell_entry_id: 33,
            spell_id: 9033,
            threshold: 2,
            apply: true,
        }],
        "C++ only blocks heirloom item-set bonuses when player level is greater than the derived max level"
    );
}
#[test]
fn represented_item_set_uses_primary_spec_not_loot_spec_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 917);
    session.set_player_guid(Some(player_guid));
    session.set_loot_specialization_id_like_cpp(66);
    session.set_item_set_store(Arc::new(ItemSetStore::from_entries([ItemSetEntry {
        id: 708,
        name: "Primary Spec Set".to_string(),
        set_flags: 0,
        required_skill: 0,
        required_skill_rank: 0,
        item_id: std::array::from_fn(|i| if i == 0 { 111 } else { 0 }),
    }])));
    session.set_item_set_spell_store(Arc::new(ItemSetSpellStore::from_entries([
        ItemSetSpellEntry {
            id: 32,
            chr_spec_id: 66,
            spell_id: 9032,
            threshold: 1,
            item_set_id: 708,
        },
    ])));
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        item_guid,
        111,
        InventoryType::Chest,
    );

    assert!(!session.record_represented_items_set_item_like_cpp(item_guid, true));
    assert!(
        session
            .represented_item_set_spell_events_like_cpp()
            .is_empty(),
        "C++ HandleSetLootSpecialization changes LootSpecID only; item-set ChrSpecID uses GetPrimarySpecialization"
    );

    session.set_represented_primary_specialization_id_like_cpp(66);
    assert_eq!(
        session.record_represented_update_item_set_auras_like_cpp(false),
        2
    );
    assert_eq!(
        session.represented_item_set_aura_refresh_events_like_cpp(),
        &[
            RepresentedItemSetAuraRefreshEventLikeCpp {
                item_set_id: 708,
                spell_entry_id: 32,
                spell_id: 9032,
                apply: false,
                form_change: false,
            },
            RepresentedItemSetAuraRefreshEventLikeCpp {
                item_set_id: 708,
                spell_entry_id: 32,
                spell_id: 9032,
                apply: true,
                form_change: false,
            },
        ],
        "C++ UpdateItemSetAuras uses the current primary specialization, not LootSpecID"
    );
}
#[tokio::test]
async fn repair_item_handler_requires_repair_npc_and_repairs_single_item_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let repair_npc_guid = test_creature_guid(71_701);
    let vendor_npc_guid = test_creature_guid(71_702);
    let item_guid = ObjectGuid::create_item(1, 71_703);
    let feign_slot = 7;

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "RepairTester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_repair_cost_rate_like_cpp(2.0);
    session.set_faction_store(Arc::new(FactionStore::from_entries([
        FactionEntry::for_test_like_cpp(72, 5),
    ])));
    session.set_faction_template_store(Arc::new(FactionTemplateStore::from_entries([
        faction_template_entry(35, 72, 0, 0, 0),
        faction_template_entry(1, 1, 0, 0, 0),
    ])));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical map");
    session.set_player_faction_template_like_cpp(1);
    session.set_player_gold_like_cpp(500);
    assert!(session.load_character_reputation_rows_like_cpp([
        crate::reputation::mgr::CharacterReputationRowLikeCpp {
            faction_id: 72,
            standing: 9_000,
            flags: 0,
        },
    ]));
    add_canonical_test_creature(
        &canonical,
        repair_npc_guid,
        500,
        Position::new(14.0, 0.0, 0.0, 0.0),
        wow_constants::unit::NPCFlags1::REPAIR.bits(),
    );
    add_canonical_test_creature(
        &canonical,
        vendor_npc_guid,
        501,
        Position::new(14.0, 0.0, 0.0, 0.0),
        wow_constants::unit::NPCFlags1::VENDOR.bits(),
    );

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
        23,
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
        40,
        ItemContext::None,
        23,
    );
    session.insert_inventory_item_object(item);
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().add_unit_state(UnitState::DIED.bits());
        })
        .expect("canonical player");
    let mut feign_death = reputation_aura_for_test(
        feign_slot,
        RepresentedAuraEffectLikeCpp::FeignDeath,
        0,
        None,
    );
    feign_death.spell_id = 5384;
    assert!(session.insert_player_visible_aura_like_cpp(feign_death));

    session
        .handle_repair_item(wow_packet::packets::misc::RepairItem {
            npc_guid: vendor_npc_guid,
            item_guid,
            use_guild_bank: false,
        })
        .await;
    assert_eq!(session.player_gold_like_cpp(), 500);
    assert_eq!(
        session.inventory_item_objects_like_cpp()[&item_guid]
            .data()
            .durability,
        40
    );

    session
        .handle_repair_item(wow_packet::packets::misc::RepairItem {
            npc_guid: repair_npc_guid,
            item_guid,
            use_guild_bank: false,
        })
        .await;
    assert_eq!(session.player_gold_like_cpp(), 207);
    assert!(
        !session
            .resolved_player_visible_auras_like_cpp()
            .expect("canonical Player aura owner")
            .contains_key(&feign_slot)
    );
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| {
                player.unit().has_unit_state(UnitState::DIED.bits())
            })
            .expect("canonical player"),
        false
    );
    assert_eq!(
        session.inventory_item_objects_like_cpp()[&item_guid]
            .data()
            .durability,
        50
    );
}
#[tokio::test]
async fn repair_all_inventory_item_durability_charges_once_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let weapon_guid = ObjectGuid::create_item(1, 900);
    let bag_guid = ObjectGuid::create_item(1, 901);
    let armor_guid = ObjectGuid::create_item(1, 902);
    session.set_player_guid(Some(player_guid));
    session.set_player_gold_like_cpp(2_000);
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
        0,
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

    assert!(
        session
            .repair_all_inventory_item_durability_with_player_money_like_cpp(0.8, 2.0)
            .await
    );
    assert_eq!(session.player_gold_like_cpp(), 670);
    assert_eq!(
        session.inventory_item_objects_like_cpp()[&weapon_guid]
            .data()
            .durability,
        50
    );
    assert_eq!(
        session.inventory_item_objects_like_cpp()[&armor_guid]
            .data()
            .durability,
        13
    );
    assert_eq!(
        session.represented_item_mod_reapply_events_like_cpp(),
        &[RepresentedItemModsReapplyEventLikeCpp {
            item_guid: weapon_guid,
            slot: EQUIPMENT_SLOT_OFFHAND,
            apply: true,
        }],
        "C++ DurabilityRepairAll delegates each item to DurabilityRepair, which reapplies item mods when an equipped item was broken before repair"
    );
    assert_eq!(
        session.represented_item_bonus_actions_like_cpp(),
        &[
            RepresentedItemBonusActionLikeCpp {
                item_guid: weapon_guid,
                slot: EQUIPMENT_SLOT_OFFHAND,
                action: ApplyEnchantmentEffectAction::UnitModifier {
                    unit_mod: wow_entities::ApplyEnchantmentUnitMod::StatStrength,
                    modifier: wow_entities::ApplyEnchantmentUnitModifier::BaseValue,
                    amount: 12,
                    apply: true,
                },
            },
            RepresentedItemBonusActionLikeCpp {
                item_guid: weapon_guid,
                slot: EQUIPMENT_SLOT_OFFHAND,
                action: ApplyEnchantmentEffectAction::UpdateStatBuffMod(
                    wow_constants::Stats::Strength,
                ),
            },
            RepresentedItemBonusActionLikeCpp {
                item_guid: weapon_guid,
                slot: EQUIPMENT_SLOT_OFFHAND,
                action: ApplyEnchantmentEffectAction::RatingModifier {
                    rating: wow_entities::ApplyEnchantmentCombatRating::HitMelee,
                    amount: 5,
                    apply: true,
                },
            },
            RepresentedItemBonusActionLikeCpp {
                item_guid: weapon_guid,
                slot: EQUIPMENT_SLOT_OFFHAND,
                action: ApplyEnchantmentEffectAction::RatingModifier {
                    rating: wow_entities::ApplyEnchantmentCombatRating::HitRanged,
                    amount: 5,
                    apply: true,
                },
            },
            RepresentedItemBonusActionLikeCpp {
                item_guid: weapon_guid,
                slot: EQUIPMENT_SLOT_OFFHAND,
                action: ApplyEnchantmentEffectAction::RatingModifier {
                    rating: wow_entities::ApplyEnchantmentCombatRating::HitSpell,
                    amount: 5,
                    apply: true,
                },
            },
            RepresentedItemBonusActionLikeCpp {
                item_guid: weapon_guid,
                slot: EQUIPMENT_SLOT_OFFHAND,
                action: ApplyEnchantmentEffectAction::UnitModifier {
                    unit_mod: wow_entities::ApplyEnchantmentUnitMod::Resistance(
                        wow_constants::spell::SpellSchools::Normal as u32,
                    ),
                    modifier: wow_entities::ApplyEnchantmentUnitModifier::BaseValue,
                    amount: 17,
                    apply: true,
                },
            },
            RepresentedItemBonusActionLikeCpp {
                item_guid: weapon_guid,
                slot: EQUIPMENT_SLOT_OFFHAND,
                action: ApplyEnchantmentEffectAction::UnitModifier {
                    unit_mod: wow_entities::ApplyEnchantmentUnitMod::Resistance(
                        wow_constants::spell::SpellSchools::Fire as u32,
                    ),
                    modifier: wow_entities::ApplyEnchantmentUnitModifier::BaseValue,
                    amount: 7,
                    apply: true,
                },
            },
            RepresentedItemBonusActionLikeCpp {
                item_guid: weapon_guid,
                slot: EQUIPMENT_SLOT_OFFHAND,
                action: ApplyEnchantmentEffectAction::SetShieldBlockValue { amount: 42 },
            },
        ],
        "C++ _ApplyItemMods calls _ApplyItemBonuses before equip spells/auras/enchantments"
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::UpdateObject,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::UpdateObject
        ],
        "C++ DurabilityRepairAll delegates DurabilityRepair per item: repaired items send durability updates, and the equipped broken item reapply sends a stat VALUES delta"
    );

    session.set_player_gold_like_cpp(10);
    session
        .inventory_item_objects
        .get_mut(&weapon_guid)
        .unwrap()
        .set_durability(40);
    session
        .inventory_item_objects
        .get_mut(&armor_guid)
        .unwrap()
        .set_durability(10);
    assert!(
        !session
            .repair_all_inventory_item_durability_with_player_money_like_cpp(0.8, 2.0)
            .await
    );
    assert_eq!(session.player_gold_like_cpp(), 10);
    assert_eq!(
        session.inventory_item_objects_like_cpp()[&weapon_guid]
            .data()
            .durability,
        40
    );
    assert_eq!(
        session.inventory_item_objects_like_cpp()[&armor_guid]
            .data()
            .durability,
        10
    );
}
