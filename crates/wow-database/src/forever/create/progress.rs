//! Character achievement, reputation and quest-criteria save projections at 02245dcd.
use super::{PreparedStatement, SqlTransaction};
use std::collections::BTreeMap;
use wow_persistence::forever::creation::save::{
    NewAchievementSave, NewCriteriaProgress, NewQuestCriteriaSave, NewReputation,
};

#[cfg(test)]
#[path = "../../../unit_tests/forever/creation_progress.rs"]
mod tests;

const ACHIEVEMENT_DELETE: &str =
    "DELETE FROM character_achievement WHERE achievement = ? AND guid = ?";
const ACHIEVEMENT_INSERT: &str =
    "INSERT INTO character_achievement (guid, achievement, date) VALUES (?, ?, ?)";
const ACHIEVEMENT_PROGRESS_DELETE: &str =
    "DELETE FROM character_achievement_progress WHERE guid = ? AND criteria = ?";
const ACHIEVEMENT_PROGRESS_INSERT: &str = "INSERT INTO character_achievement_progress (guid, criteria, counter, date) VALUES (?, ?, ?, ?)";
const REPUTATION_DELETE: &str = "DELETE FROM character_reputation WHERE guid = ? AND faction = ?";
const REPUTATION_INSERT: &str =
    "INSERT INTO character_reputation (guid, faction, standing, flags) VALUES (?, ?, ? , ?)";
const OBJECTIVES_DELETE: &str =
    "DELETE FROM character_queststatus_objectives_criteria WHERE guid = ?";
const OBJECTIVE_INSERT: &str =
    "INSERT INTO character_queststatus_objectives_criteria (guid, questObjectiveId) VALUES (?, ?)";
const QUEST_PROGRESS_DELETE: &str = "DELETE FROM character_queststatus_objectives_criteria_progress WHERE guid = ? AND criteriaId = ?";
const QUEST_PROGRESS_INSERT: &str = "INSERT INTO character_queststatus_objectives_criteria_progress (guid, criteriaId, counter, date) VALUES (?, ?, ?, ?)";

pub(super) fn append(
    transaction: &mut SqlTransaction,
    guid: u64,
    achievements: &NewAchievementSave,
    reputation: &BTreeMap<u32, NewReputation>,
    quest_criteria: &NewQuestCriteriaSave,
) {
    for row in statements(guid, achievements, reputation, quest_criteria) {
        transaction.append(row);
    }
}

fn statements(
    guid: u64,
    achievements: &NewAchievementSave,
    reputation: &BTreeMap<u32, NewReputation>,
    quest_criteria: &NewQuestCriteriaSave,
) -> Vec<PreparedStatement> {
    let mut rows = achievement_statements(guid, achievements);
    rows.extend(reputation_statements(guid, reputation));
    rows.extend(quest_statements(guid, quest_criteria));
    rows
}

fn guid_statement(sql: &'static str, guid: u64) -> PreparedStatement {
    let mut row = PreparedStatement::new(sql);
    row.set_u64(0, guid);
    row
}

fn achievement_statements(guid: u64, data: &NewAchievementSave) -> Vec<PreparedStatement> {
    let mut rows = Vec::new();
    for achievement in data
        .completed
        .iter()
        .filter(|achievement| achievement.changed)
    {
        // This source DELETE reverses the usual guid/ID parameter order.
        let mut delete = PreparedStatement::new(ACHIEVEMENT_DELETE);
        delete.set_u32(0, achievement.achievement);
        delete.set_u64(1, guid);
        rows.push(delete);
        let mut insert = guid_statement(ACHIEVEMENT_INSERT, guid);
        insert.set_u32(1, achievement.achievement);
        insert.set_i64(2, achievement.date);
        rows.push(insert);
    }
    rows.extend(criteria_statements(
        guid,
        &data.progress,
        ACHIEVEMENT_PROGRESS_DELETE,
        ACHIEVEMENT_PROGRESS_INSERT,
    ));
    rows
}

fn criteria_statements(
    guid: u64,
    progress: &[NewCriteriaProgress],
    delete_sql: &'static str,
    insert_sql: &'static str,
) -> Vec<PreparedStatement> {
    let mut rows = Vec::new();
    for criteria in progress.iter().filter(|criteria| criteria.changed) {
        let mut delete = guid_statement(delete_sql, guid);
        delete.set_u32(1, criteria.criteria);
        rows.push(delete);
        if criteria.counter != 0 {
            let mut insert = guid_statement(insert_sql, guid);
            insert.set_u32(1, criteria.criteria);
            insert.set_u64(2, criteria.counter);
            insert.set_i64(3, criteria.date);
            rows.push(insert);
        }
    }
    rows
}

fn reputation_statements(guid: u64, data: &BTreeMap<u32, NewReputation>) -> Vec<PreparedStatement> {
    let mut rows = Vec::new();
    // BTreeMap traverses RepListID keys; the saved faction comes from each value.
    for faction in data.values().filter(|faction| faction.need_save) {
        let mut delete = guid_statement(REPUTATION_DELETE, guid);
        delete.set_u16(1, faction.faction as u16);
        rows.push(delete);
        let mut insert = guid_statement(REPUTATION_INSERT, guid);
        insert.set_u16(1, faction.faction as u16);
        insert.set_i32(2, faction.standing);
        insert.set_u16(3, faction.flags);
        rows.push(insert);
    }
    rows
}

fn quest_statements(guid: u64, data: &NewQuestCriteriaSave) -> Vec<PreparedStatement> {
    let mut rows = vec![guid_statement(OBJECTIVES_DELETE, guid)];
    for objective in &data.completed_objectives {
        let mut row = guid_statement(OBJECTIVE_INSERT, guid);
        row.set_u32(1, *objective);
        rows.push(row);
    }
    rows.extend(criteria_statements(
        guid,
        &data.progress,
        QUEST_PROGRESS_DELETE,
        QUEST_PROGRESS_INSERT,
    ));
    rows
}
