//! Item::SaveToDB's base auxiliary groups, appended after the primary row.
use super::{CreateError, PreparedStatement, SqlTransaction};
use std::fmt::Write;
use wow_persistence::forever::creation::save::{
    NewItem, NewItemArtifact, NewItemArtifactPower, NewItemGem,
};

#[cfg(test)]
#[path = "../../../../unit_tests/forever/creation_item_aux.rs"]
mod tests;

const GEM_SOCKET_COUNT: usize = 3;

pub(super) const DELETE_GEMS: &str = "DELETE FROM item_instance_gems WHERE itemGuid = ?";
pub(super) const INSERT_GEMS: &str = "INSERT INTO item_instance_gems (itemGuid, gemItemId1, gemBonuses1, gemContext1, gemScalingLevel1, gemItemId2, gemBonuses2, gemContext2, gemScalingLevel2, gemItemId3, gemBonuses3, gemContext3, gemScalingLevel3) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";
pub(super) const DELETE_TRANSMOG: &str = "DELETE FROM item_instance_transmog WHERE itemGuid = ?";
pub(super) const INSERT_TRANSMOG: &str = "INSERT INTO item_instance_transmog (itemGuid, itemModifiedAppearanceAllSpecs, itemModifiedAppearanceSpec1, itemModifiedAppearanceSpec2, itemModifiedAppearanceSpec3, itemModifiedAppearanceSpec4, itemModifiedAppearanceSpec5, spellItemEnchantmentAllSpecs, spellItemEnchantmentSpec1, spellItemEnchantmentSpec2, spellItemEnchantmentSpec3, spellItemEnchantmentSpec4, spellItemEnchantmentSpec5, secondaryItemModifiedAppearanceAllSpecs, secondaryItemModifiedAppearanceSpec1, secondaryItemModifiedAppearanceSpec2, secondaryItemModifiedAppearanceSpec3, secondaryItemModifiedAppearanceSpec4, secondaryItemModifiedAppearanceSpec5) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";
pub(super) const DELETE_ARTIFACT: &str = "DELETE FROM item_instance_artifact WHERE itemGuid = ?";
pub(super) const DELETE_ARTIFACT_POWERS: &str =
    "DELETE FROM item_instance_artifact_powers WHERE itemGuid = ?";
pub(super) const INSERT_ARTIFACT: &str = "INSERT INTO item_instance_artifact (itemGuid, xp, artifactAppearanceId, artifactTierId) VALUES (?, ?, ?, ?)";
pub(super) const INSERT_ARTIFACT_POWER: &str = "INSERT INTO item_instance_artifact_powers (itemGuid, artifactPowerId, purchasedRank) VALUES (?, ?, ?)";
pub(super) const DELETE_MODIFIERS: &str = "DELETE FROM item_instance_modifiers WHERE itemGuid = ?";
pub(super) const INSERT_MODIFIERS: &str = "INSERT INTO item_instance_modifiers (itemGuid, fixedScalingLevel, artifactKnowledgeLevel) VALUES (?, ?, ?)";

pub(super) fn validate(item: &NewItem) -> Result<(), CreateError> {
    if item.gems.len() > GEM_SOCKET_COUNT {
        return Err(CreateError::InvalidRow);
    }
    Ok(())
}

pub(super) fn append(transaction: &mut SqlTransaction, item: &NewItem) {
    for statement in statements(item) {
        transaction.append(statement);
    }
}

