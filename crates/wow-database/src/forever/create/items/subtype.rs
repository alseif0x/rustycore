//! Explicit item subtype saves that run before Item::SaveToDB's base row.
use super::{CreateError, PreparedStatement, SqlTransaction};
use wow_persistence::forever::creation::save::{NewItem, NewItemAzerite};

#[cfg(test)]
#[path = "../../../../unit_tests/forever/creation_item_subtype.rs"]
mod tests;

const SELECTED_ESSENCE_GROUPS: usize = 4;

pub(super) const DELETE_AZERITE: &str = "DELETE FROM item_instance_azerite WHERE itemGuid = ?";
pub(super) const DELETE_AZERITE_MILESTONES: &str =
    "DELETE FROM item_instance_azerite_milestone_power WHERE itemGuid = ?";
pub(super) const DELETE_AZERITE_ESSENCES: &str =
    "DELETE FROM item_instance_azerite_unlocked_essence WHERE itemGuid = ?";
pub(super) const INSERT_AZERITE: &str = "INSERT INTO item_instance_azerite (itemGuid, xp, level, knowledgeLevel, selectedAzeriteEssences1specId, selectedAzeriteEssences1azeriteEssenceId1, selectedAzeriteEssences1azeriteEssenceId2, selectedAzeriteEssences1azeriteEssenceId3, selectedAzeriteEssences1azeriteEssenceId4, selectedAzeriteEssences2specId, selectedAzeriteEssences2azeriteEssenceId1, selectedAzeriteEssences2azeriteEssenceId2, selectedAzeriteEssences2azeriteEssenceId3, selectedAzeriteEssences2azeriteEssenceId4, selectedAzeriteEssences3specId, selectedAzeriteEssences3azeriteEssenceId1, selectedAzeriteEssences3azeriteEssenceId2, selectedAzeriteEssences3azeriteEssenceId3, selectedAzeriteEssences3azeriteEssenceId4, selectedAzeriteEssences4specId, selectedAzeriteEssences4azeriteEssenceId1, selectedAzeriteEssences4azeriteEssenceId2, selectedAzeriteEssences4azeriteEssenceId3, selectedAzeriteEssences4azeriteEssenceId4) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";
pub(super) const INSERT_AZERITE_MILESTONE: &str = "INSERT INTO item_instance_azerite_milestone_power (itemGuid, azeriteItemMilestonePowerId) VALUES (?, ?)";
pub(super) const INSERT_AZERITE_ESSENCE: &str = "INSERT INTO item_instance_azerite_unlocked_essence (itemGuid, azeriteEssenceId, `rank`) VALUES (?, ?, ?)";
pub(super) const DELETE_AZERITE_EMPOWERED: &str =
    "DELETE FROM item_instance_azerite_empowered WHERE itemGuid = ?";
pub(super) const INSERT_AZERITE_EMPOWERED: &str = "INSERT INTO item_instance_azerite_empowered (itemGuid, azeritePowerId1, azeritePowerId2, azeritePowerId3, azeritePowerId4, azeritePowerId5) VALUES (?, ?, ?, ? ,? ,?)";

pub(super) fn validate(item: &NewItem) -> Result<(), CreateError> {
    if matches!(&item.subtype, wow_persistence::forever::creation::save::NewItemSubtype::Azerite(data) if data.selected_essences.len() > SELECTED_ESSENCE_GROUPS)
    {
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
    use wow_persistence::forever::creation::save::NewItemSubtype;

    match &item.subtype {
        NewItemSubtype::Base => Vec::new(),
        NewItemSubtype::Azerite(data) => azerite_statements(item.guid, data),
        NewItemSubtype::AzeriteEmpowered(power_ids) => vec![
            item_guid_statement(DELETE_AZERITE_EMPOWERED, item.guid),
            empowered_insert_statement(item.guid, power_ids),
        ],
    }
}

fn azerite_statements(item_guid: u64, data: &NewItemAzerite) -> Vec<PreparedStatement> {
    let mut statements =
        Vec::with_capacity(4 + data.unlocked_milestones.len() + data.unlocked_essences.len());
    statements.push(item_guid_statement(DELETE_AZERITE, item_guid));
    statements.push(item_guid_statement(DELETE_AZERITE_MILESTONES, item_guid));
    statements.push(item_guid_statement(DELETE_AZERITE_ESSENCES, item_guid));
    statements.push(azerite_insert_statement(item_guid, data));
    statements.extend(
        data.unlocked_milestones
            .iter()
            .map(|milestone| milestone_insert_statement(item_guid, *milestone)),
    );
    statements.extend(
        data.unlocked_essences
            .iter()
            .map(|essence| essence_insert_statement(item_guid, essence.id, essence.rank)),
    );
    statements
}

fn item_guid_statement(sql: &'static str, item_guid: u64) -> PreparedStatement {
    let mut statement = PreparedStatement::new(sql);
    statement.set_u64(0, item_guid);
    statement
}

fn azerite_insert_statement(item_guid: u64, data: &NewItemAzerite) -> PreparedStatement {
    let mut statement = PreparedStatement::new(INSERT_AZERITE);
    statement.set_u64(0, item_guid);
    statement.set_u64(1, data.xp);
    statement.set_u32(2, data.level);
    statement.set_u32(3, data.knowledge_level);
    for index in 0..SELECTED_ESSENCE_GROUPS {
        let parameter = 4 + index * 5;
        if let Some(group) = data.selected_essences.get(index) {
            statement.set_u32(parameter, group.specialization_id);
            for (essence_index, essence_id) in group.essence_ids.iter().enumerate() {
                statement.set_u32(parameter + 1 + essence_index, *essence_id);
            }
        } else {
            for offset in 0..5 {
                statement.set_u32(parameter + offset, 0);
            }
        }
    }
    statement
}

fn milestone_insert_statement(item_guid: u64, milestone: u32) -> PreparedStatement {
    let mut statement = PreparedStatement::new(INSERT_AZERITE_MILESTONE);
    statement.set_u64(0, item_guid);
    statement.set_u32(1, milestone);
    statement
}

fn essence_insert_statement(item_guid: u64, essence_id: u32, rank: u32) -> PreparedStatement {
    let mut statement = PreparedStatement::new(INSERT_AZERITE_ESSENCE);
    statement.set_u64(0, item_guid);
    statement.set_u32(1, essence_id);
    statement.set_u32(2, rank);
    statement
}

fn empowered_insert_statement(item_guid: u64, power_ids: &[i32; 5]) -> PreparedStatement {
    let mut statement = PreparedStatement::new(INSERT_AZERITE_EMPOWERED);
    statement.set_u64(0, item_guid);
    for (index, power_id) in power_ids.iter().enumerate() {
        statement.set_i32(index + 1, *power_id);
    }
    statement
}
