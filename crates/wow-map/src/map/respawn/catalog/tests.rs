use super::*;
use crate::map::loaded_grid_admission::{LoadedGridAttemptPlan, LoadedGridSpawnAttemptResult};
use crate::map_manager::WorldCreature;
use crate::pool::{PoolGroupLikeCpp, PoolTemplateDataLikeCpp};
use crate::spawn::{SpawnData, SpawnPosition};
use wow_core::{Position, guid::HighGuid};
use wow_entities::{AppliedAuraRef, Player, SPELL_AURA_INTERRUPT_FLAG_ENTER_WORLD_LIKE_CPP};

mod oracle;
mod record;
mod failures;
mod pooled;
mod gates;
mod actors;
mod mixed_queue;

fn map(loaded: bool) -> Map {
    let mut map = Map::new(571, 7, 1, 1000);
    if loaded { map.load_grid(1.0, 2.0); }
    map
}

fn metadata(kind: SpawnObjectType, id: SpawnId) -> SpawnData {
    SpawnData {
        object_type: kind, spawn_id: id, map_id: 571, db_data: true,
        spawn_group: SpawnGroupTemplateData::default_group(), id: 42,
        spawn_point: SpawnPosition::new(1.0, 2.0, 3.0, 0.0),
        phase_use_flags: 0, phase_id: 0, phase_group: 0, terrain_swap_map: 0,
        pool_id: 0, spawn_time_secs: 0, spawn_difficulties: vec![1],
        script_id: 0, string_id: String::new(),
    }
}

fn store(entries: &[(SpawnObjectType, SpawnId)]) -> SpawnStore {
    let mut store = SpawnStore::new();
    for (kind, id) in entries { store.add_object_spawn(&metadata(*kind, *id), |_| false); }
    store
}

fn guid(kind: HighGuid, id: SpawnId) -> ObjectGuid {
    ObjectGuid::create_world_object(kind, 0, 1, 571, 7, 42, id as i64)
}

fn creature(id: SpawnId, map_id: u32) -> Creature {
    let mut creature = Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid(HighGuid::Creature, id));
    creature.unit_mut().world_mut().set_map(map_id, 7).unwrap();
    creature.unit_mut().world_mut().relocate(Position::xyz(1.0, 2.0, 3.0));
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(75);
    creature.set_spawn_id(id);
    creature
}

fn record(kind: SpawnObjectType, id: SpawnId, map_id: u32) -> MapObjectRecord {
    match kind {
        SpawnObjectType::Creature => MapObjectRecord::new_creature(creature(id, map_id)).unwrap(),
        SpawnObjectType::GameObject => {
            let mut object = wow_entities::GameObject::new();
            object.world_mut().object_mut().create(guid(HighGuid::GameObject, id));
            object.world_mut().object_mut().set_entry(42);
            object.world_mut().set_map(map_id, 7).unwrap();
            object.world_mut().relocate(Position::xyz(1.0, 2.0, 3.0));
            object.set_spawn_id(id);
            MapObjectRecord::new_game_object(object).unwrap()
        }
        SpawnObjectType::AreaTrigger => panic!("Catalog rejects unsupported metadata before load"),
    }
}

fn player(id: i64, map_id: u32) -> MapObjectRecord {
    let mut player = Player::new(None, false);
    player.unit_mut().world_mut().object_mut().create(ObjectGuid::create_player(1, id));
    player.unit_mut().world_mut().set_map(map_id, 7).unwrap();
    player.unit_mut().world_mut().relocate(Position::xyz(1.0, 2.0, 3.0));
    MapObjectRecord::new_player(player).unwrap()
}

fn timer(map: &mut Map, kind: SpawnObjectType, id: SpawnId, time: i64) {
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: kind, spawn_id: id, entry: 42, respawn_time: time,
        grid_id: crate::compute_grid_coord(1.0, 2.0).get_id(),
    });
}

fn run_owned<L>(map: &mut Map, store: &SpawnStore, pools: &PoolMgrLikeCpp, consume: bool, load: L) -> LoadedGridRespawnOutcome
where L: FnMut(&mut Map, SpawnObjectType, SpawnId) -> Result<Option<LoadedGridMaterialization>, LoadedGridRespawnRecordsLikeCpp> {
    map.process_due_respawns_materialized(100, store, &LinkedRespawnStoreLikeCpp::new(), pools,
        5, false, |_, _| false, |_, _| 0.0, |_, count| (0..count).collect(), consume, load)
}

fn run_record<L>(map: &mut Map, store: &SpawnStore, pools: &PoolMgrLikeCpp, consume: bool, load: L) -> ProcessRespawnsSafeSideEffectsSummaryLikeCpp
where L: FnMut(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp> {
    map.process_due_respawns_composite_loaded_grid_respawns_like_cpp(100, store, &LinkedRespawnStoreLikeCpp::new(), pools,
        5, false, |_, _| false, |_, _| 0.0, |_, count| (0..count).collect(), consume, load)
}

fn run_original<L>(map: &mut Map, store: &SpawnStore, pools: &PoolMgrLikeCpp, consume: bool, load: L) -> ProcessRespawnsSafeSideEffectsSummaryLikeCpp
where L: FnMut(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp> {
    map.original_catalog(100, store, &LinkedRespawnStoreLikeCpp::new(), pools,
        5, false, |_, _| false, |_, _| 0.0, |_, count| (0..count).collect(), consume, load)
}

fn catalog_plan(plan: &LoadedGridAttemptPlan) -> (SpawnObjectType, SpawnId) {
    match plan {
        LoadedGridAttemptPlan::Catalog { object_type, spawn_id } => (*object_type, *spawn_id),
        LoadedGridAttemptPlan::Pool(_) | LoadedGridAttemptPlan::SpawnGroup(_) => panic!("nonpool provenance"),
    }
}

fn assert_summary(mut actual: ProcessRespawnsSafeSideEffectsSummaryLikeCpp, mut old: ProcessRespawnsSafeSideEffectsSummaryLikeCpp) {
    let records = std::mem::take(&mut actual.loaded_grid_primary_records);
    let previous = std::mem::take(&mut old.loaded_grid_primary_records);
    assert_eq!(actual, old);
    assert_eq!(records.len(), previous.len());
    for (record, old) in records.into_iter().zip(previous) {
        assert_eq!(record.kind(), old.kind());
        assert_eq!(record.object().guid(), old.object().guid());
        assert_eq!(record.object().position(), old.object().position());
        assert_eq!(record.object().object().is_in_world(), old.object().object().is_in_world());
        if let (Some(record), Some(old)) = (record.creature(), old.creature()) {
            assert_eq!(record.current_health(), old.current_health());
            assert_eq!(record.unit().subsystems().auras.has_applied(aura()), old.unit().subsystems().auras.has_applied(aura()));
        }
    }
}

fn aura() -> AppliedAuraRef {
    AppliedAuraRef::new(47_510, ObjectGuid::create_player(1, 99), 1, 1)
}

fn saved_actor_timer(map: &mut Map, id: SpawnId, time: i64) {
    let creature = creature(id, 571);
    let data = WorldCreature::create_data_from_canonical_like_cpp(&creature);
    let actor = WorldCreature::from_canonical(creature, data);
    let now = std::time::Instant::now();
    let pending = crate::map_manager::pending_respawn_from_world_creature_like_cpp(&actor, now, 571);
    map.respawn_store.save_actor_row(&pending, 571, 7, now, time);
    map.respawn_store.queue_actor(pending).unwrap();
}
