// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private travel capability handlers extracted from the legacy misc owner.
//!
//! `#1263 F5 remaining families`: the three registrations that lived here
//! (`CMSG_ACTIVATE_TAXI`, `CMSG_AREA_TRIGGER`, `CMSG_WORLD_PORT_RESPONSE`)
//! moved unchanged to the application crate's `crate::travel_handlers` area
//! registrar. `crates/wow-world/src/session/travel_handler_contexts.rs` lends
//! that registrar the same `WorldSession` operations and catalog destructuring
//! the legacy closures used.
//!
//! `#1363 slice 4d`: the area-trigger body with its client conditions moved
//! to `wow_world_instances` with the taxi activation; their World adapters
//! are `session/travel_handler_contexts.rs` and `handlers/travel_host.rs`.

#[cfg(test)]
mod test_shims;

impl crate::session::WorldSession {
    /// CMSG_WORLD_PORT_RESPONSE — client confirms it has loaded the new map.
    /// The body (C++ `WorldSession::HandleMoveWorldportAck`) moved to
    /// `wow_world_character` (#1363 slice 13a); the integration tests call it
    /// as a session method.
    pub async fn handle_world_port_response_with_catalogs_like_cpp(
        &mut self,
        creature_spawn_catalogs: &crate::session::CreatureSpawnCatalogsLikeCpp,
        trait_node_entries: &wow_data::trait_tree::TraitNodeEntryStore,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        pkt: wow_packet::WorldPacket,
    ) {
        wow_world_character::handle_world_port_response_with_catalogs_like_cpp(
            self,
            creature_spawn_catalogs,
            trait_node_entries,
            item_guid_generator,
            pkt,
        )
        .await;
    }

    #[cfg(test)]
    pub(crate) async fn handle_world_port_response(&mut self, pkt: wow_packet::WorldPacket) {
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        let generators = self.id_generators_for_test_like_cpp();
        self.handle_world_port_response_with_catalogs_like_cpp(
            &catalogs,
            &wow_data::trait_tree::TraitNodeEntryStore::from_entries([]),
            generators.item.as_ref(),
            pkt,
        )
        .await;
    }
}
