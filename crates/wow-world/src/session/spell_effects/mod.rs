//! Represented spell-effect execution and the ticks that drive it.
//!
//! Separated from the Session root under #621.

use super::*;
use wow_world_spell::represented_homebind_like_cpp;
mod destinations;
mod effect_apply;
mod effect_combat;
mod effect_host;
mod effects;
mod effects_player;
mod effects_progress;
mod execution_host;
