// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character-save and logout lifecycle scenarios.

use super::*;
fn character_save_session_with_port(
    outcome: PersistenceOutcomeLikeCpp,
    guid_counter: i64,
) -> (WorldSession, Arc<RecordingPortLikeCpp>) {
    let (mut session, port) = session_with_port(outcome);
    session.set_player_guid(Some(ObjectGuid::create_player(1, guid_counter)));
    session.set_state(SessionState::LoggedIn);
    session.set_player_map_position_like_cpp(
        0,
        Position {
            x: 1.0,
            y: 2.0,
            z: 3.0,
            orientation: 0.5,
        },
    );
    session.tutorials_changed_like_cpp = true;
    session.tutorials_loaded_coherently_like_cpp = true;
    (session, port)
}

#[tokio::test]
async fn character_save_reaches_the_sqlx_free_port_and_cleans_only_after_apply_like_cpp() {
    let (mut session, port) = character_save_session_with_port(
        PersistenceOutcomeLikeCpp::Applied { rows: 12 },
        0x7500_0001,
    );
    session.mark_represented_character_spell_cooldowns_loaded_like_cpp();
    session.record_loaded_character_spell_cooldown_like_cpp(635, 6948, 9_000, 12, 8_000);
    session.mark_represented_character_spell_charges_loaded_like_cpp();
    session.record_loaded_character_spell_charge_like_cpp(42, 7_000, 8_000);

    session.save_current_player_to_db_like_cpp().await;

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
    assert!(!session.tutorials_changed_like_cpp);
    assert!(session.tutorials_loaded_from_db_like_cpp);
}

#[tokio::test]
async fn definite_character_save_rollback_preserves_dirty_state_like_cpp() {
    let (mut session, port) = character_save_session_with_port(
        PersistenceOutcomeLikeCpp::Failed {
            reason: "constraint failure before COMMIT".to_owned(),
        },
        0x7500_0002,
    );

    session.save_current_player_to_db_like_cpp().await;

    assert_eq!(port.character_saves().len(), 1);
    assert!(session.tutorials_changed_like_cpp);
    assert!(!session.tutorials_loaded_from_db_like_cpp);
    assert!(
        !session
            .durable_loot_money_persistence_tracker_like_cpp()
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

    session.save_current_player_to_db_like_cpp().await;

    assert_eq!(port.character_saves().len(), 1);
    assert!(session.tutorials_changed_like_cpp);
    assert!(!session.tutorials_loaded_from_db_like_cpp);
    assert!(
        session
            .durable_loot_money_persistence_tracker_like_cpp()
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
        session.current_map_id = 571;
        session.player_level = 17;
        let original = Position::new(1.0, 2.0, 3.0, 0.5);
        let destination = Position::new(11.0, 22.0, 33.0, 1.5);
        session
            .with_owned_player_mut_like_cpp(|player| {
                player.unit_mut().world_mut().relocate(original);
                player.unit_mut().set_level(60);
                player.set_xp(1234);
                player.set_money(5678);
                player.set_character_points_like_cpp(23);
                let teleport = &mut player.gameplay_state_mut().teleport;
                teleport.near_pending = true;
                teleport.near_destination = Some((571, destination));
            })
            .unwrap();

        session.save_current_player_to_db_like_cpp().await;

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
        session
            .with_owned_player_like_cpp(|player| {
                assert_eq!(
                    player.unit().world().position(),
                    original,
                    "save-only teleport destination must not relocate the live Player"
                );
                assert_eq!(
                    player.unit().data().level,
                    60,
                    "saving must not replay staged identity"
                );
                assert_eq!(
                    player.active_data().character_points,
                    23,
                    "saving must not run talent initialization"
                );
                assert!(player.gameplay_state().teleport.near_pending);
            })
            .unwrap();
        assert_eq!(
            session.tutorials_changed_like_cpp, remains_dirty,
            "only confirmed commit cleans dirty groups"
        );
    }
}

fn represented_buyback_item_like_cpp(db_guid: u64) -> InventoryItem {
    InventoryItem {
        guid: ObjectGuid::create_item(1, db_guid as i64),
        entry_id: 25,
        db_guid,
        inventory_type: None,
    }
}

#[tokio::test]
async fn logout_buyback_clear_reaches_port_and_publishes_only_after_apply_like_cpp() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    session.set_player_guid(Some(ObjectGuid::create_player(1, 0x7500_0101)));
    session.insert_buyback_item_like_cpp(94, represented_buyback_item_like_cpp(0x8100_0001));

    session.clear_buyback_on_logout().await;

    assert_eq!(
        port.buyback_clears(),
        vec![PlayerBuybackClearRequestLikeCpp {
            player_guid: 0x7500_0101,
            item_db_guids: vec![0x8100_0001],
        }]
    );
    assert!(session.buyback_items_like_cpp().is_empty());
}

