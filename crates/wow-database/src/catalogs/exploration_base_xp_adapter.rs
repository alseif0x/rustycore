//! MariaDB adapter for C++ `ObjectMgr::LoadExplorationBaseXP`.

use std::sync::Arc;

use wow_persistence::{
    ExplorationBaseXpCatalogLoadOutcomeLikeCpp, ExplorationBaseXpCatalogPersistencePortLikeCpp,
    ExplorationBaseXpPersistenceRowLikeCpp, PersistenceFutureLikeCpp,
};

use crate::{SqlResult, WorldDatabase, WorldStatements};

fn signed_base_xp_like_cpp(value: i32) -> u32 {
    value as u32
}

fn base_xp_like_cpp(result: &SqlResult, column: usize) -> u32 {
    if let Some(value) = result.try_read::<u32>(column) {
        return value;
    }

    // C++ reads the signed `int` column through `Field::GetInt32()` and
    // assigns it to `uint32`. Preserve that conversion, including wrapping.
    result
        .try_read::<i32>(column)
        .map(signed_base_xp_like_cpp)
        .unwrap_or(0)
}

fn decode_row_like_cpp(result: &SqlResult) -> ExplorationBaseXpPersistenceRowLikeCpp {
    ExplorationBaseXpPersistenceRowLikeCpp {
        level: result.read(0),
        base_xp: base_xp_like_cpp(result, 1),
    }
}

pub struct MariaDbExplorationBaseXpCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbExplorationBaseXpCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }
}

impl ExplorationBaseXpCatalogPersistencePortLikeCpp
    for MariaDbExplorationBaseXpCatalogPersistenceAdapterLikeCpp
{
    fn load_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, ExplorationBaseXpCatalogLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let result = async {
                let mut result = self
                    .world_db
                    .query(
                        &self
                            .world_db
                            .prepare(WorldStatements::SEL_EXPLORATION_BASE_XP),
                    )
                    .await?;
                let mut rows = Vec::with_capacity(result.count());
                if result.is_empty() {
                    return Ok(rows);
                }

                loop {
                    rows.push(decode_row_like_cpp(&result));
                    if !result.next_row() {
                        break;
                    }
                }
                Ok::<_, anyhow::Error>(rows)
            }
            .await;

            match result {
                Ok(rows) => ExplorationBaseXpCatalogLoadOutcomeLikeCpp::Loaded(rows),
                Err(error) => ExplorationBaseXpCatalogLoadOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StatementDef;

    #[test]
    fn statement_matches_cpp_columns_and_order() {
        assert_eq!(
            WorldStatements::SEL_EXPLORATION_BASE_XP.sql(),
            "SELECT level, basexp FROM exploration_basexp"
        );
    }

    #[test]
    fn signed_basexp_assignment_wraps_to_uint32_like_cpp() {
        assert_eq!(signed_base_xp_like_cpp(-1), u32::MAX);
        assert_eq!(signed_base_xp_like_cpp(i32::MIN), 2_147_483_648);
    }
}
