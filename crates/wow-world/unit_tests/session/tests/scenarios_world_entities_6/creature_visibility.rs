use super::*;

struct Fixture {
    session: crate::session::WorldSession,
    manager: SharedCanonicalMapManager,
    player_guid: ObjectGuid,
    body_position: Position,
}

fn fixture(counter: i64, body_position: Position) -> Fixture {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let player_guid = ObjectGuid::create_player(1, counter);
    session.set_canonical_map_manager(Arc::clone(&manager));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "VisibilityQueryOwner".to_string(),
        body_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&manager, player_guid, body_position, 571, 0);
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
    let mut fixture = Fixture {
        session,
        manager,
        player_guid,
        body_position,
    };
    set_player_visibility(&mut fixture, false, 0x1, 0x1);
    fixture
}

fn set_player_visibility(fixture: &mut Fixture, dead: bool, visibility: u32, detect: u32) {
    let handle = fixture
        .session
        .core
        .player_handle_like_cpp
        .expect("canonical Player fixture handle");
    fixture
        .manager
        .lock()
        .expect("canonical manager")
        .with_player_mut_like_cpp(handle, |player| {
            player.unit_mut().set_max_health(100);
            player.unit_mut().set_health(if dead { 1 } else { 100 });
            player.unit_mut().set_death_state(if dead {
                wow_constants::DeathState::Dead
            } else {
                wow_constants::DeathState::Alive
            });
            player
                .unit_mut()
                .set_server_side_ghost_visibility_like_cpp(visibility);
            player
                .unit_mut()
                .set_server_side_ghost_visibility_detect_like_cpp(detect);
        })
        .expect("current canonical Player");
}

fn add_creature(
    fixture: &Fixture,
    counter: i64,
    position: Position,
    visibility: u32,
) -> ObjectGuid {
    let guid = test_creature_guid(counter);
    add_canonical_test_creature_indexed_on_map_with_level(
        &fixture.manager,
        guid,
        u32::try_from(counter).expect("test entry id"),
        position,
        571,
        0,
        80,
    );
    fixture
        .manager
        .lock()
        .expect("canonical manager")
        .find_map_mut(571, 0)
        .expect("canonical map")
        .map_mut()
        .get_typed_creature_mut(guid)
        .expect("canonical Creature")
        .unit_mut()
        .set_server_side_ghost_visibility_like_cpp(visibility);
    guid
}

fn add_corpse(fixture: &Fixture, counter: i64, position: Position, instance_id: u32) -> ObjectGuid {
    let guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Corpse, 0, 1, 571, 0, 0, counter);
    let mut corpse = wow_entities::Corpse::new_at(wow_entities::CorpseType::ResurrectablePve, 0);
    corpse.world_mut().object_mut().create(guid);
    corpse
        .world_mut()
        .set_map(571, instance_id)
        .expect("corpse map");
    corpse.world_mut().relocate(position);
    corpse.set_owner_guid(fixture.player_guid);
    fixture
        .manager
        .lock()
        .expect("canonical manager")
        .create_world_map(571, instance_id)
        .map_mut()
        .add_map_object_record_to_map_like_cpp(
            wow_entities::MapObjectRecord::new_corpse(corpse).expect("corpse record"),
        )
        .expect("map corpse insertion");
    guid
}

fn query(fixture: &Fixture, caller_position: Position) -> Vec<wow_entities::Creature> {
    wow_world_entities::visible_creatures_from_canonical_map_like_cpp(
        &fixture.session,
        571,
        &caller_position,
    )
    .expect("active canonical Player and map")
}

fn visible(query: &[wow_entities::Creature], guid: ObjectGuid) -> bool {
    query.iter().any(|creature| creature.guid() == guid)
}

fn search_radius_and_sight(fixture: &Fixture) -> (f32, f32) {
    let handle = fixture.session.core.player_handle_like_cpp.unwrap();
    let manager = fixture.manager.lock().expect("canonical manager");
    manager
        .with_player_like_cpp(handle, |player| {
            let map = manager.find_map(571, 0).expect("canonical map").map();
            let viewpoint = player.unit().world();
            let sight = viewpoint.get_sight_range(None, map);
            (sight + viewpoint.combat_reach(), sight)
        })
        .expect("canonical Player")
}

