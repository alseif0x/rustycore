//! MariaDB adapter for Rust's transitional on-demand item-template-addon reads.

use std::sync::Arc;

use wow_persistence::{
    ItemTemplateAddonCatalogPersistencePortLikeCpp, ItemTemplateAddonCatalogRequestLikeCpp,
    ItemTemplateAddonLootMetadataOutcomeLikeCpp, ItemTemplateAddonLootMetadataRowLikeCpp,
    ItemTemplateAddonMoneyOutcomeLikeCpp, ItemTemplateAddonMoneyRowLikeCpp,
    PersistenceFutureLikeCpp,
};

use crate::{PreparedStatement, WorldDatabase, WorldStatements};

fn money_statement_like_cpp(item_entry: u32) -> PreparedStatement {
    let mut statement =
        PreparedStatement::for_statement(WorldStatements::SEL_ITEM_TEMPLATE_ADDON_MONEY_LOOT);
    statement.set_u32(0, item_entry);
    statement
}

fn loot_metadata_statement_like_cpp(item_entry: u32) -> PreparedStatement {
    let mut statement =
        PreparedStatement::for_statement(WorldStatements::SEL_ITEM_TEMPLATE_ADDON_LOOT_METADATA);
    statement.set_u32(0, item_entry);
    statement
}

pub struct MariaDbItemTemplateAddonCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbItemTemplateAddonCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }
}

impl ItemTemplateAddonCatalogPersistencePortLikeCpp
    for MariaDbItemTemplateAddonCatalogPersistenceAdapterLikeCpp
{
    fn load_item_template_addon_money_like_cpp<'a>(
        &'a self,
        request: ItemTemplateAddonCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, ItemTemplateAddonMoneyOutcomeLikeCpp> {
        Box::pin(async move {
            let result = match self
                .world_db
                .query(&money_statement_like_cpp(request.item_entry))
                .await
            {
                Ok(result) => result,
                Err(error) => {
                    return ItemTemplateAddonMoneyOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    };
                }
            };

            if result.is_empty() {
                return ItemTemplateAddonMoneyOutcomeLikeCpp::Missing;
            }

            ItemTemplateAddonMoneyOutcomeLikeCpp::Found(ItemTemplateAddonMoneyRowLikeCpp {
                min_money: result.try_read::<u32>(0),
                max_money: result.try_read::<u32>(1),
            })
        })
    }

    fn load_item_template_addon_loot_metadata_like_cpp<'a>(
        &'a self,
        request: ItemTemplateAddonCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, ItemTemplateAddonLootMetadataOutcomeLikeCpp> {
        Box::pin(async move {
            let result = match self
                .world_db
                .query(&loot_metadata_statement_like_cpp(request.item_entry))
                .await
            {
                Ok(result) => result,
                Err(error) => {
                    return ItemTemplateAddonLootMetadataOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    };
                }
            };

            if result.is_empty() {
                return ItemTemplateAddonLootMetadataOutcomeLikeCpp::Missing;
            }

            ItemTemplateAddonLootMetadataOutcomeLikeCpp::Found(
                ItemTemplateAddonLootMetadataRowLikeCpp {
                    flags_cu: result.try_read::<u32>(0).unwrap_or(0),
                    quest_log_item_id: result.try_read::<i32>(1).unwrap_or(0),
                },
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SqlParam, StatementDef};

    #[test]
    fn item_template_addon_statements_preserve_identity_and_entry_bind() {
        let money = money_statement_like_cpp(0x0102_0304);
        assert_eq!(
            money.sql(),
            WorldStatements::SEL_ITEM_TEMPLATE_ADDON_MONEY_LOOT.sql()
        );
        assert_eq!(money.params(), [SqlParam::U32(0x0102_0304)]);

        let metadata = loot_metadata_statement_like_cpp(0x0506_0708);
        assert_eq!(
            metadata.sql(),
            WorldStatements::SEL_ITEM_TEMPLATE_ADDON_LOOT_METADATA.sql()
        );
        assert_eq!(metadata.params(), [SqlParam::U32(0x0506_0708)]);
    }
}
