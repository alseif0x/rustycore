//! Battle-pet packets and updates published to the client.
//!
//! Moved out of the Session root under #607. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {}

#[cfg(test)]
#[path = "../../../unit_tests/session/pets/battle_pet_publication/f3_shims.rs"]
mod f3_shims;
