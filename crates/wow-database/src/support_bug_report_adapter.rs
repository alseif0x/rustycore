//! MariaDB adapter for legacy `CMSG_BUG_REPORT` persistence.

use std::sync::Arc;

use wow_persistence::{
    PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp, SupportBugReportPersistencePortLikeCpp,
    SupportBugReportWriteRequestLikeCpp,
};

use crate::{CharStatements, CharacterDatabase, PreparedStatement};

fn bug_report_insert_statement_like_cpp(
    request: &SupportBugReportWriteRequestLikeCpp,
) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(CharStatements::INS_BUG_REPORT);
    // C++ parses `Type` but binds Text and DiagInfo to the `(type, content)`
    // SQL columns in that order.
    statement.set_string(0, request.text.clone());
    statement.set_string(1, request.diagnostic_info.clone());
    statement
}

pub struct MariaDbSupportBugReportPersistenceAdapterLikeCpp {
    character_db: Arc<CharacterDatabase>,
}

impl MariaDbSupportBugReportPersistenceAdapterLikeCpp {
    pub fn new(character_db: Arc<CharacterDatabase>) -> Self {
        Self { character_db }
    }
}

impl SupportBugReportPersistencePortLikeCpp for MariaDbSupportBugReportPersistenceAdapterLikeCpp {
    fn persist_bug_report_like_cpp<'a>(
        &'a self,
        request: SupportBugReportWriteRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let statement = bug_report_insert_statement_like_cpp(&request);
            match self.character_db.execute(&statement).await {
                Ok(rows) => PersistenceOutcomeLikeCpp::Applied { rows },
                Err(error) => PersistenceOutcomeLikeCpp::Failed {
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
    fn bug_report_statement_preserves_cpp_identity_and_bind_order() {
        let statement =
            bug_report_insert_statement_like_cpp(&SupportBugReportWriteRequestLikeCpp {
                text: "client bug".to_owned(),
                diagnostic_info: "diag blob".to_owned(),
            });

        assert_eq!(statement.sql(), CharStatements::INS_BUG_REPORT.sql());
        assert_eq!(
            statement.params(),
            vec![
                SqlParam::String("client bug".to_owned()),
                SqlParam::String("diag blob".to_owned()),
            ]
        );
    }
}
