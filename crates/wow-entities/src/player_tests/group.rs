//! Group scenarios for [`super`].
//!
//! Split out of player_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn clearing_duel_clears_player_owned_arbiter_like_cpp() {
    let opponent = ObjectGuid::create_player(1, 42);
    let arbiter = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 0, 0, 123, 7);
    let mut player = Player::new(Some(1), false);

    player.set_duel_info_like_cpp(Some(PlayerDuelInfoLikeCpp {
        opponent,
        state: PlayerDuelStateLikeCpp::Challenged,
    }));
    player.set_duel_arbiter_like_cpp(Some(arbiter));
    assert_eq!(player.duel_arbiter_like_cpp(), Some(arbiter));

    player.clear_duel_like_cpp();
    assert_eq!(player.duel_info_like_cpp(), None);
    assert_eq!(player.duel_arbiter_like_cpp(), None);
}
