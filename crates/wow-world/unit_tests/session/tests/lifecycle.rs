// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Login claim, logout finalize and disconnect cleanup, through the real
//! entry points in `session::lifecycle`.

use super::*;

/// One character, one live login: a second session cannot claim it while the
/// first still holds it, and the claim returns after an explicit release.
#[test]
fn character_login_claim_is_exclusive_and_released_like_cpp() {
    let guid = ObjectGuid::create_player(1, 0x5100_0001);
    let (mut first, _, _) = make_session();
    let (mut second, _, _) = make_session();

    assert!(first.try_claim_character_login_like_cpp(guid));
    // Re-claiming from the owner is idempotent, not a self-lockout.
    assert!(first.try_claim_character_login_like_cpp(guid));
    assert!(!second.try_claim_character_login_like_cpp(guid));

    first.release_character_login_claim_like_cpp();
    assert!(second.try_claim_character_login_like_cpp(guid));
    second.release_character_login_claim_like_cpp();
}

/// Releasing a claim the session never took must not free another session's.
#[test]
fn releasing_without_a_claim_leaves_the_owner_untouched_like_cpp() {
    let guid = ObjectGuid::create_player(1, 0x5100_0002);
    let (mut owner, _, _) = make_session();
    let (mut other, _, _) = make_session();
    assert!(owner.try_claim_character_login_like_cpp(guid));

    other.release_character_login_claim_like_cpp();

    assert!(!other.try_claim_character_login_like_cpp(guid));
    owner.release_character_login_claim_like_cpp();
}

fn logout_opcodes_like_cpp(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<(u16, Vec<u8>)> {
    send_rx
        .try_iter()
        .map(|bytes| (u16::from_le_bytes([bytes[0], bytes[1]]), bytes))
        .collect()
}

fn logged_in_canonical_session_like_cpp() -> (
    WorldSession,
    flume::Sender<WorldPacket>,
    flume::Receiver<Vec<u8>>,
) {
    let (mut session, pkt_tx, send_rx) = make_session();
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);
    session.set_state(SessionState::LoggedIn);
    (session, pkt_tx, send_rx)
}

fn logout_response_bytes_like_cpp(logout_result: i32, instant: bool) -> Vec<u8> {
    wow_packet::ServerPacket::to_bytes(&wow_packet::packets::misc::LogoutResponse {
        logout_result,
        instant,
    })
}

/// C++ `HandleLogoutRequestOpcode` (`MiscHandler.cpp:243-258`): the denial
/// order is combat, falling, then duel/GM freeze; resting allows combat;
/// `IdleLogout` ignores resting and the permission, never the taxi.
#[test]
fn logout_admission_follows_the_cpp_reason_order_and_instant_rules() {
    use wow_world_character::{LogoutDenialLikeCpp, logout_admission_like_cpp};
    let base = wow_world_core::session::LogoutAdmissionSnapshotLikeCpp {
        in_flight: false,
        resting: false,
        in_combat: false,
        falling: false,
        dueling_or_frozen: false,
        can_free_move: true,
        stand_state: wow_constants::UnitStandStateType::Stand,
    };
    assert_eq!(
        logout_admission_like_cpp(&base, false, false),
        (None, false)
    );
    let all_denials = wow_world_core::session::LogoutAdmissionSnapshotLikeCpp {
        in_combat: true,
        falling: true,
        dueling_or_frozen: true,
        ..base
    };
    assert_eq!(
        logout_admission_like_cpp(&all_denials, false, false).0,
        Some(LogoutDenialLikeCpp::InCombat)
    );
    let falling_duel = wow_world_core::session::LogoutAdmissionSnapshotLikeCpp {
        in_combat: false,
        ..all_denials
    };
    assert_eq!(
        logout_admission_like_cpp(&falling_duel, false, false).0,
        Some(LogoutDenialLikeCpp::Falling)
    );
    let duel = wow_world_core::session::LogoutAdmissionSnapshotLikeCpp {
        dueling_or_frozen: true,
        ..base
    };
    assert_eq!(
        logout_admission_like_cpp(&duel, false, true),
        (Some(LogoutDenialLikeCpp::DuelingOrFrozen), true),
        "the permission keeps `Instant` set even on a denial"
    );
    // Resting in combat: admitted (`canLogoutInCombat`) but not instant.
    let resting_combat = wow_world_core::session::LogoutAdmissionSnapshotLikeCpp {
        resting: true,
        in_combat: true,
        ..base
    };
    assert_eq!(
        logout_admission_like_cpp(&resting_combat, false, false),
        (None, false)
    );
    let resting = wow_world_core::session::LogoutAdmissionSnapshotLikeCpp {
        resting: true,
        ..base
    };
    assert_eq!(
        logout_admission_like_cpp(&resting, false, false),
        (None, true)
    );
    assert_eq!(
        logout_admission_like_cpp(&resting, true, false),
        (None, false)
    );
    assert_eq!(logout_admission_like_cpp(&base, false, true), (None, true));
    assert_eq!(logout_admission_like_cpp(&base, true, true), (None, false));
    let taxi = wow_world_core::session::LogoutAdmissionSnapshotLikeCpp {
        in_flight: true,
        ..base
    };
    assert_eq!(logout_admission_like_cpp(&taxi, true, false), (None, true));
}

