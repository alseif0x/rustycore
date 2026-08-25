//! Read one canonical `Player` by GUID and placement, without going through the
//! owning [`WorldSession`](crate::session::WorldSession).
//!
//! Issue #252 retires the temporary `PlayerBroadcastInfo` gameplay mirror. Each
//! mirrored field existed because a remote session appeared unable to reach
//! another player's canonical `Player`. The data was never actually unreachable:
//! `WorldSession::canonical_player_snapshot_like_cpp` already resolves it out of
//! the shared canonical `MapManager`. It was only *unaddressable*, because that
//! accessor is bound to `self` and therefore answers for one GUID — the
//! session's own. The functions here take the placement explicitly, so any
//! caller holding a GUID plus the map key the session directory already stores
//! reads the canonical owner directly, and the mirrored copy can retire.
//!
//! Lock discipline: every function here acquires the canonical `MapManager`
//! mutex and releases it before returning. A caller reading the session
//! directory must copy the placement it needs out of the registry entry and drop
//! that `DashMap` guard *before* calling in. No path then holds a registry shard
//! guard and the canonical mutex at the same time, so this introduces no lock
//! nesting and no new ordering obligation.

use wow_core::ObjectGuid;
use wow_entities::Player;

use crate::session::SharedCanonicalMapManager;

/// Borrow the canonical `Player` for `guid` on one exact map instance.
///
/// Returns `None` when the mutex is poisoned, the map instance is not resident,
/// or the GUID names no player on it — the same three misses the session-bound
/// accessor already treats as "no canonical value".
pub(crate) fn with_canonical_player_at_like_cpp<R>(
    manager: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    map_id: u32,
    instance_id: u32,
    read: impl FnOnce(&Player) -> R,
) -> Option<R> {
    let manager = manager.lock().ok()?;
    let map = manager.find_map(map_id, instance_id)?;
    let player = map.map().get_typed_player(guid)?;
    Some(read(player))
}

/// The seven honor counters an inspect response carries, in C++ field order.
pub(crate) type HonorStatsLikeCpp = (u32, u32, u32, u16, u16, u32, u32);

/// Read the honor block off a canonical `Player`.
///
/// This is the only reader of the honor counters. Before #252 the same values
/// were copied into the broadcast mirror at registration and refreshed again on
/// every registry sync; both copies are gone, so an inspect response can no
/// longer show a stale honor level.
pub(crate) fn canonical_player_honor_stats_like_cpp(player: &Player) -> HonorStatsLikeCpp {
    (
        // C++ reads these six values from `m_activePlayerData`, but Rust's
        // canonical `ActivePlayerDataValues` does not model the historic honor
        // counters yet. Keep the packet field order represented and leave the
        // counters zero until that L4/PvP state is ported.
        0,
        0,
        0,
        0,
        0,
        0,
        player.data().honor_level.max(0) as u32,
    )
}
