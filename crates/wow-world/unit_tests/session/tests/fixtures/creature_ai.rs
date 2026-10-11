//! Creature spell, wire, and melee fixtures.
//!
//! These builders retain the original session-test behavior and are
//! visible only within the parent `session::tests` subtree.

use super::*;

pub(in crate::session::tests) fn aggro_hostile_config_like_cpp() -> CreatureAggroConfigLikeCpp {
    aggro_relation_config_like_cpp(
        faction_template_entry(14, 72, 0, 0, 930),
        faction_template_entry(1, 930, 0, 0, 0),
        FactionEntry::for_test_like_cpp(72, 1),
    )
}

pub(in crate::session::tests) fn aggro_hostile_config_with_rate_like_cpp(
    creature_aggro_rate: f32,
) -> CreatureAggroConfigLikeCpp {
    CreatureAggroConfigLikeCpp {
        creature_aggro_rate,
        ..aggro_hostile_config_like_cpp()
    }
}

pub(in crate::session::tests) fn creature_ai_spell_test_config_like_cpp(
    spell: wow_data::SpellInfo,
    passive: bool,
    range_max: f32,
) -> CreatureAggroConfigLikeCpp {
    let spell_id = u32::try_from(spell.spell_id).unwrap();
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(spell.spell_id, spell);
    let mut misc = spell_misc_entry_like_cpp(8_001, spell_id, 71);
    if passive {
        misc.attributes[0] |= wow_data::spell::attributes::SPELL_ATTR0_PASSIVE as i32;
    }
    spell_store.insert_spell_misc_attributes_like_cpp(
        i32::try_from(spell_id).unwrap(),
        misc.attributes.map(|attribute| attribute as u32),
    );
    spell_store.insert_spell_hit_metadata_for_difficulty_like_cpp(
        i32::try_from(spell_id).unwrap(),
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
    CreatureAggroConfigLikeCpp {
        spell_misc_store: Some(Arc::new(wow_data::SpellMiscStore::from_entries([misc]))),
        spell_range_store: Some(Arc::new(wow_data::SpellRangeStore::from_entries([
            spell_range_entry_like_cpp(71, 0.0, range_max),
        ]))),
        spell_cooldowns_store: Some(Arc::new(wow_data::SpellCooldownsStore::from_entries([
            wow_data::SpellCooldownsEntry {
                id: 8_002,
                difficulty_id: 0,
                category_recovery_time: 12_000,
                recovery_time: 6_000,
                start_recovery_time: 0,
                spell_id,
            },
        ]))),
        spell_category_store: Some(Arc::new(SpellCategoryStore::from_entries([]))),
        spell_x_spell_visual_store: Some(Arc::new(wow_data::SpellXSpellVisualStore::from_entries(
            [wow_data::SpellXSpellVisualEntry {
                id: 8_003,
                difficulty_id: 0,
                spell_visual_id: 99_999,
                probability: 1.0,
                flags: 0,
                priority: 0,
                spell_icon_file_id: 0,
                active_icon_file_id: 0,
                viewer_unit_condition_id: 0,
                viewer_player_condition_id: 0,
                caster_unit_condition_id: 0,
                caster_player_condition_id: 0,
                spell_id,
            }],
        ))),
        spell_target_restrictions_store: Some(Arc::new(
            wow_data::SpellTargetRestrictionsStore::from_entries([]),
        )),
        spell_casting_requirements_store: Some(Arc::new(
            wow_data::SpellCastingRequirementsStore::from_entries([]),
        )),
        spell_aura_restrictions_store: Some(Arc::new(
            wow_data::SpellAuraRestrictionsStore::from_entries([]),
        )),
        spell_store: Some(Arc::new(spell_store)),
        spell_chain_store: Some(Arc::new(SpellChainStoreLikeCpp::default())),
        spell_linked_store: Some(Arc::new(SpellLinkedStoreLikeCpp::default())),
        spell_condition_store: Some(Arc::new(ConditionEntriesByTypeStore::default())),
        spell_script_exact_spell_ids_like_cpp: Some(Arc::new(BTreeSet::new())),
        spell_script_all_rank_root_spell_ids_like_cpp: Some(Arc::new(BTreeSet::new())),
        legacy_spell_script_spell_ids_like_cpp: Some(Arc::new(BTreeSet::new())),
        spell_linked_rejected_trigger_spell_ids_like_cpp: Some(Arc::new(BTreeSet::new())),
        ..aggro_hostile_config_like_cpp()
    }
}

pub(in crate::session::tests) fn creature_spell_bound_hook_tick_fixture_like_cpp(
    ai_name: &str,
    passive: bool,
    spell_id: i32,
    creature_counter: i64,
    victim_counter: i64,
) -> (
    SharedCanonicalMapManager,
    SharedCanonicalMapManager,
    CreatureAggroConfigLikeCpp,
    ObjectGuid,
) {
    let manager = shared_map_manager();
    let canonical = Arc::clone(&manager);
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(creature_counter);
    let victim_guid = ObjectGuid::create_player(1, victim_counter);
    add_canonical_creature_spell_test_pair_like_cpp(&canonical, creature_guid, victim_guid);
    let manager = session
        .core
        .canonical_map_manager
        .clone()
        .unwrap_or_else(|| manager.clone());
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.set_ai_identity_names_runtime_like_cpp(ai_name, String::new());
            creature.set_spell(0, u32::try_from(spell_id).unwrap());
            creature.enter_combat(victim_guid);
            creature.ai_ownership_mut().last_swing_ms = 0;
            creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    let mut config = creature_ai_spell_test_config_like_cpp(
        creature_ai_test_spell_info_like_cpp(spell_id, 6, 0),
        passive,
        30.0,
    );
    config.spell_script_exact_spell_ids_like_cpp =
        Some(Arc::new(BTreeSet::from([u32::try_from(spell_id).unwrap()])));
    (manager, canonical, config, creature_guid)
}

pub(in crate::session::tests) fn decode_atomic_creature_spell_wire_pair_like_cpp(
    event: &RuntimeEvent,
) -> (
    CreatureSpellWireHeaderLikeCpp,
    CreatureSpellWireHeaderLikeCpp,
) {
    let basic_go_packet_bytes = match &event.recipients {
        crate::runtime_plan::RecipientRule::NearbyVisibleDurableSpellCast {
            basic_go_packet_bytes,
            ..
        } => basic_go_packet_bytes,
        _ => panic!("creature spell START/GO must use the atomic cast recipient rule"),
    };
    (
        decode_creature_spell_wire_header_like_cpp(&event.packet_bytes),
        decode_creature_spell_wire_header_like_cpp(basic_go_packet_bytes),
    )
}
