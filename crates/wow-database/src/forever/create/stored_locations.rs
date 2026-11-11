//! Player::_SaveStoredAuraTeleportLocations at 02245dcd, source policy unrepaired.
use super::{PreparedStatement, SqlTransaction};
use wow_persistence::forever::creation::save::{
    NewStoredAuraLocation, NewStoredAuraLocationState, NewStoredAuraPosition,
};

#[cfg(test)]
#[path = "../../../unit_tests/forever/creation_stored_locations.rs"]
mod tests;

const DELETE: &str = "DELETE FROM character_aura_stored_location WHERE Guid = ? AND Spell = ?";
const INSERT: &str = "INSERT INTO character_aura_stored_location (Guid, Spell, MapId, PositionX, PositionY, PositionZ, Orientation) VALUES (?, ?, ?, ?, ?, ?, ?)";

pub(super) fn append(
    transaction: &mut SqlTransaction,
    guid: u64,
    locations: &[NewStoredAuraLocation],
) {
    for statement in statements(guid, locations) {
        transaction.append(statement);
    }
}

fn statements(
    guid: u64,
    locations: &[NewStoredAuraLocation],
) -> impl Iterator<Item = PreparedStatement> + '_ {
    locations
        .iter()
        .flat_map(move |location| match &location.state {
            NewStoredAuraLocationState::Unchanged => Vec::new(),
            NewStoredAuraLocationState::Deleted => vec![delete_statement(guid)],
            NewStoredAuraLocationState::Changed(position) => vec![
                delete_statement(guid),
                insert_statement(guid, location.spell, position),
            ],
        })
}

fn delete_statement(guid: u64) -> PreparedStatement {
    let mut statement = PreparedStatement::new(DELETE);
    statement.set_u64(0, guid);
    // Both source branches leave Spell (index 1) unset. Its initialized
    // Bool(false) binds spell zero, not the dirty spell key. Preserve this
    // source policy without repairing the key or mutating canonical state.
    statement
}

fn insert_statement(guid: u64, spell: u32, position: &NewStoredAuraPosition) -> PreparedStatement {
    let mut statement = PreparedStatement::new(INSERT);
    statement.set_u64(0, guid);
    statement.set_u32(1, spell);
    statement.set_u32(2, position.map);
    for (index, value) in position.position.iter().enumerate() {
        statement.set_f32(3 + index, *value);
    }
    statement
}