/// Timed logout: `LogoutResponse(0, false)`, then sit, root and stun on the
/// canonical Player, and the countdown starts; nothing is finalized yet.
#[tokio::test]
async fn timed_logout_request_sits_roots_stuns_and_starts_the_countdown_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = logged_in_canonical_session_like_cpp();
    let guid = session.player_guid().unwrap();

    session
        .handle_logout_request(wow_packet::packets::misc::LogoutRequest { idle_logout: false })
        .await;

    let sent = logout_opcodes_like_cpp(&send_rx);
    assert_eq!(sent[0].1, logout_response_bytes_like_cpp(0, false));
    let opcodes: Vec<u16> = sent.iter().map(|(opcode, _)| *opcode).collect();
    let position = |opcode: wow_constants::ServerOpcodes| {
        opcodes
            .iter()
            .position(|sent| *sent == opcode as u16)
            .unwrap_or_else(|| panic!("{opcode:?} not sent: {opcodes:x?}"))
    };
    // C++ order: SetStandState(SIT), SetRooted(true), SetUnitFlag(STUNNED).
    assert!(
        position(wow_constants::ServerOpcodes::StandStateUpdate)
            < position(wow_constants::ServerOpcodes::MoveRoot)
    );
    assert!(!opcodes.contains(&(wow_constants::ServerOpcodes::LogoutComplete as u16)));

    let (stand, flags, unit_flags) = session
        .core
        .with_owned_player_like_cpp(|player| {
            (
                player.unit().stand_state_like_cpp(),
                player.unit().movement_flags_like_cpp(),
                player.unit().unit_flags_like_cpp(),
            )
        })
        .unwrap();
    assert_eq!(stand, wow_constants::UnitStandStateType::Sit);
    assert!(flags.contains(wow_constants::MovementFlag::ROOT));
    assert!(unit_flags.contains(wow_constants::UnitFlags::STUNNED));
    assert_ne!(session.lifecycle.logout_time_for_test_like_cpp(), 0);
    assert!(session.lifecycle.is_logging_out_like_cpp());
    assert_eq!(session.state(), SessionState::LoggedIn);
    assert_eq!(session.player_guid(), Some(guid));
    assert!(session.finalization_report_like_cpp().is_none());
}

