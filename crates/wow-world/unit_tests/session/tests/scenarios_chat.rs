//! Session scenarios exercising the represented chat responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn update_zone_linked_chat_sets_city_rest_only_when_not_hostile_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 0xE1A2);
    session.ensure_login_player_controller_like_cpp(
        player_guid,
        "CityRest".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    );
    session.set_player_zone_area_like_cpp(10, 100);
    session.set_area_table_store(Arc::new(wow_data::AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 20,
            continent_id: 571,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: wow_data::AREA_FLAG_LINKED_CHAT_LIKE_CPP,
        },
        wow_data::AreaTableEntry {
            id: 30,
            continent_id: 571,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
        wow_data::AreaTableEntry {
            id: 40,
            continent_id: 571,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: wow_data::AREA_FLAG_LINKED_CHAT_LIKE_CPP | AREA_FLAG_FREE_FOR_ALL_PVP_LIKE_CPP,
        },
        wow_data::AreaTableEntry {
            id: 50,
            continent_id: 571,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: wow_data::AREA_FLAG_LINKED_CHAT_LIKE_CPP | wow_data::AREA_FLAG_NO_PVP_LIKE_CPP,
        },
    ])));

    session.set_player_pvp_hostile_like_cpp(true);
    assert!(session.update_zone_represented_like_cpp(20, 101));
    assert!(session.represented_is_resting_like_cpp());
    assert!(
        !session.player_pvp_hostile_like_cpp,
        "C++ Player::UpdateZone recalculates pvpInfo.IsHostile before city rest"
    );

    assert!(session.update_zone_represented_like_cpp(40, 102));
    assert!(
        session.represented_is_resting_like_cpp(),
        "C++ leaves an existing city-rest flag untouched in hostile LinkedChat zones"
    );
    assert!(session.player_pvp_hostile_like_cpp);

    assert!(session.update_zone_represented_like_cpp(50, 103));
    assert!(session.represented_is_resting_like_cpp());

    session.set_player_pvp_hostile_like_cpp(false);
    assert!(session.update_zone_represented_like_cpp(30, 104));
    assert!(!session.represented_is_resting_like_cpp());
}
#[test]
fn chat_afk_sets_player_flag_and_auto_reply_like_cpp() {
    let (mut session, _, guid) = session_with_canonical_player_for_away_like_cpp();

    assert!(
        session.apply_chat_away_mode_like_cpp(PlayerAwayModeLikeCpp::Afk, "back soon".to_string())
    );

    let reply = session.auto_reply_msg_like_cpp();
    assert_eq!(reply.as_deref(), Some("back soon"));
    assert_eq!(
        session.canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_AFK_LIKE_CPP),
        Some(true)
    );
    assert_eq!(
        session.canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_DND_LIKE_CPP),
        Some(false)
    );
}
#[test]
fn chat_afk_empty_text_uses_cpp_default_auto_reply_like_cpp() {
    let (mut session, _, guid) = session_with_canonical_player_for_away_like_cpp();

    assert!(session.apply_chat_away_mode_like_cpp(PlayerAwayModeLikeCpp::Afk, String::new()));

    let reply = session.auto_reply_msg_like_cpp();
    assert_eq!(reply.as_deref(), Some("Away from Keyboard"));
    assert_eq!(
        session.canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_AFK_LIKE_CPP),
        Some(true)
    );
    assert_eq!(
        session.canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_DND_LIKE_CPP),
        Some(false)
    );
}
#[test]
fn chat_dnd_clears_afk_like_cpp() {
    let (mut session, _, guid) = session_with_canonical_player_for_away_like_cpp();
    assert!(session.apply_chat_away_mode_like_cpp(PlayerAwayModeLikeCpp::Afk, "afk".to_string()));

    assert!(session.apply_chat_away_mode_like_cpp(PlayerAwayModeLikeCpp::Dnd, "busy".to_string()));

    assert_eq!(session.auto_reply_msg_like_cpp().as_deref(), Some("busy"));
    assert_eq!(
        session.canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_AFK_LIKE_CPP),
        Some(false)
    );
    assert_eq!(
        session.canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_DND_LIKE_CPP),
        Some(true)
    );
}
#[test]
fn chat_dnd_empty_text_uses_cpp_default_auto_reply_like_cpp() {
    let (mut session, _, guid) = session_with_canonical_player_for_away_like_cpp();

    assert!(session.apply_chat_away_mode_like_cpp(PlayerAwayModeLikeCpp::Dnd, String::new()));

    let reply = session.auto_reply_msg_like_cpp();
    assert_eq!(reply.as_deref(), Some("Do not Disturb"));
    assert_eq!(
        session.canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_AFK_LIKE_CPP),
        Some(false)
    );
    assert_eq!(
        session.canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_DND_LIKE_CPP),
        Some(true)
    );
}
