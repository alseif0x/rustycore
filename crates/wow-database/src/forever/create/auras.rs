//! Player::_SaveAuras projection at 02245dcd; no eligibility/key construction.
use super::{PreparedStatement, SqlTransaction};
use wow_persistence::forever::creation::save::{NewAuraEffect, NewAuraSave};

#[cfg(test)]
#[path = "../../../unit_tests/forever/creation_auras.rs"]
mod tests;

const EFFECTS_DELETE: &str = "DELETE FROM character_aura_effect WHERE guid = ?";
const AURAS_DELETE: &str = "DELETE FROM character_aura WHERE guid = ?";
const AURA_INSERT: &str = "INSERT INTO character_aura (guid, casterGuid, itemGuid, spell, effectMask, recalculateMask, difficulty, stackCount, maxDuration, remainTime, remainCharges, castItemId, castItemLevel) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";
const EFFECT_INSERT: &str = "INSERT INTO character_aura_effect (guid, casterGuid, itemGuid, spell, effectMask, effectIndex, amount, baseAmount) VALUES (?, ?, ?, ?, ?, ?, ?, ?)";

pub(super) fn append(transaction: &mut SqlTransaction, guid: u64, auras: &[NewAuraSave]) {
    for statement in statements(guid, auras) {
        transaction.append(statement);
    }
}

fn statements(guid: u64, auras: &[NewAuraSave]) -> Vec<PreparedStatement> {
    let mut rows = vec![
        guid_statement(EFFECTS_DELETE, guid),
        guid_statement(AURAS_DELETE, guid),
    ];
    // Input order projects AuraMap iteration. CanBeSaved and GenerateKey are
    // canonical construction obligations; the adapter consumes frozen results.
    for aura in auras.iter().filter(|aura| aura.saveable) {
        rows.push(aura_statement(guid, aura));
        rows.extend(
            aura.effects
                .iter()
                .map(|effect| effect_statement(guid, aura, effect)),
        );
    }
    rows
}

fn guid_statement(sql: &'static str, guid: u64) -> PreparedStatement {
    let mut statement = PreparedStatement::new(sql);
    statement.set_u64(0, guid);
    statement
}

fn key_statement(sql: &'static str, guid: u64, aura: &NewAuraSave) -> PreparedStatement {
    let mut statement = guid_statement(sql, guid);
    statement.set_bytes(1, aura.caster_guid.to_vec());
    statement.set_bytes(2, aura.item_guid.to_vec());
    statement.set_u32(3, aura.spell);
    statement.set_u32(4, aura.effect_mask);
    statement
}

fn aura_statement(guid: u64, aura: &NewAuraSave) -> PreparedStatement {
    let mut statement = key_statement(AURA_INSERT, guid, aura);
    statement.set_u32(5, aura.recalculate_mask);
    statement.set_u8(6, aura.difficulty);
    statement.set_u8(7, aura.stack_count);
    statement.set_i32(8, aura.max_duration);
    statement.set_i32(9, aura.remaining_time);
    statement.set_u8(10, aura.charges);
    statement.set_u32(11, aura.cast_item);
    statement.set_i32(12, aura.cast_item_level);
    statement
}

fn effect_statement(guid: u64, aura: &NewAuraSave, effect: &NewAuraEffect) -> PreparedStatement {
    let mut statement = key_statement(EFFECT_INSERT, guid, aura);
    statement.set_u8(5, effect.index);
    statement.set_f64(6, effect.amount);
    statement.set_f64(7, effect.base_amount);
    statement
}
