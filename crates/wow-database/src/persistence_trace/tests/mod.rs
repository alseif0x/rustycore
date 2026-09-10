//! Persistence-trace regressions.
//!
//! Separated from persistence_trace.rs under #685.

use super::*;

fn money_plan() -> PersistenceTrace {
    PersistenceTrace {
        events: vec![
            PersistenceEvent::TransactionBegin {
                database: LogicalDatabase::Character,
            },
            PersistenceEvent::Statement {
                database: LogicalDatabase::Character,
                connection: ConnectionAffinity::Transaction,
                statement: "UPD_CHARACTER_MONEY".to_owned(),
                params: vec![TracedParam::Uint {
                    value: 100,
                    width_bits: 64,
                }],
                expected_rows_affected: Some(1),
                observed_rows_affected: None,
            },
            PersistenceEvent::Commit {
                database: LogicalDatabase::Character,
                outcome: CommitOutcome::Committed,
            },
            PersistenceEvent::Publication {
                label: "money".to_owned(),
            },
        ],
    }
}

mod scenarios;
