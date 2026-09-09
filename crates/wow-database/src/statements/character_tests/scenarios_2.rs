//! Character-statement C++ contrast regressions, part 2 of 4.
//!
//! Moved out of the character_tests.rs root under #652; every test is unchanged.

use super::*;

#[test]
fn world_state_value_statements_match_cpp_sql_exactly() {
    assert_eq!(
        CharStatements::SEL_WORLD_STATE_VALUES.sql(),
        "SELECT Id, Value FROM world_state_value"
    );
    assert_eq!(
        CharStatements::SEL_WORLD_STATE_VALUES
            .sql()
            .matches('?')
            .count(),
        0
    );
    assert_eq!(
        CharStatements::REP_WORLD_STATE.sql(),
        "REPLACE INTO world_state_value (Id, Value) VALUES (?, ?)"
    );
}

#[test]
fn character_maintenance_social_and_position_statements_are_pinned() {
    assert_eq!(
        CharStatements::UPD_GROUP_DIFFICULTY.sql(),
        "UPDATE `groups` SET difficulty = ? WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_GROUP_RAID_DIFFICULTY.sql(),
        "UPDATE `groups` SET raidDifficulty = ? WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_GROUP_LEGACY_RAID_DIFFICULTY.sql(),
        "UPDATE `groups` SET legacyRaidDifficulty = ? WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_INVALID_SPELL_SPELLS.sql(),
        "DELETE FROM character_spell WHERE spell = ?"
    );
    assert_eq!(
        CharStatements::UPD_DELETE_INFO.sql(),
        "UPDATE characters SET deleteInfos_Name = name, deleteInfos_Account = account, deleteDate = UNIX_TIMESTAMP(), name = '', account = 0 WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_RESTORE_DELETE_INFO.sql(),
        "UPDATE characters SET name = ?, account = ?, deleteDate = NULL, deleteInfos_Name = NULL, deleteInfos_Account = NULL WHERE deleteDate IS NOT NULL AND guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_ZONE.sql(),
        "UPDATE characters SET zone = ? WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_LEVEL.sql(),
        "UPDATE characters SET level = ?, xp = 0 WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_INVALID_ACHIEV_PROGRESS_CRITERIA.sql(),
        "DELETE FROM character_achievement_progress WHERE criteria = ?"
    );
    assert_eq!(
        CharStatements::DEL_INVALID_ACHIEV_PROGRESS_CRITERIA_GUILD.sql(),
        "DELETE FROM guild_achievement_progress WHERE criteria = ?"
    );
    assert_eq!(
        CharStatements::DEL_INVALID_ACHIEVMENT.sql(),
        "DELETE FROM character_achievement WHERE achievement = ?"
    );
    assert_eq!(
        CharStatements::DEL_INVALID_PET_SPELL.sql(),
        "DELETE FROM pet_spell WHERE spell = ?"
    );
    assert_eq!(
        CharStatements::UPD_CHAR_NAME_AT_LOGIN.sql(),
        "UPDATE characters SET name = ?, at_login = ? WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::REP_WORLD_VARIABLE.sql(),
        "REPLACE INTO world_variable (Id, Value) VALUES (?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_CHARACTER_SKILL.sql(),
        "DELETE FROM character_skills WHERE guid = ? AND skill = ?"
    );
    assert_eq!(
        CharStatements::UPD_CHARACTER_SOCIAL_FLAGS.sql(),
        "UPDATE character_social SET flags = ? WHERE guid = ? AND friend = ?"
    );
    assert_eq!(
        CharStatements::INS_CHARACTER_SOCIAL.sql(),
        "INSERT INTO character_social (guid, friend, flags) VALUES (?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_CHARACTER_SOCIAL.sql(),
        "DELETE FROM character_social WHERE guid = ? AND friend = ?"
    );
    assert_eq!(
        CharStatements::UPD_CHARACTER_SOCIAL_NOTE.sql(),
        "UPDATE character_social SET note = ? WHERE guid = ? AND friend = ?"
    );
    assert_eq!(
        CharStatements::UPD_CHARACTER_POSITION.sql(),
        "UPDATE characters SET position_x = ?, position_y = ?, position_z = ?, orientation = ?, map = ?, instance_id = ?, zone = ?, trans_x = 0, trans_y = 0, trans_z = 0, transguid = 0, taxi_path = '', cinematic = 1 WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_CHARACTER_POSITION_BY_MAPID.sql(),
        "UPDATE characters SET position_x = ?, position_y = ?, position_z = ?, orientation = ?, map = ?, zone = ?, trans_x = 0, trans_y = 0, trans_z = 0, transguid = 0, taxi_path = '', cinematic = 1 WHERE guid = ? AND map = ?"
    );
    assert_eq!(
        CharStatements::UPD_CHARACTER_POSITION_PRESERVE_TRAVEL.sql(),
        "UPDATE characters SET position_x = ?, position_y = ?, position_z = ?, orientation = ?, map = ?, instance_id = ?, zone = ? WHERE guid = ?"
    );
}

