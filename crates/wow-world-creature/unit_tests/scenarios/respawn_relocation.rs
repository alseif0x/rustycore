use super::*;

#[test]
fn blocked_patrol_starts_its_default_route_again_after_respawn_relocation() {
    check_relocated_patrol(43, Some(4.0));
}

#[test]
fn blocked_patrol_resolves_the_current_path_id_after_respawn_relocation() {
    check_relocated_patrol(44, Some(12.0));
}

#[test]
fn blocked_patrol_with_a_missing_default_route_stays_stopped_after_respawn_relocation() {
    check_relocated_patrol(45, None);
}

fn check_relocated_patrol(current_path_id: u32, expected_first_offset: Option<f32>) {
    let guid = test_creature_guid(200_027);
    let mut creature = make_test_world_creature(guid);
    let home = creature.position();
    let first = wow_movement::WaypointNode::new(1, home.x + 4.0, home.y, home.z);
    let second = wow_movement::WaypointNode::new(2, home.x + 8.0, home.y, home.z);
    let route = wow_movement::WaypointPath::new(43, vec![first, second]);
    creature.load_path_like_cpp(route.id);
    let config = MMapRuntimeConfigLikeCpp {
        enabled: false,
        ..Default::default()
    };
    let lookups = std::sync::atomic::AtomicUsize::new(0);
    let resolve = |id| {
        assert_eq!(id, current_path_id);
        lookups.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        match id {
            43 => Some(route.clone()),
            44 => Some(wow_movement::WaypointPath::new(
                id,
                vec![wow_movement::WaypointNode::new(
                    1,
                    home.x + 12.0,
                    home.y,
                    home.z,
                )],
            )),
            _ => None,
        }
    };
    let step = |creature: &mut wow_entities::Creature, diff| {
        crate::step_creature_movement_on_view_like_cpp(
            &mut crate::creature_movement::CreatureMovementLikeCpp::canonical_like_cpp(creature),
            guid,
            &config,
            None,
            &resolve,
            None,
            None,
            diff,
        )
    };
    crate::creature_movement::CreatureMovementLikeCpp::canonical_like_cpp(&mut creature)
        .initialize_default_waypoint_movement_like_cpp(Some(route.clone()));
    let initial_delay = wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32;
    assert!(step(&mut creature, initial_delay).is_some());
    let first_leg = creature
        .active_move_spline_like_cpp()
        .unwrap()
        .duration_ms() as u32;
    for diff in [first_leg + 1, 1, 1] {
        step(&mut creature, diff);
    }
    assert_eq!(creature.move_target(), Some(second.position));
    assert_eq!(lookups.load(std::sync::atomic::Ordering::Relaxed), 0);
    creature.load_path_like_cpp(current_path_id);

    let canonical = shared_map_manager();
    let mut manager = canonical.lock().unwrap();
    assert!(manager.add_creature(0, 0, creature));
    let map = manager.find_map_mut(0, 0).unwrap().map_mut();
    map.ensure_grid_loaded(&wow_map::Cell::from_world(home.x, home.y));
    map.add_creature_to_move_list_like_cpp(guid, Position::xyz(3000.0, 3000.0, 0.0));
    assert_eq!(
        map.move_all_creatures_in_move_list_like_cpp()
            .respawn_relocated,
        1
    );
    let creature = map.get_typed_creature_mut(guid).unwrap();
    assert_eq!(creature.position(), home);
    assert!(creature.active_move_spline_like_cpp().is_none());
    assert!(step(creature, initial_delay - 1).is_none());
    let movement = step(creature, 1);
    if let Some(offset) = expected_first_offset {
        let bytes = movement.expect("relocated patrol must launch its first leg again");
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            wow_constants::ServerOpcodes::OnMonsterMove as u16
        );
        assert_eq!(
            creature.move_target(),
            Some(Position::xyz(home.x + offset, home.y, home.z))
        );
    } else {
        assert!(movement.is_none());
        assert!(step(creature, 2 * initial_delay).is_none());
        assert!(creature.move_target().is_none());
    }
    assert_eq!(lookups.load(std::sync::atomic::Ordering::Relaxed), 1);
}
