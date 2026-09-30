use super::*;

pub(super) fn actor(counter: i64, spawn_id: u64) -> WorldCreature {
    let mut creature = Creature::new(false);
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 7, 42, counter);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().set_map(571, 7).unwrap();
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(1.0, 2.0, 3.0));
    creature.unit_mut().world_mut().object_mut().add_to_world();
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.set_spawn_id(spawn_id);
    creature.set_respawn_compatibility_mode(false);
    creature.set_respawn_time(0);
    let data = WorldCreature::create_data_from_canonical_like_cpp(&creature);
    WorldCreature::from_canonical(creature, data)
}

pub(super) fn dead(actor: &mut WorldCreature, state: DeathState, save: bool) {
    actor.creature.unit_mut().set_health(0);
    actor.creature.unit_mut().set_death_state(state);
    actor.creature.runtime_state_mut().save_respawn_requested = save;
    actor.creature.ai_ownership_mut().death_time_ms = Some(0);
    actor.creature.ai_ownership_mut().respawn_time_secs = 10;
    actor.creature.set_ai_corpse_despawn_at(Some(5_000));
    actor.advance_runtime_clock_like_cpp(5_000);
}

pub(super) fn pair(counter: i64, spawn_id: u64) -> (Map, LegacyMapManager, ObjectGuid) {
    let mut map = Map::new(571, 7, 0, 1000);
    let mut legacy = LegacyMapManager::new();
    let incoming = actor(counter, spawn_id);
    let guid = incoming.guid();
    map.admit_creature_actor(incoming).unwrap();
    let incoming = actor(counter, spawn_id);
    let (x, y) = world_to_grid_coords(incoming.position().x, incoming.position().y);
    assert!(legacy.add_creature(571, 7, x, y, incoming));
    (map, legacy, guid)
}

pub(super) fn clocks() -> Clocks {
    let conversion_now = Instant::now();
    Clocks {
        now: conversion_now - Duration::from_secs(9),
        conversion_now,
        conversion_now_secs: 100,
    }
}

pub(super) fn canonical(
    map: &mut Map,
    clocks: Clocks,
    persistent: bool,
) -> (ActorRespawnPhaseOutcome, VecDeque<PendingRespawn>) {
    map.prepare_actor_respawns(
        clocks.now,
        clocks.conversion_now,
        clocks.conversion_now_secs,
        persistent,
    )
}

pub(super) fn run_legacy_prefix(
    manager: &mut LegacyMapManager,
    clocks: Clocks,
    persistent: bool,
) -> LegacyCreatureRespawnPrefix {
    manager.prepare_creature_respawns(
        571,
        7,
        clocks.now,
        clocks.conversion_now,
        clocks.conversion_now_secs,
        persistent,
    )
}

pub(super) fn assert_save_only(mutations: &[RespawnPersistenceMutationLikeCpp], seconds: i64) {
    assert_eq!(mutations.len(), 1);
    assert!(matches!(mutations[0],
        RespawnPersistenceMutationLikeCpp::Save { respawn_time, .. } if respawn_time == seconds));
}
