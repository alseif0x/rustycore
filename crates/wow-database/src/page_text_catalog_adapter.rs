//! MariaDB adapter for Rust's transitional on-demand page-text catalog.

use std::collections::HashSet;
use std::sync::Arc;

use wow_persistence::{
    PageTextCatalogDiagnosticLikeCpp, PageTextCatalogOutcomeLikeCpp,
    PageTextCatalogPersistencePortLikeCpp, PageTextCatalogRequestLikeCpp,
    PageTextCatalogRowLikeCpp, PersistenceFutureLikeCpp,
};

use crate::{PreparedStatement, WorldDatabase, WorldStatements};

const PAGE_TEXT_QUERY_CHAIN_DEFENSIVE_LIMIT: usize = 100;

fn statement_like_cpp(statement: WorldStatements, page_text_id: u32) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(statement);
    statement.set_u32(0, page_text_id);
    statement
}

pub struct MariaDbPageTextCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbPageTextCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }
}

impl PageTextCatalogPersistencePortLikeCpp for MariaDbPageTextCatalogPersistenceAdapterLikeCpp {
    fn load_page_text_catalog_like_cpp<'a>(
        &'a self,
        request: PageTextCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PageTextCatalogOutcomeLikeCpp> {
        Box::pin(async move {
            let mut pages = Vec::new();
            let mut diagnostics = Vec::new();
            let mut page_text_id = request.page_text_id;
            let mut visited = HashSet::new();

            while page_text_id != 0
                && visited.insert(page_text_id)
                && pages.len() < PAGE_TEXT_QUERY_CHAIN_DEFENSIVE_LIMIT
            {
                let result = match self
                    .world_db
                    .query(&statement_like_cpp(
                        WorldStatements::SEL_PAGE_TEXT,
                        page_text_id,
                    ))
                    .await
                {
                    Ok(result) => result,
                    Err(error) => {
                        diagnostics.push(PageTextCatalogDiagnosticLikeCpp::PageReadFailed {
                            page_text_id,
                            reason: error.to_string(),
                        });
                        break;
                    }
                };
                if result.is_empty() {
                    break;
                }

                let id = result.try_read(0).unwrap_or(page_text_id);
                let mut text = result.read_string(1);
                let next_page_id = result.try_read(2).unwrap_or(0);
                let player_condition_id = result.try_read(3).unwrap_or(0);
                let flags = result.try_read(4).unwrap_or(0);

                if !request.locale.is_empty() && request.locale != "enUS" {
                    let mut statement =
                        statement_like_cpp(WorldStatements::SEL_PAGE_TEXT_LOCALE, id);
                    statement.set_string(1, &request.locale);
                    match self.world_db.query(&statement).await {
                        Ok(locale) if !locale.is_empty() => {
                            let locale_text = locale.read_string(0);
                            if !locale_text.is_empty() {
                                text = locale_text;
                            }
                        }
                        Ok(_) => {}
                        Err(error) => {
                            diagnostics.push(PageTextCatalogDiagnosticLikeCpp::LocaleReadFailed {
                                page_text_id: id,
                                locale: request.locale.clone(),
                                reason: error.to_string(),
                            })
                        }
                    }
                }

                pages.push(PageTextCatalogRowLikeCpp {
                    id,
                    next_page_id,
                    player_condition_id,
                    flags,
                    text,
                });
                page_text_id = next_page_id;
            }

            PageTextCatalogOutcomeLikeCpp { pages, diagnostics }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SqlParam, StatementDef};

    #[test]
    fn page_text_statements_preserve_identity_and_binds() {
        let base = statement_like_cpp(WorldStatements::SEL_PAGE_TEXT, 0xA1B2_C3D4);
        assert_eq!(base.sql(), WorldStatements::SEL_PAGE_TEXT.sql());
        assert_eq!(base.params(), [SqlParam::U32(0xA1B2_C3D4)]);

        let mut locale = statement_like_cpp(WorldStatements::SEL_PAGE_TEXT_LOCALE, 42);
        locale.set_string(1, "esES");
        assert_eq!(locale.sql(), WorldStatements::SEL_PAGE_TEXT_LOCALE.sql());
        assert_eq!(
            locale.params(),
            [SqlParam::U32(42), SqlParam::String("esES".to_owned())]
        );
    }
}
