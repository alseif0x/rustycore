//! Values updates and packets published for represented creatures.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

pub(crate) use wow_world_entities::represented_creature_aura_info_like_cpp;

impl WorldSession {
    /// Route a creature-originated packet through the existing
    /// `MessageDistDeliverer`-style candidate and per-session visibility gates.
    /// The activating session receives its direct packet separately; this
    /// queues only nearby observers, matching `WorldObject::SendMessageToSet`.
    pub(crate) fn broadcast_creature_packet_to_visible_set_like_cpp(
        &self,
        source_guid: ObjectGuid,
        bytes: Vec<u8>,
    ) {
        self.broadcast_creature_packet_to_visible_set_and_connection_like_cpp(
            source_guid,
            bytes,
            false,
        );
    }
    pub(crate) fn broadcast_creature_packet_to_visible_set_realm_like_cpp(
        &self,
        source_guid: ObjectGuid,
        bytes: Vec<u8>,
    ) {
        self.broadcast_creature_packet_to_visible_set_and_connection_like_cpp(
            source_guid,
            bytes,
            true,
        );
    }
    pub(crate) fn broadcast_player_packet_to_visible_set_realm_like_cpp(&self, bytes: Vec<u8>) {
        let (state, hub) = crate::session::split_world_entities_ref(self);
        state.broadcast_player_packet_to_visible_set_realm_like_cpp(hub, bytes)
    }
    fn broadcast_creature_packet_to_visible_set_and_connection_like_cpp(
        &self,
        source_guid: ObjectGuid,
        bytes: Vec<u8>,
        realm_connection: bool,
    ) {
        let Some(source) = self.canonical_creature_access_like_cpp(source_guid) else {
            return;
        };
        {
            let (s, h) = crate::session::split_world_entities_ref(self);
            s.broadcast_creature_packet_from_position_to_visible_set_and_connection_like_cpp(
                h,
                source_guid,
                source.position,
                bytes,
                realm_connection,
                false,
            )
        };
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/world_entities/creature_publication/f3_shims.rs"]
mod f3_shims;
