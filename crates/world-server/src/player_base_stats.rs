//! Composition boundary for C++ player base-stat World sources.

use std::path::Path;

use anyhow::{Result, bail};
use wow_persistence::{
    PlayerBaseStatsLoadOutcomeLikeCpp, PlayerBaseStatsPersistencePortLikeCpp,
    PlayerClassLevelStatsPersistenceRowLikeCpp, PlayerRaceStatsPersistenceRowLikeCpp,
};

fn race_row_like_cpp(
    row: PlayerRaceStatsPersistenceRowLikeCpp,
) -> wow_data::PlayerRaceStatsRowLikeCpp {
    wow_data::PlayerRaceStatsRowLikeCpp {
        race: row.race,
        stat_modifiers: row.stat_modifiers,
    }
}

fn class_level_row_like_cpp(
    row: PlayerClassLevelStatsPersistenceRowLikeCpp,
) -> wow_data::PlayerClassLevelStatsRowLikeCpp {
    wow_data::PlayerClassLevelStatsRowLikeCpp {
        class: row.class,
        level: row.level,
        primary_stats: row.primary_stats,
    }
}

fn compose_race_rows_like_cpp(
    outcome: PlayerBaseStatsLoadOutcomeLikeCpp<PlayerRaceStatsPersistenceRowLikeCpp>,
) -> Result<wow_data::PlayerRaceStatsRowsLikeCpp> {
    match outcome {
        PlayerBaseStatsLoadOutcomeLikeCpp::Loaded(rows) => {
            wow_data::PlayerRaceStatsRowsLikeCpp::try_from_rows_like_cpp(
                rows.into_iter().map(race_row_like_cpp),
            )
        }
        PlayerBaseStatsLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    }
}

fn compose_class_level_rows_like_cpp(
    outcome: PlayerBaseStatsLoadOutcomeLikeCpp<PlayerClassLevelStatsPersistenceRowLikeCpp>,
) -> Result<wow_data::PlayerClassLevelStatsRowsLikeCpp> {
    match outcome {
        PlayerBaseStatsLoadOutcomeLikeCpp::Loaded(rows) => {
            wow_data::PlayerClassLevelStatsRowsLikeCpp::try_from_rows_like_cpp(
                rows.into_iter().map(class_level_row_like_cpp),
            )
        }
        PlayerBaseStatsLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    }
}

pub(super) async fn load_player_base_stats_like_cpp(
    persistence: &dyn PlayerBaseStatsPersistencePortLikeCpp,
    data_dir: impl AsRef<Path>,
    max_player_level: u8,
    valid_race_classes: &[(u8, u8)],
) -> Result<wow_data::PlayerStatsStore> {
    let race_rows =
        compose_race_rows_like_cpp(persistence.load_player_race_stats_rows_like_cpp().await)?;
    let class_level_rows = compose_class_level_rows_like_cpp(
        persistence
            .load_player_class_level_stats_rows_like_cpp()
            .await,
    )?;

    wow_data::PlayerStatsStore::load_from_validated_rows_like_cpp(
        data_dir,
        max_player_level,
        valid_race_classes,
        race_rows,
        class_level_rows,
    )
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use wow_persistence::PersistenceFutureLikeCpp;

    #[test]
    fn typed_rows_preserve_all_player_base_stat_fields() {
        assert_eq!(
            race_row_like_cpp(PlayerRaceStatsPersistenceRowLikeCpp {
                race: 7,
                stat_modifiers: [-1, 2, -3, 4, -5],
            }),
            wow_data::PlayerRaceStatsRowLikeCpp {
                race: 7,
                stat_modifiers: [-1, 2, -3, 4, -5],
            }
        );
        assert_eq!(
            class_level_row_like_cpp(PlayerClassLevelStatsPersistenceRowLikeCpp {
                class: 5,
                level: 80,
                primary_stats: [1, 2, 3, 4, u16::MAX],
            }),
            wow_data::PlayerClassLevelStatsRowLikeCpp {
                class: 5,
                level: 80,
                primary_stats: [1, 2, 3, 4, u16::MAX],
            }
        );
    }

    struct RecordingPort {
        race_calls: AtomicUsize,
        class_calls: AtomicUsize,
        race_outcome: PlayerBaseStatsLoadOutcomeLikeCpp<PlayerRaceStatsPersistenceRowLikeCpp>,
        class_outcome:
            PlayerBaseStatsLoadOutcomeLikeCpp<PlayerClassLevelStatsPersistenceRowLikeCpp>,
    }

    impl PlayerBaseStatsPersistencePortLikeCpp for RecordingPort {
        fn load_player_race_stats_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            PlayerBaseStatsLoadOutcomeLikeCpp<PlayerRaceStatsPersistenceRowLikeCpp>,
        > {
            self.race_calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { self.race_outcome.clone() })
        }

        fn load_player_class_level_stats_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            PlayerBaseStatsLoadOutcomeLikeCpp<PlayerClassLevelStatsPersistenceRowLikeCpp>,
        > {
            self.class_calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { self.class_outcome.clone() })
        }
    }

    #[tokio::test]
    async fn empty_race_batch_stops_before_class_query_and_basemp_like_cpp() {
        let port = RecordingPort {
            race_calls: AtomicUsize::new(0),
            class_calls: AtomicUsize::new(0),
            race_outcome: PlayerBaseStatsLoadOutcomeLikeCpp::Loaded(Vec::new()),
            class_outcome: PlayerBaseStatsLoadOutcomeLikeCpp::Loaded(vec![
                PlayerClassLevelStatsPersistenceRowLikeCpp {
                    class: 1,
                    level: 1,
                    primary_stats: [1; 5],
                },
            ]),
        };

        let result =
            load_player_base_stats_like_cpp(&port, "/path/that/must/not/be/read", 80, &[(1, 1)])
                .await;
        let error = match result {
            Ok(_) => panic!("empty race-stat batch must not publish player stats"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("player_racestats is empty"));
        assert_eq!(port.race_calls.load(Ordering::SeqCst), 1);
        assert_eq!(port.class_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn failed_class_stage_stops_before_basemp_or_store_publication() {
        let port = RecordingPort {
            race_calls: AtomicUsize::new(0),
            class_calls: AtomicUsize::new(0),
            race_outcome: PlayerBaseStatsLoadOutcomeLikeCpp::Loaded(vec![
                PlayerRaceStatsPersistenceRowLikeCpp {
                    race: 1,
                    stat_modifiers: [0; 5],
                },
            ]),
            class_outcome: PlayerBaseStatsLoadOutcomeLikeCpp::Failed {
                reason: "class-level read failed".into(),
            },
        };

        let result =
            load_player_base_stats_like_cpp(&port, "/path/that/must/not/be/read", 80, &[(1, 1)])
                .await;
        let error = match result {
            Ok(_) => panic!("failed class stage must not publish player stats"),
            Err(error) => error,
        };
        assert_eq!(error.to_string(), "class-level read failed");
        assert_eq!(port.race_calls.load(Ordering::SeqCst), 1);
        assert_eq!(port.class_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn app_composes_one_adapter_at_the_existing_player_stats_stage() {
        let source = include_str!("app.rs");
        assert_eq!(
            source
                .matches("MariaDbPlayerBaseStatsPersistenceAdapterLikeCpp::new")
                .count(),
            1
        );
        assert_eq!(source.matches("load_player_base_stats_like_cpp").count(), 1);
    }
}
