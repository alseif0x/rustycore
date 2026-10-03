//! C++ completes worldport native effects before logout/save (WorldSession.cpp:544).
//! Rust can suspend in visibility or world-state reads after clearing the far flag.
//! Retain only remaining native phases on the canonical Player; never replay the ACK.
use crate::session::WorldSession;
use wow_core::Position;
use wow_entities::PlayerWorldportPostAddPhaseLikeCpp as Phase;

impl WorldSession {
    pub(crate) fn begin_worldport_post_add_like_cpp(
        &mut self,
        map_id: u32,
        position: Position,
    ) -> bool {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.begin_worldport_post_add_like_cpp(&mut hub, map_id, position)
    }

    pub(crate) fn advance_worldport_post_add_like_cpp(&mut self, phase: Phase) -> bool {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.advance_worldport_post_add_like_cpp(&mut hub, phase)
    }

    /// Finish represented native phases, not client initialization or a durable save.
    /// False forbids ordinary disconnect cleanup: the owner/completion is unavailable.
    /// No packet, query or await is needed; terrain reads remain outside map guards.
    pub fn finish_worldport_native_before_disconnect_like_cpp(&mut self) -> bool {
        if self.player_guid().is_none() {
            return true;
        }
        let Some(mut state) =
            crate::session::hub_ref(self).player_teleport_state_snapshot_like_cpp()
        else {
            return false;
        };
        if state.far_pending && state.recovery != wow_entities::PlayerTransferRecovery::Terminal {
            if !self.finish_pending_far_entry_before_disconnect_like_cpp() {
                return false;
            }
            let Some(updated) =
                crate::session::hub_ref(self).player_teleport_state_snapshot_like_cpp()
            else {
                return false;
            };
            state = updated;
            if state.far_pending && state.recovery != wow_entities::PlayerTransferRecovery::Terminal
            {
                return false;
            }
        }
        let Some(progress) = state.post_add else {
            return true;
        };
        if !self
            .core
            .with_owned_player_like_cpp(|player| {
                player.unit().world().map_id() == progress.map_id
                    && player.unit().world().position() == progress.position
            })
            .unwrap_or(false)
        {
            return false;
        }
        if progress.phase == Phase::BeforeZone {
            if !self.apply_post_add_zone_from_terrain_like_cpp(
                progress.map_id as i32,
                &progress.position,
            ) || !self.advance_worldport_post_add_like_cpp(Phase::ZoneApplied)
            {
                return false;
            }
        }
        if progress.phase < Phase::ScalingApplied {
            if self
                .update_represented_item_level_area_based_scaling_with_publication_like_cpp(false)
                .is_none()
                || !self.advance_worldport_post_add_like_cpp(Phase::ScalingApplied)
            {
                return false;
            }
        }
        crate::session::cx_pets(self).resummon_pet_temporary_unsummoned_like_cpp();
        self.process_represented_delayed_resurrection_after_teleport_like_cpp();
        self.core
            .with_owned_player_mut_like_cpp(|player| {
                player.teleport_state_mut_like_cpp().post_add = None;
            })
            .is_some()
    }

