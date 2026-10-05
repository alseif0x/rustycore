use wow_core::Position;
use wow_entities::PlayerWorldportPostAddPhaseLikeCpp as Phase;
use wow_entities::{PlayerTransferRecovery, PlayerWorldportPostAddLikeCpp};
use wow_world_core::session::{HubMut, HubRef};

use super::SessionLifecycleState;

impl SessionLifecycleState {
    pub fn pending_teleport_like_cpp(&self, hub: HubRef<'_>) -> Option<(u32, Position)> {
        hub.player_teleport_state_snapshot_like_cpp()
            .and_then(|state| state.far_destination)
    }

    pub fn set_pending_teleport_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        destination: Option<(u32, Position)>,
    ) -> bool {
        // C++ Player.cpp:1456 constructs WorldLocation; Position.h:29 normalizes
        // orientation there, before the same destination is used for attachment.
        // Retaining raw orientation would disagree with canonical WorldObject
        // after relocation and falsely block native post-add/finalization.
        let destination = destination.map(|(map, pos)| {
            let location =
                wow_entities::WorldLocation::new(map, pos.x, pos.y, pos.z, pos.orientation);
            (map, location.position())
        });
        hub.update_player_teleport_state_like_cpp(|state| state.far_destination = destination)
    }

    pub fn begin_worldport_post_add_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        map_id: u32,
        position: Position,
    ) -> bool {
        let begun = hub
            .core
            .with_owned_player_mut_like_cpp(|player| {
                if player.unit().world().map_id() != map_id
                    || player.unit().world().position() != position
                {
                    return false;
                }
                let state = player.teleport_state_mut_like_cpp();
                if state.post_add.is_some() {
                    return false;
                }
                state.post_add = Some(PlayerWorldportPostAddLikeCpp {
                    map_id,
                    position,
                    phase: Phase::BeforeZone,
                });
                true
            })
            .unwrap_or(false);
        if begun && self.pending_periodic_player_save_like_cpp() {
            // The timer can expire before Transfer stops ordinary Session autosaves.
            // Give that due request the same native delayed-operation phase as a
            // direct SaveToDB call, before any following queued packet is admitted.
            if self.defer_player_save_for_transfer_like_cpp(hub)
                != Some(crate::PlayerSaveOutcomeLikeCpp::Deferred)
            {
                return false;
            }
            self.reset_player_save_timer_like_cpp();
        }
        begun
    }

    pub fn advance_worldport_post_add_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        phase: Phase,
    ) -> bool {
        hub.core
            .with_owned_player_mut_like_cpp(|player| {
                if let Some(progress) = player.teleport_state_like_cpp().post_add
                    && (player.unit().world().map_id() != progress.map_id
                        || player.unit().world().position() != progress.position)
                {
                    return false;
                }
                if let Some(progress) = &mut player.teleport_state_mut_like_cpp().post_add {
                    progress.phase = progress.phase.max(phase);
                }
                true
            })
            .unwrap_or(false)
    }

    pub fn recovery_worldport_ack_ready_like_cpp(&self, hub: HubRef<'_>) -> bool {
        hub.core
            .with_owned_player_like_cpp(|player| {
                matches!(
                    player.teleport_state_like_cpp().recovery,
                    PlayerTransferRecovery::None | PlayerTransferRecovery::HomebindWorldportReady
                )
            })
            .unwrap_or(false)
    }

    pub fn recovery_new_world_sent_like_cpp(&mut self, hub: &mut HubMut<'_>) {
        let _ = hub.core.with_owned_player_mut_like_cpp(|player| {
            let state = player.teleport_state_mut_like_cpp();
            if state.recovery == PlayerTransferRecovery::Homebind {
                state.recovery = PlayerTransferRecovery::HomebindWorldportReady;
            }
        });
    }

    pub fn terminate_worldport_recovery_like_cpp(&mut self, hub: &mut HubMut<'_>) {
        let _ = hub.update_player_teleport_state_like_cpp(|state| {
            state.recovery = PlayerTransferRecovery::Terminal;
            // Cancel stale near/delayed commands, not the unresolved far transfer.
            state.near_pending = false;
            state.near_destination = None;
            state.near_destination_zone_area = None;
            state.has_delayed = false;
            state.delayed = None;
        });
        hub.core
            .kick("worldport and homebind recovery failed; disconnect at retained source");
    }
}
