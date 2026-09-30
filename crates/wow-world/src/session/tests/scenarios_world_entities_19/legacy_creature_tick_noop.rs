//! Legacy creature melee and aggro no-op scenarios.

use super::*;

#[test]
fn legacy_creature_aggro_tick_once_is_noop_under_session_owner_like_cpp() {
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_005);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    let player = ObjectGuid::create_player(1, 91_006);
    let candidates = vec![legacy_aggro_candidate_like_cpp(player, Position::ZERO)];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_with_rate_like_cpp(3.0),
    );

    assert!(outcome.skipped_owner_not_global);
    assert_eq!(outcome.maps_seen, 0);
    assert!(outcome.commands.is_empty());
    let combat_target = {
        let guard = manager.read().unwrap();
        guard
            .find_creature(0, 0, creature_guid)
            .unwrap()
            .creature
            .ai_ownership()
            .combat_target
    };
    assert_eq!(combat_target, None);
}
