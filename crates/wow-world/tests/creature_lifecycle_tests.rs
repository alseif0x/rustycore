//! Original lifecycle APP contracts against the real legacy/canonical operations.
#![cfg(feature = "test-fixtures")]
use std::sync::Arc;
use std::time::{Duration, Instant};
use support::*;
use wow_core::guid::HighGuid;
use wow_core::{ObjectGuid, Position};
use wow_entities::PhaseShift;
use wow_packet::opcodes::ServerOpcodes;
use wow_world::map_manager;
use wow_world::session::creature_melee_fixtures::{
    make_session, register_test_creature, shared_canonical_map_manager, shared_map_manager,
    test_creature_create_data, test_creature_guid,
};
use wow_world::session::{WorldSession, run_legacy_creature_lifecycle_tick_once_like_cpp};
#[path = "creature_lifecycle_tests/death_and_corpse.rs"]
mod death_and_corpse;
#[path = "creature_lifecycle_tests/fixture_routes.rs"]
mod fixture_routes;
#[path = "creature_lifecycle_tests/ready_respawn.rs"]
mod ready_respawn;
#[path = "creature_lifecycle_tests/session_queue.rs"]
mod session_queue;
#[path = "creature_lifecycle_tests/support.rs"]
mod support;
