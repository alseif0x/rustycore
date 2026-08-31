//! Composition boundary for C++ `ObjectMgr::LoadExplorationBaseXP`.

use anyhow::{Result, bail};
use tracing::info;
use wow_persistence::{
    ExplorationBaseXpCatalogLoadOutcomeLikeCpp, ExplorationBaseXpCatalogPersistencePortLikeCpp,
    ExplorationBaseXpPersistenceRowLikeCpp,
};

fn domain_row_like_cpp(
    row: ExplorationBaseXpPersistenceRowLikeCpp,
) -> wow_data::ExplorationBaseXpRowLikeCpp {
    wow_data::ExplorationBaseXpRowLikeCpp {
        level: row.level,
        base_xp: row.base_xp,
    }
}

async fn load_domain_rows_like_cpp(
    persistence: &dyn ExplorationBaseXpCatalogPersistencePortLikeCpp,
) -> Result<Vec<wow_data::ExplorationBaseXpRowLikeCpp>> {
    match persistence.load_rows_like_cpp().await {
        ExplorationBaseXpCatalogLoadOutcomeLikeCpp::Loaded(rows) => {
            Ok(rows.into_iter().map(domain_row_like_cpp).collect())
        }
        ExplorationBaseXpCatalogLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    }
}

pub(super) async fn load_exploration_base_xp_catalog_like_cpp(
    persistence: &dyn ExplorationBaseXpCatalogPersistencePortLikeCpp,
) -> Result<wow_data::ExplorationBaseXpStoreLikeCpp> {
    let rows = load_domain_rows_like_cpp(persistence).await?;
    let store = wow_data::ExplorationBaseXpStoreLikeCpp::from_rows_like_cpp(rows);
    info!("Loaded {} BaseXP definitions", store.len());
    Ok(store)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_persistence::PersistenceFutureLikeCpp;

    struct FixedPort {
        outcome: ExplorationBaseXpCatalogLoadOutcomeLikeCpp,
    }

    impl ExplorationBaseXpCatalogPersistencePortLikeCpp for FixedPort {
        fn load_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<'_, ExplorationBaseXpCatalogLoadOutcomeLikeCpp> {
            Box::pin(async move { self.outcome.clone() })
        }
    }

    #[tokio::test]
    async fn typed_row_preserves_level_and_wrapped_base_xp() {
        let rows = load_domain_rows_like_cpp(&FixedPort {
            outcome: ExplorationBaseXpCatalogLoadOutcomeLikeCpp::Loaded(vec![
                ExplorationBaseXpPersistenceRowLikeCpp {
                    level: 80,
                    base_xp: u32::MAX,
                },
            ]),
        })
        .await
        .unwrap();

        assert_eq!(
            rows,
            [wow_data::ExplorationBaseXpRowLikeCpp {
                level: 80,
                base_xp: u32::MAX,
            }]
        );
    }

    #[tokio::test]
    async fn empty_success_remains_a_successful_empty_catalog() {
        let store = load_exploration_base_xp_catalog_like_cpp(&FixedPort {
            outcome: ExplorationBaseXpCatalogLoadOutcomeLikeCpp::Loaded(Vec::new()),
        })
        .await
        .unwrap();
        assert!(store.is_empty());
    }

    #[tokio::test]
    async fn failure_preserves_existing_startup_fatal_policy() {
        let error = load_exploration_base_xp_catalog_like_cpp(&FixedPort {
            outcome: ExplorationBaseXpCatalogLoadOutcomeLikeCpp::Failed {
                reason: "world read failed".into(),
            },
        })
        .await
        .unwrap_err();
        assert_eq!(error.to_string(), "world read failed");
    }
}
