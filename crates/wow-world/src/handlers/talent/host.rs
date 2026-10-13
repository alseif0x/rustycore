// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side adapter for the application talent registrar (#1263 F5 deferred
//! families).
//!
//! `CMSG_CONFIRM_RESPEC_WIPE` and `CMSG_LEARN_TALENT` left the legacy inventory
//! path for `wow_world_application::register_talent_handlers_like_cpp`. This
//! adapter lends that registrar the same `WorldSession` operations with the
//! same catalog destructuring the legacy closures used; the talent-reset
//! commit and its publication stay in those operations. The CMSG_LEARN_TALENT
//! body lives in `wow_world_spell` (#1363 slice 9b) with this session as host.

use wow_handler::HandlerFuture;
use wow_packet::WorldPacket;
use wow_world_application::TalentHandlerHostLikeCpp;

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl TalentHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn handle_confirm_respec_wipe_with_catalogs_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            self.handle_confirm_respec_wipe_with_generator_like_cpp(
                catalogs.id_generators.item.as_ref(),
                catalogs.progression.no_reset_talent_cost,
                pkt,
            )
            .await
        })
    }

    fn handle_learn_talent_with_catalogs_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            wow_world_spell::handle_learn_talent(
                self,
                catalogs.player_bootstrap.talent_tabs.as_ref(),
                pkt,
            )
            .await
        })
    }
}
