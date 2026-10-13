//! Represented damage, heal and taunt effect application.
//!
//! Moved out of the Session root under #621. Behaviour is preserved.
//! Application methods live in private damage/combat and healing children;
//! shared damage, healing and target projections remain in this parent module.

use super::*;

#[path = "effect_combat/damage_and_combat_application.rs"]
mod damage_and_combat_application;
