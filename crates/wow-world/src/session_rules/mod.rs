// Copyright (c) 2026 alseif0x

//! Receiver-free rules moved out of the Session under #676.
//!
//! The bodies now live in `wow-world-spell`; this module keeps the original
//! `crate::session_rules::…` paths working (#1263 F5).
//!
//! The re-exports are named instead of globbed. The session-ownership checker
//! resolves a relative path such as `crate::session_rules::white_swing_roll_like_cpp`
//! through a named declaration, and a glob from another crate hides every name it
//! supplies; the names below are exactly the ones this crate consumes. The second
//! block is consumed only by this crate's own tests.

pub(crate) use wow_world_spell::aura_effects::{
    applied_aura_mechanic_mask_like_cpp, aura_application_mechanic_mask_like_cpp,
    creature_absorb_shields_like_cpp, creature_aura_effects_like_cpp,
    player_absorb_shields_like_cpp, player_aura_effects_all_like_cpp,
    player_aura_effects_by_spell_aura_type_like_cpp,
    player_aura_effects_full_by_spell_aura_type_like_cpp, player_heal_absorb_shields_like_cpp,
    player_mana_shields_like_cpp,
};
pub(crate) use wow_world_spell::melee_damage::{
    armor_reduced_damage_like_cpp, melee_damage_bonus_done_apply_like_cpp,
    melee_damage_bonus_done_from_effects_like_cpp, melee_damage_bonus_done_like_cpp,
    white_swing_roll_like_cpp,
};
pub(crate) use wow_world_spell::melee_rules::{
    CREATURE_BLOCK_PERCENT_LIKE_CPP, RepresentedAbsorbConsumptionLikeCpp,
    RepresentedMeleeAttackerFactsLikeCpp, RepresentedMeleeDamageTakenLikeCpp,
    RepresentedMeleeOutcomeLikeCpp, RepresentedMeleeVictimFactsLikeCpp,
    melee_damage_taken_apply_like_cpp, melee_damage_taken_flat_pct_like_cpp,
    melee_outcome_damage_like_cpp, melee_outcome_inputs_like_cpp,
    melee_outcome_presentation_like_cpp, player_block_percent_like_cpp,
    represented_heal_absorb_like_cpp, represented_melee_absorb_like_cpp,
    represented_melee_ignore_absorb_like_cpp, represented_melee_mana_absorb_like_cpp,
    rolled_melee_outcome_like_cpp,
};

#[cfg(test)]
pub(crate) use wow_world_spell::aura_effects::{
    AppliedAuraEffectLikeCpp, RepresentedAbsorbShieldLikeCpp, RepresentedHealAbsorbShieldLikeCpp,
    RepresentedManaShieldLikeCpp,
};
#[cfg(test)]
pub(crate) use wow_world_spell::melee_rules::{
    represented_absorb_priority_like_cpp, represented_melee_ignored_absorb_amount_like_cpp,
};
