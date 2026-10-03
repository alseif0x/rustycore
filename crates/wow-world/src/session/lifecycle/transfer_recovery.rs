//! Bounded, Player-owned homebind recovery. No retry counter or owner mirror in Session.
//! The terminal source-save policy is an explicitly approved legacy departure.
use crate::session::WorldSession;
use wow_entities::PlayerTransferRecovery;

impl WorldSession {
    pub(crate) fn recovery_worldport_ack_ready_like_cpp(&self) -> bool {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.recovery_worldport_ack_ready_like_cpp(hub)
    }

    pub(crate) async fn recover_rejected_worldport_like_cpp(&mut self) {
        let Some(state) = self
            .core
            .with_owned_player_like_cpp(|player| *player.teleport_state_like_cpp())
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
        if !crate::session::hub_mut(self).update_player_teleport_state_like_cpp(|state| {
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

    pub(in crate::session) fn terminate_worldport_recovery_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.terminate_worldport_recovery_like_cpp(&mut hub)
    }
}
