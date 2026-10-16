//! Represented combat responsibility, separated from the
//! Session root under #617. Each submodule owns one complete operation
//! group; the canonical owners keep authority over the state they touch.

use super::*;

pub(crate) use wow_world_inventory::CR_ARMOR_PENETRATION_LIKE_CPP;

mod damage;
mod host;
#[cfg(test)]
mod melee;
mod regeneration;
mod state;
mod vitals;

#[cfg(test)]
pub(in crate::session) use wow_world_creature::RepresentedArmorMitigationLikeCpp;
