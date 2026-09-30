//! Original world-entity scenarios; inputs, bodies and assertions retained.

use super::*;

#[test]
fn visible_world_creatures_use_map_grid_and_phase_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 26_820);
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);
    let visible_guid = test_creature_guid(20);
    let far_guid = test_creature_guid(21);
    let visible_pos = Position::new(20.0, 20.0, 0.0, 0.0);
    let far_pos = Position::new(5000.0, 5000.0, 0.0, 0.0);

    session.set_map_manager(Arc::clone(&manager));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "VisibilityOwner".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical viewer map");
    for (guid, position) in [(visible_guid, visible_pos), (far_guid, far_pos)] {
        let (grid_x, grid_y) = crate::map_manager::world_to_grid_coords(position.x, position.y);
        manager.write().unwrap().add_creature(
            571,
            0,
            grid_x,
            grid_y,
            crate::map_manager::WorldCreature::new(
                guid, 900, position, 100, 80, 1, 2, 0.0, 1, 35, 0, 0,
            ),
        );
    }

    let visible = session.visible_world_creatures_from_map_like_cpp(571, &player_position);

    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].guid(), visible_guid);
}
#[test]
fn visible_world_creatures_prefer_legacy_runtime_duplicate_with_active_spline_like_cpp() {
    use wow_constants::movement::MovementFlag;

    let (mut session, _pkt_tx, _send_rx) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 76_010);
    let creature_guid = test_creature_guid(76_011);
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);
    let creature_position = Position::new(20.0, 20.0, 0.0, 0.0);

    session.set_map_manager(Arc::clone(&manager));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "VisibleCreatureViewer".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical viewer map");
    add_canonical_test_creature_on_map_with_world_state(
        &canonical,
        creature_guid,
        76_011,
        creature_position,
        0,
        571,
        0,
        true,
    );

    let mut legacy_creature = crate::map_manager::WorldCreature::new(
        creature_guid,
        76_011,
        creature_position,
        100,
        12,
        1,
        2,
        0.0,
        1,
        35,
        0,
        0,
    );
    legacy_creature
        .creature
        .unit_mut()
        .world_mut()
        .set_map(571, 0)
        .unwrap();
    legacy_creature
        .creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    legacy_creature
        .begin_move_spline_like_cpp(Position::new(24.0, 20.0, 0.0, 0.0))
        .expect("legacy runtime spline must launch");
    let (grid_x, grid_y) =
        crate::map_manager::world_to_grid_coords(creature_position.x, creature_position.y);
    manager
        .write()
        .unwrap()
        .add_creature(571, 0, grid_x, grid_y, legacy_creature);

    let visible = session.visible_world_creatures_from_map_like_cpp(571, &player_position);

    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].guid(), creature_guid);
    assert!(
        visible[0].create().active_move_spline().is_some(),
        "C++ has one map Creature; Rust visibility must keep the legacy active MoveSpline for duplicate canonical/legacy GUIDs"
    );
    assert!(
        MovementFlag::from_bits_retain(visible[0].create().create_data().movement_flags)
            .contains(MovementFlag::FORWARD),
        "active legacy spline should carry C++ MoveSplineInit::Launch movement flags into CREATE"
    );
}
#[test]
fn active_creature_update_guids_use_cpp_cell_activation_area() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let manager = shared_map_manager();
    let active_guid = test_creature_guid(40);
    let inactive_guid = test_creature_guid(41);

    session.set_map_manager(Arc::clone(&manager));
    session.set_player_map_position_like_cpp(571, Position::new(0.0, 0.0, 0.0, 0.0));

    for (guid, position) in [
        (active_guid, Position::new(40.0, 40.0, 0.0, 0.0)),
        (inactive_guid, Position::new(260.0, 260.0, 0.0, 0.0)),
    ] {
        let (grid_x, grid_y) = crate::map_manager::world_to_grid_coords(position.x, position.y);
        manager.write().unwrap().add_creature(
            571,
            0,
            grid_x,
            grid_y,
            crate::map_manager::WorldCreature::new(
                guid, 900, position, 100, 80, 1, 2, 0.0, 1, 35, 0, 0,
            ),
        );
    }

    let active = session.active_world_creature_guids_for_update_like_cpp();

    assert_eq!(active, vec![active_guid]);
}
#[test]
fn creature_message_to_set_gate_requires_have_at_client_phase_map_and_range_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let guid = test_creature_guid(9301);
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);
    let visible_position = Position::new(20.0, 20.0, 0.0, 0.0);
    let far_position = Position::new(5000.0, 5000.0, 0.0, 0.0);

    session.set_player_map_position_like_cpp(571, player_position);
    session.set_represented_player_phase_shift_like_cpp(PhaseShift::from_phases([10]));

    let mut creature = crate::map_manager::WorldCreature::new(
        guid,
        900,
        visible_position,
        100,
        80,
        1,
        2,
        0.0,
        1,
        35,
        0,
        0,
    );
    let _ = creature.creature.unit_mut().world_mut().set_map(571, 0);
    creature
        .creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    *creature.creature.unit_mut().world_mut().phase_shift_mut() = PhaseShift::from_phases([10]);

    assert!(
        !session.represented_can_receive_creature_message_to_set_like_cpp(guid, &creature, false),
        "C++ MessageDistDeliverer::SendPacket requires HaveAtClient"
    );

    session.client_visible_guids_like_cpp.insert(guid);
    assert!(
        session.represented_can_receive_creature_message_to_set_like_cpp(guid, &creature, false),
        "same phase + in range + HaveAtClient must receive"
    );

    let mut wrong_phase = creature.clone();
    *wrong_phase
        .creature
        .unit_mut()
        .world_mut()
        .phase_shift_mut() = PhaseShift::from_phases([20]);
    assert!(
        !session.represented_can_receive_creature_message_to_set_like_cpp(
            guid,
            &wrong_phase,
            false
        ),
        "MessageDistDeliverer rejects players outside source phase"
    );

    let mut far_creature = creature.clone();
    far_creature.creature.set_ai_position(far_position);
    assert!(
        !session.represented_can_receive_creature_message_to_set_like_cpp(
            guid,
            &far_creature,
            false
        ),
        "MessageDistDeliverer applies source GetVisibilityRange distance"
    );

    let mut wrong_map = crate::map_manager::WorldCreature::new(
        test_creature_guid(9302),
        900,
        visible_position,
        100,
        80,
        1,
        2,
        0.0,
        1,
        35,
        0,
        0,
    );
    let _ = wrong_map.creature.unit_mut().world_mut().set_map(530, 0);
    wrong_map
        .creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    *wrong_map.creature.unit_mut().world_mut().phase_shift_mut() = PhaseShift::from_phases([10]);
    session
        .client_visible_guids_like_cpp
        .insert(wrong_map.guid());
    assert!(
        !session.represented_can_receive_creature_message_to_set_like_cpp(
            wrong_map.guid(),
            &wrong_map,
            false
        ),
        "Cell::VisitWorldObjects cannot deliver across maps"
    );
}
#[test]
fn visible_creatures_skip_not_in_world_canonical_objects_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 9303);
    let in_world_guid = test_creature_guid(9304);
    let removed_guid = test_creature_guid(9305);
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "VisibleCreatureViewer".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_represented_player_phase_shift_like_cpp(PhaseShift::from_phases([10]));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
    add_canonical_test_creature_indexed_on_map_with_level(
        &canonical,
        in_world_guid,
        9304,
        Position::new(20.0, 20.0, 0.0, 0.0),
        571,
        0,
        80,
    );
    add_canonical_test_creature_indexed_on_map_with_level(
        &canonical,
        removed_guid,
        9305,
        Position::new(21.0, 21.0, 0.0, 0.0),
        571,
        0,
        80,
    );
    let pet_guid = test_pet_guid(9306);
    add_canonical_test_pet(
        &canonical,
        pet_guid,
        player_guid,
        9307,
        Position::new(22.0, 22.0, 0.0, 0.0),
        0,
    );
    {
        let mut guard = canonical.lock().unwrap();
        let map = guard.find_map_mut(571, 0).unwrap().map_mut();
        *map.get_typed_player_mut(player_guid)
            .unwrap()
            .unit_mut()
            .world_mut()
            .phase_shift_mut() = PhaseShift::from_phases([10]);
        *map.get_typed_creature_mut(in_world_guid)
            .unwrap()
            .unit_mut()
            .world_mut()
            .phase_shift_mut() = PhaseShift::from_phases([10]);
        map.get_typed_creature_mut(in_world_guid)
            .unwrap()
            .unit_mut()
            .world_mut()
            .object_mut()
            .add_to_world();
        map.get_typed_creature_mut(removed_guid)
            .unwrap()
            .unit_mut()
            .world_mut()
            .object_mut()
            .remove_from_world();
        *map.get_typed_pet_mut(pet_guid)
            .unwrap()
            .creature_mut()
            .unit_mut()
            .world_mut()
            .phase_shift_mut() = PhaseShift::from_phases([10]);
    }

    let visible = session
        .visible_creatures_from_canonical_map_like_cpp(571, &player_position)
        .expect("canonical map");

    assert_eq!(visible.len(), 2);
    assert!(visible.iter().any(|creature| creature.guid() == pet_guid));
}
