// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the spell handler context (#1263 F5).
//!
//! The application crate owns the bodies and their context; the session only
//! lends its hub and the aura-application participants (spell, inventory,
//! loot and quest state), so no session reference crosses into the handler.
//!
//! `#1263 F5 remaining families`: the adapter also lends the four
//! `SpellHandler.cpp` entries (`CMSG_CAST_SPELL`, `CMSG_OPEN_ITEM`,
//! `CMSG_SELF_RES`, `CMSG_SPELL_CLICK`) that moved off the legacy inventory
//! path in `crates/wow-world/src/handlers/spell.rs`, delegating to the same
//! `WorldSession` operations with the same catalog destructuring the legacy
//! closures used.

use wow_handler::HandlerFuture;
use wow_packet::WorldPacket;
use wow_world_application::{SpellHandlerCxLikeCpp, SpellHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl SpellHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn spell_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> SpellHandlerCxLikeCpp<'a> {
        let (spell_state, inventory, loot, quest_state, hub) =
            crate::session::state::split_aura_application_mut(self);
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = quest_state;
        SpellHandlerCxLikeCpp::new(
            hub,
            spell_state,
            inventory,
            loot,
            #[cfg(any(test, feature = "test-fixtures"))]
            quest_state,
            cfg!(test),
        )
    }

    fn handle_cast_spell_with_catalogs_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_cast_spell_with_catalogs_like_cpp(
                self,
                catalogs.area_triggers.as_ref(),
                catalogs.creature_spawns.as_ref(),
                catalogs.progression.as_ref(),
                &catalogs.player_grid_loader,
                catalogs.id_generators.item.as_ref(),
                pkt,
            )
            .await
        })
    }

    fn handle_open_item<'a>(&'a mut self, pkt: WorldPacket) -> HandlerFuture<'a, ()> {
        Box::pin(async move { WorldSession::handle_open_item(self, pkt).await })
    }

    fn handle_self_res_with_generator_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_self_res_with_generator_like_cpp(
                self,
                catalogs.id_generators.item.as_ref(),
                catalogs.creature_spawns.as_ref(),
                pkt,
            )
            .await
        })
    }

    fn handle_spell_click_with_generator_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_spell_click_with_generator_like_cpp(
                self,
                catalogs.id_generators.item.as_ref(),
                catalogs.creature_spawns.as_ref(),
                pkt,
            )
            .await
        })
    }
}