pub(super) fn statements(item: &NewItem) -> Vec<PreparedStatement> {
    let mut statements = Vec::with_capacity(
        9 + item.gems.len() + item.artifact.as_ref().map_or(0, |a| a.powers.len()),
    );

    statements.push(item_guid_statement(DELETE_GEMS, item.guid));
    if !item.gems.is_empty() {
        statements.push(gems_insert_statement(item.guid, &item.gems));
    }

    statements.push(item_guid_statement(DELETE_TRANSMOG, item.guid));
    if item.transmog_modifiers.iter().any(|value| *value != 0) {
        statements.push(transmog_insert_statement(item));
    }

    statements.push(item_guid_statement(DELETE_ARTIFACT, item.guid));
    statements.push(item_guid_statement(DELETE_ARTIFACT_POWERS, item.guid));
    if let Some(artifact) = &item.artifact {
        statements.push(artifact_insert_statement(item.guid, artifact));
        statements.extend(
            artifact
                .powers
                .iter()
                .map(|power| artifact_power_insert_statement(item.guid, power)),
        );
    }

    statements.push(item_guid_statement(DELETE_MODIFIERS, item.guid));
    if item.modifiers.timewalker_level != 0 || item.modifiers.artifact_knowledge_level != 0 {
        statements.push(modifiers_insert_statement(item));
    }
    statements
}

fn item_guid_statement(sql: &'static str, item_guid: u64) -> PreparedStatement {
    let mut statement = PreparedStatement::new(sql);
    statement.set_u64(0, item_guid);
    statement
}

fn gems_insert_statement(item_guid: u64, gems: &[NewItemGem]) -> PreparedStatement {
    let mut statement = PreparedStatement::new(INSERT_GEMS);
    statement.set_u64(0, item_guid);
    for socket in 0..GEM_SOCKET_COUNT {
        let parameter = 1 + socket * 4;
        match gems.get(socket).filter(|gem| gem.item_id != 0) {
            Some(gem) => {
                statement.set_u32(parameter, gem.item_id);
                statement.set_string(parameter + 1, bonus_list_string(&gem.bonus_list_ids));
                statement.set_u8(parameter + 2, gem.context);
                statement.set_u32(parameter + 3, gem.scaling_level);
            }
            None => {
                statement.set_u32(parameter, 0);
                statement.set_string(parameter + 1, String::new());
                statement.set_u8(parameter + 2, 0);
                statement.set_u32(parameter + 3, 0);
            }
        }
    }
    statement
}

fn bonus_list_string(bonus_list_ids: &[u16; 16]) -> String {
    let mut output = String::new();
    for id in bonus_list_ids.iter().filter(|id| **id != 0) {
        write!(output, "{id} ").expect("String formatting");
    }
    output
}

fn transmog_insert_statement(item: &NewItem) -> PreparedStatement {
    let mut statement = PreparedStatement::new(INSERT_TRANSMOG);
    statement.set_u64(0, item.guid);
    for (index, value) in item.transmog_modifiers.iter().enumerate() {
        // Item::SaveToDB calls setUInt32 for these fields even though the
        // legacy transmog table declares them signed.
        statement.set_u32(index + 1, *value);
    }
    statement
}

fn artifact_insert_statement(item_guid: u64, artifact: &NewItemArtifact) -> PreparedStatement {
    let mut statement = PreparedStatement::new(INSERT_ARTIFACT);
    statement.set_u64(0, item_guid);
    statement.set_u64(1, artifact.xp);
    statement.set_u32(2, artifact.appearance_id);
    statement.set_u32(3, artifact.tier_id);
    statement
}

fn artifact_power_insert_statement(
    item_guid: u64,
    power: &NewItemArtifactPower,
) -> PreparedStatement {
    let mut statement = PreparedStatement::new(INSERT_ARTIFACT_POWER);
    statement.set_u64(0, item_guid);
    statement.set_u32(1, power.id);
    statement.set_u8(2, power.purchased_rank);
    statement
}

fn modifiers_insert_statement(item: &NewItem) -> PreparedStatement {
    let mut statement = PreparedStatement::new(INSERT_MODIFIERS);
    statement.set_u64(0, item.guid);
    statement.set_u32(1, item.modifiers.timewalker_level);
    statement.set_u32(2, item.modifiers.artifact_knowledge_level);
    statement
}
