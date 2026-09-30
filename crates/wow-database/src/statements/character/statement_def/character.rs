//! Character identity and account SQL values.
pub(super) const DEL_EXPIRED_BANS: &str = {
    "UPDATE character_banned SET active = 0 WHERE unbandate <= UNIX_TIMESTAMP() AND unbandate <> bandate"
};
pub(super) const SEL_ENUM: &str = {
    "SELECT c.guid, c.name, c.race, c.class, c.gender, c.level, c.zone, c.map, \
                 c.position_x, c.position_y, c.position_z, gm.guildid, c.playerFlags, \
                 c.at_login, cp.entry, cp.modelid, cp.level, c.equipmentCache, cb.guid, \
                 c.slot, c.logout_time, c.activeTalentGroup, c.lastLoginBuild, \
                 c.personalTabardEmblemStyle, c.personalTabardEmblemColor, \
                 c.personalTabardBorderStyle, c.personalTabardBorderColor, \
                 c.personalTabardBackgroundColor \
                 FROM characters AS c LEFT JOIN character_pet AS cp ON c.summonedPetNumber = cp.id \
                 LEFT JOIN guild_member AS gm ON c.guid = gm.guid \
                 LEFT JOIN character_banned AS cb ON c.guid = cb.guid AND cb.active = 1 \
                 WHERE c.account = ? AND c.deleteInfos_Name IS NULL"
};
pub(super) const SEL_ENUM_DECLINED_NAME: &str = {
    "SELECT c.guid, c.name, c.race, c.class, c.gender, c.level, c.zone, c.map, \
                 c.position_x, c.position_y, c.position_z, gm.guildid, c.playerFlags, \
                 c.at_login, cp.entry, cp.modelid, cp.level, c.equipmentCache, cb.guid, \
                 c.slot, c.logout_time, c.activeTalentGroup, c.lastLoginBuild, \
                 c.personalTabardEmblemStyle, c.personalTabardEmblemColor, \
                 c.personalTabardBorderStyle, c.personalTabardBorderColor, \
                 c.personalTabardBackgroundColor, cd.genitive \
                 FROM characters AS c LEFT JOIN character_pet AS cp ON c.summonedPetNumber = cp.id \
                 LEFT JOIN guild_member AS gm ON c.guid = gm.guid \
                 LEFT JOIN character_banned AS cb ON c.guid = cb.guid AND cb.active = 1 \
                 LEFT JOIN character_declinedname AS cd ON c.guid = cd.guid \
                 WHERE c.account = ? AND c.deleteInfos_Name IS NULL"
};
pub(super) const SEL_ENUM_CUSTOMIZATIONS: &str = {
    "SELECT cc.guid, cc.chrCustomizationOptionID, cc.chrCustomizationChoiceID FROM character_customizations cc \
                 LEFT JOIN characters c ON cc.guid = c.guid WHERE c.account = ? AND c.deleteInfos_Name IS NULL ORDER BY cc.guid, cc.chrCustomizationOptionID"
};
pub(super) const SEL_UNDELETE_ENUM: &str = {
    "SELECT c.guid, c.deleteInfos_Name, c.race, c.class, c.gender, c.level, c.zone, c.map, \
                 c.position_x, c.position_y, c.position_z, gm.guildid, c.playerFlags, \
                 c.at_login, cp.entry, cp.modelid, cp.level, c.equipmentCache, cb.guid, \
                 c.slot, c.logout_time, c.activeTalentGroup, c.lastLoginBuild, \
                 c.personalTabardEmblemStyle, c.personalTabardEmblemColor, \
                 c.personalTabardBorderStyle, c.personalTabardBorderColor, \
                 c.personalTabardBackgroundColor \
                 FROM characters AS c LEFT JOIN character_pet AS cp ON c.summonedPetNumber = cp.id \
                 LEFT JOIN guild_member AS gm ON c.guid = gm.guid \
                 LEFT JOIN character_banned AS cb ON c.guid = cb.guid AND cb.active = 1 \
                 WHERE c.deleteInfos_Account = ? AND c.deleteInfos_Name IS NOT NULL"
};
pub(super) const SEL_UNDELETE_ENUM_DECLINED_NAME: &str = {
    "SELECT c.guid, c.deleteInfos_Name, c.race, c.class, c.gender, c.level, c.zone, c.map, \
                 c.position_x, c.position_y, c.position_z, gm.guildid, c.playerFlags, \
                 c.at_login, cp.entry, cp.modelid, cp.level, c.equipmentCache, cb.guid, \
                 c.slot, c.logout_time, c.activeTalentGroup, c.lastLoginBuild, \
                 c.personalTabardEmblemStyle, c.personalTabardEmblemColor, \
                 c.personalTabardBorderStyle, c.personalTabardBorderColor, \
                 c.personalTabardBackgroundColor, cd.genitive \
                 FROM characters AS c LEFT JOIN character_pet AS cp ON c.summonedPetNumber = cp.id \
                 LEFT JOIN guild_member AS gm ON c.guid = gm.guid \
                 LEFT JOIN character_banned AS cb ON c.guid = cb.guid AND cb.active = 1 \
                 LEFT JOIN character_declinedname AS cd ON c.guid = cd.guid \
                 WHERE c.deleteInfos_Account = ? AND c.deleteInfos_Name IS NOT NULL"
};
pub(super) const SEL_UNDELETE_ENUM_CUSTOMIZATIONS: &str = {
    "SELECT cc.guid, cc.chrCustomizationOptionID, cc.chrCustomizationChoiceID FROM character_customizations cc \
                 LEFT JOIN characters c ON cc.guid = c.guid WHERE c.deleteInfos_Account = ? AND c.deleteInfos_Name IS NOT NULL ORDER BY cc.guid, cc.chrCustomizationOptionID"
};
pub(super) const SEL_CHECK_NAME: &str = "SELECT 1 FROM characters WHERE name = ?";
pub(super) const SEL_RESERVED_NAMES: &str = "SELECT name FROM reserved_name";
pub(super) const SEL_CHECK_GUID: &str = "SELECT 1 FROM characters WHERE guid = ?";
pub(super) const SEL_SUM_CHARS: &str =
    { "SELECT COUNT(guid) FROM characters WHERE account = ? AND deleteDate IS NULL" };
