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
