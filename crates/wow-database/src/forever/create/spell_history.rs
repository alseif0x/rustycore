//! SpellHistory::PersistenceHelper<Player>/SaveToDB at 02245dcd.
use super::{PreparedStatement, SqlTransaction};
use wow_persistence::forever::creation::save::{NewSpellCharge, NewSpellCooldown, NewSpellHistory};

#[cfg(test)]
#[path = "../../../unit_tests/forever/creation_spell_history.rs"]
mod tests;

const COOLDOWN_DELETE: &str = "DELETE FROM character_spell_cooldown WHERE guid = ?";
const COOLDOWN_INSERT: &str = "INSERT INTO character_spell_cooldown (guid, spell, item, time, categoryId, categoryEnd) VALUES (?, ?, ?, ?, ?, ?)";
const CHARGES_DELETE: &str = "DELETE FROM character_spell_charges WHERE guid = ?";
const CHARGE_INSERT: &str = "INSERT INTO character_spell_charges (guid, categoryId, rechargeStart, rechargeEnd) VALUES (?, ?, ?, ?)";

pub(super) fn append(transaction: &mut SqlTransaction, guid: u64, history: &NewSpellHistory) {
    for statement in statements(guid, history) {
        transaction.append(statement);
    }
}

fn statements(guid: u64, history: &NewSpellHistory) -> Vec<PreparedStatement> {
    let mut rows = vec![guid_statement(COOLDOWN_DELETE, guid)];
    for cooldown in history
        .cooldowns
        .iter()
        .filter(|cooldown| !cooldown.on_hold)
    {
        rows.push(cooldown_statement(guid, cooldown));
    }
    rows.push(guid_statement(CHARGES_DELETE, guid));
    for category in &history.charges {
        for charge in &category.charges {
            rows.push(charge_statement(guid, category.category, charge));
        }
    }
    rows
}

fn guid_statement(sql: &'static str, guid: u64) -> PreparedStatement {
    let mut statement = PreparedStatement::new(sql);
    statement.set_u64(0, guid);
    statement
}

fn cooldown_statement(guid: u64, cooldown: &NewSpellCooldown) -> PreparedStatement {
    let mut statement = guid_statement(COOLDOWN_INSERT, guid);
    statement.set_u32(1, cooldown.spell);
    statement.set_u32(2, cooldown.item);
    // Canonical construction owns Clock::to_time_t. SaveToDB checks only
    // OnHold; these projected seconds are neither converted nor expiry-pruned.
    statement.set_i64(3, cooldown.end_time);
    statement.set_u32(4, cooldown.category);
    statement.set_i64(5, cooldown.category_end);
    statement
}

fn charge_statement(guid: u64, category: u32, charge: &NewSpellCharge) -> PreparedStatement {
    let mut statement = guid_statement(CHARGE_INSERT, guid);
    statement.set_u32(1, category);
    statement.set_i64(2, charge.start_time);
    statement.set_i64(3, charge.end_time);
    statement
}
