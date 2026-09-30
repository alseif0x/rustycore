//! Actual map admission and token preparation; no token/witness constructors.
use super::*;
use wow_constants::DeathState;
use wow_core::{Position, guid::HighGuid};
use wow_entities::Creature;
use wow_map::map_manager::WorldCreature;
use wow_map::{GridCoord, Map, MapTickCoordinationStateLikeCpp};

pub(super) fn actor(counter: i64) -> WorldCreature {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 1, 0, 42, counter);
    let mut creature = Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().set_map(1, 0).unwrap();
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(10.0, 20.0, 30.0));
    creature.unit_mut().world_mut().set_active(true);
    creature.unit_mut().world_mut().object_mut().add_to_world();
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(0);
    creature.unit_mut().set_death_state(DeathState::Corpse);
    let data = WorldCreature::create_data_from_canonical_like_cpp(&creature);
    WorldCreature::from_canonical(creature, data)
}

pub(super) fn admit(manager: &mut MapManager, counter: i64) -> ObjectGuid {
    let incoming = actor(counter);
    let guid = incoming.guid();
    let position = incoming.position();
    let map = manager.find_map_mut(1, 0).unwrap().map_mut();
    map.test_fixture_admit_creature_actor(incoming);
    let cell = wow_map::map::cell_from_world(position.x, position.y);
    map.ensure_grid_loaded(&cell);
    map.get_ngrid_mut(GridCoord::new(cell.grid_x(), cell.grid_y()))
        .unwrap()
        .get_grid_type_mut(cell.cell_x(), cell.cell_y())
        .unwrap()
        .grid_objects
        .creatures
        .insert(guid);
    map.add_to_active_like_cpp(guid);
    guid
}

pub(super) fn setup(
    counter: i64,
) -> (
    MapManager,
    CanonicalObjectWork,
    ObjectMapUpdateToken,
    ObjectGuid,
    OwnedLootAuthority,
) {
    let mut manager = MapManager::new(wow_map::MIN_GRID_DELAY_MS, 200);
    manager.create_world_map(1, 0);
    let guid = admit(&mut manager, counter);
    let authority = manager
        .find_map(1, 0)
        .unwrap()
        .map()
        .with_creature_like_cpp(guid, |creature| creature.loot_authority_like_cpp().clone())
        .unwrap();
    let plan = manager.begin_tick_like_cpp(200).into_started().unwrap();
    // These cases start at the actual post-respawn boundary. The six original
    // object_work tests retain their complete begin/respawn catalog coverage.
    let object_tick = manager.begin_object_tick(plan).unwrap();
    let mut work = CanonicalObjectWork {
        object_tick,
        respawn_summary: Default::default(),
    };
    let token = work.prepare_next(&mut manager).unwrap().unwrap();
    (manager, work, token, guid, authority)
}

pub(super) fn begin(
    work: &CanonicalObjectWork,
    manager: &mut MapManager,
    token: ObjectMapUpdateToken,
    guid: ObjectGuid,
) -> ReservedCreatureLootGeneration {
    match work.begin_reserved_creature_loot(manager, token, guid) {
        Ok(operation) => operation,
        Err((error, _token)) => panic!("real reservation failed: {error:?}"),
    }
}

pub(super) fn no_record(
    _: &mut Map,
    _: wow_map::SpawnObjectType,
    _: wow_map::SpawnId,
) -> Option<wow_map::map::LoadedGridRespawnRecordsLikeCpp> {
    None
}

pub(super) fn finish(
    work: &mut CanonicalObjectWork,
    manager: &mut MapManager,
    token: ObjectMapUpdateToken,
) -> wow_map::ObjectMapFinishOutcome {
    match work.try_finish_map(manager, token, None, &mut no_record) {
        Ok(outcome) => outcome,
        Err((error, _token)) => panic!("recoverable finish rejected: {error:?}"),
    }
}

pub(super) fn dispose(
    work: &CanonicalObjectWork,
    manager: &MapManager,
    operation: ReservedCreatureLootGeneration,
) -> ObjectMapUpdateToken {
    match work.dispose_reserved_creature_loot(manager, operation) {
        Ok(token) => token,
        Err((error, _operation)) => panic!("explicit disposal rejected: {error:?}"),
    }
}

pub(super) fn pool(guid: ObjectGuid) -> CreatureLoot {
    CreatureLoot {
        loot_guid: guid,
        coins: 7,
        unlooted_count: 0,
        loot_type: 1,
        dungeon_encounter_id: 0,
        loot_method: 0,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: Vec::new(),
        items: Vec::new(),
        looted_by_player: false,
    }
}

pub(super) fn assert_pending(
    work: &CanonicalObjectWork,
    manager: &mut MapManager,
    operation: &mut ReservedCreatureLootGeneration,
    guid: ObjectGuid,
) {
    assert!(matches!(
        manager.observe_tick_creature_loot(&work.object_tick, &mut operation.token, guid),
        CreatureLootAccess::Rejected(CreatureLootAccessError::PendingActorOperation)
    ));
    assert!(manager.begin_tick_like_cpp(1).is_busy());
    assert!(matches!(
        manager.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::Resuming(_)
    ));
    assert!(
        manager
            .find_map(1, 0)
            .unwrap()
            .delayed_update_calls()
            .is_empty()
    );
}

pub(super) fn health_identity(manager: &MapManager, guid: ObjectGuid) -> (usize, u32, u64) {
    manager
        .find_map(1, 0)
        .unwrap()
        .map()
        .with_creature_like_cpp(guid, |creature| {
            (
                creature as *const _ as usize,
                creature.current_health(),
                creature.unit().health_state_revision_like_cpp(),
            )
        })
        .unwrap()
}
