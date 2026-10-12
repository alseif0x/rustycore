//! Represented combat responsibility, separated from the
//! Session root under #617. Each submodule owns one complete operation
//! group; the canonical owners keep authority over the state they touch.

use super::*;

pub(crate) use wow_world_inventory::CR_ARMOR_PENETRATION_LIKE_CPP;

mod damage;
mod death;
mod melee;
mod regeneration;
mod state;
mod vitals;

pub(crate) use melee::PlayerAttackStartLikeCppResult;
#[cfg(test)]
pub(in crate::session) use wow_world_creature::RepresentedArmorMitigationLikeCpp;
pub(in crate::session) use wow_world_spell::aura_effects::write_absorbed_shield_amount_like_cpp;
