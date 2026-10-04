// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::{PacketSpoofConfigLikeCpp, SystemTime, UNIX_EPOCH, WorldPacket, warn};

impl super::WorldSession {
    fn packet_spoof_now_secs_like_cpp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    pub(super) fn evaluate_packet_spoof_like_cpp(&mut self, pkt: &WorldPacket) -> bool {
        let Some(opcode) = pkt.client_opcode() else {
            return true;
        };
        let max_packet_counter_allowed =
            wow_world_core::session::packet_spoof_max_packet_counter_allowed_like_cpp(opcode);
        if max_packet_counter_allowed == 0 {
            return true;
        }

        let now = Self::packet_spoof_now_secs_like_cpp();
        let counter = self
            .core
            .admission
            .packet_counter_like_cpp(pkt.opcode_raw());
        if counter.last_receive_time_secs != now {
            counter.last_receive_time_secs = now;
            counter.amount_counter = 0;
        }
        counter.amount_counter = counter.amount_counter.saturating_add(1);
        if counter.amount_counter <= max_packet_counter_allowed {
            return true;
        }
        let amount_counter = counter.amount_counter;

        warn!(
            "AntiDOS: Account {}, Character: {:?}, flooding packet (opc: {:?} (0x{:X}), count: {})",
            self.core.account_id,
            crate::session::hub_ref(self).player_name_like_cpp(),
            opcode,
            pkt.opcode_raw(),
            amount_counter
        );

        match self.core.admission.packet_spoof_config_like_cpp.policy {
            PacketSpoofConfigLikeCpp::POLICY_LOG => true,
            PacketSpoofConfigLikeCpp::POLICY_KICK => {
                self.kick("WorldSession::DosProtection::EvaluateOpcode AntiDOS");
                false
            }
            PacketSpoofConfigLikeCpp::POLICY_BAN => {
                self.core.stage_packet_spoof_ban_like_cpp();
                self.kick("WorldSession::DosProtection::EvaluateOpcode AntiDOS");
                false
            }
            _ => true,
        }
    }

    pub(super) async fn flush_packet_spoof_ban_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.flush_packet_spoof_ban_like_cpp(&mut hub).await
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/admission/f3_shims.rs"]
mod f3_shims;
