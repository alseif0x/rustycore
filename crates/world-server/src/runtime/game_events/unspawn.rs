use super::super::*;
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct GameEventPoolUnspawnSummaryLikeCpp {
    pub(crate) event_pool_ids_seen: usize,
    pub(crate) missing_pool_templates: usize,
    pub(crate) invalid_template_map_ids: usize,
    pub(crate) pools_without_loaded_canonical_maps: usize,
    pub(crate) maps_matched: usize,
    pub(crate) pool_objects_removed: usize,
    pub(crate) pool_respawn_timers_removed: usize,
    pub(crate) pool_respawn_timers_missing: usize,
    pub(crate) pool_stale_index_entries: usize,
    pub(crate) pool_remove_errors: usize,
    pub(crate) pool_unsupported_action_kind: usize,
    pub(crate) blocked_pool_plan_errors: Vec<wow_map::PoolMgrPlanErrorLikeCpp>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct GameEventPoolEventUnspawnSummaryLikeCpp {
    pub(crate) event_id: i16,
    pub(crate) missing_event_pool_ids: bool,
    pub(crate) pool_summary: GameEventPoolUnspawnSummaryLikeCpp,
}

impl GameEventPoolUnspawnSummaryLikeCpp {
    pub(crate) fn accumulate_despawn_summary_like_cpp(
        &mut self,
        summary: &wow_map::map::ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
    ) {
        self.pool_objects_removed += summary.pool_objects_removed;
        self.pool_respawn_timers_removed += summary.pool_respawn_timers_removed;
        self.pool_respawn_timers_missing += summary.pool_respawn_timers_missing;
        self.pool_stale_index_entries += summary.pool_stale_index_entries;
        self.pool_remove_errors += summary.pool_remove_errors;
        self.pool_unsupported_action_kind += summary.pool_unsupported_action_kind;
        self.blocked_pool_plan_errors
            .extend(summary.blocked_pool_plan_errors.iter().copied());
    }
}

pub(crate) fn game_event_unspawn_pools_like_cpp(
    manager: &mut wow_map::MapManager,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    event_pool_ids: &[u32],
) -> GameEventPoolUnspawnSummaryLikeCpp {
    let pool_mgr = canonical_spawn_metadata.pool_mgr_like_cpp();
    let mut summary = GameEventPoolUnspawnSummaryLikeCpp::default();

    for &pool_id in event_pool_ids {
        summary.event_pool_ids_seen += 1;
        let Some(pool_template) = pool_mgr.pool_template_like_cpp(pool_id) else {
            summary.missing_pool_templates += 1;
            continue;
        };
        let Ok(map_id) = u32::try_from(pool_template.map_id) else {
            summary.invalid_template_map_ids += 1;
            continue;
        };

        let mut maps_matched_for_pool = 0usize;
        manager.do_for_all_maps_mut(|managed_map| {
            if managed_map.map_id() != map_id {
                return;
            }
            maps_matched_for_pool += 1;
            match managed_map
                .map_mut()
                .despawn_pool_safe_map_actions_like_cpp(pool_mgr, pool_id, true)
            {
                Ok(map_summary) => summary.accumulate_despawn_summary_like_cpp(&map_summary),
                Err(error) => summary.blocked_pool_plan_errors.push(error),
            }
        });
        summary.maps_matched += maps_matched_for_pool;
        if maps_matched_for_pool == 0 {
            summary.pools_without_loaded_canonical_maps += 1;
        }
    }

    summary
}

pub(crate) fn game_event_unspawn_pools_for_event_like_cpp(
    manager: &mut wow_map::MapManager,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    event_id: i16,
) -> GameEventPoolEventUnspawnSummaryLikeCpp {
    let Some(event_pool_ids) = canonical_spawn_metadata.game_event_pool_ids_like_cpp(event_id)
    else {
        return GameEventPoolEventUnspawnSummaryLikeCpp {
            event_id,
            missing_event_pool_ids: true,
            pool_summary: GameEventPoolUnspawnSummaryLikeCpp::default(),
        };
    };

    GameEventPoolEventUnspawnSummaryLikeCpp {
        event_id,
        missing_event_pool_ids: false,
        pool_summary: game_event_unspawn_pools_like_cpp(
            manager,
            canonical_spawn_metadata,
            event_pool_ids,
        ),
    }
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GameEventObjectUnspawnBucketSummaryLikeCpp {
    pub(crate) guids_seen: usize,
    pub(crate) skipped_active_in_other_event: usize,
    pub(crate) missing_spawn_metadata: usize,
    pub(crate) represented_object_mgr_grid_removals: usize,
    pub(crate) maps_matched: usize,
    pub(crate) without_loaded_canonical_maps: usize,
    pub(crate) respawn_timers_removed: usize,
    pub(crate) respawn_timers_missing: usize,
    pub(crate) live_objects_queued: usize,
    pub(crate) duplicate_queue_attempts: usize,
    pub(crate) stale_index_entries: usize,
    pub(crate) remove_errors: usize,
    pub(crate) unsupported_live_despawn_type: usize,
}

impl GameEventObjectUnspawnBucketSummaryLikeCpp {
    pub(crate) fn accumulate_despawn_outcome_like_cpp(
        &mut self,
        outcome: wow_map::map::DespawnAllBySpawnIdOutcomeLikeCpp,
    ) {
        self.live_objects_queued += outcome.queued;
        self.duplicate_queue_attempts += outcome.duplicates;
        self.stale_index_entries += outcome.stale_index_entries;
        self.remove_errors += outcome.remove_errors;
        self.unsupported_live_despawn_type += outcome.unsupported_live_despawn_type;
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct GameEventCreatureGameObjectUnspawnSummaryLikeCpp {
    pub(crate) event_id: i16,
    pub(crate) missing_event_creature_guids: bool,
    pub(crate) missing_event_gameobject_guids: bool,
    pub(crate) creature: GameEventObjectUnspawnBucketSummaryLikeCpp,
    pub(crate) gameobject: GameEventObjectUnspawnBucketSummaryLikeCpp,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct GameEventUnspawnForEventSummaryLikeCpp {
    pub(crate) event_id: i16,
    pub(crate) non_pool: GameEventCreatureGameObjectUnspawnSummaryLikeCpp,
    pub(crate) pool_skipped_due_to_non_pool_bucket: bool,
    pub(crate) pool: GameEventPoolEventUnspawnSummaryLikeCpp,
}

pub(crate) fn game_event_guid_is_active_in_other_event_like_cpp(
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    active_event_ids: &[u16],
    event_id: i16,
    object_type: wow_map::SpawnObjectType,
    spawn_id: wow_map::SpawnId,
) -> bool {
    if event_id <= 0 {
        return false;
    }

    active_event_ids.iter().copied().any(|active_event_id| {
        if active_event_id == event_id as u16 {
            return false;
        }
        let Ok(active_event_id) = i16::try_from(active_event_id) else {
            return false;
        };
        let active_guids = match object_type {
            wow_map::SpawnObjectType::Creature => {
                canonical_spawn_metadata.game_event_creature_guids_like_cpp(active_event_id)
            }
            wow_map::SpawnObjectType::GameObject => {
                canonical_spawn_metadata.game_event_gameobject_guids_like_cpp(active_event_id)
            }
            wow_map::SpawnObjectType::AreaTrigger => None,
        };
        active_guids.is_some_and(|guids| guids.contains(&spawn_id))
    })
}

pub(crate) fn game_event_unspawn_object_guid_list_for_event_like_cpp(
    manager: &mut wow_map::MapManager,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    active_event_ids: &[u16],
    event_id: i16,
    object_type: wow_map::SpawnObjectType,
    spawn_ids: &[wow_map::SpawnId],
) -> GameEventObjectUnspawnBucketSummaryLikeCpp {
    let mut summary = GameEventObjectUnspawnBucketSummaryLikeCpp::default();

    for &spawn_id in spawn_ids {
        summary.guids_seen += 1;
        if game_event_guid_is_active_in_other_event_like_cpp(
            canonical_spawn_metadata,
            active_event_ids,
            event_id,
            object_type,
            spawn_id,
        ) {
            summary.skipped_active_in_other_event += 1;
            continue;
        }

        let Some(spawn_data) = canonical_spawn_metadata
            .spawn_store()
            .spawn_data(object_type, spawn_id)
        else {
            summary.missing_spawn_metadata += 1;
            continue;
        };

        // C++ anchor: GameEventMgr.cpp:1246-1327 removes ObjectMgr grid metadata
        // before walking loaded maps. RustyCore has no safe ObjectMgr mutation here,
        // so this is represented as a count only and SpawnStore remains immutable.
        summary.represented_object_mgr_grid_removals += 1;

        let mut maps_matched_for_spawn = 0usize;
        manager.do_for_all_maps_mut(|managed_map| {
            if managed_map.map_id() != spawn_data.map_id {
                return;
            }
            maps_matched_for_spawn += 1;
            let map = managed_map.map_mut();
            if map
                .remove_respawn_time_like_cpp(object_type, spawn_id)
                .is_some()
            {
                summary.respawn_timers_removed += 1;
            } else {
                summary.respawn_timers_missing += 1;
            }
            let despawn = map.despawn_all_by_spawn_id_like_cpp(object_type, spawn_id);
            summary.accumulate_despawn_outcome_like_cpp(despawn);
        });

        summary.maps_matched += maps_matched_for_spawn;
        if maps_matched_for_spawn == 0 {
            summary.without_loaded_canonical_maps += 1;
        }
    }

    summary
}

pub(crate) fn game_event_unspawn_creatures_and_gameobjects_for_event_like_cpp(
    manager: &mut wow_map::MapManager,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    active_event_ids: &[u16],
    event_id: i16,
) -> GameEventCreatureGameObjectUnspawnSummaryLikeCpp {
    let Some(creature_guids) =
        canonical_spawn_metadata.game_event_creature_guids_like_cpp(event_id)
    else {
        return GameEventCreatureGameObjectUnspawnSummaryLikeCpp {
            event_id,
            missing_event_creature_guids: true,
            missing_event_gameobject_guids: false,
            creature: GameEventObjectUnspawnBucketSummaryLikeCpp::default(),
            gameobject: GameEventObjectUnspawnBucketSummaryLikeCpp::default(),
        };
    };

    let creature = game_event_unspawn_object_guid_list_for_event_like_cpp(
        manager,
        canonical_spawn_metadata,
        active_event_ids,
        event_id,
        wow_map::SpawnObjectType::Creature,
        creature_guids,
    );

    let Some(gameobject_guids) =
        canonical_spawn_metadata.game_event_gameobject_guids_like_cpp(event_id)
    else {
        return GameEventCreatureGameObjectUnspawnSummaryLikeCpp {
            event_id,
            missing_event_creature_guids: false,
            missing_event_gameobject_guids: true,
            creature,
            gameobject: GameEventObjectUnspawnBucketSummaryLikeCpp::default(),
        };
    };

    let gameobject = game_event_unspawn_object_guid_list_for_event_like_cpp(
        manager,
        canonical_spawn_metadata,
        active_event_ids,
        event_id,
        wow_map::SpawnObjectType::GameObject,
        gameobject_guids,
    );

    GameEventCreatureGameObjectUnspawnSummaryLikeCpp {
        event_id,
        missing_event_creature_guids: false,
        missing_event_gameobject_guids: false,
        creature,
        gameobject,
    }
}

pub(crate) fn game_event_unspawn_for_event_like_cpp(
    manager: &mut wow_map::MapManager,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    active_event_ids: &[u16],
    event_id: i16,
) -> GameEventUnspawnForEventSummaryLikeCpp {
    let non_pool = game_event_unspawn_creatures_and_gameobjects_for_event_like_cpp(
        manager,
        canonical_spawn_metadata,
        active_event_ids,
        event_id,
    );
    let pool_skipped_due_to_non_pool_bucket =
        non_pool.missing_event_creature_guids || non_pool.missing_event_gameobject_guids;
    let pool = if pool_skipped_due_to_non_pool_bucket {
        GameEventPoolEventUnspawnSummaryLikeCpp {
            event_id,
            missing_event_pool_ids: false,
            pool_summary: GameEventPoolUnspawnSummaryLikeCpp::default(),
        }
    } else {
        game_event_unspawn_pools_for_event_like_cpp(manager, canonical_spawn_metadata, event_id)
    };

    GameEventUnspawnForEventSummaryLikeCpp {
        event_id,
        non_pool,
        pool_skipped_due_to_non_pool_bucket,
        pool,
    }
}
