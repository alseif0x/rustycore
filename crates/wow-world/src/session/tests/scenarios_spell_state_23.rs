//! Session scenarios exercising the represented spell state responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn initial_equipped_item_equip_auras_apply_on_equip_effects_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 420);
    let chest_guid = ObjectGuid::create_item(1, 920);
    let hands_guid = ObjectGuid::create_item(1, 921);
    let legacy_guid = ObjectGuid::create_item(1, 922);
    session.set_player_guid(Some(player_guid));
    session.set_represented_primary_specialization_id_like_cpp(66);
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        chest_guid,
        20_100,
        InventoryType::Chest,
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_HANDS,
        hands_guid,
        20_101,
        InventoryType::Hands,
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_HEAD,
        legacy_guid,
        20_102,
        InventoryType::Head,
    );

    let sparse = |inventory_type: InventoryType| ItemSparseTemplateEntry {
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
        allowable_class: -1,
        required_expansion: 0,
        bonding: ItemBondingType::None as u8,
        container_slots: 0,
        inventory_type: inventory_type as i8,
    };
    let mut legacy_sparse = sparse(InventoryType::Head);
    legacy_sparse.flags = [ItemFlags::LEGACY.bits() as u32, 0, 0, 0];
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([
        (20_100, sparse(InventoryType::Chest)),
        (20_101, sparse(InventoryType::Hands)),
        (20_102, legacy_sparse),
    ])));
    session.set_item_effect_store(Arc::new(ItemEffectStore::from_entries([
        ItemEffectEntry {
            id: 1,
            legacy_slot_index: 0,
            trigger_type: 1,
            charges: 0,
            cooldown_msec: -1,
            category_cooldown_msec: -1,
            spell_category_id: 0,
            spell_id: 30_100,
            chr_specialization_id: 0,
            parent_item_id: 20_100,
        },
        ItemEffectEntry {
            id: 2,
            legacy_slot_index: 1,
            trigger_type: 0,
            charges: 0,
            cooldown_msec: -1,
            category_cooldown_msec: -1,
            spell_category_id: 0,
            spell_id: 30_101,
            chr_specialization_id: 0,
            parent_item_id: 20_100,
        },
        ItemEffectEntry {
            id: 3,
            legacy_slot_index: 0,
            trigger_type: 1,
            charges: 0,
            cooldown_msec: -1,
            category_cooldown_msec: -1,
            spell_category_id: 0,
            spell_id: 30_102,
            chr_specialization_id: 66,
            parent_item_id: 20_101,
        },
        ItemEffectEntry {
            id: 4,
            legacy_slot_index: 0,
            trigger_type: 1,
            charges: 0,
            cooldown_msec: -1,
            category_cooldown_msec: -1,
            spell_category_id: 0,
            spell_id: 30_103,
            chr_specialization_id: 0,
            parent_item_id: 20_102,
        },
    ])));
    let mut spell_store = SpellStore::new();
    for spell_id in [30_100, 30_101, 30_102, 30_103] {
        spell_store.insert(
            spell_id,
            SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: 0,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE,
                    ..Default::default()
                }],
            },
        );
    }
    session.set_spell_store(Arc::new(spell_store));

    assert_eq!(
        session.apply_initial_equipped_item_equip_auras_like_cpp(),
        Some(2)
    );
    assert!(session.visible_auras.values().any(|aura| {
        aura.spell_id == 30_100
            && aura.caster_guid == chest_guid
            && aura.aura_flags == AFLAG_NOCASTER_LIKE_CPP | 0x0000_0100 | 0x0000_0200
            && aura.effect_mask == 1
    }));
    assert!(session.visible_auras.values().any(|aura| {
        aura.spell_id == 30_102 && aura.caster_guid == hands_guid && aura.effect_mask == 1
    }));
    assert!(
        !session
            .visible_auras
            .values()
            .any(|aura| aura.spell_id == 30_101 || aura.spell_id == 30_103)
    );
}
#[test]
fn initial_loaded_item_mods_follow_cpp_loaded_equip_enchant_aura_order() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 423);
    let item_guid = ObjectGuid::create_item(1, 923);
    let second_item_guid = ObjectGuid::create_item(1, 924);
    let broken_item_guid = ObjectGuid::create_item(1, 925);
    let item_id = 20_104;
    let second_item_id = 20_105;
    let broken_item_id = 20_106;
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
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        item_guid,
        item_id,
        InventoryType::Chest,
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_HANDS,
        second_item_guid,
        second_item_id,
        InventoryType::Hands,
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_FEET,
        broken_item_guid,
        broken_item_id,
        InventoryType::Feet,
    );
    session.update_inventory_item_object_like_cpp(item_guid, |item| {
        item.set_enchantment(EnchantmentSlot::EnhancementPermanent, 908, 0, 0);
    });
    session.update_inventory_item_object_like_cpp(second_item_guid, |item| {
        item.set_enchantment(EnchantmentSlot::EnhancementPermanent, 909, 0, 0);
    });
    session.update_inventory_item_object_like_cpp(broken_item_guid, |item| {
        item.set_max_durability(100);
        item.set_enchantment(EnchantmentSlot::EnhancementPermanent, 910, 0, 0);
    });
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([
        (
            item_id,
            sparse_template_for_inventory_type_like_cpp(InventoryType::Chest, 0),
        ),
        (
            second_item_id,
            sparse_template_for_inventory_type_like_cpp(InventoryType::Hands, 0),
        ),
        (
            broken_item_id,
            sparse_template_for_inventory_type_like_cpp(InventoryType::Feet, 0),
        ),
    ])));
    session.set_item_effect_store(Arc::new(ItemEffectStore::from_entries([
        ItemEffectEntry {
            id: 5,
            legacy_slot_index: 0,
            trigger_type: 1,
            charges: 0,
            cooldown_msec: -1,
            category_cooldown_msec: -1,
            spell_category_id: 0,
            spell_id: 30_105,
            chr_specialization_id: 0,
            parent_item_id: item_id,
        },
        ItemEffectEntry {
            id: 6,
            legacy_slot_index: 0,
            trigger_type: 1,
            charges: 0,
            cooldown_msec: -1,
            category_cooldown_msec: -1,
            spell_category_id: 0,
            spell_id: 30_107,
            chr_specialization_id: 0,
            parent_item_id: second_item_id,
        },
        ItemEffectEntry {
            id: 7,
            legacy_slot_index: 0,
            trigger_type: 1,
            charges: 0,
            cooldown_msec: -1,
            category_cooldown_msec: -1,
            spell_category_id: 0,
            spell_id: 30_109,
            chr_specialization_id: 0,
            parent_item_id: broken_item_id,
        },
    ])));
    let enchantment = |id, spell_id| SpellItemEnchantmentEntry {
        id,
        effect_arg: [spell_id, 0, 0],
        effect_points_min: [0; 3],
        item_visual: 0,
        flags: SpellItemEnchantmentFlags::empty(),
        required_skill_id: 0,
        required_skill_rank: 0,
        item_level: 1,
        charges: 0,
        effect: [
            ItemEnchantmentType::EquipSpell as u8,
            ItemEnchantmentType::None as u8,
            ItemEnchantmentType::None as u8,
        ],
        condition_id: 0,
        min_level: 1,
        max_level: 0,
    };
    session.set_spell_item_enchantment_store(Arc::new(SpellItemEnchantmentStore::from_entries([
        enchantment(908, 30_106),
        enchantment(909, 30_108),
        enchantment(910, 30_110),
    ])));
    let mut spell_store = SpellStore::new();
    for spell_id in [30_104, 30_105, 30_106, 30_107, 30_108, 30_109, 30_110] {
        spell_store.insert(
            spell_id,
            SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: 0,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE,
                    ..Default::default()
                }],
            },
        );
    }
    session.set_spell_store(Arc::new(spell_store));

    assert_eq!(
        session.load_represented_character_auras_like_cpp(
            [CharacterAuraRowLikeCpp {
                caster_guid: player_guid,
                spell_id: 30_104,
                effect_mask: 1,
                recalculate_mask: 0,
                difficulty: 0,
                stack_count: 1,
                max_duration_ms: -1,
                remain_time_ms: -1,
                remain_charges: 0,
            }],
            std::iter::empty::<CharacterAuraEffectRowLikeCpp>(),
            0,
        ),
        1
    );
    let outcome = session.apply_initial_loaded_item_mods_like_cpp(&[
        second_item_guid,
        broken_item_guid,
        item_guid,
    ]);

    assert_eq!(outcome.item_set_auras, 0);
    assert_eq!(outcome.item_equip_auras, 2);
    assert_eq!(
        outcome
            .enchantments
            .effect_actions
            .iter()
            .filter_map(|action| match action.action {
                ApplyEnchantmentEffectAction::CastEquipSpell {
                    spell_id,
                    item_guid,
                } => Some((spell_id, item_guid)),
                _ => None,
            })
            .collect::<Vec<_>>(),
        vec![(30_106, item_guid), (30_108, second_item_guid)]
    );
    let mut visible_auras = session.visible_auras.values().collect::<Vec<_>>();
    visible_auras.sort_by_key(|aura| aura.slot);
    assert_eq!(
        visible_auras
            .iter()
            .map(|aura| (aura.slot, aura.spell_id, aura.caster_guid))
            .collect::<Vec<_>>(),
        vec![
            (0, 30_104, player_guid),
            (1, 30_105, item_guid),
            (2, 30_106, item_guid),
            (3, 30_107, second_item_guid),
            (4, 30_108, second_item_guid),
        ],
        "C++ loads saved auras first, then replays equip and enchant auras per item in equipment-slot order"
    );
}
#[test]
fn login_passive_known_spell_auras_apply_like_cpp_addspell() {
    let (mut session, _, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 421);
    session.set_player_guid(Some(player_guid));
    let mut known_spells = vec![822, 28877, 14_914, 60_000, 60_001, 60_002, 60_003];

    let mut spell_store = SpellStore::new();
    for spell_id in [822, 28877, 14_914, 14_908, 60_000, 60_001, 60_002, 60_003] {
        let has_aura = spell_id != 60_001;
        spell_store.insert(
            spell_id,
            SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: if has_aura {
                    wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
                } else {
                    0
                },
                effect_base_points: 0,
                effect_bonus_coefficient: 0.0,
                aura_type: has_aura
                    .then_some(wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: has_aura
                    .then(|| {
                        vec![wow_data::SpellEffectInfo {
                            effect_index: 0,
                            effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                            effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE,
                            ..Default::default()
                        }]
                    })
                    .unwrap_or_default(),
            },
        );
    }
    for spell_id in [822, 28877, 14_908, 60_000, 60_001, 60_002, 60_003] {
        let mut attributes = [0u32; 15];
        attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_PASSIVE;
        spell_store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
    }
    spell_store.insert_spell_shapeshift_masks_like_cpp(60_000, 1 << 4, 0);
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_chain_store(Arc::new(
        wow_data::SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [wow_data::SpellRankEdgeLikeCpp {
                spell_id: 14_914,
                supercedes_spell_id: 14_908,
            }],
            |spell_id| matches!(spell_id, 14_908 | 14_914),
        ),
    ));
    session.set_spell_aura_restrictions_store(Arc::new(SpellAuraRestrictionsStore::from_entries(
        [wow_data::SpellAuraRestrictionsEntry {
            id: 1,
            difficulty_id: 0,
            caster_aura_state: 7,
            target_aura_state: 0,
            exclude_caster_aura_state: 0,
            exclude_target_aura_state: 0,
            caster_aura_spell: 0,
            target_aura_spell: 0,
            exclude_caster_aura_spell: 0,
            exclude_target_aura_spell: 0,
            spell_id: 60_002,
        }],
    )));
    session.set_spell_equipped_items_store(Arc::new(SpellEquippedItemsStore::from_entries([
        SpellEquippedItemsEntry {
            id: 1,
            spell_id: 60_003,
            equipped_item_class: ItemClass::Weapon as i8,
            equipped_item_inv_types: 0,
            equipped_item_subclass: 1_i32 << (ItemSubClassWeapon::Axe as u32),
        },
    ])));

    assert_eq!(
        session.apply_loaded_known_spell_dependencies_like_cpp(&mut known_spells),
        0
    );
    assert!(
        !known_spells.contains(&14_908),
        "C++ keeps the previous rank inactive when a higher known rank supersedes it"
    );
    session.set_known_spells_like_cpp(known_spells.clone());
    assert_eq!(session.apply_login_passive_known_spell_auras_like_cpp(), 2);
    assert_eq!(
        session.apply_loaded_known_spell_previous_rank_passive_auras_like_cpp(&known_spells),
        1,
        "C++ AddSpell recursively adds/casts previous passive ranks during load"
    );
    let visible: Vec<_> = session
        .visible_auras
        .values()
        .map(|aura| (aura.slot, aura.spell_id, aura.caster_guid, aura.aura_flags))
        .collect();
    assert!(visible.contains(&(
        0,
        822,
        player_guid,
        AFLAG_NOCASTER_LIKE_CPP | 0x0000_0100 | 0x0000_0200
    )));
    assert!(visible.contains(&(
        1,
        28877,
        player_guid,
        AFLAG_NOCASTER_LIKE_CPP | 0x0000_0100 | 0x0000_0200
    )));
    assert!(visible.contains(&(
        2,
        14908,
        player_guid,
        AFLAG_NOCASTER_LIKE_CPP | 0x0000_0100 | 0x0000_0200
    )));
    assert!(
        !session
            .visible_auras
            .values()
            .any(|aura| matches!(aura.spell_id, 60_000 | 60_001 | 60_002 | 60_003))
    );
}
#[test]
fn login_total_stat_percentage_aura_records_cpp_multiplier_and_misc_b_mask() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 15);
    session.set_player_guid(Some(player_guid));
    session.set_known_spells_like_cpp(vec![20_598]);

    let mut spell_store = SpellStore::new();
    spell_store.insert(
        20_598,
        SpellInfo {
            spell_id: 20_598,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE,
                effect_base_points: 2,
                effect_die_sides: 1,
                // C++ Unit::UpdateStatBuffMod filters this aura type with
                // MiscValueA, independently of the total-stat mask below.
                effect_misc_value_1: 4,
                // C++ HandleModTotalPercentStat selects with MiscValueB.
                // Zero applies the effect to all five primary stats.
                effect_misc_value_2: 0,
                ..Default::default()
            }],
            ..test_spell_info_like_cpp(20_598)
        },
    );
    let mut attributes = [0u32; 15];
    attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_PASSIVE;
    spell_store.insert_spell_misc_attributes_like_cpp(20_598, attributes);
    session.set_spell_store(Arc::new(spell_store));

    assert_eq!(session.apply_login_passive_known_spell_auras_like_cpp(), 1);
    assert_eq!(
        session.represented_total_stat_multipliers_like_cpp(),
        [1.03; 5]
    );
    assert_eq!(
        session.represented_total_stat_buff_multipliers_like_cpp(),
        [1.0, 1.0, 1.0, 1.0, 1.03]
    );
    let aura = session
        .visible_auras
        .values()
        .find(|aura| aura.spell_id == 20_598)
        .expect("Human Spirit aura");
    assert_eq!(
        aura.represented_effect,
        Some(RepresentedAuraEffectLikeCpp::ModTotalStatPercentage)
    );
    assert_eq!(aura.represented_amount, 3);
    assert_eq!(aura.represented_misc_value, Some(0));

    session.remove_aura(aura.slot).expect("remove Human Spirit");
    assert_eq!(
        session.represented_total_stat_multipliers_like_cpp(),
        [1.0; 5]
    );
    assert_eq!(
        session.represented_total_stat_buff_multipliers_like_cpp(),
        [1.0; 5]
    );
}
#[test]
fn represented_item_set_skips_unknown_spell_info_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let chest_guid = ObjectGuid::create_item(1, 920);
    let hands_guid = ObjectGuid::create_item(1, 921);
    let mut spell_store = SpellStore::new();
    spell_store.insert(
        9041,
        SpellInfo {
            spell_id: 9041,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );

    session.set_player_guid(Some(player_guid));
    session.set_spell_store(Arc::new(spell_store));
    session.set_item_set_store(Arc::new(ItemSetStore::from_entries([ItemSetEntry {
        id: 710,
        name: "Unknown Spell Set".to_string(),
        set_flags: 0,
        required_skill: 0,
        required_skill_rank: 0,
        item_id: std::array::from_fn(|i| match i {
            0 => 114,
            1 => 115,
            _ => 0,
        }),
    }])));
    session.set_item_set_spell_store(Arc::new(ItemSetSpellStore::from_entries([
        ItemSetSpellEntry {
            id: 34,
            chr_spec_id: 0,
            spell_id: 9040,
            threshold: 2,
            item_set_id: 710,
        },
        ItemSetSpellEntry {
            id: 35,
            chr_spec_id: 0,
            spell_id: 9041,
            threshold: 2,
            item_set_id: 710,
        },
    ])));

    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        chest_guid,
        114,
        InventoryType::Chest,
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_HANDS,
        hands_guid,
        115,
        InventoryType::Hands,
    );

    assert!(!session.record_represented_items_set_item_like_cpp(chest_guid, true));
    assert!(session.record_represented_items_set_item_like_cpp(hands_guid, true));
    assert_eq!(
        session.represented_item_set_spell_events_like_cpp(),
        &[RepresentedItemSetSpellEventLikeCpp {
            item_set_id: 710,
            spell_entry_id: 35,
            spell_id: 9041,
            threshold: 2,
            apply: true,
        }],
        "C++ AddItemsSetItem logs and continues before SetBonuses.insert when sSpellMgr has no SpellInfo"
    );
    assert_eq!(
        session
            .represented_item_set_effect_like_cpp(710)
            .expect("known spell still creates represented set effect")
            .set_bonuses,
        BTreeSet::from([35])
    );
}
#[test]
fn represented_update_item_set_auras_replays_active_bonuses_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let chest_guid = ObjectGuid::create_item(1, 915);
    let hands_guid = ObjectGuid::create_item(1, 916);
    session.set_player_guid(Some(player_guid));
    session.set_represented_primary_specialization_id_like_cpp(66);
    session.set_item_set_store(Arc::new(ItemSetStore::from_entries([ItemSetEntry {
        id: 707,
        name: "Refresh Set".to_string(),
        set_flags: 0,
        required_skill: 0,
        required_skill_rank: 0,
        item_id: std::array::from_fn(|i| match i {
            0 => 109,
            1 => 110,
            _ => 0,
        }),
    }])));
    session.set_item_set_spell_store(Arc::new(ItemSetSpellStore::from_entries([
        ItemSetSpellEntry {
            id: 30,
            chr_spec_id: 65,
            spell_id: 9030,
            threshold: 2,
            item_set_id: 707,
        },
        ItemSetSpellEntry {
            id: 31,
            chr_spec_id: 66,
            spell_id: 9031,
            threshold: 2,
            item_set_id: 707,
        },
    ])));

    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        chest_guid,
        109,
        InventoryType::Chest,
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_HANDS,
        hands_guid,
        110,
        InventoryType::Hands,
    );

    assert!(!session.record_represented_items_set_item_like_cpp(chest_guid, true));
    assert!(session.record_represented_items_set_item_like_cpp(hands_guid, true));
    assert_eq!(
        session.represented_item_set_spell_events_like_cpp(),
        &[RepresentedItemSetSpellEventLikeCpp {
            item_set_id: 707,
            spell_entry_id: 31,
            spell_id: 9031,
            threshold: 2,
            apply: true,
        }],
        "C++ AddItemsSetItem stores all threshold-met set bonuses but only casts the current-spec spell"
    );
    assert_eq!(
        session
            .represented_item_set_effect_like_cpp(707)
            .expect("set effect")
            .set_bonuses,
        BTreeSet::from([30, 31])
    );

    session.set_represented_primary_specialization_id_like_cpp(65);
    assert_eq!(
        session.record_represented_update_item_set_auras_like_cpp(true),
        2
    );
    assert_eq!(
        session.represented_item_set_aura_refresh_events_like_cpp(),
        &[
            RepresentedItemSetAuraRefreshEventLikeCpp {
                item_set_id: 707,
                spell_entry_id: 30,
                spell_id: 9030,
                apply: true,
                form_change: true,
            },
            RepresentedItemSetAuraRefreshEventLikeCpp {
                item_set_id: 707,
                spell_entry_id: 31,
                spell_id: 9031,
                apply: false,
                form_change: false,
            },
        ],
        "C++ ApplyEquipSpell(false, formChange=true) skips removal when the spell still fits the current shapeshift"
    );
}
#[test]
fn represented_update_item_set_auras_skips_apply_when_shapeshift_rejected_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 922);
    let mut spell_store = SpellStore::new();
    spell_store.insert(9042, test_spell_info_like_cpp(9042));
    spell_store.insert_spell_shapeshift_masks_like_cpp(9042, 1 << 4, 0);

    session.set_player_guid(Some(player_guid));
    session.set_spell_store(Arc::new(spell_store));
    session.set_item_set_store(Arc::new(ItemSetStore::from_entries([ItemSetEntry {
        id: 711,
        name: "Form Restricted Set".to_string(),
        set_flags: 0,
        required_skill: 0,
        required_skill_rank: 0,
        item_id: std::array::from_fn(|i| if i == 0 { 116 } else { 0 }),
    }])));
    session.set_item_set_spell_store(Arc::new(ItemSetSpellStore::from_entries([
        ItemSetSpellEntry {
            id: 36,
            chr_spec_id: 0,
            spell_id: 9042,
            threshold: 1,
            item_set_id: 711,
        },
    ])));
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        item_guid,
        116,
        InventoryType::Chest,
    );

    assert!(session.record_represented_items_set_item_like_cpp(item_guid, true));
    assert_eq!(
        session.record_represented_update_item_set_auras_like_cpp(false),
        1
    );
    assert_eq!(
        session.represented_item_set_aura_refresh_events_like_cpp(),
        &[RepresentedItemSetAuraRefreshEventLikeCpp {
            item_set_id: 711,
            spell_entry_id: 36,
            spell_id: 9042,
            apply: false,
            form_change: false,
        }],
        "C++ ApplyEquipSpell(true) returns without casting when CheckShapeshift is not OK"
    );
}
#[test]
fn represented_update_item_set_auras_form_change_skips_remove_when_form_still_fits_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 923);
    let mut spell_store = SpellStore::new();
    spell_store.insert(9043, test_spell_info_like_cpp(9043));
    spell_store.insert_spell_shapeshift_masks_like_cpp(9043, 1 << 4, 0);

    session.set_player_guid(Some(player_guid));
    session.set_spell_store(Arc::new(spell_store));
    session.set_represented_shapeshift_form_like_cpp(5);
    session.set_item_set_store(Arc::new(ItemSetStore::from_entries([ItemSetEntry {
        id: 712,
        name: "Matching Form Set".to_string(),
        set_flags: 0,
        required_skill: 0,
        required_skill_rank: 0,
        item_id: std::array::from_fn(|i| if i == 0 { 117 } else { 0 }),
    }])));
    session.set_item_set_spell_store(Arc::new(ItemSetSpellStore::from_entries([
        ItemSetSpellEntry {
            id: 37,
            chr_spec_id: 0,
            spell_id: 9043,
            threshold: 1,
            item_set_id: 712,
        },
    ])));
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        item_guid,
        117,
        InventoryType::Chest,
    );

    assert!(session.record_represented_items_set_item_like_cpp(item_guid, true));
    assert_eq!(
        session.record_represented_update_item_set_auras_like_cpp(true),
        1
    );
    assert_eq!(
        session.represented_item_set_aura_refresh_events_like_cpp(),
        &[RepresentedItemSetAuraRefreshEventLikeCpp {
            item_set_id: 712,
            spell_entry_id: 37,
            spell_id: 9043,
            apply: true,
            form_change: true,
        }],
        "C++ ApplyEquipSpell(false, formChange=true) returns early when CheckShapeshift is OK"
    );
}