pub(super) const SEL_CHAR_CREATE_INFO: &str =
    { "SELECT level, race, class FROM characters WHERE account = ? LIMIT 0, ?" };
pub(super) const INS_CHARACTER_BAN: &str = {
    "INSERT INTO character_banned (guid, bandate, unbandate, bannedby, banreason, active) VALUES (?, UNIX_TIMESTAMP(), UNIX_TIMESTAMP()+?, ?, ?, 1)"
};
pub(super) const UPD_CHARACTER_BAN: &str =
    { "UPDATE character_banned SET active = 0 WHERE guid = ? AND active != 0" };
pub(super) const DEL_CHARACTER_BAN: &str = {
    "DELETE cb FROM character_banned cb INNER JOIN characters c ON c.guid = cb.guid WHERE c.account = ?"
};
pub(super) const SEL_BANINFO: &str = {
    "SELECT bandate, unbandate-bandate, active, unbandate, banreason, bannedby FROM character_banned WHERE guid = ? ORDER BY bandate ASC"
};
pub(super) const SEL_GUID_BY_NAME_FILTER: &str =
    { "SELECT guid, name FROM characters WHERE name LIKE CONCAT('%%', ?, '%%')" };
pub(super) const SEL_BANINFO_LIST: &str = {
    "SELECT bandate, unbandate, bannedby, banreason FROM character_banned WHERE guid = ? ORDER BY unbandate"
};
pub(super) const SEL_BANNED_NAME: &str = {
    "SELECT characters.name FROM characters, character_banned WHERE character_banned.guid = ? AND character_banned.guid = characters.guid"
};
pub(super) const SEL_FREE_NAME: &str = {
    "SELECT name, at_login FROM characters WHERE guid = ? AND NOT EXISTS (SELECT NULL FROM characters WHERE name = ?)"
};
pub(super) const INS_CHARACTER: &str = {
    "INSERT INTO characters (guid, account, name, race, class, gender, level, xp, money, inventorySlots, bankSlots, restState, playerFlags, playerFlagsEx, map, instance_id, dungeonDifficulty, raidDifficulty, legacyRaidDifficulty, position_x, position_y, position_z, orientation, trans_x, trans_y, trans_z, trans_o, transguid, taximask, createTime, createMode, cinematic, totaltime, leveltime, rest_bonus, logout_time, is_logout_resting, resettalents_cost, resettalents_time, activeTalentGroup, bonusTalentGroups,extra_flags, summonedPetNumber, at_login, death_expire_time, taxi_path, totalKills, todayKills, yesterdayKills, chosenTitle, watchedFaction, drunk, health, power1, power2, power3, power4, power5, power6, power7, power8, power9, power10, latency, lootSpecId, exploredZones, equipmentCache, knownTitles, actionBars, lastLoginBuild) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"
};
pub(super) const UPD_CHARACTER: &str = {
    "UPDATE characters SET name=?,race=?,class=?,gender=?,level=?,xp=?,money=?,inventorySlots=?,bankSlots=?,restState=?,playerFlags=?,playerFlagsEx=?,map=?,instance_id=?,dungeonDifficulty=?,raidDifficulty=?,legacyRaidDifficulty=?,position_x=?,position_y=?,position_z=?,orientation=?,trans_x=?,trans_y=?,trans_z=?,trans_o=?,transguid=?,taximask=?,cinematic=?,totaltime=?,leveltime=?,rest_bonus=?,logout_time=?,is_logout_resting=?,resettalents_cost=?,resettalents_time=?,numRespecs=?,activeTalentGroup=?,bonusTalentGroups=?,extra_flags=?,summonedPetNumber=?,at_login=?,zone=?,death_expire_time=?,taxi_path=?,totalKills=?,todayKills=?,yesterdayKills=?,chosenTitle=?,watchedFaction=?,drunk=?,health=?,power1=?,power2=?,power3=?,power4=?,power5=?,power6=?,power7=?,power8=?,power9=?,power10=?,latency=?,lootSpecId=?,exploredZones=?,equipmentCache=?,knownTitles=?,actionBars=?,online=?,honor=?,honorLevel=?,honorRestState=?,honorRestBonus=?,lastLoginBuild=? WHERE guid=?"
};
pub(super) const INS_CHAR_CUSTOMIZATION: &str = {
    "INSERT INTO character_customizations (guid, chrCustomizationOptionID, chrCustomizationChoiceID) VALUES (?, ?, ?)"
};
pub(super) const INS_CHARACTER_CUSTOMIZATION: &str = {
    "INSERT INTO character_customizations (guid, chrCustomizationOptionID, chrCustomizationChoiceID) VALUES (?, ?, ?)"
};
pub(super) const UPD_ADD_AT_LOGIN_FLAG: &str =
    { "UPDATE characters SET at_login = at_login | ? WHERE guid = ?" };
