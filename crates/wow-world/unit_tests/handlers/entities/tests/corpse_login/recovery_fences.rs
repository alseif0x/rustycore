use super::*;

fn phase(session: &crate::session::WorldSession) -> Option<wow_entities::PlayerResurrectionPhase> {
    session
        .core
        .with_owned_player_like_cpp(|player| {
            player
                .resurrection_state_like_cpp()
                .delayed_after_teleport
                .map(|continuation| continuation.phase)
        })
        .flatten()
}

#[tokio::test]
async fn recovery_fences_login_no_corpse_clears_stale_flag_without_save_or_delete() {
    let (mut session, guid, _send_rx) =
        corpse_login_session(571, maps_with_world_and_instance(), false);
    session.core.with_owned_player_mut_like_cpp(|player| {
        player.set_at_login_flags_like_cpp(AT_LOGIN_RESURRECT)
    });
    let corpse_port = Arc::new(super::super::corpse::RecordingCorpsePortLikeCpp::default());
    session.set_map_corpse_persistence_port_like_cpp(corpse_port.clone());
    let save_port = crate::session::RecordingPlayerLifecyclePort::new(
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.set_player_lifecycle_port_like_cpp(save_port.clone());

    restore_corpse(&mut session, Some(corpse_location_row(33)))
        .await
        .expect("alive login follows the source's no-corpse path");

    assert_eq!(session.player_guid(), Some(guid));
    assert_eq!(corpse_port.delete_requests(), Vec::<u64>::new());
    assert_eq!(save_port.character_save_count(), 0);
    assert_eq!(phase(&session), None);
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| player
            .persistent_capability_state_like_cpp()
            .at_login_flags),
        Some(0)
    );
}

#[tokio::test]
async fn recovery_fences_login_confirmed_bones_resurrect_without_sickness_or_full_save() {
    let (mut session, send_rx, _realm_rx, player_guid, corpse_port) =
        super::super::corpse::dead_player_session_with_graveyard_like_cpp(43_240);
    super::super::corpse::handle_repop_request(&mut session, repop_request_packet(true)).await;
    assert_eq!(
        wow_world_entities::handle_move_teleport_ack_like_cpp(&mut session, player_guid, 0, 0),
        wow_world_core::session::movement_protocol::MoveTeleportAckActionLikeCpp::Accepted
    );
    let _release_and_ack_packets = send_rx.try_iter().collect::<Vec<_>>();
    session.core.with_owned_player_mut_like_cpp(|player| {
        player.set_at_login_flags_like_cpp(AT_LOGIN_RESURRECT)
    });
    let _corpse_guid = super::super::corpse::current_corpse_guid_like_cpp(&session, player_guid)
        .expect("the fixture has a real persisted corpse before login restoration");
    let save_port = crate::session::RecordingPlayerLifecyclePort::new(
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.set_player_lifecycle_port_like_cpp(save_port.clone());
    super::super::spirit_healer::install_recovery_sickness_fixture(&mut session);
    session.set_death_policy_like_cpp(wow_world_core::session::DeathPolicyLikeCpp {
        sickness_level: 80,
        ..Default::default()
    });

    restore_corpse(&mut session, Some(corpse_location_row(571)))
        .await
        .expect("the confirmed corpse delete allows the pinned LoadCorpse resurrection");

    assert_eq!(corpse_port.delete_requests(), vec![43_240]);
    assert!(super::super::corpse::current_corpse_guid_like_cpp(&session, player_guid).is_none());
    assert!(session.player_is_alive_like_cpp());
    assert_eq!(
        save_port.character_save_count(),
        0,
        "login keeps triggerSave=false"
    );
    assert_eq!(phase(&session), None);
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| player
            .persistent_capability_state_like_cpp()
            .at_login_flags),
        Some(0)
    );
    assert!(
        !super::super::spirit_healer::recovery_sickness_is_present(&session),
        "the valid 15007 fixture remains unapplied when LoadCorpse uses applySickness=false"
    );
    assert!(
        std::iter::from_fn(|| send_rx.try_recv().ok()).all(|packet| {
            u16::from_le_bytes([packet[0], packet[1]]) != ServerOpcodes::SpellGo as u16
        }),
        "LoadCorpse uses ResurrectPlayer's default applySickness=false"
    );
}

