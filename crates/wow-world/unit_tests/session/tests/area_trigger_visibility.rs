use super::*;

#[tokio::test]
async fn area_trigger_visibility_tracks_phase_instance_and_expiry() {
    const MAP: u32 = 571;
    const INSTANCE: u32 = 7;
    const PHASE: u32 = 70;
    let position = Position::new(100.0, 200.0, 30.0, 0.0);
    let manager = shared_canonical_map_manager();
    let trigger_guid = test_area_trigger_guid(49_621, 49_621);
    let make_observer = |counter, instance, phase| {
        let (mut session, _, receiver) = make_session();
        let guid = ObjectGuid::create_player(1, counter);
        session.set_canonical_map_manager(Arc::clone(&manager));
        session.attach_player_controller_like_cpp(SessionPlayerController::new(
            guid,
            format!("Observer{counter}"),
            position,
            MAP as u16,
            1,
            1,
            80,
            0,
        ));
        add_canonical_test_player_on_map(&manager, guid, position, MAP, instance);
        session.set_represented_player_phase_shift_like_cpp(PhaseShift::from_phases([phase]));
        wow_world_entities::apply_move_init_active_mover_complete_like_cpp(&mut session, 0);
        let _ = drain_server_packet_bytes(&receiver);
        (session, receiver)
    };
    let (mut eligible, _eligible_packets) = make_observer(49_622, INSTANCE, PHASE);
    let (mut wrong_phase, _other_phase_packets) = make_observer(49_623, INSTANCE, PHASE + 1);
    let (mut wrong_instance, _other_instance_packets) = make_observer(49_624, INSTANCE + 1, PHASE);

    {
        let mut trigger = wow_entities::AreaTrigger::new();
        let world = trigger.world_mut();
        world.object_mut().create(trigger_guid);
        world.object_mut().set_entry(49_621);
        world.set_map(MAP, INSTANCE).unwrap();
        world.relocate(position);
        *world.phase_shift_mut() = PhaseShift::from_phases([PHASE]);
        trigger.set_duration(1000);

        let mut manager = manager.lock().unwrap();
        let map = manager.find_map_mut(MAP, INSTANCE).unwrap().map_mut();
        map.add_to_map_like_cpp(AccessorObjectKind::AreaTrigger, trigger.world().clone())
            .unwrap();
        trigger.world_mut().object_mut().add_to_world();
        map.insert_map_object_record(
            wow_entities::MapObjectRecord::new_area_trigger(trigger).unwrap(),
        )
        .unwrap();
    }

    let sees_trigger = |session: &crate::session::WorldSession| {
        session
            .core
            .client_visible_guids_like_cpp
            .contains(&trigger_guid)
    };
    eligible.force_update_visibility_like_cpp().await;
    wrong_phase.force_update_visibility_like_cpp().await;
    wrong_instance.force_update_visibility_like_cpp().await;
    assert!(sees_trigger(&eligible));
    assert!(!sees_trigger(&wrong_phase));
    assert!(!sees_trigger(&wrong_instance));

    wrong_phase.set_represented_player_phase_shift_like_cpp(PhaseShift::from_phases([PHASE]));
    wrong_phase.force_update_visibility_like_cpp().await;
    assert!(sees_trigger(&wrong_phase));

    assert!(manager.lock().unwrap().update(1000).is_some());
    assert!(
        manager
            .lock()
            .unwrap()
            .find_map(MAP, INSTANCE)
            .unwrap()
            .map()
            .with_area_trigger_like_cpp(trigger_guid, |_| ())
            .is_none()
    );
    eligible.force_update_visibility_like_cpp().await;
    wrong_phase.force_update_visibility_like_cpp().await;
    assert!(!sees_trigger(&eligible));
    assert!(!sees_trigger(&wrong_phase));
}
