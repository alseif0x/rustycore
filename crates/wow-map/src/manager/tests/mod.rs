//! Managed-map lifecycle and updater regression scenarios.
//!
//! Separated from the manager.rs root under #644.

use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::coords::GridCoord;
use crate::grid::GridStateKind;
use crate::map::{
    MapObjectMoveListFamilyLikeCpp, RepresentedFarSpellCallbackActionLikeCpp,
    RepresentedFarSpellCallbackLikeCpp,
};
use crate::pool::{
    PoolGroupLikeCpp, PoolMemberKindLikeCpp, PoolObjectLikeCpp, PoolTemplateDataLikeCpp,
};
use crate::spawn::{SpawnData, SpawnGroupFlags, SpawnGroupTemplateData, SpawnObjectType};
use wow_constants::{DeathState, TypeId, TypeMask};
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_entities::{
    AccessorObjectKind, AreaTrigger, Conversation, Creature, DynamicObject,
    GAMEOBJECT_TYPE_GENERIC, GameObject, LootState, MapObjectRecord, ObjectNotifyFlags, Player,
    SceneObject, Transport, TransportPathLeg, TransportTemplate, WorldObject,
};

fn guid(high: HighGuid, counter: i64, map_id: u32, instance_id: u32) -> ObjectGuid {
    if high == HighGuid::Player {
        ObjectGuid::create_global(high, 0, counter)
    } else if high == HighGuid::Transport {
        ObjectGuid::create_transport(high, counter)
    } else {
        ObjectGuid::create_world_object(high, 0, 1, map_id as u16, instance_id, 100, counter)
    }
}

fn test_object_needs_notify_visibility(
    map: &Map<NoopTerrainGridLoader, NoopGridLifecycle>,
    guid: ObjectGuid,
) -> bool {
    map.map_object(guid).is_some_and(|object| {
        object
            .object()
            .is_need_notify(ObjectNotifyFlags::VISIBILITY_CHANGED)
    })
}

fn insert_player_for_relocation_notify(
    manager: &mut MapManager,
    counter: i64,
    position: Position,
) -> ObjectGuid {
    let player_guid = guid(HighGuid::Player, counter, 1, 0);
    let mut player = Player::new(None, false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    player.unit_mut().world_mut().set_map(1, 0).unwrap();
    player.unit_mut().world_mut().relocate(position);
    let record = MapObjectRecord::new_player(player).unwrap();
    manager
        .find_map_mut(1, 0)
        .unwrap()
        .map_mut()
        .add_map_object_record_to_map_like_cpp(record)
        .unwrap();
    player_guid
}

fn insert_creature_at_for_relocation_notify(
    manager: &mut MapManager,
    counter: i64,
    position: Position,
    active: bool,
) -> ObjectGuid {
    let creature_guid = guid(HighGuid::Creature, counter, 1, 0);
    let mut creature = Creature::new(false);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(creature_guid);
    creature.unit_mut().world_mut().set_map(1, 0).unwrap();
    creature.unit_mut().world_mut().relocate(position);
    creature.unit_mut().world_mut().set_active(active);
    creature.unit_mut().set_death_state(DeathState::Alive);
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    let record = MapObjectRecord::new_creature(creature).unwrap();
    manager
        .find_map_mut(1, 0)
        .unwrap()
        .map_mut()
        .add_map_object_record_to_map_like_cpp(record)
        .unwrap();
    creature_guid
}

fn insert_creature_for_update(
    manager: &mut MapManager,
    counter: i64,
    in_world: bool,
) -> ObjectGuid {
    let creature_guid = guid(HighGuid::Creature, counter, 1, 0);
    let mut creature = Creature::new(false);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(creature_guid);
    creature.unit_mut().world_mut().set_map(1, 0).unwrap();
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(10.0, 20.0, 30.0));
    let record = MapObjectRecord::new_creature(creature).unwrap();
    let map = manager.find_map_mut(1, 0).unwrap().map_mut();
    // Full C++ Map::AddToMap: creates the grid, inserts the guid into the cell,
    // and calls the unit AddToWorld — so the cell-anchored ObjectUpdater can
    // reach it. (Building the creature already in-world would trip the
    // already-in-world early return that skips cell insertion.)
    map.add_map_object_record_to_map_like_cpp(record).unwrap();
    if !in_world {
        // Keep the record cell-resident but flip it out of the world to
        // exercise update_creature's NotInWorld skip (C++ RemoveFromWorld).
        map.test_remove_creature_from_world_keep_cell_like_cpp(creature_guid);
    }
    creature_guid
}

