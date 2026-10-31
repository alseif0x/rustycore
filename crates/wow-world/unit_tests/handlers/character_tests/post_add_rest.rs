use super::*;

#[test]
fn same_area_sanctuary_uses_known_catalog_and_publishes_only_native_map_values() {
    use wow_constants::UnitPvpFlags;
    let other = UnitPvpFlags::PVP | UnitPvpFlags::FFA_PVP;
    for (mode, initial, expected) in [
        ("normal", other | UnitPvpFlags::SANCTUARY, other),
        ("normal_noop", other, other),
        ("missing_row", other | UnitPvpFlags::SANCTUARY, other),
        ("sanctuary", other, other | UnitPvpFlags::SANCTUARY),
        (
            "missing_catalog",
            other | UnitPvpFlags::SANCTUARY,
            other | UnitPvpFlags::SANCTUARY,
        ),
    ] {
        let (mut session, output) = make_session_with_send_capacity(64);
        let guid = ObjectGuid::create_player(1, 92_800);
        session.set_player_guid(Some(guid));
        crate::canonical_player_access::install_canonical_player_owner_for_test(
            &mut session,
            571,
            0,
        );
        session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
        crate::session::hub_mut(&mut session).set_player_zone_area_like_cpp(20, 20);
        if mode != "missing_catalog" {
            let rows = if mode == "missing_row" {
                vec![]
            } else {
                vec![wow_data::AreaTableEntry {
                    id: 20,
                    continent_id: 571,
                    parent_area_id: 0,
                    area_bit: -1,
                    exploration_level: 0,
                    mount_flags: 0,
                    flags: if mode == "sanctuary" {
                        wow_data::AREA_FLAG_NO_PVP_LIKE_CPP
                    } else {
                        0
                    },
                }]
            };
            session.set_area_table_store(Arc::new(wow_data::AreaTableStore::from_entries(rows)));
        }
        session
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.unit_mut().replace_all_pvp_flags_like_cpp(initial);
            })
            .unwrap();
        let manager = Arc::clone(session.core.canonical_map_manager.as_ref().unwrap());
        manager.lock().unwrap().update(1);
        session
            .send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(
            );
        while output.try_recv().is_ok() {}

        // Same-area returns false for criteria, but still runs native UpdateArea effects.
        assert!(
            !wow_world_instances::update_area_represented_with_rest_update_like_cpp(
                &mut session,
                20,
                false,
            )
        );
        assert!(
            output.try_recv().is_err(),
            "area flags have no immediate packet: {mode}"
        );
        session
            .core
            .with_owned_player_like_cpp(|player| {
                assert_eq!(player.unit().pvp_flags_like_cpp(), expected, "{mode}");
                assert_eq!(
                    player.unit().unit_data_changes_mask().is_set(64),
                    mode != "missing_catalog"
                );
                assert_eq!(
                    player.unit().unit_data_changes_mask().is_set(78),
                    mode != "missing_catalog"
                );
            })
            .unwrap();
        manager.lock().unwrap().update(1);
        let count = session
            .send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(
            );
        if mode == "missing_catalog" || mode == "normal_noop" {
            assert_eq!(count, 0, "no Object update is queued for {mode}");
            assert!(
                output.try_recv().is_err(),
                "no UpdateObject packet is emitted for {mode}"
            );
            continue;
        }
        assert_eq!(count, 1);
        let packet = output.try_recv().expect("owner native PvP VALUES");
        let mut wire = wow_packet::WorldPacket::from_bytes(&packet);
        assert_eq!(
            wire.read_uint16().unwrap(),
            ServerOpcodes::UpdateObject as u16
        );
        assert_eq!(wire.read_uint32().unwrap(), 1);
        assert_eq!(wire.read_uint16().unwrap(), 571);
        assert_eq!(wire.read_uint8().unwrap(), 0); // no destroy/out-of-range list
        let _block_bytes = wire.read_uint32().unwrap();
        assert_eq!(wire.read_uint8().unwrap(), 0); // VALUES
        assert_eq!(wire.read_packed_guid().unwrap(), guid);
        assert_eq!(wire.read_uint32().unwrap(), 10); // types + Unit mask + PvP byte
        assert_eq!(wire.read_uint32().unwrap(), 1 << 5); // UnitData only
        assert_eq!(wire.read_uint8().unwrap(), 1 << 2); // mask group64-95
        assert_eq!(wire.read_bytes(4).unwrap(), vec![0, 0, 0x40, 1]); // bits64/78
        assert_eq!(wire.read_uint8().unwrap(), expected.bits());
        assert_eq!(wire.remaining(), 0);
        assert!(output.try_recv().is_err());
        assert_eq!(session.send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(), 0);
    }
}

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
            .deferred_flag_update_dirty_like_cpp()
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
    assert!(
        session
            .send_initial_packets_after_add_to_map(guid, &position, 571, false)
            .await
    );
    let after_resume = drain_server_opcodes(&send_rx);
    assert_eq!(
        after_resume.last(),
        Some(&ServerOpcodes::UpdateObject),
        "the pending rest update must survive both cancellation and same-zone reentry"
    );
    assert!(
        !wow_world_instances::take_player_deferred_rest_flag_update_dirty_like_cpp(&mut session),
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
            .deferred_flag_update_dirty_like_cpp()
    );
    assert!(!session.send_represented_resting_player_flag_update_like_cpp());
    std::fs::remove_dir(terrain_dir).unwrap();
}
