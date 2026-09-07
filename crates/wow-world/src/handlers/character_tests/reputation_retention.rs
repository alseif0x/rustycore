//! CharacterHandler.cpp:1070/1141/1176: LoadFromDB, initial packets, AddPlayerToMap
//! retain the same Player and loaded reputation; map admission is not another load.
use super::*;

#[test]
fn repeated_login_attachment_preserves_loaded_reputation_for_final_save() {
    let (mut session, _packets) = make_session_with_send_capacity(100);
    let guid = ObjectGuid::create_player(1, 585_0910);
    assert!(session.ensure_login_player_controller_like_cpp(
        guid,
        "ReputationRetention".into(),
        Position::ZERO,
        0,
        10,
        3,
        80,
        0,
    ));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);
    let mut faction = wow_data::progression_rewards::FactionEntry::for_test_like_cpp(910, 54);
    faction.reputation_race_mask[0] = 1791;
    faction.reputation_base[0] = -42000;
    session.set_faction_store(Arc::new(
        wow_data::progression_rewards::FactionStore::from_entries([faction]),
    ));
    assert!(session.load_character_reputation_rows_like_cpp([
        crate::reputation::mgr::CharacterReputationRowLikeCpp {
            faction_id: 910,
            standing: 17,
            flags: 2,
        },
    ]));
    let before = session
        .with_reputation_mgr_like_cpp(|mgr| mgr.get_state(54).cloned())
        .unwrap()
        .unwrap();
    assert_eq!(before.flags.bits(), 2);
    assert_eq!(before.standing, 17);
    assert!(!before.need_save);

    // This is the second ensure call in send_login_sequence, after hydration
    // and initial reputation publication. An unchanged identity must not reset it.
    assert!(!session.ensure_login_player_controller_like_cpp(
        guid,
        "ReputationRetention".into(),
        Position::ZERO,
        0,
        10,
        3,
        80,
        0,
    ));
    let after = session
        .with_reputation_mgr_like_cpp(|mgr| mgr.get_state(54).cloned())
        .unwrap()
        .unwrap();
    assert_eq!(after.flags, before.flags);
    assert_eq!(after.standing, before.standing);
    assert_eq!(after.need_save, before.need_save);
    assert_eq!(after.need_send, before.need_send);
    assert!(
        session
            .with_reputation_mgr_like_cpp(|mgr| mgr.pending_save_rows_like_cpp().is_empty())
            .unwrap()
    );

    // Updating only location/level/gender must not reinitialize loaded standing.
    session.set_loaded_player_identity_like_cpp(1, 10, 3, 81, 1);
    assert_eq!(
        session.with_reputation_mgr_like_cpp(|mgr| mgr.get_state(54).unwrap().standing),
        Some(17)
    );
    // A genuinely different race still selects its initial faction state.
    session.set_loaded_player_identity_like_cpp(1, 1, 3, 81, 1);
    assert_eq!(
        session.with_reputation_mgr_like_cpp(|mgr| mgr.get_state(54).unwrap().standing),
        Some(0)
    );
}
