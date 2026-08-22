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

/// Timed logout sends `SMSG_LOGOUT_COMPLETE` and moves the session to
/// disconnecting, but must keep the represented player alive: C++ saves while
/// `_player` still exists, so clearing it here would make the later
/// disconnect save a no-op.
#[test]
fn complete_logout_notifies_the_client_and_keeps_the_player_for_the_save_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player = ObjectGuid::create_player(1, 0x5100_0003);
    session.set_player_guid(Some(player));
    session.set_state(SessionState::LoggedIn);

    session.complete_logout();

    let packet = send_rx.try_recv().expect("LogoutComplete");
    assert_eq!(
        u16::from_le_bytes([packet[0], packet[1]]),
        ServerOpcodes::LogoutComplete as u16
    );
    assert!(session.is_disconnecting());
    assert_eq!(
        session.player_guid(),
        Some(player),
        "the disconnect save still needs the represented player"
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
    assert!(session.try_claim_character_login_like_cpp(guid));
    assert!(!next.try_claim_character_login_like_cpp(guid));

    session.cleanup_shared_runtime_state();

    assert!(
        next.try_claim_character_login_like_cpp(guid),
        "cleanup must free the claim for the next login"
    );
    next.release_character_login_claim_like_cpp();
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
