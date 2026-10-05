// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the player query handler context (#1263 F5).
//!
//! The application crate owns the handlers and their context; the session only
//! lends its hub, so no session reference crosses into the handler.

use wow_world_application::{PlayerHandlerCxLikeCpp, PlayerHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl PlayerHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn player_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> PlayerHandlerCxLikeCpp<'a> {
        let (quest_state, inventory, lifecycle, visibility, instances, hub) =
            crate::session::split_player_handler_states_mut(self);
        PlayerHandlerCxLikeCpp::new(
            hub,
            quest_state,
            inventory,
            lifecycle,
            visibility,
            instances,
        )
    }

    fn force_update_visibility_after_far_sight_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> wow_handler::HandlerFuture<'a, ()> {
        Box::pin(async move {
            self.force_update_visibility_with_catalogs_like_cpp(catalogs.creature_spawns.as_ref())
                .await;
        })
    }
}
