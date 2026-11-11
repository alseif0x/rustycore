//! Periodic quest changed-state saves from Player.cpp:21605-21720 at 02245dcd.
use super::PreparedStatement;
use std::collections::BTreeSet;
use wow_persistence::forever::creation::save::{
    NewDailyQuestSave, NewQuestSave, NewSeasonalQuestGroup,
};

#[cfg(test)]
#[path = "../../../../unit_tests/forever/creation_quest_periodic.rs"]
mod tests;

const DAILY_DELETE: &str = "DELETE FROM character_queststatus_daily WHERE guid = ?";
const DAILY_INSERT: &str =
    "INSERT INTO character_queststatus_daily (guid, quest, time) VALUES (?, ?, ?)";
const WEEKLY_DELETE: &str = "DELETE FROM character_queststatus_weekly WHERE guid = ?";
const WEEKLY_INSERT: &str = "INSERT INTO character_queststatus_weekly (guid, quest) VALUES (?, ?)";
const SEASONAL_DELETE: &str = "DELETE FROM character_queststatus_seasonal WHERE guid = ?";
const SEASONAL_INSERT: &str = "INSERT INTO character_queststatus_seasonal (guid, quest, event, completedTime) VALUES (?, ?, ?, ?)";
const MONTHLY_DELETE: &str = "DELETE FROM character_queststatus_monthly WHERE guid = ?";
const MONTHLY_INSERT: &str =
    "INSERT INTO character_queststatus_monthly (guid, quest) VALUES (?, ?)";

pub(super) fn statements(
    guid: u64,
    save: &NewQuestSave,
) -> impl Iterator<Item = PreparedStatement> {
    daily_statements(guid, save.daily.as_ref())
        .into_iter()
        .chain(set_statements(
            guid,
            save.weekly.as_ref(),
            WEEKLY_DELETE,
            WEEKLY_INSERT,
        ))
        .chain(seasonal_statements(guid, save.seasonal.as_deref()))
        .chain(set_statements(
            guid,
            save.monthly.as_ref(),
            MONTHLY_DELETE,
            MONTHLY_INSERT,
        ))
}

fn guid_statement(sql: &'static str, guid: u64) -> PreparedStatement {
    let mut statement = PreparedStatement::new(sql);
    statement.set_u64(0, guid);
    statement
}

fn daily_statements(guid: u64, data: Option<&NewDailyQuestSave>) -> Vec<PreparedStatement> {
    let Some(data) = data else { return Vec::new() };
    let mut rows = vec![guid_statement(DAILY_DELETE, guid)];
    for quest in data
        .quests
        .iter()
        .map(|quest| *quest as u32)
        .chain(data.dungeon_quests.iter().copied())
    {
        let mut row = guid_statement(DAILY_INSERT, guid);
        row.set_u32(1, quest);
        row.set_i64(2, data.last_time);
        rows.push(row);
    }
    rows
}

fn set_statements(
    guid: u64,
    quests: Option<&BTreeSet<u32>>,
    delete_sql: &'static str,
    insert_sql: &'static str,
) -> Vec<PreparedStatement> {
    // Weekly/monthly changed-but-empty sets produce no DELETE in the source.
    let Some(quests) = quests.filter(|quests| !quests.is_empty()) else {
        return Vec::new();
    };
    let mut rows = vec![guid_statement(delete_sql, guid)];
    for quest in quests {
        let mut row = guid_statement(insert_sql, guid);
        row.set_u32(1, *quest);
        rows.push(row);
    }
    rows
}

fn seasonal_statements(
    guid: u64,
    groups: Option<&[NewSeasonalQuestGroup]>,
) -> Vec<PreparedStatement> {
    let Some(groups) = groups else {
        return Vec::new();
    };
    let mut rows = vec![guid_statement(SEASONAL_DELETE, guid)];
    for group in groups {
        // The C++ uint32 map key passes through a uint16 local before setUInt32.
        let event = u32::from(group.event as u16);
        for quest in &group.quests {
            let mut row = guid_statement(SEASONAL_INSERT, guid);
            row.set_u32(1, quest.quest);
            row.set_u32(2, event);
            row.set_i64(3, quest.completed_time);
            rows.push(row);
        }
    }
    rows
}
