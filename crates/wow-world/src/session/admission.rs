// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

mod ingress;

pub(super) use ingress::IngressObservation;
use ingress::FloodAction;

use super::{
    KickLikeCppCommand, PACKET_SPOOF_BAN_AUTHOR_LIKE_CPP, PACKET_SPOOF_BAN_REASON_LIKE_CPP,
    PacketSpoofConfigLikeCpp, PacketSpoofPendingBanLikeCpp, PacketSpoofPendingBanTargetLikeCpp,
    SessionCommand, WorldPacket, warn,
};
use std::vec::Vec;

impl super::WorldSession {
    pub(crate) fn reset_timeout_time_like_cpp(&mut self, only_active: bool) {
        self.admission.reset_timeout(&self.state, only_active);
    }

    pub(crate) fn is_connection_idle_like_cpp(&self) -> bool {
        self.admission.is_idle()
    }

    pub(super) fn apply_packet_ingress_observation(
        &mut self,
        pkt: &WorldPacket,
        observation: IngressObservation,
    ) -> bool {
        let IngressObservation::Flood { opcode, count } = observation else {
            return true;
        };

        warn!(
            "AntiDOS: Account {}, Character: {:?}, flooding packet (opc: {:?} (0x{:X}), count: {})",
            self.account_id,
            self.player_name_like_cpp(),
            opcode,
            pkt.opcode_raw(),
            count
        );

        match self.admission.flood_action() {
            FloodAction::Allow => true,
            FloodAction::Kick => {
                self.kick("WorldSession::DosProtection::EvaluateOpcode AntiDOS");
                false
            }
            FloodAction::Ban => {
                self.stage_packet_spoof_ban_like_cpp();
                self.kick("WorldSession::DosProtection::EvaluateOpcode AntiDOS");
                false
            }
        }
    }

    fn stage_packet_spoof_ban_like_cpp(&mut self) {
        let target = match self.admission.packet_spoof_config_like_cpp.ban_mode {
            PacketSpoofConfigLikeCpp::BAN_IP => {
                let Some(address) = self.transport.remote_address_like_cpp.clone() else {
                    warn!(
                        account = self.account_id,
                        "AntiDOS: PacketSpoof BAN_IP requested but remote address is unavailable; kicking without persistent IP ban"
                    );
                    return;
                };
                PacketSpoofPendingBanTargetLikeCpp::Ip { address }
            }
            _ => {
                // TrinityCore's AntiDOS path maps BAN_CHARACTER to account bans because
                // character-level packet spoof bans are not implemented there either.
                PacketSpoofPendingBanTargetLikeCpp::Account {
                    account_id: self.account_id,
                }
            }
        };

        self.admission.pending_packet_spoof_ban_like_cpp = Some(PacketSpoofPendingBanLikeCpp {
            target,
            duration_secs: self
                .admission
                .packet_spoof_config_like_cpp
                .ban_duration_secs,
        });
    }

    pub(super) async fn flush_packet_spoof_ban_like_cpp(&mut self) {
        let Some(plan) = self.admission.pending_packet_spoof_ban_like_cpp.take() else {
            return;
        };
        let Some(port) = self
            .lifecycle
            .persistence_ports_like_cpp
            .admission
            .packet_spoof_ban
            .clone()
        else {
            warn!(
                account = self.account_id,
                "AntiDOS: PacketSpoof ban requested but login DB is unavailable"
            );
            self.admission.pending_packet_spoof_ban_like_cpp = Some(plan);
            return;
        };

        let affected_account_ids = self
            .packet_spoof_ban_affected_account_ids_like_cpp(port.as_ref(), &plan)
            .await;
        let target = match &plan.target {
            PacketSpoofPendingBanTargetLikeCpp::Account { account_id } => {
                wow_persistence::PacketSpoofBanTargetLikeCpp::Account {
                    account_id: *account_id,
                }
            }
            PacketSpoofPendingBanTargetLikeCpp::Ip { address } => {
                wow_persistence::PacketSpoofBanTargetLikeCpp::Ip {
                    address: address.clone(),
                }
            }
        };
        let result = port
            .persist_packet_spoof_ban_like_cpp(wow_persistence::PacketSpoofBanWriteRequestLikeCpp {
                target,
                duration_secs: plan.duration_secs,
                author: PACKET_SPOOF_BAN_AUTHOR_LIKE_CPP.to_string(),
                reason: PACKET_SPOOF_BAN_REASON_LIKE_CPP.to_string(),
            })
            .await;

        match result {
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {
                self.kick_packet_spoof_affected_sessions_like_cpp(&affected_account_ids);
            }
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
            | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!(
                    account = self.account_id,
                    error = %reason,
                    "AntiDOS: failed to persist PacketSpoof ban"
                );
                self.admission.pending_packet_spoof_ban_like_cpp = Some(plan);
            }
        }
    }

    async fn packet_spoof_ban_affected_account_ids_like_cpp(
        &self,
        port: &dyn wow_persistence::PacketSpoofBanPersistencePortLikeCpp,
        plan: &PacketSpoofPendingBanLikeCpp,
    ) -> Vec<u32> {
        match &plan.target {
            PacketSpoofPendingBanTargetLikeCpp::Account { account_id } => vec![*account_id],
            PacketSpoofPendingBanTargetLikeCpp::Ip { address } => {
                match port.load_accounts_by_ip_like_cpp(address).await {
                    wow_persistence::PacketSpoofAffectedAccountsLoadOutcomeLikeCpp::Loaded(
                        account_ids,
                    ) => account_ids,
                    wow_persistence::PacketSpoofAffectedAccountsLoadOutcomeLikeCpp::Failed {
                        reason,
                    } => {
                        warn!(
                            account = self.account_id,
                            error = %reason,
                            ip = address,
                            "AntiDOS: failed to query accounts affected by PacketSpoof IP ban"
                        );
                        Vec::new()
                    }
                }
            }
        }
    }

    pub(super) fn kick_packet_spoof_affected_sessions_like_cpp(
        &self,
        affected_account_ids: &[u32],
    ) -> usize {
        if affected_account_ids.is_empty() {
            return 0;
        }
        let Some(registry) = self.player_registry() else {
            return 0;
        };

        let mut sent = 0usize;
        for (account_id, registration) in registry.registrations_for_accounts(affected_account_ids)
        {
            let command = SessionCommand::KickLikeCpp(KickLikeCppCommand {
                reason: "World::BanAccount Banning account".to_string(),
            });
            if let Err(error) = registry.try_send_current_command(registration, command) {
                warn!(
                    account = account_id,
                    ?error,
                    "AntiDOS: failed to queue PacketSpoof ban kick for affected session"
                );
                continue;
            }
            sent = sent.saturating_add(1);
        }
        sent
    }
}
