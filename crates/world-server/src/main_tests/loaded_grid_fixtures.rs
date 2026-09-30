use super::{
    Arc, BTreeMap, Condition, ConditionSourceType, ConditionType,
    LoadedGridCreatureRespawnCachesLikeCpp, SpawnData, SpawnGroupFlags, SpawnGroupMemberRow,
    SpawnGroupTemplateData, SpawnObjectType, SpawnPosition, SpawnStore, spawn_store_loader,
};

pub(super) fn canonical_test_map_store_like_cpp() -> wow_data::MapStore {
    wow_data::MapStore::from_entries([0, 530, 571, 999].map(|id| wow_data::MapEntry {
        id,
        instance_type: wow_data::map::MAP_COMMON,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }))
}

pub(super) fn empty_loaded_grid_creature_respawn_caches_like_cpp()
-> LoadedGridCreatureRespawnCachesLikeCpp {
    LoadedGridCreatureRespawnCachesLikeCpp {
        realm_id: 1,
        template_store: Arc::new(wow_data::CreatureTemplateLifecycleStoreLikeCpp::default()),
        sparring_store: Arc::new(wow_data::CreatureTemplateSparringStoreLikeCpp::default()),
        difficulty_store: Arc::new(wow_data::CreatureDifficultyStoreLikeCpp::default()),
        base_stats_store: Arc::new(wow_data::CreatureBaseStatsStoreLikeCpp::default()),
        chr_classes_store: Arc::new(
            wow_data::character_progression::ChrClassesStore::from_entries([]),
        ),
        power_type_store: Arc::new(
            wow_data::character_progression::PowerTypeStore::from_entries([]),
        ),
        health_rates: wow_data::CreatureClassificationHealthRatesLikeCpp::default(),
        display_store: Arc::new(wow_data::CreatureDisplayInfoStore::from_entries([])),
        model_store: Arc::new(wow_data::CreatureModelDataStore::from_entries([])),
        model_info_store: Arc::new(wow_data::CreatureModelInfoStoreLikeCpp::from_entries([])),
        creature_equipment_store: Arc::new(wow_data::CreatureEquipmentStoreLikeCpp::default()),
        creature_addon_store: Arc::new(wow_data::CreatureAddonStoreLikeCpp::default()),
        spell_x_spell_visual_store: Arc::new(wow_data::SpellXSpellVisualStore::from_entries([])),
        vehicle_store: Arc::new(wow_data::VehicleStore::from_entries([])),
        vehicle_seat_store: Arc::new(wow_data::VehicleSeatStore::from_entries([])),
        vehicle_accessory_store: Arc::new(wow_data::VehicleAccessoryStoreLikeCpp::from_parts(
            [],
            [],
        )),
        gameobject_template_store: Arc::new(
            wow_data::GameObjectTemplateLifecycleStoreLikeCpp::default(),
        ),
        gameobject_override_store: Arc::new(
            wow_data::GameObjectOverrideLifecycleStoreLikeCpp::default(),
        ),
    }
}

pub(super) fn loaded_grid_map_store_like_cpp(map_id: u32, instance_type: i8) -> wow_data::MapStore {
    wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: map_id,
        instance_type,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }])
}