pub(super) const UPD_REM_AT_LOGIN_FLAG: &str =
    { "UPDATE characters set at_login = at_login & ~ ? WHERE guid = ?" };
pub(super) const UPD_ALL_AT_LOGIN_FLAGS: &str = "UPDATE characters SET at_login = at_login | ?";
pub(super) const UPD_ACCOUNT_ONLINE: &str = "UPDATE characters SET online = 0 WHERE account = ?";
pub(super) const DEL_CHARACTER_CUSTOMIZATIONS: &str =
    { "DELETE FROM character_customizations WHERE guid = ?" };
pub(super) const DEL_CHARACTER: &str = "DELETE FROM characters WHERE guid = ?";
pub(super) const SEL_CHARACTER: &str = {
    "SELECT c.guid, account, name, race, class, gender, level, xp, money, inventorySlots, \
                 bankSlots, restState, playerFlags, playerFlagsEx, position_x, position_y, position_z, \
                 map, orientation, taximask, createTime, createMode, cinematic, totaltime, leveltime, \
                 rest_bonus, logout_time, is_logout_resting, resettalents_cost, resettalents_time, \
                 activeTalentGroup, bonusTalentGroups, trans_x, trans_y, trans_z, trans_o, transguid, \
                 extra_flags, summonedPetNumber, at_login, zone, online, death_expire_time, taxi_path, \
                 dungeonDifficulty, totalKills, todayKills, yesterdayKills, chosenTitle, watchedFaction, \
                 drunk, health, power1, power2, power3, power4, power5, power6, power7, power8, power9, \
                 power10, instance_id, lootSpecId, exploredZones, knownTitles, actionBars, raidDifficulty, \
                 legacyRaidDifficulty, fishingSteps, honor, honorLevel, honorRestState, honorRestBonus, \
                 numRespecs, personalTabardEmblemStyle, personalTabardEmblemColor, \
                 personalTabardBorderStyle, personalTabardBorderColor, personalTabardBackgroundColor \
                 FROM characters c LEFT JOIN character_fishingsteps cfs ON c.guid = cfs.guid WHERE c.guid = ?"
};
pub(super) const SEL_CHARACTER_IDENTITY_CACHE: &str =
    { "SELECT guid, name, account, race, gender, class, level, deleteDate FROM characters" };
