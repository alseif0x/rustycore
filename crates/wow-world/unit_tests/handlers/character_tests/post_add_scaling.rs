use super::*;

#[tokio::test]
async fn worldport_scales_items_only_after_post_add_initialization() {
    assert_post_add_scaling(true, OutputClosure::Never).await;
}

#[tokio::test]
async fn login_post_add_applies_destination_item_scaling() {
    assert_post_add_scaling(false, OutputClosure::Never).await;
}

#[tokio::test]
async fn worldport_does_not_report_logged_in_when_post_add_delivery_closes() {
    assert_post_add_scaling(true, OutputClosure::DuringWorldStateRead).await;
}

#[tokio::test]
async fn worldport_finishes_native_effects_when_self_create_delivery_is_closed() {
    assert_post_add_scaling(true, OutputClosure::BeforeAck).await;
}

#[tokio::test]
async fn cancelled_worldport_finishes_native_effects_before_disconnect_save() {
    assert_post_add_scaling(true, OutputClosure::CancelDuringWorldStateRead).await;
}

#[tokio::test]
async fn retained_worldport_before_zone_finishes_native_effects_before_disconnect_save() {
    assert_post_add_scaling(true, OutputClosure::RetainedBeforeZone).await;
}

#[tokio::test]
async fn worldport_initial_aura_failure_does_not_resume_native_or_deferred_save() {
    assert_post_add_scaling(true, OutputClosure::SelectedAuraUnavailable).await;
}

#[tokio::test]
async fn login_initial_aura_failure_aborts_the_partial_login() {
    assert_post_add_scaling(false, OutputClosure::SelectedAuraUnavailable).await;
}

#[derive(Clone, Copy, PartialEq)]
enum OutputClosure {
    Never,
    BeforeAck,
    DuringWorldStateRead,
    CancelDuringWorldStateRead,
    RetainedBeforeZone,
    SelectedAuraUnavailable,
}

