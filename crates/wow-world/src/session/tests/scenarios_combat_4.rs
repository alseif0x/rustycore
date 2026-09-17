//! Session scenarios exercising the represented melee damage math (#29).
//!
//! Split out of `scenarios_combat_1.rs`; the shared fixtures stay in the parent
//! module.

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

#[test]
fn melee_attack_table_matches_roll_melee_outcome_against_like_cpp() {
    use crate::session_rules::{
        RepresentedMeleeOutcomeInputsLikeCpp as Inputs, RepresentedMeleeOutcomeLikeCpp as Outcome,
        melee_outcome_like_cpp,
    };

    // 1/10000-unit bands with miss 5%, dodge 3%, parry 6%, glancing 10% and
    // crit 20%: miss [0,500), dodge [500,800), parry [800,1400),
    // glancing [1400,2400), crit [2400,4400), hit [4400,10000).
    let inputs = Inputs {
        miss_chance_pct: 5.0,
        dodge_chance_pct: 3.0,
        parry_chance_pct: 6.0,
        block_chance_pct: 5.0,
        glancing_chance_pct: 10.0,
        crit_chance_pct: 20.0,
        can_dodge: true,
        can_parry: true,
        is_evading_attacks: false,
        always_crits: false,
        is_immune_to_damage: false,
    };
    for (roll, expected) in [
        (0, Outcome::Miss),
        (499, Outcome::Miss),
        (500, Outcome::Dodge),
        (799, Outcome::Dodge),
        (800, Outcome::Parry),
        (1_399, Outcome::Parry),
        (1_400, Outcome::Glancing),
        (2_399, Outcome::Glancing),
        (2_400, Outcome::Block),
        (2_899, Outcome::Block),
        (2_900, Outcome::Crit),
        (4_899, Outcome::Crit),
        (4_900, Outcome::Hit),
        (9_999, Outcome::Hit),
    ] {
        assert_eq!(
            melee_outcome_like_cpp(&inputs, roll),
            expected,
            "roll {roll}"
        );
    }

    // A gated band is skipped without consuming probability: with no dodge the
    // parry band starts where the miss band ends.
    let no_dodge = Inputs {
        can_dodge: false,
        ..inputs
    };
    assert_eq!(melee_outcome_like_cpp(&no_dodge, 500), Outcome::Parry);
    let no_parry = Inputs {
        can_parry: false,
        ..inputs
    };
    assert_eq!(melee_outcome_like_cpp(&no_parry, 800), Outcome::Glancing);

    // An evading victim short-circuits every band.
    let evading = Inputs {
        is_evading_attacks: true,
        ..inputs
    };
    assert_eq!(melee_outcome_like_cpp(&evading, 0), Outcome::Evade);
    assert_eq!(melee_outcome_like_cpp(&evading, 9_999), Outcome::Evade);

    // C++ truncates each percentage with `int32(chance * 100.0f)`.
    let truncated = Inputs {
        miss_chance_pct: 7.999,
        ..Inputs::default()
    };
    assert_eq!(melee_outcome_like_cpp(&truncated, 798), Outcome::Miss);
    assert_eq!(melee_outcome_like_cpp(&truncated, 799), Outcome::Hit);
}

