// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Deterministic melee bonus rules over caller-resolved aura effects.

use crate::AppliedAuraEffectLikeCpp;

/// C++ `Unit::MeleeDamageBonusDone`'s white-swing terms (`Unit.cpp:7558-7668`)
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
pub fn melee_damage_bonus_done_from_effects_like_cpp(
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
    // This represented path converts the attacker's AP-versus sum here and
    // `victim_attack_power_bonus` below as separate components. C++ combines
    // those components in one `APbonus` before calling `GetAPMultiplier`; the
    // current separate truncation is retained as an existing boundary.
    let mut flat: i32 = 0;
    if creature_type_mask != 0 {
        flat = flat.saturating_add(flat_sum_by_mask(
            wow_constants::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_CREATURE,
        ));
        let versus_aura_type = if is_ranged {
            wow_constants::spell::aura_types::SPELL_AURA_MOD_RANGED_ATTACK_POWER_VERSUS
        } else {
            wow_constants::spell::aura_types::SPELL_AURA_MOD_MELEE_ATTACK_POWER_VERSUS
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
            pct_by_mask(wow_constants::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS);
    }
    // C++ `Unit::MeleeDamageBonusDone`'s auto-attack percentage branch
    // (`Unit.cpp:7620-7627`), read for every white swing.
    done_total_mod *= attacker_effects
        .iter()
        .filter(|effect| {
            effect.aura_type == wow_constants::spell::aura_types::SPELL_AURA_MOD_AUTOATTACK_DAMAGE
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
                    == wow_constants::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE
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
                    == wow_constants::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE_BY_TARGET_AURA_MECHANIC
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
/// before returning it to `CalculateMeleeDamage` (`Unit.cpp:7659-7667`).
pub fn melee_damage_bonus_done_apply_like_cpp(damage: u32, bonus: (i32, f32)) -> u32 {
    let damage = (damage as f32 + bonus.0 as f32) * bonus.1;
    damage.max(0.0) as u32
}

/// C++ `Unit::MeleeDamageBonusTaken`'s `(TakenFlatBenefit, TakenTotalMod)` for a
/// white swing, already resolved from the victim's and attacker's auras by the
/// swing owner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RepresentedMeleeDamageTakenLikeCpp {
    pub flat: i32,
    pub pct: f32,
}

impl RepresentedMeleeDamageTakenLikeCpp {
    /// No represented taken modifiers: the damage passes through unchanged.
    pub const NONE: Self = Self { flat: 0, pct: 1.0 };
}

/// C++ `Unit::MeleeDamageBonusTaken` for `spellProto == null` and a melee attack
/// type (`Unit.cpp:7670-7777`): the victim's `SPELL_AURA_MOD_DAMAGE_TAKEN` sum
/// for the attacker's melee school, its `SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN` sum,
/// the `SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN` multiplier for that school, the
/// `SPELL_AURA_MOD_MELEE_DAMAGE_FROM_CASTER` multiplier restricted to auras the
/// attacker cast, and the `SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN_PCT` multiplier.
///
/// The Sanctified Wrath bypass uses the attacker's
/// `SPELL_AURA_MOD_IGNORE_TARGET_RESIST` effects covering the same school.
/// Ranged and `spellProto` branches are excluded by these represented inputs.
/// C++ can also apply the fixed cheat-death aura (45182) on this operation; it
/// has no represented input here and remains a legacy gap. The versatility term
/// is commented out in the 3.4.3 source itself.
pub fn melee_damage_taken_flat_pct_like_cpp(
    victim_effects: &[AppliedAuraEffectLikeCpp],
    attacker_ignore_resist: &[(i32, i32)],
    attacker_guid: wow_core::ObjectGuid,
    school_mask: i32,
) -> RepresentedMeleeDamageTakenLikeCpp {
    use wow_constants::spell::aura_types::{
        SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN, SPELL_AURA_MOD_DAMAGE_TAKEN,
        SPELL_AURA_MOD_MELEE_DAMAGE_FROM_CASTER, SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN,
        SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN_PCT,
    };

    let flat = victim_effects
        .iter()
        .filter(|effect| {
            effect.aura_type == SPELL_AURA_MOD_DAMAGE_TAKEN && effect.misc_value & school_mask != 0
        })
        .map(|effect| effect.amount)
        .sum::<i32>()
        + victim_effects
            .iter()
            .filter(|effect| effect.aura_type == SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN)
            .map(|effect| effect.amount)
            .sum::<i32>();

    let multiplier = |aura_type: i32, caster: Option<wow_core::ObjectGuid>| {
        victim_effects
            .iter()
            .filter(|effect| {
                effect.aura_type == aura_type
                    && caster.is_none_or(|guid| effect.caster_guid == guid)
            })
            .fold(1.0_f32, |total, effect| {
                total * (1.0 + effect.amount as f32 / 100.0)
            })
    };
    let mut pct = 1.0_f32;
    pct *= victim_effects
        .iter()
        .filter(|effect| {
            effect.aura_type == SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN
                && effect.misc_value & school_mask != 0
        })
        .fold(1.0_f32, |total, effect| {
            total * (1.0 + effect.amount as f32 / 100.0)
        });
    pct *= multiplier(SPELL_AURA_MOD_MELEE_DAMAGE_FROM_CASTER, Some(attacker_guid));
    pct *= multiplier(SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN_PCT, None);

    // C++ `Unit::MeleeDamageBonusTaken`'s Sanctified Wrath bypass: while the
    // victim's total modifier reduces damage, the attacker's
    // `SPELL_AURA_MOD_IGNORE_TARGET_RESIST` shrinks that reduction.
    if pct < 1.0 {
        let mut damage_reduction = 1.0 - pct;
        for (misc_value, amount) in attacker_ignore_resist {
            if misc_value & school_mask == 0 {
                continue;
            }
            damage_reduction *= 1.0 - *amount as f32 / 100.0;
        }
        pct = 1.0 - damage_reduction;
    }

    RepresentedMeleeDamageTakenLikeCpp { flat, pct }
}

/// C++ `Unit::MeleeDamageBonusTaken`'s tail (`Unit.cpp:7775-7777`): the flat
/// benefit is added, the total modifier multiplies and the result truncates at
/// zero. C++ returns zero before the arithmetic when the flat benefit is
/// negative enough to absorb the whole hit.
pub fn melee_damage_taken_apply_like_cpp(
    taken: RepresentedMeleeDamageTakenLikeCpp,
    damage: u32,
) -> u32 {
    if damage == 0 {
        return 0;
    }
    if taken.flat < 0 && (damage as i32) < -taken.flat {
        return 0;
    }
    (((damage as i32 + taken.flat) as f32) * taken.pct).max(0.0) as u32
}

/// C++ `Unit::CalcAbsorbResist`'s `auraAbsorbMod` (`Unit.cpp:1803-1811`):
/// select the attacker's `SPELL_AURA_MOD_TARGET_ABSORB_SCHOOL` effects matching
/// the school mask, take the maximum positive modifier and clamp it to
/// `[0, 100]`. Aura and catalog selection remain world-owned.
pub fn represented_melee_ignore_absorb_like_cpp(
    attacker_effects: &[AppliedAuraEffectLikeCpp],
    school_mask: u32,
) -> f32 {
    attacker_effects
        .iter()
        .filter(|effect| {
            effect.aura_type
                == wow_constants::spell::aura_types::SPELL_AURA_MOD_TARGET_ABSORB_SCHOOL
                && (effect.misc_value as u32) & school_mask != 0
        })
        .map(|effect| effect.amount as f32)
        .fold(0.0_f32, f32::max)
        .clamp(0.0, 100.0)
}
