//! C++ Player::_SaveSpells consumes only rows visited while preparing the save
//! (Player.cpp:20399-20451). Rust's confirmed-COMMIT gate must not consume a later row.
use super::*;

fn new_spell(id: i32) -> wow_entities::PlayerKnownSpellRecord {
    wow_entities::PlayerKnownSpellRecord {
        spell_id: id,
        state: wow_entities::PlayerSpellLoadState::New,
        active: true,
        disabled: false,
        favorite: false,
        dependent: false,
    }
}

fn canonical_session(
    outcome: PersistenceOutcomeLikeCpp,
) -> (WorldSession, Arc<RecordingPortLikeCpp>) {
    let (mut session, port) = session_with_port(outcome);
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    session
        .with_owned_player_mut_like_cpp(|player| {
            let spells = &mut player.gameplay_state_mut().spells;
            spells.rows_loaded = true;
            spells.rows_complete = true;
            spells.rows.insert(10, new_spell(10));
        })
        .unwrap();
    (session, port)
}

#[tokio::test]
async fn full_save_ack_does_not_clean_a_reputation_row_shadowed_by_the_projection() {
    let (mut session, port) = canonical_session(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    session
        .with_owned_player_mut_like_cpp(|p| {
            p.gameplay_state_mut().reputations = vec![
                wow_entities::PlayerReputationRecord {
                    faction_id: 72,
                    reputation_list_id: 1,
                    standing: 10,
                    need_save: true,
                    ..Default::default()
                },
                wow_entities::PlayerReputationRecord {
                    faction_id: 76,
                    reputation_list_id: 1,
                    standing: 20,
                    need_save: true,
                    ..Default::default()
                },
            ];
        })
        .unwrap();
    session.save_current_player_to_db_like_cpp().await;
    // The current adapter constructs C++ FactionStateList keyed by ReputationListID.
    // This intentionally malformed native vector projects only its last key owner.
    let requests = port.character_saves();
    assert_eq!(requests[0].reputations.len(), 1);
    assert_eq!(requests[0].reputations[0].faction_id, 76);
    assert_eq!(
        session.with_owned_player_like_cpp(|p| {
            p.gameplay_state()
                .reputations
                .iter()
                .map(|row| row.need_save)
                .collect::<Vec<_>>()
        }),
        Some(vec![true, false])
    );
}

#[tokio::test]
async fn full_save_ack_rebases_changed_new_spell_for_the_next_transaction() {
    let (mut session, port) = canonical_session(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    let handle = session.player_handle_like_cpp.unwrap();
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    *port.during_save.lock().unwrap() = Some(Box::new(move || {
        manager
            .try_lock()
            .unwrap()
            .with_player_mut_like_cpp(handle, |player| {
                player
                    .gameplay_state_mut()
                    .spells
                    .rows
                    .get_mut(&10)
                    .unwrap()
                    .favorite = true;
            })
            .unwrap();
    }));
    session.save_current_player_to_db_like_cpp().await;
    assert_eq!(
        session.with_owned_player_like_cpp(|p| p.gameplay_state().spells.rows[&10].state),
        Some(wow_entities::PlayerSpellLoadState::Changed)
    );
    session.save_current_player_to_db_like_cpp().await;
    let requests = port.character_saves();
    let Some(wow_persistence::PlayerSpellSaveGroupLikeCpp::Complete { rows, .. }) =
        &requests[1].spells
    else {
        panic!("complete spell authority")
    };
    assert_eq!(
        rows[0].state,
        wow_persistence::PlayerSpellStateLikeCpp::Changed
    );
    assert!(rows[0].favorite);
    assert_eq!(
        session.with_owned_player_like_cpp(|p| p.gameplay_state().spells.rows[&10].state),
        Some(wow_entities::PlayerSpellLoadState::Unchanged)
    );
}

#[tokio::test]
async fn full_save_rollback_and_unknown_leave_native_dirty_state_and_distinct_fences() {
    for unknown in [false, true] {
        let outcome = if unknown {
            PersistenceOutcomeLikeCpp::Unknown {
                reason: "lost reply".into(),
            }
        } else {
            PersistenceOutcomeLikeCpp::Failed {
                reason: "definite rollback".into(),
            }
        };
        let (mut session, port) = canonical_session(outcome);
        session.save_current_player_to_db_like_cpp().await;
        assert_eq!(port.character_saves().len(), 1);
        assert_eq!(
            session.with_owned_player_like_cpp(|p| p.gameplay_state().spells.rows[&10].state),
            Some(wow_entities::PlayerSpellLoadState::New)
        );
        assert_eq!(
            session
                .durable_loot_money_persistence_tracker_like_cpp()
                .is_indeterminate_like_cpp(),
            unknown
        );
    }
}

#[tokio::test]
async fn full_save_cancellation_keeps_receipt_unapplied_and_quarantines_unknown_commit() {
    let (mut session, port) = canonical_session(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    port.save_pending
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    let mut future = Box::pin(session.save_current_player_to_db_like_cpp());
    assert!(
        std::future::poll_fn(|cx| std::task::Poll::Ready(future.as_mut().poll(cx).is_pending()))
            .await
    );
    assert_eq!(port.character_saves().len(), 1);
    manager.try_lock().unwrap().update(10);
    drop(future);
    assert!(
        session
            .durable_loot_money_persistence_tracker_like_cpp()
            .is_indeterminate_like_cpp()
    );
    assert_eq!(
        session.with_owned_player_like_cpp(|p| p.gameplay_state().spells.rows[&10].state),
        Some(wow_entities::PlayerSpellLoadState::New)
    );
}

#[test]
fn full_save_preparation_is_owned_and_matches_previous_projection_for_loaded_groups() {
    let (mut session, _) = canonical_session(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    session.mark_represented_glyphs_loaded_like_cpp();
    session.mark_represented_character_spell_cooldowns_loaded_like_cpp();
    session.record_loaded_character_spell_cooldown_like_cpp(635, 6948, 9_000, 12, 8_000);
    session.mark_represented_character_spell_charges_loaded_like_cpp();
    session.record_loaded_character_spell_charge_like_cpp(42, 7_000, 8_000);
    session.tutorials_changed_like_cpp = true;
    session.tutorials_loaded_coherently_like_cpp = true;
    session
        .with_owned_player_mut_like_cpp(|p| {
            let game = p.gameplay_state_mut();
            game.equipment_sets_loaded = true;
            game.equipment_sets.insert(
                1,
                wow_entities::PlayerEquipmentSetLikeCpp::equipment(
                    1,
                    0,
                    wow_entities::PlayerEquipmentSetUpdateStateLikeCpp::New,
                ),
            );
            game.cuf_profiles_loaded = true;
            game.action_buttons_loaded = true;
            game.talents.talents_loaded = true;
        })
        .unwrap();
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        let header = session
            .current_player_save_to_db_snapshot_like_cpp()
            .unwrap();
        let old = session
            .current_player_character_save_request_like_cpp(&header, 123)
            .unwrap();
        let prepared = session.prepare_player_save_like_cpp(123).unwrap();
        assert_eq!(prepared.request, old);
        session
            .with_owned_player_mut_like_cpp(|p| {
                p.set_money(p.money() + 1);
            })
            .unwrap();
        assert_eq!(prepared.request.character.money, old.character.money);
        assert!(
            session
                .canonical_map_manager
                .as_ref()
                .unwrap()
                .try_lock()
                .is_ok()
        );
    }
}

#[tokio::test]
async fn full_save_ack_does_not_clean_a_spell_added_after_capture() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    let handle = session.player_handle_like_cpp.unwrap();
    session
        .with_owned_player_mut_like_cpp(|player| {
            let spells = &mut player.gameplay_state_mut().spells;
            spells.rows_loaded = true;
            spells.rows_complete = true;
            spells.rows.insert(10, new_spell(10));
        })
        .unwrap();
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    *port.during_save.lock().unwrap() = Some(Box::new(move || {
        // Real shared owner remains usable while persistence is pending.
        manager
            .try_lock()
            .expect("save must release its owner guard before I/O")
            .with_player_mut_like_cpp(handle, |player| {
                player
                    .gameplay_state_mut()
                    .spells
                    .rows
                    .insert(20, new_spell(20));
            })
            .unwrap();
    }));
    session.save_current_player_to_db_like_cpp().await;
    assert_eq!(port.character_saves().len(), 1);
    session
        .with_owned_player_like_cpp(|player| {
            assert_eq!(
                player.gameplay_state().spells.rows[&10].state,
                wow_entities::PlayerSpellLoadState::Unchanged
            );
            assert_eq!(
                player.gameplay_state().spells.rows[&20].state,
                wow_entities::PlayerSpellLoadState::New,
                "the confirmed transaction never contained the later spell"
            );
        })
        .unwrap();
}
