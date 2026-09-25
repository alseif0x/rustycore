// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Receiver-free melee damage rules used by the world-session runtime.

use super::aura_effects::{
    AppliedAuraEffectLikeCpp, player_aura_effects_all_like_cpp,
    player_aura_effects_by_spell_aura_type_like_cpp,
};
use crate::session::*;
use std::collections::HashMap;
use wow_data::SpellStore;
use wow_entities::AuraApplicationLikeCpp;

/// C++ `Unit::MeleeDamageBonusDone`'s auto-attack percentage term
/// (`Unit.cpp:7620-7627`): `AddPct(DoneTotalMod, amount)` for every active
/// `SPELL_AURA_MOD_AUTOATTACK_DAMAGE` effect. The represented white swing
/// multiplies its rolled damage by the returned factor; `1.0` when nothing is
/// active.
pub(crate) fn represented_autoattack_damage_multiplier_like_cpp(
    auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
) -> f32 {
    player_aura_effects_by_spell_aura_type_like_cpp(
        auras,
        spell_store,
        wow_data::spell::aura_types::SPELL_AURA_MOD_AUTOATTACK_DAMAGE,
    )
    .into_iter()
    .fold(1.0_f32, |total, (_, amount)| {
        total * (1.0 + amount as f32 / 100.0)
    })
}

/// C++ `Unit::MeleeDamageBonusDone`'s white-swing terms (`Unit.cpp:7558-7650`)
/// resolved from one attacker's aura effects: the flat
/// `SPELL_AURA_MOD_DAMAGE_DONE_CREATURE` benefit, the
/// `SPELL_AURA_MOD_MELEE_ATTACK_POWER_VERSUS` bonus converted with
/// `GetAPMultiplier`, the `SPELL_AURA_MOD_DAMAGE_DONE_VERSUS` multiplier, plus
/// the victim's `HasAuraState` (`SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE`)
/// and `HasAuraWithMechanic`
/// (`SPELL_AURA_MOD_DAMAGE_PERCENT_DONE_BY_TARGET_AURA_MECHANIC`) multipliers.
/// `(DoneFlatBenefit, DoneTotalMod)`.
///
/// `victim_attack_power_bonus` is the victim's
/// `SPELL_AURA_MELEE_ATTACK_POWER_ATTACKER_BONUS` (`165`) sum, which C++ reads
/// from the victim and the caller resolves because only a player victim's auras
/// carry represented effect amounts.
pub(crate) fn melee_damage_bonus_done_from_effects_like_cpp(
    attacker_effects: &[AppliedAuraEffectLikeCpp],
    victim_attack_power_bonus: i32,
    creature_type_mask: u32,
    victim_aura_state_mask: u32,
    victim_mechanic_mask: u64,
    is_ranged: bool,
    attack_power_multiplier: f32,
) -> (i32, f32) {
    let matches = |misc_value: i32| misc_value & creature_type_mask as i32 != 0;
    let flat_sum_by_mask = |aura_type: i32| -> i32 {
        attacker_effects
            .iter()
            .filter(|effect| effect.aura_type == aura_type && matches(effect.misc_value))
            .map(|effect| effect.amount)
            .sum()
    };
    let pct_by_mask = |aura_type: i32| -> f32 {
        attacker_effects
            .iter()
            .filter(|effect| effect.aura_type == aura_type && matches(effect.misc_value))
            .fold(1.0_f32, |total, effect| {
                total * (1.0 + effect.amount as f32 / 100.0)
            })
    };
    // C++ accumulates the victim's attacker-bonus aura into `APbonus` and
    // converts the complete value through `GetAPMultiplier` below. It does
    // not add the raw aura amount as a second flat term.
    let mut flat: i32 = 0;
    if creature_type_mask != 0 {
        flat = flat.saturating_add(flat_sum_by_mask(
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_CREATURE,
        ));
        let versus_aura_type = if is_ranged {
            wow_data::spell::aura_types::SPELL_AURA_MOD_RANGED_ATTACK_POWER_VERSUS
        } else {
            wow_data::spell::aura_types::SPELL_AURA_MOD_MELEE_ATTACK_POWER_VERSUS
        };
        let ap_bonus = flat_sum_by_mask(versus_aura_type);
        if ap_bonus != 0 {
            flat = flat.saturating_add((ap_bonus as f32 / 3.5 * attack_power_multiplier) as i32);
        }
    }
    if victim_attack_power_bonus != 0 {
        // The victim's own AP-granting aura is converted with the same
        // `GetAPMultiplier` factor C++ uses for the attacker-side `APbonus`.
        flat = flat.saturating_add(
            (victim_attack_power_bonus as f32 / 3.5 * attack_power_multiplier) as i32,
        );
    }
    let mut done_total_mod = 1.0_f32;
    if creature_type_mask != 0 {
        done_total_mod *=
            pct_by_mask(wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS);
    }
    // C++ `Unit::MeleeDamageBonusDone`'s auto-attack percentage branch
    // (`Unit.cpp:7620-7627`), read for every white swing.
    done_total_mod *= attacker_effects
        .iter()
        .filter(|effect| {
            effect.aura_type == wow_data::spell::aura_types::SPELL_AURA_MOD_AUTOATTACK_DAMAGE
        })
        .fold(1.0_f32, |total, effect| {
            total * (1.0 + effect.amount as f32 / 100.0)
        });
    // C++ `Unit::MeleeDamageBonusDone`'s "bonus against aurastate"
    // (`Unit.cpp:7634-7640`).
    if victim_aura_state_mask != 0 {
        done_total_mod *= attacker_effects
            .iter()
            .filter(|effect| {
                effect.aura_type
                    == wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE
                    && u32::try_from(effect.misc_value)
                        .ok()
                        .and_then(|state| state.checked_sub(1))
                        .and_then(|bit| 1_u32.checked_shl(bit))
                        .is_some_and(|bit| victim_aura_state_mask & bit != 0)
            })
            .fold(1.0_f32, |total, effect| {
                total * (1.0 + effect.amount as f32 / 100.0)
            });
    }
    // C++ `Unit::MeleeDamageBonusDone`'s "bonus against target aura mechanic"
    // (`Unit.cpp:7642-7648`).
    if victim_mechanic_mask != 0 {
        done_total_mod *= attacker_effects
            .iter()
            .filter(|effect| {
                effect.aura_type
                    == wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE_BY_TARGET_AURA_MECHANIC
                    && (1..64).contains(&effect.misc_value)
                    && victim_mechanic_mask & (1_u64 << effect.misc_value) != 0
            })
            .fold(1.0_f32, |total, effect| {
                total * (1.0 + effect.amount as f32 / 100.0)
            });
    }
    (flat, done_total_mod)
}