pub(super) const SEL_CHARACTER_CUSTOMIZATIONS: &str = {
    "SELECT chrCustomizationOptionID, chrCustomizationChoiceID FROM character_customizations WHERE guid = ? ORDER BY chrCustomizationOptionID"
};
pub(super) const UPD_CHAR_ONLINE: &str = "UPDATE characters SET online = 1 WHERE guid = ?";
pub(super) const UPD_CHAR_OFFLINE: &str = "UPDATE characters SET online = 0 WHERE guid = ?";
pub(super) const SEL_CHAR_DEL_CHECK: &str =
    { "SELECT guid, account FROM characters WHERE guid = ? AND account = ?" };
pub(super) const SEL_MAX_GUID: &str = "SELECT MAX(guid) FROM characters";
pub(super) const SEL_CHARACTER_DECLINEDNAMES: &str = {
    "SELECT genitive, dative, accusative, instrumental, prepositional FROM character_declinedname WHERE guid = ?"
};
pub(super) const SEL_CHARACTER_BANNED: &str =
    { "SELECT guid FROM character_banned WHERE guid = ? AND active = 1" };
pub(super) const UPD_DELETE_INFO: &str = {
    "UPDATE characters SET deleteInfos_Name = name, deleteInfos_Account = account, deleteDate = UNIX_TIMESTAMP(), name = '', account = 0 WHERE guid = ?"
};
pub(super) const UPD_RESTORE_DELETE_INFO: &str = {
    "UPDATE characters SET name = ?, account = ?, deleteDate = NULL, deleteInfos_Name = NULL, deleteInfos_Account = NULL WHERE deleteDate IS NOT NULL AND guid = ?"
};
pub(super) const UPD_CHAR_NAME_AT_LOGIN: &str =
    { "UPDATE characters SET name = ?, at_login = ? WHERE guid = ?" };
pub(super) const SEL_CHARACTER_ONLINE: &str =
    { "SELECT name, account, map, zone FROM characters WHERE online > 0" };
pub(super) const SEL_CHAR_DEL_INFO_BY_GUID: &str = {
    "SELECT guid, deleteInfos_Name, deleteInfos_Account, deleteDate FROM characters WHERE deleteDate IS NOT NULL AND guid = ?"
};
pub(super) const SEL_CHAR_DEL_INFO_BY_NAME: &str = {
    "SELECT guid, deleteInfos_Name, deleteInfos_Account, deleteDate FROM characters WHERE deleteDate IS NOT NULL AND deleteInfos_Name LIKE CONCAT('%%', ?, '%%')"
};
pub(super) const SEL_CHAR_DEL_INFO: &str = {
    "SELECT guid, deleteInfos_Name, deleteInfos_Account, deleteDate FROM characters WHERE deleteDate IS NOT NULL"
};
pub(super) const SEL_CHARS_BY_ACCOUNT_ID: &str = "SELECT guid FROM characters WHERE account = ?";
pub(super) const SEL_CHAR_PINFO: &str = {
    "SELECT totaltime, level, money, account, race, class, map, zone, gender, health, playerFlags FROM characters WHERE guid = ?"
};
pub(super) const SEL_PINFO_BANS: &str = {
    "SELECT unbandate, bandate = unbandate, bannedby, banreason FROM character_banned WHERE guid = ? AND active ORDER BY bandate ASC LIMIT 1"
};
pub(super) const SEL_CHAR_GUID_NAME_BY_ACC: &str =
    { "SELECT guid, name, online FROM characters WHERE account = ?" };
pub(super) const SEL_CHAR_CUSTOMIZE_INFO: &str =
    { "SELECT name, race, class, gender, at_login FROM characters WHERE guid = ?" };
