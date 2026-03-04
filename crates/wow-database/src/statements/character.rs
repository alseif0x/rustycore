//! Character database prepared statement definitions.
//!
//! These correspond to the `characters` database and the C# `CharStatements` enum.

use super::StatementDef;

/// Prepared statements for the character database.
///
/// Covers character list, creation, deletion, and login operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types)]
pub enum CharStatements {
    /// SELECT c.guid, c.name, c.race, c.class, c.gender, c.level, c.zone, c.map,
    /// c.position_x, c.position_y, c.position_z, IFNULL(gm.guildid, 0), c.playerFlags,
    /// c.at_login, c.equipmentCache, c.lastLoginBuild
    /// FROM characters AS c LEFT JOIN guild_member AS gm ON c.guid = gm.guid
    /// WHERE c.account = ? AND c.deleteInfos_Name IS NULL
    SEL_ENUM,

    /// SELECT name FROM characters WHERE name = ?
    SEL_CHECK_NAME,

    /// SELECT COUNT(*) FROM characters WHERE account = ?
    SEL_SUM_CHARS,

    /// INSERT INTO characters (guid, account, name, race, class, gender, level, money,
    /// zone, map, position_x, position_y, position_z, orientation,
    /// taximask, createTime, createMode, playerFlags, at_login,
    /// health, power1, lastLoginBuild) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
    INS_CHARACTER,

    /// INSERT INTO character_customizations (guid, chrCustomizationOptionID,
    /// chrCustomizationChoiceID) VALUES (?,?,?)
    INS_CHAR_CUSTOMIZATION,

    /// DELETE FROM characters WHERE guid = ?
    DEL_CHARACTER,

    /// SELECT guid, account, name, race, class, gender, level, zone, map,
    /// position_x, position_y, position_z, orientation, playerFlags, at_login
    /// FROM characters WHERE guid = ?
    SEL_CHARACTER,

    /// UPDATE characters SET online = 1 WHERE guid = ?
    UPD_CHAR_ONLINE,

    /// UPDATE characters SET online = 0 WHERE guid = ?
    UPD_CHAR_OFFLINE,

    /// SELECT guid, account FROM characters WHERE guid = ? AND account = ?
    SEL_CHAR_DEL_CHECK,

    /// SELECT MAX(guid) FROM characters
    SEL_MAX_GUID,

    /// SELECT ci.slot, ii.itemEntry, ci.item
    /// FROM character_inventory ci
    /// JOIN item_instance ii ON ci.item = ii.guid
    /// WHERE ci.guid = ? AND ci.bag = 0 AND ci.slot < 19
    SEL_CHAR_EQUIPMENT,

    /// UPDATE character_inventory SET slot = ? WHERE guid = ? AND item = ?
    UPD_CHAR_INVENTORY_SLOT,

    /// DELETE FROM character_inventory WHERE guid = ? AND item = ?
    DEL_CHAR_INVENTORY_ITEM,

    /// SELECT skill FROM character_skills WHERE guid = ?
    SEL_CHARACTER_SKILLS,

    /// SELECT spell, active, disabled FROM character_spell WHERE guid = ?
    SEL_CHARACTER_SPELL,

    /// SELECT button, action, type FROM character_action
    /// WHERE guid = ? AND spec = ? AND traitConfigId = ? ORDER BY button
    SEL_CHARACTER_ACTIONS_SPEC,

    /// INSERT INTO character_action (guid, spec, traitConfigId, button, action, type)
    /// VALUES (?, 0, 0, ?, ?, ?)
    INS_CHARACTER_ACTION,

    /// UPDATE characters SET totaltime = ?, leveltime = ? WHERE guid = ?
    UPD_CHAR_PLAYED_TIME,
    // Quest status
    SEL_CHAR_QUEST_STATUS,
    INS_CHAR_QUEST_STATUS,
    DEL_CHAR_QUEST_STATUS,

    /// UPDATE characters SET money = ? WHERE guid = ?
    UPD_CHAR_MONEY,

    /// SELECT MAX(guid) FROM item_instance
    SEL_MAX_ITEM_GUID,

    /// INSERT INTO item_instance (guid, itemEntry, owner_guid, count, durability, enchantments, charges)
    /// VALUES (?, ?, ?, ?, ?, '', '')
    INS_ITEM_INSTANCE,

    /// INSERT INTO character_inventory (guid, bag, slot, item) VALUES (?, 0, ?, ?)
    INS_CHAR_INVENTORY,

