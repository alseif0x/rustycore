//! Persistence scenarios for [`super`].
//!
//! Split out of map_manager_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn loaded_grid_canonical_bridge_preserves_level_and_stats_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 29_715, 97_932);
    let position = Position::new(5875.25, 609.063, 650.368, 1.676);
    let template = wow_entities::CreatureTemplateLifecycleRecord {
        entry: 29_715,
        original_entry: 29_715,
        difficulty_id: 0,
        name: "Quartermaster".to_string(),
        ai_name: String::new(),
        script_name: String::new(),
        required_expansion: 2,
        unit_class: 1,
        trainer_class: 0,
        faction: 35,
        npc_flags: 0x280,
        display_id: 26_441,
        model_dimensions: Some(wow_entities::CreatureModelDimensions {
            bounding_radius: 0.389,
            combat_reach: 1.5,
        }),
        scale: 1.0,
        speed_walk: 1.0,
        speed_run: 1.14286,
        spells: [0; wow_entities::MAX_CREATURE_SPELLS],
        classification: 0,
        damage_school: wow_constants::spell::SpellSchools::Normal as u8,
        unit_flags: 0,
        unit_flags2: wow_constants::UnitFlags2::REGENERATE_POWER.bits(),
        unit_flags3: 0,
        flags_extra: 0,
        static_flags: [0; 8],
        creature_type: 7,
        type_flags: 0,
        loot_id: 21_779,
        skin_loot_id: 21_780,
        gold_min: 13,
        gold_max: 31,
        movement_type: wow_entities::MovementGeneratorType::Idle,
        ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
        swim_allowed: true,
        flight_movement_type: wow_constants::CreatureFlightMovementType::None as u8,
        rooted: false,
        chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
        random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
        interaction_pause_timer_ms:
            wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        min_level: 75,
        max_level: 75,
        equipment_id: 0,
        original_equipment_id: 0,
    };
    let spawn = wow_entities::CreatureSpawnLifecycleRecord {
        spawn_id: 97_932,
        map_id: 571,
        instance_id: 0,
        position,
        home_position: position,
        phase_id: None,
        phase_group: None,
        terrain_swap_map: None,
        spawn_group_id: None,
        spawn_group_name: None,
        pool_id: None,
        equipment_id: Some(0),
        original_equipment_id: Some(0),
        wander_distance: 0.0,
        respawn_delay: 120,
        respawn_time: 0,
        movement_type: wow_entities::MovementGeneratorType::Idle,
        string_id: None,
        is_active: true,
        inactive_by_spawn_group: false,
        duplicate_spawn_found: false,
        add_to_map: true,
        respawn_compatibility_mode: false,
    };
    let canonical = wow_entities::Creature::load_from_db_lifecycle(
        wow_entities::CreatureLoadFromDbLifecycleRecord {
            create: wow_entities::CreatureCreateLifecycleRecord {
                guid,
                entry: 29_715,
                map_id: 571,
                instance_id: 0,
                position,
                dynamic: false,
                vehicle_id: None,
                vehicle_kit_create_input: None,
                add_to_world_vehicle_reset_context: None,
                template,
                spawn: Some(spawn.clone()),
                selected_level: 75,
                stats: wow_entities::CreatureLifecycleStats::new(4_652, 4_652, 0, 0),
                selected_display_id: 26_441,
                selected_model_dimensions: Some(wow_entities::CreatureModelDimensions {
                    bounding_radius: 0.389,
                    combat_reach: 1.5,
                }),
                selected_equipment_id: 0,
                selected_original_equipment_id: 0,
                selected_virtual_items: [(0, 0, 0); 3],
                corpse_delay: 60,
                ignore_corpse_decay_ratio: false,
                addon: None,
            },
            spawn,
        },
    );

    let bridged = loaded_grid_creature_like_cpp(canonical, |_| None);
    let create_data = creature_create_data_like_cpp(&bridged);

    assert_eq!(bridged.level(), 75);
    assert_eq!(bridged.current_hp(), 4_652);
    assert_eq!(bridged.max_hp(), 4_652);
    assert_eq!(create_data.level, 75);
    assert_eq!(create_data.health, 4_652);
    assert_eq!(create_data.max_health, 4_652);
    assert_eq!(create_data.display_id, 26_441);
    assert_eq!(create_data.npc_flags, 0x280);
    assert_eq!(
        create_data.unit_flags2,
        wow_constants::UnitFlags2::REGENERATE_POWER.bits()
    );
    assert_eq!(create_data.speed_walk_rate, 1.0);
    assert_eq!(create_data.speed_run_rate, 1.14286);
    assert_eq!(bridged.ai_ownership().loot_id, 21_779);
    assert_eq!(bridged.ai_ownership().skin_loot_id, 21_780);
    assert_eq!(bridged.ai_ownership().gold_min, 13);
    assert_eq!(bridged.ai_ownership().gold_max, 31);
}
#[test]
fn loaded_grid_canonical_bridge_only_sets_vehicle_create_flag_for_real_vehicle_kit_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Vehicle, 0, 1, 571, 0, 29_715, 97_933);
    let position = Position::new(5875.25, 609.063, 650.368, 1.676);
    let template = wow_entities::CreatureTemplateLifecycleRecord {
        entry: 29_715,
        original_entry: 29_715,
        difficulty_id: 0,
        name: "Vehicle-shaped creature".to_string(),
        ai_name: String::new(),
        script_name: String::new(),
        required_expansion: 2,
        unit_class: 1,
        trainer_class: 0,
        faction: 35,
        npc_flags: 0,
        display_id: 26_441,
        model_dimensions: Some(wow_entities::CreatureModelDimensions {
            bounding_radius: 0.389,
            combat_reach: 1.5,
        }),
        scale: 1.0,
        speed_walk: 1.0,
        speed_run: 1.14286,
        spells: [0; wow_entities::MAX_CREATURE_SPELLS],
        classification: 0,
        damage_school: wow_constants::spell::SpellSchools::Normal as u8,
        unit_flags: 0,
        unit_flags2: 0,
        unit_flags3: 0,
        flags_extra: 0,
        static_flags: [0; 8],
        creature_type: 7,
        type_flags: 0,
        loot_id: 0,
        skin_loot_id: 0,
        gold_min: 0,
        gold_max: 0,
        movement_type: wow_entities::MovementGeneratorType::Idle,
        ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
        swim_allowed: true,
        flight_movement_type: wow_constants::CreatureFlightMovementType::None as u8,
        rooted: false,
        chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
        random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
        interaction_pause_timer_ms:
            wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        min_level: 75,
        max_level: 75,
        equipment_id: 0,
        original_equipment_id: 0,
    };
    let spawn = wow_entities::CreatureSpawnLifecycleRecord {
        spawn_id: 97_933,
        map_id: 571,
        instance_id: 0,
        position,
        home_position: position,
        phase_id: None,
        phase_group: None,
        terrain_swap_map: None,
        spawn_group_id: None,
        spawn_group_name: None,
        pool_id: None,
        equipment_id: Some(0),
        original_equipment_id: Some(0),
        wander_distance: 0.0,
        respawn_delay: 120,
        respawn_time: 0,
        movement_type: wow_entities::MovementGeneratorType::Idle,
        string_id: None,
        is_active: true,
        inactive_by_spawn_group: false,
        duplicate_spawn_found: false,
        add_to_map: true,
        respawn_compatibility_mode: false,
    };
    let canonical = wow_entities::Creature::load_from_db_lifecycle(
        wow_entities::CreatureLoadFromDbLifecycleRecord {
            create: wow_entities::CreatureCreateLifecycleRecord {
                guid,
                entry: 29_715,
                map_id: 571,
                instance_id: 0,
                position,
                dynamic: false,
                vehicle_id: Some(909),
                vehicle_kit_create_input: None,
                add_to_world_vehicle_reset_context: None,
                template,
                spawn: Some(spawn.clone()),
                selected_level: 75,
                stats: wow_entities::CreatureLifecycleStats::new(4_652, 4_652, 0, 0),
                selected_display_id: 26_441,
                selected_model_dimensions: Some(wow_entities::CreatureModelDimensions {
                    bounding_radius: 0.389,
                    combat_reach: 1.5,
                }),
                selected_equipment_id: 0,
                selected_original_equipment_id: 0,
                selected_virtual_items: [(0, 0, 0); 3],
                corpse_delay: 60,
                ignore_corpse_decay_ratio: false,
                addon: None,
            },
            spawn,
        },
    );

    assert_eq!(canonical.lifecycle_metadata().vehicle_id, Some(909));
    assert!(canonical.unit().subsystems().vehicle.kit.is_none());

    let bridged = loaded_grid_creature_like_cpp(canonical, |_| None);

    assert_eq!(creature_create_data_like_cpp(&bridged).vehicle_id, 0);
}
#[test]
fn pending_respawn_save_uses_cpp_respawn_table_statement() {
    let mut queue = CreatureRespawnQueueLikeCpp::default();
    let now = Instant::now();
    let now_secs = 1_700_000_000;
    let mut pending = make_pending_respawn(now + Duration::from_secs(45));
    pending.spawn_id = u64::from(u32::MAX) + 17;
    pending.map_id = 571;

    let mutation =
        save_pending_respawn_time_on_queue_like_cpp(&mut queue, 571, 0, &pending, now, now_secs)
            .expect("future creature respawn should queue CHAR_REP_RESPAWN");

    assert_eq!(
        mutation,
        RespawnPersistenceMutationLikeCpp::Save {
            key: RespawnPersistenceKeyLikeCpp {
                object_type_raw: 0,
                spawn_id: pending.spawn_id,
                map_id: 571,
                instance_id: 0,
            },
            respawn_time: now_secs + 45,
        }
    );

    assert_eq!(
        queue.persisted_respawn_time_like_cpp(SpawnObjectType::Creature, pending.spawn_id),
        Some(now_secs + 45)
    );
    let rows = queue.persisted_respawn_rows_like_cpp();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].respawn_time, now_secs + 45);
}
#[test]
fn push_respawn_keeps_persistent_and_synthetic_id_namespaces_separate_like_cpp() {
    let mut map = CreatureRespawnQueueLikeCpp::default();
    let now = Instant::now();

    let mut persistent = make_pending_respawn(now);
    persistent.spawn_id = 91;
    persistent.persistent_spawn = true;
    let mut synthetic = make_pending_respawn(now);
    synthetic.spawn_id = 91;
    synthetic.persistent_spawn = false;

    map.push_respawn(persistent);
    map.push_respawn(synthetic);

    assert_eq!(map.respawn_queue_len(), 2);
    let ready = map.drain_ready_respawns(now);
    assert_eq!(ready.len(), 2);
    assert!(ready.iter().any(|respawn| respawn.persistent_spawn));
    assert!(ready.iter().any(|respawn| !respawn.persistent_spawn));
}
