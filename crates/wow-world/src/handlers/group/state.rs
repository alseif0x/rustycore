//! Group handlers state facade.
//!
//! The session-independent group projections, fan-out and packet builders now
//! live in `wow-world-social::group_fanout` (#1263 F5); this facade keeps the
//! original `state::…` paths working and holds the two helpers that need the
//! `WorldSession` itself. Behaviour is preserved.

use super::*;
pub(crate) use wow_world_lifecycle::group_persistence_command_like_cpp;

pub(super) use wow_world_social::group_fanout::*;

pub(super) fn current_player_party_invite_map_instance_like_cpp(
    session: &WorldSession,
    registry: &PlayerRegistry,
    player_guid: ObjectGuid,
) -> (u16, u32) {
    if let Some(key) = session.core.current_canonical_player_map_key_like_cpp() {
        return (key.map_id.min(u32::from(u16::MAX)) as u16, key.instance_id);
    }

    registry
        .group_presence(player_guid)
        .map(|entry| (entry.map_id, entry.instance_id))
        .unwrap_or_else(|| (session.core.player_map_id_like_cpp(), 0))
}