#[tokio::test]
async fn recovery_fences_login_cancelled_delete_keeps_ghost_flag_and_canonical_owner() {
    let (mut session, _send_rx, _realm_rx, player_guid, corpse_port) =
        super::super::corpse::dead_player_session_with_graveyard_like_cpp(43_254);
    super::super::corpse::handle_repop_request(&mut session, repop_request_packet(true)).await;
    assert_eq!(
        wow_world_entities::handle_move_teleport_ack_like_cpp(&mut session, player_guid, 0, 0),
        wow_world_core::session::movement_protocol::MoveTeleportAckActionLikeCpp::Accepted
    );
    session.core.with_owned_player_mut_like_cpp(|player| {
        player.set_at_login_flags_like_cpp(AT_LOGIN_RESURRECT)
    });
    let _corpse_guid = super::super::corpse::current_corpse_guid_like_cpp(&session, player_guid)
        .expect("the login fixture owns a live corpse before conversion");
    let save_port = crate::session::RecordingPlayerLifecyclePort::new(
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.set_player_lifecycle_port_like_cpp(save_port.clone());
    corpse_port.hold_delete();

    let mut restoration = Box::pin(restore_corpse(&mut session, Some(corpse_location_row(571))));
    tokio::select! {
        result = tokio::time::timeout(std::time::Duration::from_secs(5), corpse_port.wait_for_delete()) => {
            assert!(result.is_ok(), "login did not reach its controlled corpse delete");
        }
        outcome = &mut restoration => panic!("login returned before its held corpse delete resolved: {outcome:?}"),
    }
    drop(restoration);

    assert_eq!(corpse_port.delete_requests(), vec![43_254]);
    assert_eq!(
        super::super::corpse::current_corpse_guid_like_cpp(&session, player_guid),
        None
    );
    assert!(!session.player_is_alive_like_cpp());
    assert_eq!(save_port.character_save_count(), 0);
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| player
            .persistent_capability_state_like_cpp()
            .at_login_flags
            & AT_LOGIN_RESURRECT),
        Some(AT_LOGIN_RESURRECT)
    );
    assert_eq!(
        phase(&session),
        Some(wow_entities::PlayerResurrectionPhase::RecoveryCorpseBonesInFlight)
    );
    assert!(session.core.player_handle_like_cpp.is_some());

    assert!(
        restore_corpse(&mut session, Some(corpse_location_row(571)))
            .await
            .is_err()
    );
    assert_eq!(corpse_port.delete_requests(), vec![43_254]);
    assert_eq!(save_port.character_save_count(), 0);
    assert_eq!(
        phase(&session),
        Some(wow_entities::PlayerResurrectionPhase::RecoveryCorpseBonesInFlight)
    );
}

#[tokio::test]
async fn recovery_fences_ordinary_dead_login_restores_ghost_auras_and_clears_owner() {
    let (mut session, guid, send_rx) =
        corpse_login_session(571, maps_with_world_and_instance(), true);
    super::super::corpse::install_release_cast_spell_fixture(&mut session);
    restore_corpse(&mut session, None)
        .await
        .expect("ordinary dead login restores ghost auras and completes its owner");

    assert_eq!(session.player_guid(), Some(guid));
    assert!(!session.player_is_alive_like_cpp());
    assert_eq!(
        phase(&session),
        None,
        "ordinary dead login clears the same owner after ghost restoration returns"
    );
    assert!(
        std::iter::from_fn(|| send_rx.try_recv().ok()).any(|packet| {
            u16::from_le_bytes([packet[0], packet[1]]) == ServerOpcodes::SpellGo as u16
        }),
        "the real ghost aura attempt used the ordinary cast path"
    );
}

#[tokio::test]
async fn recovery_fences_login_delete_failure_precedes_native_resurrection_and_flag_clear() {
    let (mut session, _send_rx, _realm_rx, player_guid, corpse_port) =
        super::super::corpse::dead_player_session_with_graveyard_like_cpp(43_241);
    super::super::corpse::handle_repop_request(&mut session, repop_request_packet(true)).await;
    let _ = wow_world_entities::handle_move_teleport_ack_like_cpp(&mut session, player_guid, 0, 0);
    session.core.with_owned_player_mut_like_cpp(|player| {
        player.set_at_login_flags_like_cpp(AT_LOGIN_RESURRECT)
    });
    corpse_port.fail_next_delete();
    let save_port = crate::session::RecordingPlayerLifecyclePort::new(
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.set_player_lifecycle_port_like_cpp(save_port.clone());

    let result = restore_corpse(&mut session, Some(corpse_location_row(571))).await;

    assert!(result.is_err());
    assert!(!session.player_is_alive_like_cpp());
    assert_eq!(corpse_port.delete_requests(), vec![43_241]);
    assert_eq!(
        save_port.character_save_count(),
        0,
        "login exempts bones from a full save"
    );
    assert_eq!(
        phase(&session),
        Some(wow_entities::PlayerResurrectionPhase::RecoveryCorpseBonesInFlight)
    );
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| player
            .persistent_capability_state_like_cpp()
            .at_login_flags
            & AT_LOGIN_RESURRECT),
        Some(AT_LOGIN_RESURRECT),
        "an unconfirmed delete cannot clear AT_LOGIN_RESURRECT"
    );
    assert!(session.core.player_handle_like_cpp.is_some());
}

#[tokio::test]
async fn recovery_fences_login_native_error_after_confirmed_delete_keeps_at_login_flag() {
    let (mut session, _send_rx, _realm_rx, corpse_port) =
        super::super::spirit_healer::logged_in_released_ghost(43_242).await;
    session.core.with_owned_player_mut_like_cpp(|player| {
        player.set_at_login_flags_like_cpp(AT_LOGIN_RESURRECT)
    });
    let has_ghost = session
        .core
        .with_owned_player_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .runtime_applications_like_cpp()
                .values()
                .any(|application| {
                    application.spell_id == wow_world_entities::SPELL_GHOST_LIKE_CPP
                })
        })
        .expect("canonical released-ghost Player");
    assert!(has_ghost, "the aura remains present while its removal metadata is missing");
    super::super::spirit_healer::remove_ghost_spell_metadata(&mut session);
    let save_port = crate::session::RecordingPlayerLifecyclePort::new(
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.set_player_lifecycle_port_like_cpp(save_port.clone());

    let result = restore_corpse(&mut session, Some(corpse_location_row(571))).await;

    assert!(result.is_err());
    assert_eq!(corpse_port.delete_requests(), vec![43_242]);
    assert_eq!(save_port.character_save_count(), 0);
    assert_eq!(
        phase(&session),
        Some(wow_entities::PlayerResurrectionPhase::ResurrectPlayerInFlight)
    );
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| player
            .persistent_capability_state_like_cpp()
            .at_login_flags
            & AT_LOGIN_RESURRECT),
        Some(AT_LOGIN_RESURRECT)
    );
}
