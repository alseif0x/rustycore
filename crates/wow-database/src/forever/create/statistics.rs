//! Player::_SaveStats SQL setter projection at 02245dcd; eligibility is already frozen.
use super::{PreparedStatement, SqlTransaction};
use wow_persistence::forever::creation::save::NewCharacterStatistics;

#[cfg(test)]
#[path = "../../../unit_tests/forever/creation_statistics.rs"]
mod tests;

const DELETE: &str = "DELETE FROM character_stats WHERE guid = ?";
const INSERT: &str = concat!(
    "INSERT INTO character_stats (guid, maxhealth, maxpower1, maxpower2, maxpower3, maxpower4, maxpower5, maxpower6, maxpower7, maxpower8, maxpower9, maxpower10, ",
    "strength, agility, stamina, intellect, spirit, armor, resHoly, resFire, resNature, resFrost, resShadow, resArcane, blockPct, dodgePct, parryPct, critPct, rangedCritPct, spellCritPct, ",
    "attackPower, rangedAttackPower, spellPower, resilience, mastery, versatility) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
);

pub(super) fn append(
    transaction: &mut SqlTransaction,
    guid: u64,
    data: Option<&NewCharacterStatistics>,
) {
    for row in statements(guid, data) {
        transaction.append(row);
    }
}

fn statements(guid: u64, data: Option<&NewCharacterStatistics>) -> Vec<PreparedStatement> {
    let Some(data) = data else {
        return Vec::new();
    };
    let mut delete = PreparedStatement::new(DELETE);
    delete.set_u64(0, guid);
    let mut row = PreparedStatement::new(INSERT);
    row.set_u64(0, guid);
    row.set_u32(1, data.max_health);
    for (index, value) in data
        .powers
        .iter()
        .chain(&data.stats)
        .chain(&data.resistances)
        .enumerate()
    {
        row.set_u32(index + 2, *value);
    }
    for (index, value) in data.percentages.iter().enumerate() {
        row.set_f32(index + 24, *value);
    }
    row.set_u32(30, data.attack_power);
    row.set_u32(31, data.ranged_attack_power);
    row.set_u32(32, data.spell_power);
    // Retain the source's missing index increment: mastery overwrites resilience,
    // versatility occupies 34, and 35 retains the C++ value-initialized bool false.
    row.set_u32(33, data.resilience);
    row.set_f32(33, data.mastery);
    row.set_i32(34, data.versatility);
    row.set_bool(35, false);
    vec![delete, row]
}
