#![cfg(test)]

mod builder;
mod resolver;

use super::*;
use wow_constants::PowerType;

fn position(x: f32, y: f32, z: f32, orientation: f32) -> Position {
    Position {
        x,
        y,
        z,
        orientation,
    }
}

fn template(entry: u32) -> ResolvedCreatureTemplateLikeCpp {
    ResolvedCreatureTemplateLikeCpp {
        entry,
        original_entry: entry - 1,
        difficulty_id: 2,
        name: "Loaded Grid Test Creature".to_string(),
        ai_name: "SmartAI".to_string(),
        script_name: "npc_loaded_grid_test".to_string(),
        required_expansion: 2,
        unit_class: 1,
        trainer_class: 0,
        faction: 35,
        npc_flags: 0x1_0000_0040,
        display_id: 9001,
        model_dimensions: Some(CreatureModelDimensions {
            bounding_radius: 0.7,
            combat_reach: 1.5,
        }),
        scale: 1.25,
        speed_walk: 1.0,
        speed_run: 1.14286,
        spells: [11, 22, 33, 44, 55, 66, 77, 88],
        classification: 4,
        damage_school: wow_constants::spell::SpellSchools::Fire as u8,
        sparring_health_pct: None,
        unit_flags: wow_constants::UnitFlags::IMMUNE_TO_NPC.bits(),
        unit_flags2: wow_constants::UnitFlags2::FEIGN_DEATH.bits(),
        unit_flags3: wow_constants::UnitFlags3::AI_OBSTACLE.bits(),
        flags_extra: 0x10,
        static_flags: [0; 8],
        creature_type: 0,
        type_flags: 0x20,
        loot_id: 7_001,
        skin_loot_id: 7_002,
        gold_min: 17,
        gold_max: 29,
        movement_type: MovementGeneratorType::Idle,
        ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
        swim_allowed: true,
        flight_movement_type: 0,
        rooted: false,
        chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
        random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
        interaction_pause_timer_ms:
            wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        min_level: 18,
        max_level: 20,
        equipment_id: 3,
        original_equipment_id: -2,
        vehicle_id: None,
        vehicle_kit_create_input: None,
        add_to_world_vehicle_reset_context: None,
        corpse_delay: 61,
        ignore_corpse_decay_ratio: true,
        addon: None,
    }
}

fn vehicle_template(entry: u32, vehicle_id: u32) -> ResolvedCreatureTemplateLikeCpp {
    ResolvedCreatureTemplateLikeCpp {
        vehicle_id: Some(vehicle_id),
        vehicle_kit_create_input: Some(VehicleKitCreateInputLikeCpp {
            vehicle_id,
            creature_entry: entry,
            loading: true,
            seat_defs: Vec::new(),
        }),
        ..template(entry)
    }
}

fn map_creature_guid(entry: u32, map_id: u16, counter: i64) -> ObjectGuid {
    ObjectGuid::create_creature_like_cpp(1, map_id, entry, counter)
}

fn map_vehicle_guid(entry: u32, map_id: u16, counter: i64) -> ObjectGuid {
    ObjectGuid::create_vehicle_like_cpp(1, map_id, entry, counter)
}

fn spawn(spawn_id: u64, entry: u32, add_to_map: bool) -> ResolvedCreatureSpawnLikeCpp {
    ResolvedCreatureSpawnLikeCpp {
        spawn_id,
        entry,
        map_id: 571,
        instance_id: 9,
        position: position(100.0, 200.0, 30.0, 1.57),
        home_position: position(101.0, 201.0, 31.0, 2.57),
        phase_id: Some(5),
        phase_group: Some(6),
        terrain_swap_map: Some(7),
        spawn_group_id: Some(8),
        spawn_group_name: Some("wintergrasp-test".to_string()),
        pool_id: Some(9),
        equipment_id: Some(4),
        original_equipment_id: Some(-4),
        wander_distance: 12.5,
        respawn_delay: 300,
        respawn_time: 123_456,
        movement_type: MovementGeneratorType::Waypoint,
        string_id: Some("loaded_grid_string".to_string()),
        is_active: false,
        inactive_by_spawn_group: true,
        duplicate_spawn_found: true,
        add_to_map,
        respawn_compatibility_mode: true,
        formation_info: None,
    }
}

