//! Runtime update loops and event delivery.

use super::*;

// #1263 F6-8D3b-1: the `RuntimeTickOwner::CanonicalMap` creature tick owner.
pub(crate) mod canonical_creature_runtime;
pub(super) use canonical_creature_runtime::{
    CanonicalCreatureRuntimeLikeCpp, creature_tick_owner_loops_like_cpp,
};
mod deferred_visibility;
mod delivery;
mod game_events;
// #1263 F6-8D2: the isolated admitted creature execution adapter. Deliberately
// not re-exported into this module's glob: `lib.rs` re-exports its two entries
// as composition API, and a glob here would collide with the private
// `use runtime::*` at the crate root.
pub(crate) mod isolated_creature_execution;
mod map;
mod map_session_pass;
pub(crate) mod map_tick;
mod tick_summary;
mod world_session_pass;

#[cfg(test)]
pub(crate) use world_session_pass::run_world_phase_session_passes_like_cpp;

pub(super) use delivery::*;
pub(crate) use game_events::{
    bootstrap::*, consume::*, grid::*, live::*, scheduler::*, spawn::*, unspawn::*,
};
pub(super) use map::*;
pub(crate) use map_tick::*;
pub(crate) use tick_summary::*;