/// `HandleLogoutCancelOpcode`: clear the timer, `LogoutCancelAck`, then
/// unroot, stand up and clear `UNIT_FLAG_STUNNED`.
#[tokio::test]
async fn logout_cancel_acks_and_restores_movement_like_cpp() {
    use wow_world_lifecycle::AccountDataHandlerHostLikeCpp;
    let (mut session, _pkt_tx, send_rx) = logged_in_canonical_session_like_cpp();
    session
        .handle_logout_request(wow_packet::packets::misc::LogoutRequest { idle_logout: false })
        .await;
    let _ = logout_opcodes_like_cpp(&send_rx);

    let catalogs = crate::session::SessionHandlerCatalogsLikeCpp::default();
    assert!(
        session
            .account_data_handler_cx_like_cpp(&catalogs)
            .handle_logout_cancel()
            .await
    );
    session.restore_logout_wait_movement_like_cpp();

    let opcodes: Vec<u16> = logout_opcodes_like_cpp(&send_rx)
        .into_iter()
        .map(|(opcode, _)| opcode)
        .collect();
    assert_eq!(
        opcodes[0],
        wow_constants::ServerOpcodes::LogoutCancelAck as u16
    );
    let unroot = opcodes
        .iter()
        .position(|opcode| *opcode == wow_constants::ServerOpcodes::MoveUnroot as u16)
        .expect("MoveUnroot");
    let stand = opcodes
        .iter()
        .position(|opcode| *opcode == wow_constants::ServerOpcodes::StandStateUpdate as u16)
        .expect("StandStateUpdate");
    assert!(unroot < stand, "C++ unroots before standing up");
    let (stand, flags, unit_flags) = session
        .core
        .with_owned_player_like_cpp(|player| {
            (
                player.unit().stand_state_like_cpp(),
                player.unit().movement_flags_like_cpp(),
                player.unit().unit_flags_like_cpp(),
            )
        })
        .unwrap();
    assert_eq!(stand, wow_constants::UnitStandStateType::Stand);
    assert!(!flags.contains(wow_constants::MovementFlag::ROOT));
    assert!(!unit_flags.contains(wow_constants::UnitFlags::STUNNED));
    assert_eq!(session.lifecycle.logout_time_for_test_like_cpp(), 0);
    assert_eq!(session.state(), SessionState::LoggedIn);

    // No player: too late to cancel, no acknowledgement.
    let (mut detached, _pkt_tx, detached_rx) = make_session();
    assert!(
        !detached
            .account_data_handler_cx_like_cpp(&catalogs)
            .handle_logout_cancel()
            .await
    );
    assert!(detached_rx.try_recv().is_err());
}

/// Denied in combat: `LogoutResponse(1, false)`, no countdown, no root.
#[tokio::test]
async fn logout_request_in_combat_is_denied_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = logged_in_canonical_session_like_cpp();
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .subsystems_mut()
                .combat
                .set_in_combat_with(ObjectGuid::create_player(1, 0x5100_00FF), true, false)
        })
        .unwrap();

    session
        .handle_logout_request(wow_packet::packets::misc::LogoutRequest { idle_logout: false })
        .await;

    let sent = logout_opcodes_like_cpp(&send_rx);
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].1, logout_response_bytes_like_cpp(1, false));
    assert_eq!(session.lifecycle.logout_time_for_test_like_cpp(), 0);
    let flags = session
        .core
        .with_owned_player_like_cpp(|player| player.unit().movement_flags_like_cpp())
        .unwrap();
    assert!(!flags.contains(wow_constants::MovementFlag::ROOT));
}

