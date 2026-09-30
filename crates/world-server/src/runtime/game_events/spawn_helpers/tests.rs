use super::*;
use crate::spawn_store_loader;
use std::{collections::BTreeMap, sync::Arc};
use wow_constants::{ConditionSourceType, ConditionType};
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_data::Condition;
use wow_entities::{Creature, Player};
use wow_map::spawn::{
    SpawnData, SpawnGroupFlags, SpawnGroupMemberRow, SpawnGroupTemplateData, SpawnObjectType,
    SpawnPosition, SpawnStore,
};

// Reuse the existing DB-backed fixtures verbatim without exposing test APIs or
// reconstructing a second factory. Their original mount and tests are intact.
mod admission;
mod factory;
#[path = "../../../main_tests/loaded_grid_fixtures.rs"]
mod fixtures;
// Exercise the same Catalog adapter source without widening its private parent
// module for a sibling test. Its production mount is a parent-owned snippet.
#[path = "../../map_tick/respawn_catalog/prepared.rs"]
mod catalog_prepared;

fn creature_record(counter: i64) -> MapObjectRecord {
    let mut creature = Creature::new(false);
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 7, 42, counter);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().set_map(571, 7).unwrap();
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(1.0, 2.0, 3.0));
    creature.unit_mut().set_max_health(321);
    creature.unit_mut().set_health(123);
    creature.set_spawn_id(counter as u64 * 10);
    MapObjectRecord::new_creature(creature).unwrap()
}

fn player_record(counter: i64, map_id: u32) -> MapObjectRecord {
    let mut player = Player::new(None, false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(ObjectGuid::create_player(1, counter));
    player.unit_mut().world_mut().set_map(map_id, 7).unwrap();
    player
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(1.0, 2.0, 3.0));
    MapObjectRecord::new_player(player).unwrap()
}

fn prepared(counter: i64) -> PreparedLoadedGridCreature {
    prepare_loaded_grid_creature(
        LoadedGridRespawnRecordsLikeCpp::primary_only(creature_record(counter)),
        &WaypointPathStoreLikeCpp::default(),
    )
    .unwrap()
}
