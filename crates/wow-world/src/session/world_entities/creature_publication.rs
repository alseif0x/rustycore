//! Values updates and packets published for represented creatures.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

pub(crate) use wow_world_entities::represented_creature_aura_info_like_cpp;

impl WorldSession {
    pub(crate) fn broadcast_player_packet_to_visible_set_realm_like_cpp(&self, bytes: Vec<u8>) {
        let (state, hub) = crate::session::split_world_entities_ref(self);
        state.broadcast_player_packet_to_visible_set_realm_like_cpp(hub, bytes)
    }
}
