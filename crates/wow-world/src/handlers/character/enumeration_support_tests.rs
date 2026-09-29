use super::*;

#[test]
fn enum_character_flags_keep_declined_names_config_gated_like_cpp() {
    let disabled = enum_character_flags_like_cpp(0, 0, 0, Some("Genitive"), false);
    let empty = enum_character_flags_like_cpp(0, 0, 0, Some(""), true);
    let enabled = enum_character_flags_like_cpp(0, 0, 0, Some("Genitive"), true);

    assert_eq!(disabled.flags & CHARACTER_FLAG_DECLINED_LIKE_CPP, 0);
    assert_eq!(empty.flags & CHARACTER_FLAG_DECLINED_LIKE_CPP, 0);
    assert_eq!(
        enabled.flags & CHARACTER_FLAG_DECLINED_LIKE_CPP,
        CHARACTER_FLAG_DECLINED_LIKE_CPP
    );
}

#[test]
fn enum_character_flags_suppress_ghost_by_resurrect_like_cpp() {
    let flags = enum_character_flags_like_cpp(
        PLAYER_FLAGS_GHOST_LIKE_CPP,
        AT_LOGIN_RESURRECT_LIKE_CPP,
        0,
        None,
        false,
    );

    assert_eq!(flags.flags & CHARACTER_FLAG_GHOST_LIKE_CPP, 0);
}

#[test]
fn enum_character_flags2_use_cpp_customize_values_and_priority() {
    let customize = enum_character_flags_like_cpp(0, AT_LOGIN_CUSTOMIZE_LIKE_CPP, 0, None, false);
    let faction = enum_character_flags_like_cpp(
        0,
        AT_LOGIN_CHANGE_FACTION_LIKE_CPP | AT_LOGIN_CHANGE_RACE_LIKE_CPP,
        0,
        None,
        false,
    );
    let race = enum_character_flags_like_cpp(0, AT_LOGIN_CHANGE_RACE_LIKE_CPP, 0, None, false);
    let first = enum_character_flags_like_cpp(0, AT_LOGIN_FIRST_LIKE_CPP, 0, None, false);

    assert_eq!(customize.flags2, CHAR_CUSTOMIZE_FLAG_CUSTOMIZE_LIKE_CPP);
    assert_eq!(faction.flags2, CHAR_CUSTOMIZE_FLAG_FACTION_LIKE_CPP);
    assert_eq!(race.flags2, CHAR_CUSTOMIZE_FLAG_RACE_LIKE_CPP);
    assert!(first.first_login);
}

#[test]
fn raw_player_flags_not_passed_directly() {
    let flags = enum_character_flags_like_cpp(0x02, 0, 0, None, false);

    assert_eq!(flags.flags, 0);
}

#[test]
fn enum_character_flags_do_not_map_resting_like_cpp() {
    let flags = enum_character_flags_like_cpp(0x20, 0, 0, None, false);

    assert_eq!(flags.flags, 0);
}

#[test]
fn enum_character_flags_map_ghost_rename_billing_and_declined_like_cpp() {
    let flags = enum_character_flags_like_cpp(
        PLAYER_FLAGS_GHOST_LIKE_CPP,
        AT_LOGIN_RENAME_LIKE_CPP,
        42,
        Some("Genitive"),
        true,
    );

    assert_eq!(
        flags.flags,
        CHARACTER_FLAG_GHOST_LIKE_CPP
            | CHARACTER_FLAG_RENAME_LIKE_CPP
            | CHARACTER_FLAG_LOCKED_BY_BILLING_LIKE_CPP
            | CHARACTER_FLAG_DECLINED_LIKE_CPP
    );
    assert_eq!(flags.flags2, 0);
    assert!(!flags.first_login);
}

#[test]
fn enum_character_pet_family_uses_creature_template_for_pet_classes_like_cpp() {
    let store = enum_pet_template_store(416, 8);

    assert_eq!(
        enum_character_pet_data_like_cpp(0, 0, CLASS_HUNTER_LIKE_CPP, 416, 1234, 27, Some(&store),),
        (1234, 27, 8)
    );
}

#[test]
fn enum_character_pet_data_stays_zero_for_ghost_non_pet_class_or_missing_template_like_cpp() {
    let store = enum_pet_template_store(416, 8);

    assert_eq!(
        enum_character_pet_data_like_cpp(
            PLAYER_FLAGS_GHOST_LIKE_CPP,
            0,
            CLASS_HUNTER_LIKE_CPP,
            416,
            1234,
            27,
            Some(&store),
        ),
        (0, 0, 0)
    );
    assert_eq!(
        enum_character_pet_data_like_cpp(0, 0, 1, 416, 1234, 27, Some(&store)),
        (0, 0, 0)
    );
    assert_eq!(
        enum_character_pet_data_like_cpp(0, 0, CLASS_HUNTER_LIKE_CPP, 999, 1234, 27, Some(&store)),
        (0, 0, 0)
    );
}

fn enum_pet_template_store(
    entry: u32,
    family: u32,
) -> wow_data::CreatureTemplateLifecycleStoreLikeCpp {
    wow_data::CreatureTemplateLifecycleStoreLikeCpp::from_templates([
        wow_data::CreatureTemplateLifecycleRecordLikeCpp {
            entry,
            name: String::new(),
            ai_name: String::new(),
            script_name: String::new(),
            required_expansion: 0,
            faction: 0,
            npc_flags: 0,
            speed_walk: 1.0,
            speed_run: 1.0,
            scale: 1.0,
            classification: 0,
            damage_school: 0,
            unit_flags: 0,
            unit_flags2: 0,
            unit_flags3: 0,
            creature_type: 0,
            family,
            trainer_class: 0,
            unit_class: 0,
            vehicle_id: 0,
            movement_type: 0,
            ground_movement_type: 1,
            swim_allowed: true,
            flight_movement_type: 0,
            rooted: false,
            chase_movement_type: 0,
            random_movement_type: 0,
            interaction_pause_timer_ms: 180_000,
            flags_extra: 0,
            string_id: String::new(),
            regen_health: true,
            spells: [0; wow_data::MAX_CREATURE_SPELLS_LIKE_CPP],
            models: Vec::new(),
        },
    ])
}
