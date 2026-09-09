//! Represented spell-effect execution and the ticks that drive it.
//!
//! Separated from the Session root under #621.

use super::*;
mod checks;
mod combat;
mod destinations;
mod effect_apply;
mod effect_combat;
mod effect_summon;
mod effects;
mod effects_player;
mod effects_power;
mod effects_progress;
mod execution;
mod execution_overloads;
mod threat;
mod ticks;