pub(super) const SEL_CHAR_RACE_OR_FACTION_CHANGE_INFOS: &str = {
    "SELECT c.at_login, c.knownTitles, gm.guid FROM characters c LEFT JOIN group_member gm ON c.guid = gm.memberGuid WHERE c.guid = ?"
};
pub(super) const SEL_CHAR_OLD_CHARS: &str = {
    "SELECT guid, deleteInfos_Account FROM characters WHERE deleteDate IS NOT NULL AND deleteDate < ?"
};
pub(super) const DEL_CHAR_DECLINED_NAME: &str = "DELETE FROM character_declinedname WHERE guid = ?";
pub(super) const INS_CHAR_DECLINED_NAME: &str = {
    "INSERT INTO character_declinedname (guid, genitive, dative, accusative, instrumental, prepositional) VALUES (?, ?, ?, ?, ?, ?)"
};
pub(super) const UPD_CHAR_RACE: &str =
    { "UPDATE characters SET race = ?, extra_flags = extra_flags | ? WHERE guid = ?" };
pub(super) const UPD_CHAR_LIST_SLOT: &str =
    { "UPDATE characters SET slot = ? WHERE guid = ? AND account = ?" };
pub(super) const SEL_CHAR_CUF_PROFILES: &str = {
    "SELECT id, name, frameHeight, frameWidth, sortBy, healthText, boolOptions, topPoint, bottomPoint, leftPoint, topOffset, bottomOffset, leftOffset FROM character_cuf_profiles WHERE guid = ?"
};
pub(super) const REP_CHAR_CUF_PROFILES: &str = {
    "REPLACE INTO character_cuf_profiles (guid, id, name, frameHeight, frameWidth, sortBy, healthText, boolOptions, topPoint, bottomPoint, leftPoint, topOffset, bottomOffset, leftOffset) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
};
pub(super) const DEL_CHAR_CUF_PROFILES_BY_ID: &str =
    { "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?" };
pub(super) const DEL_CHAR_CUF_PROFILES: &str = "DELETE FROM character_cuf_profiles WHERE guid = ?";
pub(super) const UPD_CHAR_PLAYER_FLAGS: &str =
    "UPDATE characters SET playerFlags = ? WHERE guid = ?";
pub(super) const SEL_ACCOUNT_BY_NAME: &str = "SELECT account FROM characters WHERE name = ?";
pub(super) const UPD_ACCOUNT_BY_GUID: &str = "UPDATE characters SET account = ? WHERE guid = ?";
pub(super) const SEL_CHARACTER_COUNT: &str =
    { "SELECT account, COUNT(guid) FROM characters WHERE account = ? GROUP BY account" };
pub(super) const UPD_NAME_BY_GUID: &str = "UPDATE characters SET name = ? WHERE guid = ?";
pub(super) const SEL_ACCOUNT_DATA: &str =
    { "SELECT type, time, data FROM account_data WHERE accountId = ?" };
pub(super) const REP_ACCOUNT_DATA: &str =
    { "REPLACE INTO account_data (accountId, type, time, data) VALUES (?, ?, ?, ?)" };
pub(super) const DEL_ACCOUNT_DATA: &str = "DELETE FROM account_data WHERE accountId = ?";
pub(super) const SEL_PLAYER_ACCOUNT_DATA: &str =
    { "SELECT type, time, data FROM character_account_data WHERE guid = ?" };
pub(super) const REP_PLAYER_ACCOUNT_DATA: &str =
    { "REPLACE INTO character_account_data(guid, type, time, data) VALUES (?, ?, ?, ?)" };
pub(super) const DEL_PLAYER_ACCOUNT_DATA: &str =
    "DELETE FROM character_account_data WHERE guid = ?";
pub(super) const SEL_TUTORIALS: &str = {
    "SELECT tut0, tut1, tut2, tut3, tut4, tut5, tut6, tut7 FROM account_tutorial WHERE accountId = ?"
};
pub(super) const INS_TUTORIALS: &str = {
    "INSERT INTO account_tutorial(tut0, tut1, tut2, tut3, tut4, tut5, tut6, tut7, accountId) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
};
pub(super) const UPD_TUTORIALS: &str = {
    "UPDATE account_tutorial SET tut0 = ?, tut1 = ?, tut2 = ?, tut3 = ?, tut4 = ?, tut5 = ?, tut6 = ?, tut7 = ? WHERE accountId = ?"
};
pub(super) const DEL_TUTORIALS: &str = "DELETE FROM account_tutorial WHERE accountId = ?";
