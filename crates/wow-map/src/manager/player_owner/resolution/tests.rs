use super::*;
use wow_core::{ObjectGuid, Position};
use wow_entities::Player;

fn installed(active: bool) -> (MapManager, PlayerHandle) {
    let mut manager = MapManager::default();
    let mut player = Box::new(Player::new(Some(70_001), false));
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(ObjectGuid::create_player(1, 70_001));
    let handle = manager.install_detached_player_like_cpp(player).unwrap();
    if active {
        manager.create_world_map(530, 0);
        manager
            .attach_player_like_cpp(handle, MapKey::new(530, 0), Position::default())
            .unwrap();
    }
    (manager, handle)
}

#[test]
fn canonical_residence_follows_the_same_player_through_transfer() {
    let (mut manager, handle) = installed(true);
    assert_eq!(
        manager.checked_player_residence_like_cpp(handle),
        Ok(PlayerResidenceLikeCpp::Active(MapKey::new(530, 0)))
    );
    manager.detach_player_like_cpp(handle).unwrap();
    assert_eq!(
        manager.checked_player_residence_like_cpp(handle),
        Ok(PlayerResidenceLikeCpp::Detached)
    );
    manager.create_world_map(571, 0);
    manager
        .attach_player_like_cpp(handle, MapKey::new(571, 0), Position::default())
        .unwrap();
    assert_eq!(
        manager.checked_player_residence_like_cpp(handle),
        Ok(PlayerResidenceLikeCpp::Active(MapKey::new(571, 0)))
    );
}

#[test]
fn retirement_and_replacement_are_not_a_missing_backing_player() {
    let (mut manager, old) = installed(false);
    let player = manager.retire_player_like_cpp(old).unwrap();
    assert_eq!(
        manager.checked_player_residence_like_cpp(old),
        Err(PlayerOwnerError::MissingOwner { guid: old.guid() })
    );
    let current = manager.install_detached_player_like_cpp(player).unwrap();
    assert_eq!(
        manager.checked_player_residence_like_cpp(old),
        Err(PlayerOwnerError::StaleHandle)
    );
    assert_eq!(
        manager.checked_player_residence_like_cpp(current),
        Ok(PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(manager.player_residence_like_cpp(old), None);
}

#[test]
fn detached_index_without_player_fails_closed() {
    let (mut manager, handle) = installed(false);
    manager.detached_players_like_cpp.remove(&handle.guid());
    assert_eq!(
        manager.checked_player_residence_like_cpp(handle),
        Err(PlayerOwnerError::MissingPlayer {
            guid: handle.guid()
        })
    );
    assert_eq!(manager.player_residence_like_cpp(handle), None);
}

#[test]
fn active_index_without_map_fails_closed() {
    let (mut manager, handle) = installed(true);
    let key = MapKey::new(530, 0);
    // Deliberate private-state corruption: ordinary occupied-map unload is guarded.
    manager.maps.remove(&key);
    assert_eq!(
        manager.checked_player_residence_like_cpp(handle),
        Err(PlayerOwnerError::MissingMap { key })
    );
    assert_eq!(manager.player_residence_like_cpp(handle), None);
}

#[test]
fn backing_player_identity_must_match_the_incarnation() {
    let (mut manager, handle) = installed(false);
    let actual = ObjectGuid::create_player(1, 70_002);
    manager.with_player_mut_like_cpp(handle, |player| {
        player.unit_mut().world_mut().object_mut().create(actual);
    });
    assert_eq!(
        manager.checked_player_residence_like_cpp(handle),
        Err(PlayerOwnerError::PlayerGuidMismatch {
            expected: handle.guid(),
            actual
        })
    );
    assert_eq!(manager.player_residence_like_cpp(handle), None);
}

#[test]
fn active_index_without_its_typed_player_fails_closed() {
    let (mut manager, handle) = installed(true);
    manager.create_world_map(571, 0);
    // Point the index at an existing but empty map, leaving the real Player alone.
    manager
        .player_owners_like_cpp
        .get_mut(&handle.guid())
        .unwrap()
        .residence = PlayerResidenceLikeCpp::Active(MapKey::new(571, 0));
    assert_eq!(
        manager.checked_player_residence_like_cpp(handle),
        Err(PlayerOwnerError::ActivePlayerMissing {
            guid: handle.guid()
        })
    );
    assert_eq!(manager.player_residence_like_cpp(handle), None);
}

#[test]
fn detached_player_marked_in_world_is_not_admitted() {
    let (mut manager, handle) = installed(false);
    manager.with_player_mut_like_cpp(handle, |player| {
        player.unit_mut().world_mut().object_mut().add_to_world();
    });
    assert_eq!(
        manager.checked_player_residence_like_cpp(handle),
        Err(PlayerOwnerError::DetachedPlayerStillInWorld {
            guid: handle.guid()
        })
    );
    assert_eq!(manager.player_residence_like_cpp(handle), None);
}

#[test]
fn detached_player_with_a_map_binding_is_not_admitted() {
    let (mut manager, handle) = installed(false);
    let key = MapKey::new(571, 0);
    manager.with_player_mut_like_cpp(handle, |player| {
        player
            .unit_mut()
            .world_mut()
            .set_map(key.map_id, key.instance_id)
            .unwrap();
    });
    assert_eq!(
        manager.checked_player_residence_like_cpp(handle),
        Err(PlayerOwnerError::DetachedPlayerStillBound {
            guid: handle.guid(),
            key
        })
    );
    assert_eq!(manager.player_residence_like_cpp(handle), None);
}

#[test]
fn active_player_outside_world_is_not_admitted() {
    let (mut manager, handle) = installed(true);
    manager.with_player_mut_like_cpp(handle, |player| {
        player
            .unit_mut()
            .world_mut()
            .object_mut()
            .remove_from_world();
    });
    assert_eq!(
        manager.checked_player_residence_like_cpp(handle),
        Err(PlayerOwnerError::ActivePlayerNotInWorld {
            guid: handle.guid()
        })
    );
    assert_eq!(manager.player_residence_like_cpp(handle), None);
}

#[test]
fn active_player_bound_to_another_map_is_not_admitted() {
    let (mut manager, handle) = installed(true);
    manager.with_player_mut_like_cpp(handle, |player| {
        let world = player.unit_mut().world_mut();
        world.object_mut().remove_from_world();
        world.reset_map().unwrap();
        world.set_map(571, 0).unwrap();
        world.object_mut().add_to_world();
    });
    assert_eq!(
        manager.checked_player_residence_like_cpp(handle),
        Err(PlayerOwnerError::ActivePlayerMapMismatch {
            guid: handle.guid(),
            expected: MapKey::new(530, 0),
            actual: Some(MapKey::new(571, 0)),
        })
    );
    assert_eq!(manager.player_residence_like_cpp(handle), None);
}
