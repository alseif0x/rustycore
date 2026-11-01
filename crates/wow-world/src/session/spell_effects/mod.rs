//! Represented spell-effect execution and the ticks that drive it.
//!
//! Separated from the Session root under #621. The reputation and
//! force-deselect effects moved to `wow_world_social` (#1363 slice 12b3).

use super::*;
mod effect_host;
#[cfg(test)]
#[path = "../../../unit_tests/session/spell_effects/effects_player/test_only.rs"]
mod effects_player_test_only;
mod execution_host;
