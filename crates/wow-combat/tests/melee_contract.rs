// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Public-API regression cases relocated from wow-world under #1233.
//! These tests need no Session, catalogs, database or async runtime.

#[test]
fn melee_attack_table_matches_roll_melee_outcome_against_like_cpp() {
    use wow_combat::{
        RepresentedMeleeOutcomeInputsLikeCpp as Inputs, RepresentedMeleeOutcomeLikeCpp as Outcome,
        melee_outcome_like_cpp,
    };

    // 1/10000-unit bands with miss 5%, dodge 3%, parry 6%, glancing 10% and
    // crit 20%: miss [0,500), dodge [500,800), parry [800,1400),
    // glancing [1400,2400), block [2400,2900), crit [2900,4900), hit [4900,10000).
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
        crushing_chance_units: 0,
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
fn melee_attack_table_inputs_resolve_cpp_chances_like_cpp() {
    use wow_combat::{
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
        is_controlled_by_player: true,
        no_crushing_blows: true,
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
    // never applies against a player victim, and the block band reads the
    // published BlockPercentage. Its damage reduction is resolved separately.
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
        [wow_combat::RepresentedMeleeOutcomeInputsLikeCpp::NONE; 2]
    );
}

#[test]
fn melee_crushing_band_preserves_cpp_expression_and_gates() {
    use wow_combat::{
        RepresentedMeleeAttackerFactsLikeCpp as Attacker,
        RepresentedMeleeOutcomeLikeCpp as Outcome, RepresentedMeleeVictimFactsLikeCpp as Victim,
        melee_outcome_inputs_like_cpp, melee_outcome_like_cpp,
    };

    // C++ `Unit.cpp:2364-2378` admits the band for a creature attacker four
    // levels above its victim, then evaluates `attackerLevel - victimLevel *
    // 1000 - 1500` verbatim. At 80 versus 76 that is -77,420, so the outcome
    // remains Hit even at the top of the roll range.
    let mut attacker = Attacker {
        level: 80,
        is_controlled_by_player: false,
        no_crushing_blows: false,
        ..Default::default()
    };
    let victim = Victim {
        level: 76,
        is_creature: true,
        ..Default::default()
    };
    let inputs = melee_outcome_inputs_like_cpp(&attacker, &victim);
    assert_eq!(inputs[0].crushing_chance_units, -77_420);
    assert_eq!(melee_outcome_like_cpp(&inputs[0], 9_999), Outcome::Hit);

    // Player-controlled creatures and NO_CRUSHING_BLOWS skip the band before
    // the source expression is evaluated.
    attacker.is_controlled_by_player = true;
    assert_eq!(
        melee_outcome_inputs_like_cpp(&attacker, &victim)[0].crushing_chance_units,
        0
    );
    attacker.is_controlled_by_player = false;
    attacker.no_crushing_blows = true;
    assert_eq!(
        melee_outcome_inputs_like_cpp(&attacker, &victim)[0].crushing_chance_units,
        0
    );
}

#[test]
fn player_block_percent_matches_get_block_percent_like_cpp() {
    use wow_combat::player_block_percent_like_cpp as percent;
    // C++ `Player::GetBlockPercent` (`Player.cpp:25288-25298`): a fraction
    // capped at `0.85`, and `0` when both inputs are zero.
    assert_eq!(percent(2_000, 1.0), 0.85);
    assert_eq!(percent(2_000, 8_000.0), 0.2);
    assert_eq!(percent(0, 0.0), 0.0);
}
