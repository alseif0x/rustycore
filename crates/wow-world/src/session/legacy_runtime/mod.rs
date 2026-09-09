//! Legacy creature and player runtime tick, separated from the Session root.
//!
//! Separated from the Session root under #619. Each submodule owns one
//! complete tick responsibility.

use super::*;
mod creature_aggro_tick;
mod creature_lifecycle_tick;
mod creature_melee_tick;
mod creature_movement_tick;
mod creature_spell_tick;
mod creature_spell_validation;
mod creature_threat;
mod creature_tick;
mod player_tick;

// These re-exports look unused inside this module: they are consumed through
// the Session root glob, so removing them breaks the callers.
#[allow(unused_imports)]
pub(in crate::session) use creature_aggro_tick::*;
#[allow(unused_imports)]
pub(in crate::session) use creature_lifecycle_tick::*;
#[allow(unused_imports)]
pub(in crate::session) use creature_melee_tick::*;
#[allow(unused_imports)]
pub(in crate::session) use creature_movement_tick::*;
#[allow(unused_imports)]
pub(in crate::session) use creature_spell_tick::*;
#[allow(unused_imports)]
pub(in crate::session) use creature_spell_validation::*;
#[allow(unused_imports)]
pub(in crate::session) use creature_threat::*;
#[allow(unused_imports)]
pub(in crate::session) use creature_tick::*;
#[allow(unused_imports)]
pub(in crate::session) use player_tick::*;

// The tick entry points keep their original external path,
// `wow_world::session::run_legacy_*`, so world-server keeps calling them
// unchanged. Only these seven are public; everything else stays crate-internal.
pub use creature_aggro_tick::run_legacy_creature_aggro_tick_once_like_cpp;
pub use creature_aggro_tick::run_legacy_creature_aggro_tick_once_with_config_like_cpp;
pub use creature_lifecycle_tick::run_legacy_creature_lifecycle_tick_once_like_cpp;
pub use creature_melee_tick::run_legacy_creature_melee_tick_once_like_cpp;
pub use creature_movement_tick::run_legacy_creature_movement_tick_once_like_cpp;
pub use creature_spell_tick::run_legacy_creature_spell_tick_once_like_cpp;
pub use player_tick::run_legacy_player_melee_tick_once_like_cpp;