#[test]
fn melee_attack_table_outcome_effects_match_calculate_melee_damage_like_cpp() {
    use crate::session_rules::{
        RepresentedMeleeOutcomeLikeCpp as Outcome, melee_outcome_damage_like_cpp,
        melee_outcome_presentation_like_cpp,
    };
    use wow_packet::packets::combat::{
        HIT_INFO_AFFECTS_VICTIM, HIT_INFO_BLOCK, HIT_INFO_CRITICAL_HIT, HIT_INFO_GLANCING,
        HIT_INFO_MISS, HIT_INFO_OFFHAND, HIT_INFO_SWING_NO_HIT_SOUND, VICTIM_STATE_DODGE,
        VICTIM_STATE_EVADES, VICTIM_STATE_HIT, VICTIM_STATE_INTACT, VICTIM_STATE_PARRY,
    };

    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Evade, 100, 80, 80, 1.0, 30.0),
        (0, 0, 100)
    );
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Miss, 100, 80, 80, 1.0, 30.0),
        (0, 0, 100)
    );
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Dodge, 100, 80, 80, 1.0, 30.0),
        (0, 0, 100)
    );
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Parry, 100, 80, 80, 1.0, 30.0),
        (0, 0, 100)
    );
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Hit, 100, 80, 80, 1.0, 30.0),
        (100, 0, 100)
    );
    assert_eq!(
        // C++ assigns `OriginalDamage` after the doubling, so a critical swing
        // publishes the doubled value as its original too (`Unit.cpp:1370-1378`).
        melee_outcome_damage_like_cpp(Outcome::Crit, 100, 80, 80, 1.0, 30.0),
        (200, 0, 200)
    );
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Crit, 100, 80, 80, 2.0, 30.0),
        (400, 0, 400)
    );
    // C++ `CalculatePct(damage, GetBlockPercent)` with the flat 30% creature
    // base (`Unit.h:947`).
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Block, 100, 80, 80, 1.0, 30.0),
        (70, 30, 100)
    );
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Block, 7, 80, 80, 1.0, 30.0),
        (5, 2, 7)
    );
    // C++ `leveldif = min(victimLevel - attackerLevel, 3)` then
    // `reducePercent = 1 - leveldif * 0.1`.
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Glancing, 100, 80, 81, 1.0, 30.0),
        (90, 0, 100)
    );
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Glancing, 100, 80, 84, 1.0, 30.0),
        (70, 0, 100)
    );
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Glancing, 100, 80, 90, 1.0, 30.0),
        (70, 0, 100)
    );

    // C++ returns before any band on a physical-immune victim, publishing a
    // zero `HITINFO_NORMALSWING` with `VICTIMSTATE_IS_IMMUNE`.
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Immune, 100, 80, 80, 1.0, 30.0),
        (0, 0, 100)
    );
    assert_eq!(
        melee_outcome_presentation_like_cpp(Outcome::Immune, false),
        (0, wow_packet::packets::combat::VICTIM_STATE_IS_IMMUNE)
    );

    assert_eq!(
        melee_outcome_presentation_like_cpp(Outcome::Evade, false),
        (
            HIT_INFO_MISS | HIT_INFO_SWING_NO_HIT_SOUND,
            VICTIM_STATE_EVADES
        )
    );
    assert_eq!(
        melee_outcome_presentation_like_cpp(Outcome::Miss, false),
        (HIT_INFO_MISS, VICTIM_STATE_INTACT)
    );
    assert_eq!(
        melee_outcome_presentation_like_cpp(Outcome::Dodge, false),
        (HIT_INFO_AFFECTS_VICTIM, VICTIM_STATE_DODGE)
    );
    assert_eq!(
        melee_outcome_presentation_like_cpp(Outcome::Parry, false),
        (HIT_INFO_AFFECTS_VICTIM, VICTIM_STATE_PARRY)
    );
    assert_eq!(
        melee_outcome_presentation_like_cpp(Outcome::Glancing, false),
        (
            HIT_INFO_AFFECTS_VICTIM | HIT_INFO_GLANCING,
            VICTIM_STATE_HIT
        )
    );
    assert_eq!(
        melee_outcome_presentation_like_cpp(Outcome::Block, false),
        (HIT_INFO_AFFECTS_VICTIM | HIT_INFO_BLOCK, VICTIM_STATE_HIT)
    );
    assert_eq!(
        melee_outcome_presentation_like_cpp(Outcome::Crit, false),
        (
            HIT_INFO_AFFECTS_VICTIM | HIT_INFO_CRITICAL_HIT,
            VICTIM_STATE_HIT
        )
    );
    assert_eq!(
        melee_outcome_presentation_like_cpp(Outcome::Hit, false),
        (HIT_INFO_AFFECTS_VICTIM, VICTIM_STATE_HIT)
    );
    // C++ sets `HITINFO_OFFHAND` before the table for `OFF_ATTACK`.
    assert_eq!(
        melee_outcome_presentation_like_cpp(Outcome::Hit, true),
        (HIT_INFO_AFFECTS_VICTIM | HIT_INFO_OFFHAND, VICTIM_STATE_HIT)
    );
}