fn selection(entry: u32) -> (u32, ResolvedCreatureRuntimeSelectionLikeCpp) {
    (
        entry,
        ResolvedCreatureRuntimeSelectionLikeCpp {
            selected_level: 19,
            stats: CreatureLifecycleStats {
                max_health: 1_234,
                health: 777,
                power_type: PowerType::Mana,
                base_mana: 456,
                max_power: 456,
                power: 123,
                min_damage: 12.0,
                max_damage: 34.0,
                combat_log: CreatureCombatLogStatsLikeCpp::default(),
            },
            selected_display_id: 9002,
            selected_model_dimensions: None,
            selected_equipment_id: 6,
            selected_original_equipment_id: -6,
            selected_virtual_items: [(0, 0, 0); 3],
        },
    )
}

fn db_backed_spawn(entry: u32) -> wow_map::SpawnData {
    wow_map::SpawnData {
        object_type: wow_map::SpawnObjectType::Creature,
        spawn_id: 70,
        map_id: 571,
        db_data: true,
        spawn_group: wow_map::SpawnGroupTemplateData {
            group_id: 22,
            name: "compat-group".to_string(),
            map_id: 571,
            flags: wow_map::SpawnGroupFlags::COMPATIBILITY_MODE,
        },
        id: entry,
        spawn_point: wow_map::SpawnPosition::new(1.0, 2.0, 3.0, 4.0),
        phase_use_flags: 0,
        phase_id: 0,
        phase_group: 6,
        terrain_swap_map: -1,
        pool_id: 0,
        spawn_time_secs: 90,
        spawn_difficulties: Vec::new(),
        script_id: 0,
        string_id: "spawn-string".to_string(),
    }
}

fn db_backed_template_store(entry: u32) -> CreatureTemplateLifecycleStoreLikeCpp {
    db_backed_template_store_with_regen(entry, true)
}

fn db_backed_template_store_with_regen(
    entry: u32,
    regen_health: bool,
) -> CreatureTemplateLifecycleStoreLikeCpp {
    db_backed_template_store_with_regen_and_vehicle(entry, regen_health, 0)
}

fn db_backed_template_store_with_regen_and_vehicle(
    entry: u32,
    regen_health: bool,
    vehicle_id: u32,
) -> CreatureTemplateLifecycleStoreLikeCpp {
    CreatureTemplateLifecycleStoreLikeCpp::from_templates([
        wow_data::CreatureTemplateLifecycleRecordLikeCpp {
            entry,
            name: "DB Creature".to_string(),
            ai_name: "AggressorAI".to_string(),
            script_name: "npc_db_creature".to_string(),
            required_expansion: 2,
            faction: 35,
            npc_flags: 0x1_0000_0040,
            speed_walk: 1.0,
            speed_run: 1.14286,
            scale: 1.25,
            classification: 1,
            damage_school: wow_constants::spell::SpellSchools::Nature as u8,
            unit_flags: wow_constants::UnitFlags::IMMUNE_TO_NPC.bits(),
            unit_flags2: wow_constants::UnitFlags2::FEIGN_DEATH.bits(),
            unit_flags3: wow_constants::UnitFlags3::AI_OBSTACLE.bits(),
            creature_type: 0,
            family: 0,
            trainer_class: 4,
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
            flags_extra: 0x40,
            string_id: "template-string".to_string(),
            regen_health,
            spells: [10, 20, 0, 0, 0, 0, 0, 0],
            models: vec![
                wow_data::CreatureTemplateLifecycleModelLikeCpp {
                    creature_display_id: 111,
                    display_scale: 1.0,
                    probability: 50.0,
                },
                wow_data::CreatureTemplateLifecycleModelLikeCpp {
                    creature_display_id: 222,
                    display_scale: 1.0,
                    probability: 50.0,
                },
            ],
        },
    ])
}

fn db_backed_difficulty_store(entry: u32) -> CreatureDifficultyStoreLikeCpp {
    db_backed_difficulty_store_with_static_flags(entry, [0; 8])
}

fn db_backed_difficulty_store_with_static_flags(
    entry: u32,
    static_flags: [u32; 8],
) -> CreatureDifficultyStoreLikeCpp {
    db_backed_difficulty_store_with_static_flags_and_loot(entry, static_flags, 0, 0, 0, 0)
}

