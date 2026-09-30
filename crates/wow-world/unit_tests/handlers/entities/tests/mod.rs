//! Corpse, gameobject and player handler regression scenarios.
//!
//! Shared entries/packets/session fixtures live in
//! [`crate::handlers::test_support`]; each scenario module below exercises the
//! production module next to it through `use super::*;`.

use super::*;
use crate::handlers::test_support::*;

mod corpse;
mod gameobject;
mod player;
