//! Represented duels and their lifecycle.
//!
//! Moved out of the Session root under #615. The duel spell effect and its
//! duel-state steps moved to `wow_world_social::apply_duel_effect_like_cpp`
//! (#1363 slice 12b5); only the test shims stay mounted here.

#[cfg(test)]
#[path = "../../../unit_tests/session/social/duel/f3_shims.rs"]
mod f3_shims;