#[test]
fn character_admin_lookup_and_item_search_statements_match_cpp_sql_exactly() {
    assert_eq!(
        CharStatements::SEL_CHARACTER_AURA_FROZEN.sql(),
        "SELECT characters.name, character_aura.remainTime FROM characters LEFT JOIN character_aura ON (characters.guid = character_aura.guid) WHERE character_aura.spell = 9454"
    );
    assert_eq!(
        CharStatements::SEL_CHARACTER_ONLINE.sql(),
        "SELECT name, account, map, zone FROM characters WHERE online > 0"
    );
    assert_eq!(
        CharStatements::SEL_CHAR_DEL_INFO_BY_GUID.sql(),
        "SELECT guid, deleteInfos_Name, deleteInfos_Account, deleteDate FROM characters WHERE deleteDate IS NOT NULL AND guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHAR_DEL_INFO_BY_NAME.sql(),
        "SELECT guid, deleteInfos_Name, deleteInfos_Account, deleteDate FROM characters WHERE deleteDate IS NOT NULL AND deleteInfos_Name LIKE CONCAT('%%', ?, '%%')"
    );
    assert_eq!(
        CharStatements::SEL_CHAR_DEL_INFO.sql(),
        "SELECT guid, deleteInfos_Name, deleteInfos_Account, deleteDate FROM characters WHERE deleteDate IS NOT NULL"
    );
    assert_eq!(
        CharStatements::SEL_CHARS_BY_ACCOUNT_ID.sql(),
        "SELECT guid FROM characters WHERE account = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHAR_PINFO.sql(),
        "SELECT totaltime, level, money, account, race, class, map, zone, gender, health, playerFlags FROM characters WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_PINFO_BANS.sql(),
        "SELECT unbandate, bandate = unbandate, bannedby, banreason FROM character_banned WHERE guid = ? AND active ORDER BY bandate ASC LIMIT 1"
    );
    assert_eq!(
        CharStatements::SEL_PINFO_MAILS.sql(),
        "SELECT SUM(CASE WHEN (checked & 1) THEN 1 ELSE 0 END) AS 'readmail', COUNT(*) AS 'totalmail' FROM mail WHERE `receiver` = ?"
    );
    assert_eq!(
        CharStatements::SEL_PINFO_XP.sql(),
        "SELECT a.xp, b.guid FROM characters a LEFT JOIN guild_member b ON a.guid = b.guid WHERE a.guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHAR_HOMEBIND.sql(),
        "SELECT mapId, zoneId, posX, posY, posZ, orientation FROM character_homebind WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHAR_GUID_NAME_BY_ACC.sql(),
        "SELECT guid, name, online FROM characters WHERE account = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHAR_CUSTOMIZE_INFO.sql(),
        "SELECT name, race, class, gender, at_login FROM characters WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHAR_RACE_OR_FACTION_CHANGE_INFOS.sql(),
        "SELECT c.at_login, c.knownTitles, gm.guid FROM characters c LEFT JOIN group_member gm ON c.guid = gm.memberGuid WHERE c.guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHAR_COD_ITEM_MAIL.sql(),
        "SELECT id, messageType, mailTemplateId, sender, subject, body, money, has_items FROM mail WHERE receiver = ? AND has_items <> 0 AND cod <> 0"
    );
    assert_eq!(
        CharStatements::SEL_CHAR_SOCIAL.sql(),
        "SELECT DISTINCT guid FROM character_social WHERE friend = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHAR_OLD_CHARS.sql(),
        "SELECT guid, deleteInfos_Account FROM characters WHERE deleteDate IS NOT NULL AND deleteDate < ?"
    );
    assert_eq!(
        CharStatements::SEL_MAIL.sql(),
        "SELECT id, messageType, sender, receiver, subject, body, expire_time, deliver_time, money, cod, checked, stationery, mailTemplateId FROM mail WHERE receiver = ? ORDER BY id DESC"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_AURA_FROZEN.sql(),
        "DELETE FROM character_aura WHERE spell = 9454 AND guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHAR_INVENTORY_COUNT_ITEM.sql(),
        "SELECT COUNT(itemEntry) FROM character_inventory ci INNER JOIN item_instance ii ON ii.guid = ci.item WHERE itemEntry = ?"
    );
    assert_eq!(
        CharStatements::SEL_MAIL_COUNT_ITEM.sql(),
        "SELECT COUNT(itemEntry) FROM mail_items mi INNER JOIN item_instance ii ON ii.guid = mi.item_guid WHERE itemEntry = ?"
    );
    assert_eq!(
        CharStatements::SEL_AUCTIONHOUSE_COUNT_ITEM.sql(),
        "SELECT COUNT(*) FROM auction_items ai INNER JOIN item_instance ii ON ii.guid = ai.itemGuid WHERE ii.itemEntry = ?"
    );
    assert_eq!(
        CharStatements::SEL_GUILD_BANK_COUNT_ITEM.sql(),
        "SELECT COUNT(itemEntry) FROM guild_bank_item gbi INNER JOIN item_instance ii ON ii.guid = gbi.item_guid WHERE itemEntry = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHAR_INVENTORY_ITEM_BY_ENTRY.sql(),
        "SELECT ci.item, cb.slot AS bag, ci.slot, ci.guid, c.account, c.name FROM characters c INNER JOIN character_inventory ci ON ci.guid = c.guid INNER JOIN item_instance ii ON ii.guid = ci.item LEFT JOIN character_inventory cb ON cb.item = ci.bag WHERE ii.itemEntry = ? LIMIT ?"
    );
    assert_eq!(
        CharStatements::SEL_MAIL_ITEMS_BY_ENTRY.sql(),
        "SELECT mi.item_guid, m.sender, m.receiver, cs.account, cs.name, cr.account, cr.name FROM mail m INNER JOIN mail_items mi ON mi.mail_id = m.id INNER JOIN item_instance ii ON ii.guid = mi.item_guid INNER JOIN characters cs ON cs.guid = m.sender INNER JOIN characters cr ON cr.guid = m.receiver WHERE ii.itemEntry = ? LIMIT ?"
    );
    assert_eq!(
        CharStatements::SEL_AUCTIONHOUSE_ITEM_BY_ENTRY.sql(),
        "SELECT ai.itemGuid, c.guid, c.account, c.name FROM auctionhouse ah INNER JOIN auction_items ai ON ah.id = ai.auctionId INNER JOIN characters c ON c.guid = ah.owner INNER JOIN item_instance ii ON ii.guid = ai.itemGuid WHERE ii.itemEntry = ? LIMIT ?"
    );
    assert_eq!(
        CharStatements::SEL_GUILD_BANK_ITEM_BY_ENTRY.sql(),
        "SELECT gi.item_guid, gi.guildid, g.name FROM guild_bank_item gi INNER JOIN guild g ON g.guildid = gi.guildid INNER JOIN item_instance ii ON ii.guid = gi.item_guid WHERE ii.itemEntry = ? LIMIT ?"
    );
}

