//! Represented combat responsibility, separated from the
//! Session root under #617. Each submodule owns one complete operation
//! group; the canonical owners keep authority over the state they touch.

use super::*;

/// C++ `CombatRating::CR_ARMOR_PENETRATION` (`Unit.h:309`).
pub(crate) const CR_ARMOR_PENETRATION_LIKE_CPP: u8 = 24;
/// C++ `CombatRating::CR_HIT_MELEE` (`Unit.h:310`).
pub(crate) const CR_HIT_MELEE_LIKE_CPP: u8 = 5;

mod damage;
mod death;
mod melee;
mod regeneration;
mod state;
mod vitals;

pub(in crate::session) use damage::write_absorbed_shield_amount_like_cpp;
pub(in crate::session) use melee::{
    RepresentedArmorMitigationLikeCpp, RepresentedMeleeSwingLikeCpp,
};
