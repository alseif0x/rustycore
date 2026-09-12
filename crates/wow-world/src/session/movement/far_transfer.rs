//! Represented far world transfer: the interruptible writer fence, its
//! cancellation and recovery, and the realm handoff it publishes.
//!
//! Moved out of the Session root under #603. Behaviour is preserved; the
//! ordering established by #585 and #586 is unchanged.

use super::*;

impl WorldSession {
    pub(in crate::session) async fn initiate_far_teleport_after_delay_like_cpp(
        &mut self,
        map_id: u32,
        destination: wow_core::Position,
        options: TeleportToOptionsLikeCpp,
    ) {
        if !self.update_player_teleport_state_like_cpp(|state| {
            state.near_pending = false;
            state.near_destination = None;
            state.near_destination_zone_area = None;
        }) {
            return;
        }
        let options = if self.current_canonical_player_map_key_like_cpp().is_none() {
            options & !TELE_TO_SEAMLESS_LIKE_CPP
        } else {
            options
        };
        self.set_selection_guid_like_cpp(None);
        self.combat_stop_like_cpp();
        self.reset_contested_pvp_like_cpp();
        self.maybe_leave_represented_battleground_on_far_teleport_like_cpp(map_id);
        self.unsummon_represented_pet_temporary_if_any_like_cpp();
        let _ = self.remove_all_dynamic_objects_for_current_player_like_cpp();
        let _ = self.remove_all_area_triggers_for_current_player_like_cpp();
        if options & TELE_TO_SPELL_LIKE_CPP == 0 {
            let _ = self.interrupt_non_melee_spells_for_far_teleport_like_cpp();
        }
        let _ = self.remove_moving_or_turning_interrupt_auras_for_far_teleport_like_cpp();

        let Some(current_pos) = self.player_position_like_cpp() else {
            return;
        };

        use wow_packet::packets::misc::{SuspendToken, TransferPending};

        if !self.player_logout_like_cpp && options & TELE_TO_SEAMLESS_LIKE_CPP == 0 {
            let transfer_pending = TransferPending {
                map_id,
                old_map_position: current_pos,
                ship: None,
                transfer_spell_id: None,
            };
            self.send_packet_realm(&transfer_pending);
            self.clear_active_player_transport_server_time_override_for_far_teleport_like_cpp();
        }

        let _ = self.remove_current_player_from_canonical_current_map_like_cpp();

        if !self.set_pending_teleport_like_cpp(Some((map_id, destination))) {
            return;
        }
        self.active_area_trigger = None;
        if !self.set_represented_far_teleport_pending_like_cpp(true) {
            return;
        }
        self.state = SessionState::Transfer;

        if !self.player_logout_like_cpp {
            if options & TELE_TO_SEAMLESS_LIKE_CPP == 0
                && !self
                    .wait_for_realm_send_before_instance_update_like_cpp()
                    .await
            {
                self.kick("Delayed TransferPending writer fence failed; retain native destination");
                return;
            }
            // C++ SuspendToken.SequenceIndex = m_movementCounter (Player.cpp:1466); must match
            // the ResumeToken sent later so the client resumes. #NEXT.R8.ENTITIES.1229.
            let Some(suspend_seq) = self.movement_counter_like_cpp() else {
                return;
            };
            self.send_packet(&SuspendToken {
                sequence_index: suspend_seq,
                reason: if options & TELE_TO_SEAMLESS_LIKE_CPP != 0 {
                    2
                } else {
                    1
                },
            });
        }
    }
    pub(in crate::session) fn maybe_leave_represented_battleground_on_far_teleport_like_cpp(
        &mut self,
        new_map: u32,
    ) {
        if self
            .player_battleground_state_snapshot_like_cpp()
            .and_then(|state| state.battleground_map_id_like_cpp())
            .is_some_and(|bg_map_id| bg_map_id != new_map)
        {
            self.request_represented_battleground_leave_like_cpp();
        }
    }
    pub(in crate::session) fn clear_active_player_transport_server_time_override_for_far_teleport_like_cpp(
        &mut self,
    ) {
        let _ = self.mutate_active_player_update_state_like_cpp(|state| {
            state.active_local_flags &= !PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP;
            state.active_transport_server_time = 0;
        });
        self.sync_current_player_session_visibility_detection_like_cpp();
    }
    pub(crate) fn represented_far_teleport_pending_like_cpp(&self) -> bool {
        self.player_teleport_state_snapshot_like_cpp()
            .is_some_and(|state| state.far_pending)
    }
    pub(crate) fn set_represented_far_teleport_pending_like_cpp(&mut self, pending: bool) -> bool {
        self.update_player_teleport_state_like_cpp(|state| state.far_pending = pending)
    }
}
