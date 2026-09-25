//! Session scenarios exercising the represented melee damage math (#29).
//!
//! Split out of `scenarios_combat_1.rs`; the shared fixtures stay in the parent
//! module.

use super::*;

#[path = "scenarios_combat_4/attack_table_outcomes.rs"]
mod attack_table_outcomes;
#[path = "scenarios_combat_4/avoidance_critical_and_evade.rs"]
mod avoidance_critical_and_evade;
#[path = "scenarios_combat_4/melee_damage_taken.rs"]
mod melee_damage_taken;
#[path = "scenarios_combat_4/white_swing_damage.rs"]
mod white_swing_damage;
