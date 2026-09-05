//! Real wow-map library, without cfg(test) Player ownership shortcuts.
//! C++ MapManager::DestroyMap refuses destruction while Map::HavePlayers is true
//! (Maps/MapManager.cpp:322-339); RemoveAllPlayers requests transfer, not deletion.
use wow_core::{ObjectGuid, Position};
use wow_entities::Player;
use wow_map::{MapKey, MapManager, PlayerHandle, PlayerResidenceLikeCpp};

fn active_player(manager: &mut MapManager, id: i64, key: MapKey) -> PlayerHandle {
    manager.create_world_map(key.map_id, key.instance_id);
    let mut player = Box::new(Player::new(Some(1), false));
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(ObjectGuid::create_player(1, id));
    player.set_money(123);
    let handle = manager.install_detached_player_like_cpp(player).unwrap();
    manager
        .attach_player_like_cpp(handle, key, Position::default())
        .unwrap();
    handle
}

#[test]
fn production_destroy_map_cannot_drop_a_live_player_behind_its_handle() {
    let mut manager = MapManager::default();
    let key = MapKey::new(1, 0);
    let handle = active_player(&mut manager, 42, key);
    assert!(!manager.destroy_map(key.map_id, key.instance_id));
    assert_eq!(
        manager.player_residence_like_cpp(handle),
        Some(PlayerResidenceLikeCpp::Active(key))
    );
    assert_eq!(
        manager.with_player_like_cpp(handle, Player::money),
        Some(123)
    );
    assert_eq!(manager.find_map(1, 0).unwrap().unload_all_calls(), 0);

    manager.detach_player_like_cpp(handle).unwrap();
    assert!(manager.destroy_map(key.map_id, key.instance_id));
    assert_eq!(
        manager.with_player_like_cpp(handle, Player::money),
        Some(123)
    );
    assert_eq!(
        manager.player_residence_like_cpp(handle),
        Some(PlayerResidenceLikeCpp::Detached)
    );
}

#[test]
fn production_update_cannot_destroy_an_occupied_unload_candidate() {
    let mut manager = MapManager::default();
    let key = MapKey::new(1, 0);
    let handle = active_player(&mut manager, 42, key);
    manager.find_map_mut(1, 0).unwrap().set_can_unload(true);
    manager.update(100);
    assert_eq!(
        manager.with_player_like_cpp(handle, Player::money),
        Some(123)
    );
    assert_eq!(manager.find_map(1, 0).unwrap().unload_all_calls(), 0);
}

#[test]
fn production_unload_is_atomic_and_preserves_detached_lifetime_after_drain() {
    let mut manager = MapManager::default();
    let first = active_player(&mut manager, 42, MapKey::new(1, 0));
    let second = active_player(&mut manager, 43, MapKey::new(2, 0));
    manager.create_world_map(0, 0); // Empty map must not be partially unloaded either.
    let blocked = manager.unload_all().unwrap_err();
    assert_eq!(
        blocked.occupied_maps,
        [MapKey::new(1, 0), MapKey::new(2, 0)]
    );
    for map in 0..=2 {
        assert_eq!(manager.find_map(map, 0).unwrap().unload_all_calls(), 0);
    }
    for handle in [first, second] {
        assert_eq!(
            manager.with_player_like_cpp(handle, Player::money),
            Some(123)
        );
        manager.detach_player_like_cpp(handle).unwrap();
    }
    manager.unload_all().unwrap();
    for map in 0..=2 {
        assert!(manager.find_map(map, 0).is_none());
    }
    for handle in [first, second] {
        assert_eq!(
            manager.with_player_like_cpp(handle, Player::money),
            Some(123)
        );
        assert_eq!(
            manager.player_residence_like_cpp(handle),
            Some(PlayerResidenceLikeCpp::Detached)
        );
    }
}

#[test]
fn production_unadopted_player_also_blocks_destruction_with_zero_compatibility_count() {
    let mut manager = MapManager::default();
    let guid = ObjectGuid::create_player(1, 42);
    let mut player = Player::new(Some(1), false);
    player.unit_mut().world_mut().object_mut().create(guid);
    player.unit_mut().world_mut().set_map(1, 0).unwrap();
    player.unit_mut().world_mut().object_mut().add_to_world();
    let map = manager.create_world_map(1, 0);
    map.map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    assert_eq!(map.player_count(), 0);
    assert!(!manager.destroy_map(1, 0));
    assert!(manager.unload_all().is_err());
    assert!(
        manager
            .find_map(1, 0)
            .unwrap()
            .map()
            .get_typed_player(guid)
            .is_some()
    );
}

#[test]
fn production_failed_destroy_does_not_recycle_the_occupied_instance_id() {
    let mut manager = MapManager::default();
    manager.init_instance_ids(10);
    for id in 1..=7 {
        manager.register_instance_id(id);
    }
    manager.create_map_entry(489, 7, 0, wow_map::ManagedMapKind::Battleground);
    let handle = active_player(&mut manager, 42, MapKey::new(489, 7));
    assert!(!manager.destroy_map(489, 7));
    assert_eq!(manager.generate_instance_id(), Some(8));
    manager.detach_player_like_cpp(handle).unwrap();
    assert!(manager.destroy_map(489, 7));
    assert_eq!(manager.generate_instance_id(), Some(7));
    assert_eq!(
        manager.with_player_like_cpp(handle, Player::money),
        Some(123)
    );
}
