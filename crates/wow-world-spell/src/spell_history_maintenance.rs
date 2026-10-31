//! Native Unit spell-history maintenance for an admitted Player update.

use wow_map::{MapKey, MapManager, PlayerHandle};

/// Advance the admitted Player's native spell history under the caller's MapManager guard.
///
/// C++ `Unit::Update` gates on in-world and calls `_UpdateSpells` first
/// (`Entities/Unit/Unit.cpp:418-432`); `_UpdateSpells` starts with
/// `SpellHistory::Update` (`Unit.cpp:2836+`, `Spells/SpellHistory.cpp:220-244`).
/// The map incarnation and active residence are rechecked on the same manager
/// borrow that reaches the canonical Unit, so a stale pass cannot mutate a
/// replacement or detached Player.
pub fn update_player_spell_history(
    manager: &mut MapManager,
    handle: PlayerHandle,
    expected_map: MapKey,
    expected_map_incarnation: u64,
    expected_residence_revision: u64,
    unix_time_ns: u128,
) -> bool {
    if manager.map_incarnation_like_cpp(expected_map) != Some(expected_map_incarnation)
        || manager.player_active_residence_revision_like_cpp(handle)
            != Some((expected_map, expected_residence_revision))
    {
        return false;
    }

    manager
        .with_player_mut_like_cpp(handle, |player| {
            let world = player.unit().world();
            if player.guid() != handle.guid()
                || world.object().guid() != handle.guid()
                || !world.object().is_in_world()
                || !world.has_current_map()
                || MapKey::new(world.map_id(), world.instance_id()) != expected_map
            {
                return false;
            }

            player
                .unit_mut()
                .subsystems_mut()
                .spells
                .history
                .update_at_unix_nanos(unix_time_ns);
            true
        })
        .unwrap_or(false)
}
