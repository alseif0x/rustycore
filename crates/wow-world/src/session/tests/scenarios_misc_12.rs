//! Session scenarios exercising the represented misc responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn feature_status_borrows_process_policy_without_session_copy_like_cpp() {
    let (session, _, _) = make_session();
    let policy = SupportFeaturePolicyLikeCpp {
        tickets_enabled: true,
        bugs_enabled: true,
        complaints_enabled: true,
        suggestions_enabled: true,
        character_undelete_enabled: true,
        bpay_store_enabled: true,
        max_characters_per_realm: 77,
        ..Default::default()
    };

    let in_world = session.feature_system_status_with_policy_like_cpp(&policy);
    assert!(in_world.config.support_tickets_enabled);
    assert!(in_world.config.support_bugs_enabled);
    assert!(in_world.config.support_complaints_enabled);
    assert!(in_world.config.support_suggestions_enabled);
    assert!(in_world.config.char_undelete_enabled);
    assert!(in_world.config.bpay_store_enabled);

    let glue = session.feature_system_status_glue_screen_with_policy_like_cpp(&policy);
    assert_eq!(glue.max_characters_per_realm, 77);
    assert!(glue.config.char_undelete_enabled);
    assert!(glue.config.bpay_store_enabled);
}
