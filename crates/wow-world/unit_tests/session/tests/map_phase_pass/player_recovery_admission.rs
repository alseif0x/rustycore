use super::*;
use crate::session::mailbox::{PlayerRecoveryAdmission, SessionPassKind};
use wow_map::{MapKey, MapTickCoordinationStateLikeCpp};

fn logged_in_session() -> (WorldSession, wow_core::ObjectGuid) {
    let (mut session, _packet_tx, _send_rx) = make_session();
    let guid = crate::canonical_player_access::install_canonical_player_owner_for_test(
        &mut session,
        571,
        0,
    );
    register_session_for_phases(&mut session);
    session.core.state = SessionState::LoggedIn;
    (session, guid)
}

fn start_map_tick(session: &WorldSession) -> u64 {
    let manager = session
        .core
        .canonical_map_manager
        .as_ref()
        .expect("canonical map manager is installed");
    manager
        .lock()
        .expect("map manager lock")
        .begin_tick_like_cpp(1_000)
        .into_started()
        .expect("map tick starts")
        .epoch_like_cpp()
}

fn recovery_admission(session: &WorldSession, map_tick_epoch: u64) -> PlayerRecoveryAdmission {
    let map_admission = session
        .current_map_phase_admission_for_test_like_cpp(77, 9, 33)
        .expect("current canonical player facts are available");
    PlayerRecoveryAdmission {
        coordinator_id: 77,
        tick_epoch: 9,
        map_tick_epoch,
        registration: map_admission.registration,
        handle: map_admission.handle,
        map_key: map_admission.map_key,
        map_incarnation: map_admission.map_incarnation,
        residence_revision: map_admission.residence_revision,
        diff_ms: map_admission.diff_ms,
        unix_time_ns: 50_000_000_000,
    }
}

#[test]
fn player_recovery_admission_uses_the_map_plan_epoch_and_requires_an_active_logged_in_player() {
    let (mut session, _) = logged_in_session();
    let map_tick_epoch = start_map_tick(&session);
    let admission = recovery_admission(&session, map_tick_epoch);

    assert_eq!(admission.coordinator_id, 77);
    assert_eq!(admission.tick_epoch, 9);
    assert_eq!(map_tick_epoch, 1);
    assert_eq!(admission.map_tick_epoch, 1);
    assert!(
        session
            .core
            .is_player_recovery_admission_current(&admission)
    );

    let wrong_map_epoch = PlayerRecoveryAdmission {
        map_tick_epoch: map_tick_epoch + 1,
        ..admission
    };
    assert!(
        !session
            .core
            .is_player_recovery_admission_current(&wrong_map_epoch)
    );
    assert_eq!(
        session
            .core
            .canonical_map_manager
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::AwaitingSessions(map_tick_epoch)
    );

    session.core.state = SessionState::Authed;
    assert!(
        !session
            .core
            .is_player_recovery_admission_current(&admission)
    );
}

#[test]
fn player_recovery_admission_rejects_idle_map_and_missing_player_binding() {
    let (idle_session, _) = logged_in_session();
    let idle_admission = recovery_admission(&idle_session, 1);
    assert!(
        !idle_session
            .core
            .is_player_recovery_admission_current(&idle_admission)
    );

    let (mut session, guid) = logged_in_session();
    let map_tick_epoch = start_map_tick(&session);
    let admission = recovery_admission(&session, map_tick_epoch);
    session.core.player_guid = Some(wow_core::ObjectGuid::create_player(1, 84_200_099));
    assert!(
        !session
            .core
            .is_player_recovery_admission_current(&admission)
    );
    session.core.player_guid = Some(guid);

    session
        .core
        .canonical_map_manager
        .as_ref()
        .unwrap()
        .lock()
        .unwrap()
        .detach_player_like_cpp(admission.handle)
        .expect("canonical Player detaches");
    assert!(
        !session
            .core
            .is_player_recovery_admission_current(&admission)
    );
}

#[test]
fn player_recovery_admission_rejects_a_replaced_registry_registration() {
    let (mut session, _) = logged_in_session();
    let map_tick_epoch = start_map_tick(&session);
    let admission = recovery_admission(&session, map_tick_epoch);
    let registry = std::sync::Arc::clone(session.core.player_registry.as_ref().unwrap());
    let (guid, replacement) = phase_registration_for(&session);
    let replacement_registration =
        registry.register_or_replace(guid, replacement, Default::default());

    assert_eq!(
        replacement_registration.guid(),
        admission.registration.guid()
    );
    assert_ne!(
        replacement_registration.generation(),
        admission.registration.generation()
    );
    assert!(
        !session
            .core
            .is_player_recovery_admission_current(&admission)
    );
}

