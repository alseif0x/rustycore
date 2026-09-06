//! C++ Player.cpp:1494-1503: resurrection, then delayed save, then ACK completion.
//! Controlled persistence exercises the registered thunk, not a real DB/client.
use super::*;
use crate::session::PlayerSaveOutcomeLikeCpp;

async fn exercise(outcome: PersistenceOutcomeLikeCpp, cancel: bool, periodic: bool) {
    let (mut session, _input, output) = make_session();
    let port = RecordingPortLikeCpp::new(outcome.clone());
    session.set_player_lifecycle_port_like_cpp(port.clone());
    let guid = ObjectGuid::create_player(1, 42);
    let position = Position::new(1.0, 2.0, 3.0, 0.5);
    session.set_player_guid(Some(guid));
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    session.set_player_position_like_cpp(position);
    session.set_spell_store(Arc::new(wow_data::SpellStore::new()));
    session.set_chr_classes_store(Arc::new(
        wow_data::character_progression::ChrClassesStore::from_entries([
            wow_data::character_progression::ChrClassesEntry {
                id: 5,
                starting_level: 1,
                ..Default::default()
            },
        ]),
    ));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
        (1, 5, 80),
        wow_data::PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect: 20,
            spirit: 30,
            base_mana: 1000,
        },
    )])));
    assert!(session.complete_represented_trait_config_authority_load_like_cpp([], true));
    assert!(
        session.schedule_represented_resurrection_after_teleport_like_cpp(
            wow_entities::PlayerResurrectionRequestLikeCpp {
                resurrecter: guid,
                map_id: 571,
                position,
                health: 100,
                mana: 50,
                aura: 0,
            }
        )
    );
    session
        .with_owned_player_mut_like_cpp(|p| {
            p.unit_mut().set_max_health(1000);
            p.unit_mut().set_health(500);
        })
        .unwrap();
    assert!(session.set_pending_teleport_like_cpp(Some((571, position))));
    assert!(session.set_represented_far_teleport_pending_like_cpp(true));
    session.set_state(SessionState::Transfer);
    if periodic {
        session.set_player_save_interval_ms_like_cpp(100);
        session.update_player_save_timer_like_cpp(100);
    } else {
        for _ in 0..2 {
            assert_eq!(
                session.save_current_player_to_db_like_cpp().await,
                PlayerSaveOutcomeLikeCpp::Deferred
            );
        }
    }
    assert!(port.character_saves().is_empty());
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        session.with_owned_player_like_cpp(|p| p.has_deferred_player_save_like_cpp()),
        Some(!periodic)
    );
    let handle = session.player_handle_like_cpp.unwrap();
    let manager = session.canonical_map_manager.as_ref().unwrap().clone();
    *port.during_save.lock().unwrap() = Some(Box::new(move || {
        manager
            .try_lock()
            .unwrap()
            .with_player_like_cpp(handle, |p| {
                assert!(p.has_deferred_player_save_like_cpp());
                assert!(p.teleport_state_like_cpp().post_add.is_none());
                assert!(!p.teleport_state_like_cpp().far_pending);
                assert_eq!(p.unit().data().health, 100);
            })
            .unwrap();
    }));
    port.save_pending
        .store(cancel, std::sync::atomic::Ordering::SeqCst);
    let catalogs = session.session_handler_catalogs_for_test_like_cpp();
    let registration =
        crate::session::registry::get_handler(wow_constants::ClientOpcodes::WorldPortResponse)
            .unwrap();
    assert_eq!(registration.status, wow_handler::SessionStatus::Transfer);
    assert_eq!(
        registration.processing,
        wow_handler::PacketProcessing::ThreadUnsafe
    );
    if cancel {
        let mut ack = Box::pin((registration.handler)(
            &mut session,
            &catalogs,
            WorldPacket::new_empty(),
        ));
        use std::future::Future;
        assert!(
            std::future::poll_fn(|cx| std::task::Poll::Ready(ack.as_mut().poll(cx).is_pending()))
                .await
        );
        assert_eq!(port.character_saves().len(), 1);
        drop(ack);
        session.kick("controlled cancellation after deferred save submission");
    } else {
        (registration.handler)(&mut session, &catalogs, WorldPacket::new_empty()).await;
    }
    let applied = !cancel && matches!(outcome, PersistenceOutcomeLikeCpp::Applied { .. });
    assert_eq!(
        session.state(),
        if !cancel && !matches!(outcome, PersistenceOutcomeLikeCpp::Unknown { .. }) {
            SessionState::LoggedIn
        } else {
            SessionState::Disconnecting
        }
    );
    assert_eq!(port.character_saves().len(), 1);
    assert_eq!(port.character_saves()[0].character.health, 100);
    assert_eq!(
        session.with_owned_player_like_cpp(|p| p.has_deferred_player_save_like_cpp()),
        Some(!applied)
    );
    if cancel || matches!(outcome, PersistenceOutcomeLikeCpp::Unknown { .. }) {
        assert_eq!(
            session.save_current_player_to_db_like_cpp().await,
            PlayerSaveOutcomeLikeCpp::Quarantined
        );
        assert_eq!(
            port.character_saves().len(),
            1,
            "unknown submission is never replayed"
        );
    }
    if matches!(outcome, PersistenceOutcomeLikeCpp::Failed { .. }) {
        session.set_player_save_interval_ms_like_cpp(100);
        session
            .process_pending_periodic_player_save_like_cpp()
            .await;
        assert_eq!(port.character_saves().len(), 1, "no every-tick retry");
        session.update_player_save_timer_like_cpp(100);
        session
            .process_pending_periodic_player_save_like_cpp()
            .await;
        assert_eq!(
            port.character_saves().len(),
            2,
            "retry at the next due autosave"
        );
        assert_eq!(session.state(), SessionState::LoggedIn);
        assert_eq!(
            session.with_owned_player_like_cpp(|p| p.has_deferred_player_save_like_cpp()),
            Some(true)
        );
    }
    assert!(!output.is_empty());
}

