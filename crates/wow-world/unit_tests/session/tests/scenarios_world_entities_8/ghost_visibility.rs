use super::*;

fn materialized_spawn(
    guid: ObjectGuid,
    type_flags: u32,
    npc_flags: u64,
    flags_extra: u32,
) -> wow_world_entities::MaterializedCreatureSpawnLikeCpp {
    let mut create_data = test_creature_create_data(guid, 9_101, 25);
    create_data.npc_flags = npc_flags;
    wow_world_entities::MaterializedCreatureSpawnLikeCpp {
        guid,
        position: Position::new(10.0, 20.0, 30.0, 1.0),
        create_data,
        min_damage: 3,
        max_damage: 5,
        aggro_radius: 20.0,
        loot_id: 0,
        skin_loot_id: 0,
        gold_min: 0,
        gold_max: 0,
        respawn_delay_secs: wow_entities::DEFAULT_RESPAWN_DELAY_SECS,
        selected_equipment_id: 0,
        original_equipment_id: 0,
        script_name: String::new(),
        string_id: None,
        addon: None,
        phase_use_flags: 0,
        phase_id: 0,
        phase_group_id: 0,
        terrain_swap_map: -1,
        flags_extra,
        type_flags,
        ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
        swim_allowed: true,
        flight_movement_type: 0,
        rooted: false,
        chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
        random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
        interaction_pause_timer_ms:
            wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        wander_distance: 0.0,
        default_movement_type: wow_entities::MovementGeneratorType::Idle,
        waypoint_path_id: 0,
    }
}

fn register_materialized_spawn(
    session: &mut crate::session::WorldSession,
    spawn: &wow_world_entities::MaterializedCreatureSpawnLikeCpp,
) {
    let (state, mut hub) = crate::session::split_world_entities_mut(session);
    state.register_materialized_creature_spawn_like_cpp(&mut hub, 571, spawn);
}

fn assert_visibility_masks(
    creature: &wow_entities::Creature,
    visibility: u32,
    detect: u32,
    name: &str,
) {
    let mut alive_seer = wow_entities::Unit::new(true);
    alive_seer.set_server_side_ghost_visibility_detect_like_cpp(0b01);
    let mut ghost_seer = wow_entities::Unit::new(true);
    ghost_seer.set_server_side_ghost_visibility_detect_like_cpp(0b10);
    let mut ghost_player = wow_entities::Unit::new(true);
    ghost_player.set_server_side_ghost_visibility_like_cpp(0b10);

    assert_eq!(
        alive_seer.can_see_or_detect_unit_like_cpp(creature.unit(), false, true, false),
        visibility & 0b01 != 0,
        "alive seer visibility for {name}"
    );
    assert_eq!(
        ghost_seer.can_see_or_detect_unit_like_cpp(creature.unit(), false, true, false),
        visibility & 0b10 != 0,
        "ghost seer visibility for {name}"
    );
    assert_eq!(
        creature
            .unit()
            .can_see_or_detect_unit_like_cpp(&ghost_player, false, false, true),
        detect & 0b10 != 0,
        "creature detection mask for {name}"
    );
}

#[test]
fn materialized_compat_registration_applies_type_flags_and_post_addon_override() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    session.set_canonical_map_manager(Arc::clone(&manager));
    manager
        .lock()
        .expect("canonical manager")
        .create_world_map(571, 0);
    session.core.current_map_id = 571;

    let visible = wow_constants::CreatureTypeFlags::VISIBLE_TO_GHOSTS.bits();
    let healer = u64::from(wow_constants::NPCFlags1::SPIRIT_HEALER.bits());
    let extra = wow_constants::CreatureFlagsExtra::GHOST_VISIBILITY.bits();
    let visible_guid = test_creature_guid(74_710);
    let healer_guid = test_creature_guid(74_711);
    let extra_guid = test_creature_guid(74_712);
    for spawn in [
        materialized_spawn(visible_guid, visible, 0, 0),
        materialized_spawn(healer_guid, visible, healer, 0),
        materialized_spawn(extra_guid, visible, 0, extra),
    ] {
        register_materialized_spawn(&mut session, &spawn);
    }

    let manager = manager.lock().expect("canonical manager");
    let visible_creature = manager
        .find_creature(571, 0, visible_guid)
        .expect("materialized visible-to-ghosts creature is canonical");
    assert_visibility_masks(&visible_creature, 0b11, 0b01, "TypeFlags visible");

    let healer_creature = manager
        .find_creature(571, 0, healer_guid)
        .expect("materialized healer creature is canonical");
    assert_visibility_masks(&healer_creature, 0b10, 0b10, "spirit healer addon override");

    let extra_creature = manager
        .find_creature(571, 0, extra_guid)
        .expect("materialized extra-flag creature is canonical");
    assert_visibility_masks(
        &extra_creature,
        0b10,
        0b10,
        "ghost-visibility extra flag override",
    );
}

