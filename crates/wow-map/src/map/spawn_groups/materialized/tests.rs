use super::*;
use crate::map::loaded_grid_admission::LoadedGridSpawnAttemptResult;
use crate::map::loaded_grid_admission::LoadedGridAttemptPlan;
use crate::map_manager::WorldCreature;
use crate::spawn::{SpawnData, SpawnPosition, SpawnGroupMemberRow};
use std::collections::BTreeMap;
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_entities::{AppliedAuraRef, Player, SPELL_AURA_INTERRUPT_FLAG_ENTER_WORLD_LIKE_CPP};

mod oracle;
mod record;
mod gates;
mod owned;
mod conditions;

fn group_plan(plan: &LoadedGridAttemptPlan) -> &SpawnGroupSpawnLoadPlanLikeCpp {
    match plan {
        LoadedGridAttemptPlan::SpawnGroup(plan) => plan,
        LoadedGridAttemptPlan::Pool(_) | LoadedGridAttemptPlan::Catalog { .. } => panic!("Group emits only Group provenance"),
    }
}

fn group(id: u32, flags: SpawnGroupFlags) -> SpawnGroupTemplateData {
    SpawnGroupTemplateData { group_id: id, name: format!("owned-group-{id}"), map_id: 571, flags }
}

fn spawn(kind: SpawnObjectType, id: SpawnId) -> SpawnData {
    SpawnData {
        object_type: kind, spawn_id: id, map_id: 571, db_data: true,
        spawn_group: SpawnGroupTemplateData::default_group(), id: 42,
        spawn_point: SpawnPosition::new(0.0, 0.0, 0.0, 0.0),
        phase_use_flags: 0, phase_id: 0, phase_group: 0, terrain_swap_map: 0,
        pool_id: 0, spawn_time_secs: 0, spawn_difficulties: vec![1],
        script_id: 0, string_id: String::new(),
    }
}

fn store(groups: &[SpawnGroupTemplateData], members: Vec<(u32, SpawnData)>) -> SpawnStore {
    let mut store = SpawnStore::new();
    let mut templates = groups.iter().map(|group| (group.group_id, group.clone())).collect::<BTreeMap<_, _>>();
    for (_, spawn) in &members { store.add_object_spawn(spawn, |_| false); }
    store.apply_spawn_groups_like_cpp(&mut templates, members.iter().map(|(group_id, spawn)| {
        SpawnGroupMemberRow { group_id: *group_id, spawn_type: spawn.object_type as u8, spawn_id: spawn.spawn_id }
    }));
    store
}

fn map(loaded: bool) -> Map {
    let mut map = Map::new(571, 7, 1, 1000);
    if loaded { map.load_grid(0.0, 0.0); }
    map
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

fn player(id: i64, map_id: u32) -> MapObjectRecord {
    let mut player = Player::new(None, false);
    player.unit_mut().world_mut().object_mut().create(ObjectGuid::create_player(1, id));
    player.unit_mut().world_mut().set_map(map_id, 7).unwrap();
    player.unit_mut().world_mut().relocate(Position::xyz(1.0, 2.0, 3.0));
    MapObjectRecord::new_player(player).unwrap()
}

fn aura() -> AppliedAuraRef {
    AppliedAuraRef::new(47_510, ObjectGuid::create_player(1, 99), 1, 1)
}

fn timer(map: &mut Map, id: SpawnId) {
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature, spawn_id: id, entry: 42,
        respawn_time: 12345, grid_id: 0,
    });
}

fn assert_record_summary(mut actual: SpawnGroupSpawnOutcomeLikeCpp, mut expected: SpawnGroupSpawnOutcomeLikeCpp) {
    let records = std::mem::take(&mut actual.loaded_grid_primary_records);
    let old_records = std::mem::take(&mut expected.loaded_grid_primary_records);
    // Every counter, plan, active change and blocked status is compared here.
    assert_eq!(actual, expected);
    assert_eq!(records.len(), old_records.len());
    for (record, old) in records.into_iter().zip(old_records) {
        assert_eq!(record.kind(), old.kind());
        assert_eq!(record.object().guid(), old.object().guid());
        assert_eq!(record.object().position(), old.object().position());
        assert_eq!(record.object().object().is_in_world(), old.object().object().is_in_world());
        assert_eq!(record.creature().unwrap().current_health(), old.creature().unwrap().current_health());
        assert_eq!(record.creature().unwrap().unit().subsystems().auras.has_applied(aura()),
            old.creature().unwrap().unit().subsystems().auras.has_applied(aura()));
    }
}