async fn assert_post_add_scaling(worldport: bool, closure: OutputClosure) {
    let (mut session, send_rx) = make_session_with_send_capacity(64);
    let guid = ObjectGuid::create_player(1, 42);
    let position = Position::new(1.0, 2.0, 3.0, 0.5);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0x40,
        },
    ])));
    session.set_player_guid(Some(guid));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    crate::player_inventory_test_fixture::complete_known_empty_player_inventory_like_cpp(
        &mut session,
    );
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    session.set_player_position_like_cpp(position);
    set_priest_level80_stats(&mut session, 1000, 20);
    assert!(session.player_stat_changes_like_cpp().is_some());
    if closure == OutputClosure::SelectedAuraUnavailable {
        session
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player
                    .unit_mut()
                    .subsystems_mut()
                    .auras
                    .insert_runtime_application_like_cpp(wow_entities::AuraApplicationLikeCpp {
                        spell_id: 8_326,
                        difficulty_id: 0,
                        caster_guid: guid,
                        slot: 4,
                        duration_total: 60_000,
                        duration_remaining: 60_000,
                        stack_count: 1,
                        aura_flags: 1,
                        effect_mask: 1,
                        aura_interrupt_flags: 0,
                        aura_interrupt_flags2: 0,
                        represented_effect: None,
                        represented_amount: 0,
                        represented_effect_amounts: Vec::new(),
                        represented_misc_value: None,
                        represented_multiplier: 1.0,
                        applied_at: std::time::Instant::now(),
                    });
            })
            .expect("canonical Player owns the selected initial aura");
    }
    assert!(session.complete_represented_trait_config_authority_load_like_cpp([], true));
    let canonical = session.core.canonical_map_manager.as_ref().unwrap().clone();
    let port = CollectionLoadPortLikeCpp::for_initial_world_states([]);
    let observed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let observation = observed.clone();
    let mut send_rx = Some(send_rx);
    if closure == OutputClosure::BeforeAck {
        drop(send_rx.take());
    }
    let close_during_read = if closure == OutputClosure::DuringWorldStateRead {
        send_rx.take()
    } else {
        None
    };
    port.initial_world_state_outcomes
        .lock()
        .unwrap()
        .push_back(Box::pin(async move {
            {
                let manager = canonical.lock().unwrap();
                let player = manager
                    .find_map(571, 0)
                    .unwrap()
                    .map()
                    .get_typed_player(guid)
                    .unwrap();
                assert!(
                    !player.gameplay_state().using_pvp_item_levels,
                    "scaling must not precede post-add InitWorldStates/auras/phase"
                );
            }
            observation.store(true, std::sync::atomic::Ordering::SeqCst);
            drop(close_during_read);
            if closure == OutputClosure::CancelDuringWorldStateRead {
                std::future::pending::<()>().await;
            }
            PlayerInitialWorldStatesLoadOutcomeLikeCpp {
                templates: PlayerInitialWorldStateRowsLikeCpp::Loaded(vec![]),
                saved_values: PlayerInitialWorldStateRowsLikeCpp::Loaded(vec![]),
            }
        }));
    session.set_player_lifecycle_port_like_cpp(port);
    if worldport {
        let mut native_player_state_before_post_add = None;
        assert!(session.schedule_represented_resurrection_after_teleport(
            wow_entities::PlayerResurrectionRequestLikeCpp {
                resurrecter: guid,
                map_id: 571,
                position,
                health: 100,
                mana: 50,
                aura: 0,
            }
        ));
        assert!(session.set_pending_teleport_like_cpp(Some((571, position))));
        assert!(session.set_represented_far_teleport_pending_like_cpp(true));
        session.set_state(crate::session::SessionState::Transfer);
        if closure == OutputClosure::SelectedAuraUnavailable {
            assert_eq!(
                session.save_current_player_to_db_like_cpp().await,
                crate::session::PlayerSaveOutcomeLikeCpp::Deferred
            );
            assert_eq!(
                session.core.with_owned_player_like_cpp(|player| {
                    player.has_deferred_player_save_like_cpp()
                }),
                Some(true)
            );
            native_player_state_before_post_add = Some(
                session
                    .core
                    .with_owned_player_like_cpp(|player| {
                        (player.unit().death_state(), player.unit().data().health)
                    })
                    .expect("canonical Player state before post-add initialization"),
            );
        }
        if closure == OutputClosure::RetainedBeforeZone {
            assert!(session.set_represented_far_teleport_pending_like_cpp(false));
            assert!(session.set_pending_teleport_like_cpp(None));
            assert!(session.begin_worldport_post_add_like_cpp(571, position));
            assert!(!session.begin_worldport_post_add_like_cpp(571, position));
            session.set_player_position_like_cpp(Position::new(4.0, 5.0, 6.0, 0.0));
            assert!(
                !session
                    .finish_worldport_native_before_disconnect_like_cpp()
                    .await
            );
            assert!(!session.represented_using_pvp_item_levels_like_cpp());
            assert!(
                session
                    .represented_delayed_resurrection_after_teleport_like_cpp()
                    .is_some()
            );
            session.set_player_position_like_cpp(position);
            drop(send_rx.take());
            session.kick("controlled retained post-add before zone");
            session.save_disconnect_player_to_db_like_cpp().await;
        } else if closure == OutputClosure::CancelDuringWorldStateRead {
            assert!(
                tokio::time::timeout(
                    std::time::Duration::from_millis(50),
                    session.handle_world_port_response(WorldPacket::new_empty()),
                )
                .await
                .is_err()
            );
            assert!(observed.load(std::sync::atomic::Ordering::SeqCst));
            assert!(!crate::session::hub_ref(&session).represented_far_teleport_pending_like_cpp());
            assert!(session.pending_teleport_like_cpp().is_none());
            drop(send_rx.take());
            session.kick("controlled worldport cancellation before disconnect save");
            session.save_disconnect_player_to_db_like_cpp().await;
        } else {
            session
                .handle_world_port_response(WorldPacket::new_empty())
                .await;
        }
        if closure == OutputClosure::SelectedAuraUnavailable {
            assert_eq!(session.state(), crate::session::SessionState::Disconnecting);
            assert!(
                session
                    .represented_delayed_resurrection_after_teleport_like_cpp()
                    .is_some(),
                "failed initial aura publication stops before native completion"
            );
            assert_eq!(
                session.core.with_owned_player_like_cpp(|player| {
                    player
                        .resurrection_state_like_cpp()
                        .delayed_after_teleport
                        .map(|continuation| continuation.phase)
                }),
                Some(Some(
                    wow_entities::PlayerResurrectionPhase::TeleportCompleted
                )),
                "the valid worldport ACK keeps its owned phase while post-add initialization fails"
            );
            assert_eq!(
                session.core.with_owned_player_like_cpp(|player| {
                    player.has_deferred_player_save_like_cpp()
                }),
                Some(true),
                "initial aura failure leaves deferred save intent for later owner handling"
            );
            assert_eq!(
                session
                    .core
                    .with_owned_player_like_cpp(|player| {
                        (player.unit().death_state(), player.unit().data().health)
                    })
                    .expect("canonical Player state after failed post-add initialization"),
                native_player_state_before_post_add
                    .expect("initial failure case captures its pre-native Player state"),
                "failed initial Aura publication does not apply native resurrection health or death state"
            );
            assert_eq!(session.temporary_pet_resummon_requests_like_cpp(), 0);
            assert!(!session.represented_using_pvp_item_levels_like_cpp());
            assert!(observed.load(std::sync::atomic::Ordering::SeqCst));
            return;
        }
        assert_eq!(
            session.state(),
            if closure != OutputClosure::Never {
                crate::session::SessionState::Disconnecting
            } else {
                crate::session::SessionState::LoggedIn
            }
        );
        assert!(
            session
                .represented_delayed_resurrection_after_teleport_like_cpp()
                .is_none()
        );
        let pet_resummons = session.temporary_pet_resummon_requests_like_cpp();
        assert_eq!(pet_resummons, 1);
        assert!(
            session
                .finish_worldport_native_before_disconnect_like_cpp()
                .await
        );
        assert_eq!(
            session.temporary_pet_resummon_requests_like_cpp(),
            pet_resummons
        );
    } else {
        let complete = session
            .send_initial_packets_after_add_to_map(guid, &position, 571, false)
            .await;
        if closure == OutputClosure::SelectedAuraUnavailable {
            assert!(!complete, "the selected full aura update is unavailable");
            assert_ne!(session.state(), crate::session::SessionState::LoggedIn);
            wow_world_character::abort_partial_login_sequence_like_cpp(&mut session);
            assert_eq!(session.state(), crate::session::SessionState::Disconnecting);
            return;
        }
        assert!(complete, "the successful post-add sequence completes");
    }
    assert_eq!(
        observed.load(std::sync::atomic::Ordering::SeqCst),
        closure != OutputClosure::RetainedBeforeZone
    );
    assert!(
        session.represented_using_pvp_item_levels_like_cpp(),
        "the shared post-add phase must apply destination scaling for login and worldport"
    );
    if let Some(send_rx) = send_rx {
        assert!(drain_server_opcodes(&send_rx).contains(&ServerOpcodes::InitWorldStates));
    }
}
