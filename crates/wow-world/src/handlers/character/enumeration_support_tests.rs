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
