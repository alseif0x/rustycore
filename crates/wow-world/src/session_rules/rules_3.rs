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
use wow_entities::AuraApplicationLikeCpp;

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

/// C++ `Unit::MeleeDamageBonusDone`'s victim-creature-type terms
/// (`Unit.cpp:7558-7650`) for the represented attacker: the flat
/// `SPELL_AURA_MOD_DAMAGE_DONE_CREATURE` benefit, the
/// `SPELL_AURA_MOD_MELEE_ATTACK_POWER_VERSUS` bonus converted with
/// `GetAPMultiplier`, and the `SPELL_AURA_MOD_DAMAGE_DONE_VERSUS` multiplier.
/// `(DoneFlatBenefit, DoneTotalMod)`.
///
/// Boundary: the victim's `SPELL_AURA_MELEE_ATTACK_POWER_ATTACKER_BONUS`
/// (165) / `SPELL_AURA_RANGED_ATTACK_POWER_ATTACKER_BONUS` (127) term has no
/// represented creature-aura producer, so only the attacker's side is folded.
pub(crate) fn melee_damage_bonus_done_creature_type_like_cpp(
    attacker_auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
    creature_type_mask: u32,
    is_ranged: bool,
    attack_power_multiplier: f32,
) -> (i32, f32) {
    if creature_type_mask == 0 {
        return (0, 1.0);
    }
    let matches = |misc_value: i32| misc_value & creature_type_mask as i32 != 0;
    let mut flat = player_aura_effects_by_spell_aura_type_like_cpp(
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
    let done_total_mod = player_aura_effects_by_spell_aura_type_like_cpp(
        attacker_auras,
        spell_store,
        wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS,
    )
    .into_iter()
    .filter(|(misc_value, _)| matches(*misc_value))
    .fold(1.0_f32, |total, (_, amount)| {
        total * (1.0 + amount as f32 / 100.0)
    });
    (flat, done_total_mod)
}
