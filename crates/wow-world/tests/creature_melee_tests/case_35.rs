use super::*;

/// C++ `Unit::CalcAbsorbResist`'s ignore-absorb term (`Unit.cpp:1803-1832`).
///
/// A creature attacker carrying `SPELL_AURA_MOD_TARGET_ABSORB_SCHOOL` may only
/// push the non-ignored portion of its hit into the victim's school-absorb
/// shield, unless that shield's spell carries
/// `SPELL_ATTR6_ABSORB_CANNOT_BE_IGNORE`.
#[test]
fn legacy_creature_melee_tick_once_honors_ignore_absorb_like_cpp() {
    use wow_world::map_manager::RuntimeTickOwner;
    use wow_packet::packets::combat::{HIT_INFO_AFFECTS_VICTIM, HIT_INFO_PARTIAL_ABSORB};

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);

    let player = ObjectGuid::create_player(1, 91_800);
    let creature_guid = test_creature_guid(91_801);

    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.fixture_melee_attach_player_controller(SessionPlayerController::new(
        player,
        "Victim".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.fixture_melee_ensure_world_map();
    session
        .fixture_melee_mutate_player(|player| {
            player.unit_mut().set_max_health(100);
            player.unit_mut().set_health(100);
            let mut stats = *player.effective_combat_stats_like_cpp();
            stats.dodge_pct = 0.0;
            stats.parry_pct = 0.0;
            stats.block_pct = 0.0;
            player.replace_effective_combat_stats_like_cpp(stats);
        })
        .unwrap();
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .fixture_melee_mutate_creature(creature_guid, |creature| {
            creature
                .creature
                .set_ai_position(Position::new(10.0, 10.0, 0.0, 0.0));
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature.creature.ai_ownership_mut().min_damage = 10;
            creature.creature.ai_ownership_mut().max_damage = 10;
            creature.creature.set_flags_extra_runtime_like_cpp(
                wow_constants::CreatureFlagsExtra::NO_CRIT.bits(),
            );
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            // C++ `GetMaxPositiveAuraModifierByMiscMask` reads the attacker's
            // own `SPELL_AURA_MOD_TARGET_ABSORB_SCHOOL` effects.
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .add_applied(wow_entities::AppliedAuraRef::new(91_812_u32, player, 0, 1));
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, amount, misc_value) in [
        (
            91_810_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
            5_i32,
            0_i32,
        ),
        (
            91_811,
            wow_data::spell::aura_types::SPELL_AURA_SCHOOL_ABSORB,
            30,
            0x01,
        ),
        (
            91_812,
            wow_data::spell::aura_types::SPELL_AURA_MOD_TARGET_ABSORB_SCHOOL,
            50,
            0x01,
        ),
        (
            91_813,
            wow_data::spell::aura_types::SPELL_AURA_SCHOOL_ABSORB,
            30,
            0x01,
        ),
    ] {
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: amount,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(aura_type),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: aura_type,
                    effect_base_points: amount,
                    effect_misc_value_1: misc_value,
                    effect_misc_value_2: 0,
                    ..Default::default()
                }],
            },
        );
    }
    // The protected shield carries `SPELL_ATTR6_ABSORB_CANNOT_BE_IGNORE`.
    let mut attributes = [0_u32; 15];
    attributes[6] = wow_data::spell::attributes::SPELL_ATTR6_ABSORB_CANNOT_BE_IGNORE;
    spell_store.insert_spell_misc_attributes_like_cpp(91_813, attributes);
    let spell_store = Arc::new(spell_store);
    session.set_spell_store(Arc::clone(&spell_store));
    let config = wow_world::session::LegacyCreatureAggroConfigLikeCpp {
        spell_store: Some(Arc::clone(&spell_store)),
        ..Default::default()
    };
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);
    session
        .apply_aura(91_810, player, 30_000, 1)
        .expect("apply hit-chance aura");

    let reset_swing = |session: &mut WorldSession| {
        session
            .fixture_melee_mutate_creature(creature_guid, |creature| {
                creature.creature.ai_ownership_mut().last_swing_ms = 0;
                creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            })
            .unwrap();
    };
    let victim_health = || {
        canonical
            .lock()
            .unwrap()
            .find_map(0, 0)
            .unwrap()
            .map()
            .get_typed_player(player)
            .unwrap()
            .unit()
            .data()
            .health
    };

    // A 30-point shield without the attribute may only take the half the
    // attacker's 50% modifier leaves: 5 of the 10-point hit.
    session
        .apply_aura(91_811, player, 30_000, 1)
        .expect("apply absorb shield");
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = outcome.commands.last().expect("command").clone();
    assert_eq!(command.absorbed, 5);
    assert_eq!(command.damage, 5);
    assert_eq!(
        command.hit_info,
        HIT_INFO_AFFECTS_VICTIM | HIT_INFO_PARTIAL_ABSORB
    );
    assert_eq!(victim_health(), 95);

    // The shield whose spell carries
    // `SPELL_ATTR6_ABSORB_CANNOT_BE_IGNORE` ignores the modifier and absorbs the
    // whole hit.
    session
        .apply_aura(91_813, player, 30_000, 1)
        .expect("apply protected absorb shield");
    reset_swing(&mut session);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = outcome.commands.last().expect("command").clone();
    assert_eq!(command.absorbed, 10);
    assert_eq!(command.damage, 0);
    assert_eq!(victim_health(), 95);
}
