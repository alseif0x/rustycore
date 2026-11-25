use super::*;

#[test]
fn area_phase_uses_class_and_live_event_and_clears_on_exit() {
    const MAP: u32 = 571;
    const PHASE: u32 = 70;
    const AREA: u32 = 101;
    const EVENT: u32 = 86;
    let (mut session, _, receiver) = make_session();
    let manager = shared_canonical_map_manager();
    let guid = ObjectGuid::create_player(1, 49_620);
    session.set_canonical_map_manager(Arc::clone(&manager));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        guid,
        "PhaseObserver".to_string(),
        Position::ZERO,
        MAP as u16,
        1,
        8,
        80,
        0,
    ));
    let handle = {
        let mut manager = manager.lock().unwrap();
        manager.create_world_map(MAP, 0);
        let mut player = Box::new(wow_entities::Player::new(Some(1), false));
        player.unit_mut().world_mut().object_mut().create(guid);
        player.unit_mut().set_max_health(100);
        player.unit_mut().set_health(100);
        player.set_race_class_gender(1, 8, wow_constants::Gender::Male);
        let handle = manager.install_detached_player_like_cpp(player).unwrap();
        manager
            .attach_player_like_cpp(handle, wow_map::MapKey::new(MAP, 0), Position::ZERO)
            .unwrap();
        handle
    };
    session.core.player_handle_like_cpp = Some(handle);
    wow_world_entities::apply_move_init_active_mover_complete_like_cpp(&mut session, 0);

    let areas = Arc::new(wow_data::AreaTableStore::from_entries(
        [AREA, AREA + 1].map(|id| wow_data::AreaTableEntry {
            id,
            continent_id: MAP as u16,
            parent_area_id: 0,
            area_bit: 0,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        }),
    ));
    let phases = Arc::new(wow_data::PhaseStore::from_entries([wow_data::PhaseEntry {
        id: PHASE,
        flags: 0,
    }]));
    let mut rules = wow_data::PhaseInfoStore::from_phase_store_like_cpp(&phases);
    rules.load_area_phases_from_rows_like_cpp(&areas, &phases, [(AREA, PHASE)]);
    let conditions = Arc::new(
        wow_data::ConditionEntriesByTypeStore::from_conditions_like_cpp(
            [
                (wow_constants::ConditionType::Class, 128),
                (wow_constants::ConditionType::ActiveEvent, EVENT),
            ]
            .map(|(condition_type, condition_value1)| wow_data::Condition {
                source_type: wow_constants::ConditionSourceType::Phase,
                source_group: PHASE,
                source_entry: AREA as i32,
                condition_type,
                condition_value1,
                ..Default::default()
            }),
        ),
    );
    rules.attach_phase_conditions_like_cpp(&conditions);
    let active = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let source = Arc::clone(&active);
    session.set_player_area_phases_like_cpp(Arc::new(
        wow_world_core::phasing::PlayerAreaPhaseCatalogLikeCpp::new(rules, move || {
            Some(
                source
                    .load(std::sync::atomic::Ordering::Relaxed)
                    .then_some(EVENT)
                    .into_iter()
                    .collect(),
            )
        }),
    ));
    session.set_condition_store(conditions);
    session.set_phase_group_store(Arc::new(wow_data::PhaseGroupStore::from_entries(
        &phases,
        [],
    )));
    session.set_phase_store(phases);
    session.set_area_table_store(areas);
    let _ = drain_server_packet_bytes(&receiver);
    let has_phase = |session: &crate::session::WorldSession| {
        session
            .core
            .with_owned_player_like_cpp(|player| {
                player
                    .unit()
                    .world()
                    .phase_shift()
                    .has_phase_like_cpp(PHASE)
            })
            .unwrap()
    };

    wow_world_instances::update_area_represented_with_rest_update_like_cpp(
        &mut session,
        AREA,
        false,
    );
    assert!(!has_phase(&session));
    active.store(true, std::sync::atomic::Ordering::Relaxed);
    wow_world_instances::update_area_represented_with_rest_update_like_cpp(
        &mut session,
        AREA,
        false,
    );
    assert!(has_phase(&session));
    wow_world_instances::update_area_represented_with_rest_update_like_cpp(
        &mut session,
        AREA + 1,
        false,
    );
    assert!(!has_phase(&session));
}

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