#[test]
fn melee_attack_table_inputs_resolve_cpp_chances_like_cpp() {
    use crate::session_rules::{
        RepresentedMeleeAttackerFactsLikeCpp as Attacker,
        RepresentedMeleeOutcomeLikeCpp as Outcome, RepresentedMeleeVictimFactsLikeCpp as Victim,
        melee_outcome_inputs_like_cpp, melee_outcome_like_cpp,
    };

    let attacker = Attacker {
        level: 80,
        crit_damage_multiplier: 1.0,
        dual_wielding: false,
        ignores_dual_wield_hit_penalty: false,
        melee_hit_chance_pct: 7.5,
        hit_chance_aura_pct: 0.0,
        crit_pct: [10.0, 5.0],
        autoattack_crit_aura_pct: 2.0,
        expertise_reduction_pct: [0.5, 0.25],
        dodge_reduction_pct: 0.0,
    };
    let creature = Victim {
        level: 80,
        is_creature: true,
        is_player: false,
        is_totem: false,
        is_evading_attacks: false,
        dodge_pct: 3.0,
        parry_pct: 6.0,
        block_pct: 3.0,
        dodge_aura_pct: 0.0,
        parry_aura_pct: 0.0,
        block_aura_pct: 0.0,
        attacker_melee_hit_chance_pct: 0.0,
        attacker_melee_crit_chance_pct: 0.0,
        crit_chance_vs_target_health_pct: 0.0,
        crit_chance_for_caster_pct: 0.0,
        faces_attacker: true,
        is_controlled: false,
        is_stand_state: true,
        is_immune_to_damage: false,
    };
    let inputs = melee_outcome_inputs_like_cpp(&attacker, &creature);
    // C++ `MeleeSpellMissChance`: 5.0 + 0 (two-hander) - 7.5 -> clamped to 0.
    assert_eq!(inputs[0].miss_chance_pct, 0.0);
    assert_eq!(inputs[0].dodge_chance_pct, 2.5);
    assert_eq!(inputs[0].parry_chance_pct, 5.5);
    assert_eq!(inputs[0].glancing_chance_pct, 0.0);
    assert_eq!(inputs[0].crit_chance_pct, 12.0);
    assert_eq!(inputs[0].block_chance_pct, 3.0);
    assert_eq!(inputs[1].dodge_chance_pct, 2.75);
    assert_eq!(inputs[1].parry_chance_pct, 5.75);
    assert_eq!(inputs[1].block_chance_pct, 3.0);
    assert_eq!(inputs[1].crit_chance_pct, 7.0);
    assert!(inputs[0].can_dodge && inputs[0].can_parry);

    // C++ adds +19% miss while dual wielding and 1.5% per victim level above
    // the attacker; glancing needs four levels of difference.
    let dual_wielding = Attacker {
        dual_wielding: true,
        ..attacker
    };
    // `SPELL_AURA_IGNORE_DUAL_WIELD_HIT_PENALTY` removes the +19% penalty while
    // the flag is present, regardless of amount.
    let ignoring = Attacker {
        ignores_dual_wield_hit_penalty: true,
        ..dual_wielding
    };
    let inputs = melee_outcome_inputs_like_cpp(&ignoring, &creature);
    assert_eq!(inputs[0].miss_chance_pct, 0.0);
    let higher = Victim {
        level: 84,
        ..creature
    };
    let inputs = melee_outcome_inputs_like_cpp(&dual_wielding, &higher);
    assert_eq!(inputs[0].miss_chance_pct, 16.5);
    assert_eq!(inputs[0].dodge_chance_pct, 8.5);
    assert_eq!(inputs[0].parry_chance_pct, 11.5);
    // C++ `GetUnitBlockChance` has no expertise reduction, so only the
    // victim-level bonus applies.
    assert_eq!(inputs[0].block_chance_pct, 9.0);
    assert_eq!(inputs[0].glancing_chance_pct, 50.0);

    // The health-conditioned and for-caster crit auras also add to the band.
    // The victim's percentage auras and the attacker's dodge reductions feed the
    // same bands: `GetUnitDodgeChance` adds `MOD_DODGE_PERCENT` and the
    // attacker's combat-result/enemy-dodge sums, `GetUnitBlockChance` adds
    // `MOD_BLOCK_PERCENT`, the miss band subtracts
    // `MOD_ATTACKER_MELEE_HIT_CHANCE` and the crit band adds
    // `MOD_ATTACKER_MELEE_CRIT_CHANCE`.
    let with_auras = Victim {
        dodge_aura_pct: 100.0,
        parry_aura_pct: -4.0,
        block_aura_pct: 2.0,
        attacker_melee_hit_chance_pct: 5.0,
        attacker_melee_crit_chance_pct: 3.0,
        crit_chance_vs_target_health_pct: 4.0,
        crit_chance_for_caster_pct: 6.0,
        ..creature
    };
    let attacker = Attacker {
        dodge_reduction_pct: -1.0,
        ..attacker
    };
    let inputs = melee_outcome_inputs_like_cpp(&attacker, &with_auras);
    assert_eq!(inputs[0].miss_chance_pct, 0.0);
    assert_eq!(inputs[0].dodge_chance_pct, 101.5);
    assert_eq!(inputs[0].parry_chance_pct, 1.5);
    assert_eq!(inputs[0].block_chance_pct, 5.0);
    assert_eq!(inputs[0].crit_chance_pct, 25.0);

    // C++ returns `MELEE_HIT_EVADE` before rolling for an evading creature.
    let evading = Victim {
        is_evading_attacks: true,
        ..creature
    };
    let inputs = melee_outcome_inputs_like_cpp(&attacker, &evading);
    assert!(inputs[0].is_evading_attacks);

    // A controlled victim can neither dodge nor parry/block.
    let controlled = Victim {
        is_controlled: true,
        ..creature
    };
    let inputs = melee_outcome_inputs_like_cpp(&attacker, &controlled);
    assert!(!inputs[0].can_dodge && !inputs[0].can_parry);

    // A totem has no dodge, parry or block.
    let totem = Victim {
        is_totem: true,
        ..higher
    };
    let inputs = melee_outcome_inputs_like_cpp(&attacker, &totem);
    assert_eq!(inputs[0].dodge_chance_pct, 0.0);
    assert_eq!(inputs[0].parry_chance_pct, 0.0);
    assert_eq!(inputs[0].block_chance_pct, 0.0);

    // A player victim resolves the miss, dodge, parry and crit bands from its
    // published `DodgePercentage`/`ParryPercentage`; `canDodge`/`canParryOrBlock`
    // need the victim to face the attacker and to be uncontrolled, glancing
    // never applies to a creature attacker, and the block band is left out
    // because the blocked damage needs C++ `Player::GetBlockPercent`'s DB2
    // `ExpectedStatType::ArmorConstant` table.
    let player_victim = Victim {
        is_creature: false,
        is_player: true,
        attacker_melee_hit_chance_pct: 2.5,
        dodge_pct: 20.0,
        parry_pct: 15.0,
        block_pct: 10.0,
        ..creature
    };
    let two_handed = Attacker {
        melee_hit_chance_pct: 0.0,
        ..attacker
    };
    let inputs = melee_outcome_inputs_like_cpp(&two_handed, &player_victim);
    assert_eq!(inputs[0].miss_chance_pct, 2.5);
    // `20 + (-1.0 attacker dodge reduction) - 0.5 expertise` and `15 - 0.5`.
    assert_eq!(inputs[0].dodge_chance_pct, 18.5);
    assert_eq!(inputs[0].parry_chance_pct, 14.5);
    assert_eq!(inputs[1].dodge_chance_pct, 18.75);
    assert_eq!(inputs[1].parry_chance_pct, 14.75);
    // C++ `GetUnitBlockChance`'s player branch: the published `BlockPercentage`.
    assert_eq!(inputs[0].block_chance_pct, 10.0);
    assert_eq!(inputs[0].glancing_chance_pct, 0.0);
    assert_eq!(inputs[0].crit_chance_pct, 12.0);
    assert!(inputs[0].can_dodge && inputs[0].can_parry);
    assert!(!inputs[0].is_evading_attacks);
    assert!(!inputs[0].always_crits);
    // After the 2.5% miss band: dodge [250, 2100), parry [2100, 3550),
    // block [3550, 4550), crit [4550, 5750), hit [5750, 10000).
    for (roll, expected) in [
        (249, Outcome::Miss),
        (250, Outcome::Dodge),
        (2_099, Outcome::Dodge),
        (2_100, Outcome::Parry),
        (3_549, Outcome::Parry),
        (3_550, Outcome::Block),
        (4_549, Outcome::Block),
        (4_550, Outcome::Crit),
        (5_749, Outcome::Crit),
        (9_999, Outcome::Hit),
    ] {
        assert_eq!(
            melee_outcome_like_cpp(&inputs[0], roll),
            expected,
            "roll {roll}"
        );
    }

    // C++ requires the victim to face the attacker for both gates, and clears
    // them for a controlled victim.
    let behind = Victim {
        faces_attacker: false,
        ..player_victim
    };
    let inputs = melee_outcome_inputs_like_cpp(&two_handed, &behind);
    assert!(!inputs[0].can_dodge && !inputs[0].can_parry);
    assert_eq!(inputs[0].dodge_chance_pct, 18.5, "the band stays published");
    let controlled_player = Victim {
        is_controlled: true,
        ..player_victim
    };
    let inputs = melee_outcome_inputs_like_cpp(&two_handed, &controlled_player);
    assert!(!inputs[0].can_dodge && !inputs[0].can_parry);

    // The block band is gated by the same facing/controlled gate as parry.
    let behind_player = Victim {
        faces_attacker: false,
        ..player_victim
    };
    let inputs = melee_outcome_inputs_like_cpp(&two_handed, &behind_player);
    assert!(!inputs[0].can_parry);
    assert_eq!(inputs[0].block_chance_pct, 10.0);

    // C++ returns `MELEE_HIT_CRIT` before the avoidance bands for a player
    // victim that is not in a stand state, while the critical chance is
    // non-zero (`Unit.cpp:2312-2314`).
    let sitting = Victim {
        is_stand_state: false,
        ..player_victim
    };
    let inputs = melee_outcome_inputs_like_cpp(&two_handed, &sitting);
    assert!(inputs[0].always_crits);
    assert_eq!(melee_outcome_like_cpp(&inputs[0], 250), Outcome::Crit);
    // A zero critical chance keeps the sitting victim on the avoidance bands.
    let no_crit = Attacker {
        crit_pct: [0.0, 0.0],
        autoattack_crit_aura_pct: 0.0,
        ..two_handed
    };
    let inputs = melee_outcome_inputs_like_cpp(&no_crit, &sitting);
    assert!(!inputs[0].always_crits);
    assert_eq!(melee_outcome_like_cpp(&inputs[0], 250), Outcome::Dodge);

    // A victim the owner cannot read keeps the pre-table behaviour.
    let unknown = Victim {
        level: 80,
        ..Default::default()
    };
    let inputs = melee_outcome_inputs_like_cpp(&attacker, &unknown);
    assert_eq!(
        inputs,
        [crate::session_rules::RepresentedMeleeOutcomeInputsLikeCpp::NONE; 2]
    );
}

