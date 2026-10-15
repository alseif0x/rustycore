//! Represented spell-effect execution and the ticks that drive it.
//!
//! Separated from the Session root under #621.

use super::*;
mod destinations;
mod effect_combat;
mod effect_host;
mod effects;
#[cfg(test)]
#[path = "../../../unit_tests/session/spell_effects/effects_player/test_only.rs"]
mod effects_player_test_only;
mod effects_progress;
mod execution_host;
