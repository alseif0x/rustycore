//! MariaDB adapter for C++ vendor interaction catalog reads.

use std::sync::Arc;

use wow_persistence::{
    PersistenceFutureLikeCpp, VendorCatalogOutcomeLikeCpp, VendorCatalogPersistencePortLikeCpp,
    VendorCatalogRowLikeCpp,
};

use crate::{PreparedStatement, WorldDatabase, WorldStatements};

fn vendor_rows_statement_like_cpp(root_entry: u32, vendor_entry: u32) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(WorldStatements::SEL_VENDOR_ITEMS);
    statement.set_u32(0, root_entry);
    statement.set_u32(1, vendor_entry);
    statement
}

pub struct MariaDbVendorCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbVendorCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }
}

impl VendorCatalogPersistencePortLikeCpp for MariaDbVendorCatalogPersistenceAdapterLikeCpp {
    fn load_vendor_rows_like_cpp(
        &self,
        root_entry: u32,
        vendor_entry: u32,
    ) -> PersistenceFutureLikeCpp<'_, VendorCatalogOutcomeLikeCpp<Vec<VendorCatalogRowLikeCpp>>>
    {
        Box::pin(async move {
            let statement = vendor_rows_statement_like_cpp(root_entry, vendor_entry);
            let mut result = match self.world_db.query(&statement).await {
                Ok(result) => result,
                Err(error) => {
                    return VendorCatalogOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    };
                }
            };
            let mut rows = Vec::new();
            if !result.is_empty() {
                loop {
                    rows.push(VendorCatalogRowLikeCpp {
                        item_id: result.try_read(0).unwrap_or(0),
                        max_count: result.try_read(1).unwrap_or(0),
                        extended_cost: result.try_read(2).unwrap_or(0),
                        item_type: result.try_read(3).unwrap_or(1),
                        buy_price: result
                            .try_read::<i64>(5)
                            .map(|v| v as u64)
                            .or_else(|| result.try_read(5))
                            .unwrap_or(0),
                        max_durability: result
                            .try_read::<i64>(7)
                            .map(|v| v as u32)
                            .or_else(|| result.try_read(7))
                            .unwrap_or(0),
                        buy_count: result
                            .try_read::<i64>(8)
                            .map(|v| v as u32)
                            .or_else(|| result.try_read(8))
                            .unwrap_or(1),
                        do_not_filter: result.try_read::<u8>(9).is_some_and(|v| v != 0),
                        incr_time: result.try_read(10).unwrap_or(0),
                        player_condition_id: result.try_read(11).unwrap_or(0),
                        has_vendor_conditions: result.try_read::<u8>(12).is_some_and(|v| v != 0),
                    });
                    if !result.next_row() {
                        break;
                    }
                }
            }
            VendorCatalogOutcomeLikeCpp::Loaded(rows)
        })
    }

    fn load_creature_entry_by_spawn_like_cpp(
        &self,
        spawn_guid: u64,
    ) -> PersistenceFutureLikeCpp<'_, VendorCatalogOutcomeLikeCpp<u32>> {
        Box::pin(async move {
            let mut statement = self
                .world_db
                .prepare(WorldStatements::SEL_CREATURE_ENTRY_BY_GUID);
            statement.set_u64(0, spawn_guid);
            match self.world_db.query(&statement).await {
                Ok(result) if result.is_empty() => VendorCatalogOutcomeLikeCpp::Missing,
                Ok(result) => result
                    .try_read(0)
                    .map(VendorCatalogOutcomeLikeCpp::Loaded)
                    .unwrap_or(VendorCatalogOutcomeLikeCpp::Missing),
                Err(error) => VendorCatalogOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn load_item_sell_price_like_cpp(
        &self,
        item_entry: u32,
    ) -> PersistenceFutureLikeCpp<'_, VendorCatalogOutcomeLikeCpp<u64>> {
        Box::pin(async move {
            let mut statement = self.world_db.prepare(WorldStatements::SEL_ITEM_SELL_PRICE);
            statement.set_u32(0, item_entry);
            match self.world_db.query(&statement).await {
                Ok(result) if result.is_empty() => VendorCatalogOutcomeLikeCpp::Missing,
                Ok(result) => result
                    .try_read(0)
                    .map(VendorCatalogOutcomeLikeCpp::Loaded)
                    .unwrap_or(VendorCatalogOutcomeLikeCpp::Missing),
                Err(error) => VendorCatalogOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SqlParam, StatementDef};

    #[test]
    fn vendor_expansion_preserves_root_then_reference_bind_order_like_cpp() {
        let statement = vendor_rows_statement_like_cpp(2456, 9000);
        assert_eq!(statement.sql(), WorldStatements::SEL_VENDOR_ITEMS.sql());
        assert_eq!(
            statement.params(),
            [SqlParam::U32(2456), SqlParam::U32(9000)]
        );
    }
}
