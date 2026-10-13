// Copyright (c) 2026 alseif0x

//! Receiver-free rules moved out of the Session under #676.
//!
//! The bodies now live in `wow-world-spell`; this module keeps the original
//! `crate::session_rules::…` paths working (#1263 F5).
//!
//! The re-exports are named instead of globbed. The session-ownership checker
//! resolves a relative path such as `crate::session_rules::applied_aura_mechanic_mask_like_cpp`
//! through a named declaration, and a glob from another crate hides every name it
//! supplies; the names below are exactly the ones this crate consumes. Since
//! #1363 slice 9c moved the spell-effect bodies to `wow-world-spell`, every
//! block is consumed only by this crate's own tests.

#[cfg(test)]
pub(crate) use wow_world_spell::aura_effects::{
    AppliedAuraEffectLikeCpp, RepresentedAbsorbShieldLikeCpp, RepresentedHealAbsorbShieldLikeCpp,
    RepresentedManaShieldLikeCpp,
};
#[cfg(test)]
pub(crate) use wow_world_spell::melee_rules::{
    represented_absorb_priority_like_cpp, represented_heal_absorb_like_cpp,
    represented_melee_ignored_absorb_amount_like_cpp,
};
// #1363 slice 3: the creature runtime moved to `wow-world-creature`; these
// names are now consumed only by this crate's tests.
#[cfg(test)]
pub(crate) use wow_world_spell::aura_effects::creature_aura_effects_like_cpp;
#[cfg(test)]
pub(crate) use wow_world_spell::melee_damage::{
    armor_reduced_damage_like_cpp, melee_damage_bonus_done_like_cpp, white_swing_roll_like_cpp,
};
#[cfg(test)]
pub(crate) use wow_world_spell::melee_rules::{
    RepresentedMeleeAttackerFactsLikeCpp, RepresentedMeleeDamageTakenLikeCpp,
    RepresentedMeleeOutcomeLikeCpp, RepresentedMeleeVictimFactsLikeCpp,
    melee_damage_taken_apply_like_cpp, melee_damage_taken_flat_pct_like_cpp,
    melee_outcome_damage_like_cpp, melee_outcome_inputs_like_cpp,
    melee_outcome_presentation_like_cpp, represented_melee_absorb_like_cpp,
    represented_melee_ignore_absorb_like_cpp, represented_melee_mana_absorb_like_cpp,
};