    /// DELETE FROM item_instance WHERE guid = ?
    DEL_ITEM_INSTANCE,

    /// INSERT IGNORE INTO character_spell (guid, spell, active, disabled) VALUES (?, ?, 1, 0)
    INS_CHARACTER_SPELL,
}

impl StatementDef for CharStatements {
    fn sql(self) -> &'static str {
        match self {
            Self::SEL_ENUM => {
                "SELECT c.guid, c.name, c.race, c.class, c.gender, c.level, c.zone, c.map, \
                 c.position_x, c.position_y, c.position_z, IFNULL(gm.guildid, 0), c.playerFlags, \
                 c.at_login, c.equipmentCache, c.lastLoginBuild \
                 FROM characters AS c \
                 LEFT JOIN guild_member AS gm ON c.guid = gm.guid \
                 WHERE c.account = ? AND c.deleteInfos_Name IS NULL"
            }
            Self::SEL_CHECK_NAME => {
                "SELECT name FROM characters WHERE name = ?"
            }
            Self::SEL_SUM_CHARS => {
                "SELECT COUNT(*) FROM characters WHERE account = ?"
            }
            Self::INS_CHARACTER => {
                "INSERT INTO characters (guid, account, name, race, class, gender, level, money, \
                 zone, map, position_x, position_y, position_z, orientation, \
                 taximask, createTime, createMode, playerFlags, at_login, \
                 health, power1, lastLoginBuild) \
                 VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"
            }
            Self::INS_CHAR_CUSTOMIZATION => {
                "INSERT INTO character_customizations (guid, chrCustomizationOptionID, \
                 chrCustomizationChoiceID) VALUES (?,?,?)"
            }
            Self::DEL_CHARACTER => {
                "DELETE FROM characters WHERE guid = ?"
            }
            Self::SEL_CHARACTER => {
                "SELECT guid, account, name, race, class, gender, level, zone, map, \
                 position_x, position_y, position_z, orientation, playerFlags, at_login, \
                 totaltime, leveltime, money \
                 FROM characters WHERE guid = ?"
            }
            Self::UPD_CHAR_ONLINE => {
                "UPDATE characters SET online = 1 WHERE guid = ?"
            }
            Self::UPD_CHAR_OFFLINE => {
                "UPDATE characters SET online = 0 WHERE guid = ?"
            }
            Self::SEL_CHAR_DEL_CHECK => {
                "SELECT guid, account FROM characters WHERE guid = ? AND account = ?"
            }
            Self::SEL_MAX_GUID => {
                "SELECT MAX(guid) FROM characters"
            }
            Self::SEL_CHAR_EQUIPMENT => {
                "SELECT ci.slot, ii.itemEntry, ci.item \
                 FROM character_inventory ci \
                 JOIN item_instance ii ON ci.item = ii.guid \
                 WHERE ci.guid = ? AND ci.bag = 0"
            }
            Self::UPD_CHAR_INVENTORY_SLOT => {
                "UPDATE character_inventory SET slot = ? WHERE guid = ? AND item = ?"
            }
            Self::DEL_CHAR_INVENTORY_ITEM => {
                "DELETE FROM character_inventory WHERE guid = ? AND item = ?"
            }
            Self::SEL_CHARACTER_SKILLS => {
                "SELECT skill FROM character_skills WHERE guid = ?"
            }
            Self::SEL_CHARACTER_SPELL => {
                "SELECT spell, active, disabled FROM character_spell WHERE guid = ?"
            }
            Self::SEL_CHARACTER_ACTIONS_SPEC => {
                "SELECT button, action, type FROM character_action \
                 WHERE guid = ? AND spec = ? AND traitConfigId = ? ORDER BY button"
            }
            Self::INS_CHARACTER_ACTION => {
                "INSERT INTO character_action (guid, spec, traitConfigId, button, action, type) \
                 VALUES (?, 0, 0, ?, ?, ?)"
            }
            Self::UPD_CHAR_PLAYED_TIME => {
                "UPDATE characters SET totaltime = ?, leveltime = ? WHERE guid = ?"
            }
            Self::UPD_CHAR_MONEY => {
                "UPDATE characters SET money = ? WHERE guid = ?"
            }
            Self::SEL_MAX_ITEM_GUID => {
                "SELECT MAX(guid) FROM item_instance"
            }
            Self::INS_ITEM_INSTANCE => {
                "INSERT INTO item_instance \
                 (guid, itemEntry, owner_guid, creatorGuid, giftCreatorGuid, count, \
                  durability, enchantments, charges, flags, randomPropertiesId, \
                  randomPropertiesSeed, context) \
                 VALUES (?, ?, ?, 0, 0, ?, ?, '', '', 0, 0, 0, 0)"
            }
            Self::INS_CHAR_INVENTORY => {
                "INSERT INTO character_inventory (guid, bag, slot, item) VALUES (?, 0, ?, ?)"
            }
            Self::DEL_ITEM_INSTANCE => {
                "DELETE FROM item_instance WHERE guid = ?"
            }
            Self::INS_CHARACTER_SPELL => {
                "INSERT IGNORE INTO character_spell (guid, spell, active, disabled) VALUES (?, ?, 1, 0)"
            }
            Self::SEL_CHAR_QUEST_STATUS => {
                "SELECT quest, status, explored FROM character_queststatus WHERE guid = ?"
            }
            Self::INS_CHAR_QUEST_STATUS => {
                "INSERT INTO character_queststatus (guid, quest, status, explored, acceptTime, endTime) \
                 VALUES (?, ?, ?, 0, UNIX_TIMESTAMP(), 0) \
                 ON DUPLICATE KEY UPDATE status = ?, explored = ?"
            }
            Self::DEL_CHAR_QUEST_STATUS => {
                "DELETE FROM character_queststatus WHERE guid = ? AND quest = ?"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn char_statements_have_sql() {
        assert!(!CharStatements::SEL_ENUM.sql().is_empty());
        assert!(!CharStatements::SEL_CHECK_NAME.sql().is_empty());
        assert!(!CharStatements::SEL_SUM_CHARS.sql().is_empty());
        assert!(!CharStatements::INS_CHARACTER.sql().is_empty());
        assert!(!CharStatements::INS_CHAR_CUSTOMIZATION.sql().is_empty());
        assert!(!CharStatements::DEL_CHARACTER.sql().is_empty());
        assert!(!CharStatements::SEL_CHARACTER.sql().is_empty());
        assert!(!CharStatements::UPD_CHAR_ONLINE.sql().is_empty());
        assert!(!CharStatements::UPD_CHAR_OFFLINE.sql().is_empty());
        assert!(!CharStatements::SEL_CHAR_DEL_CHECK.sql().is_empty());
        assert!(!CharStatements::SEL_MAX_GUID.sql().is_empty());
        assert!(!CharStatements::UPD_CHAR_PLAYED_TIME.sql().is_empty());
    }

    #[test]
    fn char_sql_contains_expected_tables() {
        assert!(CharStatements::SEL_ENUM.sql().contains("characters"));
        assert!(CharStatements::INS_CHARACTER.sql().contains("characters"));
        assert!(CharStatements::INS_CHAR_CUSTOMIZATION.sql().contains("character_customizations"));
        assert!(CharStatements::DEL_CHARACTER.sql().contains("characters"));
    }

    #[test]
    fn char_sql_has_correct_placeholders() {
        // SEL_ENUM has 1 placeholder (account id)
        assert_eq!(CharStatements::SEL_ENUM.sql().matches('?').count(), 1);
        // SEL_ENUM should select equipmentCache and lastLoginBuild
        assert!(CharStatements::SEL_ENUM.sql().contains("equipmentCache"));
        assert!(CharStatements::SEL_ENUM.sql().contains("lastLoginBuild"));
        // SEL_CHECK_NAME has 1 placeholder
        assert_eq!(CharStatements::SEL_CHECK_NAME.sql().matches('?').count(), 1);
        // SEL_SUM_CHARS has 1 placeholder
        assert_eq!(CharStatements::SEL_SUM_CHARS.sql().matches('?').count(), 1);
        // INS_CHARACTER has 22 placeholders
        assert_eq!(CharStatements::INS_CHARACTER.sql().matches('?').count(), 22);
        // INS_CHAR_CUSTOMIZATION has 3 placeholders
        assert_eq!(CharStatements::INS_CHAR_CUSTOMIZATION.sql().matches('?').count(), 3);
        // DEL_CHARACTER has 1 placeholder
        assert_eq!(CharStatements::DEL_CHARACTER.sql().matches('?').count(), 1);
        // SEL_CHARACTER has 1 placeholder
        assert_eq!(CharStatements::SEL_CHARACTER.sql().matches('?').count(), 1);
        // SEL_CHAR_DEL_CHECK has 2 placeholders
        assert_eq!(CharStatements::SEL_CHAR_DEL_CHECK.sql().matches('?').count(), 2);
    }
}
