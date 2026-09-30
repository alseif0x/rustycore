//! Represented item enchantment state and its loaded effects.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

mod catalog;
mod requirements;
mod plans;
mod runtime_effects;
mod duration_persistence;
