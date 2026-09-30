use super::*;
use crate::map::loaded_grid_admission::{LoadedGridAttemptPlan, LoadedGridSpawnAttemptResult};
use crate::map_manager::WorldCreature;
use crate::pool::{PoolGroupLikeCpp, PoolTemplateDataLikeCpp};
use crate::spawn::{SpawnData, SpawnPosition};
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_entities::{AppliedAuraRef, Player, SPELL_AURA_INTERRUPT_FLAG_ENTER_WORLD_LIKE_CPP};

mod oracle;
mod record;
mod recursive;
mod failures;
mod actors;

fn map(loaded: bool) -> Map {
    let mut map = Map::new(571, 7, 1, 1000);
    if loaded { map.load_grid(0.0, 0.0); }
    map
}

fn metadata(kind: SpawnObjectType, id: SpawnId) -> SpawnData {
    SpawnData {
        object_type: kind, spawn_id: id, map_id: 571, db_data: true,
        spawn_group: SpawnGroupTemplateData::default_group(), id: 42,
        spawn_point: SpawnPosition::new(0.0, 0.0, 0.0, 0.0),
        phase_use_flags: 0, phase_id: 0, phase_group: 0, terrain_swap_map: 0,
        pool_id: 0, spawn_time_secs: 0, spawn_difficulties: vec![1],
        script_id: 0, string_id: String::new(),
    }
}

fn store(ids: &[SpawnId]) -> SpawnStore {
    let mut store = SpawnStore::new();
    for id in ids { store.add_object_spawn(&metadata(SpawnObjectType::Creature, *id), |_| false); }
    store
}

fn add_group(manager: &mut PoolMgrLikeCpp, pool: u32, kind: PoolMemberKindLikeCpp,
    entries: &[(u64, f32)], limit: u32) {
    manager.insert_template_like_cpp(pool, PoolTemplateDataLikeCpp::new(limit, 571));
    let mut group = PoolGroupLikeCpp::with_pool_id(kind, pool);
    for (id, chance) in entries { group.add_entry_like_cpp(PoolObjectLikeCpp::new(*id, *chance), limit); }
    manager.insert_or_replace_group_like_cpp(kind, pool, group).unwrap();
}

fn creature(id: SpawnId, map_id: u32) -> Creature {
    let mut creature = Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 7, 42, id as i64),
    );
    creature.unit_mut().world_mut().set_map(map_id, 7).unwrap();
    creature.unit_mut().world_mut().relocate(Position::xyz(1.0, 2.0, 3.0));
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(75);
    creature.set_spawn_id(id);
    creature
}

fn record(id: SpawnId, map_id: u32) -> MapObjectRecord {
    MapObjectRecord::new_creature(creature(id, map_id)).unwrap()
}

fn kind_record(kind: SpawnObjectType, id: SpawnId) -> MapObjectRecord {
    match kind {
        SpawnObjectType::Creature => record(id, 571),
        SpawnObjectType::GameObject => {
            let mut object = wow_entities::GameObject::new();
            object.world_mut().object_mut().create(ObjectGuid::create_world_object(
                HighGuid::GameObject, 0, 1, 571, 7, 42, id as i64,
            ));
            object.world_mut().object_mut().set_entry(42);
            object.world_mut().set_map(571, 7).unwrap();
            object.world_mut().relocate(Position::xyz(1.0, 2.0, 3.0));
            object.set_spawn_id(id);
            MapObjectRecord::new_game_object(object).unwrap()
        }
        SpawnObjectType::AreaTrigger => panic!("not a Pool member kind"),
    }
}

fn player(id: i64, map_id: u32) -> MapObjectRecord {
    let mut player = Player::new(None, false);
    player.unit_mut().world_mut().object_mut().create(ObjectGuid::create_player(1, id));
    player.unit_mut().world_mut().set_map(map_id, 7).unwrap();
    player.unit_mut().world_mut().relocate(Position::xyz(1.0, 2.0, 3.0));
    MapObjectRecord::new_player(player).unwrap()
}

fn typed(actions: Vec<PoolSpawnObjectActionLikeCpp>) -> PoolTypedSpawnPlanLikeCpp {
    PoolTypedSpawnPlanLikeCpp {
        kind: PoolMemberKindLikeCpp::Creature, pool_id: 1000, trigger_from: 0,
        max_limit: Some(1), skip_reason: None,
        object_plan: Some(PoolSpawnObjectPlanLikeCpp { actions, ..Default::default() }),
    }
}

fn spawn_action(id: u64) -> PoolSpawnObjectActionLikeCpp {
    PoolSpawnObjectActionLikeCpp::SpawnOne { kind: PoolMemberKindLikeCpp::Creature, guid: id }
}

fn owned_typed<L>(map: &mut Map, plan: &PoolTypedSpawnPlanLikeCpp, store: &SpawnStore, load: Option<&mut L>) -> LoadedGridPoolOutcome
where L: FnMut(&mut Map, SpawnObjectType, SpawnId) -> Result<Option<LoadedGridMaterialization>, LoadedGridRespawnRecordsLikeCpp> {
    let mut summary = ProcessRespawnsSafeSideEffectsSummaryLikeCpp::default();
    let mut receipts = LoadedGridReceipts::Owned(Vec::new());
    map.apply_pool_typed_materialized(plan, store, &mut summary, load, &mut receipts);
    receipts.finish_pool(summary)
}

fn pool_plan(plan: &LoadedGridAttemptPlan) -> &PoolSpawnActionLoadPlanLikeCpp {
    match plan {
        LoadedGridAttemptPlan::Pool(plan) => plan,
        LoadedGridAttemptPlan::SpawnGroup(_) | LoadedGridAttemptPlan::Catalog { .. } => panic!("Pool emits only Pool provenance"),
    }
}

fn timer(map: &mut Map, id: SpawnId) {
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature, spawn_id: id, entry: 42,
        respawn_time: 12345, grid_id: 0,
    });
}

fn assert_summary(mut actual: ProcessRespawnsSafeSideEffectsSummaryLikeCpp, mut old: ProcessRespawnsSafeSideEffectsSummaryLikeCpp) {
    let records = std::mem::take(&mut actual.loaded_grid_primary_records);
    let previous = std::mem::take(&mut old.loaded_grid_primary_records);
    assert_eq!(actual, old); // all counters, plans, errors and timer side effects
    assert_eq!(records.len(), previous.len());
    for (record, old) in records.into_iter().zip(previous) {
        assert_eq!(record.kind(), old.kind());
        assert_eq!(record.object().guid(), old.object().guid());
        assert_eq!(record.object().object().is_in_world(), old.object().object().is_in_world());
        assert_eq!(record.creature().unwrap().current_health(), old.creature().unwrap().current_health());
        let aura = AppliedAuraRef::new(47_510, ObjectGuid::create_player(1, 99), 1, 1);
        assert_eq!(record.creature().unwrap().unit().subsystems().auras.has_applied(aura),
            old.creature().unwrap().unit().subsystems().auras.has_applied(aura));
    }
}