#[tokio::test]
async fn registered_worldport_coalesces_deferred_saves_after_resurrection() {
    exercise(PersistenceOutcomeLikeCpp::Applied { rows: 1 }, false, false).await;
    exercise(PersistenceOutcomeLikeCpp::Applied { rows: 1 }, false, true).await;
}

#[tokio::test]
async fn registered_worldport_retains_rollback_intent_and_quarantines_unknown_commit() {
    exercise(
        PersistenceOutcomeLikeCpp::Failed {
            reason: "rollback".into(),
        },
        false,
        false,
    )
    .await;
    exercise(
        PersistenceOutcomeLikeCpp::Unknown {
            reason: "lost commit reply".into(),
        },
        false,
        false,
    )
    .await;
}

#[tokio::test]
async fn cancelled_registered_worldport_save_retains_intent_without_replay() {
    exercise(PersistenceOutcomeLikeCpp::Applied { rows: 1 }, true, false).await;
}

#[tokio::test]
async fn deferred_save_newer_intent_survives_an_older_transaction_receipt() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    let handle = session.player_handle_like_cpp.unwrap();
    let manager = session.canonical_map_manager.as_ref().unwrap().clone();
    session.update_player_teleport_state_like_cpp(|s| s.far_pending = true);
    assert_eq!(
        session.save_current_player_to_db_like_cpp().await,
        PlayerSaveOutcomeLikeCpp::Deferred
    );
    session.update_player_teleport_state_like_cpp(|s| s.far_pending = false);
    *port.during_save.lock().unwrap() = Some(Box::new(move || {
        manager
            .try_lock()
            .unwrap()
            .with_player_mut_like_cpp(handle, |p| {
                p.teleport_state_mut_like_cpp().far_pending = true;
                assert_eq!(p.defer_save_if_transfer_pending_like_cpp(), Some(true));
                p.teleport_state_mut_like_cpp().far_pending = false;
            })
            .unwrap();
    }));
    assert_eq!(
        session.save_current_player_to_db_like_cpp().await,
        PlayerSaveOutcomeLikeCpp::Applied
    );
    assert_eq!(
        session.with_owned_player_like_cpp(|p| p.has_deferred_player_save_like_cpp()),
        Some(true)
    );
    assert_eq!(
        session.save_current_player_to_db_like_cpp().await,
        PlayerSaveOutcomeLikeCpp::Applied
    );
    assert_eq!(
        session.with_owned_player_like_cpp(|p| p.has_deferred_player_save_like_cpp()),
        Some(false)
    );
    assert_eq!(port.character_saves().len(), 2);
}

#[tokio::test]
async fn cancelled_before_deferred_save_submission_retains_intent_without_quarantine() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.update_player_teleport_state_like_cpp(|s| s.far_pending = true);
    assert_eq!(
        session.save_current_player_to_db_like_cpp().await,
        PlayerSaveOutcomeLikeCpp::Deferred
    );
    session.update_player_teleport_state_like_cpp(|s| s.far_pending = false);
    let tracker = session.durable_loot_money_persistence_like_cpp.clone();
    let lock = tracker.lock_money_mutation_like_cpp().await;
    let mut save = Box::pin(session.save_current_player_to_db_like_cpp());
    use std::future::Future;
    assert!(
        std::future::poll_fn(|cx| std::task::Poll::Ready(save.as_mut().poll(cx).is_pending()))
            .await
    );
    assert!(port.character_saves().is_empty());
    drop(save);
    drop(lock);
    assert!(!tracker.is_indeterminate_like_cpp());
    assert_eq!(
        session.with_owned_player_like_cpp(|p| p.has_deferred_player_save_like_cpp()),
        Some(true)
    );
    assert_eq!(
        session.save_current_player_to_db_like_cpp().await,
        PlayerSaveOutcomeLikeCpp::Applied
    );
    assert_eq!(port.character_saves().len(), 1);
    assert_eq!(
        session.with_owned_player_like_cpp(|p| p.has_deferred_player_save_like_cpp()),
        Some(false)
    );
}