fn assert_guid_is_in_visited_cells(
    fixture: &Fixture,
    guid: ObjectGuid,
    center: Position,
    radius: f32,
) {
    let manager = fixture.manager.lock().expect("canonical manager");
    let map = manager.find_map(571, 0).expect("canonical map").map();
    let nearby = map.nearby_cell_guids_like_cpp(center.x, center.y, radius);
    assert!(
        nearby.world.creatures.contains(&guid) || nearby.grid.creatures.contains(&guid),
        "the candidate must be inside the visited-cell set"
    );
}

#[test]
fn ghost_masks_select_ordinary_healer_and_combined_creatures() {
    let mut fixture = fixture(95_001, Position::new(0.0, 0.0, 0.0, 0.0));
    set_player_visibility(&mut fixture, true, 0x2, 0x2);
    let ordinary = add_creature(&fixture, 95_002, Position::new(20.0, 0.0, 0.0, 0.0), 0x1);
    let healer = add_creature(&fixture, 95_003, Position::new(22.0, 0.0, 0.0, 0.0), 0x2);
    let both = add_creature(&fixture, 95_004, Position::new(24.0, 0.0, 0.0, 0.0), 0x3);

    let without_corpse = query(&fixture, fixture.body_position);
    assert!(!visible(&without_corpse, ordinary));
    assert!(visible(&without_corpse, healer));
    assert!(visible(&without_corpse, both));

    // C++ GetCorpse lookup and the corpse distance check do not add an
    // IsInWorld gate. Clear that flag explicitly while retaining the map record.
    let corpse = add_corpse(&fixture, 95_005, Position::new(10.0, 0.0, 0.0, 0.0), 0);
    {
        let mut manager = fixture.manager.lock().expect("canonical manager");
        let map = manager
            .find_map_mut(571, 0)
            .expect("canonical map")
            .map_mut();
        map.get_typed_corpse_mut(corpse)
            .expect("indexed corpse")
            .world_mut()
            .object_mut()
            .remove_from_world();
        assert_eq!(
            map.corpse_guid_by_player_like_cpp(fixture.player_guid),
            Some(corpse)
        );
        assert!(
            !map.get_typed_corpse(corpse)
                .expect("corpse remains indexed")
                .world()
                .object()
                .is_in_world()
        );
    }
    let with_corpse = query(&fixture, fixture.body_position);
    assert!(visible(&with_corpse, ordinary));
    assert!(visible(&with_corpse, healer));
    assert!(visible(&with_corpse, both));
}

#[test]
fn corpse_distances_grant_ghost_visibility_but_corpse_presence_skips_direct_range() {
    let mut positive = fixture(95_011, Position::new(0.0, 0.0, 0.0, 0.0));
    set_player_visibility(&mut positive, true, 0x2, 0x2);
    let (search_radius, sight) = search_radius_and_sight(&positive);
    let target_position = Position::new(sight + 5.0, 0.0, 0.0, 0.0);
    let target = add_creature(&positive, 95_012, target_position, 0x1);
    let corpse_position = Position::new(sight / 2.0, 0.0, 0.0, 0.0);
    add_corpse(&positive, 95_013, corpse_position, 0);
    assert_guid_is_in_visited_cells(&positive, target, positive.body_position, search_radius);

    let manager = positive.manager.lock().expect("canonical manager");
    let map = manager.find_map(571, 0).expect("canonical map").map();
    let player_and_target_distance = manager
        .with_player_like_cpp(
            positive.session.core.player_handle_like_cpp.unwrap(),
            |player| {
                player.unit().world().is_within_dist(
                    map.get_typed_creature(target).unwrap().unit().world(),
                    sight,
                    false,
                    true,
                    true,
                )
            },
        )
        .expect("canonical Player");
    drop(manager);
    assert!(
        !player_and_target_distance,
        "target exceeds direct sight range"
    );
    let selected = query(&positive, positive.body_position);
    assert!(
        visible(&selected, target),
        "corpse and target are both in range"
    );

    let mut negative = fixture(95_021, Position::new(0.0, 0.0, 0.0, 0.0));
    set_player_visibility(&mut negative, true, 0x2, 0x2);
    let (search_radius, sight) = search_radius_and_sight(&negative);
    let target_position = Position::new(sight + 5.0, 0.0, 0.0, 0.0);
    let target = add_creature(&negative, 95_022, target_position, 0x1);
    add_corpse(&negative, 95_023, Position::new(-2_000.0, 0.0, 0.0, 0.0), 0);
    assert_guid_is_in_visited_cells(&negative, target, negative.body_position, search_radius);
    assert!(!visible(&query(&negative, negative.body_position), target));

    let mut corpse_presence = fixture(95_031, Position::new(0.0, 0.0, 0.0, 0.0));
    set_player_visibility(&mut corpse_presence, true, 0x2, 0x2);
    let (search_radius, sight) = search_radius_and_sight(&corpse_presence);
    let target_position = Position::new(sight + 5.0, 0.0, 0.0, 0.0);
    let gm_target = add_creature(&corpse_presence, 95_032, target_position, 0x1);
    corpse_presence
        .manager
        .lock()
        .expect("canonical manager")
        .find_map_mut(571, 0)
        .expect("canonical map")
        .map_mut()
        .get_typed_creature_mut(gm_target)
        .expect("GM Creature")
        .unit_mut()
        .set_server_side_gm_visibility_like_cpp(1);
    let handle = corpse_presence.session.core.player_handle_like_cpp.unwrap();
    corpse_presence
        .manager
        .lock()
        .expect("canonical manager")
        .with_player_mut_like_cpp(handle, |player| {
            player
                .unit_mut()
                .set_server_side_gm_visibility_detect_like_cpp(1);
        })
        .expect("canonical Player");
    add_corpse(
        &corpse_presence,
        95_033,
        Position::new(-2_000.0, 0.0, 0.0, 0.0),
        0,
    );
    assert_guid_is_in_visited_cells(
        &corpse_presence,
        gm_target,
        corpse_presence.body_position,
        search_radius,
    );
    assert!(
        visible(
            &query(&corpse_presence, corpse_presence.body_position),
            gm_target
        ),
        "corpse presence bypasses direct range even when corpseVisibility is false"
    );
}

