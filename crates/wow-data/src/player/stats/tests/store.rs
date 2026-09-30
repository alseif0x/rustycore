//! ObjectMgr player stat row composition and validation.

use super::super::*;
use crate::{BaseMpEntryLikeCpp, BaseMpGameTableLikeCpp};

fn base_mp_fixture() -> BaseMpGameTableLikeCpp {
    BaseMpGameTableLikeCpp::from_rows([
        BaseMpEntryLikeCpp::from_columns([
            0.0, 31.0, 0.0, 155.0, 31.0, 155.0, 31.0, 155.0, 0.0, 0.0, 155.0, 0.0,
        ]),
        BaseMpEntryLikeCpp::from_columns([
            0.0, 34.0, 0.0, 170.0, 34.0, 170.0, 34.0, 170.0, 0.0, 0.0, 170.0, 0.0,
        ]),
    ])
}

#[test]
fn combines_class_stats_race_modifiers_and_base_mp_like_cpp() {
    let store = PlayerStatsStore::from_cpp_sources(
        [(1, 5), (2, 5)],
        [(1, [3, -1, 2, 4, 0]), (2, [-2, 1, 0, -3, 5])],
        [(5, 1, [10, 11, 12, 13, 14])],
        &base_mp_fixture(),
        1,
    )
    .expect("valid C++ sources");

    assert_eq!(
        store.get(1, 5, 1),
        Some(&PlayerLevelStats {
            strength: 13,
            agility: 10,
            stamina: 14,
            intellect: 17,
            spirit: 14,
            base_mana: 155,
        })
    );
    assert_eq!(
        store.get(2, 5, 1).unwrap().primary_stats_like_cpp(),
        [8, 12, 12, 10, 19]
    );
}

#[test]
fn fills_missing_class_level_from_previous_level_like_cpp() {
    let store = PlayerStatsStore::from_cpp_sources(
        [(1, 11)],
        [(1, [0; 5])],
        [(11, 1, [1, 2, 3, 4, 5]), (11, 3, [10, 20, 30, 40, 50])],
        &base_mp_fixture(),
        3,
    )
    .expect("valid gapped C++ sources");

    assert_eq!(
        store.get(1, 11, 2).unwrap().primary_stats_like_cpp(),
        [1, 2, 3, 4, 5]
    );
    assert_eq!(
        store.get(1, 11, 3).unwrap().primary_stats_like_cpp(),
        [10, 20, 30, 40, 50]
    );
    assert_eq!(store.get(1, 11, 2).unwrap().base_mana, 34);
    assert_eq!(store.get(1, 11, 3).unwrap().base_mana, 0);
}

#[test]
fn rejects_class_without_level_one_like_cpp() {
    let error = PlayerStatsStore::from_cpp_sources(
        [(1, 5)],
        [(1, [0; 5])],
        [(5, 2, [10; 5])],
        &base_mp_fixture(),
        2,
    )
    .err()
    .expect("missing level 1 must fail");

    assert!(error.to_string().contains("level 1"));
}

#[test]
fn only_builds_and_validates_playercreateinfo_combinations_like_cpp() {
    let store = PlayerStatsStore::from_cpp_sources(
        [(1, 5)],
        [(2, [50; 5])],
        [
            (5, 1, [10, 11, 12, 13, 14]),
            // This unused class has no level-1 row. C++ never allocates
            // levelInfo for it without a matching playercreateinfo row.
            (8, 2, [20; 5]),
        ],
        &base_mp_fixture(),
        2,
    )
    .expect("unused race/class combinations must not affect integrity");

    assert_eq!(
        store.get(1, 5, 1).unwrap().primary_stats_like_cpp(),
        [10, 11, 12, 13, 14],
        "a race missing from player_racestats uses C++ zero modifiers"
    );
    assert!(store.get(2, 5, 1).is_none());
    assert!(store.get(1, 8, 2).is_none());
}
