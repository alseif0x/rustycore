// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Receiver-free melee damage rules used by the world-session runtime.

use super::aura_effects::{
    AppliedAuraEffectLikeCpp, player_aura_effects_all_like_cpp,
    player_aura_effects_by_spell_aura_type_like_cpp,
};
pub(crate) use wow_combat::{
    armor_reduced_damage_like_cpp, melee_damage_bonus_done_apply_like_cpp,
    melee_damage_bonus_done_from_effects_like_cpp,
};

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
        player_aura_effects_all_like_cpp(attacker_auras, spell_store);
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
    let (min_damage, max_damage) =
        wow_combat::white_swing_damage_roll_bounds(min_damage, max_damage);
    wow_core::urand_like_cpp(min_damage, max_damage)
}
