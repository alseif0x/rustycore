//! Represented combat responsibility, separated from the
//! Session root under #617. Each submodule owns one complete operation
//! group; the canonical owners keep authority over the state they touch.

use super::*;
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
