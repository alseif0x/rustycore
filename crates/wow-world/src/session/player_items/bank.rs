//! Represented bank storage slots.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {}

#[cfg(test)]
#[path = "../../../unit_tests/session/player_items/bank/f3_shims.rs"]
mod f3_shims;