#[test]
fn corpse_visibility_uses_body_distance_while_normal_range_uses_farsight() {
    let mut body_far = fixture(95_035, Position::new(0.0, 0.0, 0.0, 0.0));
    set_player_visibility(&mut body_far, true, 0x2, 0x2);
    let view_guid = test_dynamic_object_guid(95_036, 95_036);
    let view_position = Position::new(400.0, 0.0, 0.0, 0.0);
    add_canonical_test_dynamic_object_on_map(
        &body_far.manager,
        view_guid,
        body_far.player_guid,
        95_036,
        view_position,
        571,
        0,
    );
    let target = add_creature(&body_far, 95_037, Position::new(420.0, 0.0, 0.0, 0.0), 0x1);
    add_corpse(&body_far, 95_038, Position::new(410.0, 0.0, 0.0, 0.0), 0);
    let handle = body_far.session.core.player_handle_like_cpp.unwrap();
    body_far
        .manager
        .lock()
        .expect("canonical manager")
        .with_player_mut_like_cpp(handle, |player| {
            player.set_farsight_object_like_cpp(view_guid);
        })
        .expect("canonical Player");
    assert!(
        !visible(&query(&body_far, body_far.body_position), target),
        "the viewpoint and corpse are near the target, but the corpse is too far from the dead Player"
    );

    let mut view_far = fixture(95_041, Position::new(0.0, 0.0, 0.0, 0.0));
    set_player_visibility(&mut view_far, true, 0x2, 0x2);
    let (_, sight) = search_radius_and_sight(&view_far);
    let target_position = Position::new(sight + 5.0, 0.0, 0.0, 0.0);
    let target = add_creature(&view_far, 95_042, target_position, 0x1);
    let view_position = Position::new(target_position.x + sight + 5.0, 0.0, 0.0, 0.0);
    let view_guid = test_dynamic_object_guid(95_043, 95_043);
    add_canonical_test_dynamic_object_on_map(
        &view_far.manager,
        view_guid,
        view_far.player_guid,
        95_043,
        view_position,
        571,
        0,
    );
    add_corpse(&view_far, 95_044, Position::new(20.0, 0.0, 0.0, 0.0), 0);
    let handle = view_far.session.core.player_handle_like_cpp.unwrap();
    view_far
        .manager
        .lock()
        .expect("canonical manager")
        .with_player_mut_like_cpp(handle, |player| {
            player.set_farsight_object_like_cpp(view_guid);
        })
        .expect("canonical Player");

    let manager = view_far.manager.lock().expect("canonical manager");
    let map = manager.find_map(571, 0).expect("canonical map").map();
    let viewpoint = map
        .get_typed_dynamic_object(view_guid)
        .expect("canonical viewpoint")
        .world();
    let target_world = map
        .get_typed_creature(target)
        .expect("canonical target")
        .unit()
        .world();
    assert!(
        !viewpoint.is_within_dist(target_world, sight, false, true, true),
        "the far-sight viewpoint is outside the exact direct range"
    );
    let view_search_radius = sight + viewpoint.combat_reach();
    let nearby = map.nearby_cell_guids_like_cpp(
        viewpoint.position().x,
        viewpoint.position().y,
        view_search_radius,
    );
    assert!(
        nearby.world.creatures.contains(&target) || nearby.grid.creatures.contains(&target),
        "the target remains in the viewpoint's visited cell set"
    );
    drop(manager);
    assert!(
        visible(&query(&view_far, view_far.body_position), target),
        "the Player body and corpse distance admit the target despite the distant viewpoint"
    );
}