#[test]
fn character_achievement_petition_declined_and_cleanup_statements_match_cpp_sql_exactly() {
    assert_eq!(
        CharStatements::DEL_CHAR_ACHIEVEMENT.sql(),
        "DELETE FROM character_achievement WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_ACHIEVEMENT_PROGRESS.sql(),
        "DELETE FROM character_achievement_progress WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::INS_CHAR_ACHIEVEMENT.sql(),
        "INSERT INTO character_achievement (guid, achievement, date) VALUES (?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_ACHIEVEMENT_PROGRESS_BY_CRITERIA.sql(),
        "DELETE FROM character_achievement_progress WHERE guid = ? AND criteria = ?"
    );
    assert_eq!(
        CharStatements::INS_CHAR_ACHIEVEMENT_PROGRESS.sql(),
        "INSERT INTO character_achievement_progress (guid, criteria, counter, date) VALUES (?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::INS_CHAR_GIFT.sql(),
        "INSERT INTO character_gifts (guid, item_guid, entry, flags) VALUES (?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_MAIL_ITEM_BY_ID.sql(),
        "DELETE FROM mail_items WHERE mail_id = ?"
    );
    assert_eq!(
        CharStatements::INS_PETITION.sql(),
        "INSERT INTO petition (ownerguid, petitionguid, name) VALUES (?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_PETITION_BY_GUID.sql(),
        "DELETE FROM petition WHERE petitionguid = ?"
    );
    assert_eq!(
        CharStatements::DEL_PETITION_SIGNATURE_BY_GUID.sql(),
        "DELETE FROM petition_sign WHERE petitionguid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_DECLINED_NAME.sql(),
        "DELETE FROM character_declinedname WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::INS_CHAR_DECLINED_NAME.sql(),
        "INSERT INTO character_declinedname (guid, genitive, dative, accusative, instrumental, prepositional) VALUES (?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::UPD_CHAR_RACE.sql(),
        "UPDATE characters SET race = ?, extra_flags = extra_flags | ? WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_SKILL_LANGUAGES.sql(),
        "DELETE FROM character_skills WHERE skill IN (98, 113, 759, 111, 313, 109, 115, 315, 673, 137) AND guid = ?"
    );
    assert_eq!(
        CharStatements::INS_CHAR_SKILL_LANGUAGE.sql(),
        "INSERT INTO `character_skills` (guid, skill, value, max) VALUES (?, ?, 300, 300)"
    );
    assert_eq!(
        CharStatements::UPD_CHAR_TAXI_PATH.sql(),
        "UPDATE characters SET taxi_path = '' WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_CHAR_TAXIMASK.sql(),
        "UPDATE characters SET taximask = ? WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_QUESTSTATUS.sql(),
        "DELETE FROM character_queststatus WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_QUESTSTATUS_OBJECTIVES.sql(),
        "DELETE FROM character_queststatus_objectives WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA.sql(),
        "DELETE FROM character_queststatus_objectives_criteria WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA_PROGRESS.sql(),
        "DELETE FROM character_queststatus_objectives_criteria_progress WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA_PROGRESS_BY_CRITERIA.sql(),
        "DELETE FROM character_queststatus_objectives_criteria_progress WHERE guid = ? AND criteriaId = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_SOCIAL_BY_GUID.sql(),
        "DELETE FROM character_social WHERE guid = ?"
    );
}

