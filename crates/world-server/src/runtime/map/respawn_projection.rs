use super::*;

pub(crate) fn queue_respawn_db_delete_like_cpp(
    map_kind: wow_map::ManagedMapKind,
    map_is_instanceable: bool,
    map_id: u32,
    instance_id: u32,
    object_type: wow_map::SpawnObjectType,
    spawn_id: wow_map::SpawnId,
) -> RespawnDbDeleteQueueOutcomeLikeCpp {
    if !matches!(map_kind, wow_map::ManagedMapKind::World) {
        return RespawnDbDeleteQueueOutcomeLikeCpp::SkippedNonWorldMap;
    }
    if map_is_instanceable {
        return RespawnDbDeleteQueueOutcomeLikeCpp::SkippedInstanceableMap;
    }

    let Ok(map_id) = u16::try_from(map_id) else {
        return RespawnDbDeleteQueueOutcomeLikeCpp::SkippedInvalidMapId;
    };

    let mutation = RespawnPersistenceMutationLikeCpp::Delete {
        key: RespawnPersistenceKeyLikeCpp {
            object_type_raw: u16::from(object_type as u8),
            spawn_id,
            map_id,
            instance_id,
        },
    };
    RespawnDbDeleteQueueOutcomeLikeCpp::Queued(RespawnDbDeleteLikeCpp {
        object_type,
        spawn_id,
        map_id,
        instance_id,
        mutation,
    })
}

pub(crate) fn queue_respawn_db_save_like_cpp(
    map_kind: wow_map::ManagedMapKind,
    map_is_instanceable: bool,
    map_id: u32,
    instance_id: u32,
    info: wow_map::RespawnInfoLikeCpp,
) -> RespawnDbSaveQueueOutcomeLikeCpp {
    if !matches!(map_kind, wow_map::ManagedMapKind::World) {
        return RespawnDbSaveQueueOutcomeLikeCpp::SkippedNonWorldMap;
    }
    if map_is_instanceable {
        return RespawnDbSaveQueueOutcomeLikeCpp::SkippedInstanceableMap;
    }

    let Ok(map_id) = u16::try_from(map_id) else {
        return RespawnDbSaveQueueOutcomeLikeCpp::SkippedInvalidMapId;
    };

    let mutation = RespawnPersistenceMutationLikeCpp::Save {
        key: RespawnPersistenceKeyLikeCpp {
            object_type_raw: u16::from(info.object_type as u8),
            spawn_id: info.spawn_id,
            map_id,
            instance_id,
        },
        respawn_time: info.respawn_time,
    };
    RespawnDbSaveQueueOutcomeLikeCpp::Queued(RespawnDbSaveLikeCpp {
        object_type: info.object_type,
        spawn_id: info.spawn_id,
        respawn_time: info.respawn_time,
        map_id,
        instance_id,
        mutation,
    })
}
