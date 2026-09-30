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
        price_variance: 0.0,
        price_random_value: 0.0,
        ..inventory_sparse_template_for_test(inventory_type as i8)
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
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
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
        effect_scaling_points: [0.0; 3],
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
    let mut visible_auras = session
        .canonical_player_snapshot_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .runtime_applications_like_cpp()
                .values()
                .cloned()
                .collect::<Vec<_>>()
        })
        .expect("canonical Player aura owner");
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
