//! The worldport post-add phases and the native completion before disconnect
//! moved to `wow_world_character` (#1363 slice 13a); their session-level
//! save-fence regressions stay here.

#[cfg(test)]
mod tests {
    use {crate::session::WorldSession, wow_core::Position};

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
            crate::session::registry::build_dispatch_table(),
        );
        session.set_player_guid(Some(wow_core::ObjectGuid::create_player(1, 42)));
        crate::canonical_player_access::install_canonical_player_owner_for_test(
            &mut session,
            571,
            0,
        );
        session
    }

    #[tokio::test]
    async fn pending_far_completion_normalizes_orientation_before_exact_post_add_comparison() {
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
            assert!(session.finish_worldport_native_before_disconnect_like_cpp().await);
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

    #[tokio::test]
    async fn prepared_save_rejects_retained_post_add_until_native_completion() {
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
        assert!(session.finish_worldport_native_before_disconnect_like_cpp().await);
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
        assert!(session.finish_worldport_native_before_disconnect_like_cpp().await);
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
