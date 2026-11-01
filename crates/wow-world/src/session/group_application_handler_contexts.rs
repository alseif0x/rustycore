// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the cross-domain group handler context (#1263 F5).
//!
//! The instances crate owns the handlers and their context; the session only
//! splits its social and lifecycle state from the hub. The publication tail
//! moved to `wow_world_instances::group_session` (#1363 slice 4d).

use wow_world_instances::{
    GroupHandlerCxLikeCpp, GroupHandlerHostLikeCpp, GroupPublicationTailLikeCpp,
};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl GroupHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn group_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> GroupHandlerCxLikeCpp<'a> {
        let (social, lifecycle, loot, instances, hub) =
            crate::session::split_group_handler_states_mut(self);
        GroupHandlerCxLikeCpp::new(
            social,
            lifecycle,
            loot,
            instances,
            catalogs.group_invite_policy.as_ref(),
            hub,
            cfg!(test),
        )
    }

    fn run_group_publication_tail_like_cpp<'a>(
        &'a mut self,
        tail: GroupPublicationTailLikeCpp,
    ) -> wow_handler::HandlerFuture<'a, ()> {
        Box::pin(async move {
            wow_world_instances::apply_group_publication_tail_like_cpp(self, tail).await;
        })
    }
}
