// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1363 slice 1: production-dead `player` entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) async fn handle_set_difficulty_id(&mut self, pkt: wow_packet::WorldPacket) {
        let mut cx = self.build_instance_difficulty_handler_cx_like_cpp();
        wow_world_instances::handle_set_difficulty_id_like_cpp(&mut cx, pkt).await;
    }
    pub(crate) async fn handle_toggle_difficulty(&mut self, pkt: wow_packet::WorldPacket) {
        let mut cx = self.build_instance_difficulty_handler_cx_like_cpp();
        wow_world_instances::handle_toggle_difficulty_like_cpp(&mut cx, pkt).await;
    }
    pub(crate) async fn handle_set_dungeon_difficulty(&mut self, pkt: wow_packet::WorldPacket) {
        let mut cx = self.build_instance_difficulty_handler_cx_like_cpp();
        wow_world_instances::handle_set_dungeon_difficulty_like_cpp(&mut cx, pkt).await;
    }
    pub(crate) async fn handle_set_raid_difficulty(&mut self, pkt: wow_packet::WorldPacket) {
        let mut cx = self.build_instance_difficulty_handler_cx_like_cpp();
        wow_world_instances::handle_set_raid_difficulty_like_cpp(&mut cx, pkt).await;
    }
}
