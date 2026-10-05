//! Hub operations for movement transfer and teleport state.

use crate::session::movement_protocol::{
    MovementAckEventLikeCpp, TELE_TO_SEAMLESS_LIKE_CPP, TeleportToOptionsLikeCpp,
};
use std::time::Instant;
use wow_constants::{ClientOpcodes, MovementFlag};
use wow_core::ObjectGuid;
use wow_entities::{MovementGeneratorKind, MovementSlot, PlayerTeleportStateLikeCpp};

impl crate::session::HubMut<'_> {
    pub fn reset_teleport_movement_state_like_cpp(&mut self) {
        let Some(movement_flags) = self.shared().resolved_player_movement_flags_like_cpp() else {
            return;
        };
        let movement_flags = movement_flags & MovementFlag::MASK_HAS_PLAYER_STATUS_OPCODE;
        self.set_player_movement_flags_like_cpp(movement_flags);
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.fixtures.movement.player_movement_jump_like_cpp =
                wow_packet::packets::movement::JumpInfo::default();
        }
        let _ = self.core.mutate_canonical_player_like_cpp(|player| {
            let motion = &mut player.unit_mut().subsystems_mut().motion;
            motion.interrupt_spline();
            let _ =
                motion.remove_generator_kind(MovementGeneratorKind::Effect, MovementSlot::Active);
        });
    }

    pub fn record_move_teleport_ack_like_cpp(
        &mut self,
        mover_guid: ObjectGuid,
        ack_index: i32,
        move_time: i32,
    ) -> bool {
        let accepted = self.core.player_guid() == Some(mover_guid);
        self.record_movement_ack_event_like_cpp(MovementAckEventLikeCpp {
            opcode: ClientOpcodes::MoveTeleportAck,
            mover_guid,
            ack_index: Some(ack_index),
            movement_force_id: None,
            movement_force_type: None,
            adjusted_time: (move_time >= 0).then_some(move_time as u32),
            speed: None,
            time_skipped: None,
            spline_id: None,
            accepted,
        });
        accepted
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_near_teleport_pending_like_cpp(
        &mut self,
        pending: bool,
        destination: Option<(u16, wow_core::Position)>,
        zone_area: Option<(u32, u32)>,
    ) -> bool {
        self.update_player_teleport_state_like_cpp(|state| {
            state.near_pending = pending;
            state.near_destination = destination;
            state.near_destination_zone_area = zone_area;
        })
    }
}
impl crate::session::HubRef<'_> {
    pub fn send_transfer_aborted_like_cpp(&self, map_id: u32, transfer_abort: u32) {
        self.send_transfer_aborted_with_params_like_cpp(map_id, transfer_abort, 0, 0);
    }

    pub fn send_transfer_aborted_with_params_like_cpp(
        &self,
        map_id: u32,
        transfer_abort: u32,
        arg: u8,
        map_difficulty_x_condition_id: i32,
    ) {
        self.core
            .send_packet(&wow_packet::packets::misc::TransferAborted {
                map_id,
                arg,
                map_difficulty_x_condition_id,
                transfer_abort,
            });
    }

    pub fn teleport_options_after_seamless_gate_like_cpp(
        &self,
        new_map: u32,
        mut options: TeleportToOptionsLikeCpp,
    ) -> TeleportToOptionsLikeCpp {
        if options & TELE_TO_SEAMLESS_LIKE_CPP == 0 {
            return options;
        }

        let Some(map_store) = self.catalogs.maps.store.as_ref() else {
            return options & !TELE_TO_SEAMLESS_LIKE_CPP;
        };
        let Some(old_map_entry) = map_store
            .get(u32::from(self.core.player_map_id_like_cpp()))
            .copied()
        else {
            return options & !TELE_TO_SEAMLESS_LIKE_CPP;
        };
        let Some(new_map_entry) = map_store.get(new_map).copied() else {
            return options & !TELE_TO_SEAMLESS_LIKE_CPP;
        };

        let old_cosmetic_parent = i32::from(old_map_entry.cosmetic_parent_map_id);
        let new_cosmetic_parent = i32::from(new_map_entry.cosmetic_parent_map_id);
        let current_map_id = i32::from(self.core.player_map_id_like_cpp());
        let new_map_id = i32::try_from(new_map).unwrap_or(i32::MAX);
        if old_cosmetic_parent != new_map_id
            && current_map_id != new_cosmetic_parent
            && !((old_cosmetic_parent != -1) ^ (old_cosmetic_parent != new_cosmetic_parent))
        {
            options &= !TELE_TO_SEAMLESS_LIKE_CPP;
        }

        options
    }

    pub fn send_same_map_move_update_teleport_to_visible_set_like_cpp(
        &self,
        source_guid: ObjectGuid,
    ) {
        use wow_packet::ServerPacket;

        let Some(registry) = self.core.player_registry() else {
            return;
        };
        let Some(source_position) = self.player_position_like_cpp() else {
            return;
        };
        let map_id = self.core.player_map_id_like_cpp();
        let instance_id = self
            .core
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let Some(status) = self.current_player_movement_info_like_cpp(source_guid) else {
            return;
        };
        let packet_bytes = wow_packet::packets::movement::MoveUpdateTeleport { status }.to_bytes();

        for registration in registry.movement_recipients_within_range(
            source_guid,
            map_id,
            instance_id,
            source_position,
            crate::map_manager::VISIBILITY_RADIUS,
        ) {
            let _ = registry.try_send_current_command(
                registration,
                crate::session::mailbox::SessionCommand::SendIfVisibleLikeCpp(
                    crate::session::mailbox::SendIfVisibleLikeCppCommand {
                        queued_at: Instant::now(),
                        source_guid,
                        map_id,
                        instance_id,
                        packet_bytes: packet_bytes.clone(),
                    },
                ),
            );
        }
    }

    pub fn represented_can_delay_teleport_like_cpp(&self) -> bool {
        self.player_teleport_state_snapshot_like_cpp()
            .is_some_and(|state| state.can_delay)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_has_delayed_teleport_like_cpp(&self) -> bool {
        self.player_teleport_state_snapshot_like_cpp()
            .is_some_and(|state| state.has_delayed)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_delayed_teleport_like_cpp(
        &self,
    ) -> Option<(u32, wow_core::Position, TeleportToOptionsLikeCpp)> {
        self.player_teleport_state_snapshot_like_cpp()
            .and_then(|state| state.delayed)
    }

    pub fn near_teleport_pending_like_cpp(&self) -> bool {
        self.player_teleport_state_snapshot_like_cpp()
            .is_some_and(|state| state.near_pending)
    }
}
impl crate::session::HubMut<'_> {
    pub fn update_player_teleport_state_like_cpp(
        &mut self,
        update: impl FnOnce(&mut PlayerTeleportStateLikeCpp),
    ) -> bool {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.core
                .update_player_teleport_state_with_fixture_like_cpp(
                    &mut self.fixtures.teleport,
                    update,
                )
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            self.core
                .update_player_teleport_state_with_fixture_like_cpp(update)
        }
    }

    pub fn set_represented_can_delay_teleport_like_cpp(&mut self, can_delay: bool) -> bool {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.core.set_can_delay_teleport_with_fixture_like_cpp(
                &mut self.fixtures.teleport,
                can_delay,
            )
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            self.core
                .set_can_delay_teleport_with_fixture_like_cpp(can_delay)
        }
    }
}
impl crate::session::HubRef<'_> {
    pub fn player_teleport_state_snapshot_like_cpp(&self) -> Option<PlayerTeleportStateLikeCpp> {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.core
                .player_teleport_state_snapshot_with_fixture_like_cpp(&self.fixtures.teleport)
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            self.core
                .with_owned_player_like_cpp(|player| *player.teleport_state_like_cpp())
        }
    }
}

