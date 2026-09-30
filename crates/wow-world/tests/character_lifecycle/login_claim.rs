// Existing Character application scenarios, moved with original assertion operands.

use super::fixtures::session::make_session;
use super::fixtures::*;

#[test]
fn character_login_claim_allows_only_one_live_session_like_cpp() {
    let guid = ObjectGuid::create_player(1, 0x7FFF_FF01);
    let (mut first, _, _) = make_session();
    let (mut second, _, _) = make_session();

    assert!(first.character_try_claim_character_login_for_test(guid));
    assert!(first.character_try_claim_character_login_for_test(guid));
    assert!(!second.character_try_claim_character_login_for_test(guid));

    first.character_release_character_login_claim_for_test();
    assert!(second.character_try_claim_character_login_for_test(guid));
    second.character_release_character_login_claim_for_test();
}

#[test]
fn account_data_times_respect_global_and_character_masks_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    assert_eq!(
        GLOBAL_CACHE_MASK_LIKE_CPP | PER_CHARACTER_CACHE_MASK_LIKE_CPP,
        ALL_ACCOUNT_DATA_CACHE_MASK_LIKE_CPP
    );

    assert!(session.character_set_account_data_for_test(0, 10, "global-0".to_string()));
    assert!(session.character_set_account_data_for_test(1, 20, "character-1".to_string()));
    assert!(session.character_set_account_data_for_test(4, 40, "global-4".to_string()));
    assert!(session.character_set_account_data_for_test(14, 140, "character-14".to_string()));

    let global_times = session
        .character_account_data_times_for_test(ObjectGuid::EMPTY, GLOBAL_CACHE_MASK_LIKE_CPP);
    assert_eq!(global_times.player_guid, ObjectGuid::EMPTY);
    assert_eq!(global_times.account_times[0], 10);
    assert_eq!(global_times.account_times[1], 0);
    assert_eq!(global_times.account_times[4], 40);
    assert_eq!(global_times.account_times[14], 0);

    let player_times = session
        .character_account_data_times_for_test(player_guid, PER_CHARACTER_CACHE_MASK_LIKE_CPP);
    assert_eq!(player_times.player_guid, player_guid);
    assert_eq!(player_times.account_times[0], 0);
    assert_eq!(player_times.account_times[1], 20);
    assert_eq!(player_times.account_times[4], 0);
    assert_eq!(player_times.account_times[14], 140);
}

#[test]
fn ensure_login_player_controller_is_idempotent_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let guid = ObjectGuid::create_player(1, 43);
    let start = Position::new(1.0, 2.0, 3.0, 4.0);

    assert!(session.character_ensure_login_player_controller_for_test(
        guid,
        "LoginTester".to_string(),
        start,
        571,
        1,
        8,
        70,
        0,
    ));
    assert_eq!(session.player_guid(), Some(guid));
    assert_eq!(
        session.character_player_name_for_test(),
        Some("LoginTester".to_string())
    );
    assert_eq!(session.character_player_position_for_test(), Some(start));
    assert_eq!(session.character_player_map_id_for_test(), 571);
    assert_eq!(session.character_fall_information_for_test(), (0, start.z));

    session.character_set_player_gold_for_test(1234);
    session.character_set_player_xp_for_test(55);
    session.character_set_known_spells_for_test(vec![118, 133]);
    session.character_set_fall_information_for_test(1_200, 80.0);

    let moved = Position::new(5.0, 6.0, 7.0, 8.0);
    assert!(!session.character_ensure_login_player_controller_for_test(
        guid,
        "LoginTesterRenamed".to_string(),
        moved,
        1,
        2,
        3,
        71,
        1,
    ));

    assert_eq!(session.player_guid(), Some(guid));
    assert_eq!(
        session.character_player_name_for_test(),
        Some("LoginTesterRenamed".to_string())
    );
    assert_eq!(session.character_player_position_for_test(), Some(moved));
    assert_eq!(session.character_player_map_id_for_test(), 1);
    assert_eq!(session.character_fall_information_for_test(), (0, moved.z));
    assert_eq!(session.character_player_race_for_test(), 2);
    assert_eq!(session.character_player_class_for_test(), 3);
    assert_eq!(session.character_player_level_for_test(), 71);
    assert_eq!(session.character_player_gender_for_test(), 1);
    assert_eq!(session.character_player_gold_for_test(), 1234);
    assert_eq!(session.character_player_xp_for_test(), 55);
    assert_eq!(session.character_known_spells_for_test(), &[118, 133]);
}
