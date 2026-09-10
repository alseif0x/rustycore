//! MariaDB adapter for C++ `ObjectMgr::LoadReservedPlayersNames`.

use std::sync::Arc;

use wow_persistence::{
    PersistenceFutureLikeCpp, ReservedNameCatalogLoadOutcomeLikeCpp,
    ReservedNameCatalogPersistencePortLikeCpp, ReservedNamePersistenceRowLikeCpp,
};

use crate::{CharStatements, CharacterDatabase};

pub struct MariaDbReservedNameCatalogPersistenceAdapterLikeCpp {
    character_db: Arc<CharacterDatabase>,
}

impl MariaDbReservedNameCatalogPersistenceAdapterLikeCpp {
    pub fn new(character_db: Arc<CharacterDatabase>) -> Self {
        Self { character_db }
    }
}

impl ReservedNameCatalogPersistencePortLikeCpp
    for MariaDbReservedNameCatalogPersistenceAdapterLikeCpp
{
    fn load_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, ReservedNameCatalogLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let result = async {
                let mut result = self
                    .character_db
                    .query(
                        &self
                            .character_db
                            .prepare(CharStatements::SEL_RESERVED_NAMES),
                    )
                    .await?;
                let mut rows = Vec::with_capacity(result.count());
                if result.is_empty() {
                    return Ok(rows);
                }

                loop {
                    rows.push(ReservedNamePersistenceRowLikeCpp {
                        name: result.read(0),
                    });
                    if !result.next_row() {
                        break;
                    }
                }
                Ok::<_, anyhow::Error>(rows)
            }
            .await;

            match result {
                Ok(rows) => ReservedNameCatalogLoadOutcomeLikeCpp::Loaded(rows),
                Err(error) => ReservedNameCatalogLoadOutcomeLikeCpp::Failed {
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
            CharStatements::SEL_RESERVED_NAMES.sql(),
            "SELECT name FROM reserved_name"
        );
    }
}
