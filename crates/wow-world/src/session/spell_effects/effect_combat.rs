//! Represented damage, heal and taunt effect application.
//!
//! Moved out of the Session root under #621. Behaviour is preserved.
//! The damage/healing projections, healing and environmental damage moved to
//! `wow_world_spell` (#1363 slice 9c); the creature spell damage with its kill
//! cascade and the taunt stay in the private damage/combat child.

use super::*;

#[path = "effect_combat/damage_and_combat_application.rs"]
mod damage_and_combat_application;
