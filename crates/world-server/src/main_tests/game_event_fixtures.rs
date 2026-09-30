use super::{
    Arc, BTreeMap, Creature, GameEventPersistenceMutationLikeCpp,
    GameEventWorldEventStateDbOperationKindLikeCpp, GameEventWorldEventStateDbOperationLikeCpp,
    GameObject, HighGuid, LoadedGridCreatureRespawnCachesLikeCpp, MapObjectRecord, ObjectGuid,
    PoolGroupLikeCpp, PoolMemberKindLikeCpp, PoolMgrLikeCpp, PoolObjectLikeCpp,
    PoolTemplateDataLikeCpp, Position, SpawnData, SpawnGroupFlags, SpawnGroupTemplateData,
    SpawnObjectType, SpawnPosition, SpawnStore, spawn_store_loader,
    variable_loaded_grid_creature_respawn_caches_with_vehicle_id_and_difficulty_like_cpp,
};

pub(super) fn game_event_quest_complete_progressed_outcome_like_cpp(
    save_world_event_state_requested: bool,
    force_game_event_update_requested: bool,
) -> spawn_store_loader::GameEventQuestCompleteOutcomeLikeCpp {
    spawn_store_loader::GameEventQuestCompleteOutcomeLikeCpp::Progress(
        spawn_store_loader::GameEventConditionProgressOutcomeLikeCpp::Progressed(
            spawn_store_loader::GameEventConditionProgressSummaryLikeCpp {
                event_id: 7,
                condition_id: 44,
                done_before: 2.5,
                done_after: 5.25,
                req_num: 10.0,
                persistence_event_id: 7,
                completed_event: save_world_event_state_requested,
                check_outcome: spawn_store_loader::GameEventConditionCheckOutcomeLikeCpp::Completed(
                    spawn_store_loader::GameEventConditionCheckSummaryLikeCpp {
                        event_id: 7,
                        condition_count: 1,
                        state_before_raw: 2,
                        state_after_raw: 3,
                        next_start_before: 0,
                        next_start_after: 1_234,
                    },
                ),
                save_world_event_state_requested,
                force_game_event_update_requested,
            },
        ),
    )
}

pub(super) fn canonical_spawn_metadata_with_pool_mgr_like_cpp(
    pool_mgr: PoolMgrLikeCpp,
) -> spawn_store_loader::CanonicalSpawnMetadataLikeCpp {
    spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(SpawnStore::new(), BTreeMap::new())
        .with_pool_mgr_like_cpp(pool_mgr)
}

pub(super) fn canonical_spawn_metadata_with_store_and_pool_mgr_like_cpp(
    spawn_store: SpawnStore,
    pool_mgr: PoolMgrLikeCpp,
) -> spawn_store_loader::CanonicalSpawnMetadataLikeCpp {
    spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(spawn_store, BTreeMap::new())
        .with_pool_mgr_like_cpp(pool_mgr)
}

pub(super) fn canonical_spawn_metadata_with_store_pool_mgr_and_game_event_pools_like_cpp(
    spawn_store: SpawnStore,
    pool_mgr: PoolMgrLikeCpp,
    game_event_pools: spawn_store_loader::GameEventPoolIdsLikeCpp,
) -> spawn_store_loader::CanonicalSpawnMetadataLikeCpp {
    spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(spawn_store, BTreeMap::new())
        .with_pool_mgr_like_cpp(pool_mgr)
        .with_game_event_pools_like_cpp(game_event_pools)
}

pub(super) fn pool_mgr_with_creature_pool_like_cpp(
    pool_id: u32,
    map_id: i32,
    spawn_id: wow_map::SpawnId,
) -> PoolMgrLikeCpp {
    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(pool_id, PoolTemplateDataLikeCpp::new(1, map_id));
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, pool_id);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(spawn_id, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, pool_id, group)
        .expect("test creature pool group");
    pool_mgr
}

