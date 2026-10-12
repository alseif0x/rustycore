//! Per-type represented gameobject use handlers.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {}

#[cfg(test)]
#[path = "../../../unit_tests/session/world_entities/gameobject_use_types/f3_shims.rs"]
mod f3_shims;
