use super::*;

/// C++ `Unit::CalcArmorReducedDamage` for a creature attacker against a player
/// victim, through the production ownership path.
///
/// C++ `CalculateMeleeDamage` (`Unit.cpp:1326-1343`) mitigates through
/// `CalcArmorReducedDamage` before the outcome switch, reading the victim's
/// `GetArmor()` and the attacker's normal-school `MOD_TARGET_RESISTANCE`, while
/// the victim's `SPELL_AURA_BYPASS_ARMOR_FOR_CASTER` shrinks its own armour for
/// effects the attacker cast (`Unit.cpp:1631-1637`).
#[test]
fn legacy_creature_melee_tick_once_applies_player_victim_armor_like_cpp() {
    use wow_packet::packets::combat::HIT_INFO_AFFECTS_VICTIM;
    use wow_world::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);

    let player = ObjectGuid::create_player(1, 91_160);
    let foreign_caster = ObjectGuid::create_player(1, 91_161);
    let creature_guid = test_creature_guid(91_162);

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
        "ArmoredVictim".to_string(),
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
            stats.armor = 5_000;
            stats.dodge_pct = 0.0;
            stats.parry_pct = 0.0;
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
            // C++ `CalcArmorReducedDamage`'s `levelModifier` is the attacker's
            // level, so the fixture creature is a level-80 attacker.
            creature.creature.unit_mut().set_level(80);
            // The fixture asserts exact mitigated damage, so the flat 5% crit
            // stays off.
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
        91_170,
        wow_data::SpellInfo {
            spell_id: 91_170,
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
    spell_store.insert(
        91_171,
        wow_data::SpellInfo {
            spell_id: 91_171,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 50,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_BYPASS_ARMOR_FOR_CASTER),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_BYPASS_ARMOR_FOR_CASTER,
                effect_base_points: 50,
                ..Default::default()
            }],
        },
    );
    let spell_store = Arc::new(spell_store);
    session.set_spell_store(Arc::clone(&spell_store));
    let config = wow_world::session::LegacyCreatureAggroConfigLikeCpp {
        spell_store: Some(spell_store),
        ..Default::default()
    };
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let victim_health = |canonical: &SharedCanonicalMapManager| {
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
    let reset_swing = |session: &mut WorldSession| {
        session
            .fixture_melee_mutate_creature(creature_guid, |creature| {
                creature.creature.ai_ownership_mut().last_swing_ms = 0;
                creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            })
            .unwrap();
    };
    let swing = |manager: &wow_world::map_manager::SharedMapManager, session: &mut WorldSession| {
        reset_swing(session);
        let outcome =
            run_legacy_creature_melee_tick_once_like_cpp(manager, Some(&canonical), &config);
        outcome.commands.last().expect("one swing command").clone()
    };

    // The `+5` victim aura zeroes the flat 5.0 miss band.
    session
        .apply_aura(91_170, player, 30_000, 1)
        .expect("apply hit-chance aura");

    // 5,000 armour at level 80: `ceil(10 * (1 - 0.247127)) = 8`.
    let command = swing(&manager, &mut session);
    assert_eq!(command.damage, 8);
    assert_eq!(command.original_damage, 8);
    assert_eq!(command.hit_info, HIT_INFO_AFFECTS_VICTIM);
    assert_eq!(victim_health(&canonical), 92);

    // A foreign caster's bypass aura is ignored (`GetCasterGUID() == attacker`).
    session
        .apply_aura(91_171, foreign_caster, 30_000, 1)
        .expect("apply foreign bypass aura");
    let command = swing(&manager, &mut session);
    assert_eq!(command.damage, 8);
    assert_eq!(victim_health(&canonical), 84);

    // The attacker's own 50% bypass halves the armour: `ceil(10 * 0.859017) = 9`.
    session
        .apply_aura(91_171, creature_guid, 30_000, 1)
        .expect("apply attacker bypass aura");
    let command = swing(&manager, &mut session);
    assert_eq!(command.damage, 9);
    assert_eq!(victim_health(&canonical), 75);

    // Both bypass effects sum, removing the armour: the full 10 lands.
    let second_bypass = session
        .fixture_melee_player_snapshot(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .runtime_applications_like_cpp()
                .iter()
                .filter(|(_, aura)| aura.spell_id == 91_171 && aura.caster_guid == foreign_caster)
                .map(|(slot, _)| *slot)
                .next()
        })
        .flatten()
        .expect("foreign bypass slot");
    session
        .remove_aura(second_bypass)
        .expect("remove foreign bypass aura");
    session
        .apply_aura(91_171, creature_guid, 30_000, 1)
        .expect("apply second attacker bypass aura");
    let command = swing(&manager, &mut session);
    assert_eq!(command.damage, 10);
    assert_eq!(command.original_damage, 10);
    assert_eq!(victim_health(&canonical), 65);
}
