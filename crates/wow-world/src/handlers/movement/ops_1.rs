//! Movement handlers operations, part 1 of 1.
//!
//! The inherent `WorldSession` impl is divided by responsibility under
//! #654; every method keeps its original body.

use super::*;

impl WorldSession {
    #[cfg(test)]
    pub(crate) async fn handle_movement(&mut self, pkt: wow_packet::WorldPacket) {
        let area_trigger_catalogs = self.area_trigger_catalogs_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        let progression = self.progression_catalogs_for_test_like_cpp();
        wow_world_entities::handle_movement_with_catalogs_like_cpp(
            self,
            &area_trigger_catalogs,
            &creature_spawn_catalogs,
            &progression,
            &crate::session::SessionHandlerCatalogsLikeCpp::default().player_grid_loader,
            pkt,
        )
        .await;
    }
    #[cfg(test)]
    pub(crate) async fn handle_movement_info_like_cpp(
        &mut self,
        opcode: Option<ClientOpcodes>,
        info: MovementInfo,
    ) {
        let area_trigger_catalogs = self.area_trigger_catalogs_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        let progression = self.progression_catalogs_for_test_like_cpp();
        wow_world_entities::handle_movement_info_with_catalogs_like_cpp(
            self,
            &area_trigger_catalogs,
            &creature_spawn_catalogs,
            &progression,
            &crate::session::SessionHandlerCatalogsLikeCpp::default().player_grid_loader,
            opcode,
            info,
        )
        .await;
    }
    /// Handle C++ `HandleForceSpeedChangeAck` and movement-force magnitude ACKs.
    ///
    /// `CMSG_MOVE_SET_MOD_MOVEMENT_FORCE_MAGNITUDE_ACK`
    /// (`MovementHandler.cpp:638`) now routes through the ApplicationMovement
    /// owner like its C++ handler; its registered thunk is the production path,
    /// and this branch keeps the direct WorldSession entry point on that same
    /// path for its unit caller instead of duplicating the moved body.
    pub(crate) async fn handle_movement_speed_ack(
        &mut self,
        opcode: ClientOpcodes,
        mut pkt: MovementSpeedAck,
    ) {
        if matches!(opcode, ClientOpcodes::MoveSetModMovementForceMagnitudeAck) {
            let mut step =
                wow_world_application::MovementHandlerCxLikeCpp::new(crate::session::hub_mut(self))
                    .handle_move_set_mod_movement_force_magnitude_ack(opcode, pkt.ack, pkt.speed)
                    .await;
            let accepted = wow_world_entities::handle_movement_force_mod_magnitude_ack_like_cpp(
                self,
                step.opcode,
                &mut step.ack,
                step.speed,
            );
            wow_world_application::MovementHandlerCxLikeCpp::new(crate::session::hub_mut(self))
                .finish_move_set_mod_movement_force_magnitude_ack(step, accepted)
                .await;
            return;
        }

        trace!(
            account = self.core.account_id,
            ?opcode,
            speed = pkt.speed,
            "MovementSpeedAck"
        );
        wow_world_entities::handle_force_speed_change_ack_like_cpp(
            self,
            opcode,
            &mut pkt.ack,
            pkt.speed,
        );
    }
}
