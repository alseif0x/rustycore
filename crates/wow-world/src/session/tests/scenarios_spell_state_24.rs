//! Session scenarios exercising the represented spell state responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn represented_item_set_aura_refresh_materializes_remove_then_apply_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 924);

    session.set_player_guid(Some(player_guid));
    session.visible_auras.insert(1, test_visible_aura(1, 9044));
    session.visible_auras.insert(2, test_visible_aura(2, 9999));
    session.set_item_set_store(Arc::new(ItemSetStore::from_entries([ItemSetEntry {
        id: 713,
        name: "Materialized Refresh Set".to_string(),
        set_flags: 0,
        required_skill: 0,
        required_skill_rank: 0,
        item_id: std::array::from_fn(|i| if i == 0 { 118 } else { 0 }),
    }])));
    session.set_item_set_spell_store(Arc::new(ItemSetSpellStore::from_entries([
        ItemSetSpellEntry {
            id: 38,
            chr_spec_id: 0,
            spell_id: 9044,
            threshold: 1,
            item_set_id: 713,
        },
    ])));
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        item_guid,
        118,
        InventoryType::Chest,
    );

    assert!(session.record_represented_items_set_item_like_cpp(item_guid, true));
    assert_eq!(
        session.apply_represented_item_set_aura_refresh_events_like_cpp(false),
        2
    );
    assert_eq!(
        session
            .visible_auras
            .values()
            .filter(|aura| aura.spell_id == 9044)
            .count(),
        1,
        "C++ UpdateItemSetAuras calls RemoveAurasDueToSpell before CastSpell for item-set auras"
    );
    assert!(
        session
            .visible_auras
            .values()
            .any(|aura| aura.spell_id == 9999),
        "C++ RemoveAurasDueToSpell removes only matching spell ids"
    );
}
#[test]
fn represented_item_set_aura_refresh_form_change_does_not_duplicate_active_aura_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 925);
    let mut spell_store = SpellStore::new();
    spell_store.insert(9045, test_spell_info_like_cpp(9045));
    spell_store.insert_spell_shapeshift_masks_like_cpp(9045, 1 << 4, 0);

    session.set_player_guid(Some(player_guid));
    session.set_spell_store(Arc::new(spell_store));
    session.set_represented_shapeshift_form_like_cpp(5);
    session.visible_auras.insert(1, test_visible_aura(1, 9045));
    session.set_item_set_store(Arc::new(ItemSetStore::from_entries([ItemSetEntry {
        id: 714,
        name: "Active Aura Refresh Set".to_string(),
        set_flags: 0,
        required_skill: 0,
        required_skill_rank: 0,
        item_id: std::array::from_fn(|i| if i == 0 { 119 } else { 0 }),
    }])));
    session.set_item_set_spell_store(Arc::new(ItemSetSpellStore::from_entries([
        ItemSetSpellEntry {
            id: 39,
            chr_spec_id: 0,
            spell_id: 9045,
            threshold: 1,
            item_set_id: 714,
        },
    ])));
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        item_guid,
        119,
        InventoryType::Chest,
    );

    assert!(session.record_represented_items_set_item_like_cpp(item_guid, true));
    assert_eq!(
        session.apply_represented_item_set_aura_refresh_events_like_cpp(true),
        1
    );
    assert_eq!(
        session
            .visible_auras
            .values()
            .filter(|aura| aura.spell_id == 9045)
            .count(),
        1,
        "C++ ApplyEquipSpell(true, formChange=true) returns when the item-set aura is already active"
    );
}
#[test]
fn spell_item_enchantment_helpers_use_cpp_store_fields() {
    let (mut session, _, _) = make_session();
    session.set_spell_item_enchantment_store(Arc::new(SpellItemEnchantmentStore::from_entries([
        SpellItemEnchantmentEntry {
            id: 900,
            effect_arg: [7, 8, 9],
            effect_points_min: [10, -2, 30],
            item_visual: 44,
            flags: SpellItemEnchantmentFlags::ALLOW_ENTERING_ARENA,
            required_skill_id: 333,
            required_skill_rank: 75,
            item_level: 11,
            charges: 0,
            effect: [
                ItemEnchantmentType::Resistance as u8,
                ItemEnchantmentType::Stat as u8,
                250,
            ],
            condition_id: 12,
            min_level: 20,
            max_level: 0,
        },
    ])));

    assert!(session.is_arena_allowed_enchantment(900));
    assert!(!session.is_arena_allowed_enchantment(901));

    let template = session
        .apply_enchantment_template_ref(900, 80, false)
        .expect("template should resolve from SpellItemEnchantment.db2");
    assert_eq!(template.enchantment_id, 900);
    assert_eq!(template.condition_id, 12);
    assert!(!template.condition_fits);
    assert_eq!(template.min_level, 20);
    assert_eq!(template.required_skill_id, 333);
    assert_eq!(template.required_skill_rank, 75);
    assert_eq!(template.required_skill_value, 80);

    assert_eq!(
        session.apply_enchantment_effect_refs(900).unwrap(),
        [
            ApplyEnchantmentEffectRef::known(ItemEnchantmentType::Resistance, 10, 7),
            ApplyEnchantmentEffectRef::known(ItemEnchantmentType::Stat, (-2i16) as u32, 8),
            ApplyEnchantmentEffectRef::unknown(250, 30, 9),
        ]
    );
}
#[test]
fn spell_enchant_proc_event_lookup_matches_cpp_store_like_cpp() {
    let (mut session, _, _) = make_session();
    assert!(session.spell_enchant_proc_event_like_cpp(900).is_none());

    let mut entries = HashMap::new();
    entries.insert(
        900,
        wow_data::SpellEnchantProcEntryLikeCpp {
            chance: 12.5,
            procs_per_minute: 1.2,
            hit_mask: 0x10,
            attributes_mask: 0x2,
        },
    );
    session.set_spell_enchant_proc_store(Arc::new(wow_data::SpellEnchantProcStoreLikeCpp {
        entries_by_enchant_id: entries,
    }));

    let entry = session
        .spell_enchant_proc_event_like_cpp(900)
        .expect("enchant proc entry");
    assert_eq!(entry.chance, 12.5);
    assert_eq!(entry.procs_per_minute, 1.2);
    assert_eq!(entry.hit_mask, 0x10);
    assert_eq!(entry.attributes_mask, 0x2);
    assert!(session.spell_enchant_proc_event_like_cpp(901).is_none());
}
#[test]
fn loaded_equipped_item_enchantments_apply_equip_spell_aura_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 90_524);
    let player_position = Position::new(1.0, 2.0, 3.0, 0.0);
    let item_guid = ObjectGuid::create_item(1, 90_525);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    session
        .mutate_canonical_player_like_cpp(|player| player.unit_mut().set_level(80))
        .unwrap();
    session.set_spell_item_enchantment_store(Arc::new(SpellItemEnchantmentStore::from_entries([
        SpellItemEnchantmentEntry {
            id: 907,
            effect_arg: [1234, 0, 0],
            effect_points_min: [0, 0, 0],
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
        },
    ])));
    let mut spell_store = SpellStore::new();
    spell_store.insert(
        1234,
        SpellInfo {
            spell_id: 1234,
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
    session.set_spell_store(Arc::new(spell_store));

    let mut item = session.make_inventory_item_object(
        item_guid,
        702,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_CHEST,
    );
    item.set_enchantment(EnchantmentSlot::EnhancementPermanent, 907, 0, 0);
    session.insert_inventory_item_object(item);

    let outcome = session.apply_loaded_equipped_item_enchantments_like_cpp(item_guid);

    assert!(matches!(
        outcome.effect_actions.first().map(|action| action.action),
        Some(ApplyEnchantmentEffectAction::CastEquipSpell {
            spell_id: 1234,
            item_guid: action_item_guid,
        }) if action_item_guid == item_guid
    ));
    assert!(outcome.unrepresented_effect_actions.is_empty());
    assert!(!outcome.send_stat_update);
    assert_eq!(
        session.represented_item_bonus_actions_like_cpp(),
        outcome.effect_actions.as_slice()
    );
    assert!(session.visible_auras.values().any(|aura| {
        aura.spell_id == 1234 && aura.caster_guid == item_guid && aura.effect_mask == 1
    }));
    assert!(
        send_rx.try_recv().is_err(),
        "login equip-spell aura is included in the later full AuraUpdate"
    );
}
#[test]
fn send_new_item_plan_group_broadcasts_to_group_members_including_self() {
    let (mut session, _, send_rx) = make_session();
    let self_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 43);
    let (self_tx, self_rx) = flume::bounded(10);
    let (self_realm_tx, self_realm_rx) = flume::bounded(10);
    let (other_tx, other_rx) = flume::bounded(10);
    let (other_realm_tx, other_realm_rx) = flume::bounded(10);
    let player_registry = Arc::new(PlayerRegistry::default());
    let mut self_info = broadcast_info(self_guid, self_tx);
    self_info.realm_send_tx = self_realm_tx;
    player_registry.register_or_replace(self_guid, self_info, Default::default());
    let mut other_info = broadcast_info(other_guid, other_tx);
    other_info.realm_send_tx = other_realm_tx;
    player_registry.register_or_replace(other_guid, other_info, Default::default());
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(self_guid);
    group.add_member(other_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.player_guid = Some(self_guid);
    session.group_guid = Some(group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    let plan = send_new_item_plan(SendNewItemDelivery::GroupBroadcast);
    let expected = crate::session_rules::item_push_result_from_send_new_item_plan(&plan).to_bytes();

    session.send_new_item_plan(&plan);

    assert_eq!(self_realm_rx.try_recv().unwrap(), expected);
    assert_eq!(other_realm_rx.try_recv().unwrap(), expected);
    assert!(self_rx.try_recv().is_err());
    assert!(other_rx.try_recv().is_err());
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn legacy_turret_ai_cast_resets_attack_timer_and_never_melees_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_208);
    let victim_guid = ObjectGuid::create_player(1, 91_209);
    let spell_id = 70_007_i32;
    add_canonical_creature_spell_test_pair_like_cpp(&canonical, creature_guid, victim_guid);
    register_test_creature_mirrored_like_cpp(
        &mut session,
        manager.clone(),
        &canonical,
        creature_guid,
        25,
    );
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .set_ai_identity_names_runtime_like_cpp("TurretAI", String::new());
            creature
                .creature
                .set_spell(0, u32::try_from(spell_id).unwrap());
            creature.enter_combat(victim_guid);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            assert!(creature.can_swing());
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);
    let config = creature_ai_spell_test_config_like_cpp(
        creature_ai_test_spell_info_like_cpp(spell_id, 6, 0),
        false,
        30.0,
    );

    let cast = run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(cast.casts_ready, 1, "spell tick outcome: {cast:?}");
    assert_eq!(cast.canonical_cast_preconditions_passed, 1);
    assert_eq!(cast.plan.events.len(), 1);
    let (start, go) = decode_atomic_creature_spell_wire_pair_like_cpp(&cast.plan.events[0]);
    assert_eq!(start.opcode, ServerOpcodes::SpellStart as u16);
    assert_eq!(go.opcode, ServerOpcodes::SpellGo as u16);
    let can_swing_immediately = session
        .mutate_world_creature(creature_guid, |creature| creature.can_swing())
        .unwrap();
    assert!(!can_swing_immediately);

    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.backdate_runtime_clock_for_test(Duration::from_secs(60));
            assert!(creature.can_swing());
        })
        .unwrap();
    let melee = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical));
    assert_eq!(melee.swings_ready, 0);
    assert_eq!(melee.canonical_hits, 0);
    assert!(melee.commands.is_empty());
}
#[test]
fn legacy_turret_ai_disabled_spell_resets_swing_only_inside_raw_range_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    // C++ `DoSpellAttackIfReady` gates on `IsWithinCombatRange(max +
    // combat reaches)` and only then calls `CastSpell`/`resetAttackTimer`.
    // A `disables` row makes `Spell::CheckCast` reject the cast, so the
    // swing is still consumed inside that raw range but never outside it.
    const SPELL_ID: u32 = 70_141;
    const RANGE_MAX: f32 = 5.0;
    const COMBAT_REACH: f32 = 1.0;

    let attempt = |victim_x: f32| {
        let manager = shared_map_manager();
        let canonical = shared_canonical_map_manager();
        let (mut session, _, _) = make_session();
        let creature_guid = test_creature_guid(91_312);
        let victim_guid = ObjectGuid::create_player(1, 91_313);
        add_canonical_creature_spell_test_pair_like_cpp(&canonical, creature_guid, victim_guid);
        register_test_creature_mirrored_like_cpp(
            &mut session,
            manager.clone(),
            &canonical,
            creature_guid,
            25,
        );
        session
            .mutate_world_creature(creature_guid, |creature| {
                creature
                    .creature
                    .set_ai_identity_names_runtime_like_cpp("TurretAI", String::new());
                creature.creature.set_spell(0, SPELL_ID);
                creature.enter_combat(victim_guid);
                creature.creature.ai_ownership_mut().last_swing_ms = 0;
                creature.creature.ai_ownership_mut().swing_timer_ms = 0;
                assert!(creature.can_swing());
            })
            .unwrap();
        // Both registration and legacy mutation mirror into the canonical
        // map, so pin the exact range inputs last.
        {
            let mut canonical = canonical.lock().unwrap();
            let map = canonical.find_map_mut(0, 0).unwrap().map_mut();
            let caster = map.get_typed_creature_mut(creature_guid).unwrap();
            caster.unit_mut().world_mut().set_combat_reach(COMBAT_REACH);
            let victim = map.get_typed_player_mut(victim_guid).unwrap();
            victim.unit_mut().world_mut().set_combat_reach(COMBAT_REACH);
            victim
                .unit_mut()
                .world_mut()
                .relocate(Position::new(victim_x, 10.0, 0.0, 0.0));
        }
        manager
            .write()
            .unwrap()
            .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

        let mut config = creature_ai_spell_test_config_like_cpp(
            creature_ai_test_spell_info_like_cpp(SPELL_ID as i32, 6, 0),
            false,
            RANGE_MAX,
        );
        config.spell_range_store = Some(Arc::new(wow_data::SpellRangeStore::from_entries([
            spell_range_entry_like_cpp(71, 0.0, RANGE_MAX),
        ])));
        config.disable_mgr = Some(Arc::new(
            wow_data::DisableMgrLikeCpp::from_rows_like_cpp(
                [wow_data::DisableDbRowLikeCpp {
                    source_type: wow_data::DISABLE_TYPE_SPELL,
                    entry: SPELL_ID,
                    flags: wow_data::SPELL_DISABLE_CREATURE,
                    params_0: String::new(),
                    params_1: String::new(),
                }],
                wow_data::DisableMgrRefsLikeCpp::default(),
            )
            .0,
        ));

        let outcome =
            run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);
        let swing_ready = session
            .mutate_world_creature(creature_guid, |creature| creature.can_swing())
            .unwrap();
        (outcome, swing_ready)
    };

    // Center distance 6 is strictly inside max 5 + reaches 1 + 1.
    let (inside, swing_ready) = attempt(16.0);
    assert_eq!(inside.spells_disabled, 1, "spell tick outcome: {inside:?}");
    assert_eq!(inside.casts_ready, 0);
    assert!(inside.plan.events.is_empty());
    assert_eq!(inside.turret_rejected_attempt_swings, 1);
    assert!(
        !swing_ready,
        "an in-range disabled spell still consumes the TurretAI swing"
    );

    // `IsWithinCombatRange` is strict, so the bound itself is out of range.
    let (bound, swing_ready) = attempt(17.0);
    assert_eq!(bound.spells_disabled, 1, "spell tick outcome: {bound:?}");
    assert_eq!(bound.turret_rejected_attempt_swings, 0);
    assert!(
        swing_ready,
        "the raw max + reaches bound itself must not reset BASE_ATTACK"
    );

    let (outside, swing_ready) = attempt(30.0);
    assert_eq!(
        outside.spells_disabled, 1,
        "spell tick outcome: {outside:?}"
    );
    assert_eq!(outside.turret_rejected_attempt_swings, 0);
    assert!(
        swing_ready,
        "an out-of-range disabled spell leaves BASE_ATTACK ready"
    );
}
#[test]
fn legacy_turret_ai_ignores_noninstant_spells_outside_slot_zero_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_308);
    let victim_guid = ObjectGuid::create_player(1, 91_309);
    let turret_spell_id = 70_102_i32;
    let ignored_spell_id = 70_103_i32;
    add_canonical_creature_spell_test_pair_like_cpp(&canonical, creature_guid, victim_guid);
    register_test_creature_mirrored_like_cpp(
        &mut session,
        manager.clone(),
        &canonical,
        creature_guid,
        25,
    );
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .set_ai_identity_names_runtime_like_cpp("TurretAI", String::new());
            creature
                .creature
                .set_spell(0, u32::try_from(turret_spell_id).unwrap());
            creature
                .creature
                .set_spell(1, u32::try_from(ignored_spell_id).unwrap());
            creature.enter_combat(victim_guid);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            assert!(creature.can_swing());
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let turret_spell = creature_ai_test_spell_info_like_cpp(turret_spell_id, 6, 0);
    let mut ignored_noninstant_spell = creature_ai_test_spell_info_like_cpp(ignored_spell_id, 6, 0);
    ignored_noninstant_spell.cast_time_ms = 1_500;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(turret_spell_id, turret_spell.clone());
    spell_store.insert(ignored_spell_id, ignored_noninstant_spell);
    spell_store.insert_spell_misc_attributes_like_cpp(
        turret_spell_id,
        represented_creature_spell_test_attributes_like_cpp(true),
    );
    spell_store.insert_spell_hit_metadata_for_difficulty_like_cpp(
        turret_spell_id,
        0,
        wow_data::SpellHitMetadataLikeCpp {
            category_id: 0,
            charge_category_id: 0,
            defense_type: 2,
            spell_mechanic: 0,
            school_mask: 0x01,
            effect_mechanics: BTreeMap::from([(0, 0)]),
        },
    );
    let mut config = creature_ai_spell_test_config_like_cpp(turret_spell, false, 30.0);
    config.spell_store = Some(Arc::new(spell_store));

    let cast = run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);

    assert_eq!(cast.noninstant_casts_unrepresented, 0);
    assert_eq!(cast.casts_ready, 1, "spell tick outcome: {cast:?}");
    assert_eq!(cast.plan.events.len(), 1);
    let (start, go) = decode_atomic_creature_spell_wire_pair_like_cpp(&cast.plan.events[0]);
    assert_eq!(start.spell_id, turret_spell_id);
    assert_eq!(go.spell_id, turret_spell_id);
}