#[test]
fn white_swing_publishes_the_attack_table_outcome_like_cpp() {
    use crate::session_rules::melee_outcome_inputs_like_cpp;
    use wow_packet::packets::combat::{
        HIT_INFO_AFFECTS_VICTIM, HIT_INFO_MISS, VICTIM_STATE_DODGE, VICTIM_STATE_INTACT,
        VICTIM_STATE_PARRY,
    };

    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_035);
    let player = ObjectGuid::create_player(1, 89);

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
        "Table".to_string(),
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
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 7.0, 7.0);
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
            // This test needs the real table, so it seeds the fixture's
            // creature avoidance and the untrained player's zero hit rating:
            // the C++ 5% miss, 3% dodge and 6% parry bands.
            creature
                .creature
                .set_avoidance_like_cpp(wow_entities::CreatureAvoidanceLikeCpp {
                    dodge_pct: 3.0,
                    parry_pct: 6.0,
                    // Block has its own scenario; this one covers the
                    // damage-zeroing bands.
                    block_pct: 0.0,
                });
        })
        .unwrap();
    session
        .mutate_canonical_player_like_cpp(|player| {
            let mut stats = *player.effective_combat_stats_like_cpp();
            stats.melee_hit_chance_pct = 0.0;
            player.replace_effective_combat_stats_like_cpp(stats);
        })
        .unwrap();

    let swing = |session: &mut WorldSession| {
        let melee_damage_bonus = session.represented_melee_damage_bonus_like_cpp();
        let armor_mitigation = session.represented_melee_armor_mitigation_like_cpp();
        let outcome_facts = session.represented_melee_outcome_facts_like_cpp();
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
                    outcome_facts,
                    crate::session_rules::RepresentedMeleeDamageTakenLikeCpp::NONE,
                )
            })
            .flatten()
            .map(|(swings, _)| swings)
    };

    // The creature is a normal level-2 victim facing the attacker, so the
    // represented table has 3% dodge and 6% parry bands and no miss band
    // (`5.0 - 7.5` clamps to zero). Over 400 landed swings both a full hit and
    // an avoided swing must appear.
    let mut hits = 0;
    let mut avoids = 0;
    for _ in 0..400 {
        let swings = swing(&mut session).expect("white swing resolves");
        assert_eq!(swings.len(), 1);
        if swings[0].damage == 0 {
            avoids += 1;
            assert!(
                swings[0].hit_info & HIT_INFO_MISS != 0
                    || swings[0].victim_state == VICTIM_STATE_DODGE
                    || swings[0].victim_state == VICTIM_STATE_PARRY,
                "an avoided swing must publish an avoid outcome: {:?}",
                swings[0]
            );
        } else {
            hits += 1;
            assert_eq!(swings[0].damage, 7);
            assert_eq!(swings[0].hit_info, HIT_INFO_AFFECTS_VICTIM);
        }
    }
    assert!(hits > 0, "some swings must land");
    assert!(avoids > 0, "the dodge/parry bands must be reachable");

    // A -200% `SPELL_AURA_MOD_HIT_CHANCE` makes the miss band exceed the whole
    // roll, so the swing is guaranteed to miss.
    let spell_id = 91_122_i32;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: -200,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_HIT_CHANCE),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_HIT_CHANCE,
                effect_base_points: -200,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    session
        .apply_aura(spell_id, player, 30_000, 1)
        .expect("apply hit-chance aura");
    let facts = session.represented_melee_outcome_facts_like_cpp();
    assert_eq!(facts.0.hit_chance_aura_pct, -200.0);
    let inputs = melee_outcome_inputs_like_cpp(&facts.0, &facts.1);
    assert!(inputs[0].miss_chance_pct >= 100.0);

    let misses = swing(&mut session).expect("missed swing still resolves");
    assert_eq!(misses[0].damage, 0);
    // C++ keeps the post-armour `OriginalDamage` on an avoided swing.
    assert_eq!(misses[0].original_damage, 7);
    assert_eq!(misses[0].hit_info, HIT_INFO_MISS);
    assert_eq!(misses[0].victim_state, VICTIM_STATE_INTACT);
}