pub(super) fn spawn_data_like_cpp(
    object_type: SpawnObjectType,
    spawn_id: wow_map::SpawnId,
    map_id: u32,
) -> SpawnData {
    SpawnData {
        object_type,
        spawn_id,
        map_id,
        db_data: true,
        spawn_group: SpawnGroupTemplateData {
            group_id: 534,
            name: "game-event-object-guid-unspawn".to_string(),
            map_id,
            flags: SpawnGroupFlags::NONE,
        },
        id: 99,
        spawn_point: SpawnPosition::new(1_000.0, 1_000.0, 0.0, 0.0),
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

pub(super) fn add_spawn_data_like_cpp(
    store: &mut SpawnStore,
    object_type: SpawnObjectType,
    spawn_id: wow_map::SpawnId,
    map_id: u32,
) {
    store.add_object_spawn(&spawn_data_like_cpp(object_type, spawn_id, map_id), |_| {
        false
    });
}

pub(super) fn game_event_npc_flag_template_store_like_cpp()
-> wow_data::CreatureTemplateLifecycleStoreLikeCpp {
    wow_data::CreatureTemplateLifecycleStoreLikeCpp::from_templates([
        wow_data::CreatureTemplateLifecycleRecordLikeCpp {
            entry: 99,
            name: "Game Event NPC Flag Template".to_string(),
            ai_name: String::new(),
            script_name: String::new(),
            required_expansion: 2,
            faction: 35,
            npc_flags: 0x80,
            speed_walk: 1.0,
            speed_run: 1.14286,
            scale: 1.0,
            classification: 0,
            damage_school: wow_constants::spell::SpellSchools::Normal as u8,
            unit_flags: 0,
            unit_flags2: 0,
            unit_flags3: 0,
            creature_type: 0,
            family: 0,
            trainer_class: 0,
            unit_class: 1,
            vehicle_id: 0,
            movement_type: 0,
            ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: 0,
            rooted: false,
            chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
            random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
            interaction_pause_timer_ms:
                wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
            flags_extra: wow_constants::creature::CreatureFlagsExtra::WORLDEVENT.bits(),
            string_id: String::new(),
            regen_health: true,
            spells: [0; wow_data::MAX_CREATURE_SPELLS_LIKE_CPP],
            models: Vec::new(),
        },
    ])
}

pub(super) fn game_event_spawn_test_spawn_data_like_cpp(
    object_type: SpawnObjectType,
    spawn_id: wow_map::SpawnId,
    map_id: u32,
    entry: u32,
    x: f32,
    y: f32,
    spawn_time_secs: i32,
) -> SpawnData {
    SpawnData {
        object_type,
        spawn_id,
        map_id,
        db_data: true,
        spawn_group: SpawnGroupTemplateData {
            group_id: 535,
            name: "game-event-object-guid-spawn".to_string(),
            map_id,
            flags: SpawnGroupFlags::NONE,
        },
        id: entry,
        spawn_point: SpawnPosition::new(x, y, 0.0, 0.0),
        phase_use_flags: 0,
        phase_id: 0,
        phase_group: 0,
        terrain_swap_map: 0,
        pool_id: 0,
        spawn_time_secs,
        spawn_difficulties: vec![0],
        script_id: 0,
        string_id: String::new(),
    }
}

pub(super) fn game_event_spawn_test_caches_like_cpp(
    creature_entry: u32,
    gameobject_entry: u32,
) -> LoadedGridCreatureRespawnCachesLikeCpp {
    let mut caches =
        variable_loaded_grid_creature_respawn_caches_with_vehicle_id_and_difficulty_like_cpp(
            creature_entry,
            0,
            0,
        );
    let mut data = [0; wow_entities::MAX_GAMEOBJECT_DATA];
    data[11] = 1;
    caches.gameobject_template_store = Arc::new(
        wow_data::GameObjectTemplateLifecycleStoreLikeCpp::from_templates([
            wow_data::GameObjectTemplateLifecycleRecordLikeCpp {
                entry: gameobject_entry,
                go_type: wow_entities::GAMEOBJECT_TYPE_GOOBER,
                display_id: 44,
                name: "GameEventSpawn GO".to_string(),
                size: 1.0,
                data,
                content_tuning_id: 0,
                ai_name: String::new(),
                script_name: String::new(),
                string_id: String::new(),
                addon: None,
            },
        ]),
    );
    caches
}

pub(super) fn canonical_spawn_metadata_with_store_and_game_event_guids_like_cpp(
    spawn_store: SpawnStore,
    game_event_guids: spawn_store_loader::GameEventSpawnGuidsLikeCpp,
) -> spawn_store_loader::CanonicalSpawnMetadataLikeCpp {
    spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(spawn_store, BTreeMap::new())
        .with_game_event_spawn_guids_like_cpp(game_event_guids)
}

pub(super) fn push_game_event_guid_for_test_like_cpp(
    mut guids: spawn_store_loader::GameEventSpawnGuidsLikeCpp,
    object_type: SpawnObjectType,
    event_id: i16,
    spawn_id: wow_map::SpawnId,
) -> spawn_store_loader::GameEventSpawnGuidsLikeCpp {
    assert!(
        guids.push_guid_like_cpp(object_type, event_id, spawn_id),
        "test event id/type must fit C++ GameEvent creature/gameobject GUID range"
    );
    guids
}

pub(super) fn test_guid_like_cpp(high: HighGuid, counter: i64, entry: u32) -> ObjectGuid {
    match high {
        HighGuid::Creature => ObjectGuid::create_creature_like_cpp(1, 1, entry, counter),
        HighGuid::Vehicle => ObjectGuid::create_vehicle_like_cpp(1, 1, entry, counter),
        HighGuid::GameObject => ObjectGuid::create_gameobject_like_cpp(1, entry, counter),
        HighGuid::AreaTrigger => ObjectGuid::create_area_trigger_like_cpp(1, entry, counter),
        _ => ObjectGuid::create_world_object(high, 0, 0, 1, 0, entry, counter),
    }
}

pub(super) fn insert_live_creature_for_spawn_like_cpp(
    manager: &mut wow_map::MapManager,
    map_id: u32,
    spawn_id: wow_map::SpawnId,
    counter: i64,
) {
    let mut creature = Creature::new(false);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(test_guid_like_cpp(HighGuid::Creature, counter, 99));
    creature.unit_mut().world_mut().set_map(map_id, 0).unwrap();
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(1_000.0, 1_000.0, 0.0));
    creature.unit_mut().world_mut().object_mut().add_to_world();
    creature.set_spawn_id(spawn_id);
    manager
        .find_map_mut(map_id, 0)
        .expect("test map")
        .map_mut()
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .expect("test creature add to map");
}

pub(super) fn insert_live_gameobject_for_spawn_like_cpp(
    manager: &mut wow_map::MapManager,
    map_id: u32,
    spawn_id: wow_map::SpawnId,
    counter: i64,
) {
    let mut gameobject = GameObject::new();
    gameobject
        .world_mut()
        .object_mut()
        .create(test_guid_like_cpp(HighGuid::GameObject, counter, 99));
    gameobject.world_mut().set_map(map_id, 0).unwrap();
    gameobject
        .world_mut()
        .relocate(Position::xyz(1_000.0, 1_000.0, 0.0));
    gameobject.world_mut().object_mut().add_to_world();
    gameobject.set_spawn_id(spawn_id);
    manager
        .find_map_mut(map_id, 0)
        .expect("test map")
        .map_mut()
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .expect("test gameobject add to map");
}

pub(super) fn game_event_world_state_metadata_like_cpp(
    max_event_entry: u32,
    events: &[spawn_store_loader::GameEventDataLikeCpp],
) -> spawn_store_loader::CanonicalSpawnMetadataLikeCpp {
    let store = events.iter().cloned().fold(
        spawn_store_loader::GameEventDataStoreLikeCpp::from_game_event_max_entry_like_cpp(Some(
            max_event_entry,
        )),
        spawn_store_loader::GameEventDataStoreLikeCpp::with_event_like_cpp,
    );
    spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(SpawnStore::new(), BTreeMap::new())
        .with_game_events_like_cpp(store)
}

pub(super) fn game_event_world_state_start_outcome_like_cpp(
    event_id: u16,
) -> spawn_store_loader::GameEventUpdateOutcomeLikeCpp {
    spawn_store_loader::GameEventUpdateOutcomeLikeCpp {
        current_time_secs: 650,
        scanned_event_ids: vec![],
        check_outcomes: vec![],
        next_check_outcomes: vec![],
        queued_activation_event_ids: vec![event_id],
        queued_deactivation_event_ids: vec![],
        start_outcomes: vec![spawn_store_loader::GameEventStartOutcomeLikeCpp::Started(
            spawn_store_loader::GameEventStartSummaryLikeCpp {
                event_id,
                state_before_raw: 0,
                state_after_raw: 0,
                active_added: true,
                active_was_present: false,
                apply_new_event_requested: true,
                save_world_event_state_requested: false,
                force_game_event_update_requested: false,
                completed: false,
            },
        )],
        stop_outcomes: vec![],
        negative_spawn_event_ids: vec![],
        world_nextphase_finished: vec![],
        world_conditions_save_requested: vec![],
        invalid_check_outcomes: vec![],
        invalid_next_check_outcomes: vec![],
        next_event_delay_secs_before_padding: 0,
        next_update_delay_millis: 1_000,
    }
}

pub(super) fn empty_game_event_update_outcome_for_db_bridge_like_cpp()
-> spawn_store_loader::GameEventUpdateOutcomeLikeCpp {
    spawn_store_loader::GameEventUpdateOutcomeLikeCpp {
        current_time_secs: 650,
        scanned_event_ids: vec![],
        check_outcomes: vec![],
        next_check_outcomes: vec![],
        queued_activation_event_ids: vec![],
        queued_deactivation_event_ids: vec![],
        start_outcomes: vec![],
        stop_outcomes: vec![],
        negative_spawn_event_ids: vec![],
        world_nextphase_finished: vec![],
        world_conditions_save_requested: vec![],
        invalid_check_outcomes: vec![],
        invalid_next_check_outcomes: vec![],
        next_event_delay_secs_before_padding: 0,
        next_update_delay_millis: 1_000,
    }
}

pub(super) fn assert_game_event_save_operation_like_cpp(
    operation: &GameEventWorldEventStateDbOperationLikeCpp,
    event_id: u8,
    state: u8,
    next_start: i64,
) {
    assert_eq!(operation.event_id, event_id);
    assert_eq!(
        operation.kind,
        GameEventWorldEventStateDbOperationKindLikeCpp::Save
    );
    assert_eq!(
        operation.mutation,
        wow_persistence::GameEventPersistenceMutationLikeCpp::SaveWorldEventState {
            event_id,
            state,
            next_start,
        }
    );
}

pub(super) fn game_event_live_update_npc_vendor_record_like_cpp(
    spawn_id: wow_map::SpawnId,
    entry: u32,
    item: u32,
    vendor_type: u8,
) -> spawn_store_loader::GameEventNpcVendorRecordLikeCpp {
    spawn_store_loader::GameEventNpcVendorRecordLikeCpp {
        spawn_id,
        guid: spawn_id,
        entry,
        item,
        maxcount: 0,
        incrtime: 0,
        extended_cost: 0,
        vendor_type,
        item_type: vendor_type,
        bonus_list_ids: Vec::new(),
        player_condition_id: 0,
        ignore_filtering: false,
        event_npc_flag_low32: 0,
    }
}

pub(super) fn game_event_live_update_npc_vendor_metadata_like_cpp(
    max_event_entry: u32,
    records: &[(u16, wow_map::SpawnId, u32, u32, u8)],
) -> spawn_store_loader::CanonicalSpawnMetadataLikeCpp {
    let mut vendors =
        spawn_store_loader::GameEventNpcVendorsLikeCpp::from_game_event_max_entry_like_cpp(Some(
            max_event_entry,
        ));
    for (event_id, spawn_id, entry, item, vendor_type) in records {
        assert!(vendors.push_record_like_cpp(
            *event_id,
            game_event_live_update_npc_vendor_record_like_cpp(
                *spawn_id,
                *entry,
                *item,
                *vendor_type,
            ),
        ));
    }
    spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(SpawnStore::new(), BTreeMap::new())
        .with_game_event_npc_vendors_like_cpp(vendors)
}

pub(super) fn live_npc_flags_like_cpp(
    manager: &wow_map::MapManager,
    map_id: u32,
    spawn_id: wow_map::SpawnId,
) -> u32 {
    manager
        .find_map(map_id, 0)
        .expect("test map")
        .map()
        .get_creature_by_spawn_id_like_cpp(spawn_id)
        .expect("test live creature")
        .ai_ownership()
        .npc_flags
}

pub(super) fn live_npc_flags2_like_cpp(
    manager: &wow_map::MapManager,
    map_id: u32,
    spawn_id: wow_map::SpawnId,
) -> u32 {
    manager
        .find_map(map_id, 0)
        .expect("test map")
        .map()
        .get_creature_by_spawn_id_like_cpp(spawn_id)
        .expect("test live creature")
        .ai_ownership()
        .npc_flags2
}
