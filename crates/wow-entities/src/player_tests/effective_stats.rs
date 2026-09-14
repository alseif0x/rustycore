use crate::{Player, PlayerEffectiveCombatStatsLikeCpp};

#[test]
fn effective_combat_stats_are_player_owned_and_replace_as_one_snapshot() {
    let mut player = Player::new(None, false);
    assert_eq!(
        player.effective_combat_stats_like_cpp(),
        &PlayerEffectiveCombatStatsLikeCpp::default()
    );

    let mut stats = PlayerEffectiveCombatStatsLikeCpp::default();
    stats.stats = [10, 20, 30, 40, 50];
    stats.attack_power = 123;
    stats.resistances[2] = 17;
    player.replace_effective_combat_stats_like_cpp(stats);
    assert_eq!(player.effective_combat_stats_like_cpp(), &stats);

    player.clear_effective_combat_stats_like_cpp();
    assert_eq!(
        player.effective_combat_stats_like_cpp(),
        &PlayerEffectiveCombatStatsLikeCpp::default()
    );
}

#[test]
fn total_attack_power_matches_cpp_non_negative_modifier_and_multiplier_order() {
    let mut player = Player::new(None, false);
    let mut stats = PlayerEffectiveCombatStatsLikeCpp {
        attack_power: 120,
        attack_power_mod_pos: 15,
        attack_power_mod_neg: -5,
        attack_power_multiplier: 0.25,
        ranged_attack_power: 80,
        ranged_attack_power_mod_pos: 10,
        ranged_attack_power_mod_neg: -100,
        ranged_attack_power_multiplier: 0.5,
        ..Default::default()
    };
    player.replace_effective_combat_stats_like_cpp(stats);

    assert_eq!(player.total_attack_power_like_cpp(), 162.5);
    // C++ clamps a negative pre-multiplier total to zero.
    stats.ranged_attack_power_mod_neg = -200;
    player.replace_effective_combat_stats_like_cpp(stats);
    assert_eq!(player.total_ranged_attack_power_like_cpp(), 0.0);
}
