//! Hub operation for movement time-skipped ACKs.

use crate::session::movement_protocol::MovementAckEventLikeCpp;
use wow_constants::ClientOpcodes;
use wow_core::ObjectGuid;

impl crate::session::HubMut<'_> {
    pub fn apply_move_time_skipped_like_cpp(
        &mut self,
        mover_guid: ObjectGuid,
        time_skipped: u32,
    ) -> bool {
        // C++ validates against the active `m_unitMovedByMe`, so a controlled
        // Creature/Pet is a valid mover too (MovementHandler.cpp:721-739).
        // Keep the owner-specific write on the mover rather than silently
        // advancing the Player clock for every ACK.
        let adjusted_time = if self.shared().player_moved_unit_guid_like_cpp() != Some(mover_guid) {
            None
        } else if self.core.player_guid() == Some(mover_guid) {
            self.shared()
                .resolved_player_movement_time_like_cpp()
                .map(|time| time.wrapping_add(time_skipped))
                .inspect(|adjusted_time| self.set_player_movement_time_like_cpp(*adjusted_time))
        } else {
            self.core.mutate_world_creature(mover_guid, |creature| {
                let adjusted_time = creature
                    .creature
                    .unit()
                    .movement_time_like_cpp()
                    .wrapping_add(time_skipped);
                creature
                    .creature
                    .unit_mut()
                    .set_movement_time_like_cpp(adjusted_time);
                adjusted_time
            })
        };
        let accepted = adjusted_time.is_some();

        self.record_movement_ack_event_like_cpp(MovementAckEventLikeCpp {
            opcode: ClientOpcodes::MoveTimeSkipped,
            mover_guid,
            ack_index: None,
            movement_force_id: None,
            movement_force_type: None,
            adjusted_time,
            speed: None,
            time_skipped: Some(time_skipped),
            spline_id: None,
            accepted,
        });
        accepted
    }
}