#[test]
fn white_swing_publishes_a_block_like_cpp() {
    use wow_packet::packets::combat::{HIT_INFO_AFFECTS_VICTIM, HIT_INFO_BLOCK, VICTIM_STATE_HIT};

    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_036);
    let player = ObjectGuid::create_player(1, 90);

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
        "Block".to_string(),
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
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 100.0, 100.0);
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
            // Dodge and parry stay zero, so the 100% block band is the first
            // non-empty band after the (fixture-inert) miss band.
            creature
                .creature
                .set_avoidance_like_cpp(wow_entities::CreatureAvoidanceLikeCpp {
                    dodge_pct: 0.0,
                    parry_pct: 0.0,
                    block_pct: 100.0,
                });
        })
        .unwrap();

    let melee_damage_bonus = session.represented_melee_damage_bonus_like_cpp();
    let armor_mitigation = session.represented_melee_armor_mitigation_like_cpp();
    let outcome_facts = session.represented_melee_outcome_facts_like_cpp();
    let swings = session
        .mutate_canonical_player_like_cpp(|player| {
            take_canonical_player_attack_swings_like_cpp(
                player,
                0,
                true,
                true,
                true,
                melee_damage_bonus,
                armor_mitigation,
                outcome_facts,
                crate::session_rules::RepresentedMeleeDamageTakenLikeCpp::NONE,
            )
        })
        .flatten()
        .expect("white swing resolves")
        .0;
    assert_eq!(swings.len(), 1);
    // C++ `CalculatePct(100, GetBlockPercent)`: a flat 30% block.
    assert_eq!(swings[0].blocked, 30);
    assert_eq!(swings[0].damage, 70);
    assert_eq!(swings[0].hit_info, HIT_INFO_AFFECTS_VICTIM | HIT_INFO_BLOCK);
    assert_eq!(swings[0].victim_state, VICTIM_STATE_HIT);
}

#[test]
fn creature_aura_effects_resolve_the_applied_mask_like_cpp() {
    use wow_data::spell::aura_types::{
        SPELL_AURA_MOD_DAMAGE_TAKEN, SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN,
    };

    let caster = ObjectGuid::create_player(1, 91);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        91_130,
        wow_data::SpellInfo {
            spell_id: 91_130,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: -50,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(SPELL_AURA_MOD_DAMAGE_TAKEN),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![
                wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: SPELL_AURA_MOD_DAMAGE_TAKEN,
                    effect_misc_value_1: 0x01,
                    effect_base_points: -50,
                    ..Default::default()
                },
                wow_data::SpellEffectInfo {
                    effect_index: 1,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN,
                    effect_misc_value_1: 0,
                    effect_base_points: -5,
                    ..Default::default()
                },
            ],
        },
    );

    // Only effect 0 is applied, so only its aura type is resolved.
    let effects = crate::session_rules::creature_aura_effects_like_cpp(
        &[wow_entities::AppliedAuraRef::new(91_130, caster, 0, 0b1)],
        &spell_store,
        0,
        None,
    );
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].aura_type, SPELL_AURA_MOD_DAMAGE_TAKEN);
    assert_eq!(effects[0].misc_value, 0x01);
    assert_eq!(effects[0].amount, -50);
    assert_eq!(effects[0].caster_guid, caster);

    // Both effects when the application's mask covers both slots.
    let effects = crate::session_rules::creature_aura_effects_like_cpp(
        &[wow_entities::AppliedAuraRef::new(91_130, caster, 0, 0b11)],
        &spell_store,
        0,
        None,
    );
    assert_eq!(effects.len(), 2);
    assert_eq!(effects[1].aura_type, SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN);
    assert_eq!(effects[1].amount, -5);
}

