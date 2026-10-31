use super::{RepresentedMeleeSwingLikeCpp, apply_player_melee_to_canonical_player_like_cpp};
use wow_constants::DeathState;
use wow_entities::Player;

fn victim() -> Player {
    let mut player = Player::new(None, false);
    player.unit_mut().set_max_health(100);
    player.unit_mut().set_health(100);
    player.unit_mut().set_level(20);
    player
}

#[test]
fn lethal_player_melee_enters_just_died_and_does_not_apply_later_swings() {
    for damage in [100, 125] {
        let mut player = victim();
        let swings = [
            RepresentedMeleeSwingLikeCpp::hit_like_cpp(damage),
            RepresentedMeleeSwingLikeCpp::hit_like_cpp(50),
        ];
        let (applied, level) =
            apply_player_melee_to_canonical_player_like_cpp(&mut player, &swings).unwrap();
        assert_eq!(applied, vec![(damage, (damage - 100) as i32)]);
        assert_eq!(level, 20);
        assert_eq!(player.unit().data().health, 0);
        assert_eq!(player.unit().death_state(), DeathState::JustDied);
        assert_ne!(
            player.extra_flags() & 0x0100,
            0,
            "the corpse must use PvP policy"
        );
        assert!(apply_player_melee_to_canonical_player_like_cpp(&mut player, &swings).is_none());
    }
}

#[test]
fn nonlethal_and_avoided_swings_keep_the_player_alive() {
    let mut player = victim();
    let swings = [
        RepresentedMeleeSwingLikeCpp::hit_like_cpp(0),
        RepresentedMeleeSwingLikeCpp::hit_like_cpp(25),
        RepresentedMeleeSwingLikeCpp::hit_like_cpp(30),
    ];
    let (applied, _) =
        apply_player_melee_to_canonical_player_like_cpp(&mut player, &swings).unwrap();
    assert_eq!(applied, vec![(0, -1), (25, -1), (30, -1)]);
    assert_eq!(player.unit().data().health, 45);
    assert_eq!(player.unit().death_state(), DeathState::Alive);
    assert_eq!(player.extra_flags() & 0x0100, 0);
}

#[test]
fn zero_health_victim_is_not_killed_again_even_with_stale_alive_state() {
    let mut player = victim();
    player.unit_mut().set_health(0);
    assert!(
        apply_player_melee_to_canonical_player_like_cpp(
            &mut player,
            &[RepresentedMeleeSwingLikeCpp::hit_like_cpp(25)],
        )
        .is_none()
    );
}
