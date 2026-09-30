use super::*;

/// C++ `Player::GetBlockPercent`'s armour constant comes from the DB2
/// `ExpectedStat` table (`DB2Stores.cpp:2103-2173`), and the represented
/// runtime reads it through the aggro config's store handle.
#[test]
fn legacy_creature_melee_tick_once_reads_the_expected_stat_store_like_cpp() {
    use wow_world::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    let player = ObjectGuid::create_player(1, 91_300);
    let creature_guid = test_creature_guid(91_301);

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
        "BlockDB2".to_string(),
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
            player.unit_mut().set_max_health(100_000);
            player.unit_mut().set_health(100_000);
            let mut stats = *player.effective_combat_stats_like_cpp();
            stats.armor = 0;
            stats.dodge_pct = 0.0;
            stats.parry_pct = 0.0;
            stats.block_pct = 100.0;
            stats.shield_block = 2_000;
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
            creature.creature.unit_mut().set_level(80);
            creature.creature.ai_ownership_mut().min_damage = 10_000;
            creature.creature.ai_ownership_mut().max_damage = 10_000;
            creature.creature.set_flags_extra_runtime_like_cpp(
                wow_constants::CreatureFlagsExtra::NO_CRIT.bits(),
            );
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        91_310,
        wow_data::SpellInfo {
            spell_id: 91_310,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 5,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
                effect_base_points: 5,
                ..Default::default()
            }],
        },
    );
    let spell_store = Arc::new(spell_store);
    session.set_spell_store(Arc::clone(&spell_store));
    session
        .apply_aura(91_310, player, 30_000, 1)
        .expect("apply hit-chance aura");
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let expected_stat = |armor_constant: f32| {
        wow_data::ExpectedStatStore::from_entries([wow_data::ExpectedStatEntry {
            id: 0,
            expansion_id: -2,
            creature_health: 0.0,
            player_health: 0.0,
            creature_auto_attack_dps: 0.0,
            creature_armor: 0.0,
            player_mana: 0.0,
            player_primary_stat: 0.0,
            player_secondary_stat: 0.0,
            armor_constant,
            creature_spell_damage: 0.0,
            lvl: 80,
        }])
    };
    let tick = |session: &mut WorldSession,
                expected_stat_store: Option<Arc<wow_data::ExpectedStatStore>>| {
        session
            .fixture_melee_mutate_creature(creature_guid, |creature| {
                creature.creature.ai_ownership_mut().last_swing_ms = 0;
                creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            })
            .unwrap();
        let config = wow_world::session::LegacyCreatureAggroConfigLikeCpp {
            spell_store: Some(Arc::clone(&spell_store)),
            expected_stat_store,
            ..Default::default()
        };
        run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config)
    };

    // No store: C++'s empty-store `EvaluateExpectedStat` fallback (`1.0`) gives
    // `min(2000 / 2001, 0.85) = 0.85`, so `10000 * 0.85 / 100 = 85` blocked.
    let outcome = tick(&mut session, None);
    assert_eq!(
        outcome.commands.last().expect("command").damage,
        10_000 - 85
    );

    // A level-80 row with `ArmorConstant = 8000` gives `2000 / 10000 = 0.2`,
    // so only `10000 * 0.2 / 100 = 20` is blocked.
    let outcome = tick(&mut session, Some(Arc::new(expected_stat(8_000.0))));
    assert_eq!(
        outcome.commands.last().expect("command").damage,
        10_000 - 20
    );
}