#[test]
fn character_faction_change_cooldown_delete_and_action_statements_match_cpp_sql_exactly() {
    assert_eq!(
        CharStatements::DEL_CHAR_SOCIAL_BY_FRIEND.sql(),
        "DELETE FROM character_social WHERE friend = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_ACHIEVEMENT_BY_ACHIEVEMENT.sql(),
        "DELETE FROM character_achievement WHERE achievement = ? AND guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_CHAR_ACHIEVEMENT.sql(),
        "UPDATE character_achievement SET achievement = ? where achievement = ? AND guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_CHAR_INVENTORY_FACTION_CHANGE.sql(),
        "UPDATE item_instance ii, character_inventory ci SET ii.itemEntry = ? WHERE ii.itemEntry = ? AND ci.guid = ? AND ci.item = ii.guid"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_SPELL_BY_SPELL.sql(),
        "DELETE FROM character_spell WHERE spell = ? AND guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_CHAR_SPELL_FACTION_CHANGE.sql(),
        "UPDATE character_spell SET spell = ? where spell = ? AND guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHAR_REP_BY_FACTION.sql(),
        "SELECT standing FROM character_reputation WHERE faction = ? AND guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_REP_BY_FACTION.sql(),
        "DELETE FROM character_reputation WHERE faction = ? AND guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_CHAR_REP_FACTION_CHANGE.sql(),
        "UPDATE character_reputation SET faction = ?, standing = ? WHERE faction = ? AND guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_CHAR_TITLES_FACTION_CHANGE.sql(),
        "UPDATE characters SET knownTitles = ? WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::RES_CHAR_TITLES_FACTION_CHANGE.sql(),
        "UPDATE characters SET chosenTitle = 0 WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_SPELL_COOLDOWNS.sql(),
        "DELETE FROM character_spell_cooldown WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::INS_CHAR_SPELL_COOLDOWN.sql(),
        "INSERT INTO character_spell_cooldown (guid, spell, item, time, categoryId, categoryEnd) VALUES (?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_SPELL_CHARGES.sql(),
        "DELETE FROM character_spell_charges WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::INS_CHAR_SPELL_CHARGES.sql(),
        "INSERT INTO character_spell_charges (guid, categoryId, rechargeStart, rechargeEnd) VALUES (?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_CHARACTER.sql(),
        "DELETE FROM characters WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_ACTION.sql(),
        "DELETE FROM character_action WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_AURA.sql(),
        "DELETE FROM character_aura WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_AURA_EFFECT.sql(),
        "DELETE FROM character_aura_effect WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_GIFT.sql(),
        "DELETE FROM character_gifts WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_INVENTORY.sql(),
        "DELETE FROM character_inventory WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_QUESTSTATUS_REWARDED.sql(),
        "DELETE FROM character_queststatus_rewarded WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_REPUTATION.sql(),
        "DELETE FROM character_reputation WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_SPELL.sql(),
        "DELETE FROM character_spell WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_MAIL.sql(),
        "DELETE FROM mail WHERE receiver = ?"
    );
    assert_eq!(
        CharStatements::DEL_MAIL_ITEMS.sql(),
        "DELETE FROM mail_items WHERE receiver = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_EQUIPMENTSETS.sql(),
        "DELETE FROM character_equipmentsets WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_TRANSMOG_OUTFITS.sql(),
        "DELETE FROM character_transmog_outfits WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_GUILD_EVENTLOG_BY_PLAYER.sql(),
        "DELETE FROM guild_eventlog WHERE PlayerGuid1 = ? OR PlayerGuid2 = ?"
    );
    assert_eq!(
        CharStatements::DEL_GUILD_BANK_EVENTLOG_BY_PLAYER.sql(),
        "DELETE FROM guild_bank_eventlog WHERE PlayerGuid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_GLYPHS.sql(),
        "DELETE FROM character_glyphs WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_TALENT.sql(),
        "DELETE FROM character_talent WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_SKILLS.sql(),
        "DELETE FROM character_skills WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_CHAR_MONEY.sql(),
        "UPDATE characters SET money = ? WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_CHAR_PLAYER_FLAGS.sql(),
        "UPDATE characters SET playerFlags = ? WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_CHAR_HEALTH.sql(),
        "UPDATE characters SET health = ? WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_CHAR_POWERS.sql(),
        "UPDATE characters SET power1 = ?, power2 = ?, power3 = ?, power4 = ?, power5 = ?, power6 = ?, power7 = ?, power8 = ?, power9 = ?, power10 = ? WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_CHAR_REST_STATE.sql(),
        "UPDATE characters SET restState = ?, playerFlags = ?, rest_bonus = ?, logout_time = ?, is_logout_resting = ? WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_CHAR_ONLINE_REST_STATE.sql(),
        "UPDATE characters SET restState = ?, playerFlags = ?, rest_bonus = ? WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_CHAR_DIFFICULTIES.sql(),
        "UPDATE characters SET dungeonDifficulty = ?, raidDifficulty = ?, legacyRaidDifficulty = ? WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::INS_CHAR_ACTION.sql(),
        "INSERT INTO character_action (guid, spec, traitConfigId, button, action, type) VALUES (?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::INS_CHAR_ACTION.sql().matches('?').count(),
        6
    );
    assert_eq!(
        CharStatements::UPD_CHAR_ACTION.sql(),
        "UPDATE character_action SET action = ?, type = ? WHERE guid = ? AND button = ? AND spec = ? AND traitConfigId = ?"
    );
    assert_eq!(
        CharStatements::UPD_CHAR_ACTION.sql().matches('?').count(),
        6
    );
    assert_eq!(
        CharStatements::DEL_CHAR_ACTION_BY_BUTTON_SPEC.sql(),
        "DELETE FROM character_action WHERE guid = ? and button = ? and spec = ? AND traitConfigId = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_ACTION_BY_SPEC.sql(),
        "DELETE FROM character_action WHERE guid = ? AND spec = ? AND traitConfigId = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_ACTION_BY_TRAIT_CONFIG.sql(),
        "DELETE FROM character_action WHERE guid = ? AND traitConfigId = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_INVENTORY_BY_ITEM.sql(),
        "DELETE FROM character_inventory WHERE item = ?"
    );
    assert!(
        CharStatements::DEL_CHAR_ACHIEVEMENTS
            .sql()
            .starts_with("DELETE FROM character_achievement WHERE guid = ? AND achievement NOT IN")
    );
    assert_eq!(
        CharStatements::DEL_CHAR_ACHIEVEMENTS
            .sql()
            .matches('?')
            .count(),
        1
    );
}

