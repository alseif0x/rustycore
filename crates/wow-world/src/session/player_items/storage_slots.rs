//! Slot resolution for represented inventory storage: free space, position search and fit checks.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {}

#[cfg(test)]
#[path = "../../../unit_tests/session/player_items/storage_slots/f3_shims.rs"]
mod f3_shims;