#[test]
fn melee_damage_taken_matches_cpp_like_cpp() {
    use crate::session_rules::{
        AppliedAuraEffectLikeCpp as Effect, RepresentedMeleeDamageTakenLikeCpp as Taken,
        melee_damage_taken_apply_like_cpp, melee_damage_taken_flat_pct_like_cpp,
    };
    use wow_data::spell::aura_types::{
        SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN, SPELL_AURA_MOD_DAMAGE_TAKEN,
        SPELL_AURA_MOD_MELEE_DAMAGE_FROM_CASTER, SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN,
        SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN_PCT,
    };

    let attacker = ObjectGuid::create_player(1, 92);
    let effect = |aura_type, misc_value, amount, caster_guid| Effect {
        spell_id: 1,
        caster_guid,
        aura_type,
        misc_value,
        misc_value_b: 0,
        amount,
    };

    // A normal-school `MOD_DAMAGE_TAKEN` and a `MOD_MELEE_DAMAGE_TAKEN` sum
    // into the flat benefit; a fire-school row does not cover normal damage.
    let effects = [
        effect(SPELL_AURA_MOD_DAMAGE_TAKEN, 0x01, -20, attacker),
        effect(SPELL_AURA_MOD_DAMAGE_TAKEN, 0x04, -100, attacker),
        effect(SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN, 0, -5, attacker),
    ];
    let taken = melee_damage_taken_flat_pct_like_cpp(&effects, &[], attacker, 0x01);
    assert_eq!(
        taken,
        Taken {
            flat: -25,
            pct: 1.0
        }
    );
    assert_eq!(melee_damage_taken_apply_like_cpp(taken, 100), 75);
    // C++ returns zero before the arithmetic when the flat benefit absorbs the
    // whole hit.
    assert_eq!(melee_damage_taken_apply_like_cpp(taken, 10), 0);

    // Percent terms multiply: school mask, caster-restricted and the melee
    // taken percentage.
    let effects = [
        effect(SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN, 0x01, -50, attacker),
        effect(SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN_PCT, 0, -50, attacker),
        effect(SPELL_AURA_MOD_MELEE_DAMAGE_FROM_CASTER, 0, 100, attacker),
        // Another caster's aura must not apply.
        effect(
            SPELL_AURA_MOD_MELEE_DAMAGE_FROM_CASTER,
            0,
            100,
            ObjectGuid::create_player(1, 93),
        ),
    ];
    let taken = melee_damage_taken_flat_pct_like_cpp(&effects, &[], attacker, 0x01);
    // `0.5 * 0.5 * 2.0`.
    assert_eq!(taken.pct, 0.5);
    assert_eq!(melee_damage_taken_apply_like_cpp(taken, 101), 50);

    // The Sanctified Wrath bypass shrinks the reduction with the attacker's
    // normal-school `SPELL_AURA_MOD_IGNORE_TARGET_RESIST`.
    let taken = melee_damage_taken_flat_pct_like_cpp(
        &[effect(
            SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN,
            0x01,
            -50,
            attacker,
        )],
        &[(0x01, 100)],
        attacker,
        0x01,
    );
    assert_eq!(taken.pct, 1.0);
    // A bypass that does not cover the school leaves the reduction alone.
    let taken = melee_damage_taken_flat_pct_like_cpp(
        &[effect(
            SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN,
            0x01,
            -50,
            attacker,
        )],
        &[(0x04, 100)],
        attacker,
        0x01,
    );
    assert_eq!(taken.pct, 0.5);
}

#[test]
fn white_swing_applies_victim_melee_damage_taken_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_037);
    let player = ObjectGuid::create_player(1, 94);

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
        "Taken".to_string(),
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
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 100.0, 100.0);
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

    // The victim's `SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN` halves the swing and the
    // attacker's `SPELL_AURA_MOD_IGNORE_TARGET_RESIST` bypasses a
    // `SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN` reduction.
    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura, misc, amount) in [
        (
            91_131_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN,
            0,
            -50,
        ),
        (
            91_132,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN,
            0x01,
            -50,
        ),
        (
            91_133,
            wow_data::spell::aura_types::SPELL_AURA_MOD_IGNORE_TARGET_RESIST,
            0x01,
            100,
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
                aura_type: Some(aura),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: aura,
                    effect_misc_value_1: misc,
                    effect_base_points: amount,
                    ..Default::default()
                }],
            },
        );
    }
    session.set_spell_store(Arc::new(spell_store));

    let swing = |session: &mut WorldSession| {
        let melee_damage_bonus = session.represented_melee_damage_bonus_like_cpp();
        let armor_mitigation = session.represented_melee_armor_mitigation_like_cpp();
        let outcome_facts = session.represented_melee_outcome_facts_like_cpp();
        let damage_taken = session.represented_melee_damage_taken_like_cpp();
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
                    outcome_facts,
                    damage_taken,
                )
            })
            .flatten()
            .map(|(swings, _)| swings)
    };

    assert_eq!(
        swing(&mut session).map(|swings| swings[0].damage),
        Some(100),
        "no victim modifier"
    );

    // The flat -50 melee-damage-taken aura applies first; taken modifiers live
    // on the victim.
    let apply_victim_aura = |session: &mut WorldSession, spell_id: u32| {
        session
            .mutate_world_creature(guid, |creature| {
                creature
                    .creature
                    .unit_mut()
                    .subsystems_mut()
                    .auras
                    .add_applied(wow_entities::AppliedAuraRef::new(spell_id, player, 0, 1));
            })
            .expect("victim aura");
    };
    apply_victim_aura(&mut session, 91_131);
    assert_eq!(swing(&mut session).map(|swings| swings[0].damage), Some(50));

    // A -50% damage-percent-taken aura on the victim halves `(100 - 50) * 0.5`.
    apply_victim_aura(&mut session, 91_132);
    assert_eq!(swing(&mut session).map(|swings| swings[0].damage), Some(25));

    // The attacker's 100% normal-school ignore-resist aura cancels the
    // reduction, so only the flat term remains.
    session
        .apply_aura(91_133, player, 30_000, 1)
        .expect("apply ignore-target-resist aura");
    assert_eq!(swing(&mut session).map(|swings| swings[0].damage), Some(50));
}

