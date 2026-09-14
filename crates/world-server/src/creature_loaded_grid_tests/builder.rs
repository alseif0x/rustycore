#![cfg(test)]

use super::*;

#[test]
fn movement_type_random_requires_positive_wander_distance_like_cpp() {
    assert_eq!(
        movement_type_like_cpp(1, 0, 0.0),
        MovementGeneratorType::Idle,
        "C++ Creature::Create forces RANDOM_MOTION_TYPE to IDLE_MOTION_TYPE when m_wanderDistance is zero"
    );
    assert_eq!(
        movement_type_like_cpp(1, 0, 7.5),
        MovementGeneratorType::Random,
        "C++ preserves RANDOM_MOTION_TYPE only when CreatureData::wander_distance is positive"
    );
}
#[test]
fn loaded_grid_db_backed_builder_maps_spawn_template_runtime_like_cpp() {
    let entry = 12_400;
    let spawn = db_backed_spawn(entry);
    let runtime_row = CreatureSpawnRuntimeRowLikeCpp {
        spawn_id: spawn.spawn_id,
        model_id: 999,
        equipment_id: -7,
        wander_distance: 15.0,
        curhealth: 77,
        curmana: 33,
        movement_type: 2,
        npc_flags: Some(0x2_0000_0080),
        unit_flags: Some(wow_constants::UnitFlags::IMMUNE_TO_PC.bits()),
        unit_flags2: Some(wow_constants::UnitFlags2::IGNORE_REPUTATION.bits()),
        unit_flags3: Some(wow_constants::UnitFlags3::IGNORE_COMBAT.bits()),
        ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
        swim_allowed: true,
        flight_movement_type: wow_constants::CreatureFlightMovementType::CanFly as u8,
        rooted: false,
        chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
        random_movement_type: wow_constants::CreatureRandomMovementType::AlwaysRun as u8,
        interaction_pause_timer_ms:
            wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        string_id: "runtime-string".to_string(),
        spawn_time_secs: 300,
    };
    let (display_store, model_store) = empty_display_stores();
    let model_info_store = loaded_grid_model_info_store_like_cpp();
    let mut random = TestLoadedGridCreatureRandomLikeCpp::default();
    let mut static_flags = [0; 8];
    static_flags[0] = wow_constants::creature::CreatureStaticFlags::NO_MELEE_FLEE.bits();

    let (template, resolved_spawn, runtime) = build_loaded_grid_creature_inputs_from_db_like_cpp(
        &spawn,
        &runtime_row,
        &db_backed_template_store(entry),
        &db_backed_fallback_difficulty_store_with_loot(entry, static_flags, 21_779, 21_780, 13, 31),
        &db_backed_base_stats_store(),
        &CreatureClassificationHealthRatesLikeCpp::default(),
        &display_store,
        &model_store,
        &model_info_store,
        None,
        &CreatureAddonStoreLikeCpp::default(),
        2,
        9,
        123,
        true,
        None,
        &mut random,
    )
    .expect("DB-backed builder should compose resolver inputs");

    assert_eq!(template.entry, entry);
    assert_eq!(template.name, "DB Creature");
    assert_eq!(template.ai_name, "AggressorAI");
    assert_eq!(template.script_name, "npc_db_creature");
    assert_eq!(template.faction, 35);
    assert_eq!(template.trainer_class, 4);
    assert_eq!(template.npc_flags, 0x2_0000_0080);
    assert_eq!(template.spells[0..2], [10, 20]);
    assert_eq!(
        template.unit_flags,
        wow_constants::UnitFlags::IMMUNE_TO_PC.bits()
    );
    assert_eq!(
        template.unit_flags2, 0,
        "C++ ObjectMgr strips disallowed creature `unit_flags2` SQL bits before UpdateEntry"
    );
    assert_eq!(
        template.unit_flags3, 0,
        "C++ ObjectMgr strips disallowed creature `unit_flags3` SQL bits before UpdateEntry"
    );
    assert_eq!(template.static_flags[0], static_flags[0]);
    assert_eq!(template.loot_id, 21_779);
    assert_eq!(template.skin_loot_id, 21_780);
    assert_eq!((template.gold_min, template.gold_max), (13, 31));
    assert_eq!(template.difficulty_id, 2);
    assert_eq!(template.display_id, 999);
    assert_eq!(
        template.flight_movement_type,
        wow_constants::CreatureFlightMovementType::CanFly as u8
    );
    assert_eq!(
        template.random_movement_type,
        wow_constants::CreatureRandomMovementType::AlwaysRun as u8
    );
    assert_eq!(template.equipment_id, 0);
    assert_eq!(template.original_equipment_id, 0);
    assert_eq!(resolved_spawn.spawn_id, 70);
    assert_eq!(resolved_spawn.map_id, 571);
    assert_eq!(resolved_spawn.instance_id, 9);
    assert_eq!(resolved_spawn.phase_id, None);
    assert_eq!(resolved_spawn.phase_group, Some(6));
    assert_eq!(resolved_spawn.terrain_swap_map, None);
    assert_eq!(resolved_spawn.pool_id, None);
    assert_eq!(resolved_spawn.spawn_group_id, Some(22));
    assert!(resolved_spawn.respawn_compatibility_mode);
    assert_eq!(resolved_spawn.equipment_id, Some(0));
    assert_eq!(resolved_spawn.original_equipment_id, Some(0));
    assert_eq!(resolved_spawn.wander_distance, 15.0);
    assert_eq!(resolved_spawn.respawn_delay, 300);
    assert_eq!(resolved_spawn.respawn_time, 123);
    assert_eq!(resolved_spawn.string_id.as_deref(), Some("runtime-string"));
    assert!(resolved_spawn.add_to_map);
    assert_eq!(runtime.selected_level, 19);
    assert_eq!(runtime.selected_display_id, 999);
    assert_eq!(
        runtime.selected_model_dimensions,
        Some(CreatureModelDimensions {
            bounding_radius: 0.45,
            combat_reach: 1.75,
        })
    );
    assert_eq!(runtime.stats.max_health, 200);
    assert_eq!(runtime.stats.health, 200);
    assert_eq!(runtime.stats.max_power, 150);
    assert_eq!(runtime.stats.power, 150);
    assert_eq!(runtime.stats.min_damage, 20.0);
    assert_eq!(runtime.stats.max_damage, 30.0);
    assert_eq!(runtime.stats.combat_log.attack_power, 456);
    assert_eq!(runtime.stats.combat_log.ranged_attack_power, 789);
    assert_eq!(runtime.stats.combat_log.spell_power, 0);
    assert_eq!(runtime.stats.combat_log.armor, 123);

    let (default_loot_template, _, _) = build_loaded_grid_creature_inputs_from_db_like_cpp(
        &spawn,
        &runtime_row,
        &db_backed_template_store(entry),
        &CreatureDifficultyStoreLikeCpp::default(),
        &db_backed_base_stats_store(),
        &CreatureClassificationHealthRatesLikeCpp::default(),
        &display_store,
        &model_store,
        &model_info_store,
        None,
        &CreatureAddonStoreLikeCpp::default(),
        2,
        9,
        123,
        true,
        None,
        &mut random,
    )
    .expect("missing difficulty rows should use C++'s zero-valued fallback record");
    assert_eq!(default_loot_template.loot_id, 0);
    assert_eq!(default_loot_template.skin_loot_id, 0);
    assert_eq!(default_loot_template.gold_min, 0);
    assert_eq!(default_loot_template.gold_max, 0);

    let chr_classes_store =
        ChrClassesStore::from_entries([wow_data::character_progression::ChrClassesEntry {
            id: 1,
            display_power: PowerType::Focus as u8,
            ..Default::default()
        }]);
    let power_type_store =
        PowerTypeStore::from_entries([wow_data::character_progression::PowerTypeEntry {
            id: 7_777,
            name_global_string_tag: String::new(),
            cost_global_string_tag: String::new(),
            power_type_enum: PowerType::Focus as i8,
            min_power: 0,
            max_base_power: 100,
            center_power: 0,
            default_power: 25,
            display_modifier: 1,
            regen_interrupt_time_ms: 0,
            regen_peace: 0.0,
            regen_combat: 0.0,
            flags: 0x0020 | 0x0080,
        }]);
    let (_, _, focus_runtime) =
        build_loaded_grid_creature_inputs_with_power_stores_from_db_like_cpp(
            &spawn,
            &runtime_row,
            &db_backed_template_store(entry),
            &db_backed_fallback_difficulty_store_with_loot(entry, static_flags, 0, 0, 0, 0),
            &db_backed_base_stats_store(),
            &CreatureClassificationHealthRatesLikeCpp::default(),
            &display_store,
            &model_store,
            &model_info_store,
            None,
            &CreatureAddonStoreLikeCpp::default(),
            Some(&chr_classes_store),
            Some(&power_type_store),
            2,
            9,
            123,
            true,
            None,
            &mut random,
        )
        .expect("DB-backed builder should seed a non-mana C++ creature power");
    assert_eq!(focus_runtime.stats.power_type, PowerType::Focus);
    assert_eq!(focus_runtime.stats.base_mana, 50);
    assert_eq!(focus_runtime.stats.max_power, 300);
    assert_eq!(focus_runtime.stats.power, 25);
}
#[test]
fn loaded_grid_db_backed_builder_sanitizes_sql_unit_flags_like_cpp() {
    let entry = 12_401;
    let spawn = db_backed_spawn(entry);
    let runtime_row = CreatureSpawnRuntimeRowLikeCpp {
        spawn_id: spawn.spawn_id,
        model_id: 0,
        equipment_id: 0,
        wander_distance: 0.0,
        curhealth: 77,
        curmana: 33,
        movement_type: 0,
        npc_flags: None,
        unit_flags: Some(
            wow_constants::UnitFlags::SKINNABLE.bits()
                | wow_constants::UnitFlags::PREVENT_EMOTES_FROM_CHAT_TEXT.bits()
                | wow_constants::UnitFlags::CAN_SWIM.bits(),
        ),
        unit_flags2: Some(
            wow_constants::UnitFlags2::FEIGN_DEATH.bits()
                | wow_constants::UnitFlags2::REGENERATE_POWER.bits()
                | wow_constants::UnitFlags2::CANNOT_TURN.bits(),
        ),
        unit_flags3: Some(
            wow_constants::UnitFlags3::IGNORE_COMBAT.bits()
                | wow_constants::UnitFlags3::FAKE_DEAD.bits()
                | wow_constants::UnitFlags3::AI_OBSTACLE.bits(),
        ),
        ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
        swim_allowed: true,
        flight_movement_type: 0,
        rooted: false,
        chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
        random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
        interaction_pause_timer_ms:
            wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        string_id: String::new(),
        spawn_time_secs: 300,
    };
    let (display_store, model_store) = empty_display_stores();
    let model_info_store = loaded_grid_model_info_store_like_cpp();
    let mut random = TestLoadedGridCreatureRandomLikeCpp::default();

    let (template, _, _) = build_loaded_grid_creature_inputs_from_db_like_cpp(
        &spawn,
        &runtime_row,
        &db_backed_template_store(entry),
        &db_backed_difficulty_store(entry),
        &db_backed_base_stats_store(),
        &CreatureClassificationHealthRatesLikeCpp::default(),
        &display_store,
        &model_store,
        &model_info_store,
        None,
        &CreatureAddonStoreLikeCpp::default(),
        2,
        9,
        123,
        true,
        None,
        &mut random,
    )
    .expect("DB-backed builder should sanitize SQL creature unit flags");

    assert_eq!(
        template.unit_flags,
        wow_constants::UnitFlags::CAN_SWIM.bits(),
        "C++ ObjectMgr strips disallowed creature `unit_flags` SQL bits"
    );
    assert_eq!(
        template.unit_flags2,
        (wow_constants::UnitFlags2::REGENERATE_POWER | wow_constants::UnitFlags2::CANNOT_TURN)
            .bits(),
        "C++ ObjectMgr strips FEIGN_DEATH but keeps allowed `unit_flags2` SQL bits"
    );
    assert_eq!(
        template.unit_flags3,
        (wow_constants::UnitFlags3::FAKE_DEAD | wow_constants::UnitFlags3::AI_OBSTACLE).bits(),
        "C++ ObjectMgr strips disallowed `unit_flags3` SQL bits and keeps allowed fake-dead/AI-obstacle bits"
    );
}
#[test]
fn loaded_grid_db_backed_builder_applies_creature_equipment_like_cpp() {
    let entry = 12_402;
    let spawn = db_backed_spawn(entry);
    let runtime_row = CreatureSpawnRuntimeRowLikeCpp {
        spawn_id: spawn.spawn_id,
        model_id: 0,
        equipment_id: 6,
        wander_distance: 0.0,
        curhealth: 77,
        curmana: 33,
        movement_type: 0,
        npc_flags: None,
        unit_flags: None,
        unit_flags2: None,
        unit_flags3: None,
        ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
        swim_allowed: true,
        flight_movement_type: 0,
        rooted: false,
        chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
        random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
        interaction_pause_timer_ms:
            wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        string_id: String::new(),
        spawn_time_secs: 300,
    };
    let equipment_store = CreatureEquipmentStoreLikeCpp::from_entries([(
        entry,
        6,
        wow_data::CreatureEquipmentInfoLikeCpp {
            items: [
                wow_data::CreatureEquipmentItemLikeCpp {
                    item_id: 10_001,
                    appearance_mod_id: 3,
                    item_visual: 4,
                },
                wow_data::CreatureEquipmentItemLikeCpp {
                    item_id: 10_002,
                    appearance_mod_id: 5,
                    item_visual: 6,
                },
                wow_data::CreatureEquipmentItemLikeCpp::default(),
            ],
        },
    )]);
    let (display_store, model_store) = empty_display_stores();
    let model_info_store = loaded_grid_model_info_store_like_cpp();
    let mut random = TestLoadedGridCreatureRandomLikeCpp::default();

    let (template, resolved_spawn, runtime) = build_loaded_grid_creature_inputs_from_db_like_cpp(
        &spawn,
        &runtime_row,
        &db_backed_template_store(entry),
        &db_backed_difficulty_store(entry),
        &db_backed_base_stats_store(),
        &CreatureClassificationHealthRatesLikeCpp::default(),
        &display_store,
        &model_store,
        &model_info_store,
        Some(&equipment_store),
        &CreatureAddonStoreLikeCpp::default(),
        2,
        9,
        123,
        true,
        None,
        &mut random,
    )
    .expect("DB-backed equipment should resolve");

    assert_eq!(
        runtime.selected_virtual_items,
        [(10_001, 3, 4), (10_002, 5, 6), (0, 0, 0)]
    );

    let guid = map_creature_guid(entry, 571, spawn.spawn_id as i64);
    let resolved = CreatureLoadedGridLifecycleResolverLikeCpp::new(
        [template],
        [resolved_spawn],
        [(entry, runtime)],
    )
    .resolve_loaded_grid_creature_like_cpp(spawn.spawn_id, guid)
    .expect("resolved equipment creature");
    let virtual_items = resolved.creature.unit().data().virtual_items;
    assert_eq!(virtual_items[0].item_id, 10_001);
    assert_eq!(virtual_items[0].item_appearance_mod_id, 3);
    assert_eq!(virtual_items[0].item_visual, 4);
    assert_eq!(virtual_items[1].item_id, 10_002);
    assert_eq!(virtual_items[1].item_appearance_mod_id, 5);
    assert_eq!(virtual_items[1].item_visual, 6);
    assert_eq!(virtual_items[2].item_id, 0);
}
#[test]
fn loaded_grid_db_backed_builder_resolves_creature_addon_fallback_like_cpp() {
    let entry = 12_401;
    let spawn = db_backed_spawn(entry);
    let runtime_row = CreatureSpawnRuntimeRowLikeCpp {
        spawn_id: spawn.spawn_id,
        model_id: 999,
        equipment_id: 0,
        wander_distance: 5.0,
        curhealth: 10,
        curmana: 5,
        movement_type: 0,
        npc_flags: None,
        unit_flags: None,
        unit_flags2: None,
        unit_flags3: None,
        ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
        swim_allowed: true,
        flight_movement_type: 0,
        rooted: false,
        chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
        random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
        interaction_pause_timer_ms:
            wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        string_id: String::new(),
        spawn_time_secs: 20,
    };
    let addon_store = CreatureAddonStoreLikeCpp::from_rows_like_cpp(
        [wow_data::CreatureAddonRowLikeCpp {
            owner_id: spawn.spawn_id,
            path_id: 0,
            mount: 1234,
            stand_state: wow_constants::UnitStandStateType::Kneel as u8,
            anim_tier: 0,
            vis_flags: 0,
            sheath_state: 0,
            pvp_flags: wow_constants::UnitPvpFlags::PVP.bits(),
            emote: 77,
            ai_anim_kit: 0,
            movement_anim_kit: 0,
            melee_anim_kit: 0,
            visibility_distance_type: 0,
            auras: String::new(),
        }],
        [wow_data::CreatureAddonRowLikeCpp {
            owner_id: u64::from(entry),
            path_id: 0,
            mount: 5678,
            stand_state: wow_constants::UnitStandStateType::Sleep as u8,
            anim_tier: 0,
            vis_flags: 0,
            sheath_state: 0,
            pvp_flags: wow_constants::UnitPvpFlags::SANCTUARY.bits(),
            emote: 88,
            ai_anim_kit: 0,
            movement_anim_kit: 0,
            melee_anim_kit: 0,
            visibility_distance_type: 0,
            auras: String::new(),
        }],
        |spawn_id| spawn_id == spawn.spawn_id,
        |template_entry| template_entry == entry,
        |display_id| matches!(display_id, 1234 | 5678),
        |emote| matches!(emote, 77 | 88),
        |_| true,
        |_| false,
        |_| false,
        |_| 0,
        |_| 0,
        |_| 0,
    );
    let (display_store, model_store) = empty_display_stores();
    let model_info_store = loaded_grid_model_info_store_like_cpp();
    let mut random = TestLoadedGridCreatureRandomLikeCpp::default();

    let (template, _, _) = build_loaded_grid_creature_inputs_from_db_like_cpp(
        &spawn,
        &runtime_row,
        &db_backed_template_store(entry),
        &db_backed_difficulty_store(entry),
        &db_backed_base_stats_store(),
        &CreatureClassificationHealthRatesLikeCpp::default(),
        &display_store,
        &model_store,
        &model_info_store,
        None,
        &addon_store,
        2,
        0,
        0,
        false,
        None,
        &mut random,
    )
    .expect("DB-backed builder should resolve addon fallback");

    assert_eq!(
        template.addon,
        Some(CreatureAddonLifecycleRecordLikeCpp {
            path_id: 0,
            mount_display_id: 1234,
            stand_state: wow_constants::UnitStandStateType::Kneel,
            vis_flags: 0,
            anim_tier: 0,
            sheath_state: wow_constants::SheathState::Unarmed,
            pvp_flags: wow_constants::UnitPvpFlags::PVP,
            emote: 77,
            ai_anim_kit_id: 0,
            movement_anim_kit_id: 0,
            melee_anim_kit_id: 0,
            visibility_distance_type: wow_entities::VisibilityDistanceTypeLikeCpp::Normal,
            auras: Vec::new(),
            aura_applications: Vec::new(),
        }),
        "C++ Creature::GetCreatureAddon prefers creature_addon by spawn id over template addon"
    );
}
#[test]
fn loaded_grid_db_backed_builder_regen_true_scales_max_and_current_health_like_cpp() {
    let entry = 12_405;
    let spawn = db_backed_spawn(entry);
    let runtime_row = CreatureSpawnRuntimeRowLikeCpp {
        spawn_id: spawn.spawn_id,
        model_id: 999,
        equipment_id: 1,
        wander_distance: 0.0,
        curhealth: 77,
        curmana: 33,
        movement_type: 0,
        npc_flags: None,
        unit_flags: None,
        unit_flags2: None,
        unit_flags3: None,
        ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
        swim_allowed: true,
        flight_movement_type: 0,
        rooted: false,
        chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
        random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
        interaction_pause_timer_ms:
            wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        string_id: String::new(),
        spawn_time_secs: 20,
    };
    let health_rates = CreatureClassificationHealthRatesLikeCpp {
        elite: 2.0,
        ..CreatureClassificationHealthRatesLikeCpp::default()
    };
    let (display_store, model_store) = empty_display_stores();
    let model_info_store = loaded_grid_model_info_store_like_cpp();
    let mut random = TestLoadedGridCreatureRandomLikeCpp::default();

    let (_, _, runtime) = build_loaded_grid_creature_inputs_from_db_like_cpp(
        &spawn,
        &runtime_row,
        &db_backed_template_store_with_regen(entry, true),
        &db_backed_difficulty_store(entry),
        &db_backed_base_stats_store(),
        &health_rates,
        &display_store,
        &model_store,
        &model_info_store,
        None,
        &CreatureAddonStoreLikeCpp::default(),
        2,
        0,
        0,
        false,
        None,
        &mut random,
    )
    .expect("regen=true should use health-rate-scaled max health");

    assert_eq!(runtime.stats.max_health, 400);
    assert_eq!(runtime.stats.health, 400);
    assert_eq!(runtime.stats.max_power, 150);
    assert_eq!(runtime.stats.power, 150);
}
#[test]
fn loaded_grid_db_backed_builder_flags5_no_health_regen_preserves_initial_stats_like_cpp() {
    let entry = 12_406;
    let spawn = db_backed_spawn(entry);
    let runtime_row = CreatureSpawnRuntimeRowLikeCpp {
        spawn_id: spawn.spawn_id,
        model_id: 999,
        equipment_id: 1,
        wander_distance: 0.0,
        curhealth: 77,
        curmana: 33,
        movement_type: 0,
        npc_flags: None,
        unit_flags: None,
        unit_flags2: None,
        unit_flags3: None,
        ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
        swim_allowed: true,
        flight_movement_type: 0,
        rooted: false,
        chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
        random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
        interaction_pause_timer_ms:
            wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        string_id: String::new(),
        spawn_time_secs: 20,
    };
    let health_rates = CreatureClassificationHealthRatesLikeCpp {
        elite: 2.0,
        ..CreatureClassificationHealthRatesLikeCpp::default()
    };
    let mut static_flags = [0; 8];
    static_flags[4] = wow_constants::creature::CreatureStaticFlags5::NO_HEALTH_REGEN.bits();
    let (display_store, model_store) = empty_display_stores();
    let model_info_store = loaded_grid_model_info_store_like_cpp();
    let mut random = TestLoadedGridCreatureRandomLikeCpp::default();

    let (_, _, runtime) = build_loaded_grid_creature_inputs_from_db_like_cpp(
        &spawn,
        &runtime_row,
        &db_backed_template_store_with_regen(entry, false),
        &db_backed_difficulty_store_with_static_flags(entry, static_flags),
        &db_backed_base_stats_store(),
        &health_rates,
        &display_store,
        &model_store,
        &model_info_store,
        None,
        &CreatureAddonStoreLikeCpp::default(),
        2,
        0,
        0,
        false,
        None,
        &mut random,
    )
    .expect("flags5 NO_HEALTH_REGEN should preserve initial spawned stats");

    assert_eq!(runtime.stats.max_health, 400);
    assert_eq!(runtime.stats.health, runtime.stats.max_health);
    assert_eq!(runtime.stats.max_power, 150);
    assert_eq!(runtime.stats.power, runtime.stats.max_power);
    assert_ne!(runtime.stats.health, u64::from(runtime_row.curhealth) * 2);
    assert_ne!(
        runtime.stats.power,
        i32::try_from(runtime_row.curmana).unwrap()
    );
}
#[test]
fn loaded_grid_builder_handles_missing_template_and_difficulty_like_cpp() {
    let entry = 12_401;
    let spawn = db_backed_spawn(entry);
    let runtime_row = CreatureSpawnRuntimeRowLikeCpp {
        spawn_id: spawn.spawn_id,
        model_id: 0,
        equipment_id: 1,
        wander_distance: 0.0,
        curhealth: 0,
        curmana: 0,
        movement_type: 0,
        npc_flags: None,
        unit_flags: None,
        unit_flags2: None,
        unit_flags3: None,
        ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
        swim_allowed: true,
        flight_movement_type: 0,
        rooted: false,
        chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
        random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
        interaction_pause_timer_ms:
            wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        string_id: String::new(),
        spawn_time_secs: 10,
    };
    let (display_store, model_store) = empty_display_stores();
    let model_info_store = loaded_grid_model_info_store_like_cpp();
    let mut random = TestLoadedGridCreatureRandomLikeCpp::default();

    assert_eq!(
        build_loaded_grid_creature_inputs_from_db_like_cpp(
            &spawn,
            &runtime_row,
            &CreatureTemplateLifecycleStoreLikeCpp::default(),
            &db_backed_difficulty_store(entry),
            &db_backed_base_stats_store(),
            &CreatureClassificationHealthRatesLikeCpp::default(),
            &display_store,
            &model_store,
            &model_info_store,
            None,
            &CreatureAddonStoreLikeCpp::default(),
            2,
            0,
            0,
            false,
            None,
            &mut random,
        ),
        Err(CreatureLoadedGridResolveErrorLikeCpp::MissingTemplate { entry })
    );
    let mut random = TestLoadedGridCreatureRandomLikeCpp::default();
    let (template, _, runtime) = build_loaded_grid_creature_inputs_from_db_like_cpp(
        &spawn,
        &runtime_row,
        &db_backed_template_store(entry),
        &CreatureDifficultyStoreLikeCpp::default(),
        &db_backed_base_stats_store(),
        &CreatureClassificationHealthRatesLikeCpp::default(),
        &display_store,
        &model_store,
        &model_info_store,
        None,
        &CreatureAddonStoreLikeCpp::default(),
        2,
        0,
        0,
        false,
        None,
        &mut random,
    )
    .expect("C++ uses its static level-1 difficulty defaults when no row exists");
    assert_eq!((template.min_level, template.max_level), (1, 1));
    assert_eq!(template.static_flags, [0; 8]);
    assert_eq!(template.type_flags, 0);
    assert_eq!((template.loot_id, template.skin_loot_id), (0, 0));
    assert_eq!((template.gold_min, template.gold_max), (0, 0));
    assert_eq!(runtime.selected_level, 1);
}
#[test]
fn loaded_grid_db_backed_builder_uses_cpp_display_model_and_full_health_fallback_like_cpp() {
    let entry = 12_402;
    let spawn = db_backed_spawn(entry);
    let runtime_row = CreatureSpawnRuntimeRowLikeCpp {
        spawn_id: spawn.spawn_id,
        model_id: 0,
        equipment_id: 3,
        wander_distance: 0.0,
        curhealth: 0,
        curmana: 0,
        movement_type: 0,
        npc_flags: None,
        unit_flags: None,
        unit_flags2: None,
        unit_flags3: None,
        ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
        swim_allowed: true,
        flight_movement_type: 0,
        rooted: false,
        chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
        random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
        interaction_pause_timer_ms:
            wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        string_id: String::new(),
        spawn_time_secs: 20,
    };
    let (display_store, model_store) = empty_display_stores();
    let model_info_store = loaded_grid_model_info_store_like_cpp();
    let mut random = TestLoadedGridCreatureRandomLikeCpp::default();
    let (resolved_template, resolved_spawn, runtime) =
        build_loaded_grid_creature_inputs_from_db_like_cpp(
            &spawn,
            &runtime_row,
            &db_backed_template_store(entry),
            &CreatureDifficultyStoreLikeCpp::from_records(
                [wow_data::CreatureDifficultyRecordLikeCpp {
                    min_level: 19,
                    max_level: 19,
                    ..db_backed_difficulty_store(entry)
                        .get_like_cpp(entry, 2)
                        .clone()
                }],
                |_| 1.0,
            ),
            &db_backed_base_stats_store(),
            &CreatureClassificationHealthRatesLikeCpp::default(),
            &display_store,
            &model_store,
            &model_info_store,
            None,
            &CreatureAddonStoreLikeCpp::default(),
            2,
            0,
            0,
            false,
            None,
            &mut random,
        )
        .expect("first template model/full health fallback should resolve");

    assert_eq!(runtime.selected_display_id, 111);
    assert_eq!(
        runtime.selected_model_dimensions,
        Some(CreatureModelDimensions {
            bounding_radius: 0.70,
            combat_reach: 1.50,
        })
    );
    assert_eq!(runtime.stats.health, runtime.stats.max_health);
    assert_eq!(runtime.stats.power, runtime.stats.max_power);
    assert_eq!(
        resolved_template.corpse_delay, DEFAULT_CORPSE_DELAY_SECS,
        "C++ Creature constructor keeps a 60-second corpse delay for DB spawns"
    );
    assert_eq!(resolved_spawn.string_id.as_deref(), Some("spawn-string"));
    assert!(!resolved_spawn.add_to_map);
}
#[test]
fn loaded_grid_db_backed_builder_uses_weighted_cpp_display_model_like_cpp() {
    let entry = 12_407;
    let spawn = db_backed_spawn(entry);
    let runtime_row = CreatureSpawnRuntimeRowLikeCpp {
        spawn_id: spawn.spawn_id,
        model_id: 0,
        equipment_id: 0,
        wander_distance: 0.0,
        curhealth: 0,
        curmana: 0,
        movement_type: 0,
        npc_flags: None,
        unit_flags: None,
        unit_flags2: None,
        unit_flags3: None,
        ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
        swim_allowed: true,
        flight_movement_type: 0,
        rooted: false,
        chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
        random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
        interaction_pause_timer_ms:
            wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        string_id: String::new(),
        spawn_time_secs: 20,
    };
    let (display_store, model_store) = empty_display_stores();
    let model_info_store = loaded_grid_model_info_store_like_cpp();
    let mut random = TestLoadedGridCreatureRandomLikeCpp {
        weighted_model_roll: 50.01,
        ..TestLoadedGridCreatureRandomLikeCpp::default()
    };

    let (_, _, runtime) = build_loaded_grid_creature_inputs_from_db_like_cpp(
        &spawn,
        &runtime_row,
        &db_backed_template_store(entry),
        &CreatureDifficultyStoreLikeCpp::from_records(
            [wow_data::CreatureDifficultyRecordLikeCpp {
                min_level: 19,
                max_level: 19,
                ..db_backed_difficulty_store(entry)
                    .get_like_cpp(entry, 2)
                    .clone()
            }],
            |_| 1.0,
        ),
        &db_backed_base_stats_store(),
        &CreatureClassificationHealthRatesLikeCpp::default(),
        &display_store,
        &model_store,
        &model_info_store,
        None,
        &CreatureAddonStoreLikeCpp::default(),
        2,
        0,
        0,
        false,
        None,
        &mut random,
    )
    .expect("weighted C++ display model should resolve");

    assert_eq!(runtime.selected_display_id, 222);
    assert_eq!(
        runtime.selected_model_dimensions,
        Some(CreatureModelDimensions {
            bounding_radius: 0.90,
            combat_reach: 2.25,
        })
    );
}
#[test]
fn loaded_grid_db_backed_builder_regen_false_preserves_zero_health_and_db_mana_like_cpp() {
    let entry = 12_403;
    let spawn = db_backed_spawn(entry);
    let runtime_row = CreatureSpawnRuntimeRowLikeCpp {
        spawn_id: spawn.spawn_id,
        model_id: 0,
        equipment_id: 3,
        wander_distance: 0.0,
        curhealth: 0,
        curmana: 33,
        movement_type: 0,
        npc_flags: None,
        unit_flags: None,
        unit_flags2: None,
        unit_flags3: None,
        ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
        swim_allowed: true,
        flight_movement_type: 0,
        rooted: false,
        chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
        random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
        interaction_pause_timer_ms:
            wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        string_id: String::new(),
        spawn_time_secs: 20,
    };
    let (display_store, model_store) = empty_display_stores();
    let health_rates = CreatureClassificationHealthRatesLikeCpp {
        elite: 2.0,
        ..CreatureClassificationHealthRatesLikeCpp::default()
    };
    let model_info_store = loaded_grid_model_info_store_like_cpp();
    let mut random = TestLoadedGridCreatureRandomLikeCpp::default();

    let (_, _, runtime) = build_loaded_grid_creature_inputs_from_db_like_cpp(
        &spawn,
        &runtime_row,
        &db_backed_template_store_with_regen(entry, false),
        &db_backed_difficulty_store(entry),
        &db_backed_base_stats_store(),
        &health_rates,
        &display_store,
        &model_store,
        &model_info_store,
        None,
        &CreatureAddonStoreLikeCpp::default(),
        2,
        0,
        0,
        false,
        None,
        &mut random,
    )
    .expect("regen=false zero current health should preserve dead DB health");

    assert_eq!(runtime.stats.max_health, 400);
    assert_eq!(runtime.stats.health, 0);
    assert_eq!(runtime.stats.max_power, 150);
    assert_eq!(runtime.stats.power, 33);
}
#[test]
fn loaded_grid_db_backed_builder_regen_false_scales_current_health_and_min_one_like_cpp() {
    let entry = 12_404;
    let spawn = db_backed_spawn(entry);
    let (display_store, model_store) = empty_display_stores();
    let model_info_store = loaded_grid_model_info_store_like_cpp();
    let mut random = TestLoadedGridCreatureRandomLikeCpp::default();
    let health_rates = CreatureClassificationHealthRatesLikeCpp {
        elite: 0.25,
        ..CreatureClassificationHealthRatesLikeCpp::default()
    };

    let low_health_row = CreatureSpawnRuntimeRowLikeCpp {
        spawn_id: spawn.spawn_id,
        model_id: 0,
        equipment_id: 3,
        wander_distance: 0.0,
        curhealth: 1,
        curmana: 44,
        movement_type: 0,
        npc_flags: None,
        unit_flags: None,
        unit_flags2: None,
        unit_flags3: None,
        ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
        swim_allowed: true,
        flight_movement_type: 0,
        rooted: false,
        chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
        random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
        interaction_pause_timer_ms:
            wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        string_id: String::new(),
        spawn_time_secs: 20,
    };
    let (_, _, low_health_runtime) = build_loaded_grid_creature_inputs_from_db_like_cpp(
        &spawn,
        &low_health_row,
        &db_backed_template_store_with_regen(entry, false),
        &db_backed_difficulty_store(entry),
        &db_backed_base_stats_store(),
        &health_rates,
        &display_store,
        &model_store,
        &model_info_store,
        None,
        &CreatureAddonStoreLikeCpp::default(),
        2,
        0,
        0,
        false,
        None,
        &mut random,
    )
    .expect("regen=false non-zero current health should min-clamp after scaling");
    assert_eq!(low_health_runtime.stats.max_health, 50);
    assert_eq!(low_health_runtime.stats.health, 1);
    assert_eq!(low_health_runtime.stats.power, 44);

    let scaled_health_row = CreatureSpawnRuntimeRowLikeCpp {
        curhealth: 80,
        curmana: 55,
        ..low_health_row
    };
    let mut random = TestLoadedGridCreatureRandomLikeCpp::default();
    let (_, _, scaled_health_runtime) = build_loaded_grid_creature_inputs_from_db_like_cpp(
        &spawn,
        &scaled_health_row,
        &db_backed_template_store_with_regen(entry, false),
        &db_backed_difficulty_store(entry),
        &db_backed_base_stats_store(),
        &health_rates,
        &display_store,
        &model_store,
        &model_info_store,
        None,
        &CreatureAddonStoreLikeCpp::default(),
        2,
        0,
        0,
        false,
        None,
        &mut random,
    )
    .expect("regen=false current health should scale by classification health rate");
    assert_eq!(scaled_health_runtime.stats.max_health, 50);
    assert_eq!(scaled_health_runtime.stats.health, 20);
    assert_eq!(scaled_health_runtime.stats.power, 55);
}
#[test]
fn loaded_grid_db_backed_builder_preserves_vehicle_template_id_like_cpp() {
    let entry = 12_407;
    let spawn = db_backed_spawn(entry);
    let runtime_row = CreatureSpawnRuntimeRowLikeCpp {
        spawn_id: spawn.spawn_id,
        model_id: 999,
        equipment_id: 1,
        wander_distance: 0.0,
        curhealth: 0,
        curmana: 0,
        movement_type: 0,
        npc_flags: None,
        unit_flags: None,
        unit_flags2: None,
        unit_flags3: None,
        ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
        swim_allowed: true,
        flight_movement_type: 0,
        rooted: false,
        chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
        random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
        interaction_pause_timer_ms:
            wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        string_id: String::new(),
        spawn_time_secs: 20,
    };
    let (display_store, model_store) = empty_display_stores();
    let model_info_store = loaded_grid_model_info_store_like_cpp();
    let mut random = TestLoadedGridCreatureRandomLikeCpp::default();

    let (template, _, _) = build_loaded_grid_creature_inputs_from_db_like_cpp(
        &spawn,
        &runtime_row,
        &db_backed_template_store_with_regen_and_vehicle(entry, true, 77),
        &db_backed_difficulty_store(entry),
        &db_backed_base_stats_store(),
        &CreatureClassificationHealthRatesLikeCpp::default(),
        &display_store,
        &model_store,
        &model_info_store,
        None,
        &CreatureAddonStoreLikeCpp::default(),
        2,
        0,
        0,
        false,
        None,
        &mut random,
    )
    .expect("DB-backed vehicle template should compose resolver inputs");

    assert_eq!(template.vehicle_id, Some(77));
}
#[test]
fn loaded_grid_creature_lifecycle_resolver_maps_spawn_template_and_selection_like_cpp() {
    let entry = 12_345;
    let resolver = CreatureLoadedGridLifecycleResolverLikeCpp::new(
        [template(entry)],
        [spawn(55, entry, true)],
        [selection(entry)],
    );

    let map_object_guid = map_creature_guid(entry, 571, 55);
    let resolved = resolver
        .resolve_loaded_grid_creature_like_cpp(55, map_object_guid)
        .expect("resolver should build lifecycle record");
    let record = &resolved.lifecycle_record;
    let creature = &resolved.creature;
    let metadata = creature.lifecycle_metadata();

    assert_eq!(record.create.entry, entry);
    assert_eq!(record.create.guid, map_object_guid);
    assert_eq!(record.create.guid.high_type(), HighGuid::Creature);
    assert_eq!(u32::from(record.create.guid.map_id()), 571);
    assert_eq!(record.create.template.original_entry, entry - 1);
    assert_eq!(record.create.template.ai_name, "SmartAI");
    assert_eq!(record.create.template.script_name, "npc_loaded_grid_test");
    assert_eq!(record.create.template.required_expansion, 2);
    assert_eq!(record.create.template.npc_flags, 0x1_0000_0040);
    assert_eq!(
        record.create.template.damage_school,
        wow_constants::spell::SpellSchools::Fire as u8
    );
    assert_eq!(
        record.create.template.unit_flags,
        wow_constants::UnitFlags::IMMUNE_TO_NPC.bits()
    );
    assert_eq!(
        record.create.template.unit_flags2,
        wow_constants::UnitFlags2::FEIGN_DEATH.bits()
    );
    assert_eq!(
        record.create.template.unit_flags3,
        wow_constants::UnitFlags3::AI_OBSTACLE.bits()
    );
    assert_eq!(record.create.map_id, 571);
    assert_eq!(record.create.instance_id, 9);
    assert_eq!(record.spawn.spawn_id, 55);
    assert_eq!(record.spawn.position, position(100.0, 200.0, 30.0, 1.57));
    assert_eq!(
        record.spawn.home_position,
        position(101.0, 201.0, 31.0, 2.57)
    );
    assert_eq!(record.spawn.respawn_delay, 300);
    assert_eq!(record.spawn.respawn_time, 123_456);
    assert_eq!(
        record.spawn.movement_type,
        MovementGeneratorType::Waypoint,
        "C++ preserves spawn MovementType=WAYPOINT_MOTION_TYPE when no spawn-addon PathId=0 downgrade applied"
    );
    assert_eq!(
        record.spawn.string_id.as_deref(),
        Some("loaded_grid_string")
    );
    assert_eq!(record.spawn.spawn_group_id, Some(8));
    assert_eq!(record.spawn.pool_id, Some(9));
    assert!(record.spawn.inactive_by_spawn_group);
    assert!(record.spawn.duplicate_spawn_found);
    assert_eq!(record.spawn.equipment_id, Some(4));
    assert_eq!(record.spawn.original_equipment_id, Some(-4));
    assert_eq!(record.create.selected_level, 19);
    assert_eq!(record.create.stats.health, 777);
    assert_eq!(record.create.selected_display_id, 9002);
    assert_eq!(record.create.selected_model_dimensions, None);

    assert_eq!(metadata.ai_name, "SmartAI");
    assert_eq!(metadata.script_name, "npc_loaded_grid_test");
    assert_eq!(metadata.required_expansion, 2);
    assert_eq!(metadata.spawn_id, 55);
    assert_eq!(metadata.spawn_map_id, 571);
    assert_eq!(metadata.spawn_instance_id, 9);
    assert_eq!(metadata.spawn_position, position(100.0, 200.0, 30.0, 1.57));
    assert_eq!(metadata.home_position, position(101.0, 201.0, 31.0, 2.57));
    assert_eq!(metadata.phase_id, Some(5));
    assert_eq!(metadata.terrain_swap_map, Some(7));
    assert_eq!(
        metadata.spawn_group_name.as_deref(),
        Some("wintergrasp-test")
    );
    assert_eq!(metadata.pool_id, Some(9));
    assert!(!metadata.is_spawn_active);
    assert!(metadata.inactive_by_spawn_group);
    assert!(metadata.duplicate_spawn_found);
    assert_eq!(metadata.equipment_id, 4);
    assert_eq!(metadata.original_equipment_id, -4);
    assert_eq!(
        creature.melee_damage_school_mask(),
        1 << (wow_constants::spell::SpellSchools::Fire as u8)
    );
    assert_eq!(creature.ai_current_health(), 777);
    assert_eq!(creature.ai_max_health(), 1_234);
    assert_eq!(creature.ai_level(), 19);
    assert_eq!(creature.unit().npc_flags_like_cpp(), [0x40, 0x1]);
    assert_eq!(
        creature.unit().unit_flags_like_cpp(),
        wow_constants::UnitFlags::IMMUNE_TO_NPC
    );
    assert_eq!(
        creature.unit().unit_flags2_like_cpp(),
        wow_constants::UnitFlags2::FEIGN_DEATH
    );
    assert_eq!(
        creature.unit().unit_flags3_like_cpp(),
        wow_constants::UnitFlags3::AI_OBSTACLE
    );
    assert!(resolved.map_insertion_requested);
    assert!(resolved.map_object_record.is_some());
    assert!(
        resolved
            .map_object_record
            .as_ref()
            .and_then(MapObjectRecord::creature)
            .is_some()
    );
}
