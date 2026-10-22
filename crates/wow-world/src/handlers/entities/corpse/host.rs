// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side adapter for the application corpse registrar (#1263 F5 deferred
//! families).
//!
//! The four `MiscHandler.cpp` death/corpse opcodes left the legacy inventory
//! path for `wow_world_entities::register_corpse_handlers_like_cpp`. This
//! adapter lends that registrar the same `WorldSession` operations with the
//! same catalog destructuring the legacy closures used; the alive/ghost state,
//! the resurrection request, the teleport/transfer path and every publication
//! stay in those operations.

use wow_handler::HandlerFuture;
use wow_packet::WorldPacket;
use wow_world_entities::CorpseHandlerHostLikeCpp;

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl CorpseHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn handle_resurrect_response_like_cpp<'a>(
        &'a mut self,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move { wow_world_entities::handle_resurrect_response(self, pkt).await })
    }

    fn handle_repop_request_with_catalogs_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            wow_world_entities::handle_repop_request(
                self,
                corpse_lifecycle_catalogs_like_cpp(catalogs),
                pkt,
            )
            .await
        })
    }

    fn handle_reclaim_corpse_like_cpp<'a>(&'a mut self, pkt: WorldPacket) -> HandlerFuture<'a, ()> {
        Box::pin(async move { wow_world_entities::handle_reclaim_corpse(self, pkt).await })
    }

    fn handle_request_cemetery_list_with_catalogs_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            wow_world_entities::handle_request_cemetery_list_with_catalog_like_cpp(
                self,
                catalogs.graveyards.as_ref(),
                pkt,
            )
            .await
        })
    }
}

/// The catalogs the death/corpse lifecycle borrows from the handler view.
pub(crate) fn corpse_lifecycle_catalogs_like_cpp(
    catalogs: &SessionHandlerCatalogsLikeCpp,
) -> wow_world_entities::CorpseLifecycleCatalogsLikeCpp<'_> {
    wow_world_entities::CorpseLifecycleCatalogsLikeCpp {
        item_guid_generator: catalogs.id_generators.item.as_ref(),
        creature_spawns: catalogs.creature_spawns.as_ref(),
        graveyards: catalogs.graveyards.as_ref(),
    }
}

impl wow_world_entities::CorpseHostLikeCpp for WorldSession {
    fn teleport_to_with_options_like_cpp(
        &mut self,
        new_map: u32,
        new_pos: wow_core::Position,
        options: wow_world_core::session::movement_protocol::TeleportToOptionsLikeCpp,
    ) -> impl std::future::Future<Output = ()> + Send {
        wow_world_entities::teleport_to_with_options(self, new_map, new_pos, options)
    }

    fn pending_teleport_like_cpp(&self) -> Option<(u32, wow_core::Position)> {
        self.lifecycle
            .pending_teleport_like_cpp(crate::session::hub_ref(self))
    }

    fn map_corpse_persistence_port_like_cpp(
        &self,
    ) -> Option<std::sync::Arc<dyn wow_persistence::MapCorpsePersistencePortLikeCpp>> {
        self.lifecycle
            .map_corpse_persistence_port_like_cpp()
            .map(std::sync::Arc::clone)
    }

    fn player_has_spell_like_cpp(&self, spell_id: i32) -> bool {
        self.core
            .with_owned_player_like_cpp(|player| {
                player
                    .spell_runtime_like_cpp()
                    .known_spells_like_cpp()
                    .contains(&spell_id)
            })
            .unwrap_or(false)
    }

    fn cast_triggered_self_spell_like_cpp(
        &mut self,
        catalogs: wow_world_entities::CorpseLifecycleCatalogsLikeCpp<'_>,
        spell_id: i32,
    ) -> impl std::future::Future<Output = bool> + Send {
        async move {
            let Some(player_guid) = self.player_guid() else {
                return false;
            };
            // C++ `CastSpell(this, spellId, true)` is TRIGGERED_FULL_MASK; the
            // non-client triggered cast carries CAST_FLAG_PENDING.
            wow_world_spell::execute_server_triggered_spell_like_cpp(
                self,
                catalogs.item_guid_generator,
                catalogs.creature_spawns,
                spell_id,
                player_guid,
                wow_entities::SpellCastMetadata {
                    cast_flags: 0x0000_0001,
                    triggered_ignores_global_cooldown_like_cpp: true,
                    ..wow_entities::SpellCastMetadata::default()
                },
            )
            .await
            .is_ok()
        }
    }

    fn player_logout_with_save_like_cpp(&self) -> bool {
        // Rust's logout finalization always runs the Player save, so a
        // logging-out Player is C++ `m_playerLogout && m_playerSave`.
        self.lifecycle.player_logout_like_cpp()
    }

    fn save_player_after_corpse_bones_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
    ) -> impl std::future::Future<Output = ()> + Send {
        async move {
            wow_world_lifecycle::save_current_player_to_db_with_generator_like_cpp(
                self,
                item_guid_generator,
            )
            .await;
        }
    }

    fn graveyard_conditions_meet_like_cpp(
        &mut self,
        conditions_ref: &wow_data::ConditionsReference,
    ) -> bool {
        WorldSession::graveyard_conditions_meet_like_cpp(self, conditions_ref)
    }

    #[cfg(test)]
    fn represented_resurrection_requested_by_like_cpp(
        &self,
        resurrecter: wow_core::ObjectGuid,
    ) -> bool {
        WorldSession::represented_resurrection_requested_by_like_cpp(self, resurrecter)
    }

    fn schedule_represented_resurrection_after_teleport_like_cpp(
        &mut self,
        request: wow_entities::PlayerResurrectionRequestLikeCpp,
    ) -> bool {
        wow_world_entities::schedule_represented_resurrection_after_teleport_like_cpp(self, request)
    }

    fn teleport_to(
        &mut self,
        new_map: u32,
        new_pos: wow_core::Position,
    ) -> impl std::future::Future<Output = ()> + Send {
        WorldSession::teleport_to(self, new_map, new_pos)
    }
}
