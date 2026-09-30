// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Deterministic combat rules over caller-resolved values.
//!
//! This crate owns deterministic melee, mitigation, aura and spell rules over
//! caller-resolved values. Callers own canonical state, randomness, clocks,
//! catalogs and publication; this crate depends only on the foundation crates
//! `wow-core` and `wow-constants`, not data catalogs.

mod absorb;
mod armor;
mod attack_table;
mod damage;
mod effects;
mod facts;
mod melee_bonuses;
mod spell_damage;
mod spell_healing;
mod spell_power;
mod white_swing;

pub use white_swing::{ArmorMitigation, calculate_white_swing, white_swing_damage_roll_bounds};

pub use absorb::{
    RepresentedAbsorbConsumptionLikeCpp, RepresentedAbsorbShieldLikeCpp,
    RepresentedHealAbsorbLikeCpp, RepresentedHealAbsorbShieldLikeCpp,
    RepresentedManaShieldConsumptionLikeCpp, RepresentedManaShieldLikeCpp,
    RepresentedMeleeAbsorbLikeCpp, RepresentedMeleeManaAbsorbLikeCpp,
    represented_absorb_priority_like_cpp, represented_heal_absorb_like_cpp,
    represented_melee_absorb_like_cpp, represented_melee_ignored_absorb_amount_like_cpp,
    represented_melee_mana_absorb_like_cpp,
};
pub use armor::armor_reduced_damage_like_cpp;
pub use attack_table::{
    MELEE_OUTCOME_ROLL_MAX_LIKE_CPP, RepresentedMeleeOutcomeInputsLikeCpp,
    RepresentedMeleeOutcomeLikeCpp, melee_outcome_like_cpp,
};
pub use damage::{
    CREATURE_BLOCK_PERCENT_LIKE_CPP, melee_outcome_damage_like_cpp, player_block_percent_like_cpp,
};
pub use effects::AppliedAuraEffectLikeCpp;
pub use facts::{
    RepresentedMeleeAttackerFactsLikeCpp, RepresentedMeleeVictimFactsLikeCpp,
    melee_outcome_inputs_like_cpp,
};
pub use melee_bonuses::{
    RepresentedMeleeDamageTakenLikeCpp, melee_damage_bonus_done_apply_like_cpp,
    melee_damage_bonus_done_from_effects_like_cpp, melee_damage_taken_apply_like_cpp,
    melee_damage_taken_flat_pct_like_cpp, represented_melee_ignore_absorb_like_cpp,
};
pub use spell_damage::{
    SpellDamagePctDoneInputsLikeCpp, spell_damage_bonus_done_like_cpp,
    spell_damage_pct_done_like_cpp,
};
pub use spell_healing::{
    SpellHealingPctDoneInputsLikeCpp, spell_healing_bonus_done_like_cpp,
    spell_healing_bonus_from_victim_aura_effects_like_cpp, spell_healing_bonus_taken_like_cpp,
    spell_healing_pct_done_like_cpp,
};
pub use spell_power::{
    spell_advertised_coefficient_benefit_like_cpp, spell_base_damage_bonus_fallback_like_cpp,
    spell_base_healing_bonus_fallback_like_cpp, spell_bonus_coefficient_from_ap_like_cpp,
    spell_done_flat_benefit_add_ap_like_cpp, spell_power_override_from_ap_like_cpp,
};
