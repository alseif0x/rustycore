//! Lifecycle and respawn fixtures.
//!
//! These builders retain the original session-test behavior and are
//! visible only within the parent `session::tests` subtree.

use super::*;

// ── Slice 4A.2b — respawn queue ownership migrated to MapInstance ──────

/// Prepare a dead creature whose corpse timer has already elapsed.
/// Returns the session and the creature GUID so the caller can drive
/// `run_creatures_tick` to trigger the despawn-then-respawn path.

/// After despawn, force the pending respawn entry to be immediately ready
/// by rewriting its `respawn_at` to the past via the map's queue.

// ── step_creature_movement_like_cpp unit tests (no WorldSession) ──────────

pub(in crate::session::tests) fn make_test_world_creature(
    guid: ObjectGuid,
) -> crate::map_manager::WorldCreature {
    crate::map_manager::WorldCreature::new(
        guid,
        9999,
        Position::new(10.0, 10.0, 0.0, 0.0),
        25,
        2,
        3,
        5,
        20.0,
        100,
        14,
        0,
        0,
    )
}
