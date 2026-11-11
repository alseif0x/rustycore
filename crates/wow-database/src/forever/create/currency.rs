//! Player::_SaveCurrency projection at 02245dcd; Removed deliberately emits no DELETE.
use super::{PreparedStatement, SqlTransaction};
use wow_persistence::forever::creation::save::{NewCurrencySave, NewCurrencySaveState};

#[cfg(test)]
#[path = "../../../unit_tests/forever/creation_currency.rs"]
mod tests;

const REPLACE: &str = "REPLACE INTO character_currency (CharacterGuid, Currency, Quantity, WeeklyQuantity, TrackedQuantity, IncreasedCapQuantity, EarnedQuantity, Flags) VALUES (?, ?, ?, ?, ?, ?, ?, ?)";
const UPDATE: &str = "UPDATE character_currency SET Quantity = ?, WeeklyQuantity = ?, TrackedQuantity = ?, IncreasedCapQuantity = ?, EarnedQuantity = ?, Flags = ? WHERE CharacterGuid = ? AND Currency = ?";

pub(super) fn append(transaction: &mut SqlTransaction, guid: u64, currencies: &[NewCurrencySave]) {
    for row in statements(guid, currencies) {
        transaction.append(row);
    }
}

fn statements(
    guid: u64,
    currencies: &[NewCurrencySave],
) -> impl Iterator<Item = PreparedStatement> + '_ {
    currencies
        .iter()
        .filter_map(move |currency| currency_statement(guid, currency))
}

fn currency_statement(guid: u64, currency: &NewCurrencySave) -> Option<PreparedStatement> {
    // Consume the canonical owner's lookup projection; do not perform a new
    // store lookup or infer availability from the state or quantities here.
    if !currency.has_currency_entry {
        return None;
    }
    let (sql, base, owner) = match currency.state {
        NewCurrencySaveState::New => (REPLACE, 2, 0),
        NewCurrencySaveState::Changed => (UPDATE, 0, 6),
        NewCurrencySaveState::Unchanged | NewCurrencySaveState::Removed => return None,
    };
    let mut row = PreparedStatement::new(sql);
    row.set_u64(owner, guid);
    row.set_u16(owner + 1, currency.currency_id as u16);
    for (index, quantity) in [
        currency.quantity,
        currency.weekly_quantity,
        currency.tracked_quantity,
        currency.increased_cap_quantity,
        currency.earned_quantity,
    ]
    .into_iter()
    .enumerate()
    {
        row.set_u32(base + index, quantity);
    }
    row.set_u8(base + 5, currency.flags);
    Some(row)
}