    /// C++ LogoutPlayer completes a pending far transfer without waiting for a client ACK.
    /// Preserve the existing bounded homebind/terminal-source contract, without packets.
    fn finish_pending_far_entry_before_disconnect_like_cpp(&mut self) -> bool {
        use wow_entities::PlayerTransferRecovery;
        let Some(state) = crate::session::hub_ref(self).player_teleport_state_snapshot_like_cpp()
        else {
            return false;
        };
        if state.can_delay || state.post_add.is_some() {
            return false;
        }
        let Some(mut destination) = state.far_destination else {
            return false;
        };
        if !self.try_attach_worldport_destination_with_publication_like_cpp(
            destination.0,
            destination.1,
            false,
        ) {
            let homebind = self
                .represented_homebind_like_cpp()
                .filter(|home| (home.map_id, home.position) != destination);
            if state.recovery != PlayerTransferRecovery::None || homebind.is_none() {
                self.terminate_worldport_recovery_like_cpp();
                return true;
            }
            let homebind = homebind.expect("checked homebind");
            destination = (homebind.map_id, homebind.position);
            if !crate::session::hub_mut(self).update_player_teleport_state_like_cpp(|state| {
                state.recovery = PlayerTransferRecovery::Homebind;
                state.far_destination = Some(destination);
            }) {
                return false;
            }
            if !self.try_attach_worldport_destination_with_publication_like_cpp(
                destination.0,
                destination.1,
                false,
            ) {
                self.terminate_worldport_recovery_like_cpp();
                return true;
            }
        }
        if !crate::session::hub_mut(self).update_player_teleport_state_like_cpp(|state| {
            state.far_pending = false;
            state.far_destination = None;
        }) {
            return false;
        }
        crate::session::hub_mut(self).reset_movement_counter_like_cpp();
        crate::session::hub_ref(self).update_registry_position();
        self.begin_worldport_post_add_like_cpp(destination.0, destination.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn save_fixture() -> WorldSession {
        let (_input, packets) = flume::unbounded();
        let (output, _receiver) = flume::unbounded();
        let mut session = WorldSession::new(
            1,
            "transfer-save".into(),
            0,
            2,
            2,
            54261,
            vec![],
            "enUS".into(),
            packets,
            output,
        );
        session.set_player_guid(Some(wow_core::ObjectGuid::create_player(1, 42)));
        crate::canonical_player_access::install_canonical_player_owner_for_test(
            &mut session,
            571,
            0,
        );
        session
    }

    #[test]
    fn pending_far_completion_normalizes_orientation_before_exact_post_add_comparison() {
        for orientation in [180.0, -7.0, std::f32::consts::TAU] {
            let mut session = save_fixture();
            session.set_map_store(crate::teleport_test_fixtures::world_maps([571, 1]));
            let requested = Position::new(67.7607, 2490.98, -4.29649, orientation);
            let expected = wow_entities::WorldLocation::new(
                1,
                requested.x,
                requested.y,
                requested.z,
                orientation,
            )
            .position();
            assert!(session.set_pending_teleport_like_cpp(Some((1, requested))));
            assert!(
                crate::session::hub_mut(&mut session)
                    .update_player_teleport_state_like_cpp(|state| state.far_pending = true)
            );
            assert_eq!(session.pending_teleport_like_cpp(), Some((1, expected)));
            assert!(session.finish_worldport_native_before_disconnect_like_cpp());
            assert_eq!(
                crate::session::hub_ref(&session).player_position_like_cpp(),
                Some(expected)
            );
            let state = crate::session::hub_ref(&session)
                .player_teleport_state_snapshot_like_cpp()
                .unwrap();
            assert!(!state.far_pending);
            assert!(state.post_add.is_none());
            assert!(session.prepare_player_save_like_cpp(1).is_some());
        }
    }

    #[test]
    fn prepared_save_rejects_retained_post_add_until_native_completion() {
        let mut session = save_fixture();
        let position = crate::session::hub_ref(&session)
            .player_position_like_cpp()
            .unwrap();
        assert!(session.prepare_player_save_like_cpp(1).is_some());
        assert!(session.begin_worldport_post_add_like_cpp(571, position));
        assert!(session.prepare_player_save_like_cpp(1).is_none());
        assert!(
            crate::session::hub_ref(&session)
                .player_teleport_state_snapshot_like_cpp()
                .unwrap()
                .post_add
                .is_some()
        );
        assert!(session.finish_worldport_native_before_disconnect_like_cpp());
        assert!(session.prepare_player_save_like_cpp(1).is_some());
    }

    #[test]
    fn prepared_save_rejects_pending_far_but_preserves_terminal_source_exception() {
        let mut session = save_fixture();
        assert!(session.prepare_player_save_like_cpp(1).is_some());
        assert!(
            crate::session::hub_mut(&mut session).update_player_teleport_state_like_cpp(|state| {
                state.far_pending = true;
                state.far_destination = Some((1, Position::new(7.0, 8.0, 9.0, 0.5)));
            })
        );
        assert!(session.prepare_player_save_like_cpp(1).is_none());
        assert!(
            crate::session::hub_mut(&mut session).update_player_teleport_state_like_cpp(|state| {
                state.recovery = wow_entities::PlayerTransferRecovery::Terminal;
            })
        );
        assert!(session.prepare_player_save_like_cpp(1).is_some());
    }

    #[tokio::test]
    async fn transfer_save_retains_native_intent_when_timer_is_reset() {
        use crate::session::PlayerSaveOutcomeLikeCpp;
        let mut session = save_fixture();
        session.set_player_save_interval_ms_like_cpp(100);
        session.lifecycle.update_player_save_timer_like_cpp(100);
        assert!(session.lifecycle.pending_periodic_player_save_like_cpp());
        assert!(
            crate::session::hub_mut(&mut session).update_player_teleport_state_like_cpp(|state| {
                state.far_pending = true;
                state.far_destination = Some((1, Position::new(7.0, 8.0, 9.0, 0.5)));
            })
        );
        for _ in 0..2 {
            assert_eq!(
                session.save_current_player_to_db_like_cpp().await,
                PlayerSaveOutcomeLikeCpp::Deferred
            );
            assert_eq!(
                session
                    .core
                    .with_owned_player_like_cpp(|p| p.has_deferred_player_save_like_cpp()),
                Some(true)
            );
        }
        assert_eq!(session.lifecycle.next_player_save_ms_like_cpp(), 100);
        assert!(!session.lifecycle.pending_periodic_player_save_like_cpp());
        assert!(session.finish_worldport_native_before_disconnect_like_cpp());
        // Unavailable persistence is not a confirmation and must retain the intent.
        assert_eq!(
            session.save_current_player_to_db_like_cpp().await,
            PlayerSaveOutcomeLikeCpp::Unavailable
        );
        assert_eq!(
            session
                .core
                .with_owned_player_like_cpp(|p| p.has_deferred_player_save_like_cpp()),
            Some(true)
        );
    }
}
