// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Receiver-free Session rules, part 3.
//!
//! The aura-effect projection lives here so the session adapter and the
//! map-owned legacy runtime can resolve the same canonical Player auras with
//! one implementation instead of drifting apart.

use crate::session::*;
use std::collections::HashMap;
use wow_data::SpellStore;
use wow_entities::{AppliedAuraRef, AuraApplicationLikeCpp};

/// C++ `Unit::GetAuraEffectsByType(auraType)` for one Unit's applied auras:
/// every active effect of the requested aura type as `(MiscValue, amount)`.
///
/// Auras are visited in ascending slot order so the projection is deterministic
/// (C++ walks its aura list in application order); the amount prefers the
/// represented `AuraEffect` value and falls back to the spell effect's
/// no-caster calculation, exactly like the session adapter used to do.
pub(crate) fn player_aura_effects_by_spell_aura_type_like_cpp(
    auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
    aura_type: i32,
) -> Vec<(i32, i32)> {
    let mut slots: Vec<u8> = auras.keys().copied().collect();
    slots.sort_unstable();
    let mut effects = Vec::new();
    for slot in slots {
        let aura = &auras[&slot];
        let Some(spell) = spell_store.get(aura.spell_id) else {
            continue;
        };
        for effect in spell.effects().iter().filter(|effect| {
            effect.effect_aura == aura_type
                && 1u32
                    .checked_shl(effect.effect_index)
                    .is_some_and(|bit| aura.effect_mask & bit != 0)
        }) {
            let amount = aura
                .represented_effect_amounts
                .iter()
                .find(|represented| {
                    u8::try_from(effect.effect_index).ok() == Some(represented.effect_index)
                })
                .map(|represented| represented.amount)
                .unwrap_or_else(|| effect.calc_value_no_caster_like_cpp());
            effects.push((effect.effect_misc_value_1, amount));
        }
    }
    effects
}

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

/// C++ `Unit::MeleeDamageBonusDone`'s victim-state terms
/// (`Unit.cpp:7558-7650`) for the represented attacker: the flat
/// `SPELL_AURA_MOD_DAMAGE_DONE_CREATURE` benefit, the
/// `SPELL_AURA_MOD_MELEE_ATTACK_POWER_VERSUS` bonus converted with
/// `GetAPMultiplier`, the `SPELL_AURA_MOD_DAMAGE_DONE_VERSUS` multiplier, plus
/// the victim's `HasAuraState` (`SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE`)
/// and `HasAuraWithMechanic`
/// (`SPELL_AURA_MOD_DAMAGE_PERCENT_DONE_BY_TARGET_AURA_MECHANIC`) multipliers.
/// `(DoneFlatBenefit, DoneTotalMod)`.
///
/// Boundary: the victim's `SPELL_AURA_MELEE_ATTACK_POWER_ATTACKER_BONUS`
/// (165) / `SPELL_AURA_RANGED_ATTACK_POWER_ATTACKER_BONUS` (127) term has no
/// represented creature-aura producer, so only the attacker's side is folded.
pub(crate) fn melee_damage_bonus_done_like_cpp(
    attacker_auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
    creature_type_mask: u32,
    victim_aura_state_mask: u32,
    victim_mechanic_mask: u64,
    is_ranged: bool,
    attack_power_multiplier: f32,
) -> (i32, f32) {
    let matches = |misc_value: i32| misc_value & creature_type_mask as i32 != 0;
    let mut flat = 0_i32;
    if creature_type_mask != 0 {
        flat = player_aura_effects_by_spell_aura_type_like_cpp(
            attacker_auras,
            spell_store,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_CREATURE,
        )
        .into_iter()
        .filter(|(misc_value, _)| matches(*misc_value))
        .map(|(_, amount)| amount)
        .sum::<i32>();
        let versus_aura_type = if is_ranged {
            wow_data::spell::aura_types::SPELL_AURA_MOD_RANGED_ATTACK_POWER_VERSUS
        } else {
            wow_data::spell::aura_types::SPELL_AURA_MOD_MELEE_ATTACK_POWER_VERSUS
        };
        let ap_bonus = player_aura_effects_by_spell_aura_type_like_cpp(
            attacker_auras,
            spell_store,
            versus_aura_type,
        )
        .into_iter()
        .filter(|(misc_value, _)| matches(*misc_value))
        .map(|(_, amount)| amount)
        .sum::<i32>();
        if ap_bonus != 0 {
            flat = flat.saturating_add((ap_bonus as f32 / 3.5 * attack_power_multiplier) as i32);
        }
    }
    let mut done_total_mod = 1.0_f32;
    if creature_type_mask != 0 {
        done_total_mod *= player_aura_effects_by_spell_aura_type_like_cpp(
            attacker_auras,
            spell_store,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS,
        )
        .into_iter()
        .filter(|(misc_value, _)| matches(*misc_value))
        .fold(1.0_f32, |total, (_, amount)| {
            total * (1.0 + amount as f32 / 100.0)
        });
    }
    // C++ `Unit::MeleeDamageBonusDone`'s "bonus against aurastate"
    // (`Unit.cpp:7634-7640`).
    if victim_aura_state_mask != 0 {
        done_total_mod *= player_aura_effects_by_spell_aura_type_like_cpp(
            attacker_auras,
            spell_store,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE,
        )
        .into_iter()
        .filter(|(misc_value, _)| {
            u32::try_from(*misc_value)
                .ok()
                .and_then(|state| state.checked_sub(1))
                .and_then(|bit| 1_u32.checked_shl(bit))
                .is_some_and(|bit| victim_aura_state_mask & bit != 0)
        })
        .fold(1.0_f32, |total, (_, amount)| {
            total * (1.0 + amount as f32 / 100.0)
        });
    }
    // C++ `Unit::MeleeDamageBonusDone`'s "bonus against target aura mechanic"
    // (`Unit.cpp:7642-7648`).
    if victim_mechanic_mask != 0 {
        done_total_mod *= player_aura_effects_by_spell_aura_type_like_cpp(
            attacker_auras,
            spell_store,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE_BY_TARGET_AURA_MECHANIC,
        )
        .into_iter()
        .filter(|(misc_value, _)| {
            (1..64).contains(misc_value) && victim_mechanic_mask & (1_u64 << *misc_value) != 0
        })
        .fold(1.0_f32, |total, (_, amount)| {
            total * (1.0 + amount as f32 / 100.0)
        });
    }
    (flat, done_total_mod)
}

