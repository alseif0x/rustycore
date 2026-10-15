// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the group bodies moved to
//! `wow_world_instances::group_session` (#1363 slice 4d): one-call forwards
//! with this session as the host, so no scenario call site changes.

#![cfg(test)]

use wow_world_instances::GroupPublicationTailLikeCpp;

use crate::session::WorldSession;

impl WorldSession {
    pub(crate) fn reconcile_group_state_like_cpp(&mut self) -> bool {
        wow_world_instances::reconcile_group_state_like_cpp(self)
    }

    pub(crate) async fn apply_group_publication_tail_like_cpp(
        &mut self,
        tail: GroupPublicationTailLikeCpp,
    ) {
        wow_world_instances::apply_group_publication_tail_like_cpp(self, tail).await
    }
}
