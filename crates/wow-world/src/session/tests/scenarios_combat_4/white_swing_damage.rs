use super::*;

#[test]
fn white_swing_roll_bounds_follow_calculate_damage_like_cpp() {
    // C++ `Unit::CalculateDamage` clamps both bounds at zero, orders them and
    // truncates to `uint32` before the roll (`Unit.cpp:2426-2435`).
    assert_eq!(crate::session_rules::white_swing_roll_like_cpp(0.0, 0.0), 0);
    assert_eq!(
        crate::session_rules::white_swing_roll_like_cpp(-4.0, -2.0),
        0
    );
    // An inverted, fractional range is ordered and truncated: `urand(5, 9)`.
    for _ in 0..64 {
        let rolled = crate::session_rules::white_swing_roll_like_cpp(9.9, 5.2);
        assert!((5..=9).contains(&rolled), "rolled {rolled} outside [5, 9]");
    }
}

#[test]
fn white_swing_damage_rolls_the_published_range_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_033);
    let player = ObjectGuid::create_player(1, 87);

    canonical.lock().unwrap().create_world_map(0, 0);
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
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "Roll".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_attacking(Some(guid));
            unit.set_target(guid);
            unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
            unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            unit.set_attack_timer(WeaponAttackType::BaseAttack, 0);
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 5.0, 9.0);
        })
        .unwrap();
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    let swing = |session: &mut WorldSession| {
        session
            .mutate_canonical_player_like_cpp(|player| {
                player
                    .unit_mut()
                    .set_attack_timer(WeaponAttackType::BaseAttack, 0);
                take_canonical_player_attack_swings_like_cpp(
                    player,
                    0,
                    true,
                    true,
                    true,
                    [RepresentedMeleeDamageBonusLikeCpp::NONE; 2],
                    crate::session::combat::RepresentedArmorMitigationLikeCpp::NONE,
                    Default::default(),
                    crate::session_rules::RepresentedMeleeDamageTakenLikeCpp::NONE,
                )
            })
            .flatten()
            .map(|(swings, _)| {
                swings
                    .into_iter()
                    .map(|swing| swing.damage)
                    .collect::<Vec<_>>()
            })
    };

    // Every landed white swing rolls the published `UnitData` range. The range
    // is small enough that 400 draws must reach both bounds; with no bonus the
    // damage is exactly the roll.
    let mut seen = [0_usize; 10];
    for _ in 0..400 {
        let swings = swing(&mut session).expect("white swing resolves");
        assert_eq!(swings.len(), 1);
        let damage = swings[0];
        assert!(
            (5..=9).contains(&damage),
            "damage {damage} outside the published [5, 9] range"
        );
        seen[damage as usize] += 1;
    }
    assert!(seen[5] > 0, "the lower bound must be reachable: {seen:?}");
    assert!(seen[9] > 0, "the upper bound must be reachable: {seen:?}");
    assert!(
        seen.iter().filter(|count| **count > 0).count() >= 4,
        "the roll must spread over the range, not pin one value: {seen:?}"
    );
}