pub(super) fn area_trigger_template_store_for_loaded_grid_like_cpp(
    create_properties_id: u32,
    template_id: u32,
) -> wow_data::AreaTriggerTemplateStore {
    let map_store = wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: 571,
        instance_type: 0,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }]);
    let world_safe_locs = wow_data::WorldSafeLocStore::from_rows_like_cpp([], &map_store).0;
    let mut shape_data =
        [0.0; wow_data::area_trigger_template::MAX_AREATRIGGER_ENTITY_DATA_LIKE_CPP];
    shape_data[0] = 4.0;
    shape_data[1] = 7.0;

    wow_data::AreaTriggerTemplateStore::from_rows_like_cpp(
        [wow_data::AreaTriggerTemplateRowLikeCpp {
            id: template_id,
            is_custom: false,
            flags: wow_data::area_trigger_template::AREATRIGGER_FLAG_IS_SERVER_SIDE_LIKE_CPP,
        }],
        [],
        [],
        [],
        [wow_data::AreaTriggerCreatePropertiesRowLikeCpp {
            id: create_properties_id,
            is_custom: false,
            area_trigger_id: template_id,
            is_areatrigger_custom: false,
            flags:
                wow_data::area_trigger_template::AREATRIGGER_CREATE_PROPERTIES_FLAG_UNK3_LIKE_CPP,
            move_curve_id: 0,
            scale_curve_id: 0,
            morph_curve_id: 0,
            facing_curve_id: 0,
            anim_id: 11,
            anim_kit_id: 22,
            decal_properties_id: 77,
            time_to_target: 0,
            time_to_target_scale: 0,
            shape: wow_data::area_trigger_template::AREATRIGGER_SHAPE_SPHERE_LIKE_CPP,
            shape_data,
            script_name: String::new(),
        }],
        [],
        &world_safe_locs,
        |_| true,
        |_| wow_data::ScriptIdLikeCpp(0),
    )
    .store
}

pub(super) fn variable_loaded_grid_creature_respawn_caches_like_cpp(
    entry: u32,
) -> LoadedGridCreatureRespawnCachesLikeCpp {
    variable_loaded_grid_creature_respawn_caches_with_vehicle_id_like_cpp(entry, 0)
}

pub(super) fn variable_loaded_grid_creature_respawn_caches_with_vehicle_id_like_cpp(
    entry: u32,
    vehicle_id: u32,
) -> LoadedGridCreatureRespawnCachesLikeCpp {
    variable_loaded_grid_creature_respawn_caches_with_vehicle_id_and_difficulty_like_cpp(
        entry, vehicle_id, 2,
    )
}