/// Denied while falling (3) and while dueling or GM-frozen (2).
#[tokio::test]
async fn logout_request_falling_or_dueling_is_denied_like_cpp() {
    for (result, mutate) in [
        (
            3,
            Box::new(|player: &mut wow_entities::Player| {
                player
                    .unit_mut()
                    .set_movement_flags_like_cpp(wow_constants::MovementFlag::FALLING);
            }) as Box<dyn Fn(&mut wow_entities::Player)>,
        ),
        (
            2,
            Box::new(|player: &mut wow_entities::Player| {
                player.set_duel_info_like_cpp(Some(wow_entities::PlayerDuelInfoLikeCpp {
                    opponent: ObjectGuid::create_player(1, 0x5100_00FE),
                    state: wow_entities::PlayerDuelStateLikeCpp::InProgress,
                }));
            }),
        ),
    ] {
        let (mut session, _pkt_tx, send_rx) = logged_in_canonical_session_like_cpp();
        session
            .core
            .with_owned_player_mut_like_cpp(|player| mutate(player))
            .unwrap();
        session
            .handle_logout_request(wow_packet::packets::misc::LogoutRequest { idle_logout: false })
            .await;
        let sent = logout_opcodes_like_cpp(&send_rx);
        assert_eq!(sent[0].1, logout_response_bytes_like_cpp(result, false));
        assert_eq!(session.lifecycle.logout_time_for_test_like_cpp(), 0);
    }
}

/// Resting in an inn/city, or the instant-logout permission (GM), answers
/// `Instant` and runs `LogoutPlayer(true)` at once; an idle logout while
/// resting waits.
#[tokio::test]
async fn resting_or_gm_logout_is_instant_and_idle_logout_waits_like_cpp() {
    for (resting, security, idle, instant) in [
        (true, 0, false, true),
        (false, 1, false, true),
        (true, 0, true, false),
        (false, 1, true, false),
    ] {
        let (mut session, _pkt_tx, send_rx) = logged_in_canonical_session_like_cpp();
        session.core.security = security;
        if resting {
            session
                .core
                .with_owned_player_mut_like_cpp(|player| {
                    player.set_player_flag(crate::session::PLAYER_FLAGS_RESTING_LIKE_CPP)
                })
                .unwrap();
        }
        session
            .handle_logout_request(wow_packet::packets::misc::LogoutRequest { idle_logout: idle })
            .await;
        let sent = logout_opcodes_like_cpp(&send_rx);
        assert_eq!(sent[0].1, logout_response_bytes_like_cpp(0, instant));
        if instant {
            assert_eq!(
                session
                    .finalization_report_like_cpp()
                    .map(|report| report.mode),
                Some(crate::FinalizationMode::CharacterSelection),
                "instant logout is LogoutPlayer(true)"
            );
            assert_eq!(session.lifecycle.logout_time_for_test_like_cpp(), 0);
        } else {
            assert!(session.finalization_report_like_cpp().is_none());
            assert_ne!(session.lifecycle.logout_time_for_test_like_cpp(), 0);
        }
    }
}

/// `ShouldLogOut`: due 20 game-time seconds after the start, never while a
/// player is loading; the due timer becomes `LogoutPlayer(true)` with the
/// timed reason, not a disconnect.
#[tokio::test]
async fn logout_timer_fires_after_twenty_seconds_into_character_selection_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = logged_in_canonical_session_like_cpp();
    let now = wow_entities::game_time_secs_like_cpp();
    session.lifecycle.set_logout_start_time_like_cpp(now - 18);
    assert!(!session.lifecycle.should_log_out_like_cpp(now + 1));
    assert!(session.lifecycle.should_log_out_like_cpp(now + 2));

    session.update(100).await;
    assert_ne!(session.lifecycle.logout_time_for_test_like_cpp(), 0);
    assert!(session.finalization_report_like_cpp().is_none());

    session.lifecycle.set_logout_start_time_like_cpp(now - 20);
    session.update(100).await;
    assert_eq!(session.lifecycle.logout_time_for_test_like_cpp(), 0);
    let report = session
        .finalization_report_like_cpp()
        .expect("the due timer runs LogoutPlayer(true)");
    assert_eq!(report.mode, crate::FinalizationMode::TimedLogout);
    // The timer no longer marks the session `Disconnecting` itself (C++
    // `LogoutPlayer(true)` keeps it); this fixture has no persistence ports,
    // so the save fails closed and the ledger stops before any account mark.
    assert_eq!(
        report.outcome(crate::FinalizationStep::LoginAccountOffline),
        crate::FinalizationOutcome::NotAttempted
    );
}

