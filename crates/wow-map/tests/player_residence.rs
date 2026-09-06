//! Public production-library lifecycle path; wow-map is built without cfg(test).
//! No database, scheduler, or packet-order claim follows from these owner tests.

use wow_core::{ObjectGuid, Position};
use wow_entities::Player;
use wow_map::{MapKey, MapManager, PlayerOwnerError, PlayerResidenceLikeCpp};

#[test]
fn production_residence_survives_failed_attach_transfer_and_replacement() {
    let mut manager = MapManager::default();
    let mut player = Box::new(Player::new(Some(80_001), false));
    let guid = ObjectGuid::create_player(1, 80_001);
    player.unit_mut().world_mut().object_mut().create(guid);
    player.set_money(123);
    let old = manager.install_detached_player_like_cpp(player).unwrap();
    let missing = MapKey::new(571, 0);
    assert_eq!(
        manager.attach_player_like_cpp(old, missing, Position::default()),
        Err(PlayerOwnerError::MissingMap { key: missing })
    );
    assert_eq!(
        manager.checked_player_residence_like_cpp(old),
        Ok(PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(manager.with_player_like_cpp(old, Player::money), Some(123));

    manager.create_world_map(530, 0);
    manager
        .attach_player_like_cpp(old, MapKey::new(530, 0), Position::default())
        .unwrap();
    assert_eq!(
        manager.checked_player_residence_like_cpp(old),
        Ok(PlayerResidenceLikeCpp::Active(MapKey::new(530, 0)))
    );
    manager.detach_player_like_cpp(old).unwrap();
    manager.create_world_map(571, 0);
    manager
        .attach_player_like_cpp(old, missing, Position::default())
        .unwrap();
    assert_eq!(
        manager.checked_player_residence_like_cpp(old),
        Ok(PlayerResidenceLikeCpp::Active(missing))
    );

    let same_player = manager.retire_player_like_cpp(old).unwrap();
    assert_eq!(
        manager.checked_player_residence_like_cpp(old),
        Err(PlayerOwnerError::MissingOwner { guid })
    );
    let new = manager
        .install_detached_player_like_cpp(same_player)
        .unwrap();
    assert_eq!(
        manager.checked_player_residence_like_cpp(old),
        Err(PlayerOwnerError::StaleHandle)
    );
    assert_eq!(
        manager.checked_player_residence_like_cpp(new),
        Ok(PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(manager.with_player_like_cpp(new, Player::money), Some(123));
}