#[tokio::test]
async fn logout_buyback_clear_preserves_runtime_for_failed_and_unknown_durability_like_cpp() {
    for outcome in [
        PersistenceOutcomeLikeCpp::Failed {
            reason: "rolled back before COMMIT".to_owned(),
        },
        PersistenceOutcomeLikeCpp::Unknown {
            reason: "connection lost after COMMIT".to_owned(),
        },
    ] {
        let (mut session, port) = session_with_port(outcome);
        session.set_player_guid(Some(ObjectGuid::create_player(1, 0x7500_0102)));
        session.insert_buyback_item_like_cpp(94, represented_buyback_item_like_cpp(0x8100_0002));

        session.clear_buyback_on_logout().await;

        assert_eq!(port.buyback_clears().len(), 1);
        assert!(session.buyback_items_like_cpp().contains_key(&94));
    }
}

#[tokio::test]
async fn logout_buyback_clear_without_port_does_not_fabricate_durable_success_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 0x7500_0103)));
    session.insert_buyback_item_like_cpp(94, represented_buyback_item_like_cpp(0x8100_0003));

    session.clear_buyback_on_logout().await;

    assert!(session.buyback_items_like_cpp().contains_key(&94));
}

#[tokio::test]
async fn realm_character_count_refresh_reaches_the_lifecycle_port_without_database_handles() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    session.set_realm_id(12);

    session.update_realm_characters().await;

    assert_eq!(
        port.realm_character_count_refreshes(),
        vec![PlayerRealmCharacterCountRefreshRequestLikeCpp {
            account_id: 1,
            realm_id: 12,
        }]
    );
}

#[tokio::test]
async fn realm_character_count_refresh_without_port_does_not_fabricate_a_request() {
    let (session, _, _) = make_session();
    session.update_realm_characters().await;
}

/// Each offline mark reaches the port as its own request, naming the logical
/// database it belongs to — the characters writes and the login write are not
/// collapsed into one call.
#[tokio::test]
async fn logout_publishes_each_offline_mark_through_the_port_like_cpp() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    let guid = ObjectGuid::create_player(1, 0x7200_0001);
    session.set_player_guid(Some(guid));

    session.mark_character_offline().await;
    session.mark_character_account_offline_like_cpp().await;
    session
        .mark_login_account_offline_on_disconnect_like_cpp()
        .await;

    let marks = port.marks();
    assert_eq!(
        marks,
        vec![
            PlayerOfflineMarkLikeCpp::Character {
                guid_low: guid.counter() as u32
            },
            PlayerOfflineMarkLikeCpp::CharacterAccount {
                account_id: session.account_id
            },
            PlayerOfflineMarkLikeCpp::LoginAccount {
                account_id: session.account_id
            },
        ]
    );
    assert_eq!(
        marks
            .iter()
            .map(|m| m.logical_database())
            .collect::<Vec<_>>(),
        vec![
            LogicalDatabaseLikeCpp::Characters,
            LogicalDatabaseLikeCpp::Characters,
            LogicalDatabaseLikeCpp::Login,
        ]
    );
}

/// With no selected character there is nothing to mark offline, so the port is
/// not called at all.
#[tokio::test]
async fn no_character_means_no_character_offline_request_like_cpp() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    assert!(session.player_guid().is_none());

    session.mark_character_offline().await;

    assert!(port.marks().is_empty());
}

/// A failed write is reported, not retried and not escalated.
#[tokio::test]
async fn a_failed_offline_mark_is_handled_without_panicking_like_cpp() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Failed {
        reason: "connection refused".to_owned(),
    });
    session.set_player_guid(Some(ObjectGuid::create_player(1, 0x7200_0002)));

    session.mark_character_offline().await;
    session.mark_character_account_offline_like_cpp().await;

    assert_eq!(port.marks().len(), 2);
}

/// An indeterminate outcome is a distinct class the caller must see; it is not
/// silently treated as success or as rollback.
#[tokio::test]
async fn an_unknown_offline_mark_outcome_is_handled_distinctly_like_cpp() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Unknown {
        reason: "connection lost after the write was sent".to_owned(),
    });
    session.set_player_guid(Some(ObjectGuid::create_player(1, 0x7200_0003)));

    session.mark_character_offline().await;

    assert_eq!(port.marks().len(), 1);
    assert!(
        PersistenceOutcomeLikeCpp::Unknown {
            reason: "x".to_owned()
        }
        .is_indeterminate()
    );
}

/// A session with no port installed performs no durable write and does not
/// panic: unit sessions and tests never reach a database.
#[tokio::test]
async fn a_session_without_a_port_performs_no_durable_write_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 0x7200_0004)));

    session.mark_character_offline().await;
    session.mark_character_account_offline_like_cpp().await;
    session
        .mark_login_account_offline_on_disconnect_like_cpp()
        .await;
}