#[test]
fn armor_reduction_matches_calc_armor_reduced_damage_like_cpp() {
    use crate::session_rules::armor_reduced_damage_like_cpp as reduced;
    // C++ `Unit::CalcArmorReducedDamage` (`Unit.cpp:1623-1685`) with a level-80
    // attacker: `levelModifier = 80 + 4.5 * 21 = 174.5`. Rows are
    // `(damage, attacker level, victim level, armour, CR_ARMOR_PENETRATION %,
    //   MOD_TARGET_RESISTANCE sum, MOD_IGNORE_TARGET_RESIST %, BYPASS_ARMOR %, expected)`.
    let cases: &[(u32, u8, u8, i32, f32, i32, f32, f32, u32)] = &[
        // No armour.
        (1_000, 80, 80, 0, 0.0, 0, 0.0, 0.0, 1_000),
        // 5,000 armour: `ceil(1000 * (1 - 0.247127))`.
        (1_000, 80, 80, 5_000, 0.0, 0, 0.0, 0.0, 753),
        // A 25% CR_ARMOR_PENETRATION rating ignores a quarter of the armour.
        (1_000, 80, 80, 5_000, 25.0, 0, 0.0, 0.0, 803),
        // 100% of that rating removes the whole armour value.
        (1_000, 80, 80, 5_000, 100.0, 0, 0.0, 0.0, 1_000),
        // The reduction clamps at 75%.
        (1_000, 80, 80, 10_000_000, 0.0, 0, 0.0, 0.0, 250),
        // A negative MOD_TARGET_RESISTANCE sum cancels the armour.
        (1_000, 80, 80, 5_000, 0.0, -5_000, 0.0, 0.0, 1_000),
        // A victim below level 60 uses `maxArmorPen = 400 + 85 * level`.
        (1_000, 80, 10, 500, 0.0, 0, 0.0, 0.0, 969),
        // `std::floor(AddPct(armor, -amount))` per ignore-resist effect.
        (1_000, 80, 80, 5_000, 0.0, 0, 50.0, 0.0, 860),
        (1_000, 80, 80, 5_000, 0.0, 0, 100.0, 0.0, 1_000),
        (1_000, 80, 80, 5_000, 0.0, 0, -50.0, 0.0, 671),
        // `CalculatePct(armor, 100 - min(bypass, 100))` for
        // `SPELL_AURA_BYPASS_ARMOR_FOR_CASTER`, applied before the
        // target-resistance sum (884 if applied after).
        (1_000, 80, 80, 5_000, 0.0, 0, 0.0, 50.0, 860),
        (1_000, 80, 80, 5_000, 0.0, 0, 0.0, 100.0, 1_000),
        (1_000, 80, 80, 5_000, 0.0, 0, 0.0, 150.0, 1_000),
        (1_000, 80, 80, 5_000, 0.0, -1_000, 0.0, 50.0, 911),
        (1_000, 80, 80, 5_000, 0.0, 0, 0.0, -50.0, 671),
    ];
    for &(damage, attacker, victim, armor, pen, target_resist, ignore, bypass, expected) in cases {
        assert_eq!(
            reduced(
                damage,
                attacker,
                victim,
                armor,
                pen,
                target_resist,
                ignore,
                bypass
            ),
            expected,
            "armour {armor}, pen {pen}, targetResist {target_resist}, ignore {ignore}, bypass {bypass}"
        );
    }
}

