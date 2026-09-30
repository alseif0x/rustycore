//! Original world-entity scenarios; inputs, bodies and assertions retained.

use super::*;

#[test]
fn gameobject_interaction_resolves_canonical_map_object_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(777, 9);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        2,
        1,
        10,
        0,
    ));

    let mut gameobject = GameObject::new();
    gameobject.world_mut().object_mut().create(gameobject_guid);
    gameobject.world_mut().object_mut().set_entry(777);
    gameobject.world_mut().set_map(571, 0).unwrap();
    gameobject
        .world_mut()
        .relocate(Position::new(14.9, 0.0, 0.0, 0.0));
    gameobject.world_mut().object_mut().add_to_world();

    canonical
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();

    assert_eq!(
        session.represented_gameobject_can_interact_with_like_cpp(gameobject_guid, 5.0),
        Some(RepresentedGameObjectAccessLikeCpp {
            entry: 777,
            position: Position::new(14.9, 0.0, 0.0, 0.0),
        })
    );

    session.set_player_position_like_cpp(Position::new(20.1, 0.0, 0.0, 0.0));
    assert_eq!(
        session.represented_gameobject_can_interact_with_like_cpp(gameobject_guid, 5.0),
        None
    );
}
#[test]
fn visible_gameobjects_use_canonical_map_cells_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);
    let visible_guid = test_gameobject_guid(910, 30);
    let far_guid = test_gameobject_guid(911, 31);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    canonical.lock().unwrap().create_world_map(571, 0);

    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        visible_guid,
        910,
        Position::new(20.0, 20.0, 0.0, 0.0),
        3,
    );
    session.record_represented_gameobject_display_model_like_cpp(
        visible_guid,
        7000,
        1.5,
        [0.0, 0.0, 0.0, 1.0],
    );
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        far_guid,
        911,
        Position::new(5000.0, 5000.0, 0.0, 0.0),
        3,
    );
    session.record_represented_gameobject_display_model_like_cpp(
        far_guid,
        7001,
        1.0,
        [0.0, 0.0, 0.0, 1.0],
    );

    let visible = session
        .visible_gameobjects_from_canonical_map_like_cpp(571, &player_position, 800.0)
        .expect("canonical map");

    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].guid, visible_guid);
    assert_eq!(visible[0].entry, 910);
    assert_eq!(visible[0].display_id, 7000);
    assert_eq!(visible[0].go_type, 3);
    assert_eq!(visible[0].scale, 1.5);
}
