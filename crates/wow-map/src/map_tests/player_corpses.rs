// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `Map::AddCorpse`/`GetCorpseByPlayer`/`ConvertCorpseToBones` (#43).

use super::*;

fn player_corpse(owner: ObjectGuid, counter: i64, position: Position) -> Corpse {
    let mut corpse = Corpse::new_at(CorpseType::ResurrectablePve, 1_000);
    corpse
        .world_mut()
        .object_mut()
        .create(ObjectGuid::create_world_object(
            HighGuid::Corpse,
            0,
            1,
            571,
            0,
            0,
            counter,
        ));
    corpse.world_mut().set_map(571, 0).unwrap();
    corpse.world_mut().relocate(position);
    corpse.set_owner_guid(owner);
    corpse.set_display_id(49);
    corpse.replace_all_flags(0x04);
    corpse.set_race(1);
    corpse
}

#[test]
fn add_corpse_to_map_indexes_owner_and_enters_world_like_cpp() {
    let mut map = Map::new(571, 0, 0, 60_000);
    let owner = ObjectGuid::create_player(1, 42);
    let position = Position::new(10.0, 20.0, 30.0, 1.5);
    let corpse = player_corpse(owner, 7, position);
    let corpse_guid = corpse.world().object().guid();

    assert!(map.corpse_guid_by_player_like_cpp(owner).is_none());
    map.add_corpse_to_map_like_cpp(corpse).unwrap();

    assert_eq!(map.corpse_guid_by_player_like_cpp(owner), Some(corpse_guid));
    assert!(
        map.get_typed_corpse(corpse_guid)
            .unwrap()
            .world()
            .object()
            .is_in_world()
    );
    assert!(
        map.nearby_cell_guids_like_cpp(position.x, position.y, 1.0)
            .world
            .corpses
            .contains(&corpse_guid)
    );
    assert!(!map.is_removal_grid_like_cpp(position.x, position.y));
}

#[test]
fn convert_corpse_to_bones_replaces_corpse_with_flagged_bones_like_cpp() {
    let mut map = Map::new(571, 0, 0, 60_000);
    let owner = ObjectGuid::create_player(1, 42);
    let position = Position::new(10.0, 20.0, 30.0, 1.5);
    let corpse = player_corpse(owner, 7, position);
    let corpse_guid = corpse.world().object().guid();
    map.add_corpse_to_map_like_cpp(corpse).unwrap();

    let outcome = map
        .convert_corpse_to_bones_like_cpp(owner, true, 5_000)
        .unwrap();

    assert_eq!(outcome.corpse_guid, corpse_guid);
    let bones_guid = outcome.bones_guid.expect("bones on a loaded grid");
    assert_eq!(bones_guid.counter(), corpse_guid.counter());
    assert!(map.corpse_guid_by_player_like_cpp(owner).is_none());
    let bones = map.get_typed_corpse(bones_guid).unwrap();
    assert_eq!(bones.corpse_type(), CorpseType::Bones);
    assert_eq!(bones.ghost_time(), 5_000);
    assert_eq!(bones.data().owner, owner);
    assert_eq!(bones.data().display_id, 49);
    assert_eq!(bones.data().flags, 0x04 | crate::CORPSE_FLAG_BONES_LIKE_CPP);
    assert_eq!(bones.world().position(), position);
    assert_eq!(bones.world().object().scale(), 0.0);
    assert!(
        bones.world().object().is_new_object(),
        "birth remains pending after AddToMap"
    );

    // No second corpse to convert.
    assert!(
        map.convert_corpse_to_bones_like_cpp(owner, true, 5_000)
            .is_none()
    );
}

#[test]
fn convert_corpse_to_bones_without_bones_only_removes_like_cpp() {
    let mut map = Map::new(571, 0, 0, 60_000);
    let owner = ObjectGuid::create_player(1, 43);
    let corpse = player_corpse(owner, 8, Position::new(10.0, 20.0, 30.0, 0.0));
    let corpse_guid = corpse.world().object().guid();
    map.add_corpse_to_map_like_cpp(corpse).unwrap();

    let outcome = map
        .convert_corpse_to_bones_like_cpp(owner, false, 5_000)
        .unwrap();

    assert_eq!(outcome.bones_guid, None);
    assert!(map.get_typed_corpse(corpse_guid).is_none());
    assert!(map.corpse_guid_by_player_like_cpp(owner).is_none());
}

#[test]
fn removal_grid_matches_missing_grid_like_cpp() {
    let map = Map::new(571, 0, 0, 60_000);
    assert!(map.is_removal_grid_like_cpp(10.0, 20.0));
}

#[test]
fn converting_a_corpse_in_a_removal_grid_never_births_bones() {
    let mut map = Map::new(571, 0, 0, 60_000);
    let owner = ObjectGuid::create_player(1, 44);
    let position = Position::new(10.0, 20.0, 30.0, 0.0);
    let corpse = player_corpse(owner, 9, position);
    let guid = corpse.world().guid();
    map.add_corpse_to_map_like_cpp(corpse).unwrap();
    let cell = Cell::from_world(position.x, position.y);
    map.get_ngrid_mut(GridCoord::new(cell.grid_x(), cell.grid_y()))
        .unwrap()
        .set_state(GridStateKind::Removal);
    let outcome = map
        .convert_corpse_to_bones_like_cpp(owner, true, 5_000)
        .unwrap();
    assert_eq!(outcome.corpse_guid, guid);
    assert_eq!(outcome.bones_guid, None);
    assert!(map.get_typed_corpse(guid).is_none());
}