fn insert_dynamic_object_for_update(
    manager: &mut MapManager,
    counter: i64,
    duration_ms: i32,
    in_world: bool,
) -> ObjectGuid {
    let dynamic_object_guid = guid(HighGuid::DynamicObject, counter, 1, 0);
    let mut dynamic_object = DynamicObject::new(true);
    dynamic_object
        .world_mut()
        .object_mut()
        .create(dynamic_object_guid);
    dynamic_object.world_mut().set_map(1, 0).unwrap();
    dynamic_object
        .world_mut()
        .relocate(Position::xyz(11.0, 21.0, 31.0));
    if in_world {
        dynamic_object.world_mut().object_mut().add_to_world();
    }
    dynamic_object.set_duration(duration_ms);
    let record = MapObjectRecord::new_dynamic_object(dynamic_object).unwrap();
    let map = manager.find_map_mut(1, 0).unwrap().map_mut();
    if in_world {
        map.add_map_object_record_to_map_like_cpp(record).unwrap();
    } else {
        map.insert_map_object_record(record).unwrap();
    }
    dynamic_object_guid
}

fn insert_pool_compatible_owner_created_game_object_for_update(
    manager: &mut MapManager,
    spawn_id: u64,
    pool_id: u32,
) -> ObjectGuid {
    let game_object_guid = guid(HighGuid::GameObject, spawn_id as i64, 1, 0);
    let mut game_object = GameObject::new();
    game_object
        .world_mut()
        .object_mut()
        .create(game_object_guid);
    game_object.world_mut().set_map(1, 0).unwrap();
    game_object
        .world_mut()
        .relocate(Position::xyz(13.0, 23.0, 33.0));
    game_object.world_mut().object_mut().add_to_world();
    game_object.set_go_type(GAMEOBJECT_TYPE_GENERIC as u8);
    game_object.world_mut().object_mut().set_entry(190_011);
    game_object.set_spawn_id(spawn_id);
    game_object.set_represented_gameobject_data_present_like_cpp(true);
    game_object.set_respawn_compatibility_mode(true);
    game_object.set_created_by(guid(HighGuid::Player, spawn_id as i64 + 99, 1, 0));
    game_object.set_respawn_time(0);
    game_object.set_loot_state(LootState::JustDeactivated, None);

    let map = manager.find_map_mut(1, 0).unwrap().map_mut();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(game_object).unwrap(),
    )
    .unwrap();
    map.pool_data_mut_like_cpp()
        .add_spawn_like_cpp(SpawnObjectType::GameObject, spawn_id, pool_id)
        .expect("spawned pool state");
    game_object_guid
}

fn gameobject_pool_update_context_for_manager_like_cpp(
    pool_id: u32,
    despawned_spawn_id: u64,
    replacement_spawn_id: u64,
) -> (SpawnStore, PoolMgrLikeCpp) {
    let mut store = SpawnStore::new();
    let active = spawn_group_for_manager_like_cpp(pool_id + 100, SpawnGroupFlags::NONE);
    let mut despawned_spawn = spawn_data_for_manager_like_cpp(
        SpawnObjectType::GameObject,
        despawned_spawn_id,
        active.clone(),
    );
    despawned_spawn.pool_id = pool_id;
    let mut replacement_spawn =
        spawn_data_for_manager_like_cpp(SpawnObjectType::GameObject, replacement_spawn_id, active);
    replacement_spawn.pool_id = pool_id;
    store.add_object_spawn(&despawned_spawn, |_| false);
    store.add_object_spawn(&replacement_spawn, |_| false);

    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(pool_id, PoolTemplateDataLikeCpp::new(1, 1));
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, pool_id);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(replacement_spawn_id, 0.0), 1);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(despawned_spawn_id, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::GameObject, pool_id, group)
        .expect("test pool group");
    pool_mgr
        .register_spawn_pool_relation_like_cpp(
            PoolMemberKindLikeCpp::GameObject,
            despawned_spawn_id,
            pool_id,
        )
        .expect("test pool relation");

    (store, pool_mgr)
}

