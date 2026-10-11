//! Canonical creature store fixtures shared by the session and handler tests.
//!
//! #1263 F6-8E deleted the legacy creature store: the canonical map is the only
//! creature store. [`CanonicalCreatureFixtureLikeCpp`] is read/setup sugar over
//! the canonical map for scenario tests: it addresses a creature by
//! `(map_id, instance_id, guid)` and lends the canonical `Creature` itself —
//! there is no copy.

use std::sync::Arc;

use crate::session::SharedCanonicalMapManager;

/// The session-free canonical store fixtures now live in the dev-only
/// `wow-world-test-support` crate (#1363 slice 0); this module keeps their
/// established `crate::canonical_creature_fixtures` path for the tests.
pub(crate) use wow_world_test_support::canonical_creatures::*;

/// Give `session` a canonical creature store when it has none — with the
/// canonical map at its runtime map key, so a creature registered through the
/// session is admitted as a canonical incarnation. A store the session already
/// has is left as configured (a missing map keeps deferring registrations).
pub(crate) fn ensure_canonical_creature_store_for_session_like_cpp(
    session: &mut crate::session::WorldSession,
) -> SharedCanonicalMapManager {
    if let Some(canonical) = session.core.canonical_map_manager.clone() {
        return canonical;
    }
    ensure_canonical_creature_store_and_map_for_session_like_cpp(session)
}

/// Give `session` a canonical creature store when it has none, and the
/// canonical map at its runtime map key either way (`create_world_map` is
/// idempotent): the store the former legacy registration always provided.
pub(crate) fn ensure_canonical_creature_store_and_map_for_session_like_cpp(
    session: &mut crate::session::WorldSession,
) -> SharedCanonicalMapManager {
    let canonical = match session.core.canonical_map_manager.clone() {
        Some(canonical) => canonical,
        None => {
            let canonical = shared_canonical_map_manager();
            session.set_canonical_map_manager(Arc::clone(&canonical));
            canonical
        }
    };
    let (map_id, instance_id) = session.core.current_runtime_map_key_like_cpp();
    canonical
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .create_world_map(u32::from(map_id), instance_id);
    canonical
}
