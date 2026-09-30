use super::*;

#[path = "../../map/actor_transport/tests/fixtures.rs"]
mod fixtures;

#[test]
fn source_take_restore_keeps_exact_grid_metadata_actor_motor_and_loot() {
    for point in [false, true] {
        let (_, mut source, guid, mut rng) = fixtures::pair(841, point);
        let coord = GridCoord::new(0, 0);
        let grid = source.grids.get(&coord).unwrap();
        let before = (grid.coord, grid.loaded, grid.last_player_time, grid.player_guids.clone());
        let loot = grid.creatures[&guid].creature.loot_authority_like_cpp().clone();
        let slot = source.preflight_creature_transport_slot(coord, guid).unwrap();
        let taken = source.take_creature_transport_slot(slot).unwrap();
        assert!(source.grids[&coord].creatures.is_empty());
        source.restore_creature_transport_slot(taken).unwrap();
        let grid = source.grids.get_mut(&coord).unwrap();
        assert_eq!((grid.coord, grid.loaded, grid.last_player_time, grid.player_guids.clone()), before);
        assert!(grid.creatures[&guid].creature.loot_authority_like_cpp().shares_storage_like_cpp(&loot));
        grid.creatures.get_mut(&guid).unwrap().assert_actor_storage_runtime(&mut rng, point);
    }
}

#[test]
fn missing_grid_restore_returns_the_original_receipt_without_creating_a_grid() {
    let (_, mut source, guid, mut rng) = fixtures::pair(842, true);
    let coord = GridCoord::new(0, 0);
    let slot = source.preflight_creature_transport_slot(coord, guid).unwrap();
    let taken = source.take_creature_transport_slot(slot).unwrap();
    let original_grid = source.grids.remove(&coord).unwrap();
    let (error, taken) = source.restore_creature_transport_slot(taken).unwrap_err();
    assert_eq!(error, LegacyCreatureTransportError::MissingGrid);
    assert!(source.grids.is_empty());
    source.grids.insert(coord, original_grid);
    source.restore_creature_transport_slot(taken).unwrap();
    source.grids.get_mut(&coord).unwrap().creatures.get_mut(&guid).unwrap().assert_actor_storage_runtime(&mut rng, true);
}

#[test]
fn occupied_restore_preserves_occupant_and_returns_the_complete_taken_actor() {
    let (_, mut source, guid, mut rng) = fixtures::pair(843, false);
    let coord = GridCoord::new(0, 0);
    let slot = source.preflight_creature_transport_slot(coord, guid).unwrap();
    let taken = source.take_creature_transport_slot(slot).unwrap();
    let (_, mut other, _, _) = fixtures::pair(843, true);
    let mut replacement = other.grids.get_mut(&coord).unwrap().creatures.remove(&guid).unwrap();
    replacement.create_data.npc_flags = 0x9999;
    source.grids.get_mut(&coord).unwrap().creatures.insert(guid, replacement);
    let (error, taken) = source.restore_creature_transport_slot(taken).unwrap_err();
    assert_eq!(error, LegacyCreatureTransportError::Occupied);
    assert_eq!(source.grids[&coord].creatures[&guid].create_data.npc_flags, 0x9999);
    source.grids.get_mut(&coord).unwrap().creatures.remove(&guid).unwrap();
    source.restore_creature_transport_slot(taken).unwrap();
    source.grids.get_mut(&coord).unwrap().creatures.get_mut(&guid).unwrap().assert_actor_storage_runtime(&mut rng, false);
}

#[test]
fn wrong_instance_or_duplicate_restore_never_reconstructs_or_drops_the_receipt() {
    let (_, mut source, guid, _) = fixtures::pair(844, true);
    let coord = GridCoord::new(0, 0);
    let slot = source.preflight_creature_transport_slot(coord, guid).unwrap();
    let taken = source.take_creature_transport_slot(slot).unwrap();
    source.instance_id = 8;
    let (error, taken) = source.restore_creature_transport_slot(taken).unwrap_err();
    assert_eq!(error, LegacyCreatureTransportError::WrongInstance);
    source.instance_id = 7;
    let (_, mut other, _, _) = fixtures::pair(844, false);
    let replacement = other.grids.get_mut(&coord).unwrap().creatures.remove(&guid).unwrap();
    let mut grid = super::super::Grid::new(1, 0);
    grid.creatures.insert(guid, replacement);
    source.grids.insert(GridCoord::new(1, 0), grid);
    let (error, taken) = source.restore_creature_transport_slot(taken).unwrap_err();
    assert_eq!(error, LegacyCreatureTransportError::DuplicateGuid);
    assert_eq!(taken.actor.guid(), guid);
    assert!(source.grids[&coord].creatures.is_empty());
}
