//! Represented item bonuses, modifiers and item-set effects.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

mod catalog;
mod runtime_access;
mod item_sets;
mod bonus_planning;
mod loaded_replay;
mod stat_publication;
