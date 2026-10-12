//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn legacy_creature_spell_tick_is_noop_under_session_owner_like_cpp() {
    let manager = shared_map_manager();
    let spell_id = 70_004_i32;
    let config = creature_ai_spell_test_config_like_cpp(
        creature_ai_test_spell_info_like_cpp(spell_id, 6, 0),
        false,
        30.0,
    );

    let outcome = run_canonical_creature_spell_phase_for_test_like_cpp(&manager, &config);
    assert_eq!(outcome.maps_seen, 0);
    assert!(outcome.plan.events.is_empty());
}