#[test]
fn player_recovery_admission_rejects_a_replaced_canonical_player_handle() {
    let (session, _) = logged_in_session();
    let map_tick_epoch = start_map_tick(&session);
    let admission = recovery_admission(&session, map_tick_epoch);
    let manager = std::sync::Arc::clone(
        session
            .core
            .canonical_map_manager
            .as_ref()
            .expect("canonical map manager"),
    );
    {
        let mut manager = manager.lock().unwrap();
        let player = manager
            .retire_player_like_cpp(admission.handle)
            .expect("old Player owner retires");
        let replacement = manager
            .install_detached_player_like_cpp(player)
            .expect("replacement Player owner installs");
        assert_ne!(replacement.generation(), admission.handle.generation());
        manager
            .attach_player_like_cpp(replacement, admission.map_key, wow_core::Position::ZERO)
            .expect("replacement Player becomes active");
    }

    assert!(
        !session
            .core
            .is_player_recovery_admission_current(&admission)
    );
}

#[test]
fn player_recovery_admission_rejects_map_replacement_and_a_to_b_to_a_residence() {
    let (session, _) = logged_in_session();
    let map_tick_epoch = start_map_tick(&session);
    let admission = recovery_admission(&session, map_tick_epoch);
    let manager = std::sync::Arc::clone(session.core.canonical_map_manager.as_ref().unwrap());
    {
        let mut manager = manager.lock().unwrap();
        let player = manager
            .retire_player_like_cpp(admission.handle)
            .expect("player retires before replacing its map");
        assert!(manager.destroy_map(admission.map_key.map_id, admission.map_key.instance_id));
        manager.create_world_map(admission.map_key.map_id, admission.map_key.instance_id);
        let replacement = manager
            .install_detached_player_like_cpp(player)
            .expect("player owner reinstalls");
        manager
            .attach_player_like_cpp(replacement, admission.map_key, wow_core::Position::ZERO)
            .expect("player reattaches to the replacement map");
        assert_ne!(
            manager.map_incarnation_like_cpp(admission.map_key),
            Some(admission.map_incarnation)
        );
    }
    assert!(
        !session
            .core
            .is_player_recovery_admission_current(&admission)
    );

    let (session, _) = logged_in_session();
    let map_tick_epoch = start_map_tick(&session);
    let admission = recovery_admission(&session, map_tick_epoch);
    let manager = std::sync::Arc::clone(session.core.canonical_map_manager.as_ref().unwrap());
    {
        let mut manager = manager.lock().unwrap();
        let other_key = MapKey::new(admission.map_key.map_id, admission.map_key.instance_id + 1);
        manager.create_world_map(other_key.map_id, other_key.instance_id);
        manager
            .detach_player_like_cpp(admission.handle)
            .expect("player leaves the original map");
        manager
            .attach_player_like_cpp(admission.handle, other_key, wow_core::Position::ZERO)
            .expect("player enters the other map");
        manager
            .detach_player_like_cpp(admission.handle)
            .expect("player leaves the other map");
        manager
            .attach_player_like_cpp(
                admission.handle,
                admission.map_key,
                wow_core::Position::ZERO,
            )
            .expect("player returns to the original map");
        assert_eq!(
            manager.map_incarnation_like_cpp(admission.map_key),
            Some(admission.map_incarnation)
        );
        assert_ne!(
            manager.player_active_residence_revision_like_cpp(admission.handle),
            Some((admission.map_key, admission.residence_revision))
        );
    }
    assert!(
        !session
            .core
            .is_player_recovery_admission_current(&admission)
    );
}

#[test]
fn phase_authority_rejects_retired_cross_phase_steps_and_accepts_equal_epochs_once() {
    let (mut session, _, _) = make_session();
    let authority = &mut session.core.admission;

    assert!(authority.accept_phase_authority(SessionPassKind::World, 7, 100));
    assert!(!authority.accept_phase_authority(SessionPassKind::PlayerRecovery, 6, 1_000));
    assert!(!authority.accept_phase_authority(SessionPassKind::PlayerRecovery, 7, 99));
    assert!(authority.accept_phase_authority(SessionPassKind::Map, 7, 100));
    assert!(authority.accept_phase_authority(SessionPassKind::PlayerRecovery, 7, 100));
    assert!(!authority.accept_phase_authority(SessionPassKind::World, 7, 100));
    assert!(!authority.accept_phase_authority(SessionPassKind::Map, 7, 100));
    assert!(!authority.accept_phase_authority(SessionPassKind::PlayerRecovery, 7, 100));
}

#[test]
fn phase_authority_allows_a_new_coordinator_to_start_at_a_lower_epoch() {
    let (mut session, _, _) = make_session();
    let authority = &mut session.core.admission;

    assert!(authority.accept_phase_authority(SessionPassKind::World, 4, 900));
    assert!(authority.accept_phase_authority(SessionPassKind::PlayerRecovery, 5, 1));
    assert!(!authority.accept_phase_authority(SessionPassKind::Map, 4, 901));
    assert!(!authority.accept_phase_authority(SessionPassKind::Map, 5, 0));
    assert!(authority.accept_phase_authority(SessionPassKind::Map, 5, 1));
}
