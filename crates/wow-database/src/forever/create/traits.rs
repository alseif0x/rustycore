//! Player::_SaveTraits dirty-state policies at 02245dcd, including NEW no-op.
use super::{PreparedStatement, SqlTransaction};
use wow_persistence::forever::creation::save::{
    NewTraitChange, NewTraitConfig, NewTraitEntry, NewTraitState,
};

#[cfg(test)]
#[path = "../../../unit_tests/forever/creation_traits.rs"]
mod tests;

const ENTRIES_DELETE: &str =
    "DELETE FROM character_trait_entry WHERE guid = ? AND traitConfigId = ?";
const CONFIG_DELETE: &str =
    "DELETE FROM character_trait_config WHERE guid = ? AND traitConfigId = ?";
const ACTIONS_DELETE: &str = "DELETE FROM character_action WHERE guid = ? AND traitConfigId = ?";
const CONFIG_INSERT: &str = "INSERT INTO character_trait_config (guid, traitConfigId, type, chrSpecializationId, combatConfigFlags, localIdentifier, skillLineId, traitSystemId, variationId, `name`) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";
const ENTRY_INSERT: &str = "INSERT INTO character_trait_entry (guid, traitConfigId, traitNodeId, traitNodeEntryId, `rank`) VALUES (?, ?, ?, ?, ?)";

pub(super) fn append(transaction: &mut SqlTransaction, guid: u64, changes: &[NewTraitChange]) {
    for statement in statements(guid, changes) {
        transaction.append(statement);
    }
}

fn statements(
    guid: u64,
    changes: &[NewTraitChange],
) -> impl Iterator<Item = PreparedStatement> + '_ {
    changes
        .iter()
        .flat_map(move |change| change_statements(guid, change))
}

fn change_statements(guid: u64, change: &NewTraitChange) -> Vec<PreparedStatement> {
    match &change.state {
        NewTraitState::Changed(config) => {
            let mut rows = vec![
                scoped_statement(ENTRIES_DELETE, guid, change.key),
                scoped_statement(CONFIG_DELETE, guid, change.key),
            ];
            if let Some(config) = config {
                rows.push(config_statement(guid, config));
                rows.extend(
                    config
                        .entries
                        .iter()
                        .map(|entry| entry_statement(guid, config.id, entry)),
                );
            }
            rows
        }
        NewTraitState::Removed => vec![
            scoped_statement(ENTRIES_DELETE, guid, change.key),
            scoped_statement(CONFIG_DELETE, guid, change.key),
            scoped_statement(ACTIONS_DELETE, guid, change.key),
        ],
        NewTraitState::Unchanged | NewTraitState::New | NewTraitState::Temporary => Vec::new(),
    }
}

fn scoped_statement(sql: &'static str, guid: u64, config_id: i32) -> PreparedStatement {
    let mut statement = PreparedStatement::new(sql);
    statement.set_u64(0, guid);
    statement.set_i32(1, config_id);
    statement
}

fn config_statement(guid: u64, config: &NewTraitConfig) -> PreparedStatement {
    let mut statement = scoped_statement(CONFIG_INSERT, guid, config.id);
    statement.set_i32(2, config.kind);
    match config.kind {
        1 => {
            statement.set_i32(3, config.specialization);
            statement.set_i32(4, config.combat_flags);
            statement.set_i32(5, config.local_identifier);
            for index in 6..=8 {
                statement.set_null(index);
            }
        }
        2 => {
            for index in 3..=5 {
                statement.set_null(index);
            }
            statement.set_i32(6, config.skill_line);
            statement.set_null(7);
            statement.set_null(8);
        }
        3 => {
            for index in 3..=6 {
                statement.set_null(index);
            }
            statement.set_i32(7, config.system);
            statement.set_i32(8, config.variation);
        }
        // The source default switch arm leaves fields 3..8 at the prepared
        // statement variant's initialized bool(false), not SQL NULL.
        _ => {}
    }
    statement.set_string(9, config.name.clone());
    statement
}

fn entry_statement(guid: u64, config_id: i32, entry: &NewTraitEntry) -> PreparedStatement {
    let mut statement = scoped_statement(ENTRY_INSERT, guid, config_id);
    statement.set_i32(2, entry.node);
    statement.set_i32(3, entry.entry);
    statement.set_i32(4, entry.rank);
    statement
}
