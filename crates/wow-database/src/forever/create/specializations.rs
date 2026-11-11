//! Player::_SaveGlyphs/_SaveTalents creation saves at 02245dcd.
use super::{PreparedStatement, SqlTransaction};
use wow_persistence::forever::creation::save::NewSpecializations;

#[cfg(test)]
#[path = "../../../unit_tests/forever/creation_specializations.rs"]
mod tests;

const GLYPH_DELETE: &str = "DELETE FROM character_glyphs WHERE guid = ?";
const GLYPH_INSERT: &str = "INSERT INTO character_glyphs VALUES(?, ?, ?)";
const TALENT_DELETE: &str = "DELETE FROM character_talent WHERE guid = ?";
const TALENT_INSERT: &str =
    "INSERT INTO character_talent (guid, talentId, talentGroup) VALUES (?, ?, ?)";
const PVP_DELETE: &str = "DELETE FROM character_pvp_talent WHERE guid = ?";
const PVP_INSERT: &str = "INSERT INTO character_pvp_talent (guid, talentId0, talentId1, talentId2, talentId3, talentGroup) VALUES (?, ?, ?, ?, ?, ?)";

pub(super) fn append(transaction: &mut SqlTransaction, guid: u64, data: &NewSpecializations) {
    for statement in statements(guid, data) {
        transaction.append(statement);
    }
}

fn statements(guid: u64, data: &NewSpecializations) -> Vec<PreparedStatement> {
    let mut rows = vec![guid_statement(GLYPH_DELETE, guid)];
    for (group, glyphs) in data.glyphs.iter().enumerate() {
        for glyph in glyphs {
            let mut row = guid_statement(GLYPH_INSERT, guid);
            row.set_u8(1, group as u8);
            row.set_u16(2, *glyph as u16);
            rows.push(row);
        }
    }

    rows.push(guid_statement(TALENT_DELETE, guid));
    for (group, talents) in data.talents.iter().enumerate() {
        // PlayerTalentMap is unordered; retain supplied order and skip only
        // PLAYERSPELL_REMOVED. Other source states share this INSERT.
        for talent in talents.iter().filter(|talent| !talent.removed) {
            let mut row = guid_statement(TALENT_INSERT, guid);
            row.set_u32(1, talent.id);
            row.set_u8(2, group as u8);
            rows.push(row);
        }
    }

    rows.push(guid_statement(PVP_DELETE, guid));
    for (group, slots) in data.pvp.iter().enumerate() {
        let mut row = guid_statement(PVP_INSERT, guid);
        for (index, talent) in slots.iter().enumerate() {
            row.set_u32(1 + index, *talent);
        }
        row.set_u8(5, group as u8);
        rows.push(row);
    }
    rows
}

fn guid_statement(sql: &'static str, guid: u64) -> PreparedStatement {
    let mut statement = PreparedStatement::new(sql);
    statement.set_u64(0, guid);
    statement
}
