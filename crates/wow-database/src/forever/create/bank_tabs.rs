//! Player::_SaveCharacterBankTabSettings projection at 02245dcd; character bank only.
use super::{PreparedStatement, SqlTransaction};
use wow_persistence::forever::creation::save::NewBankTabSettings;

#[cfg(test)]
#[path = "../../../unit_tests/forever/creation_bank_tabs.rs"]
mod tests;

const DELETE: &str = "DELETE FROM character_bank_tab_settings WHERE characterGuid = ?";
const INSERT: &str = "INSERT INTO character_bank_tab_settings (characterGuid, tabId, name, icon, description, depositFlags) VALUES (?, ?, ?, ?, ?, ?)";

pub(super) fn append(transaction: &mut SqlTransaction, guid: u64, tabs: &[NewBankTabSettings]) {
    for row in statements(guid, tabs) {
        transaction.append(row);
    }
}

fn statements(
    guid: u64,
    tabs: &[NewBankTabSettings],
) -> impl Iterator<Item = PreparedStatement> + '_ {
    let mut delete = PreparedStatement::new(DELETE);
    delete.set_u64(0, guid);
    std::iter::once(delete).chain(tabs.iter().enumerate().map(move |(index, tab)| {
        let mut row = PreparedStatement::new(INSERT);
        row.set_u64(0, guid);
        // Preserve the source size_t -> uint8 binding, without adding admission policy.
        row.set_u8(1, index as u8);
        row.set_string(2, tab.name.clone());
        row.set_string(3, tab.icon.clone());
        row.set_string(4, tab.description.clone());
        row.set_i32(5, tab.deposit_flags);
        row
    }))
}
