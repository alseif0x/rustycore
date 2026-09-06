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

#[derive(Clone, Copy, PartialEq)]
enum OutputClosure {
    Never,
    BeforeAck,
    DuringWorldStateRead,
    CancelDuringWorldStateRead,
    RetainedBeforeZone,
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
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    session.set_player_position_like_cpp(position);
    set_priest_level80_stats(&mut session, 1000, 20);
    assert!(session.player_stat_changes_like_cpp().is_some());
    assert!(session.complete_represented_trait_config_authority_load_like_cpp([], true));
    let canonical = session.canonical_map_manager.as_ref().unwrap().clone();
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
        assert!(
            session.schedule_represented_resurrection_after_teleport_like_cpp(
                wow_entities::PlayerResurrectionRequestLikeCpp {
                    resurrecter: guid,
                    map_id: 571,
                    position,
                    health: 100,
                    mana: 50,
                    aura: 0,
                }
            )
        );
        assert!(session.set_pending_teleport_like_cpp(Some((571, position))));
        assert!(session.set_represented_far_teleport_pending_like_cpp(true));
        session.set_state(crate::session::SessionState::Transfer);
        if closure == OutputClosure::RetainedBeforeZone {
            assert!(session.set_represented_far_teleport_pending_like_cpp(false));
            assert!(session.set_pending_teleport_like_cpp(None));
            assert!(session.begin_worldport_post_add_like_cpp(571, position));
            assert!(!session.begin_worldport_post_add_like_cpp(571, position));
            session.set_player_position_like_cpp(Position::new(4.0, 5.0, 6.0, 0.0));
            assert!(!session.finish_worldport_native_before_disconnect_like_cpp());
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
            assert!(!session.represented_far_teleport_pending_like_cpp());
            assert!(session.pending_teleport_like_cpp().is_none());
            drop(send_rx.take());
            session.kick("controlled worldport cancellation before disconnect save");
            session.save_disconnect_player_to_db_like_cpp().await;
        } else {
            session
                .handle_world_port_response(WorldPacket::new_empty())
                .await;
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
        assert!(session.finish_worldport_native_before_disconnect_like_cpp());
        assert_eq!(
            session.temporary_pet_resummon_requests_like_cpp(),
            pet_resummons
        );
    } else {
        session
            .send_initial_packets_after_add_to_map(guid, &position, 571, false)
            .await;
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