#[test]
fn white_swing_applies_victim_armor_mitigation_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_034);
    let player = ObjectGuid::create_player(1, 88);

    canonical.lock().unwrap().create_world_map(0, 0);
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
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "Armor".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_attacking(Some(guid));
            unit.set_target(guid);
            unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
            unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            unit.set_attack_timer(WeaponAttackType::BaseAttack, 0);
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 1_000.0, 1_000.0);
        })
        .unwrap();
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            // C++ `Creature::UpdateLevelDependantStats` seeds
            // `UNIT_MOD_ARMOR` from `CreatureBaseStats::GenerateArmor`.
            creature.creature.unit_mut().set_level(80);
            creature.creature.set_combat_log_stats_like_cpp(
                wow_entities::CreatureCombatLogStatsLikeCpp {
                    armor: 5_000,
                    ..Default::default()
                },
            );
        })
        .unwrap();

    let armor_before = session
        .mutate_world_creature(guid, |creature| {
            creature.creature.combat_log_stats_like_cpp().armor
        })
        .unwrap();
    assert_eq!(armor_before, 5_000, "fixture armour must persist");

    let mitigation = session.represented_melee_armor_mitigation_like_cpp();
    assert_eq!(mitigation.attacker_level, 80);
    assert_eq!(mitigation.victim_level, 80);
    assert_eq!(mitigation.victim_armor, 5_000);
    assert_eq!(mitigation.armor_penetration_pct, 0.0);
    assert_eq!(mitigation.target_resistance_normal_aura, 0);
    assert_eq!(mitigation.ignore_target_resist_normal_pct, 0.0);

    let swing = |session: &mut WorldSession| {
        let melee_damage_bonus = session.represented_melee_damage_bonus_like_cpp();
        let armor_mitigation = session.represented_melee_armor_mitigation_like_cpp();
        session
            .mutate_canonical_player_like_cpp(|player| {
                player
                    .unit_mut()
                    .set_attack_timer(WeaponAttackType::BaseAttack, 0);
                take_canonical_player_attack_swings_like_cpp(
                    player,
                    0,
                    true,
                    true,
                    true,
                    melee_damage_bonus,
                    armor_mitigation,
                    Default::default(),
                    crate::session_rules::RepresentedMeleeDamageTakenLikeCpp::NONE,
                )
            })
            .flatten()
            .map(|(swings, _)| {
                swings
                    .into_iter()
                    .map(|swing| swing.damage)
                    .collect::<Vec<_>>()
            })
    };

    assert_eq!(
        swing(&mut session),
        Some(vec![753]),
        "C++ CalcArmorReducedDamage over a 1,000 damage roll"
    );

    // The attacker's `SPELL_AURA_MOD_TARGET_RESISTANCE` and
    // `SPELL_AURA_MOD_IGNORE_TARGET_RESIST` terms only cover
    // `SPELL_SCHOOL_MASK_NORMAL`; a non-normal row leaves the armour alone.
    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, misc_value, amount) in [
        (
            91_120_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_TARGET_RESISTANCE,
            0x02_i32,
            -5_000_i32,
        ),
        (
            91_121,
            wow_data::spell::aura_types::SPELL_AURA_MOD_TARGET_RESISTANCE,
            0x01,
            -5_000,
        ),
        (
            91_122,
            wow_data::spell::aura_types::SPELL_AURA_MOD_IGNORE_TARGET_RESIST,
            0x04,
            50,
        ),
        (
            91_123,
            wow_data::spell::aura_types::SPELL_AURA_MOD_IGNORE_TARGET_RESIST,
            0x01,
            50,
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
                    effect_misc_value_1: misc_value,
                    effect_base_points: amount,
                    ..Default::default()
                }],
            },
        );
    }
    session.set_spell_store(Arc::new(spell_store));

    // A non-normal ignore-resist row is filtered out.
    session
        .apply_aura(91_122, player, 30_000, 1)
        .expect("apply non-normal ignore-resist aura");
    assert_eq!(
        session
            .represented_melee_armor_mitigation_like_cpp()
            .ignore_target_resist_normal_pct,
        0.0
    );
    assert_eq!(swing(&mut session), Some(vec![753]));

    // A normal-school sum shrinks the armour with
    // `std::floor(AddPct(armor, -amount))` before the reduction curve.
    session
        .apply_aura(91_123, player, 30_000, 1)
        .expect("apply normal ignore-resist aura");
    assert_eq!(
        session
            .represented_melee_armor_mitigation_like_cpp()
            .ignore_target_resist_normal_pct,
        50.0
    );
    assert_eq!(swing(&mut session), Some(vec![860]));

    // A non-normal `MOD_TARGET_RESISTANCE` row leaves the reduced armour alone.
    session
        .apply_aura(91_120, player, 30_000, 1)
        .expect("apply non-normal target-resistance aura");
    assert_eq!(swing(&mut session), Some(vec![860]));

    // A normal-school negative target-resistance sum is armour penetration and
    // cancels the armour, so the ignore-resist term has nothing left to reduce.
    session
        .apply_aura(91_121, player, 30_000, 1)
        .expect("apply normal target-resistance aura");
    assert_eq!(swing(&mut session), Some(vec![1_000]));
}