fn spawn_group_for_manager_like_cpp(
    group_id: u32,
    flags: SpawnGroupFlags,
) -> SpawnGroupTemplateData {
    SpawnGroupTemplateData {
        group_id,
        name: format!("manager-group-{group_id}"),
        map_id: 1,
        flags,
    }
}

fn spawn_data_for_manager_like_cpp(
    object_type: SpawnObjectType,
    spawn_id: u64,
    spawn_group: SpawnGroupTemplateData,
) -> SpawnData {
    SpawnData {
        object_type,
        spawn_id,
        map_id: 1,
        db_data: true,
        spawn_group,
        id: 99,
        spawn_point: crate::spawn::SpawnPosition::new(0.0, 0.0, 0.0, 0.0),
        phase_use_flags: 0,
        phase_id: 0,
        phase_group: 0,
        terrain_swap_map: 0,
        pool_id: 0,
        spawn_time_secs: 0,
        spawn_difficulties: vec![1],
        script_id: 0,
        string_id: String::new(),
    }
}

fn insert_game_object_for_update(
    manager: &mut MapManager,
    counter: i64,
    despawn_delay_ms: u32,
    in_world: bool,
) -> ObjectGuid {
    let game_object_guid = guid(HighGuid::GameObject, counter, 1, 0);
    let mut game_object = GameObject::new();
    game_object
        .world_mut()
        .object_mut()
        .create(game_object_guid);
    game_object.world_mut().set_map(1, 0).unwrap();
    game_object
        .world_mut()
        .relocate(Position::xyz(13.0, 23.0, 33.0));
    if in_world {
        game_object.world_mut().object_mut().add_to_world();
    }
    if despawn_delay_ms != 0 {
        assert!(game_object.schedule_despawn_or_unsummon_like_cpp(despawn_delay_ms, 77));
    }
    let record = MapObjectRecord::new_game_object(game_object).unwrap();
    let map = manager.find_map_mut(1, 0).unwrap().map_mut();
    if in_world {
        map.add_map_object_record_to_map_like_cpp(record).unwrap();
    } else {
        map.insert_map_object_record(record).unwrap();
    }
    game_object_guid
}

fn insert_generic_world_object_record_for_metrics(
    manager: &mut MapManager,
    kind: AccessorObjectKind,
    guid: ObjectGuid,
    type_id: TypeId,
    type_mask: TypeMask,
) {
    let mut object = WorldObject::new(true, type_id, type_mask);
    object.object_mut().create(guid);
    object.set_map(1, 0).unwrap();
    object.relocate(Position::xyz(17.0, 27.0, 37.0));
    object.object_mut().add_to_world();
    let record = MapObjectRecord::new(kind, object).unwrap();
    manager
        .find_map_mut(1, 0)
        .unwrap()
        .map_mut()
        .add_map_object_record_to_map_like_cpp(record)
        .unwrap();
}

fn insert_transport_for_update(
    manager: &mut MapManager,
    counter: i64,
    in_world: bool,
    expected_map_id: u32,
) -> ObjectGuid {
    let transport_guid = guid(HighGuid::Transport, counter, 1, 0);
    let template = TransportTemplate {
        total_path_time_ms: 1_000,
        path_legs: vec![TransportPathLeg {
            map_id: expected_map_id,
            start_timestamp_ms: 0,
            duration_ms: 1_000,
            segments: vec![],
        }],
        ..TransportTemplate::default()
    };
    let mut transport = Transport::with_template(template);
    transport.world_mut().object_mut().create(transport_guid);
    transport.world_mut().set_map(1, 0).unwrap();
    transport
        .world_mut()
        .relocate(Position::xyz(15.0, 25.0, 35.0));
    transport.set_path_progress_ms(100);
    if in_world {
        transport.world_mut().object_mut().add_to_world();
    }
    let record = MapObjectRecord::new_transport(transport).unwrap();
    let map = manager.find_map_mut(1, 0).unwrap().map_mut();
    if in_world {
        map.add_map_object_record_to_map_like_cpp(record).unwrap();
    } else {
        map.insert_map_object_record(record).unwrap();
    }
    transport_guid
}

