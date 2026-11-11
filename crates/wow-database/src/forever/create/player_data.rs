//! Player::_SavePlayerData projection at 02245dcd; no DB2 lookup or dirty-state acknowledgement.
use super::{PreparedStatement, SqlTransaction};
use wow_persistence::forever::creation::save::{NewPlayerDataSave, NewPlayerDataValue};

#[cfg(test)]
#[path = "../../../unit_tests/forever/creation_player_data.rs"]
mod tests;

const ELEMENT_DELETE: &str = "DELETE FROM character_player_data_element WHERE characterGuid = ? AND playerDataElementCharacterId = ?";
const ELEMENT_INSERT: &str = "INSERT INTO character_player_data_element (characterGuid, playerDataElementCharacterId, floatValue, int64Value) VALUES (?, ?, ?, ?)";
const FLAG_DELETE: &str =
    "DELETE FROM character_player_data_flag WHERE characterGuid = ? AND storageIndex = ?";
const FLAG_INSERT: &str =
    "INSERT INTO character_player_data_flag (characterGuid, storageIndex, mask) VALUES (?, ?, ?)";

pub(super) fn append(transaction: &mut SqlTransaction, guid: u64, data: &NewPlayerDataSave) {
    for row in statements(guid, data) {
        transaction.append(row);
    }
}

fn statements(guid: u64, data: &NewPlayerDataSave) -> impl Iterator<Item = PreparedStatement> + '_ {
    data.elements
        .iter()
        .flat_map(move |(&id, value)| {
            [
                Some(scoped(ELEMENT_DELETE, guid, id)),
                element_insert(guid, id, value.as_ref()),
            ]
            .into_iter()
            .flatten()
        })
        .chain(data.flags.iter().flat_map(move |(&index, &mask)| {
            let insert = if mask != 0 {
                let mut row = scoped(FLAG_INSERT, guid, index);
                row.set_u64(2, mask);
                Some(row)
            } else {
                None
            };
            [Some(scoped(FLAG_DELETE, guid, index)), insert]
                .into_iter()
                .flatten()
        }))
}

fn scoped(sql: &'static str, guid: u64, key: u32) -> PreparedStatement {
    let mut row = PreparedStatement::new(sql);
    row.set_u64(0, guid);
    row.set_u32(1, key);
    row
}

fn element_insert(
    guid: u64,
    id: u32,
    value: Option<&NewPlayerDataValue>,
) -> Option<PreparedStatement> {
    let mut row = scoped(ELEMENT_INSERT, guid, id);
    match value? {
        NewPlayerDataValue::Int64(value) if *value != 0 => {
            row.set_null(2);
            row.set_i64(3, *value);
        }
        NewPlayerDataValue::Float(value) if *value != 0.0 => {
            row.set_f32(2, *value);
            row.set_null(3);
        }
        _ => return None,
    }
    Some(row)
}
