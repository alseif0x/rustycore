#![cfg(test)]
//! Failed admission must not retire the current Player incarnation.
use super::*;

fn player(guid: ObjectGuid, money: u64) -> Box<Player> {
    let mut player = Box::new(Player::new(Some(1), false));
    player.unit_mut().world_mut().object_mut().create(guid);
    player.set_money(money);
    player
}

#[test]
fn exhausted_generation_preserves_previous_active_and_detached_player() {
    for active in [false, true] {
        let mut manager = MapManager::default();
        let guid = ObjectGuid::create_player(1, 42);
        let old = manager
            .install_detached_player_like_cpp(player(guid, 123))
            .unwrap();
        if active {
            manager.create_world_map(1, 0);
            manager
                .attach_player_like_cpp(old, MapKey::new(1, 0), Position::default())
                .unwrap();
        }
        let residence = manager.player_residence_like_cpp(old);
        manager.next_player_generation_like_cpp = u64::MAX;
        assert_eq!(
            manager.install_detached_player_like_cpp(player(guid, 456)),
            Err(PlayerOwnerError::GenerationExhausted)
        );
        assert_eq!(manager.player_residence_like_cpp(old), residence);
        assert_eq!(manager.with_player_like_cpp(old, Player::money), Some(123));
    }
}
