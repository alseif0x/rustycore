//! #1263 F6-8D3b-3 regressions: the canonical-first mutation root always gives
//! the lent incarnation back, and a position it changed moves the creature's
//! grid cell (C++ `Unit::UpdatePosition` → `Map::CreatureRelocation`).

use super::*;

fn d3b3_session_like_cpp(
    guid: ObjectGuid,
) -> (
    WorldSession,
    SharedCanonicalMapManager,
    SharedCanonicalMapManager,
) {
    let manager = shared_map_manager();
    let canonical = Arc::clone(&manager);
    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let manager = session
        .core
        .canonical_map_manager
        .clone()
        .unwrap_or_else(|| manager.clone());
    register_test_creature(&mut session, manager.clone(), guid, 25);
    (session, manager, canonical)
}

/// Whether the cell that contains `position` holds `guid`.
fn d3b3_cell_holds_like_cpp(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    position: Position,
) -> bool {
    let guard = canonical
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let nearby = guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .nearby_cell_guids_like_cpp(position.x, position.y, 0.0);
    nearby.grid.creatures.contains(&guid) || nearby.world.creatures.contains(&guid)
}

fn d3b3_creature_like_cpp<R>(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    read: impl FnOnce(&wow_entities::Creature) -> R,
) -> Option<R> {
    canonical
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_creature(guid)
        .map(read)
}

#[test]
fn a_panicking_mutation_leaves_the_incarnation_in_its_map_and_cell_like_cpp() {
    let guid = test_creature_guid(97_300);
    let (mut session, _manager, canonical) = d3b3_session_like_cpp(guid);
    let origin = d3b3_creature_like_cpp(&canonical, guid, |creature| creature.position()).unwrap();
    assert!(d3b3_cell_holds_like_cpp(&canonical, guid, origin));

    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        session.mutate_world_creature(guid, |creature| {
            creature.unit_mut().set_health(7);
            panic!("F6-8D3b-3: injected mutation panic");
        })
    }));

    assert!(unwound.is_err(), "the panic still reaches the caller");
    assert!(
        !canonical.is_poisoned(),
        "the canonical guard is released before the panic resumes"
    );
    assert_eq!(
        d3b3_creature_like_cpp(&canonical, guid, |creature| creature.current_hp()),
        Some(7),
        "the incarnation is back in entity_world, as the mutation left it"
    );
    assert!(
        d3b3_cell_holds_like_cpp(&canonical, guid, origin),
        "and its GUID is still in the cell it is in"
    );
    assert_eq!(
        session.mutate_world_creature(guid, |creature| creature.current_hp()),
        Some(7),
        "the root keeps working on the same incarnation"
    );
}

#[test]
fn a_mutation_that_rebinds_the_map_is_refused_and_the_incarnation_restored_like_cpp() {
    let guid = test_creature_guid(97_301);
    let (mut session, _manager, canonical) = d3b3_session_like_cpp(guid);
    let origin = d3b3_creature_like_cpp(&canonical, guid, |creature| creature.position()).unwrap();

    // `WorldObject::world_relocate` rebinds the object's map: the record it
    // left can no longer take it back (`MapObjectStoreError::WrongMap`).
    let refused = session.mutate_world_creature(guid, |creature| {
        creature.unit_mut().set_health(9);
        creature.unit_mut().world_mut().world_relocate(1, origin);
    });

    assert_eq!(refused, None, "the rebinding mutation is refused loudly");
    let (map_id, health, has_map) = d3b3_creature_like_cpp(&canonical, guid, |creature| {
        let world = creature.unit().world();
        (
            world.map_id(),
            creature.current_hp(),
            world.has_current_map(),
        )
    })
    .expect("the incarnation was restored instead of dropped");
    assert_eq!((map_id, has_map), (0, true), "with its original binding");
    assert_eq!(health, 9);
    assert!(d3b3_cell_holds_like_cpp(&canonical, guid, origin));
    assert_eq!(
        session.mutate_world_creature(guid, |creature| creature.current_hp()),
        Some(9)
    );
}