#[test]
fn character_quest_skill_spell_stats_trait_save_statements_match_cpp_sql_exactly() {
    assert_eq!(
        CharStatements::DEL_CHAR_INVENTORY_BY_BAG_SLOT.sql(),
        "DELETE FROM character_inventory WHERE bag = ? AND slot = ? AND guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_MAIL.sql(),
        "UPDATE mail SET has_items = ?, expire_time = ?, deliver_time = ?, money = ?, cod = ?, checked = ? WHERE id = ?"
    );
    assert_eq!(
        CharStatements::REP_CHAR_QUESTSTATUS.sql(),
        "REPLACE INTO character_queststatus (guid, quest, status, explored, acceptTime, endTime) VALUES (?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_QUESTSTATUS_BY_QUEST.sql(),
        "DELETE FROM character_queststatus WHERE guid = ? AND quest = ?"
    );
    assert_eq!(
        CharStatements::REP_CHAR_QUEST_STATUS_OBJECTIVES.sql(),
        "REPLACE INTO character_queststatus_objectives (guid, quest, objective, data) VALUES (?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_QUEST_STATUS_OBJECTIVES_BY_QUEST.sql(),
        "DELETE FROM character_queststatus_objectives WHERE guid = ? AND quest = ?"
    );
    assert_eq!(
        CharStatements::INS_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA.sql(),
        "INSERT INTO character_queststatus_objectives_criteria (guid, questObjectiveId) VALUES (?, ?)"
    );
    assert_eq!(
        CharStatements::SEL_CHARACTER_QUESTSTATUS.sql(),
        CharStatements::SEL_CHAR_QUEST_STATUS.sql()
    );
    assert_eq!(
        CharStatements::SEL_CHARACTER_QUESTSTATUS_OBJECTIVES.sql(),
        CharStatements::SEL_CHAR_QUEST_STATUS_OBJECTIVES.sql()
    );
    assert_eq!(
        CharStatements::DEL_CHAR_QUESTSTATUS_OBJECTIVES_BY_QUEST.sql(),
        CharStatements::DEL_CHAR_QUEST_STATUS_OBJECTIVES_BY_QUEST.sql()
    );
    assert_eq!(
        CharStatements::REP_CHAR_QUESTSTATUS_OBJECTIVES.sql(),
        CharStatements::REP_CHAR_QUEST_STATUS_OBJECTIVES.sql()
    );
    assert_eq!(
        CharStatements::INS_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA_PROGRESS.sql(),
        "INSERT INTO character_queststatus_objectives_criteria_progress (guid, criteriaId, counter, date) VALUES (?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::INS_CHAR_QUESTSTATUS_REWARDED.sql(),
        "INSERT IGNORE INTO character_queststatus_rewarded (guid, quest, active) VALUES (?, ?, 1)"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_QUESTSTATUS_REWARDED_BY_QUEST.sql(),
        "DELETE FROM character_queststatus_rewarded WHERE guid = ? AND quest = ?"
    );
    assert_eq!(
        CharStatements::UPD_CHAR_QUESTSTATUS_REWARDED_FACTION_CHANGE.sql(),
        "UPDATE character_queststatus_rewarded SET quest = ? WHERE quest = ? AND guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_CHAR_QUESTSTATUS_REWARDED_ACTIVE.sql(),
        "UPDATE character_queststatus_rewarded SET active = 1 WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_CHAR_QUESTSTATUS_REWARDED_ACTIVE_BY_QUEST.sql(),
        "UPDATE character_queststatus_rewarded SET active = 0 WHERE quest = ? AND guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_INVALID_QUEST_PROGRESS_CRITERIA.sql(),
        "DELETE FROM character_queststatus_objectives_criteria WHERE questObjectiveId = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_SKILL_BY_SKILL.sql(),
        "DELETE FROM character_skills WHERE guid = ? AND skill = ?"
    );
    assert_eq!(
        CharStatements::INS_CHAR_SKILLS.sql(),
        "INSERT INTO character_skills (guid, skill, value, max, professionSlot) VALUES (?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::UPD_CHAR_SKILLS.sql(),
        "UPDATE character_skills SET value = ?, max = ?, professionSlot = ? WHERE guid = ? AND skill = ?"
    );
    assert_eq!(
        CharStatements::INS_CHAR_SPELL.sql(),
        "INSERT INTO character_spell (guid, spell, active, disabled) VALUES (?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::UPSERT_CHAR_SPELL_LEARN_FALLBACK.sql(),
        "INSERT INTO character_spell (guid, spell, active, disabled) VALUES (?, ?, ?, ?) ON DUPLICATE KEY UPDATE active = IF(character_spell.disabled, character_spell.active, VALUES(active)), disabled = VALUES(disabled)"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_SPELL_FAVORITE.sql(),
        "DELETE FROM character_spell_favorite WHERE guid = ? AND spell = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_SPELL_FAVORITE_BY_CHAR.sql(),
        "DELETE FROM character_spell_favorite WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::INS_CHAR_SPELL_FAVORITE.sql(),
        "INSERT INTO character_spell_favorite (guid, spell) VALUES (?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_STATS.sql(),
        "DELETE FROM character_stats WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::INS_CHAR_STATS.sql(),
        "INSERT INTO character_stats (guid, maxhealth, maxpower1, maxpower2, maxpower3, maxpower4, maxpower5, maxpower6, maxpower7, maxpower8, maxpower9, maxpower10, strength, agility, stamina, intellect, armor, resHoly, resFire, resNature, resFrost, resShadow, resArcane, blockPct, dodgePct, parryPct, critPct, rangedCritPct, spellCritPct, attackPower, rangedAttackPower, spellPower, resilience, mastery, versatility) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::INS_CHAR_STATS.sql().matches('?').count(),
        35
    );
    assert_eq!(
        CharStatements::DEL_PETITION_BY_OWNER.sql(),
        "DELETE FROM petition WHERE ownerguid = ?"
    );
    assert_eq!(
        CharStatements::DEL_PETITION_SIGNATURE_BY_OWNER.sql(),
        "DELETE FROM petition_sign WHERE ownerguid = ?"
    );
    assert_eq!(
        CharStatements::INS_CHAR_GLYPHS.sql(),
        "INSERT INTO character_glyphs (guid, talentGroup, glyphSlot, glyphId) VALUES(?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::INS_CHAR_TALENT.sql(),
        "INSERT INTO character_talent (guid, talentId, talentRank, talentGroup) VALUES (?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::UPD_CHAR_LIST_SLOT.sql(),
        "UPDATE characters SET slot = ? WHERE guid = ? AND account = ?"
    );
    assert_eq!(
        CharStatements::INS_CHAR_FISHINGSTEPS.sql(),
        "INSERT INTO character_fishingsteps (guid, fishingSteps) VALUES (?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_FISHINGSTEPS.sql(),
        "DELETE FROM character_fishingsteps WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHAR_TRAIT_ENTRIES.sql(),
        "SELECT traitConfigId, traitNodeId, traitNodeEntryId, `rank`, grantedRanks FROM character_trait_entry WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::INS_CHAR_TRAIT_ENTRIES.sql(),
        "INSERT INTO character_trait_entry (guid, traitConfigId, traitNodeId, traitNodeEntryId, `rank`, grantedRanks) VALUES (?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_TRAIT_ENTRIES.sql(),
        "DELETE FROM character_trait_entry WHERE guid = ? AND traitConfigId = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_TRAIT_ENTRIES_BY_CHAR.sql(),
        "DELETE FROM character_trait_entry WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHAR_TRAIT_CONFIGS.sql(),
        "SELECT traitConfigId, type, chrSpecializationId, combatConfigFlags, localIdentifier, skillLineId, traitSystemId, `name` FROM character_trait_config WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::INS_CHAR_TRAIT_CONFIGS.sql(),
        "INSERT INTO character_trait_config (guid, traitConfigId, type, chrSpecializationId, combatConfigFlags, localIdentifier, skillLineId, traitSystemId, `name`) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::INS_CHAR_TRAIT_CONFIGS
            .sql()
            .matches('?')
            .count(),
        9
    );
    assert_eq!(
        CharStatements::DEL_CHAR_TRAIT_CONFIGS.sql(),
        "DELETE FROM character_trait_config WHERE guid = ? AND traitConfigId = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_TRAIT_CONFIGS_BY_CHAR.sql(),
        "DELETE FROM character_trait_config WHERE guid = ?"
    );
}