#[test]
fn absent_or_remote_instance_corpse_keeps_normal_distance_admission() {
    let mut no_corpse = fixture(95_041, Position::new(0.0, 0.0, 0.0, 0.0));
    let (search_radius, sight) = search_radius_and_sight(&no_corpse);
    let near = add_creature(&no_corpse, 95_042, Position::new(20.0, 0.0, 0.0, 0.0), 0x1);
    let far = add_creature(
        &no_corpse,
        95_043,
        Position::new(sight + 5.0, 0.0, 0.0, 0.0),
        0x1,
    );
    assert_guid_is_in_visited_cells(&no_corpse, far, no_corpse.body_position, search_radius);
    let selected = query(&no_corpse, no_corpse.body_position);
    assert!(visible(&selected, near));
    assert!(!visible(&selected, far));

    let mut remote_corpse = fixture(95_051, Position::new(0.0, 0.0, 0.0, 0.0));
    set_player_visibility(&mut remote_corpse, true, 0x2, 0x2);
    let (search_radius, sight) = search_radius_and_sight(&remote_corpse);
    let target = add_creature(
        &remote_corpse,
        95_052,
        Position::new(sight + 5.0, 0.0, 0.0, 0.0),
        0x1,
    );
    add_corpse(
        &remote_corpse,
        95_053,
        Position::new(sight / 2.0, 0.0, 0.0, 0.0),
        7,
    );
    assert_guid_is_in_visited_cells(
        &remote_corpse,
        target,
        remote_corpse.body_position,
        search_radius,
    );
    assert!(!visible(
        &query(&remote_corpse, remote_corpse.body_position),
        target
    ));
}

#[test]
fn farsight_uses_only_a_live_canonical_viewpoint_and_falls_back_to_body() {
    let mut fixture = fixture(95_061, Position::new(0.0, 0.0, 0.0, 0.0));
    let viewpoint_guid = test_dynamic_object_guid(95_062, 95_062);
    let viewpoint_position = Position::new(400.0, 0.0, 0.0, 0.0);
    add_canonical_test_dynamic_object_on_map(
        &fixture.manager,
        viewpoint_guid,
        fixture.player_guid,
        95_062,
        viewpoint_position,
        571,
        0,
    );
    let view_target = add_creature(&fixture, 95_063, Position::new(420.0, 0.0, 0.0, 0.0), 0x1);
    let body_target = add_creature(&fixture, 95_064, Position::new(20.0, 0.0, 0.0, 0.0), 0x1);
    let handle = fixture.session.core.player_handle_like_cpp.unwrap();
    fixture
        .manager
        .lock()
        .expect("canonical manager")
        .with_player_mut_like_cpp(handle, |player| {
            player.set_farsight_object_like_cpp(viewpoint_guid);
        })
        .expect("canonical Player");

    let from_viewpoint = query(&fixture, fixture.body_position);
    assert!(visible(&from_viewpoint, view_target));
    assert!(!visible(&from_viewpoint, body_target));

    fixture
        .manager
        .lock()
        .expect("canonical manager")
        .find_map_mut(571, 0)
        .expect("canonical map")
        .map_mut()
        .get_typed_dynamic_object_mut(viewpoint_guid)
        .expect("canonical viewpoint")
        .world_mut()
        .object_mut()
        .remove_from_world();
    let stale_viewpoint = query(&fixture, viewpoint_position);
    assert!(!visible(&stale_viewpoint, view_target));
    assert!(visible(&stale_viewpoint, body_target));

    fixture
        .manager
        .lock()
        .expect("canonical manager")
        .with_player_mut_like_cpp(handle, |player| {
            player.set_farsight_object_like_cpp(test_dynamic_object_guid(95_065, 95_065));
        })
        .expect("canonical Player");
    let unresolved_viewpoint = query(&fixture, viewpoint_position);
    assert!(!visible(&unresolved_viewpoint, view_target));
    assert!(visible(&unresolved_viewpoint, body_target));
}

