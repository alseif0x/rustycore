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
