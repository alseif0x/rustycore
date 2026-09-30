use super::{DeathState, Player, PowerType};

fn player_with_vitals(health: u64, maximum: u64, state: DeathState) -> Player {
    let mut player = Player::new(Some(7), false);
    let unit = player.unit_mut();
    unit.set_max_health(maximum);
    unit.set_health(health);
    for (power, index, max, current) in [
        (PowerType::Mana, 0, 200, 30),
        (PowerType::Rage, 1, 100, 40),
        (PowerType::Energy, 2, 120, 50),
        (PowerType::Focus, 3, 80, 60),
    ] {
        unit.set_power_index(power, Some(index));
        unit.set_max_power(power, max);
        unit.set_power(power, current);
    }
    unit.set_death_state(state);
    unit.clear_unit_data_changes();
    player
}

fn powers(player: &Player) -> [i32; 4] {
    [
        player.get_power(PowerType::Mana),
        player.get_power(PowerType::Rage),
        player.get_power(PowerType::Energy),
        player.get_power(PowerType::Focus),
    ]
}

#[test]
fn self_resurrection_rejects_alive_even_with_zero_health_without_mutation() {
    for health in [0, 40] {
        let mut player = player_with_vitals(health, 100, DeathState::Alive);
        let revision = player.unit().health_state_revision_like_cpp();

        assert_eq!(player.apply_self_resurrection(i32::MIN, i32::MAX), None);
        assert_eq!(player.unit().data().health, health);
        assert_eq!(player.unit().data().max_health, 100);
        assert_eq!(player.unit().death_state(), DeathState::Alive);
        assert_eq!(powers(&player), [30, 40, 50, 60]);
        assert_eq!(player.unit().health_state_revision_like_cpp(), revision);
        assert!(!player.unit().unit_data_changes_mask().is_any_set());
    }
}

#[test]
fn self_resurrection_revives_all_non_alive_states_before_writing_health() {
    for state in [
        DeathState::JustDied,
        DeathState::Corpse,
        DeathState::Dead,
        DeathState::JustRespawned,
    ] {
        let mut player = player_with_vitals(0, 100, state);
        let revision = player.unit().health_state_revision_like_cpp();

        assert_eq!(player.apply_self_resurrection(-35, 77), Some(35));
        assert_eq!(player.unit().death_state(), DeathState::Alive);
        assert_eq!(player.unit().data().health, 35);
        assert_eq!(player.unit().data().max_health, 100);
        assert_eq!(powers(&player), [77, 0, 120, 0]);
        // State then health; power writes add no health-state revisions.
        assert_eq!(player.unit().health_state_revision_like_cpp(), revision + 2);
    }
}

#[test]
fn self_flat_extremes_preserve_saturating_abs_and_setter_clamps() {
    for (damage, misc_value, maximum, health, mana) in [
        (-1, i32::MIN, 100, 1, 0),
        (-200, i32::MAX, 100, 100, 200),
        (i32::MIN, i32::MAX, u64::MAX, i32::MAX as u32, 200),
    ] {
        let mut player = player_with_vitals(0, maximum, DeathState::Corpse);

        assert_eq!(
            player.apply_self_resurrection(damage, misc_value),
            Some(health)
        );
        assert_eq!(player.unit().data().health, u64::from(health));
        assert_eq!(player.unit().data().max_health, maximum);
        assert_eq!(powers(&player), [mana, 0, 120, 0]);
    }
}

#[test]
fn self_percentage_keeps_integer_saturation_before_division() {
    for (maximum, pct, max_mana, health, mana) in [
        (400, 25, 200, 100, 50),
        (100, 200, 200, 100, 200),
        (u64::MAX, 100, i32::MAX, 42_949_672, 21_474_836),
        (u64::MAX, i32::MAX, i32::MAX, 42_949_672, 21_474_836),
    ] {
        let mut player = player_with_vitals(0, maximum, DeathState::Dead);
        player.unit_mut().set_max_power(PowerType::Mana, max_mana);

        assert_eq!(player.apply_self_resurrection(pct, 999), Some(health));
        assert_eq!(player.unit().data().health, u64::from(health));
        assert_eq!(player.unit().data().max_health, maximum);
        assert_eq!(powers(&player), [mana, 0, 120, 0]);
    }
}

#[test]
fn self_zero_percentage_keeps_alive_zero_health_and_lifecycle_gaps() {
    let mut player = player_with_vitals(20, 100, DeathState::Corpse);
    player.resurrection_state_mut_like_cpp().death_timer_active = true;
    player
        .resurrection_state_mut_like_cpp()
        .self_res_spells
        .insert(21169);
    let lifecycle = player.resurrection_state_like_cpp().clone();
    let revision = player.unit().health_state_revision_like_cpp();

    assert_eq!(player.apply_self_resurrection(0, 999), Some(0));
    assert_eq!(player.unit().death_state(), DeathState::Alive);
    assert_eq!(player.unit().data().health, 0);
    assert_eq!(powers(&player), [0, 0, 120, 0]);
    assert_eq!(player.unit().health_state_revision_like_cpp(), revision + 2);
    assert_eq!(player.resurrection_state_like_cpp(), &lifecycle);
}

#[test]
fn percentage_revives_before_health_and_accepts_an_already_alive_player() {
    for state in [
        DeathState::Alive,
        DeathState::JustDied,
        DeathState::Corpse,
        DeathState::Dead,
        DeathState::JustRespawned,
    ] {
        let mut player = player_with_vitals(0, 100, state);
        let revision = player.unit().health_state_revision_like_cpp();

        assert_eq!(player.apply_percentage_resurrection(0.5), 50);
        assert_eq!(player.unit().death_state(), DeathState::Alive);
        assert_eq!(player.unit().data().health, 50);
        assert_eq!(player.unit().data().max_health, 100);
        assert_eq!(powers(&player), [100, 0, 60, 40]);
        assert_eq!(
            player.unit().health_state_revision_like_cpp(),
            revision + 1 + u64::from(state != DeathState::Alive),
        );
    }
}

