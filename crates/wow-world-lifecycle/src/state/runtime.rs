use tracing::warn;
use wow_world_core::session::{HubMut, PacketSpoofPendingBanTargetLikeCpp};

use super::SessionLifecycleState;

const PACKET_SPOOF_BAN_REASON_LIKE_CPP: &str = "DOS (Packet Flooding/Spoofing";
const PACKET_SPOOF_BAN_AUTHOR_LIKE_CPP: &str = "Server: AutoDOS";

impl SessionLifecycleState {
    /// Restore the realm socket as primary, and clear the login-loading state
    /// the kernel does not own.
    pub fn restore_realm_channels(&mut self, hub: &mut HubMut<'_>) {
        hub.core
            .transport
            .connection
            .restore_realm_channels(hub.core.account_id);
        self.player_loading = None;
    }

    pub fn tutorial_flags_packet_like_cpp(&self) -> wow_packet::packets::misc::TutorialFlags {
        wow_packet::packets::misc::TutorialFlags {
            tutorial_data: self.tutorials_like_cpp,
        }
    }

    /// C++ `WorldSession::Update` logout decision, on the `ProcessUnsafe()`
    /// branch reserved for the world filter (`WorldSession.cpp:505-511`).
    pub fn run_logout_timer_like_cpp(&mut self, hub: &mut HubMut<'_>) {
        let Some(logout_time) = self.logout_time else {
            return;
        };
        hub.core.record_driver_phase_like_cpp(
            wow_world_core::session::SessionDriverPhaseLikeCpp::LogoutTimer,
        );
        if std::time::Instant::now() >= logout_time {
            self.logout_time = None;
            self.complete_logout(hub);
        }
    }

    pub fn submit_character_rename_like_cpp(
        &mut self,
        port: std::sync::Arc<dyn wow_persistence::CharacterAdministrationPersistencePortLikeCpp>,
        guid: wow_core::ObjectGuid,
        name: String,
    ) -> bool {
        self.character_rename_callbacks.submit(port, guid, name)
    }

    pub async fn flush_packet_spoof_ban_like_cpp(&mut self, hub: &mut HubMut<'_>) {
        let Some(plan) = hub.core.admission.pending_packet_spoof_ban_like_cpp.take() else {
            return;
        };
        let Some(port) = self
            .persistence_ports_like_cpp
            .admission
            .packet_spoof_ban
            .clone()
        else {
            warn!(
                account = hub.core.account_id,
                "AntiDOS: PacketSpoof ban requested but login DB is unavailable"
            );
            hub.core.admission.pending_packet_spoof_ban_like_cpp = Some(plan);
            return;
        };

        let affected_account_ids = hub
            .core
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
                hub.core
                    .kick_packet_spoof_affected_sessions_like_cpp(&affected_account_ids);
            }
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
            | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!(
                    account = hub.core.account_id,
                    error = %reason,
                    "AntiDOS: failed to persist PacketSpoof ban"
                );
                hub.core.admission.pending_packet_spoof_ban_like_cpp = Some(plan);
            }
        }
    }
}
