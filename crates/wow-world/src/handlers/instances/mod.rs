// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Thin compatibility entry points for the application-owned instance handlers.

pub(crate) use wow_world_application::InstanceResetMethodLikeCpp as RepresentedInstanceResetMethodLikeCpp;
use wow_packet::packets::misc::SetSavedInstanceExtend;

#[cfg(test)]
use tracing::{info, warn};
#[cfg(test)]
use wow_constants::ClientOpcodes;
#[cfg(test)]
use wow_core::ObjectGuid;
#[cfg(test)]
use wow_handler::{PacketProcessing, SessionStatus};
#[cfg(test)]
use wow_packet::ClientPacket;
#[cfg(test)]
use wow_packet::packets::instance::{
    InstanceInfo, InstanceLockInfo, InstanceLockResponse, InstanceReset, InstanceResetFailed,
    InstanceSaveCreated, PendingRaidLock,
};
#[cfg(test)]
use wow_packet::packets::misc::{CalendarRaidLockoutAdded, CalendarRaidLockoutUpdated};
#[cfg(test)]
use wow_persistence::{
    InstanceLockPersistenceOutcomeLikeCpp, InstanceLockPersistencePlanLikeCpp,
};
#[cfg(test)]
use crate::session::registry::PacketHandlerEntry;

impl crate::session::WorldSession {
    pub async fn handle_request_raid_info(&mut self, pkt: wow_packet::WorldPacket) {
        let mut cx = self.build_instance_raid_info_handler_cx_like_cpp();
        wow_world_application::handle_request_raid_info_like_cpp(&mut cx, pkt).await;
    }

    pub async fn handle_reset_instances(&mut self, pkt: wow_packet::WorldPacket) {
        let mut cx = self.build_instance_lock_operations_handler_cx_like_cpp();
        wow_world_application::handle_reset_instances_like_cpp(&mut cx, pkt).await;
    }

    pub(crate) async fn reset_represented_instances_like_cpp(
        &mut self,
        reset_owner_guid: wow_core::ObjectGuid,
        method: RepresentedInstanceResetMethodLikeCpp,
    ) -> bool {
        let mut cx = self.build_instance_lock_operations_handler_cx_like_cpp();
        wow_world_application::reset_represented_instances_like_cpp(
            &mut cx,
            reset_owner_guid,
            method,
        )
        .await
    }

    pub async fn handle_instance_lock_response(&mut self, pkt: wow_packet::WorldPacket) {
        let mut cx = self.build_instance_lock_operations_handler_cx_like_cpp();
        let _ = wow_world_application::handle_instance_lock_response_like_cpp(&mut cx, pkt).await;
    }

    /// The opcode remains in the legacy Loot registry; only its operation body
    /// is owned by the application crate.
    pub async fn handle_set_saved_instance_extend(&mut self, query: SetSavedInstanceExtend) {
        let mut cx = self.build_instance_lock_operations_handler_cx_like_cpp();
        wow_world_application::handle_set_saved_instance_extend_like_cpp(&mut cx, query).await;
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/handlers/instances/tests/mod.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../unit_tests/handlers/instances/mod/f3_shims.rs"]
mod f3_shims;
