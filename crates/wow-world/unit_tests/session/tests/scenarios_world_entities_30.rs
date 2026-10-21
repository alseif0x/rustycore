//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[path = "scenarios_world_entities_30/creature_module_events.rs"]
mod creature_module_events;

#[test]
fn respawn_queue_lives_in_map_not_in_session_like_cpp() {
    // After a creature is despawned and queued for respawn, the pending entry
    // must be stored in the canonical map's respawn queue, not in WorldSession.
    let (session, _, guid) = setup_dead_creature_past_despawn(91_001);
    let canonical = session.core.canonical_map_manager.clone().unwrap();
    let map_store = canonical_player_transfer_test_map_store_like_cpp();

    // The lifecycle phase despawns the corpse and queues it on the MAP.
    let outcome = run_canonical_creature_lifecycle_phase_for_test_like_cpp(
        &canonical,
        &map_store,
        Instant::now(),
    );
    assert_eq!(outcome.corpses_despawned, 1);

    let guard = canonical.lock().unwrap();
    assert!(guard.find_creature(0, 0, guid).is_none());
    assert_eq!(
        guard.respawn_queue_len(0, 0),
        1,
        "pending respawn must be in the map queue"
    );
}
#[test]
fn single_session_respawn_rebuilds_the_incarnation_and_refreshes_its_map_like_cpp() {
    // Kill creature → corpse despawn → respawn ready → the lifecycle phase
    // rebuilds and admits the incarnation and names its map for the visibility
    // refresh that sends the CREATE (delivered by the map loop; the end-to-end
    // CREATE is pinned by world-server `scenarios_16`).
    let (mut session, _send_rx, guid) = setup_dead_creature_past_despawn(91_002);
    let canonical = session.core.canonical_map_manager.clone().unwrap();
    let map_store = canonical_player_transfer_test_map_store_like_cpp();

    // Phase 1: despawn — removes the creature from the map and enqueues respawn.
    let despawn = run_canonical_creature_lifecycle_phase_for_test_like_cpp(
        &canonical,
        &map_store,
        Instant::now(),
    );
    assert_eq!(despawn.corpses_despawned, 1);
    assert_eq!(despawn.refresh_map_keys, vec![(0, 0)]);

    // Force the queued respawn to be immediately ready.
    force_respawn_ready(&mut session);

    // Phase 2: respawn fires — the incarnation is back, alive, and its map is
    // refreshed.
    let respawn = run_canonical_creature_lifecycle_phase_for_test_like_cpp(
        &canonical,
        &map_store,
        Instant::now(),
    );
    assert_eq!(respawn.respawns_processed, 1);
    assert_eq!(respawn.refresh_map_keys, vec![(0, 0)]);
    let guard = canonical.lock().unwrap();
    let respawned = guard
        .find_creature(0, 0, guid)
        .expect("the respawn admits the incarnation on its map");
    assert!(respawned.is_alive());
    assert_eq!(guard.respawn_queue_len(0, 0), 0);
}