/// Cleanup releases the login claim, so a disconnect never leaves a character
/// permanently unloggable.
#[test]
fn shared_runtime_cleanup_releases_the_login_claim_like_cpp() {
    let guid = ObjectGuid::create_player(1, 0x5100_0004);
    let (mut session, _, _) = make_session();
    let (mut next, _, _) = make_session();
    session.set_player_guid(Some(guid));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 1, 0);
    assert!(session.lifecycle.try_claim_character_login_like_cpp(guid));
    assert!(!next.try_claim_character_login_like_cpp(guid));

    session.cleanup_shared_runtime_state();

    assert!(
        next.try_claim_character_login_like_cpp(guid),
        "cleanup must free the claim for the next login"
    );
    next.release_character_login_claim_like_cpp();
}

#[test]
fn recovery_fences_application_and_lower_retirement_keep_the_canonical_incarnation() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 46_701);
    assert!(session.try_claim_character_login_like_cpp(guid));
    session.set_player_guid(Some(guid));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    let handle = session
        .core
        .player_handle_like_cpp
        .expect("canonical Player handle");
    let manager = session
        .core
        .canonical_map_manager
        .as_ref()
        .expect("canonical MapManager")
        .clone();
    session
        .with_owned_player_mut_like_cpp(|player| {
            assert!(player.begin_resurrection_operation(
                wow_entities::PlayerResurrectionOperation::ReclaimCorpse {
                    restore_percent: 0.5,
                },
                wow_entities::PlayerResurrectionPhase::RecoveryCorpseBonesInFlight,
            ));
        })
        .unwrap();

    let lower_outcome = {
        let (state, mut hub) = crate::session::split_lifecycle_mut(&mut session);
        state.unregister_canonical_player_from_map_like_cpp(&mut hub)
    };
    assert_eq!(
        lower_outcome,
        wow_world_lifecycle::FinalizationOutcome::Deferred
    );
    assert_eq!(session.core.player_handle_like_cpp, Some(handle));
    assert!(
        manager
            .lock()
            .unwrap()
            .with_player_like_cpp(handle, |player| player.guid() == guid)
            .unwrap()
    );

    assert_eq!(
        session.cleanup_shared_runtime_state(),
        wow_world_lifecycle::FinalizationOutcome::Deferred,
        "application cleanup refuses before registry unpublication or claim release"
    );
    assert_eq!(session.core.player_handle_like_cpp, Some(handle));
    assert_eq!(session.player_guid(), Some(guid));
    let mut replacement = make_session().0;
    assert!(!replacement.try_claim_character_login_like_cpp(guid));
}

/// Cleanup is safe on a session that never selected a character — the
/// disconnect path runs it unconditionally.
#[test]
fn shared_runtime_cleanup_is_safe_without_a_player_like_cpp() {
    let (mut session, _, _) = make_session();
    assert!(session.player_guid().is_none());

    session.cleanup_shared_runtime_state();

    assert!(session.player_guid().is_none());
}

/// The disconnect save with no selected character takes the short path: it
/// marks the account offline and returns without touching character state.
#[tokio::test]
async fn disconnect_save_without_a_player_only_marks_the_account_offline_like_cpp() {
    let (mut session, _, _) = make_session();
    assert!(session.player_guid().is_none());

    session.save_disconnect_player_to_db_like_cpp().await;

    assert!(!session.player_logout_like_cpp());
}

/// The logout flag is the guard the save path sets before it runs.
#[test]
fn player_logout_flag_round_trips_like_cpp() {
    let (mut session, _, _) = make_session();
    assert!(!session.player_logout_like_cpp());

    session.set_player_logout_like_cpp(true);
    assert!(session.player_logout_like_cpp());

    session.set_player_logout_like_cpp(false);
    assert!(!session.player_logout_like_cpp());
}
