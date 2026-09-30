//! Persistence scenarios for [`super`].
//!
//! Split out of character_tests.rs under #628; remaining assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[path = "persistence/resource_regeneration.rs"]
mod resource_regeneration;
