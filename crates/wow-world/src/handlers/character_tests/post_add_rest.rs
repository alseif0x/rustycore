use super::*;

#[tokio::test]
async fn cancelled_post_add_retains_rest_update_through_reentry() {
    let (mut session, send_rx) = make_session_with_send_capacity(64);
    let guid = ObjectGuid::create_player(1, 42);
    let position = Position::new(1.0, 2.0, 3.0, 0.5);
    let terrain_dir = std::env::temp_dir().join(format!(
        "rustycore-rest-cancel-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&terrain_dir).unwrap();
    session.set_mmap_runtime_config_like_cpp(crate::session::MMapRuntimeConfigLikeCpp {
        data_dir: terrain_dir.to_string_lossy().into_owned(),
        enabled: true,
        disabled_map_ids: Default::default(),
    });
    session.set_map_store(crate::teleport_test_fixtures::world_maps([571]));
    session.set_player_guid(Some(guid));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_position_like_cpp(position);
    session.set_area_table_store(Arc::new(wow_data::AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 20,
            continent_id: 571,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: wow_data::AREA_FLAG_ALLIANCE_RESTING_LIKE_CPP,
        },
    ])));
    assert!(session.update_zone_represented_like_cpp(20, 20));
    assert!(session.represented_is_resting_like_cpp());
    drain_server_opcodes(&send_rx);

    let port = CollectionLoadPortLikeCpp::for_initial_world_states([]);
    let reached = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let signal = Arc::clone(&reached);
    port.initial_world_state_outcomes
        .lock()
        .unwrap()
        .push_back(Box::pin(async move {
            signal.store(true, std::sync::atomic::Ordering::SeqCst);
            std::future::pending().await
        }));
    session.set_player_lifecycle_port_like_cpp(port.clone());
    {
        let mut operation =
            Box::pin(session.send_initial_packets_after_add_to_map(guid, &position, 571, false));
        assert!(
            std::future::poll_fn(|cx| std::task::Poll::Ready(operation.as_mut().poll(cx)))
                .await
                .is_pending()
        );
        assert!(reached.load(std::sync::atomic::Ordering::SeqCst));
        // Drop exactly at the controlled world-state read, without a timeout or abandoned task.
    }
    assert!(!session.represented_is_resting_like_cpp());
    assert!(
        session
            .player_rest_state_snapshot_like_cpp()
            .unwrap()
            .deferred_flag_update_dirty
    );
    let before_resume = drain_server_opcodes(&send_rx);
    assert!(!before_resume.contains(&ServerOpcodes::InitWorldStates));

    port.initial_world_state_outcomes
        .lock()
        .unwrap()
        .push_back(Box::pin(async {
            PlayerInitialWorldStatesLoadOutcomeLikeCpp {
                templates: PlayerInitialWorldStateRowsLikeCpp::Loaded(vec![]),
                saved_values: PlayerInitialWorldStateRowsLikeCpp::Loaded(vec![]),
            }
        }));
    session
        .send_initial_packets_after_add_to_map(guid, &position, 571, false)
        .await;
    let after_resume = drain_server_opcodes(&send_rx);
    assert_eq!(
        after_resume.last(),
        Some(&ServerOpcodes::UpdateObject),
        "the pending rest update must survive both cancellation and same-zone reentry"
    );
    assert!(
        !session.take_deferred_rest_flag_update_dirty_like_cpp(),
        "accepted publication retires the marker"
    );

    // A disconnected sink is not acceptance, including the normal known-zone path.
    assert!(session.update_zone_represented_without_rest_update_packet_like_cpp(20, 20));
    drop(send_rx);
    assert!(!session.update_zone_represented_like_cpp(20, 20));
    assert!(
        session
            .player_rest_state_snapshot_like_cpp()
            .unwrap()
            .deferred_flag_update_dirty
    );
    assert!(!session.send_represented_resting_player_flag_update_like_cpp());
    std::fs::remove_dir(terrain_dir).unwrap();
}