#[test]
fn a_position_change_through_the_root_relocates_the_creature_cell_like_cpp() {
    let guid = test_creature_guid(97_302);
    let (mut session, _manager, canonical) = d3b3_session_like_cpp(guid);
    let origin = d3b3_creature_like_cpp(&canonical, guid, |creature| creature.position()).unwrap();
    // Same grid, three cells (66.6 yd) away: what a vehicle or possessed
    // creature's `CMSG_MOVE_*` writes through `set_ai_position`.
    let moved = Position::new(origin.x + 200.0, origin.y, origin.z, 1.5);
    assert!(d3b3_cell_holds_like_cpp(&canonical, guid, origin));

    assert!(
        session
            .mutate_world_creature(guid, |creature| creature.set_ai_position(moved))
            .is_some()
    );

    assert_eq!(
        d3b3_creature_like_cpp(&canonical, guid, |creature| creature.position()),
        Some(moved)
    );
    assert!(
        d3b3_cell_holds_like_cpp(&canonical, guid, moved),
        "the GUID is in the cell of its new position"
    );
    assert!(
        !d3b3_cell_holds_like_cpp(&canonical, guid, origin),
        "and no longer in the cell it left"
    );
    // Visibility collects creatures from the cells around the viewer
    // (`visible_creatures_from_canonical_map_like_cpp`): a viewer at the new
    // location finds it, one at the old location no longer does.
    let guard = canonical.lock().unwrap();
    let map = guard.find_map(0, 0).unwrap().map();
    let near_new = map.nearby_cell_guids_like_cpp(moved.x, moved.y, 90.0);
    let near_old = map.nearby_cell_guids_like_cpp(origin.x, origin.y, 90.0);
    assert!(near_new.grid.creatures.contains(&guid) || near_new.world.creatures.contains(&guid));
    assert!(!near_old.grid.creatures.contains(&guid) && !near_old.world.creatures.contains(&guid));
    drop(guard);

    // An orientation-only change stays in the cell and is applied in place.
    let turned = Position::new(moved.x, moved.y, moved.z, 3.0);
    session
        .mutate_world_creature(guid, |creature| creature.set_ai_position(turned))
        .unwrap();
    assert_eq!(
        d3b3_creature_like_cpp(&canonical, guid, |creature| creature.position()),
        Some(turned)
    );
    assert!(d3b3_cell_holds_like_cpp(&canonical, guid, turned));
}

#[test]
fn a_creature_moved_in_place_follows_into_its_new_cell_like_cpp() {
    // The executor's movement generators write the position in place; the
    // phase then applies `Map::CreatureRelocation` from where it started.
    let guid = test_creature_guid(97_303);
    let (_session, _manager, canonical) = d3b3_session_like_cpp(guid);
    let origin = d3b3_creature_like_cpp(&canonical, guid, |creature| creature.position()).unwrap();
    let moved = Position::new(origin.x + 150.0, origin.y + 5.0, origin.z, 0.0);
    let mut guard = canonical.lock().unwrap();
    let map = guard.find_map_mut(0, 0).unwrap().map_mut();
    map.get_typed_creature_mut(guid)
        .unwrap()
        .set_ai_position(moved);
    assert!(
        map.relocate_creature_moved_in_place_like_cpp(guid, moved)
            .is_none(),
        "no move relative to the position it is at"
    );
    let outcome = map
        .relocate_creature_moved_in_place_like_cpp(guid, origin)
        .expect("moved")
        .expect("valid position");
    assert!(outcome.relocated && outcome.moved_between_cells);
    assert_eq!(map.get_typed_creature(guid).unwrap().position(), moved);
    drop(guard);
    assert!(d3b3_cell_holds_like_cpp(&canonical, guid, moved));
    assert!(!d3b3_cell_holds_like_cpp(&canonical, guid, origin));
}
