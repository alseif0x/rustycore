//! Original five Kill APP contracts, using explicit owner fixtures.
#![cfg(feature = "test-fixtures")]
use std::sync::Arc;
use support::*;
use wow_constants::{UnitDynFlags, UnitFlags};
use wow_core::{ObjectGuid, Position};
use wow_loot::{LootStoreKind, LootStores};
use wow_packet::packets::loot::LOOT_TYPE_CORPSE_LIKE_CPP;
use wow_social::group::{GroupInfo, GroupRegistry, PendingInvites};
use wow_world::player_directory::PlayerRegistry;
use wow_world::session::WorldSession;
use wow_world::session::creature_melee_fixtures::{
    make_session, register_test_creature, shared_map_manager, test_creature_create_data,
    test_creature_guid,
};
use wow_world::test_fixtures::creature_kill::RepresentedCreatureKillEventLikeCpp;
use wow_world::test_fixtures::loot::process_pending_for_loot_test;
use wow_world::test_fixtures::{
    canonical_map_manager_for_test, insert_player_quest_gameplay_status_for_test,
    install_canonical_player_owner_for_test, player_quest_gameplay_snapshot_for_test,
};
use wow_world::{conditions, handlers};

mod deferred_melee;
mod direct_damage;
mod fixture_routes;
mod support;
