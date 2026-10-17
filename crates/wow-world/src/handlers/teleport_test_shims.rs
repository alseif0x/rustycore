// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the teleport bodies moved to
//! `wow_world_entities::teleport_session` (#1363 slice 11a): one-call method
//! forwards to the moved free functions the scenarios call, with this session
//! as the host, so no scenario call site changes; the two `#[cfg(test)]`
//! readers that were nested in the production impl of
//! `session/movement/transfer.rs`, moved verbatim; and the #[path] mount of
//! the far-transfer F3 shim, which left the deleted
//! `session/movement/far_transfer.rs`.

#![cfg(test)]

use crate::session::*;
use wow_core::ObjectGuid;
use wow_entities::PlayerResurrectionRequestLikeCpp;

#[path = "../../unit_tests/session/movement/far_transfer/f3_shims.rs"]
mod far_transfer_f3_shims;

impl WorldSession {
    pub(crate) fn pending_teleport_save_destination_like_cpp(
        &self,
    ) -> Option<(u16, wow_core::Position)> {
        wow_world_entities::pending_teleport_save_destination_like_cpp(self)
    }
    pub(crate) async fn teleport_to_with_options(
        &mut self,
        new_map: u32,
        new_pos: wow_core::Position,
        options: wow_world_core::session::movement_protocol::TeleportToOptionsLikeCpp,
    ) {
        wow_world_entities::teleport_to_with_options(self, new_map, new_pos, options).await
    }
    pub(crate) async fn process_represented_delayed_teleport_after_update_like_cpp(
        &mut self,
    ) -> bool {
        wow_world_entities::process_represented_delayed_teleport_after_update_like_cpp(self).await
    }
    pub(crate) fn schedule_represented_resurrection_after_teleport_like_cpp(
        &mut self,
        request: PlayerResurrectionRequestLikeCpp,
    ) -> bool {
        wow_world_entities::schedule_represented_resurrection_after_teleport_like_cpp(self, request)
    }
    pub(crate) fn represented_delayed_resurrection_after_teleport_like_cpp(
        &self,
    ) -> Option<PlayerResurrectionRequestLikeCpp> {
        self.player_resurrection_state_snapshot_like_cpp()
            .and_then(|state| state.delayed_after_teleport)
    }
    pub(crate) fn handle_move_teleport_ack_like_cpp(
        &mut self,
        mover_guid: ObjectGuid,
        ack_index: i32,
        move_time: i32,
    ) -> MoveTeleportAckActionLikeCpp {
        wow_world_entities::handle_move_teleport_ack_like_cpp(
            self, mover_guid, ack_index, move_time,
        )
    }
    pub(crate) fn move_teleport_ack_events_like_cpp(&self) -> &[MoveTeleportAckEventLikeCpp] {
        &self.fixtures.teleport.move_teleport_ack_events_like_cpp
    }
}
