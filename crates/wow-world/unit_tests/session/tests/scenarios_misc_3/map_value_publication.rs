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
    session
        .core
        .client_visible_guids_like_cpp
        .insert(source_guid);

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

/// Source: Player.cpp `SetQuestSlot` (15963-16001) changes the nested native
/// row; UpdateFields.cpp (1791-1797) exposes QuestLog to PARTY_MEMBER receivers.
#[tokio::test]
async fn map_send_object_updates_publishes_native_quest_log_clear_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 50_705);
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
        assert!(player.set_quest_slot_like_cpp(4, 59_415));
        assert!(player.unit().world().object().is_object_updated());
        let _ = manager.update(1);
        let summary = manager
            .find_map(571, 0)
            .unwrap()
            .last_send_object_updates_summary_like_cpp();
        let update = &summary.player_values_updates[0]
            .values_update
            .player_data
            .as_ref()
            .unwrap()
            .quest_log[4];
        assert_eq!(update.values().quest_id(), 59_415);
        assert_eq!(update.change_mask(), (1 << 29) - 1);
        assert!(
            !manager
                .find_map(571, 0)
                .unwrap()
                .map()
                .get_typed_player(player_guid)
                .unwrap()
                .unit()
                .world()
                .object()
                .is_object_updated(),
            "Map consumes the queued PlayerData and row masks"
        );
    }
    assert_eq!(
        session
            .send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(
            ),
        1,
        "the World visibility consumer sends the Map-owned QuestLog snapshot"
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::UpdateObject]
    );

    {
        let mut manager = canonical.lock().unwrap();
        let player = manager
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player_guid)
            .unwrap();
        assert!(player.set_quest_slot_counter_like_cpp(4, 7, 0x1234));
        assert!(player.set_quest_slot_state_like_cpp(4, 1 << 8));
        let _ = manager.update(1);
        let summary = manager
            .find_map(571, 0)
            .unwrap()
            .last_send_object_updates_summary_like_cpp();
        let update = &summary.player_values_updates[0]
            .values_update
            .player_data
            .as_ref()
            .unwrap()
            .quest_log[4];
        assert_eq!(update.values().quest_id(), 59_415);
        assert_eq!(update.values().state_flags(), 1 << 8);
        assert_eq!(update.values().objective_progress()[7], 0x1234);
        assert_eq!(
            update.change_mask(),
            (1 << 0) | (1 << 3) | (1 << 4) | (1 << (5 + 7))
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

    {
        let mut manager = canonical.lock().unwrap();
        let player = manager
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player_guid)
            .unwrap();
        assert!(player.set_quest_slot_like_cpp(4, 0));
        let _ = manager.update(1);
        let summary = manager
            .find_map(571, 0)
            .unwrap()
            .last_send_object_updates_summary_like_cpp();
        let update = &summary.player_values_updates[0]
            .values_update
            .player_data
            .as_ref()
            .unwrap()
            .quest_log[4];
        assert_eq!(update.values().quest_id(), 0);
        assert_eq!(update.change_mask(), (1 << 29) - 1);
    }
    assert_eq!(
        session
            .send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(
            ),
        1,
        "the explicit native clear is published through Map and the World consumer"
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::UpdateObject]
    );
    assert_eq!(
        session
            .send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(
            ),
        0,
        "a consumed QuestLog clear cannot be published twice"
    );
    assert!(send_rx.is_empty());
}

#[tokio::test]
async fn map_send_object_updates_exposes_quest_log_to_current_group_members_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let viewer_guid = ObjectGuid::create_player(1, 50_706);
    let source_guid = ObjectGuid::create_player(1, 50_707);
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
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());

    let mut group = GroupInfo::new(viewer_guid);
    assert!(group.add_member(source_guid));
    group.group_category = wow_social::group::GROUP_CATEGORY_INSTANCE_LIKE_CPP;
    let group_guid = group.group_guid;
    let registry = Arc::new(GroupRegistry::default());
    registry.register_group_like_cpp(group_guid, group);
    session.set_group_registry(registry, Arc::new(PendingInvites::default()));
    let native_group_guid = ObjectGuid::create_group(group_guid);
    {
        let mut manager = canonical.lock().unwrap();
        manager
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(viewer_guid)
            .unwrap()
            .set_group_like_cpp(Some(wow_entities::PlayerGroupState {
                group_guid: native_group_guid,
                leader_guid: viewer_guid,
                role_mask: 0,
                subgroup: 0,
            }));
        manager
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(source_guid)
            .unwrap()
            .set_group_like_cpp(Some(wow_entities::PlayerGroupState {
                group_guid: native_group_guid,
                leader_guid: viewer_guid,
                role_mask: 0,
                subgroup: 0,
            }));
        let _ = manager.update(1);
    }
    let _ = session
        .send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp();
    let _ = drain_server_packet_bytes(&send_rx);
    session
        .core
        .client_visible_guids_like_cpp
        .insert(source_guid);

    {
        let mut manager = canonical.lock().unwrap();
        let source = manager
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(source_guid)
            .unwrap();
        assert!(source.set_quest_slot_like_cpp(0, 59_416));
        let _ = manager.update(1);
        let summary = manager
            .find_map(571, 0)
            .unwrap()
            .last_send_object_updates_summary_like_cpp();
        let update = summary
            .player_values_updates
            .iter()
            .find(|update| update.guid == source_guid)
            .expect("Map captured the current group's native QuestLog row");
        assert_eq!(
            update.values_update.player_data.as_ref().unwrap().quest_log[0]
                .values()
                .quest_id(),
            59_416,
        );
    }
    assert_eq!(
        session
            .send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(
            ),
        1,
        "PARTY_MEMBER visibility includes a current instance-group member"
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::UpdateObject]
    );

    {
        let mut manager = canonical.lock().unwrap();
        let source = manager
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(source_guid)
            .unwrap();
        source.set_group_like_cpp(Some(wow_entities::PlayerGroupState {
            group_guid: ObjectGuid::create_group(group_guid + 1),
            leader_guid: source_guid,
            role_mask: 0,
            subgroup: 0,
        }));
        assert!(source.set_quest_slot_counter_like_cpp(0, 3, 1));
        let _ = manager.update(1);
    }
    assert_eq!(
        session
            .send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(
            ),
        0,
        "a stale registry membership cannot expose the row after native group identity changes"
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
    session
        .core
        .client_visible_guids_like_cpp
        .insert(creature_guid);

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