fn insert_area_trigger_for_update(
    manager: &mut MapManager,
    counter: i64,
    duration_ms: i32,
    in_world: bool,
) -> ObjectGuid {
    let area_trigger_guid = guid(HighGuid::AreaTrigger, counter, 1, 0);
    let mut area_trigger = AreaTrigger::new();
    area_trigger
        .world_mut()
        .object_mut()
        .create(area_trigger_guid);
    area_trigger.world_mut().set_map(1, 0).unwrap();
    area_trigger
        .world_mut()
        .relocate(Position::xyz(12.0, 22.0, 32.0));
    if in_world {
        area_trigger.world_mut().object_mut().add_to_world();
    }
    area_trigger.set_duration(duration_ms);
    let record = MapObjectRecord::new_area_trigger(area_trigger).unwrap();
    let map = manager.find_map_mut(1, 0).unwrap().map_mut();
    if in_world {
        map.add_map_object_record_to_map_like_cpp(record).unwrap();
    } else {
        map.insert_map_object_record(record).unwrap();
    }
    area_trigger_guid
}

fn insert_conversation_for_update(
    manager: &mut MapManager,
    counter: i64,
    duration_ms: i32,
    in_world: bool,
) -> ObjectGuid {
    let conversation_guid = guid(HighGuid::Conversation, counter, 1, 0);
    let mut conversation = Conversation::new();
    conversation
        .world_mut()
        .object_mut()
        .create(conversation_guid);
    conversation.world_mut().set_map(1, 0).unwrap();
    conversation
        .world_mut()
        .relocate(Position::xyz(13.0, 23.0, 33.0));
    if in_world {
        conversation.world_mut().object_mut().add_to_world();
    }
    conversation.set_duration_ms(duration_ms);
    let record = MapObjectRecord::new_conversation(conversation).unwrap();
    let map = manager.find_map_mut(1, 0).unwrap().map_mut();
    if in_world {
        map.add_map_object_record_to_map_like_cpp(record).unwrap();
    } else {
        map.insert_map_object_record(record).unwrap();
    }
    conversation_guid
}

fn insert_scene_object_for_update(
    manager: &mut MapManager,
    counter: i64,
    in_world: bool,
    created_by_spell_cast: Option<ObjectGuid>,
) -> ObjectGuid {
    let scene_object_guid = guid(HighGuid::SceneObject, counter, 1, 0);
    let mut scene_object = SceneObject::new();
    scene_object
        .world_mut()
        .object_mut()
        .create(scene_object_guid);
    scene_object.world_mut().set_map(1, 0).unwrap();
    scene_object
        .world_mut()
        .relocate(Position::xyz(14.0, 24.0, 34.0));
    scene_object.set_created_by(guid(HighGuid::Player, counter + 1000, 1, 0));
    if let Some(cast_guid) = created_by_spell_cast {
        scene_object.set_created_by_spell_cast(cast_guid);
    }
    if in_world {
        scene_object.world_mut().object_mut().add_to_world();
    }
    let record = MapObjectRecord::new_scene_object(scene_object).unwrap();
    let map = manager.find_map_mut(1, 0).unwrap().map_mut();
    if in_world {
        map.add_map_object_record_to_map_like_cpp(record).unwrap();
    } else {
        map.insert_map_object_record(record).unwrap();
    }
    scene_object_guid
}

fn world_entry(map_id: u32) -> CreateMapEntryContext {
    CreateMapEntryContext {
        map_id,
        kind: CreateMapEntryKind::World,
        split_by_faction: false,
        flex_locking: false,
    }
}

fn dungeon_entry(map_id: u32, flex_locking: bool) -> CreateMapEntryContext {
    CreateMapEntryContext {
        map_id,
        kind: CreateMapEntryKind::Dungeon,
        split_by_faction: false,
        flex_locking,
    }
}

fn player() -> CreateMapPlayerContext {
    CreateMapPlayerContext {
        guid_counter: 77,
        team_id: 469,
        battleground_id: 0,
        has_battleground: false,
        player_difficulty_id: 1,
        player_recent_instance_id: 0,
        group: None,
    }
}

fn difficulty(
    difficulty_id: Difficulty,
    has_reset_schedule: bool,
    is_instance_id_bound: bool,
) -> CreateMapDifficultyContext {
    CreateMapDifficultyContext {
        difficulty_id,
        has_reset_schedule,
        is_instance_id_bound,
    }
}

mod scenarios_1;
mod scenarios_2;
mod scenarios_3;
mod scenarios_4;
