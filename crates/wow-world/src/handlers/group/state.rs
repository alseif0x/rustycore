//! Group handlers state facade.
//!
//! The session-independent group projections, fan-out and packet builders now
//! live in `wow-world-social::group_fanout` (#1263 F5); this facade keeps the
//! original `state::…` paths working and holds the two helpers that need the
//! `WorldSession` itself. Behaviour is preserved.

use super::*;
pub(crate) use wow_world_lifecycle::group_persistence_command_like_cpp;

pub(super) use wow_world_social::group_fanout::*;
