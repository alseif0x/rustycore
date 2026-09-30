// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The admitted-pass permit decides the coordinator/session race exactly once.

use super::*;

#[test]
fn a_revoked_pass_can_never_be_claimed_like_cpp() {
    let permit = SessionPhasePermitLikeCpp::new_like_cpp();

    assert_eq!(
        permit.revoke_before_start_like_cpp(),
        SessionPhaseRevokeLikeCpp::Revoked
    );
    // The session reaching the delivery after the deadline must run no effect.
    assert_eq!(permit.claim_like_cpp(), SessionPhaseClaimLikeCpp::Revoked);
    assert!(!permit.complete_like_cpp());
    assert_eq!(
        permit.state_like_cpp(),
        SessionPhasePermitStateLikeCpp::RevokedBeforeStart
    );
}

#[test]
fn a_claimed_pass_cannot_be_revoked_so_the_deadline_is_diagnostic_only() {
    let permit = SessionPhasePermitLikeCpp::new_like_cpp();

    assert_eq!(permit.claim_like_cpp(), SessionPhaseClaimLikeCpp::Claimed);
    // The coordinator's deadline arrives while the session is mutating. It
    // learns that, and learning it is not permission to continue.
    assert_eq!(
        permit.revoke_before_start_like_cpp(),
        SessionPhaseRevokeLikeCpp::AlreadyRunning
    );
    assert!(permit.complete_like_cpp());
    assert_eq!(
        permit.state_like_cpp(),
        SessionPhasePermitStateLikeCpp::Completed
    );
}

#[test]
fn a_duplicate_delivery_of_the_same_pass_claims_nothing() {
    let permit = SessionPhasePermitLikeCpp::new_like_cpp();

    assert_eq!(permit.claim_like_cpp(), SessionPhaseClaimLikeCpp::Claimed);
    assert_eq!(
        permit.claim_like_cpp(),
        SessionPhaseClaimLikeCpp::AlreadyResolved(SessionPhasePermitStateLikeCpp::Running)
    );
    assert!(permit.complete_like_cpp());
    assert_eq!(
        permit.claim_like_cpp(),
        SessionPhaseClaimLikeCpp::AlreadyResolved(SessionPhasePermitStateLikeCpp::Completed)
    );
    // Completion happens once: a second one would let two coordinators believe
    // they both observed this pass end.
    assert!(!permit.complete_like_cpp());
}

#[test]
fn refusal_and_interruption_are_distinct_terminal_states_like_cpp() {
    let refused = SessionPhasePermitLikeCpp::new_like_cpp();
    assert!(refused.refuse_before_start_like_cpp());
    assert_eq!(
        refused.state_like_cpp(),
        SessionPhasePermitStateLikeCpp::RefusedBeforeStart
    );
    // A refusal before any effect is as safe as a revocation.
    assert_eq!(
        refused.revoke_before_start_like_cpp(),
        SessionPhaseRevokeLikeCpp::AlreadyResolved(
            SessionPhasePermitStateLikeCpp::RefusedBeforeStart
        )
    );

    let interrupted = SessionPhasePermitLikeCpp::new_like_cpp();
    assert_eq!(
        interrupted.claim_like_cpp(),
        SessionPhaseClaimLikeCpp::Claimed
    );
    assert!(interrupted.interrupt_after_start_like_cpp());
    // Interruption is never laundered into completion or refusal: its mutations
    // neither finished nor provably rolled back.
    assert!(!interrupted.complete_like_cpp());
    assert!(!interrupted.refuse_before_start_like_cpp());
    assert_eq!(
        interrupted.state_like_cpp(),
        SessionPhasePermitStateLikeCpp::InterruptedAfterStart
    );
}

#[test]
fn a_pass_that_was_never_claimed_is_revocable_after_the_session_is_gone() {
    let permit = SessionPhasePermitLikeCpp::new_like_cpp();
    let session_side = Arc::clone(&permit);
    // The session task ends without ever reaching the delivery, as at shutdown.
    drop(session_side);

    // Dropping a handle changes nothing: only an explicit transition resolves
    // the pass, and the coordinator can still prove this one never ran.
    assert_eq!(
        permit.state_like_cpp(),
        SessionPhasePermitStateLikeCpp::Pending
    );
    assert_eq!(
        permit.revoke_before_start_like_cpp(),
        SessionPhaseRevokeLikeCpp::Revoked
    );
}
