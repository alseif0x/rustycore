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
//! are `session/travel_handler_contexts.rs` and `handlers/leaf_session_hosts.rs`.

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
        player_grid_loader: &wow_world_core::session::map_admission::PlayerGridLoadResolverLikeCpp,
        trait_node_entries: &wow_data::trait_tree::TraitNodeEntryStore,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        pkt: wow_packet::WorldPacket,
    ) {
        wow_world_character::handle_world_port_response_with_catalogs_like_cpp(
            self,
            creature_spawn_catalogs,
            player_grid_loader,
            None,
            trait_node_entries,
            item_guid_generator,
            pkt,
        )
        .await;
    }

    #[cfg(test)]
    pub(crate) fn worldport_catalogs_for_test(
        &self,
    ) -> crate::session::SessionHandlerCatalogsLikeCpp {
        let manager = self.core.canonical_map_manager.clone();
        let mut catalogs = self.session_handler_catalogs_for_test_like_cpp();
        // These fixtures install their objects directly on the admitted map.
        catalogs.player_grid_loader = std::sync::Arc::new(move |map, instance, _| {
            let available = manager
                .as_ref()
                .and_then(|m| m.lock().ok())
                .is_some_and(|m| {
                    instance.is_some_and(|id| m.find_map(u32::from(map), id).is_some())
                });
            crate::session::PlayerGridLoadOutcomeLikeCpp {
                map_unavailable: !available,
                ..Default::default()
            }
        });
        catalogs
    }

    #[cfg(test)]
    pub(crate) async fn handle_world_port_response(&mut self, pkt: wow_packet::WorldPacket) {
        let catalogs = self.worldport_catalogs_for_test();
        self.handle_world_port_response_with_catalogs_like_cpp(
            &catalogs.creature_spawns,
            &catalogs.player_grid_loader,
            &wow_data::trait_tree::TraitNodeEntryStore::from_entries([]),
            catalogs.id_generators.item.as_ref(),
            pkt,
        )
        .await;
    }
}
