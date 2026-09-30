use std::cell::Cell;

use super::Player;
use wow_constants::DeathState;

fn victim(health: u64, level: u8) -> Player {
    let mut player = Player::new(Some(7), false);
    let unit = player.unit_mut();
    unit.set_max_health(health.max(1));
    unit.set_health(health);
    unit.set_level(level);
    unit.clear_unit_data_changes();
    player
}

#[test]
fn non_alive_victim_rejects_before_consuming_the_iterator() {
    for state in [DeathState::JustDied, DeathState::Corpse, DeathState::Dead] {
        let mut player = victim(20, 80);
        player.unit_mut().set_death_state(state);
        let health_before = player.unit().data().health;
        let revision_before = player.unit().health_state_revision_like_cpp();
        let consumed = Cell::new(0);
        let damages = [5, 10]
            .into_iter()
            .inspect(|_| consumed.set(consumed.get() + 1));

        assert_eq!(player.apply_melee_damage_batch(damages), None);
        assert_eq!(consumed.get(), 0);
        assert_eq!(player.unit().data().health, health_before);
        assert_eq!(
            player.unit().health_state_revision_like_cpp(),
            revision_before
        );
        assert_eq!(player.unit().death_state(), state);
        assert!(!player.unit().unit_data_changes_mask().is_any_set());
    }
}

#[test]
fn alive_victim_accepts_empty_batch_without_a_health_write() {
    let mut player = victim(20, 80);
    let revision_before = player.unit().health_state_revision_like_cpp();

    assert_eq!(
        player.apply_melee_damage_batch(std::iter::empty()),
        Some((Vec::new(), 80))
    );
    assert_eq!(player.unit().data().health, 20);
    assert_eq!(
        player.unit().health_state_revision_like_cpp(),
        revision_before
    );
    assert!(!player.unit().unit_data_changes_mask().is_any_set());
}

#[test]
fn zero_damage_reports_each_swing_without_changing_health_or_revision() {
    let mut player = victim(20, 80);
    let revision_before = player.unit().health_state_revision_like_cpp();

    assert_eq!(
        player.apply_melee_damage_batch([0, 0].into_iter()),
        Some((vec![(0, -1), (0, -1)], 80))
    );
    assert_eq!(player.unit().data().health, 20);
    assert_eq!(
        player.unit().health_state_revision_like_cpp(),
        revision_before
    );
    assert!(!player.unit().unit_data_changes_mask().is_any_set());
}

#[test]
fn ordered_batch_preserves_sublethal_exact_lethal_and_postlethal_results() {
    let mut player = victim(20, 80);
    let revision_before = player.unit().health_state_revision_like_cpp();
    let consumed = Cell::new(0);
    let damages = [0, 7, 13, 0, 5]
        .into_iter()
        .inspect(|_| consumed.set(consumed.get() + 1));

    assert_eq!(
        player.apply_melee_damage_batch(damages),
        Some((vec![(0, -1), (7, -1), (13, 0), (0, -1), (5, 5)], 80)),
    );
    assert_eq!(consumed.get(), 5);
    assert_eq!(player.unit().data().health, 0);
    assert_eq!(
        player.unit().health_state_revision_like_cpp(),
        revision_before + 2
    );
    assert_eq!(player.unit().death_state(), DeathState::Alive);
    assert!(player.unit().unit_data_changes_mask().is_any_set());
}

#[test]
fn overkill_saturates_health_and_later_swings_read_zero() {
    let mut player = victim(3, 80);
    let revision_before = player.unit().health_state_revision_like_cpp();

    assert_eq!(
        player.apply_melee_damage_batch([8, 2].into_iter()),
        Some((vec![(8, 5), (2, 2)], 80))
    );
    assert_eq!(player.unit().data().health, 0);
    assert_eq!(
        player.unit().health_state_revision_like_cpp(),
        revision_before + 1
    );
    assert_eq!(player.unit().death_state(), DeathState::Alive);
}

#[test]
fn health_above_u32_is_subtracted_without_narrowing() {
    let mut player = victim(u64::from(u32::MAX) + 2, 80);
    let revision_before = player.unit().health_state_revision_like_cpp();

    assert_eq!(
        player.apply_melee_damage_batch([u32::MAX, 1, 1].into_iter()),
        Some((vec![(u32::MAX, -1), (1, -1), (1, 0)], 80)),
    );
    assert_eq!(player.unit().data().health, 0);
    assert_eq!(
        player.unit().health_state_revision_like_cpp(),
        revision_before + 3
    );
    assert_eq!(player.unit().death_state(), DeathState::Alive);
}

#[test]
fn initial_level_preserves_reachable_clamp_endpoints() {
    // The existing Unit setter admits only u8 levels; do not widen state access
    // to manufacture negative or above-byte values for this extraction.
    for level in [0, 80, u8::MAX] {
        let mut player = victim(20, level);
        assert_eq!(
            player.apply_melee_damage_batch([1].into_iter()),
            Some((vec![(1, -1)], level))
        );
        assert_eq!(player.unit().data().level, i32::from(level));
    }
}

#[test]
fn over_damage_keeps_the_existing_i32_cast_at_extreme_values() {
    for (health, damage, expected_over_damage) in [
        (1, i32::MAX as u32 + 1, i32::MAX),
        (1, u32::MAX, -2),
        (0, i32::MAX as u32 + 1, i32::MIN),
        (0, u32::MAX, -1),
    ] {
        let mut player = victim(health, 80);
        assert_eq!(
            player.apply_melee_damage_batch([damage].into_iter()),
            Some((vec![(damage, expected_over_damage)], 80)),
        );
        assert_eq!(player.unit().data().health, 0);
        assert_eq!(player.unit().death_state(), DeathState::Alive);
    }
}
