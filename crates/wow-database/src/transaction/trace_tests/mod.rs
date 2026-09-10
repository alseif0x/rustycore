//! Transaction regressions.
//!
//! Separated from transaction.rs under #683.

use super::super::*;

use super::*;
use crate::statements::{CharStatements, StatementDef};

fn traced() -> (SqlTransaction, PersistenceRecorder) {
    let recorder = PersistenceRecorder::new();
    let trans = SqlTransaction::new().with_trace(recorder.clone(), LogicalDatabase::Character);
    (trans, recorder)
}

fn commit_error(unknown: bool) -> SqlTransactionCommitError {
    let error = DatabaseError::Transaction("transport reset".to_owned());
    if unknown {
        SqlTransactionCommitError::CommitOutcomeUnknown(error)
    } else {
        SqlTransactionCommitError::DefinitelyRolledBack(error)
    }
}

mod scenarios;
