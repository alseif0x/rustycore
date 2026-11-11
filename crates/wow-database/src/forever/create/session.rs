//! WorldSession tutorial/restriction projections in CharacterDB, not Auth.
use super::{PreparedStatement, SqlTransaction};
use wow_persistence::forever::creation::save::{
    NewCharacterSessionSave, NewInstanceRestriction, NewTutorialSave,
};

#[cfg(test)]
#[path = "../../../unit_tests/forever/creation_session.rs"]
mod tests;

const TUTORIAL_INSERT: &str = "INSERT INTO account_tutorial(tut0, tut1, tut2, tut3, tut4, tut5, tut6, tut7, accountId) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)";
const TUTORIAL_UPDATE: &str = "UPDATE account_tutorial SET tut0 = ?, tut1 = ?, tut2 = ?, tut3 = ?, tut4 = ?, tut5 = ?, tut6 = ?, tut7 = ? WHERE accountId = ?";
const RESTRICTIONS_DELETE: &str = "DELETE FROM account_instance_times WHERE accountId = ?";
const RESTRICTION_INSERT: &str =
    "INSERT INTO account_instance_times (accountId, instanceId, releaseTime) VALUES (?, ?, ?)";

pub(super) fn append(
    transaction: &mut SqlTransaction,
    account: u32,
    data: &NewCharacterSessionSave,
) {
    for row in statements(account, data) {
        transaction.append(row);
    }
}

fn statements(account: u32, data: &NewCharacterSessionSave) -> Vec<PreparedStatement> {
    let mut rows = Vec::new();
    if let Some(row) = tutorial_statement(account, &data.tutorials) {
        rows.push(row);
    }
    rows.extend(restriction_statements(account, &data.instance_restrictions));
    rows
}

fn tutorial_statement(account: u32, tutorials: &NewTutorialSave) -> Option<PreparedStatement> {
    let NewTutorialSave::Changed {
        loaded_from_db,
        values,
    } = tutorials
    else {
        return None;
    };
    let mut row = PreparedStatement::new(if *loaded_from_db {
        TUTORIAL_UPDATE
    } else {
        TUTORIAL_INSERT
    });
    for (index, value) in values.iter().enumerate() {
        row.set_u32(index, *value);
    }
    row.set_u32(8, account);
    Some(row)
}

fn restriction_statements(
    account: u32,
    restrictions: &[NewInstanceRestriction],
) -> Vec<PreparedStatement> {
    if restrictions.is_empty() {
        return Vec::new();
    }
    let mut delete = PreparedStatement::new(RESTRICTIONS_DELETE);
    delete.set_u32(0, account);
    let mut rows = vec![delete];
    for restriction in restrictions {
        let mut row = PreparedStatement::new(RESTRICTION_INSERT);
        row.set_u32(0, account);
        row.set_u32(1, restriction.instance_id);
        row.set_u64(2, restriction.release_time);
        rows.push(row);
    }
    rows
}
