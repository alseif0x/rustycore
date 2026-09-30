// Existing Character application scenarios, moved with original assertion operands.

use super::fixtures::session::make_session;
use super::fixtures::*;

fn character_save_session_with_port(
    outcome: PersistenceOutcomeLikeCpp,
    guid_counter: i64,
) -> (WorldSession, Arc<RecordingPortLikeCpp>) {
    let (mut session, port) = session_with_port(outcome);
    session.set_player_guid(Some(ObjectGuid::create_player(1, guid_counter)));
    session.set_state(SessionState::LoggedIn);
    session.character_set_player_map_position_for_test(
        0,
        Position {
            x: 1.0,
            y: 2.0,
            z: 3.0,
            orientation: 0.5,
        },
    );
    session.character_set_tutorials_dirty_for_test(true, true);
    (session, port)
}

#[tokio::test]
async fn character_save_reaches_the_sqlx_free_port_and_cleans_only_after_apply_like_cpp() {
    let (mut session, port) = character_save_session_with_port(
        PersistenceOutcomeLikeCpp::Applied { rows: 12 },
        0x7500_0001,
    );
    session.character_mark_represented_character_spell_cooldowns_loaded_for_test();
    session.character_record_loaded_character_spell_cooldown_for_test(635, 6948, 9_000, 12, 8_000);
    session.character_mark_represented_character_spell_charges_loaded_for_test();
    session.character_record_loaded_character_spell_charge_for_test(42, 7_000, 8_000);

    session.character_save_current_player_to_db_for_test().await;

    let saves = port.character_saves();
    assert_eq!(saves.len(), 1);
    assert!(saves[0].tutorials.is_some());
    assert_eq!(saves[0].player_guid, 0x7500_0001);
    assert_eq!(
        saves[0].spell_cooldowns,
        Some(vec![PlayerSpellCooldownSaveLikeCpp {
            spell_id: 635,
            item_id: 6948,
            cooldown_end_unix_secs: 9_000,
            category_id: 12,
            category_end_unix_secs: 8_000,
        }])
    );
    assert_eq!(
        saves[0].spell_charges,
        Some(vec![PlayerSpellChargeSaveLikeCpp {
            category_id: 42,
            recharge_start_unix_secs: 7_000,
            recharge_end_unix_secs: 8_000,
        }])
    );
    assert!(!session.character_tutorials_changed_for_test());
    assert!(session.character_tutorials_loaded_for_test());
}

#[tokio::test]
async fn definite_character_save_rollback_preserves_dirty_state_like_cpp() {
    let (mut session, port) = character_save_session_with_port(
        PersistenceOutcomeLikeCpp::Failed {
            reason: "constraint failure before COMMIT".to_owned(),
        },
        0x7500_0002,
    );

    session.character_save_current_player_to_db_for_test().await;

    assert_eq!(port.character_saves().len(), 1);
    assert!(session.character_tutorials_changed_for_test());
    assert!(!session.character_tutorials_loaded_for_test());
    assert!(
        !session
            .character_durable_loot_money_persistence_tracker_for_test()
            .is_indeterminate_like_cpp()
    );
}

#[tokio::test]
async fn unknown_character_save_commit_fences_and_preserves_dirty_state_like_cpp() {
    let (mut session, port) = character_save_session_with_port(
        PersistenceOutcomeLikeCpp::Unknown {
            reason: "connection lost after COMMIT".to_owned(),
        },
        0x7500_0003,
    );

    session.character_save_current_player_to_db_for_test().await;

    assert_eq!(port.character_saves().len(), 1);
    assert!(session.character_tutorials_changed_for_test());
    assert!(!session.character_tutorials_loaded_for_test());
    assert!(
        session
            .character_durable_loot_money_persistence_tracker_for_test()
            .is_indeterminate_like_cpp()
    );
}

#[tokio::test]
async fn character_save_does_not_reapply_save_destination_or_progression_to_runtime() {
    for (outcome, remains_dirty) in [
        (PersistenceOutcomeLikeCpp::Applied { rows: 12 }, false),
        (
            PersistenceOutcomeLikeCpp::Failed {
                reason: "definite rollback".to_owned(),
            },
            true,
        ),
        (
            PersistenceOutcomeLikeCpp::Unknown {
                reason: "lost COMMIT reply".to_owned(),
            },
            true,
        ),
    ] {
        let (mut session, port) = character_save_session_with_port(outcome, 0x7500_0004);
        install_canonical_player_owner_for_test(&mut session, 571, 0);
        session.character_set_saved_identity_projection_for_test(571, 17);
        let original = Position::new(1.0, 2.0, 3.0, 0.5);
        let destination = Position::new(11.0, 22.0, 33.0, 1.5);
        session
            .character_prepare_save_runtime_for_test(original, destination)
            .unwrap();

        session.character_save_current_player_to_db_for_test().await;

        let saves = port.character_saves();
        assert_eq!(
            saves.len(),
            1,
            "regression must exercise the actual save port"
        );
        assert_eq!(saves[0].character.position.x, destination.x);
        assert_eq!(saves[0].character.position.y, destination.y);
        assert_eq!(saves[0].character.position.z, destination.z);
        assert_eq!(saves[0].character.level, 60);
        assert_eq!(saves[0].character.xp, 1234);
        assert_eq!(saves[0].character.money, 5678);
        {
            let player = session.character_save_runtime_for_test().unwrap();
            assert_eq!(
                player.position, original,
                "save-only teleport destination must not relocate the live Player"
            );
            assert_eq!(player.level, 60, "saving must not replay staged identity");
            assert_eq!(
                player.character_points, 23,
                "saving must not run talent initialization"
            );
            assert!(player.near_pending);
        }
        assert_eq!(
            session.character_tutorials_changed_for_test(),
            remains_dirty,
            "only confirmed commit cleans dirty groups"
        );
    }
}
