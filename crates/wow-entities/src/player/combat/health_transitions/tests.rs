use super::Player;
use wow_constants::DeathState;

fn player_with_health(health: u64, max_health: u64, state: DeathState) -> Player {
    let mut player = Player::new(Some(7), false);
    let unit = player.unit_mut();
    unit.set_max_health(max_health);
    unit.set_health(health);
    // Retain nonzero stored health when exercising non-alive admission.
    unit.set_death_state(state);
    unit.clear_unit_data_changes();
    player
}

#[test]
fn positive_snapshot_revives_only_just_died_and_corpse() {
    for (state, expected_state) in [
        (DeathState::Alive, DeathState::Alive),
        (DeathState::JustDied, DeathState::Alive),
        (DeathState::Corpse, DeathState::Alive),
        (DeathState::Dead, DeathState::Dead),
        (DeathState::JustRespawned, DeathState::JustRespawned),
    ] {
        let mut player = player_with_health(0, 100, state);
        let revision = player.unit().health_state_revision_like_cpp();

        assert_eq!(player.apply_represented_health_snapshot(25, 100), (25, 100));
        assert_eq!(player.unit().death_state(), expected_state);
        assert_eq!(player.unit().data().health, 25);
        assert_eq!(player.unit().data().max_health, 100);
        assert_eq!(
            player.unit().health_state_revision_like_cpp(),
            revision + 1 + u64::from(state != expected_state),
        );
        assert_eq!(player.unit().is_alive(), expected_state == DeathState::Alive);
    }
}

#[test]
fn zero_snapshot_sets_corpse_before_maximum_shrink_clamps_health() {
    let mut player = player_with_health(80, 100, DeathState::Alive);
    let revision = player.unit().health_state_revision_like_cpp();

    assert_eq!(player.apply_represented_health_snapshot(0, 50), (0, 50));
    assert_eq!(player.unit().death_state(), DeathState::Corpse);
    assert_eq!(player.unit().data().health, 0);
    assert_eq!(player.unit().data().max_health, 50);
    // State, maximum, then the maximum setter's forced zero: an intermediate
    // clamp to 50 before entering Corpse would add a fourth revision.
    assert_eq!(player.unit().health_state_revision_like_cpp(), revision + 3);
    assert!(player.unit().unit_data_changes_mask().is_any_set());
}

#[test]
fn positive_snapshot_revives_before_maximum_and_final_health_writes() {
    let mut player = player_with_health(80, 100, DeathState::Corpse);
    let revision = player.unit().health_state_revision_like_cpp();

    assert_eq!(player.apply_represented_health_snapshot(50, 50), (50, 50));
    assert_eq!(player.unit().death_state(), DeathState::Alive);
    assert_eq!(player.unit().data().health, 50);
    // Revival precedes the maximum setter's clamp to 50; the final health
    // setter is then unchanged. Clamping while Corpse would write zero first.
    assert_eq!(player.unit().health_state_revision_like_cpp(), revision + 3);
}

#[test]
fn snapshot_clamps_zero_maximum_and_health_above_maximum() {
    for (health, max_health, expected) in [
        (u32::MAX, 0, (1, 1)),
        (0, 0, (0, 1)),
        (u32::MAX, 17, (17, 17)),
        (u32::MAX, u32::MAX, (u32::MAX, u32::MAX)),
    ] {
        let mut player = player_with_health(20, 100, DeathState::Alive);

        assert_eq!(player.apply_represented_health_snapshot(health, max_health), expected);
        assert_eq!(player.unit().data().health, u64::from(expected.0));
        assert_eq!(player.unit().data().max_health, u64::from(expected.1));
        assert_eq!(
            player.unit().death_state(),
            if health == 0 { DeathState::Corpse } else { DeathState::Alive },
        );
    }
}

#[test]
fn identical_snapshot_does_not_advance_revision_or_dirty_unit_fields() {
    for (health, state) in [(20, DeathState::Alive), (0, DeathState::Corpse)] {
        let mut player = player_with_health(health, 100, state);
        let revision = player.unit().health_state_revision_like_cpp();

        assert_eq!(player.apply_represented_health_snapshot(health as u32, 100), (health as u32, 100));
        assert_eq!(player.unit().health_state_revision_like_cpp(), revision);
        assert!(!player.unit().unit_data_changes_mask().is_any_set());
    }
}

#[test]
fn snapshot_replaces_extreme_u64_health_with_the_bounded_pair() {
    let mut player = player_with_health(u64::MAX, u64::MAX, DeathState::Alive);
    let revision = player.unit().health_state_revision_like_cpp();

    assert_eq!(
        player.apply_represented_health_snapshot(u32::MAX, u32::MAX),
        (u32::MAX, u32::MAX),
    );
    assert_eq!(player.unit().data().health, u64::from(u32::MAX));
    assert_eq!(player.unit().data().max_health, u64::from(u32::MAX));
    assert_eq!(player.unit().health_state_revision_like_cpp(), revision + 2);
}