#[test]
fn phase_distance_and_vehicle_admission_precede_gm_visibility() {
    let mut fixture = fixture(95_071, Position::new(0.0, 0.0, 0.0, 0.0));
    let handle = fixture.session.core.player_handle_like_cpp.unwrap();
    fixture
        .manager
        .lock()
        .expect("canonical manager")
        .with_player_mut_like_cpp(handle, |player| {
            *player.unit_mut().world_mut().phase_shift_mut() = PhaseShift::from_phases([10]);
            player
                .unit_mut()
                .set_server_side_gm_visibility_detect_like_cpp(1);
        })
        .expect("canonical Player");

    let wrong_phase = add_creature(&fixture, 95_072, Position::new(20.0, 0.0, 0.0, 0.0), 0x1);
    let near_gm = add_creature(&fixture, 95_073, Position::new(25.0, 0.0, 0.0, 0.0), 0x1);
    let (search_radius, sight) = search_radius_and_sight(&fixture);
    let far_gm = add_creature(
        &fixture,
        95_074,
        Position::new(sight + 5.0, 0.0, 0.0, 0.0),
        0x1,
    );
    let accessory = add_creature(&fixture, 95_075, Position::new(30.0, 0.0, 0.0, 0.0), 0x1);
    let vehicle_guid = test_vehicle_guid(95_076);
    {
        let mut manager = fixture.manager.lock().expect("canonical manager");
        let map = manager
            .find_map_mut(571, 0)
            .expect("canonical map")
            .map_mut();
        for guid in [wrong_phase, near_gm, far_gm, accessory] {
            map.get_typed_creature_mut(guid)
                .expect("canonical Creature")
                .unit_mut()
                .set_server_side_gm_visibility_like_cpp(1);
        }
        *map.get_typed_creature_mut(wrong_phase)
            .expect("phase Creature")
            .unit_mut()
            .world_mut()
            .phase_shift_mut() = PhaseShift::from_phases([20]);
        for guid in [near_gm, far_gm, accessory] {
            *map.get_typed_creature_mut(guid)
                .expect("same-phase Creature")
                .unit_mut()
                .world_mut()
                .phase_shift_mut() = PhaseShift::from_phases([10]);
        }
        map.get_typed_creature_mut(accessory)
            .expect("vehicle accessory")
            .unit_mut()
            .subsystems_mut()
            .vehicle
            .vehicle_guid = Some(vehicle_guid);
    }
    assert_guid_is_in_visited_cells(&fixture, far_gm, fixture.body_position, search_radius);

    let selected = query(&fixture, fixture.body_position);
    assert!(!visible(&selected, wrong_phase));
    assert!(visible(&selected, near_gm));
    assert!(!visible(&selected, far_gm));
    assert!(!visible(&selected, accessory));

    fixture
        .session
        .core
        .client_visible_guids_like_cpp
        .insert(vehicle_guid);
    let vehicle_known = query(&fixture, fixture.body_position);
    assert!(visible(&vehicle_known, accessory));
}

#[test]
fn stale_player_handle_does_not_fall_back_to_same_guid_incarnation() {
    let fixture = fixture(95_081, Position::new(0.0, 0.0, 0.0, 0.0));
    let target = add_creature(&fixture, 95_082, Position::new(20.0, 0.0, 0.0, 0.0), 0x1);
    assert!(visible(&query(&fixture, fixture.body_position), target));

    let stale_handle = fixture.session.core.player_handle_like_cpp.unwrap();
    let mut replacement = wow_entities::Player::new(Some(2), false);
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(fixture.player_guid);
    let mut manager = fixture.manager.lock().expect("canonical manager");
    let current_handle = manager
        .install_detached_player_like_cpp(Box::new(replacement))
        .expect("replacement incarnation");
    manager
        .attach_player_like_cpp(
            current_handle,
            wow_map::MapKey::new(571, 0),
            fixture.body_position,
        )
        .expect("replacement active in the same map");
    assert_ne!(stale_handle, current_handle);
    drop(manager);

    assert!(
        wow_world_entities::visible_creatures_from_canonical_map_like_cpp(
            &fixture.session,
            571,
            &fixture.body_position,
        )
        .is_none()
    );
}
