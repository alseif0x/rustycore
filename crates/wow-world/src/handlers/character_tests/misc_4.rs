//! Misc scenarios for [`super`].
//!
//! Split out of character_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn character_enumeration_uses_typed_rows_and_keeps_cleanup_best_effort_like_cpp() {
    let port = CharacterEnumerationPortFixtureLikeCpp::new([
        CharacterEnumerationLoadOutcomeLikeCpp::Loaded {
            rows: vec![character_enumeration_row_like_cpp()],
            expired_ban_cleanup_error: Some("best-effort cleanup failed".to_owned()),
        },
    ]);
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    session.set_declined_names_used_like_cpp(true);
    session.set_character_enumeration_persistence_port_like_cpp(port.clone());

    session.handle_enum_characters().await;

    assert_eq!(
        port.requests(),
        vec![CharacterEnumerationRequestLikeCpp {
            account_id: 1,
            declined_names_used: true,
        }]
    );
    assert!(session.is_legit_character(&ObjectGuid::create_player(1, 42)));
    assert!(send_rx.try_recv().is_ok());
}
#[tokio::test]
async fn character_enumeration_query_failure_publishes_failure_and_no_legit_guid() {
    let port = CharacterEnumerationPortFixtureLikeCpp::new([
        CharacterEnumerationLoadOutcomeLikeCpp::Failed {
            reason: "query failed".to_owned(),
            expired_ban_cleanup_error: None,
        },
    ]);
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    session.set_character_enumeration_persistence_port_like_cpp(port);

    session.handle_enum_characters().await;

    assert!(!session.is_legit_character(&ObjectGuid::create_player(1, 42)));
    assert!(send_rx.try_recv().is_ok());
}
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
