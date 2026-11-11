//! Garrison::SaveToDB/DeleteFromDB projection at 02245dcd.
use super::{PreparedStatement, SqlTransaction};
use wow_persistence::forever::creation::save::NewGarrisonSave;

#[cfg(test)]
#[path = "../../../unit_tests/forever/creation_garrison.rs"]
mod tests;

const DELETES: [&str; 4] = [
    "DELETE FROM character_garrison WHERE guid = ?",
    "DELETE FROM character_garrison_blueprints WHERE guid = ?",
    "DELETE FROM character_garrison_buildings WHERE guid = ?",
    // Preserve the source INNER JOIN anomaly; do not split into unconditional deletes.
    "DELETE gfab, gf FROM character_garrison_follower_abilities gfab INNER JOIN character_garrison_followers gf ON gfab.dbId = gf.dbId WHERE gf.guid = ?",
];
const ROOT: &str = "INSERT INTO character_garrison (guid, siteLevelId, followerActivationsRemainingToday) VALUES (?, ?, ?)";
const BLUEPRINT: &str =
    "INSERT INTO character_garrison_blueprints (guid, buildingId) VALUES (?, ?)";
const BUILDING: &str = "INSERT INTO character_garrison_buildings (guid, plotInstanceId, buildingId, timeBuilt, active) VALUES (?, ?, ?, ?, ?)";
const FOLLOWER: &str = "INSERT INTO character_garrison_followers (dbId, guid, followerId, quality, level, itemLevelWeapon, itemLevelArmor, xp, currentBuilding, currentMission, status) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";
const ABILITY: &str =
    "INSERT INTO character_garrison_follower_abilities (dbId, abilityId, slot) VALUES (?, ?, ?)";

pub(super) fn append(transaction: &mut SqlTransaction, guid: u64, data: Option<&NewGarrisonSave>) {
    for row in statements(guid, data) {
        transaction.append(row);
    }
}

fn statements(guid: u64, data: Option<&NewGarrisonSave>) -> Vec<PreparedStatement> {
    let Some(data) = data else {
        return Vec::new();
    };
    let mut rows = Vec::new();
    for sql in DELETES {
        let mut row = PreparedStatement::new(sql);
        row.set_u64(0, guid);
        rows.push(row);
    }
    let mut row = PreparedStatement::new(ROOT);
    row.set_u64(0, guid);
    row.set_u32(1, data.site_level_id);
    row.set_u32(2, data.follower_activations_remaining_today);
    rows.push(row);
    for building in &data.known_buildings {
        let mut row = PreparedStatement::new(BLUEPRINT);
        row.set_u64(0, guid);
        row.set_u32(1, *building);
        rows.push(row);
    }
    for building in data.plots.iter().flatten() {
        let mut row = PreparedStatement::new(BUILDING);
        row.set_u64(0, guid);
        row.set_u32(1, building.plot_instance_id);
        row.set_u32(2, building.building_id);
        row.set_i64(3, building.time_built);
        row.set_bool(4, building.active);
        rows.push(row);
    }
    for follower in &data.followers {
        let mut row = PreparedStatement::new(FOLLOWER);
        row.set_u64(0, follower.db_id);
        row.set_u64(1, guid);
        for (index, value) in [
            follower.follower_id,
            follower.quality,
            follower.level,
            follower.item_level_weapon,
            follower.item_level_armor,
            follower.xp,
            follower.current_building,
            follower.current_mission,
            follower.status,
        ]
        .into_iter()
        .enumerate()
        {
            row.set_u32(index + 2, value);
        }
        rows.push(row);
        for (slot, ability) in follower.abilities.iter().enumerate() {
            let mut row = PreparedStatement::new(ABILITY);
            row.set_u64(0, follower.db_id);
            row.set_u32(1, *ability);
            // The source uint8 slot counter wraps; canonical admission is separate.
            row.set_u8(2, slot as u8);
            rows.push(row);
        }
    }
    rows
}
