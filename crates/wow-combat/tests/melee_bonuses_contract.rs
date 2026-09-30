// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_core::ObjectGuid;

#[test]
fn melee_damage_taken_matches_cpp_like_cpp() {
    use wow_combat::{
        AppliedAuraEffectLikeCpp as Effect, RepresentedMeleeDamageTakenLikeCpp as Taken,
        melee_damage_taken_apply_like_cpp, melee_damage_taken_flat_pct_like_cpp,
    };
    use wow_constants::spell::aura_types::{
        SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN, SPELL_AURA_MOD_DAMAGE_TAKEN,
        SPELL_AURA_MOD_MELEE_DAMAGE_FROM_CASTER, SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN,
        SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN_PCT,
    };

    let attacker = ObjectGuid::create_player(1, 92);
    let effect = |aura_type, misc_value, amount, caster_guid| Effect {
        slot: 0,
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

/// C++ `Unit::CalcAbsorbResist`'s `auraAbsorbMod` term (`Unit.cpp:1803-1832`):
/// an attacker's `SPELL_AURA_MOD_TARGET_ABSORB_SCHOOL` reduces how much of a hit
/// a school-absorb or mana shield may take, unless the shield's spell carries
/// `SPELL_ATTR6_ABSORB_CANNOT_BE_IGNORE`.
#[test]
fn represented_melee_ignore_absorb_matches_calc_absorb_resist_like_cpp() {
    use wow_combat::{
        AppliedAuraEffectLikeCpp, RepresentedAbsorbShieldLikeCpp as Shield,
        RepresentedManaShieldLikeCpp as ManaShield, represented_melee_absorb_like_cpp,
        represented_melee_ignore_absorb_like_cpp, represented_melee_ignored_absorb_amount_like_cpp,
        represented_melee_mana_absorb_like_cpp,
    };

    let effect = |misc_value: i32, amount: i32| AppliedAuraEffectLikeCpp {
        slot: 0,
        spell_id: 91_300,
        caster_guid: ObjectGuid::create_null(),
        aura_type: wow_constants::spell::aura_types::SPELL_AURA_MOD_TARGET_ABSORB_SCHOOL,
        misc_value,
        misc_value_b: 0,
        amount,
    };

    // `GetMaxPositiveAuraModifierByMiscMask` then `RoundToInterval(0, 100)`.
    assert_eq!(represented_melee_ignore_absorb_like_cpp(&[], 0x01), 0.0);
    assert_eq!(
        represented_melee_ignore_absorb_like_cpp(&[effect(0x04, 50)], 0x01),
        0.0,
        "a different school's modifier does not match"
    );
    assert_eq!(
        represented_melee_ignore_absorb_like_cpp(&[effect(0x01, 50)], 0x01),
        50.0
    );
    assert_eq!(
        represented_melee_ignore_absorb_like_cpp(&[effect(0x01, 25), effect(0x01, 40)], 0x01),
        40.0
    );
    assert_eq!(
        represented_melee_ignore_absorb_like_cpp(&[effect(0x01, -10)], 0x01),
        0.0,
        "the maximum starts at zero"
    );
    assert_eq!(
        represented_melee_ignore_absorb_like_cpp(&[effect(0x01, 150)], 0x01),
        100.0
    );

    // `CalculatePct(damage, pct)` truncates.
    assert_eq!(
        represented_melee_ignored_absorb_amount_like_cpp(100, 50.0),
        50
    );
    assert_eq!(represented_melee_ignored_absorb_amount_like_cpp(7, 50.0), 3);
    assert_eq!(
        represented_melee_ignored_absorb_amount_like_cpp(100, 0.0),
        0
    );

    let shield = |amount: i32, cannot_be_ignored: bool| Shield {
        slot: 1,
        effect_index: 0,
        spell_id: 91_200,
        category_id: 0,
        amount,
        cannot_be_ignored,
    };
    // A shield without the attribute may only take what the modifier left.
    let ignored = represented_melee_absorb_like_cpp(&[shield(100, false)], 100, 50.0);
    assert_eq!((ignored.absorbed, ignored.damage), (50, 50));
    // With the attribute the whole hit is absorbable.
    let protected = represented_melee_absorb_like_cpp(&[shield(100, true)], 100, 50.0);
    assert_eq!((protected.absorbed, protected.damage), (100, 0));

    let mana_shield = |amount: i32, cannot_be_ignored: bool| ManaShield {
        slot: 1,
        effect_index: 0,
        spell_id: 91_520,
        amount,
        mana_multiplier: 1.0,
        cannot_be_ignored,
    };
    let mana_ignored =
        represented_melee_mana_absorb_like_cpp(&[mana_shield(100, false)], 100, 1_000, 50.0);
    assert_eq!(
        (
            mana_ignored.absorbed,
            mana_ignored.damage,
            mana_ignored.mana_spent
        ),
        (50, 50, 50)
    );
    let mana_protected =
        represented_melee_mana_absorb_like_cpp(&[mana_shield(100, true)], 100, 1_000, 50.0);
    assert_eq!(
        (
            mana_protected.absorbed,
            mana_protected.damage,
            mana_protected.mana_spent
        ),
        (100, 0, 100)
    );
}

#[test]
fn melee_damage_bonus_done_uses_resolved_masks_and_caster_independent_aura_order() {
    use wow_combat::{
        AppliedAuraEffectLikeCpp as Effect, melee_damage_bonus_done_from_effects_like_cpp as done,
    };
    use wow_constants::spell::aura_types::{
        SPELL_AURA_MOD_AUTOATTACK_DAMAGE, SPELL_AURA_MOD_DAMAGE_DONE_CREATURE,
        SPELL_AURA_MOD_DAMAGE_DONE_VERSUS, SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE,
        SPELL_AURA_MOD_DAMAGE_PERCENT_DONE_BY_TARGET_AURA_MECHANIC,
    };

    let external_caster = ObjectGuid::create_player(1, 4);
    let another_external_caster = ObjectGuid::create_player(1, 6);
    let effect = |aura_type, misc_value, amount, caster_guid| Effect {
        slot: 0,
        spell_id: 1,
        caster_guid,
        aura_type,
        misc_value,
        misc_value_b: 0,
        amount,
    };
    let effects = [
        effect(
            SPELL_AURA_MOD_DAMAGE_DONE_CREATURE,
            0b001,
            30,
            external_caster,
        ),
        effect(
            SPELL_AURA_MOD_DAMAGE_DONE_CREATURE,
            0b010,
            700,
            external_caster,
        ),
        effect(
            SPELL_AURA_MOD_DAMAGE_DONE_VERSUS,
            0b001,
            50,
            external_caster,
        ),
        effect(
            SPELL_AURA_MOD_DAMAGE_DONE_VERSUS,
            0b010,
            100,
            external_caster,
        ),
        effect(
            SPELL_AURA_MOD_AUTOATTACK_DAMAGE,
            0,
            100,
            another_external_caster,
        ),
        effect(
            SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE,
            3,
            50,
            another_external_caster,
        ),
        effect(
            SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE,
            4,
            -50,
            external_caster,
        ),
        effect(
            SPELL_AURA_MOD_DAMAGE_PERCENT_DONE_BY_TARGET_AURA_MECHANIC,
            5,
            100,
            another_external_caster,
        ),
        effect(
            SPELL_AURA_MOD_DAMAGE_PERCENT_DONE_BY_TARGET_AURA_MECHANIC,
            4,
            -50,
            external_caster,
        ),
    ];

    // These resolved done effects aggregate regardless of their individual
    // caster GUIDs. Creature, aura-state, and mechanic masks select their own
    // terms; auto-attack percentage is independent of those masks.
    let (flat, pct) = done(&effects, 0, 0b001, 1 << 2, 1 << 5, false, 1.0);
    assert_eq!(flat, 30);
    // Creature-vs 1.5, auto-attack 2.0, aura-state 1.5, mechanic 2.0,
    // applied in the function's C++ stage order.
    assert_eq!(pct, 9.0);

    // A zero creature mask suppresses both creature terms, while the
    // independent auto-attack, aura-state, and mechanic terms remain.
    let (flat, pct) = done(&effects, 0, 0, 1 << 2, 1 << 5, false, 1.0);
    assert_eq!(flat, 0);
    assert_eq!(pct, 6.0);
}

#[test]
fn melee_damage_bonus_done_retains_separate_ap_rounding_and_apply_order() {
    use wow_combat::{
        AppliedAuraEffectLikeCpp as Effect, melee_damage_bonus_done_apply_like_cpp as apply,
        melee_damage_bonus_done_from_effects_like_cpp as done,
    };
    use wow_constants::spell::aura_types::SPELL_AURA_MOD_MELEE_ATTACK_POWER_VERSUS;

    let effect = Effect {
        slot: 0,
        spell_id: 1,
        caster_guid: ObjectGuid::create_player(1, 5),
        aura_type: SPELL_AURA_MOD_MELEE_ATTACK_POWER_VERSUS,
        misc_value: 0b001,
        misc_value_b: 0,
        amount: 2,
    };
    // The represented path converts the attacker-side +2 and victim-side +2
    // AP components separately (both truncate to zero); C++ combines them
    // before conversion (4 / 3.5 truncates to one). Keep this existing boundary.
    let bonus = done(&[effect], 2, 0b001, 0, 0, false, 1.0);
    assert_eq!(bonus.0, 0);

    // C++ adds flat benefit before multiplying by DoneTotalMod.
    assert_eq!(apply(100, (20, 0.5)), 60);
    // Its final bonus result clamps at zero.
    assert_eq!(apply(100, (-200, 0.5)), 0);
}