#[test]
fn white_swing_gates_avoidance_on_the_controlled_state_like_cpp() {
    use wow_packet::packets::combat::{HIT_INFO_AFFECTS_VICTIM, VICTIM_STATE_DODGE};

    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_041);
    let player = ObjectGuid::create_player(1, 99);

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
        "Controlled".to_string(),
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
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 7.0, 7.0);
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
            // A +100% `SPELL_AURA_MOD_DODGE_PERCENT` pushes the dodge band past
            // the roll, so the swing is guaranteed to be dodged.
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .add_applied(wow_entities::AppliedAuraRef::new(91_152, player, 0, 1));
        })
        .unwrap();
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        91_152,
        wow_data::SpellInfo {
            spell_id: 91_152,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 100,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_DODGE_PERCENT),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_DODGE_PERCENT,
                effect_base_points: 100,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    let swing = |session: &mut WorldSession| {
        let melee_damage_bonus = session.represented_melee_damage_bonus_like_cpp();
        let armor_mitigation = session.represented_melee_armor_mitigation_like_cpp();
        let outcome_facts = session.represented_melee_outcome_facts_like_cpp();
        let damage_taken = session.represented_melee_damage_taken_like_cpp();
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
                    outcome_facts,
                    damage_taken,
                )
            })
            .flatten()
            .map(|(swings, _)| swings)
    };
    // The +100% dodge aura makes the band absolute while the victim is free.
    assert_eq!(
        swing(&mut session).map(|swings| (swings[0].damage, swings[0].victim_state)),
        Some((0, VICTIM_STATE_DODGE))
    );
    // C++ clears both avoidance gates for a `UNIT_STATE_CONTROLLED` victim.
    session
        .mutate_world_creature(guid, |creature| {
            creature
                .creature
                .unit_mut()
                .add_unit_state(UnitState::CONTROLLED.bits());
        })
        .unwrap();
    let facts = session.represented_melee_outcome_facts_like_cpp();
    assert!(facts.1.is_controlled);
    assert_eq!(
        swing(&mut session).map(|swings| (swings[0].damage, swings[0].hit_info)),
        Some((7, HIT_INFO_AFFECTS_VICTIM))
    );
}

#[test]
fn white_swing_applies_victim_critical_chance_auras_like_cpp() {
    use wow_packet::packets::combat::{
        HIT_INFO_AFFECTS_VICTIM, HIT_INFO_CRITICAL_HIT, VICTIM_STATE_HIT,
    };

    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_039);
    let player = ObjectGuid::create_player(1, 96);
    let foreign_caster = ObjectGuid::create_player(1, 97);

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
        "Crit".to_string(),
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
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 7.0, 7.0);
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

    // `SPELL_AURA_MOD_CRIT_CHANCE_FOR_CASTER` only applies when the attacker
    // cast it; the fixture's foreign caster must not add critical chance.
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        91_160,
        wow_data::SpellInfo {
            spell_id: 91_160,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 100,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_CRIT_CHANCE_FOR_CASTER),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_CRIT_CHANCE_FOR_CASTER,
                effect_base_points: 100,
                ..Default::default()
            }],
        },
    );
    spell_store.insert(
        91_161,
        wow_data::SpellInfo {
            spell_id: 91_161,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 100,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(
                wow_data::spell::aura_types::SPELL_AURA_MOD_CRIT_CHANCE_VERSUS_TARGET_HEALTH,
            ),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura:
                    wow_data::spell::aura_types::SPELL_AURA_MOD_CRIT_CHANCE_VERSUS_TARGET_HEALTH,
                effect_misc_value_1: 0,
                // `MiscValueB` is the health threshold: `!HealthBelowPct(100)`
                // is always true for a living victim.
                effect_misc_value_2: 100,
                effect_base_points: 100,
                ..Default::default()
            }],
        },
    );
    spell_store.insert(
        91_162,
        wow_data::SpellInfo {
            spell_id: 91_162,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 100,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_CRIT_DAMAGE_BONUS),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_CRIT_DAMAGE_BONUS,
                effect_misc_value_1: 0x01,
                effect_base_points: 100,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    let apply = |session: &mut WorldSession, spell_id: u32, caster: ObjectGuid| {
        session
            .mutate_world_creature(guid, |creature| {
                creature
                    .creature
                    .unit_mut()
                    .subsystems_mut()
                    .auras
                    .add_applied(wow_entities::AppliedAuraRef::new(spell_id, caster, 0, 1));
            })
            .expect("victim aura");
    };
    let swing = |session: &mut WorldSession| {
        let melee_damage_bonus = session.represented_melee_damage_bonus_like_cpp();
        let armor_mitigation = session.represented_melee_armor_mitigation_like_cpp();
        let outcome_facts = session.represented_melee_outcome_facts_like_cpp();
        let damage_taken = session.represented_melee_damage_taken_like_cpp();
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
                    outcome_facts,
                    damage_taken,
                )
            })
            .flatten()
            .map(|(swings, _)| swings)
    };

    // A foreign caster's `MOD_CRIT_CHANCE_FOR_CASTER` is ignored.
    apply(&mut session, 91_160, foreign_caster);
    let facts = session.represented_melee_outcome_facts_like_cpp();
    assert_eq!(facts.1.crit_chance_for_caster_pct, 0.0);
    let swings = swing(&mut session).expect("white swing");
    assert_eq!(swings[0].damage, 7);
    assert_eq!(swings[0].hit_info, HIT_INFO_AFFECTS_VICTIM);

    // The attacker's own aura applies.
    apply(&mut session, 91_160, player);
    let facts = session.represented_melee_outcome_facts_like_cpp();
    assert_eq!(facts.1.crit_chance_for_caster_pct, 100.0);

    // The target-health aura is over the whole band, so the swing crits
    // deterministically.
    apply(&mut session, 91_161, player);
    let facts = session.represented_melee_outcome_facts_like_cpp();
    assert_eq!(facts.1.crit_chance_vs_target_health_pct, 100.0);
    let swings = swing(&mut session).expect("white swing");
    assert_eq!(swings[0].damage, 14);
    // C++ assigns `OriginalDamage` after the doubling, so the critical swing
    // publishes the doubled value as its original too (`Unit.cpp:1362-1375`).
    assert_eq!(swings[0].original_damage, 14);
    assert_eq!(
        swings[0].hit_info,
        HIT_INFO_AFFECTS_VICTIM | HIT_INFO_CRITICAL_HIT
    );
    assert_eq!(swings[0].victim_state, VICTIM_STATE_HIT);

    // C++ `SPELL_AURA_MOD_CRIT_DAMAGE_BONUS` scales the doubled damage.
    session
        .apply_aura(91_162, player, 30_000, 1)
        .expect("apply crit-damage-bonus aura");
    let swings = swing(&mut session).expect("white swing");
    assert_eq!(swings[0].damage, 28);
    assert_eq!(swings[0].original_damage, 28);
    assert_eq!(
        swings[0].hit_info,
        HIT_INFO_AFFECTS_VICTIM | HIT_INFO_CRITICAL_HIT
    );
}

