// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1363 slice 1: production-dead `ops_2` entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl WorldSession {
    /// Handle the represented `CMSG_CANCEL_MOD_SPEED_NO_CONTROL_AURAS`.
    ///
    /// The inspected opcode table assigns this packet to the shared unresolved
    /// `0xBADD` value, so `WorldSession` probes this handler from that branch
    /// and falls through to other 0xBADD packet shapes when the target does not
    /// match C++ `Player::GetUnitBeingMoved()`.
    pub(crate) async fn try_handle_cancel_mod_speed_no_control_auras_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
    ) -> bool {
        let request = match CancelModSpeedNoControlAuras::read(&mut pkt) {
            Ok(request) if pkt.is_empty() => request,
            _ => return false,
        };
        if crate::session::hub_ref(self).player_moved_unit_guid_like_cpp()
            != Some(request.target_guid)
        {
            return false;
        }

        self.remove_represented_mod_speed_no_control_auras_cancelable_like_cpp();
        true
    }
}
