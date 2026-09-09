//! Creature scenarios for [`super`].
//!
//! Split out of character_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn sql_creature_template_speed_defaults_match_cpp_check_creature_template() {
    assert_eq!(normalize_creature_template_speed_walk_like_cpp(0.0), 1.0);
    assert_eq!(normalize_creature_template_speed_run_like_cpp(0.0), 1.14286);
    assert_eq!(normalize_creature_template_speed_walk_like_cpp(0.75), 0.75);
    assert_eq!(normalize_creature_template_speed_run_like_cpp(2.0), 2.0);
}
#[test]
fn creature_spawn_difficulties_filter_matches_spawn_mode_like_cpp() {
    assert!(spawn_difficulties_contains_spawn_mode_like_cpp("0", 0));
    assert!(spawn_difficulties_contains_spawn_mode_like_cpp("0,1", 1));
    assert!(!spawn_difficulties_contains_spawn_mode_like_cpp("1", 0));
    assert!(!spawn_difficulties_contains_spawn_mode_like_cpp("", 0));
}
#[test]
fn creature_spawn_difficulties_invalid_token_maps_to_none_like_cpp() {
    assert!(
        spawn_difficulties_contains_spawn_mode_like_cpp("bad", 0),
        "C++ ObjectMgr::ParseSpawnDifficulties maps invalid tokens to DIFFICULTY_NONE"
    );
    assert!(!spawn_difficulties_contains_spawn_mode_like_cpp("bad", 1));
}
#[test]
fn creature_create_hover_offset_matches_cpp_after_addon() {
    let base = Position::new(1.0, 2.0, 3.0, 4.0);
    let adjusted = creature_create_position_after_hover_offset_like_cpp(
        base,
        MovementFlag::HOVER.bits(),
        1.25,
    );

    assert_eq!(
        adjusted,
        Position::new(1.0, 2.0, 4.25, 4.0),
        "C++ Creature::Create calls LoadCreaturesAddon() then adds GetHoverOffset() to m_positionZ"
    );
    assert_eq!(
        creature_create_position_after_hover_offset_like_cpp(base, 0, 1.25),
        base,
        "C++ GetHoverOffset() is zero unless MOVEMENTFLAG_HOVER is set"
    );
}
#[test]
fn creature_create_rooted_movement_flag_matches_cpp_template_root() {
    let flags = MovementFlag::from_bits_retain(creature_create_movement_flags_like_cpp(0, true));
    assert!(
        flags.contains(MovementFlag::ROOT),
        "C++ Creature::LoadTemplateRoot -> Unit::SetRooted adds MOVEMENTFLAG_ROOT"
    );
    assert!(
        !flags.intersects(MovementFlag::MASK_MOVING),
        "C++ Unit::SetRooted removes MOVEMENTFLAG_MASK_MOVING before adding ROOT"
    );

    let hover_root = MovementFlag::from_bits_retain(creature_create_movement_flags_like_cpp(
        wow_constants::CreatureGroundMovementType::Hover as u8,
        true,
    ));
    assert!(hover_root.contains(MovementFlag::HOVER));
    assert!(hover_root.contains(MovementFlag::ROOT));
}
#[test]
fn creature_flags_choose_spawn_override_and_sanitize_like_cpp() {
    let (npc_flags, unit_flags, unit_flags2, unit_flags3) = choose_creature_flags_like_cpp(
        0x10,
        UnitFlags::CAN_SWIM.bits(),
        0,
        0,
        Some(0x20),
        Some(UnitFlags::IN_COMBAT.bits() | UnitFlags::IMMUNE_TO_PC.bits()),
        Some(u32::MAX),
        Some(u32::MAX),
        CreatureFlagsExtra::TRIGGER.bits(),
    );

    assert_eq!(
        npc_flags, 0x20,
        "C++ ObjectMgr::ChooseCreatureFlags prefers CreatureData optional npcflag over template npcflag"
    );
    assert_eq!(
        unit_flags & UnitFlags::IN_COMBAT.bits(),
        0,
        "C++ Creature::UpdateEntry clears UNIT_FLAG_IN_COMBAT for newly-created creatures"
    );
    assert_ne!(
        unit_flags & UnitFlags::IMMUNE_TO_PC.bits(),
        0,
        "allowed UNIT_FIELD_FLAGS bits survive DB sanitization"
    );
    assert_ne!(
        unit_flags & UnitFlags::UNINTERACTIBLE.bits(),
        0,
        "C++ Creature::UpdateEntry sets uninteractible for trigger creatures"
    );
    assert_eq!(unit_flags2, UNIT_FLAGS2_ALLOWED_LIKE_CPP);
    assert_eq!(unit_flags3, UNIT_FLAGS3_ALLOWED_LIKE_CPP);
}
#[test]
fn sql_creature_movement_type_random_requires_wander_distance_like_cpp() {
    assert_eq!(
        creature_movement_generator_type_from_db_like_cpp(1, 0.0),
        MovementGeneratorType::Idle,
        "C++ Creature::Create forces RANDOM_MOTION_TYPE to IDLE_MOTION_TYPE when m_wanderDistance is zero"
    );
    assert_eq!(
        creature_movement_generator_type_from_db_like_cpp(1, 6.0),
        MovementGeneratorType::Random,
        "C++ preserves RANDOM_MOTION_TYPE only when CreatureData::wander_distance is positive"
    );
    assert_eq!(
        creature_movement_generator_type_from_db_like_cpp(WAYPOINT_MOTION_TYPE_LIKE_CPP, 0.0),
        MovementGeneratorType::Waypoint
    );
    assert_eq!(
        normalized_creature_wander_distance_like_cpp(MovementGeneratorType::Idle, 6.0),
        0.0,
        "C++ ObjectMgr::LoadCreatures clears wander_distance when MovementType is idle"
    );
    assert_eq!(
        normalized_creature_wander_distance_like_cpp(MovementGeneratorType::Random, 6.0),
        6.0,
        "C++ ObjectMgr::LoadCreatures keeps positive wander_distance for random movement"
    );
}
#[tokio::test]
async fn quest_giver_status_tracked_supplied_creature_not_visible_sends_available_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    let mut store = store_with_quests(&[3001]);
    store.starter_quests.entry(9301).or_default().push(3001);
    session.set_quest_store(Arc::new(store));
    let guid = creature_guid(9301, 301);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9301);
    attach_map_manager(&mut session, manager);
    assert!(!session.client_visible_guids_like_cpp.contains(&guid));

    session
        .handle_quest_giver_status_tracked_query(tracked_query_packet(&[guid]))
        .await;

    assert_eq!(
        recv_status_multiple(&send_rx),
        vec![(guid, quest_giver_status::TRIVIAL)]
    );
}
#[tokio::test]
async fn creature_query_uses_typed_catalog_and_preserves_packet_projection_like_cpp() {
    let row = creature_query_catalog_row_like_cpp();
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    install_world_query_catalogs_like_cpp(&mut session, [row.clone()], [], [], []);

    session
        .handle_query_creature(QueryCreature { creature_id: 42 })
        .await;

    let mut names: [String; 4] = Default::default();
    names[0] = row.name;
    let expected = QueryCreatureResponse {
        creature_id: 42,
        allow: true,
        stats: Some(CreatureStats {
            title: row.subname,
            title_alt: row.title_alt,
            cursor_name: row.icon_name,
            civilian: row.civilian,
            leader: row.racial_leader,
            names,
            name_alts: Default::default(),
            flags: row.type_flags,
            creature_type: row.creature_type,
            creature_family: row.creature_family,
            classification: row.classification,
            proxy_creature_ids: row.kill_credits,
            display: CreatureDisplayStats {
                displays: vec![CreatureXDisplay {
                    creature_display_id: 19,
                    scale: 0.75,
                    probability: 0.25,
                }],
                total_probability: 0.25,
            },
            hp_multi: row.hp_multi,
            energy_multi: row.energy_multi,
            quest_items: Vec::new(),
            creature_movement_info_id: row.movement_id,
            health_scaling_expansion: 0,
            required_expansion: row.required_expansion,
            vignette_id: row.vignette_id,
            unit_class: row.unit_class,
            creature_difficulty_id: row.creature_difficulty_id,
            widget_set_id: row.widget_set_id,
            widget_set_unit_condition_id: row.widget_set_unit_condition_id,
        }),
    };
    assert_eq!(send_rx.try_recv().unwrap(), expected.to_bytes());
}
#[tokio::test]
async fn creature_query_missing_or_failed_catalog_emits_disallowed_response_like_cpp() {
    for with_empty_capability in [false, true] {
        let (mut session, send_rx) = make_session_with_send_capacity(1);
        if with_empty_capability {
            install_world_query_catalogs_like_cpp(&mut session, [], [], [], []);
        }

        session
            .handle_query_creature(QueryCreature { creature_id: 43 })
            .await;

        assert_eq!(
            send_rx.try_recv().unwrap(),
            QueryCreatureResponse {
                creature_id: 43,
                allow: false,
                stats: None,
            }
            .to_bytes()
        );
    }
}
#[test]
fn enum_character_pet_family_uses_creature_template_for_pet_classes_like_cpp() {
    let store = enum_pet_template_store(416, 8);

    assert_eq!(
        enum_character_pet_data_like_cpp(0, 0, CLASS_HUNTER_LIKE_CPP, 416, 1234, 27, Some(&store),),
        (1234, 27, 8)
    );
}
