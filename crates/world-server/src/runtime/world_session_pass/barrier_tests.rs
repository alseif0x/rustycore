//! The cross-step barrier must retain a phase interrupted after its first effect.
use super::*;
use wow_world::session::mailbox::SessionPhaseClaimLikeCpp;

#[test]
fn interrupted_effects_remain_a_barrier_across_producer_steps() {
    let interrupted = SessionPhasePermitLikeCpp::new_like_cpp();
    let running = SessionPhasePermitLikeCpp::new_like_cpp();
    let revoked = SessionPhasePermitLikeCpp::new_like_cpp();
    let refused = SessionPhasePermitLikeCpp::new_like_cpp();
    assert_eq!(
        interrupted.claim_like_cpp(),
        SessionPhaseClaimLikeCpp::Claimed
    );
    assert!(interrupted.interrupt_after_start_like_cpp());
    assert_eq!(running.claim_like_cpp(), SessionPhaseClaimLikeCpp::Claimed);
    revoked.revoke_before_start_like_cpp();
    assert!(refused.refuse_before_start_like_cpp());
    let mut barrier = vec![
        Arc::clone(&interrupted),
        Arc::clone(&running),
        revoked,
        refused,
    ];

    retain_unresolved_phase_permits_like_cpp(&mut barrier);
    assert_eq!(barrier.len(), 2);
    assert!(Arc::ptr_eq(&barrier[0], &interrupted));
    assert!(Arc::ptr_eq(&barrier[1], &running));

    assert!(running.complete_like_cpp());
    retain_unresolved_phase_permits_like_cpp(&mut barrier);
    assert_eq!(barrier.len(), 1);
    assert!(Arc::ptr_eq(&barrier[0], &interrupted));
    assert!(!interrupted.complete_like_cpp());
    retain_unresolved_phase_permits_like_cpp(&mut barrier);
    assert_eq!(
        barrier.len(),
        1,
        "an interruption cannot become a successful later step"
    );
}
