//! Represented inventory storage: the complete store, remove and swap operations over the canonical Player item slots.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;
use crate::session::CR_ARMOR_PENETRATION_LIKE_CPP;
use wow_entities::ItemObjectUpdateLikeCpp;

mod moves;
mod item_objects;
mod owner_access;
mod placement_effects;
