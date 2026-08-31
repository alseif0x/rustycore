//! Composition boundary for the C++ GameTele World catalog.

use anyhow::{Result, bail};
use wow_persistence::{
    GameTeleCatalogLoadOutcomeLikeCpp, GameTeleCatalogPersistencePortLikeCpp,
    GameTelePersistenceRowLikeCpp,
};

fn domain_row_like_cpp(row: GameTelePersistenceRowLikeCpp) -> wow_data::GameTeleRowLikeCpp {
    wow_data::GameTeleRowLikeCpp {
        id: row.id,
        position_x: row.position_x,
        position_y: row.position_y,
        position_z: row.position_z,
        orientation: row.orientation,
        map_id: row.map_id,
        name: row.name,
    }
}

async fn load_domain_rows_like_cpp(
    persistence: &dyn GameTeleCatalogPersistencePortLikeCpp,
) -> Result<Vec<wow_data::GameTeleRowLikeCpp>> {
    match persistence.load_rows_like_cpp().await {
        GameTeleCatalogLoadOutcomeLikeCpp::Loaded(rows) => {
            Ok(rows.into_iter().map(domain_row_like_cpp).collect())
        }
        GameTeleCatalogLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    }
}

pub(super) async fn load_game_tele_catalog_like_cpp(
    persistence: &dyn GameTeleCatalogPersistencePortLikeCpp,
) -> Result<wow_data::GameTeleLoadOutcomeLikeCpp> {
    let rows = load_domain_rows_like_cpp(persistence).await?;
    Ok(wow_data::GameTeleStoreLikeCpp::from_rows_like_cpp(rows))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_persistence::PersistenceFutureLikeCpp;

    struct FixedPort {
        outcome: GameTeleCatalogLoadOutcomeLikeCpp,
    }

    impl GameTeleCatalogPersistencePortLikeCpp for FixedPort {
        fn load_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<'_, GameTeleCatalogLoadOutcomeLikeCpp> {
            Box::pin(async move { self.outcome.clone() })
        }
    }

    #[tokio::test]
    async fn typed_row_preserves_every_domain_field() {
        let rows = load_domain_rows_like_cpp(&FixedPort {
            outcome: GameTeleCatalogLoadOutcomeLikeCpp::Loaded(vec![
                GameTelePersistenceRowLikeCpp {
                    id: 7,
                    position_x: 1.25,
                    position_y: -2.5,
                    position_z: 3.75,
                    orientation: 4.5,
                    map_id: 571,
                    name: "Dalaran".into(),
                },
            ]),
        })
        .await
        .unwrap();

        assert_eq!(
            rows,
            [wow_data::GameTeleRowLikeCpp {
                id: 7,
                position_x: 1.25,
                position_y: -2.5,
                position_z: 3.75,
                orientation: 4.5,
                map_id: 571,
                name: "Dalaran".into(),
            }]
        );
    }

    #[tokio::test]
    async fn empty_success_remains_a_successful_empty_catalog() {
        let outcome = load_game_tele_catalog_like_cpp(&FixedPort {
            outcome: GameTeleCatalogLoadOutcomeLikeCpp::Loaded(Vec::new()),
        })
        .await
        .unwrap();
        assert!(outcome.store.is_empty());
        assert_eq!(outcome.report.rows_seen, 0);
    }

    #[tokio::test]
    async fn failure_preserves_existing_startup_fatal_policy() {
        let error = load_game_tele_catalog_like_cpp(&FixedPort {
            outcome: GameTeleCatalogLoadOutcomeLikeCpp::Failed {
                reason: "world read failed".into(),
            },
        })
        .await
        .unwrap_err();
        assert_eq!(error.to_string(), "world read failed");
    }
}
