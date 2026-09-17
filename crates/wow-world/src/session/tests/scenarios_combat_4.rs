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
    // attacker: `levelModifier = 80 + 4.5 * 21 = 174.5`.
    assert_eq!(reduced(1_000, 80, 80, 0, 0.0, 0), 1_000, "no armour");
    assert_eq!(
        reduced(1_000, 80, 80, 5_000, 0.0, 0),
        753,
        "5,000 armour: ceil(1000 * (1 - 0.247127))"
    );
    assert_eq!(
        reduced(1_000, 80, 80, 5_000, 25.0, 0),
        803,
        "a 25% CR_ARMOR_PENETRATION bonus ignores a quarter of the armour"
    );
    assert_eq!(
        reduced(1_000, 80, 80, 5_000, 100.0, 0),
        1_000,
        "100% penetration removes the whole armour value"
    );
    assert_eq!(
        reduced(1_000, 80, 80, 10_000_000, 0.0, 0),
        250,
        "the reduction clamps at 75%"
    );
    assert_eq!(
        reduced(1_000, 80, 80, 5_000, 0.0, -5_000),
        1_000,
        "a negative MOD_TARGET_RESISTANCE sum cancels the armour"
    );
    assert_eq!(
        reduced(1_000, 80, 10, 500, 0.0, 0),
        969,
        "a victim below level 60 uses `maxArmorPen = 400 + 85 * level`"
    );
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

    // The attacker's `SPELL_AURA_MOD_TARGET_RESISTANCE` term only covers
    // `SPELL_SCHOOL_MASK_NORMAL`; a non-normal row leaves the armour alone.
    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, misc_value) in [(91_120_i32, 0x02_i32), (91_121, 0x01)] {
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: -5_000,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_TARGET_RESISTANCE),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_TARGET_RESISTANCE,
                    effect_misc_value_1: misc_value,
                    effect_base_points: -5_000,
                    ..Default::default()
                }],
            },
        );
    }
    session.set_spell_store(Arc::new(spell_store));
    session
        .apply_aura(91_120, player, 30_000, 1)
        .expect("apply non-normal target-resistance aura");
    assert_eq!(swing(&mut session), Some(vec![753]));

    // A normal-school negative sum is armour penetration and cancels the armour.
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
        glancing_chance_pct: 10.0,
        crit_chance_pct: 20.0,
        can_dodge: true,
        can_parry: true,
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
        (2_400, Outcome::Crit),
        (4_399, Outcome::Crit),
        (4_400, Outcome::Hit),
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
        HIT_INFO_AFFECTS_VICTIM, HIT_INFO_CRITICAL_HIT, HIT_INFO_GLANCING, HIT_INFO_MISS,
        HIT_INFO_OFFHAND, VICTIM_STATE_DODGE, VICTIM_STATE_HIT, VICTIM_STATE_INTACT,
        VICTIM_STATE_PARRY,
    };

    assert_eq!(melee_outcome_damage_like_cpp(Outcome::Miss, 100, 80, 80), 0);
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Dodge, 100, 80, 80),
        0
    );
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Parry, 100, 80, 80),
        0
    );
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Hit, 100, 80, 80),
        100
    );
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Crit, 100, 80, 80),
        200
    );
    // C++ `leveldif = min(victimLevel - attackerLevel, 3)` then
    // `reducePercent = 1 - leveldif * 0.1`.
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Glancing, 100, 80, 81),
        90
    );
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Glancing, 100, 80, 84),
        70
    );
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Glancing, 100, 80, 90),
        70
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
        RepresentedMeleeVictimFactsLikeCpp as Victim, melee_outcome_inputs_like_cpp,
    };

    let attacker = Attacker {
        level: 80,
        dual_wielding: false,
        melee_hit_chance_pct: 7.5,
        hit_chance_aura_pct: 0.0,
        crit_pct: [10.0, 5.0],
        autoattack_crit_aura_pct: 2.0,
        expertise_reduction_pct: [0.5, 0.25],
    };
    let creature = Victim {
        level: 80,
        is_creature: true,
        is_totem: false,
        dodge_pct: 3.0,
        parry_pct: 6.0,
        faces_attacker: true,
    };
    let inputs = melee_outcome_inputs_like_cpp(&attacker, &creature);
    // C++ `MeleeSpellMissChance`: 5.0 + 0 (two-hander) - 7.5 -> clamped to 0.
    assert_eq!(inputs[0].miss_chance_pct, 0.0);
    assert_eq!(inputs[0].dodge_chance_pct, 2.5);
    assert_eq!(inputs[0].parry_chance_pct, 5.5);
    assert_eq!(inputs[0].glancing_chance_pct, 0.0);
    assert_eq!(inputs[0].crit_chance_pct, 12.0);
    assert_eq!(inputs[1].dodge_chance_pct, 2.75);
    assert_eq!(inputs[1].parry_chance_pct, 5.75);
    assert_eq!(inputs[1].crit_chance_pct, 7.0);
    assert!(inputs[0].can_dodge && inputs[0].can_parry);

    // C++ adds +19% miss while dual wielding and 1.5% per victim level above
    // the attacker; glancing needs four levels of difference.
    let dual_wielding = Attacker {
        dual_wielding: true,
        ..attacker
    };
    let higher = Victim {
        level: 84,
        ..creature
    };
    let inputs = melee_outcome_inputs_like_cpp(&dual_wielding, &higher);
    assert_eq!(inputs[0].miss_chance_pct, 16.5);
    assert_eq!(inputs[0].dodge_chance_pct, 8.5);
    assert_eq!(inputs[0].parry_chance_pct, 11.5);
    assert_eq!(inputs[0].glancing_chance_pct, 50.0);

    // A totem has no dodge, parry or block; a player victim has no represented
    // table at all.
    let totem = Victim {
        is_totem: true,
        ..higher
    };
    let inputs = melee_outcome_inputs_like_cpp(&attacker, &totem);
    assert_eq!(inputs[0].dodge_chance_pct, 0.0);
    assert_eq!(inputs[0].parry_chance_pct, 0.0);
    let player_victim = Victim {
        is_creature: false,
        ..creature
    };
    let inputs = melee_outcome_inputs_like_cpp(&attacker, &player_victim);
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
                    block_pct: 3.0,
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
    assert_eq!(misses[0].hit_info, HIT_INFO_MISS);
    assert_eq!(misses[0].victim_state, VICTIM_STATE_INTACT);
}