fn visibility_spawn_row(
    entry: u32,
    difficulty: u8,
) -> wow_persistence::CreatureVisibilityPersistenceRowLikeCpp {
    wow_persistence::CreatureVisibilityPersistenceRowLikeCpp {
        spawn_guid: 74_713,
        entry,
        position: [20.0, 20.0, 0.0, 0.0],
        current_health: 100,
        current_mana: 0,
        model_id: 12_345,
        min_level: 20,
        faction: 0,
        template_npc_flags: 0,
        template_unit_flags: [0; 3],
        speed_walk: 1.0,
        speed_run: 1.0,
        scale: 1.0,
        unit_class: 1,
        flags_extra: 0,
        attack_time: [2_000; 2],
        template_display_id: 0,
        template_display_scale: 1.0,
        loot_id: 0,
        skin_loot_id: 0,
        gold: [0; 2],
        phase_use_flags: 0,
        phase_id: 0,
        phase_group_id: 0,
        terrain_swap_map: -1,
        ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
        swim_allowed: false,
        flight_movement_type: 0,
        rooted: false,
        chase_movement_type: 0,
        random_movement_type: 0,
        interaction_pause_timer_ms: 0,
        wander_distance: 0.0,
        effective_movement_type: 0,
        waypoint_path_id: 0,
        classification: 0,
        regen_health: false,
        spawn_npc_flags_override: None,
        spawn_unit_flags_override: [None; 3],
        equipment_id: 0,
        respawn_delay_secs: wow_entities::DEFAULT_RESPAWN_DELAY_SECS,
        spawn_difficulties: difficulty.to_string(),
        script_name: String::new(),
        string_id: None,
        vehicle_id: 0,
    }
}

#[test]
fn spawn_materializer_returns_type_flags_from_selected_difficulty_fallback() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    manager.lock().expect("canonical manager").create_map_entry(
        571,
        0,
        2,
        wow_map::ManagedMapKind::World,
    );
    session.set_canonical_map_manager(manager);
    session.core.current_map_id = 571;

    let entry = 9_102;
    let visible = wow_constants::CreatureTypeFlags::VISIBLE_TO_GHOSTS.bits();
    session.catalogs.creature_difficulty_store_like_cpp = Some(Arc::new(
        wow_data::CreatureDifficultyStoreLikeCpp::from_records_with_difficulty_fallbacks(
            [wow_data::CreatureDifficultyRecordLikeCpp {
                entry,
                difficulty_id: 3,
                type_flags: visible,
                ..wow_data::CreatureDifficultyRecordLikeCpp::default_fallback_like_cpp()
            }],
            |_| 1.0,
            [(2, 3)],
        ),
    ));
    session.set_creature_model_info_store(Arc::new(
        wow_data::CreatureModelInfoStoreLikeCpp::from_entries([
            wow_data::CreatureModelInfoLikeCpp {
                display_id: 12_345,
                bounding_radius: 1.0,
                combat_reach: 1.5,
                display_id_other_gender: 0,
                is_trigger: false,
            },
        ]),
    ));

    let catalogs = session.creature_spawn_catalogs_for_test_like_cpp();
    assert_eq!(session.core.current_map_difficulty_id_like_cpp(), 2);
    assert_eq!(
        catalogs.difficulty.get_like_cpp(entry, 2).difficulty_id,
        3,
        "the selected map difficulty uses the configured fallback record"
    );
    let spawn = wow_world_entities::materialize_creature_spawn_row_for_test(
        &mut session,
        &catalogs,
        571,
        &visibility_spawn_row(entry, 2),
        &Position::new(10.0, 10.0, 0.0, 0.0),
        100.0,
    )
    .expect("actual spawn-row materializer accepts the difficulty fallback row");

    assert_eq!(spawn.type_flags, visible);
}
