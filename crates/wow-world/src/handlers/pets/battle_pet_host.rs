// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the battle-pet handler context (#1263 F5).
//!
//! The application crate owns the eight C++ `BattlePetHandler.cpp` handlers and
//! their context; the session only lends its lifecycle state and hub, so no
//! session reference crosses into the handler.
//!
//! `#1263 F5 remaining families`: the host also lends `CMSG_DISMISS_CRITTER`
//! (`PetHandler.cpp:41`), whose represented body stays in this `handlers/pets/`
//! module.

use wow_handler::HandlerFuture;
use wow_packet::WorldPacket;
use wow_world_application::{BattlePetHandlerCxLikeCpp, BattlePetHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl BattlePetHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn battle_pet_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> BattlePetHandlerCxLikeCpp<'a> {
        let (lifecycle, hub) = crate::session::split_battle_pet_handler_mut(self);
        BattlePetHandlerCxLikeCpp::new(hub, lifecycle, cfg!(test))
    }

    fn handle_dismiss_critter<'a>(&'a mut self, pkt: WorldPacket) -> HandlerFuture<'a, ()> {
        Box::pin(async move { WorldSession::handle_dismiss_critter(self, pkt).await })
    }
}
