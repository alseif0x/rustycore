use super::*;

#[tokio::test]
async fn health_aura_state_changes_only_in_the_current_admitted_unit_phase() {
    for (dead, stale) in [(false, false), (true, false), (false, true)] {
        let (mut session, admission, _packet_tx, _send_rx) = logged_in_recovery_session();
        session.mark_represented_character_spell_cooldowns_loaded_like_cpp();
        session
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_player_aura_authority_complete_like_cpp(true);
                player.unit_mut().set_max_health(100);
                player.unit_mut().set_health(10);
                player
                    .unit_mut()
                    .subsystems_mut()
                    .auras
                    .modify_aura_state(1, true);
                if dead {
                    player
                        .unit_mut()
                        .set_death_state(wow_constants::DeathState::Corpse);
                }
                player.clear_data_changes();
            })
            .unwrap();
        assert_eq!(
            wow_world_spell::represented_player_aura_state_mask_like_cpp(&session),
            Some(1),
            "health setters do not approximate the Unit phase"
        );
        let request = if stale {
            PlayerRecoveryAdmission {
                residence_revision: admission.residence_revision + 1,
                ..admission
            }
        } else {
            admission
        };
        let reply = run_recovery_request(
            &mut session,
            request,
            SessionPhasePermitLikeCpp::new_like_cpp(),
        )
        .await;
        if stale {
            assert_eq!(
                reply.outcome,
                SessionPhasePassOutcomeLikeCpp::RefusedBeforeStart
            );
        } else {
            assert_eq!(reply.status, Some(PlayerRecoveryStatus::Completed));
        }
        let expected = if dead || stale { 1 } else { 0x0090_1023 };
        assert_eq!(
            wow_world_spell::represented_player_aura_state_mask_like_cpp(&session),
            Some(expected)
        );
        let frozen = session
            .core
            .with_owned_player_like_cpp(|player| player.values_update(true))
            .unwrap();
        let marked = frozen
            .unit_data
            .as_ref()
            .is_some_and(|unit| unit.mask.is_set(44));
        assert_eq!(marked, !dead && !stale);
        if marked {
            let packet =
                wow_world_core::entity_update_bridge::player_values_update_to_packet(&frozen)
                    .unwrap();
            assert_eq!(packet.unit_data.unwrap().aura_state, expected);
        }
    }
}