pub(super) fn variable_loaded_grid_creature_respawn_caches_with_vehicle_id_and_difficulty_like_cpp(
    entry: u32,
    vehicle_id: u32,
    difficulty_id: u8,
) -> LoadedGridCreatureRespawnCachesLikeCpp {
    LoadedGridCreatureRespawnCachesLikeCpp {
        realm_id: 1,
        template_store: Arc::new(
            wow_data::CreatureTemplateLifecycleStoreLikeCpp::from_templates([
                wow_data::CreatureTemplateLifecycleRecordLikeCpp {
                    entry,
                    name: "Variable Level Live Creature".to_string(),
                    ai_name: String::new(),
                    script_name: String::new(),
                    required_expansion: 2,
                    faction: 35,
                    npc_flags: 0,
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
                    vehicle_id,
                    movement_type: 1,
                    ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
                    swim_allowed: true,
                    flight_movement_type: 0,
                    rooted: false,
                    chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
                    random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
                    interaction_pause_timer_ms:
                        wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
                    flags_extra: 0,
                    string_id: String::new(),
                    regen_health: true,
                    spells: [0; 8],
                    models: vec![wow_data::CreatureTemplateLifecycleModelLikeCpp {
                        creature_display_id: 111,
                        display_scale: 1.0,
                        probability: 100.0,
                    }],
                },
            ]),
        ),
        sparring_store: Arc::new(wow_data::CreatureTemplateSparringStoreLikeCpp::default()),
        difficulty_store: Arc::new(wow_data::CreatureDifficultyStoreLikeCpp::from_records(
            [wow_data::CreatureDifficultyRecordLikeCpp {
                entry,
                difficulty_id,
                min_level: 18,
                max_level: 20,
                health_scaling_expansion: -1,
                health_modifier: 2.0,
                mana_modifier: 1.0,
                armor_modifier: 1.0,
                damage_modifier: 1.0,
                creature_difficulty_id: 0,
                type_flags: 0,
                type_flags2: 0,
                loot_id: 0,
                pickpocket_loot_id: 0,
                skin_loot_id: 0,
                gold_min: 0,
                gold_max: 0,
                static_flags: [0; 8],
            }],
            |_| 1.0,
        )),
        base_stats_store: Arc::new(wow_data::CreatureBaseStatsStoreLikeCpp::from_records([
            (18, 1, creature_base_stats_record_like_cpp(180)),
            (19, 1, creature_base_stats_record_like_cpp(190)),
            (20, 1, creature_base_stats_record_like_cpp(200)),
        ])),
        chr_classes_store: Arc::new(
            wow_data::character_progression::ChrClassesStore::from_entries([]),
        ),
        power_type_store: Arc::new(
            wow_data::character_progression::PowerTypeStore::from_entries([]),
        ),
        health_rates: wow_data::CreatureClassificationHealthRatesLikeCpp::default(),
        display_store: Arc::new(wow_data::CreatureDisplayInfoStore::from_entries([])),
        model_store: Arc::new(wow_data::CreatureModelDataStore::from_entries([])),
        model_info_store: Arc::new(wow_data::CreatureModelInfoStoreLikeCpp::from_entries([
            wow_data::CreatureModelInfoLikeCpp {
                display_id: 111,
                bounding_radius: 0.0,
                combat_reach: 1.5,
                display_id_other_gender: 0,
                is_trigger: false,
            },
            wow_data::CreatureModelInfoLikeCpp {
                display_id: 999,
                bounding_radius: 0.0,
                combat_reach: 1.5,
                display_id_other_gender: 0,
                is_trigger: false,
            },
        ])),
        creature_equipment_store: Arc::new(wow_data::CreatureEquipmentStoreLikeCpp::default()),
        creature_addon_store: Arc::new(wow_data::CreatureAddonStoreLikeCpp::default()),
        spell_x_spell_visual_store: Arc::new(wow_data::SpellXSpellVisualStore::from_entries([])),
        vehicle_store: Arc::new(vehicle_store_for_loaded_grid_test(vehicle_id)),
        vehicle_seat_store: Arc::new(vehicle_seat_store_for_loaded_grid_test()),
        vehicle_accessory_store: Arc::new(wow_data::VehicleAccessoryStoreLikeCpp::from_parts(
            [],
            [],
        )),
        gameobject_template_store: Arc::new(
            wow_data::GameObjectTemplateLifecycleStoreLikeCpp::default(),
        ),
        gameobject_override_store: Arc::new(
            wow_data::GameObjectOverrideLifecycleStoreLikeCpp::default(),
        ),
    }
}

pub(super) fn vehicle_store_for_loaded_grid_test(vehicle_id: u32) -> wow_data::VehicleStore {
    if vehicle_id == 0 {
        return wow_data::VehicleStore::from_entries([]);
    }
    let mut seat_ids = [0u16; 8];
    seat_ids[0] = 700;
    seat_ids[2] = 701;
    wow_data::VehicleStore::from_entries([wow_data::VehicleEntry {
        id: vehicle_id,
        flags: 0,
        flags_b: 0,
        seat_ids,
    }])
}

pub(super) fn vehicle_seat_store_for_loaded_grid_test() -> wow_data::VehicleSeatStore {
    wow_data::VehicleSeatStore::from_entries([
        wow_data::VehicleSeatEntry {
            id: 700,
            attachment_offset_x: 0.0,
            attachment_offset_y: 0.0,
            attachment_offset_z: 0.0,
            flags: wow_data::VEHICLE_SEAT_FLAG_CAN_ENTER_OR_EXIT,
            flags_b: 0,
            flags_c: 0,
        },
        wow_data::VehicleSeatEntry {
            id: 701,
            attachment_offset_x: 0.0,
            attachment_offset_y: 0.0,
            attachment_offset_z: 0.0,
            flags: 0,
            flags_b: 0,
            flags_c: 0,
        },
    ])
}