#[test]
fn melee_attack_table_reads_the_dual_wield_penalty_aura_like_cpp() {
    use crate::session_rules::melee_outcome_inputs_like_cpp;

    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_040);
    let player = ObjectGuid::create_player(1, 98);

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
        "DualWield".to_string(),
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
        })
        .unwrap();
    register_test_creature(&mut session, manager.clone(), guid, 40);
    // The fixture trains the attacker past the dual-wield penalty; this scenario
    // needs the plain `7.5` `m_modMeleeHitChance`.
    session
        .mutate_canonical_player_like_cpp(|player| {
            let mut stats = *player.effective_combat_stats_like_cpp();
            stats.melee_hit_chance_pct = 7.5;
            player.replace_effective_combat_stats_like_cpp(stats);
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
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_IGNORE_DUAL_WIELD_HIT_PENALTY),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_IGNORE_DUAL_WIELD_HIT_PENALTY,
                effect_base_points: 0,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    let bare = session.represented_melee_outcome_facts_like_cpp();
    assert!(!bare.0.ignores_dual_wield_hit_penalty);
    // A dual-wielding attacker without the aura carries `5 + 19` miss.
    let dual = crate::session_rules::RepresentedMeleeAttackerFactsLikeCpp {
        dual_wielding: true,
        ..bare.0
    };
    let inputs = melee_outcome_inputs_like_cpp(&dual, &bare.1);
    // `5 + 19 - 7.5`.
    assert_eq!(inputs[0].miss_chance_pct, 16.5);

    // With the aura the penalty disappears.
    session
        .apply_aura(91_170, player, 30_000, 1)
        .expect("apply ignore-dual-wield aura");
    let facts = session.represented_melee_outcome_facts_like_cpp();
    assert!(facts.0.ignores_dual_wield_hit_penalty);
    let dual = crate::session_rules::RepresentedMeleeAttackerFactsLikeCpp {
        dual_wielding: true,
        ..facts.0
    };
    let inputs = melee_outcome_inputs_like_cpp(&dual, &facts.1);
    assert_eq!(inputs[0].miss_chance_pct, 0.0);
}

#[test]
fn white_swing_publishes_an_evade_like_cpp() {
    use wow_packet::packets::combat::{
        HIT_INFO_MISS, HIT_INFO_SWING_NO_HIT_SOUND, VICTIM_STATE_EVADES,
    };

    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_042);
    let player = ObjectGuid::create_player(1, 100);

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
        "Evade".to_string(),
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
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 7.0, 7.0);
        })
        .unwrap();
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);
    let swing = |session: &mut WorldSession| {
        let melee_damage_bonus = session.represented_melee_damage_bonus_like_cpp();
        let armor_mitigation = session.represented_melee_armor_mitigation_like_cpp();
        let outcome_facts = session.represented_melee_outcome_facts_like_cpp();
        let damage_taken = session.represented_melee_damage_taken_like_cpp();
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
                    outcome_facts,
                    damage_taken,
                )
            })
            .flatten()
            .map(|(swings, _)| swings)
    };

    // A free victim lands a normal hit.
    assert_eq!(swing(&mut session).map(|s| s[0].damage), Some(7));

    // C++ `IsEvadingAttacks()` returns `MELEE_HIT_EVADE` before any band.
    session
        .mutate_world_creature(guid, |creature| {
            creature.creature.set_in_evade_mode_like_cpp(true);
        })
        .unwrap();
    let facts = session.represented_melee_outcome_facts_like_cpp();
    assert!(facts.1.is_evading_attacks);
    let swings = swing(&mut session).expect("white swing resolves");
    assert_eq!(swings[0].damage, 0);
    assert_eq!(
        swings[0].hit_info,
        HIT_INFO_MISS | HIT_INFO_SWING_NO_HIT_SOUND
    );
    assert_eq!(swings[0].victim_state, VICTIM_STATE_EVADES);
}
