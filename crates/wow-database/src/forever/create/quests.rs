//! Player's ordered status/reward queues followed by periodic saves at 02245dcd.
use super::{PreparedStatement, SqlTransaction};
use wow_persistence::forever::creation::save::{
    NewQuestChange, NewQuestObjective, NewQuestSave, NewQuestSpawnTracking, NewQuestStatus,
    NewRewardedQuestChange,
};

mod periodic;

#[cfg(test)]
#[path = "../../../unit_tests/forever/creation_quests.rs"]
mod tests;

const STATUS_REPLACE: &str = "REPLACE INTO character_queststatus (guid, quest, status, explored, acceptTime, endTime) VALUES (?, ?, ?, ?, ?, ?)";
const STATUS_DELETE: &str = "DELETE FROM character_queststatus WHERE guid = ? AND quest = ?";
const OBJECTIVE_REPLACE: &str = "REPLACE INTO character_queststatus_objectives (guid, quest, objective, data) VALUES (?, ?, ?, ?)";
const OBJECTIVE_DELETE: &str =
    "DELETE FROM character_queststatus_objectives WHERE guid = ? AND quest = ?";
const SPAWN_REPLACE: &str = "REPLACE INTO character_queststatus_objectives_spawn_tracking (guid, quest, objective, spawnTrackingId) VALUES (?, ?, ?, ?)";
const SPAWN_DELETE: &str =
    "DELETE FROM character_queststatus_objectives_spawn_tracking WHERE guid = ? AND quest = ?";
const REWARDED_INSERT: &str =
    "INSERT IGNORE INTO character_queststatus_rewarded (guid, quest, active) VALUES (?, ?, 1)";
const REWARDED_DELETE: &str =
    "DELETE FROM character_queststatus_rewarded WHERE guid = ? AND quest = ?";

pub(super) fn append(transaction: &mut SqlTransaction, guid: u64, save: &NewQuestSave) {
    for statement in statements(guid, save) {
        transaction.append(statement);
    }
}

fn statements(guid: u64, save: &NewQuestSave) -> impl Iterator<Item = PreparedStatement> + '_ {
    // Both maps retain the source std::map order. Finish every status operation
    // before visiting any rewarded operation, even for overlapping quest IDs.
    let keep_abandoned = save.keep_abandoned;
    save.statuses
        .iter()
        .flat_map(move |(quest, change)| status_statements(guid, *quest, change, keep_abandoned))
        .chain(save.rewarded.iter().filter_map(move |(quest, change)| {
            rewarded_statement(guid, *quest, change, keep_abandoned)
        }))
        .chain(periodic::statements(guid, save))
}

fn status_statements(
    guid: u64,
    quest: u32,
    change: &NewQuestChange,
    keep_abandoned: bool,
) -> Vec<PreparedStatement> {
    match change {
        NewQuestChange::Default(Some(status)) if keep_abandoned || status.status != 0 => {
            let mut statements = vec![
                status_statement(guid, quest, status),
                scoped_statement(OBJECTIVE_DELETE, guid, quest),
            ];
            statements.extend(
                status
                    .objectives
                    .iter()
                    .filter(|objective| objective.count != 0)
                    .map(|objective| objective_statement(guid, quest, objective)),
            );
            statements.push(scoped_statement(SPAWN_DELETE, guid, quest));
            statements.extend(
                status
                    .spawn_tracking
                    .iter()
                    .map(|spawn| spawn_statement(guid, quest, spawn)),
            );
            statements
        }
        NewQuestChange::Default(_) => Vec::new(),
        NewQuestChange::Delete => vec![
            scoped_statement(STATUS_DELETE, guid, quest),
            scoped_statement(OBJECTIVE_DELETE, guid, quest),
            scoped_statement(SPAWN_DELETE, guid, quest),
        ],
    }
}

fn scoped_statement(sql: &'static str, guid: u64, quest: u32) -> PreparedStatement {
    let mut statement = PreparedStatement::new(sql);
    statement.set_u64(0, guid);
    statement.set_u32(1, quest);
    statement
}

fn status_statement(guid: u64, quest: u32, status: &NewQuestStatus) -> PreparedStatement {
    let mut statement = scoped_statement(STATUS_REPLACE, guid, quest);
    statement.set_u8(2, status.status);
    statement.set_bool(3, status.explored);
    statement.set_i64(4, status.accept_time);
    statement.set_i64(5, status.end_time);
    statement
}

fn objective_statement(guid: u64, quest: u32, objective: &NewQuestObjective) -> PreparedStatement {
    let mut statement = scoped_statement(OBJECTIVE_REPLACE, guid, quest);
    statement.set_u8(2, objective.index);
    statement.set_i32(3, objective.count);
    statement
}

fn spawn_statement(guid: u64, quest: u32, spawn: &NewQuestSpawnTracking) -> PreparedStatement {
    let mut statement = scoped_statement(SPAWN_REPLACE, guid, quest);
    statement.set_u8(2, spawn.index);
    statement.set_u32(3, spawn.id);
    statement
}

fn rewarded_statement(
    guid: u64,
    quest: u32,
    change: &NewRewardedQuestChange,
    keep_abandoned: bool,
) -> Option<PreparedStatement> {
    match change {
        NewRewardedQuestChange::Default => Some(scoped_statement(REWARDED_INSERT, guid, quest)),
        NewRewardedQuestChange::Delete if keep_abandoned => None,
        NewRewardedQuestChange::Delete | NewRewardedQuestChange::ForceDelete => {
            Some(scoped_statement(REWARDED_DELETE, guid, quest))
        }
    }
}
