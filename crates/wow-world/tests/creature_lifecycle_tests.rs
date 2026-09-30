//! Original lifecycle APP contracts against the real legacy/canonical operations.
#![cfg(feature = "test-fixtures")]
use std::time::{Duration, Instant};
use std::sync::Arc;
use wow_core::guid::HighGuid;
use wow_core::{ObjectGuid, Position};
use wow_entities::PhaseShift;
use wow_packet::opcodes::ServerOpcodes;
use wow_world::map_manager;
use wow_world::session::{WorldSession, run_legacy_creature_lifecycle_tick_once_like_cpp};
use wow_world::session::creature_melee_fixtures::{
    make_session, shared_map_manager, shared_canonical_map_manager,
    test_creature_guid, register_test_creature, test_creature_create_data,
};
use support::*;
#[path = "creature_lifecycle_tests/support.rs"]
mod support;
#[path = "creature_lifecycle_tests/death_and_corpse.rs"]
mod death_and_corpse;
#[path = "creature_lifecycle_tests/ready_respawn.rs"]
mod ready_respawn;
#[path = "creature_lifecycle_tests/session_queue.rs"]
mod session_queue;
#[path = "creature_lifecycle_tests/fixture_routes.rs"]
mod fixture_routes;
