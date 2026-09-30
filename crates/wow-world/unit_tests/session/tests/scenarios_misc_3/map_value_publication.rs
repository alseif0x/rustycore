use super::*;

#[tokio::test]
async fn map_send_object_updates_player_values_reaches_self_with_owner_fields_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 50_700);
    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        571,
        0,
    );

    {
        let mut manager = canonical.lock().unwrap();
        let player = manager
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player_guid)
            .unwrap();
        player.unit_mut().set_max_health(100);
        player.unit_mut().set_health(77);
        assert!(player.unit().world().object().is_object_updated());
        let _ = manager.update(1);
        assert_eq!(
            manager
                .find_map(571, 0)
                .unwrap()
                .last_send_object_updates_summary_like_cpp()
                .player_values_updates
                .len(),
            1
        );
    }

    assert_eq!(
        session
            .send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(
            ),
        1
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::UpdateObject]
    );
    assert_eq!(
        session
            .send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(
            ),
        0
    );
}

#[tokio::test]
async fn map_send_object_updates_player_values_observer_requires_committed_visibility_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let viewer_guid = ObjectGuid::create_player(1, 50_701);
    let source_guid = ObjectGuid::create_player(1, 50_702);
    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        viewer_guid,
        571,
        0,
    );
    add_canonical_test_player_on_map(
        &canonical,
        source_guid,
        Position::new(11.0, 20.0, 30.0, 0.0),
        571,
        0,
    );
    session.client_visible_guids_like_cpp.insert(source_guid);

    {
        let mut manager = canonical.lock().unwrap();
        let source = manager
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(source_guid)
            .unwrap();
        source.unit_mut().set_max_health(100);
        source.unit_mut().set_health(66);
        let _ = manager.update(1);
    }

    assert_eq!(
        session
            .send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(
            ),
        1
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::UpdateObject]
    );

    // A stale client-visible membership is insufficient once the source has
    // moved outside the current 2D visibility range.
    {
        let mut manager = canonical.lock().unwrap();
        manager
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(source_guid)
            .unwrap()
            .unit_mut()
            .world_mut()
            .relocate(Position::new(10_000.0, 20.0, 30.0, 0.0));
        let _ = manager.update(1);
    }
    assert_eq!(
        session
            .send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(
            ),
        0
    );
    assert!(drain_server_packet_bytes(&send_rx).is_empty());
}

#[tokio::test]
async fn map_send_object_updates_creature_values_reaches_visible_client_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 50_703);
    let creature_guid = test_creature_guid(50_704);
    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        571,
        0,
    );
    add_canonical_test_creature_on_map(
        &canonical,
        creature_guid,
        50_704,
        Position::new(11.0, 20.0, 30.0, 0.0),
        0,
        571,
        0,
    );
    session.client_visible_guids_like_cpp.insert(creature_guid);

    {
        let mut manager = canonical.lock().unwrap();
        let creature = manager
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_typed_creature_mut(creature_guid)
            .unwrap();
        creature.unit_mut().set_max_health(100);
        creature.unit_mut().set_health(42);
        let _ = manager.update(1);
    }

    assert_eq!(
        session
            .send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(
            ),
        1
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::UpdateObject]
    );
}
