//! Complete represented white-swing calculation over resolved scalar facts.

use crate::{
    CREATURE_BLOCK_PERCENT_LIKE_CPP, MELEE_OUTCOME_ROLL_MAX_LIKE_CPP,
    RepresentedMeleeAttackerFactsLikeCpp, RepresentedMeleeDamageTakenLikeCpp,
    RepresentedMeleeOutcomeLikeCpp, RepresentedMeleeVictimFactsLikeCpp,
    armor_reduced_damage_like_cpp, melee_damage_taken_apply_like_cpp,
    melee_outcome_damage_like_cpp, melee_outcome_inputs_like_cpp, melee_outcome_like_cpp,
};

/// C++ `Unit::CalcArmorReducedDamage` inputs the swing owner resolves once per
/// victim. `NONE` means "no represented armour": every field is zero, so the
/// reduction is zero and the swing damage is unchanged.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ArmorMitigation {
    pub attacker_level: u8,
    pub victim_level: u8,
    pub victim_armor: i32,
    pub armor_penetration_pct: f32,
    pub target_resistance_normal_aura: i32,
    /// The attacker's `SPELL_AURA_MOD_IGNORE_TARGET_RESIST` (269) sum for
    /// `SPELL_SCHOOL_MASK_NORMAL`.
    pub ignore_target_resist_normal_pct: f32,
    /// The victim's `SPELL_AURA_BYPASS_ARMOR_FOR_CASTER` (345) sum for effects
    /// this attacker cast.
    pub bypass_armor_pct_by_caster: f32,
}

impl ArmorMitigation {
    pub const NONE: Self = Self {
        attacker_level: 0,
        victim_level: 0,
        victim_armor: 0,
        armor_penetration_pct: 0.0,
        target_resistance_normal_aura: 0,
        ignore_target_resist_normal_pct: 0.0,
        bypass_armor_pct_by_caster: 0.0,
    };
}

/// Inclusive roll bounds from the caller's published weapon damage range.
///
/// C++ `Unit::CalculateDamage`, `a5f8da2e`, `Unit.cpp:2426-2435`: clamp both
/// floats at zero, order them and truncate to `uint32` before calling `urand`.
pub fn white_swing_damage_roll_bounds(min_damage: f32, max_damage: f32) -> (u32, u32) {
    let min_damage = min_damage.max(0.0);
    let max_damage = max_damage.max(0.0);
    let (min_damage, max_damage) = if min_damage > max_damage {
        (max_damage, min_damage)
    } else {
        (min_damage, max_damage)
    };
    (min_damage as u32, max_damage as u32)
}

/// Calculate one represented swing, returning its semantic outcome and
/// `(damage, blocked, original_damage)` in the existing arithmetic order.
///
/// Source: `a5f8da2e`, `Unit.cpp:1326-1343` (base/done/taken/armor/table)
/// and `1343-1440` (outcome damage). Callers resolve all state/catalog inputs
/// before entering this operation and own the random engine and publication.
/// The two callbacks are each invoked once: base damage first, then the attack
/// table after mitigation and derivation of both mainhand/offhand tables. Equal
/// bounds, immunity and evade do not skip either callback in this Rust path.
///
/// Preserved boundaries: done/autoattack retains its float expression followed
/// by `max(1).round()` before taken; it does not use the separate done-apply
/// helper's earlier clamp/truncation. Block damage retains the flat 30% value.
/// This extraction adds no C++ script hook, critical block, resilience or
/// absorb/resist stages. The represented immunity check remains after both
/// draws and retains pre-outcome original damage, unlike C++'s early return.
/// Packet hit-info/victim-state mapping remains with the caller.
#[allow(clippy::too_many_arguments)]
pub fn calculate_white_swing(
    weapon_range: [f32; 2],
    autoattack_multiplier: f32,
    done_bonus: (i32, f32),
    armor: ArmorMitigation,
    outcome_facts: (
        &RepresentedMeleeAttackerFactsLikeCpp,
        &RepresentedMeleeVictimFactsLikeCpp,
    ),
    taken: RepresentedMeleeDamageTakenLikeCpp,
    offhand: bool,
    draw_damage: impl FnOnce(u32, u32) -> u32,
    draw_attack_table: impl FnOnce(u32, u32) -> u32,
) -> (RepresentedMeleeOutcomeLikeCpp, (u32, u32, u32)) {
    let (min_damage, max_damage) = white_swing_damage_roll_bounds(weapon_range[0], weapon_range[1]);
    let rolled = draw_damage(min_damage, max_damage) as f32;
    let damage = (rolled + done_bonus.0 as f32) * done_bonus.1 * autoattack_multiplier;
    // C++ `CalculateMeleeDamage` runs `MeleeDamageBonusTaken` between the done
    // bonus and the armour reduction (`Unit.cpp:1326-1341`).
    let damage = melee_damage_taken_apply_like_cpp(taken, damage.max(1.0).round() as u32);
    let damage = armor_reduced_damage_like_cpp(
        damage,
        armor.attacker_level,
        armor.victim_level,
        armor.victim_armor,
        armor.armor_penetration_pct,
        armor.target_resistance_normal_aura,
        armor.ignore_target_resist_normal_pct,
        armor.bypass_armor_pct_by_caster,
    );
    // C++ assigns `OriginalDamage` inside the outcome switch, so the shared
    // arithmetic returns it with the dealt damage (`Unit.cpp:1343-1440`).
    // C++ rolls the attack table after mitigation and before the outcome
    // switch (`Unit.cpp:1341-1343`).
    let outcome_inputs = melee_outcome_inputs_like_cpp(outcome_facts.0, outcome_facts.1);
    let selected_inputs = &outcome_inputs[usize::from(offhand)];
    let roll =
        i32::try_from(draw_attack_table(0, MELEE_OUTCOME_ROLL_MAX_LIKE_CPP)).unwrap_or_default();
    let outcome = melee_outcome_like_cpp(selected_inputs, roll);
    let damage = melee_outcome_damage_like_cpp(
        outcome,
        damage,
        armor.attacker_level,
        armor.victim_level,
        outcome_facts.0.crit_damage_multiplier,
        CREATURE_BLOCK_PERCENT_LIKE_CPP,
    );
    (outcome, damage)
}
