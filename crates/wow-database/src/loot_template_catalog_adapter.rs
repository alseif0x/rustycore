//! MariaDB adapter for the on-demand C++ LootStore source tables.

use std::sync::Arc;

use wow_persistence::{
    LootConditionPersistenceRowLikeCpp, LootTemplateCatalogOutcomeLikeCpp,
    LootTemplateCatalogPersistencePortLikeCpp, LootTemplatePersistenceRowLikeCpp,
    LootTemplateTablePersistenceLikeCpp, PersistenceFutureLikeCpp,
};

use crate::{PreparedStatement, WorldDatabase, WorldStatements};

fn loot_template_statement_like_cpp(
    table: LootTemplateTablePersistenceLikeCpp,
    entry: u32,
) -> PreparedStatement {
    let statement = match table {
        LootTemplateTablePersistenceLikeCpp::Item => WorldStatements::SEL_ITEM_LOOT_TEMPLATE_ROWS,
        LootTemplateTablePersistenceLikeCpp::Disenchant => {
            WorldStatements::SEL_DISENCHANT_LOOT_TEMPLATE_ROWS
        }
        LootTemplateTablePersistenceLikeCpp::Reference => {
            WorldStatements::SEL_REFERENCE_LOOT_TEMPLATE_ROWS
        }
    };
    let mut statement = PreparedStatement::for_statement(statement);
    statement.set_u32(0, entry);
    statement
}

fn loot_condition_statement_like_cpp(
    source_type: i32,
    source_group: u32,
    source_entry: u32,
) -> PreparedStatement {
    let mut statement =
        PreparedStatement::for_statement(WorldStatements::SEL_LOOT_TEMPLATE_CONDITION_ROWS);
    statement.set_i32(0, source_type);
    statement.set_u32(1, source_group);
    statement.set_u32(2, source_entry);
    statement
}

pub struct MariaDbLootTemplateCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbLootTemplateCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }
}

impl LootTemplateCatalogPersistencePortLikeCpp
    for MariaDbLootTemplateCatalogPersistenceAdapterLikeCpp
{
    fn load_loot_template_rows_like_cpp(
        &self,
        table: LootTemplateTablePersistenceLikeCpp,
        entry: u32,
    ) -> PersistenceFutureLikeCpp<
        '_,
        LootTemplateCatalogOutcomeLikeCpp<LootTemplatePersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            let statement = loot_template_statement_like_cpp(table, entry);
            let mut result = match self.world_db.query(&statement).await {
                Ok(result) => result,
                Err(error) => {
                    return LootTemplateCatalogOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    };
                }
            };
            let mut rows = Vec::new();
            if !result.is_empty() {
                loop {
                    rows.push(LootTemplatePersistenceRowLikeCpp {
                        item_id: result.try_read(0).unwrap_or(0),
                        reference: result.try_read(1).unwrap_or(0),
                        chance: result.try_read(2).unwrap_or(0.0),
                        needs_quest: result.try_read(3).unwrap_or(false),
                        loot_mode: result.try_read(4).unwrap_or(0),
                        group_id: result.try_read(5).unwrap_or(0),
                        min_count: result.try_read(6).unwrap_or(0),
                        max_count: result.try_read(7).unwrap_or(0),
                    });
                    if !result.next_row() {
                        break;
                    }
                }
            }
            LootTemplateCatalogOutcomeLikeCpp::Loaded(rows)
        })
    }

    fn load_loot_condition_rows_like_cpp(
        &self,
        source_type: i32,
        source_group: u32,
        source_entry: u32,
    ) -> PersistenceFutureLikeCpp<
        '_,
        LootTemplateCatalogOutcomeLikeCpp<LootConditionPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            let statement =
                loot_condition_statement_like_cpp(source_type, source_group, source_entry);
            let mut result = match self.world_db.query(&statement).await {
                Ok(result) => result,
                Err(error) => {
                    return LootTemplateCatalogOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    };
                }
            };
            let mut rows = Vec::new();
            if !result.is_empty() {
                loop {
                    rows.push(LootConditionPersistenceRowLikeCpp {
                        else_group: result.try_read(0).unwrap_or(0),
                        condition_type_or_reference: result.try_read(1).unwrap_or(0),
                        condition_target: result.try_read(2).unwrap_or(0),
                        value1: result.try_read(3).unwrap_or(0),
                        value2: result.try_read(4).unwrap_or(0),
                        value3: result.try_read(5).unwrap_or(0),
                        string_value1: result.try_read(6).unwrap_or_default(),
                        negative: result.try_read(7).unwrap_or(false),
                        script_name: result.try_read(8).unwrap_or_default(),
                    });
                    if !result.next_row() {
                        break;
                    }
                }
            }
            LootTemplateCatalogOutcomeLikeCpp::Loaded(rows)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SqlParam, StatementDef};

    #[test]
    fn loot_catalog_preserves_statement_identity_and_bind_order_like_cpp() {
        let item = loot_template_statement_like_cpp(LootTemplateTablePersistenceLikeCpp::Item, 7);
        assert_eq!(
            item.sql(),
            WorldStatements::SEL_ITEM_LOOT_TEMPLATE_ROWS.sql()
        );
        assert_eq!(item.params(), [SqlParam::U32(7)]);

        let condition = loot_condition_statement_like_cpp(-9, 10, 11);
        assert_eq!(
            condition.sql(),
            WorldStatements::SEL_LOOT_TEMPLATE_CONDITION_ROWS.sql()
        );
        assert_eq!(
            condition.params(),
            [SqlParam::I32(-9), SqlParam::U32(10), SqlParam::U32(11)]
        );
    }
}
