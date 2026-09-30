use super::{
    RespawnDbDeleteQueueOutcomeLikeCpp,
    RespawnDbSaveQueueOutcomeLikeCpp,
    RespawnInfoLikeCpp,
    RespawnPersistenceKeyLikeCpp,
    RespawnPersistenceMutationLikeCpp,
    SpawnObjectType,
    queue_respawn_db_delete_like_cpp,
    queue_respawn_db_save_like_cpp,
};

pub(super) fn assert_del_respawn_params_like_cpp(
    mutation: &RespawnPersistenceMutationLikeCpp,
    object_type: u16,
    spawn_id: u64,
    map_id: u16,
    instance_id: u32,
) {
    let RespawnPersistenceMutationLikeCpp::Delete { key } = mutation else {
        panic!("expected typed DEL_RESPAWN mutation, got {mutation:?}");
    };
    assert_eq!(key.object_type_raw, object_type);
    assert_eq!(key.spawn_id, spawn_id);
    assert_eq!(key.map_id, map_id);
    assert_eq!(key.instance_id, instance_id);
}

pub(super) fn assert_rep_respawn_params_like_cpp(
    mutation: &RespawnPersistenceMutationLikeCpp,
    object_type: u16,
    spawn_id: u64,
    respawn_time: i64,
    map_id: u16,
    instance_id: u32,
) {
    let RespawnPersistenceMutationLikeCpp::Save {
        key,
        respawn_time: actual_respawn_time,
    } = mutation
    else {
        panic!("expected typed REP_RESPAWN mutation, got {mutation:?}");
    };
    assert_eq!(key.object_type_raw, object_type);
    assert_eq!(key.spawn_id, spawn_id);
    assert_eq!(*actual_respawn_time, respawn_time);
    assert_eq!(key.map_id, map_id);
    assert_eq!(key.instance_id, instance_id);
}

pub(super) fn respawn_persistence_key_fixture_like_cpp(spawn_id: u64) -> RespawnPersistenceKeyLikeCpp {
    RespawnPersistenceKeyLikeCpp {
        object_type_raw: 0,
        spawn_id,
        map_id: 571,
        instance_id: 0,
    }
}

pub(super) fn linked_respawn_guid_like_cpp(
    high: wow_core::guid::HighGuid,
    entry: u32,
    spawn_id: u64,
) -> wow_core::ObjectGuid {
    wow_core::ObjectGuid::create_world_object(high, 0, 0, 571, 0, entry, spawn_id as i64)
}

pub(super) fn respawn_info_like_cpp(
    object_type: SpawnObjectType,
    spawn_id: wow_map::SpawnId,
    respawn_time: i64,
) -> RespawnInfoLikeCpp {
    RespawnInfoLikeCpp {
        object_type,
        spawn_id,
        entry: 42,
        respawn_time,
        grid_id: 7,
    }
}

pub(super) fn respawn_db_save_mutation_fixture_like_cpp(
    spawn_id: u64,
    respawn_time: i64,
) -> RespawnPersistenceMutationLikeCpp {
    let RespawnDbSaveQueueOutcomeLikeCpp::Queued(save) = queue_respawn_db_save_like_cpp(
        wow_map::ManagedMapKind::World,
        false,
        571,
        0,
        RespawnInfoLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id,
            entry: 42,
            respawn_time,
            grid_id: 7,
        },
    ) else {
        panic!("world-map fixture must queue REP_RESPAWN");
    };
    save.mutation
}

pub(super) fn respawn_db_delete_mutation_fixture_like_cpp(spawn_id: u64) -> RespawnPersistenceMutationLikeCpp {
    let RespawnDbDeleteQueueOutcomeLikeCpp::Queued(delete) = queue_respawn_db_delete_like_cpp(
        wow_map::ManagedMapKind::World,
        false,
        571,
        0,
        SpawnObjectType::Creature,
        spawn_id,
    ) else {
        panic!("world-map fixture must queue DEL_RESPAWN");
    };
    delete.mutation
}