#[test]
fn damage_rejects_non_alive_states_without_health_or_revision_writes() {
    for state in [
        DeathState::JustDied,
        DeathState::Corpse,
        DeathState::Dead,
        DeathState::JustRespawned,
    ] {
        let mut player = player_with_health(u64::MAX, u64::MAX, state);
        let revision = player.unit().health_state_revision_like_cpp();

        assert_eq!(
            player.apply_represented_damage(u32::MAX, DeathState::Corpse),
            (u32::MAX, u32::MAX, u32::MAX, 0, false),
        );
        assert_eq!(player.unit().data().health, u64::MAX);
        assert_eq!(player.unit().data().max_health, u64::MAX);
        assert_eq!(player.unit().death_state(), state);
        assert_eq!(player.unit().health_state_revision_like_cpp(), revision);
        assert!(!player.unit().unit_data_changes_mask().is_any_set());
    }
}

#[test]
fn alive_zero_health_rejects_damage_without_a_death_transition() {
    let mut player = player_with_health(0, 100, DeathState::Alive);
    let revision = player.unit().health_state_revision_like_cpp();

    assert_eq!(player.apply_represented_damage(20, DeathState::JustDied), (0, 0, 100, 0, false));
    assert_eq!(player.unit().death_state(), DeathState::Alive);
    assert_eq!(player.unit().health_state_revision_like_cpp(), revision);
    assert!(!player.unit().unit_data_changes_mask().is_any_set());
}

#[test]
fn zero_damage_keeps_normal_health_but_still_normalizes_extreme_health() {
    for (health, max_health, expected_health, revision_delta) in [
        (20, 100, 20, 0),
        (u64::MAX, u64::MAX, u64::from(u32::MAX), 1),
    ] {
        let mut player = player_with_health(health, max_health, DeathState::Alive);
        let revision = player.unit().health_state_revision_like_cpp();
        let bounded_max = max_health.min(u64::from(u32::MAX)) as u32;
        let bounded_health = expected_health as u32;

        assert_eq!(
            player.apply_represented_damage(0, DeathState::Corpse),
            (bounded_health, bounded_health, bounded_max, 0, false),
        );
        assert_eq!(player.unit().data().health, expected_health);
        assert_eq!(player.unit().data().max_health, max_health);
        assert_eq!(player.unit().death_state(), DeathState::Alive);
        assert_eq!(player.unit().health_state_revision_like_cpp(), revision + revision_delta);
        assert_eq!(player.unit().unit_data_changes_mask().is_any_set(), revision_delta != 0);
    }
}

#[test]
fn successive_damage_reads_updated_health_and_saturates_applied_damage() {
    let mut player = player_with_health(20, 100, DeathState::Alive);
    let revision = player.unit().health_state_revision_like_cpp();

    assert_eq!(player.apply_represented_damage(7, DeathState::Corpse), (20, 13, 100, 7, false));
    assert_eq!(player.unit().death_state(), DeathState::Alive);
    assert_eq!(player.unit().health_state_revision_like_cpp(), revision + 1);
    assert_eq!(player.apply_represented_damage(u32::MAX, DeathState::Corpse), (13, 0, 100, 13, true));
    assert_eq!(player.unit().death_state(), DeathState::Corpse);
    assert_eq!(player.unit().health_state_revision_like_cpp(), revision + 3);

    player.unit_mut().clear_unit_data_changes();
    assert_eq!(player.apply_represented_damage(1, DeathState::Dead), (0, 0, 100, 0, false));
    assert_eq!(player.unit().death_state(), DeathState::Corpse);
    assert_eq!(player.unit().health_state_revision_like_cpp(), revision + 3);
    assert!(!player.unit().unit_data_changes_mask().is_any_set());
}

#[test]
fn lethal_damage_preserves_the_requested_state_including_alive() {
    for state in [
        DeathState::Alive,
        DeathState::JustDied,
        DeathState::Corpse,
        DeathState::Dead,
        DeathState::JustRespawned,
    ] {
        let mut player = player_with_health(20, 100, DeathState::Alive);
        let revision = player.unit().health_state_revision_like_cpp();

        assert_eq!(player.apply_represented_damage(20, state), (20, 0, 100, 20, true));
        assert_eq!(player.unit().death_state(), state);
        assert_eq!(player.unit().data().health, 0);
        assert_eq!(player.unit().data().max_health, 100);
        assert_eq!(
            player.unit().health_state_revision_like_cpp(),
            revision + 1 + u64::from(state != DeathState::Alive),
        );
    }
}

#[test]
fn damage_caps_u64_snapshot_without_rewriting_canonical_maximum() {
    let mut player = player_with_health(u64::MAX, u64::MAX, DeathState::Alive);
    let revision = player.unit().health_state_revision_like_cpp();

    assert_eq!(
        player.apply_represented_damage(1, DeathState::JustDied),
        (u32::MAX, u32::MAX - 1, u32::MAX, 1, false),
    );
    assert_eq!(player.unit().data().health, u64::from(u32::MAX - 1));
    assert_eq!(player.unit().data().max_health, u64::MAX);
    assert_eq!(player.unit().death_state(), DeathState::Alive);
    assert_eq!(player.unit().health_state_revision_like_cpp(), revision + 1);
}
