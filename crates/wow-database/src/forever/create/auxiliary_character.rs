//! Player::SaveToDB fishing and battleground groups at 02245dcd.
use super::{PreparedStatement, SqlTransaction};
use wow_persistence::forever::creation::save::NewBattlegroundData;

#[cfg(test)]
#[path = "../../../unit_tests/forever/creation_character_aux.rs"]
mod tests;

const FISHING_DELETE: &str = "DELETE FROM character_fishingsteps WHERE guid = ?";
const FISHING_INSERT: &str =
    "INSERT INTO character_fishingsteps (guid, fishingSteps) VALUES (?, ?)";
const BATTLEGROUND_DELETE: &str = "DELETE FROM character_battleground_data WHERE guid = ?";
const BATTLEGROUND_INSERT: &str = "INSERT INTO character_battleground_data (guid, instanceId, team, joinX, joinY, joinZ, joinO, joinMapId, taxiStart, taxiEnd, mountSpell, queueId) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";

pub(super) fn fishing_delete_statement(guid: u64) -> PreparedStatement {
    let mut statement = PreparedStatement::new(FISHING_DELETE);
    statement.set_u64(0, guid);
    statement
}

pub(super) fn append_fishing(transaction: &mut SqlTransaction, guid: u64, steps: u8) {
    if steps != 0 {
        transaction.append(fishing_insert_statement(guid, steps));
    }
}

fn fishing_insert_statement(guid: u64, steps: u8) -> PreparedStatement {
    let mut statement = PreparedStatement::new(FISHING_INSERT);
    statement.set_u64(0, guid);
    statement.set_u32(1, u32::from(steps));
    statement
}

pub(super) fn append_battleground(
    transaction: &mut SqlTransaction,
    guid: u64,
    data: &NewBattlegroundData,
) {
    transaction.append(battleground_delete_statement(guid));
    transaction.append(battleground_insert_statement(guid, data));
}

fn battleground_delete_statement(guid: u64) -> PreparedStatement {
    let mut statement = PreparedStatement::new(BATTLEGROUND_DELETE);
    statement.set_u64(0, guid);
    statement
}

fn battleground_insert_statement(guid: u64, data: &NewBattlegroundData) -> PreparedStatement {
    let mut statement = PreparedStatement::new(BATTLEGROUND_INSERT);
    statement.set_u64(0, guid);
    statement.set_u32(1, data.instance);
    statement.set_u16(2, data.team);
    for (index, value) in data.position.iter().enumerate() {
        // _SaveBGData binds raw floats; primary-row finiteAlways does not apply.
        statement.set_f32(3 + index, *value);
    }
    statement.set_u16(7, data.map);
    statement.set_u32(8, data.taxi[0]);
    statement.set_u32(9, data.taxi[1]);
    statement.set_u32(10, data.mount_spell);
    statement.set_u64(11, data.queue_id);
    statement
}