#[test]
fn percentage_truncates_fractional_health_and_each_power() {
    let mut player = player_with_vitals(0, 101, DeathState::Corpse);
    player.unit_mut().set_max_power(PowerType::Mana, 201);
    player.unit_mut().set_max_power(PowerType::Energy, 121);
    player.unit_mut().set_max_power(PowerType::Focus, 81);

    assert_eq!(player.apply_percentage_resurrection(0.5), 50);
    assert_eq!(player.unit().data().health, 50);
    assert_eq!(player.unit().data().max_health, 101);
    assert_eq!(powers(&player), [100, 0, 60, 40]);
}

#[test]
fn percentage_nonpositive_and_nan_still_revive_and_zero_powers() {
    for pct in [0.0, -0.5, f32::NEG_INFINITY, f32::NAN] {
        let mut player = player_with_vitals(20, 100, DeathState::Dead);
        player.resurrection_state_mut_like_cpp().death_timer_active = true;
        let lifecycle = player.resurrection_state_like_cpp().clone();
        let revision = player.unit().health_state_revision_like_cpp();

        assert_eq!(player.apply_percentage_resurrection(pct), 0);
        assert_eq!(player.unit().death_state(), DeathState::Alive);
        assert_eq!(player.unit().data().health, 0);
        assert_eq!(player.unit().data().max_health, 100);
        assert_eq!(powers(&player), [0, 0, 0, 0]);
        assert_eq!(player.unit().health_state_revision_like_cpp(), revision + 2);
        assert_eq!(player.resurrection_state_like_cpp(), &lifecycle);
    }
}

#[test]
fn percentage_above_one_and_infinity_use_existing_power_setter_caps() {
    for pct in [1.5, f32::INFINITY] {
        let mut player = player_with_vitals(0, 100, DeathState::Corpse);

        assert_eq!(player.apply_percentage_resurrection(pct), 100);
        assert_eq!(player.unit().data().health, 100);
        assert_eq!(player.unit().data().max_health, 100);
        assert_eq!(powers(&player), [200, 0, 120, 80]);
    }
}

#[test]
fn percentage_retains_f32_rounding_at_extreme_integer_bounds() {
    for (pct, health, power) in [
        (0.5, 2_147_483_648, 1_073_741_824),
        (1.0, u32::MAX, i32::MAX),
    ] {
        let mut player = player_with_vitals(0, u64::MAX, DeathState::Corpse);
        for kind in [PowerType::Mana, PowerType::Energy, PowerType::Focus] {
            player.unit_mut().set_max_power(kind, i32::MAX);
        }

        assert_eq!(player.apply_percentage_resurrection(pct), health);
        assert_eq!(player.unit().data().health, u64::from(health));
        assert_eq!(player.unit().data().max_health, u64::MAX);
        assert_eq!(powers(&player), [power, 0, power, power]);
    }
}

#[test]
fn absent_power_slots_do_not_block_either_health_transition() {
    let mut self_player = player_with_vitals(0, 100, DeathState::Corpse);
    let mut percent_player = player_with_vitals(0, 100, DeathState::Corpse);
    for player in [&mut self_player, &mut percent_player] {
        for kind in [PowerType::Mana, PowerType::Energy, PowerType::Focus] {
            player.unit_mut().set_power_index(kind, None);
        }
        player.unit_mut().clear_unit_data_changes();
    }

    assert_eq!(self_player.apply_self_resurrection(-10, 33), Some(10));
    assert_eq!(percent_player.apply_percentage_resurrection(0.5), 50);
    for player in [&self_player, &percent_player] {
        assert_eq!(powers(player), [0, 0, 0, 0]);
        assert_eq!(player.unit().death_state(), DeathState::Alive);
        assert_eq!(player.unit().data().max_health, 100);
        assert_eq!(player.get_power_index(PowerType::Mana), None);
        assert_eq!(player.get_power_index(PowerType::Energy), None);
        assert_eq!(player.get_power_index(PowerType::Focus), None);
    }
}

#[test]
fn power_only_changes_do_not_advance_the_health_revision() {
    let mut player = player_with_vitals(50, 100, DeathState::Alive);
    let revision = player.unit().health_state_revision_like_cpp();

    assert_eq!(player.apply_percentage_resurrection(0.5), 50);
    assert_eq!(powers(&player), [100, 0, 60, 40]);
    assert_eq!(player.unit().data().health, 50);
    assert_eq!(player.unit().data().max_health, 100);
    assert_eq!(player.unit().health_state_revision_like_cpp(), revision);
    assert!(player.unit().unit_data_changes_mask().is_any_set());
}

#[test]
fn both_algorithms_keep_the_existing_negative_power_maximum_setter_behavior() {
    let mut self_player = player_with_vitals(0, 100, DeathState::Corpse);
    let mut percent_player = player_with_vitals(0, 100, DeathState::Corpse);
    for player in [&mut self_player, &mut percent_player] {
        for (kind, maximum) in [
            (PowerType::Mana, -7),
            (PowerType::Rage, -9),
            (PowerType::Energy, -11),
            (PowerType::Focus, -13),
        ] {
            player.unit_mut().set_max_power(kind, maximum);
        }
    }

    assert_eq!(self_player.apply_self_resurrection(-10, -33), Some(10));
    assert_eq!(percent_player.apply_percentage_resurrection(0.5), 50);
    assert_eq!(powers(&self_player), [-7, -9, -11, -13]);
    assert_eq!(powers(&percent_player), [-7, -9, -11, -13]);
}
