// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[allow(dead_code)]
    pub(crate) fn send_pending_raid_lock_like_cpp(
        &mut self,
        instance_id: u32,
        completed_mask: u32,
        extending: bool,
        warning_only: bool,
    ) {
        let (state, mut hub) = crate::session::split_instances_mut(self);
        state.send_pending_raid_lock_like_cpp(
            &mut hub,
            instance_id,
            completed_mask,
            extending,
            warning_only,
        )
    }
}

impl crate::session::WorldSession {
    pub(crate) async fn handle_request_raid_info(&mut self, pkt: wow_packet::WorldPacket) {
        let mut cx = self.build_instance_raid_info_handler_cx_like_cpp();
        wow_world_instances::handle_request_raid_info_like_cpp(&mut cx, pkt).await;
    }
    pub(crate) async fn handle_reset_instances(&mut self, pkt: wow_packet::WorldPacket) {
        let mut cx = self.build_instance_lock_operations_handler_cx_like_cpp();
        wow_world_instances::handle_reset_instances_like_cpp(&mut cx, pkt).await;
    }
    pub(crate) async fn handle_instance_lock_response(&mut self, pkt: wow_packet::WorldPacket) {
        let mut cx = self.build_instance_lock_operations_handler_cx_like_cpp();
        let _ = wow_world_instances::handle_instance_lock_response_like_cpp(&mut cx, pkt).await;
    }
    /// The opcode remains in the legacy Loot registry; only its operation body
    /// is owned by the application crate.
    pub(crate) async fn handle_set_saved_instance_extend(&mut self, query: SetSavedInstanceExtend) {
        let mut cx = self.build_instance_lock_operations_handler_cx_like_cpp();
        wow_world_instances::handle_set_saved_instance_extend_like_cpp(&mut cx, query).await;
    }
}