pub(super) fn creature_base_stats_record_like_cpp(
    base_health: u32,
) -> wow_data::CreatureBaseStatsRecordLikeCpp {
    wow_data::CreatureBaseStatsRecordLikeCpp {
        base_health: [base_health / 4, base_health / 2, base_health],
        base_mana: 50,
        base_armor: 0,
        attack_power: 0,
        ranged_attack_power: 0,
        base_damage: [1.0, 2.0, 3.0],
    }
}

pub(super) fn test_spawn_metadata<const N: usize>(
    groups: [(u32, u32); N],
) -> super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp {
    test_spawn_metadata_with_flags(
        groups.map(|(group_id, map_id)| (group_id, map_id, SpawnGroupFlags::NONE)),
    )
}

pub(super) fn test_spawn_metadata_with_flags<const N: usize>(
    groups: [(u32, u32, SpawnGroupFlags); N],
) -> super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp {
    let mut store = SpawnStore::new();
    let mut templates = BTreeMap::new();
    let mut rows = Vec::new();
    for (index, (group_id, map_id, flags)) in groups.into_iter().enumerate() {
        templates.insert(
            group_id,
            SpawnGroupTemplateData {
                group_id,
                name: format!("test group {group_id}"),
                map_id: wow_map::spawn::SPAWNGROUP_MAP_UNSET,
                flags,
            },
        );
        let spawn_id = u64::try_from(index).expect("test index fits") + 1;
        let spawn = test_spawn(spawn_id, map_id);
        store.add_object_spawn(&spawn, |_| false);
        rows.push(SpawnGroupMemberRow {
            group_id,
            spawn_type: SpawnObjectType::Creature as u8,
            spawn_id,
        });
    }
    store.apply_spawn_groups_like_cpp(&mut templates, rows);
    super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, templates)
}

pub(super) fn test_spawn_metadata_with_explicit_spawn_ids<const N: usize>(
    groups: [(u32, u32, SpawnGroupFlags, u64); N],
) -> super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp {
    let mut store = SpawnStore::new();
    let mut templates = BTreeMap::new();
    let mut rows = Vec::new();
    for (group_id, map_id, flags, spawn_id) in groups {
        templates.insert(
            group_id,
            SpawnGroupTemplateData {
                group_id,
                name: format!("test group {group_id}"),
                map_id: wow_map::spawn::SPAWNGROUP_MAP_UNSET,
                flags,
            },
        );
        let spawn = test_spawn(spawn_id, map_id);
        store.add_object_spawn(&spawn, |_| false);
        rows.push(SpawnGroupMemberRow {
            group_id,
            spawn_type: SpawnObjectType::Creature as u8,
            spawn_id,
        });
    }
    store.apply_spawn_groups_like_cpp(&mut templates, rows);
    super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, templates)
}

pub(super) fn test_spawn(spawn_id: u64, map_id: u32) -> SpawnData {
    SpawnData {
        object_type: SpawnObjectType::Creature,
        spawn_id,
        map_id,
        db_data: true,
        spawn_group: SpawnGroupTemplateData::default_group(),
        id: 42,
        spawn_point: SpawnPosition::new(0.0, 0.0, 0.0, 0.0),
        phase_use_flags: 0,
        phase_id: 0,
        phase_group: 0,
        terrain_swap_map: -1,
        pool_id: 0,
        spawn_time_secs: 120,
        spawn_difficulties: vec![0],
        script_id: 0,
        string_id: String::new(),
    }
}

pub(super) fn mapid_condition(spawn_group_id: u32, expected_map_id: u32) -> Condition {
    Condition {
        source_type: ConditionSourceType::SpawnGroup,
        source_group: 0,
        source_entry: spawn_group_id as i32,
        source_id: 0,
        condition_type: ConditionType::MapId,
        condition_value1: expected_map_id,
        ..Condition::default()
    }
}
