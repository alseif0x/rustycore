// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the NPC handler context (#1263 F5).
//!
//! The interaction crate owns the C++ `NPCHandler.cpp` handlers and their
//! context; the session only lends its hub, so no session reference crosses
//! into the handler.

use wow_handler::HandlerFuture;
use wow_world_interaction::{NpcHandlerCxLikeCpp, NpcHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl NpcHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn npc_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> NpcHandlerCxLikeCpp<'a> {
        NpcHandlerCxLikeCpp::new(crate::session::hub_mut(self))
    }

    fn send_spirit_resurrect<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            self.player_aura_application_cx_like_cpp()
                .remove_represented_feign_death_if_needed_like_cpp();
            let corpse_catalogs = wow_world_entities::CorpseLifecycleCatalogsLikeCpp {
                item_guid_generator: catalogs.id_generators.item.as_ref(),
                creature_spawns: catalogs.creature_spawns.as_ref(),
                graveyards: catalogs.graveyards.as_ref(),
            };
            wow_world_entities::send_spirit_resurrect(self, corpse_catalogs).await;
        })
    }
}