fn db_backed_difficulty_store_with_static_flags_and_loot(
    entry: u32,
    static_flags: [u32; 8],
    loot_id: u32,
    skin_loot_id: u32,
    gold_min: u32,
    gold_max: u32,
) -> CreatureDifficultyStoreLikeCpp {
    CreatureDifficultyStoreLikeCpp::from_records(
        [wow_data::CreatureDifficultyRecordLikeCpp {
            entry,
            difficulty_id: 2,
            min_level: 18,
            max_level: 20,
            health_scaling_expansion: -1,
            health_modifier: 2.0,
            mana_modifier: 3.0,
            armor_modifier: 1.0,
            damage_modifier: 4.0,
            creature_difficulty_id: 0,
            type_flags: 0x55,
            type_flags2: 0,
            loot_id,
            pickpocket_loot_id: 0,
            skin_loot_id,
            gold_min,
            gold_max,
            static_flags,
        }],
        |_| 1.0,
    )
}

fn db_backed_fallback_difficulty_store_with_loot(
    entry: u32,
    static_flags: [u32; 8],
    loot_id: u32,
    skin_loot_id: u32,
    gold_min: u32,
    gold_max: u32,
) -> CreatureDifficultyStoreLikeCpp {
    CreatureDifficultyStoreLikeCpp::from_records_with_difficulty_fallbacks(
        [wow_data::CreatureDifficultyRecordLikeCpp {
            entry,
            difficulty_id: 0,
            min_level: 18,
            max_level: 20,
            health_scaling_expansion: -1,
            health_modifier: 2.0,
            mana_modifier: 3.0,
            armor_modifier: 1.0,
            damage_modifier: 4.0,
            creature_difficulty_id: 0,
            type_flags: 0x55,
            type_flags2: 0,
            loot_id,
            pickpocket_loot_id: 0,
            skin_loot_id,
            gold_min,
            gold_max,
            static_flags,
        }],
        |_| 1.0,
        [(2, 0)],
    )
}

fn db_backed_base_stats_store() -> CreatureBaseStatsStoreLikeCpp {
    CreatureBaseStatsStoreLikeCpp::from_records([(
        19,
        1,
        wow_data::CreatureBaseStatsRecordLikeCpp {
            base_health: [10, 20, 100],
            base_mana: 50,
            base_armor: 123,
            attack_power: 456,
            ranged_attack_power: 789,
            base_damage: [1.0, 2.0, 5.0],
        },
    )])
}

fn empty_display_stores() -> (CreatureDisplayInfoStore, CreatureModelDataStore) {
    (
        CreatureDisplayInfoStore::from_entries([]),
        CreatureModelDataStore::from_entries([]),
    )
}

fn loaded_grid_model_info_store_like_cpp() -> CreatureModelInfoStoreLikeCpp {
    CreatureModelInfoStoreLikeCpp::from_entries([999, 111, 222, 11_686].map(|display_id| {
        let (bounding_radius, combat_reach) = match display_id {
            999 => (0.45, 1.75),
            111 => (0.70, 1.50),
            222 => (0.90, 2.25),
            11_686 => (0.0, 1.5),
            _ => unreachable!("fixture only builds known display ids"),
        };
        wow_data::CreatureModelInfoLikeCpp {
            display_id,
            bounding_radius,
            combat_reach,
            display_id_other_gender: 0,
            is_trigger: display_id == 11_686,
        }
    }))
}

struct TestLoadedGridCreatureRandomLikeCpp {
    selected_level: u8,
    weighted_model_roll: f32,
    other_gender_roll_zero: bool,
}

impl Default for TestLoadedGridCreatureRandomLikeCpp {
    fn default() -> Self {
        Self {
            selected_level: 19,
            weighted_model_roll: 0.0,
            other_gender_roll_zero: false,
        }
    }
}

impl CreatureModelSelectionRandomLikeCpp for TestLoadedGridCreatureRandomLikeCpp {
    fn weighted_model_roll_like_cpp(&mut self, _total_weight: f32) -> f32 {
        self.weighted_model_roll
    }

    fn other_gender_roll_zero_like_cpp(&mut self) -> bool {
        self.other_gender_roll_zero
    }
}

impl LoadedGridCreatureRandomSourceLikeCpp for TestLoadedGridCreatureRandomLikeCpp {
    fn select_creature_level_like_cpp(&mut self, min_level: u8, max_level: u8) -> u8 {
        assert_eq!((min_level, max_level), (18, 20));
        self.selected_level
    }
}