/// C++ `Unit::HasAuraWithMechanic` (`Unit.cpp:4714-4729`) over a canonical
/// Player's `AuraApplication` map: the union of each application's
/// `SpellInfo::Mechanic` and its applied `IsEffect()` slots' mechanics, read at
/// the application's own difficulty.
pub(crate) fn aura_application_mechanic_mask_like_cpp(
    auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
    difficulty_store: Option<&wow_data::DifficultyStore>,
) -> u64 {
    auras.values().fold(0_u64, |mask, aura| {
        mask | spell_mechanic_mask_like_cpp(
            spell_store,
            difficulty_store,
            aura.spell_id,
            aura.difficulty_id,
            aura.effect_mask,
        )
    })
}

/// C++ `Unit::HasAuraWithMechanic` over a creature's `AppliedAuraRef` list.
/// Those refs do not retain a difficulty, so the caller supplies the one it
/// resolves the rest of the victim's state with.
pub(crate) fn applied_aura_mechanic_mask_like_cpp(
    applied_auras: &[AppliedAuraRef],
    spell_store: &SpellStore,
    difficulty_id: u8,
    difficulty_store: Option<&wow_data::DifficultyStore>,
) -> u64 {
    applied_auras.iter().fold(0_u64, |mask, aura| {
        mask | spell_mechanic_mask_like_cpp(
            spell_store,
            difficulty_store,
            i32::try_from(aura.spell_id).unwrap_or(0),
            difficulty_id,
            aura.effect_mask,
        )
    })
}

/// The mechanic bits one `AuraApplication`/`AppliedAuraRef` contributes: its
/// spell's own `Mechanic` plus every applied `IsEffect()` slot that carries one.
fn spell_mechanic_mask_like_cpp(
    spell_store: &SpellStore,
    difficulty_store: Option<&wow_data::DifficultyStore>,
    spell_id: i32,
    difficulty_id: u8,
    effect_mask: u32,
) -> u64 {
    let Some(metadata) =
        spell_store.hit_metadata_for_difficulty_like_cpp(spell_id, difficulty_id, difficulty_store)
    else {
        return 0;
    };
    let mut mask = mechanic_bit_like_cpp(i32::from(metadata.spell_mechanic)).unwrap_or(0);
    let effects =
        spell_store.effects_for_difficulty_like_cpp(spell_id, difficulty_id, difficulty_store);
    for (effect_index, mechanic) in metadata.effect_mechanics {
        if !(1..32).contains(&effect_index) || effect_mask & (1_u32 << effect_index) == 0 {
            continue;
        }
        let is_effect = effects.is_some_and(|effects| {
            effects
                .iter()
                .any(|effect| effect.effect_index == effect_index && effect.effect != 0)
        });
        if is_effect {
            mask |= mechanic_bit_like_cpp(mechanic).unwrap_or(0);
        }
    }
    mask
}

/// C++ `UI64LIT(1) << mechanic` for a positive mechanic; `None` for the unset
/// or out-of-range value a malformed row could carry.
fn mechanic_bit_like_cpp(mechanic: i32) -> Option<u64> {
    (1..64).contains(&mechanic).then(|| 1_u64 << mechanic)
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