/// Apply the `(DoneFlatBenefit, DoneTotalMod)` pair produced by
/// `Unit::MeleeDamageBonusDone` to one already rolled white swing.
///
/// C++ performs the float multiplication and clamps the bonus result at zero
/// before returning it to `CalculateMeleeDamage` (`Unit.cpp:7649-7650`).
pub(crate) fn melee_damage_bonus_done_apply_like_cpp(damage: u32, bonus: (i32, f32)) -> u32 {
    let damage = (damage as f32 + bonus.0 as f32) * bonus.1;
    damage.max(0.0) as u32
}

/// C++ `Unit::MeleeDamageBonusDone`'s white-swing terms for a canonical
/// Player attacker, read across the represented aura-effect projection.
///
/// Boundary: a creature victim's `SPELL_AURA_MELEE_ATTACK_POWER_ATTACKER_BONUS`
/// has no represented effect amount, so `victim_attack_power_bonus` is zero
/// here; a player victim resolves it through
/// [`melee_damage_bonus_done_from_effects_like_cpp`] directly.
pub(crate) fn melee_damage_bonus_done_like_cpp(
    attacker_auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
    creature_type_mask: u32,
    victim_aura_state_mask: u32,
    victim_mechanic_mask: u64,
    is_ranged: bool,
    attack_power_multiplier: f32,
) -> (i32, f32) {
    let effects: Vec<AppliedAuraEffectLikeCpp> =
        player_aura_effects_all_like_cpp(attacker_auras, spell_store)
            .into_iter()
            .map(|effect| effect.as_applied_like_cpp())
            .collect();
    melee_damage_bonus_done_from_effects_like_cpp(
        &effects,
        0,
        creature_type_mask,
        victim_aura_state_mask,
        victim_mechanic_mask,
        is_ranged,
        attack_power_multiplier,
    )
}

/// C++ `Unit::CalculateDamage(attType, normalized = false, addTotalPct = true)`
/// (`Unit.cpp:2384-2435`): both `UnitData` bounds are clamped at zero, ordered,
/// truncated to `uint32` and resolved with one inclusive `urand`. The represented
/// `UnitData` range is already `Player::CalculateMinMaxDamage`'s published value,
/// so the roll is the only missing step; C++ draws it per landed swing, never
/// from a timer that is not ready.
pub(crate) fn white_swing_roll_like_cpp(min_damage: f32, max_damage: f32) -> u32 {
    let min_damage = min_damage.max(0.0);
    let max_damage = max_damage.max(0.0);
    let (min_damage, max_damage) = if min_damage > max_damage {
        (max_damage, min_damage)
    } else {
        (min_damage, max_damage)
    };
    wow_core::urand_like_cpp(min_damage as u32, max_damage as u32)
}