impl crate::session::SessionCore {
    pub(crate) fn update_player_teleport_state_with_fixture_like_cpp(
        &mut self,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture: &mut crate::session::state::TeleportState,
        update: impl FnOnce(&mut PlayerTeleportStateLikeCpp),
    ) -> bool {
        if self.player_handle_like_cpp.is_some() {
            return self
                .with_owned_player_mut_like_cpp(|player| {
                    update(player.teleport_state_mut_like_cpp())
                })
                .is_some();
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            let mut state = self
                .player_teleport_state_snapshot_with_fixture_like_cpp(fixture)
                .unwrap_or_default();
            update(&mut state);
            fixture.pending_teleport = state.far_destination;
            fixture.represented_can_delay_teleport_like_cpp = state.can_delay;
            fixture.represented_has_delayed_teleport_like_cpp = state.has_delayed;
            fixture.near_teleport_pending_like_cpp = state.near_pending;
            fixture.represented_far_teleport_pending_like_cpp = state.far_pending;
            fixture.near_teleport_destination_like_cpp = state.near_destination;
            fixture.represented_delayed_teleport_like_cpp = state.delayed;
            fixture.near_teleport_destination_zone_area_like_cpp = state.near_destination_zone_area;
            true
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            let _ = update;
            false
        }
    }

    pub(crate) fn set_can_delay_teleport_with_fixture_like_cpp(
        &mut self,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture: &mut crate::session::state::TeleportState,
        can_delay: bool,
    ) -> bool {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.update_player_teleport_state_with_fixture_like_cpp(fixture, |state| {
                state.can_delay = can_delay
            })
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            self.update_player_teleport_state_with_fixture_like_cpp(|state| {
                state.can_delay = can_delay
            })
        }
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    fn player_teleport_state_snapshot_with_fixture_like_cpp(
        &self,
        fixture: &crate::session::state::TeleportState,
    ) -> Option<PlayerTeleportStateLikeCpp> {
        let canonical = self.with_owned_player_like_cpp(|player| *player.teleport_state_like_cpp());
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(PlayerTeleportStateLikeCpp {
                recovery: Default::default(),
                far_destination: fixture.pending_teleport,
                post_add: None,
                can_delay: fixture.represented_can_delay_teleport_like_cpp,
                has_delayed: fixture.represented_has_delayed_teleport_like_cpp,
                near_pending: fixture.near_teleport_pending_like_cpp,
                far_pending: fixture.represented_far_teleport_pending_like_cpp,
                near_destination: fixture.near_teleport_destination_like_cpp,
                delayed: fixture.represented_delayed_teleport_like_cpp,
                near_destination_zone_area: fixture.near_teleport_destination_zone_area_like_cpp,
            });
        }
        canonical
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) fn represented_can_delay_teleport_with_fixture_like_cpp(
        &self,
        fixture: &crate::session::state::TeleportState,
    ) -> bool {
        self.player_teleport_state_snapshot_with_fixture_like_cpp(fixture)
            .is_some_and(|state| state.can_delay)
    }
}
