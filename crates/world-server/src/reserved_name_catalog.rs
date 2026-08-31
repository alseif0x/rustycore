//! Composition boundary for C++ `ObjectMgr::LoadReservedPlayersNames`.

use anyhow::{Result, bail};
use wow_persistence::{
    ReservedNameCatalogLoadOutcomeLikeCpp, ReservedNameCatalogPersistencePortLikeCpp,
    ReservedNamePersistenceRowLikeCpp,
};

fn domain_name_like_cpp(row: ReservedNamePersistenceRowLikeCpp) -> String {
    row.name
}

async fn load_domain_names_like_cpp(
    persistence: &dyn ReservedNameCatalogPersistencePortLikeCpp,
) -> Result<Vec<String>> {
    match persistence.load_rows_like_cpp().await {
        ReservedNameCatalogLoadOutcomeLikeCpp::Loaded(rows) => {
            Ok(rows.into_iter().map(domain_name_like_cpp).collect())
        }
        ReservedNameCatalogLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    }
}

pub(super) async fn load_reserved_name_catalog_like_cpp(
    persistence: &dyn ReservedNameCatalogPersistencePortLikeCpp,
) -> Result<wow_data::ReservedNameStoreLikeCpp> {
    let names = load_domain_names_like_cpp(persistence).await?;
    Ok(wow_data::ReservedNameStoreLikeCpp::from_names_like_cpp(
        names,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_persistence::PersistenceFutureLikeCpp;

    struct FixedPort {
        outcome: ReservedNameCatalogLoadOutcomeLikeCpp,
    }

    impl ReservedNameCatalogPersistencePortLikeCpp for FixedPort {
        fn load_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<'_, ReservedNameCatalogLoadOutcomeLikeCpp> {
            Box::pin(async move { self.outcome.clone() })
        }
    }

    #[tokio::test]
    async fn typed_names_preserve_normalization_and_duplicate_accounting() {
        let store = load_reserved_name_catalog_like_cpp(&FixedPort {
            outcome: ReservedNameCatalogLoadOutcomeLikeCpp::Loaded(vec![
                ReservedNamePersistenceRowLikeCpp {
                    name: "Arthas".into(),
                },
                ReservedNamePersistenceRowLikeCpp {
                    name: "arthas".into(),
                },
            ]),
        })
        .await
        .unwrap();

        assert!(store.is_reserved_name_like_cpp("ARTHAS"));
        assert_eq!(store.loaded_rows_like_cpp(), 2);
        assert_eq!(store.len(), 1);
    }

    #[tokio::test]
    async fn empty_success_remains_a_successful_empty_catalog() {
        let store = load_reserved_name_catalog_like_cpp(&FixedPort {
            outcome: ReservedNameCatalogLoadOutcomeLikeCpp::Loaded(Vec::new()),
        })
        .await
        .unwrap();
        assert!(store.is_empty());
        assert_eq!(store.loaded_rows_like_cpp(), 0);
    }

    #[tokio::test]
    async fn failure_preserves_existing_startup_fatal_policy() {
        let error = load_reserved_name_catalog_like_cpp(&FixedPort {
            outcome: ReservedNameCatalogLoadOutcomeLikeCpp::Failed {
                reason: "character read failed".into(),
            },
        })
        .await
        .unwrap_err();
        assert_eq!(error.to_string(), "character read failed");
    }
}