/// C++ `Unit::CalcArmorReducedDamage` (`Unit.cpp:1623-1685`) for the represented
/// physical melee swing.
///
/// The represented inputs are the victim's `Unit::GetArmor()` (a creature's
/// `GenerateArmor` value or a player's published armour), the attacker's live
/// `GetRatingBonusValue(CR_ARMOR_PENETRATION)` percentage, the victim's
/// `SPELL_AURA_BYPASS_ARMOR_FOR_CASTER` (345) sum for effects the attacker cast,
/// the attacker's `SPELL_AURA_MOD_TARGET_RESISTANCE` (123) sum covering
/// `SPELL_SCHOOL_MASK_NORMAL`, and the attacker's
/// `SPELL_AURA_MOD_IGNORE_TARGET_RESIST` (269) sum covering the same school.
/// `GetArmorMultiplierForTarget` is `1.0` for every 3.4.3 unit (no override), so
/// it is not a term.
///
/// Boundaries: a spell's `SpellModOp::TargetResistance` adjustment cannot apply
/// to an auto-attack (`spellInfo == null`), and C++ truncates each
/// `SPELL_AURA_MOD_IGNORE_TARGET_RESIST` effect separately
/// (`armor = std::floor(AddPct(armor, -amount))`) while the owner sums the
/// amounts first, so two concurrent effects differ from C++ by that per-step
/// truncation.
pub(crate) fn armor_reduced_damage_like_cpp(
    damage: u32,
    attacker_level: u8,
    victim_level: u8,
    victim_armor: i32,
    armor_penetration_pct: f32,
    target_resistance_normal_aura: i32,
    ignore_target_resist_normal_pct: f32,
    bypass_armor_pct_by_caster: f32,
) -> u32 {
    // `armor *= victim->GetArmorMultiplierForTarget(attacker)` is a no-op.
    let mut armor = victim_armor.max(0) as f32;
    // C++ `armor = CalculatePct(armor, 100 - std::min(armorBypassPct, 100))`
    // over the victim's `SPELL_AURA_BYPASS_ARMOR_FOR_CASTER` effects that the
    // attacker cast (`Unit.cpp:1631-1637`).
    if bypass_armor_pct_by_caster != 0.0 {
        armor = armor * (100.0 - bypass_armor_pct_by_caster.min(100.0)) / 100.0;
    }
    // `armor += attacker->GetTotalAuraModifierByMiscMask(MOD_TARGET_RESISTANCE,
    // NORMAL)`; a negative sum is armour penetration.
    armor += target_resistance_normal_aura as f32;

    // `armor = std::floor(AddPct(armor, -amount))` over the attacker's
    // `SPELL_AURA_MOD_IGNORE_TARGET_RESIST` effects of the normal school.
    if ignore_target_resist_normal_pct != 0.0 {
        armor = (armor * (1.0 - ignore_target_resist_normal_pct / 100.0)).floor();
    }

    // `Player` CR_ARMOR_PENETRATION rating bonus, capped the way C++ caps it.
    let victim_level = victim_level as f32;
    let max_armor_pen = if victim_level < 60.0 {
        400.0 + 85.0 * victim_level
    } else {
        400.0 + 85.0 * victim_level + 4.5 * 85.0 * (victim_level - 59.0)
    };
    let max_armor_pen = ((armor + max_armor_pen) / 3.0).min(armor);
    armor -= max_armor_pen * (armor_penetration_pct.clamp(0.0, 100.0) / 100.0);

    if armor <= 0.0 {
        armor = 0.0;
    }

    // `levelModifier = attacker->GetLevel()`, extended above level 59.
    let mut level_modifier = attacker_level as f32;
    if level_modifier > 59.0 {
        level_modifier += 4.5 * (level_modifier - 59.0);
    }
    let mut damage_reduction = 0.1 * armor / (8.5 * level_modifier + 40.0);
    damage_reduction /= 1.0 + damage_reduction;
    let damage_reduction = damage_reduction.clamp(0.0, 0.75);

    (damage as f32 * (1.0 - damage_reduction)).max(0.0).ceil() as u32
}
