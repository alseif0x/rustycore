//! Composition boundary for the C++ item random-enchantment World catalog.

use anyhow::{Result, bail};
use tracing::info;
use wow_persistence::{
    ItemRandomEnchantmentCatalogLoadOutcomeLikeCpp,
    ItemRandomEnchantmentCatalogPersistencePortLikeCpp, ItemRandomEnchantmentPersistenceRowLikeCpp,
};

fn domain_row_like_cpp(
    row: ItemRandomEnchantmentPersistenceRowLikeCpp,
) -> wow_data::ItemRandomEnchantmentTemplateEntry {
    wow_data::ItemRandomEnchantmentTemplateEntry {
        group_id: row.group_id,
        enchantment_id: row.enchantment_id,
        chance: f64::from(row.chance),
    }
}

async fn load_entries_like_cpp(
    persistence: &dyn ItemRandomEnchantmentCatalogPersistencePortLikeCpp,
) -> Result<Vec<wow_data::ItemRandomEnchantmentTemplateEntry>> {
    match persistence.load_rows_like_cpp().await {
        ItemRandomEnchantmentCatalogLoadOutcomeLikeCpp::Loaded(rows) => {
            Ok(rows.into_iter().map(domain_row_like_cpp).collect())
        }
        ItemRandomEnchantmentCatalogLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    }
}

pub(super) async fn load_item_random_enchantment_store_like_cpp(
    persistence: &dyn ItemRandomEnchantmentCatalogPersistencePortLikeCpp,
    random_properties: &wow_data::ItemRandomPropertiesStore,
    random_suffixes: &wow_data::ItemRandomSuffixStore,
) -> Result<wow_data::ItemRandomEnchantmentTemplateStore> {
    let rows = load_entries_like_cpp(persistence).await?;
    let store = wow_data::ItemRandomEnchantmentTemplateStore::from_entries_validated(
        rows,
        random_properties,
        random_suffixes,
    );
    info!(
        "Loaded {} validated item random enchantment groups",
        store.len()
    );
    Ok(store)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_persistence::PersistenceFutureLikeCpp;

    struct FixedPort {
        outcome: ItemRandomEnchantmentCatalogLoadOutcomeLikeCpp,
    }

    impl ItemRandomEnchantmentCatalogPersistencePortLikeCpp for FixedPort {
        fn load_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<'_, ItemRandomEnchantmentCatalogLoadOutcomeLikeCpp> {
            Box::pin(async move { self.outcome.clone() })
        }
    }

    #[tokio::test]
    async fn typed_row_preserves_every_domain_field() {
        let entries = load_entries_like_cpp(&FixedPort {
            outcome: ItemRandomEnchantmentCatalogLoadOutcomeLikeCpp::Loaded(vec![
                ItemRandomEnchantmentPersistenceRowLikeCpp {
                    group_id: 10,
                    enchantment_id: 20,
                    chance: 12.5,
                },
            ]),
        })
        .await
        .unwrap();
        assert_eq!(
            entries,
            [wow_data::ItemRandomEnchantmentTemplateEntry {
                group_id: 10,
                enchantment_id: 20,
                chance: 12.5,
            }]
        );
    }

    #[tokio::test]
    async fn empty_success_remains_a_successful_empty_batch() {
        let entries = load_entries_like_cpp(&FixedPort {
            outcome: ItemRandomEnchantmentCatalogLoadOutcomeLikeCpp::Loaded(Vec::new()),
        })
        .await
        .unwrap();
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn failure_preserves_existing_startup_fatal_policy() {
        let error = load_entries_like_cpp(&FixedPort {
            outcome: ItemRandomEnchantmentCatalogLoadOutcomeLikeCpp::Failed {
                reason: "world read failed".into(),
            },
        })
        .await
        .unwrap_err();
        assert_eq!(error.to_string(), "world read failed");
    }
}
