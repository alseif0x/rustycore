use crate::handlers::test_support::world::make_session;
use crate::test_fixtures::set_loaded_player_identity_like_cpp;
use std::sync::Arc;
use wow_data::{PlayerLevelStats, PlayerStatsStore};

#[test]
fn level_up_deltas_use_cpp_class_race_stats_and_base_mp() {
    let (mut session, _send_rx) = make_session();
    set_loaded_player_identity_like_cpp(&mut session, 571, 1, 5, 1, 0);
    session.set_player_stats(Arc::new(PlayerStatsStore::from_entries([
        (
            (1, 5, 1),
            PlayerLevelStats {
                strength: 10,
                agility: 11,
                stamina: 12,
                intellect: 13,
                spirit: 14,
                base_mana: 155,
            },
        ),
        (
            (1, 5, 2),
            PlayerLevelStats {
                strength: 12,
                agility: 11,
                stamina: 15,
                intellect: 17,
                spirit: 19,
                base_mana: 170,
            },
        ),
    ])));

    assert_eq!(
        session.level_up_stat_deltas_like_cpp(2),
        Some((15, [2, 0, 3, 4, 5]))
    );
    assert_eq!(session.level_up_stat_deltas_like_cpp(3), None);
}
