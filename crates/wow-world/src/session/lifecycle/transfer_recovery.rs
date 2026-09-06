//! Bounded, Player-owned homebind recovery. No retry counter or owner mirror in Session.
//! The terminal source-save policy is an explicitly approved legacy departure.
use crate::session::WorldSession;
use wow_entities::PlayerTransferRecovery;

impl WorldSession {
    pub(crate) fn recovery_worldport_ack_ready_like_cpp(&self) -> bool {
        self.with_owned_player_like_cpp(|player| {
            matches!(
                player.teleport_state_like_cpp().recovery,
                PlayerTransferRecovery::None | PlayerTransferRecovery::HomebindWorldportReady
            )
        })
        .unwrap_or(false)
    }

    pub(crate) fn recovery_new_world_sent_like_cpp(&mut self) {
        let _ = self.with_owned_player_mut_like_cpp(|player| {
            let state = player.teleport_state_mut_like_cpp();
            if state.recovery == PlayerTransferRecovery::Homebind {
                state.recovery = PlayerTransferRecovery::HomebindWorldportReady;
            }
        });
    }

    pub(crate) async fn recover_rejected_worldport_like_cpp(&mut self) {
        let Some(state) =
            self.with_owned_player_like_cpp(|player| *player.teleport_state_like_cpp())
        else {
            self.kick("worldport recovery has no Player owner");
            return;
        };
        if state.recovery != PlayerTransferRecovery::None {
            self.terminate_worldport_recovery_like_cpp();
            return;
        }
        // A delayed Player update is not a failed recovery attempt. The normal
        // Session driver resets can_delay before admitting an ACK.
        if state.can_delay {
            return;
        }
        let Some(homebind) = self.represented_homebind_like_cpp() else {
            self.terminate_worldport_recovery_like_cpp();
            return;
        };
        let destination = (homebind.map_id, homebind.position);
        if self.pending_teleport_like_cpp() == Some(destination) {
            // Already rejected this exact destination; do not replay it forever.
            self.terminate_worldport_recovery_like_cpp();
            return;
        }
        if !self.update_player_teleport_state_like_cpp(|state| {
            state.recovery = PlayerTransferRecovery::Homebind;
        }) {
            self.kick("worldport recovery lost its Player owner");
            return;
        }
        self.teleport_to(homebind.map_id, homebind.position).await;
        if self.pending_teleport_like_cpp() != Some(destination) {
            self.terminate_worldport_recovery_like_cpp();
        }
    }

    fn terminate_worldport_recovery_like_cpp(&mut self) {
        let _ = self.update_player_teleport_state_like_cpp(|state| {
            state.recovery = PlayerTransferRecovery::Terminal;
            // Cancel stale near/delayed commands, not the unresolved far transfer.
            state.near_pending = false;
            state.near_destination = None;
            state.near_destination_zone_area = None;
            state.has_delayed = false;
            state.delayed = None;
        });
        self.kick("worldport and homebind recovery failed; disconnect at retained source");
    }
}
