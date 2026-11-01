//! Packet draining and wire-summary fixtures.
//!
//! These builders retain the original session-test behavior and are
//! visible only within the parent `session::tests` subtree.

use super::*;

// ── Slice 4A.1b: SendIfVisibleLikeCpp per-session gate tests ────────────
// C++ anchor: GridNotifiers.h : MessageDistDeliverer::SendPacket — HaveAtClient
// (client_visible_guids_like_cpp) is the final gate; map/instance filter first.

pub(in crate::session::tests) fn install_committed_canonical_player_health_for_melee_test_like_cpp(
    session: &mut WorldSession,
    victim_guid: ObjectGuid,
    health: u64,
    death_state: wow_constants::DeathState,
) -> u64 {
    let canonical = shared_canonical_map_manager();
    add_canonical_test_player_on_map(&canonical, victim_guid, Position::ZERO, 571, 0);
    let revision = {
        let mut guard = canonical.lock().unwrap();
        let player = guard
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(victim_guid)
            .unwrap();
        player.unit_mut().set_max_health(100);
        player.unit_mut().set_health(100);
        player.unit_mut().set_health(health);
        player.unit_mut().set_death_state(death_state);
        player.unit().health_state_revision_like_cpp()
    };
    session.set_canonical_map_manager(canonical);
    revision
}

pub(in crate::session::tests) fn expected_gameobject_dynamic_flags_update_like_cpp(
    guid: ObjectGuid,
    map_id: u16,
    dynamic_flags: u32,
) -> Vec<u8> {
    crate::session::represented_gameobject_dynamic_flags_update_like_cpp(
        guid,
        map_id,
        dynamic_flags,
    )
    .expect("dynamic flags update")
    .to_bytes()
}
