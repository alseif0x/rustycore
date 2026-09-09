//! Character-statement C++ contrast regressions, part 1 of 4.
//!
//! Moved out of the character_tests.rs root under #652; every test is unchanged.

use super::*;

#[test]
fn generated_cpp_statements_cover_character_database() {
    let statements = cpp_character_sql();
    assert_eq!(statements.len(), 523);

    for cpp_sql in statements {
        let sql: &'static str = Box::leak(cpp_sql.into_boxed_str());
        assert_eq!(CharStatements::cpp("CHAR_TEST", sql).sql(), sql);
        assert!(!sql.is_empty());
    }
}

#[test]
fn respawn_startup_load_statement_reads_all_rows_without_placeholders() {
    let sql = CharStatements::SEL_ALL_RESPAWNS.sql();
    assert_eq!(
        sql,
        "SELECT type, spawnId, respawnTime, mapId, instanceId FROM respawn"
    );
    assert_eq!(sql.matches('?').count(), 0);
}

#[test]
fn group_type_update_statement_matches_cpp_exactly() {
    assert_eq!(
        CharStatements::UPD_GROUP_TYPE.sql(),
        "UPDATE `groups` SET groupType = ? WHERE guid = ?"
    );
    assert_eq!(CharStatements::UPD_GROUP_TYPE.sql().matches('?').count(), 2);
}

#[test]
fn group_member_insert_statement_matches_cpp_exactly() {
    assert_eq!(
        CharStatements::INS_GROUP_MEMBER.sql(),
        "INSERT INTO group_member (guid, memberGuid, memberFlags, subgroup, roles) VALUES(?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::INS_GROUP_MEMBER.sql().matches('?').count(),
        5
    );
}

#[test]
fn group_member_subgroup_update_statement_matches_cpp_exactly() {
    assert_eq!(
        CharStatements::UPD_GROUP_MEMBER_SUBGROUP.sql(),
        "UPDATE group_member SET subgroup = ? WHERE memberGuid = ?"
    );
    assert_eq!(
        CharStatements::UPD_GROUP_MEMBER_SUBGROUP
            .sql()
            .matches('?')
            .count(),
        2
    );
}

#[test]
fn group_member_flag_update_statement_matches_cpp_exactly() {
    assert_eq!(
        CharStatements::UPD_GROUP_MEMBER_FLAG.sql(),
        "UPDATE group_member SET memberFlags = ? WHERE memberGuid = ?"
    );
    assert_eq!(
        CharStatements::UPD_GROUP_MEMBER_FLAG
            .sql()
            .matches('?')
            .count(),
        2
    );
}

#[test]
fn group_insert_statement_matches_cpp_exactly() {
    assert_eq!(
        CharStatements::INS_GROUP.sql(),
        "INSERT INTO `groups` (guid, leaderGuid, lootMethod, looterGuid, lootThreshold, icon1, icon2, icon3, icon4, icon5, icon6, icon7, icon8, groupType, difficulty, raidDifficulty, legacyRaidDifficulty, masterLooterGuid) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(CharStatements::INS_GROUP.sql().matches('?').count(), 18);
}

#[test]
fn group_delete_and_leader_statements_match_cpp_exactly() {
    assert_eq!(
        CharStatements::UPD_GROUP_LEADER.sql(),
        "UPDATE `groups` SET leaderGuid = ? WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_GROUP_MEMBER.sql(),
        "DELETE FROM group_member WHERE memberGuid = ?"
    );
    assert_eq!(
        CharStatements::DEL_GROUP.sql(),
        "DELETE FROM `groups` WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_GROUP_MEMBER_ALL.sql(),
        "DELETE FROM group_member WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_LFG_DATA.sql(),
        "DELETE FROM lfg_data WHERE guid = ?"
    );
}

#[test]
fn group_startup_load_statements_match_cpp_exactly() {
    assert_eq!(
        CharStatements::DEL_GROUP_MEMBERS_WITHOUT_CHARACTER.sql(),
        "DELETE FROM group_member WHERE memberGuid NOT IN (SELECT guid FROM characters)"
    );
    assert_eq!(
        CharStatements::DEL_GROUPS_WITHOUT_LEADER.sql(),
        "DELETE FROM `groups` WHERE leaderGuid NOT IN (SELECT guid FROM characters)"
    );
    assert_eq!(
        CharStatements::DEL_GROUPS_WITH_FEWER_THAN_TWO_MEMBERS.sql(),
        "DELETE FROM `groups` WHERE guid NOT IN (SELECT guid FROM group_member GROUP BY guid HAVING COUNT(guid) > 1)"
    );
    assert_eq!(
        CharStatements::DEL_GROUP_MEMBERS_WITHOUT_GROUP.sql(),
        "DELETE FROM group_member WHERE guid NOT IN (SELECT guid FROM `groups`)"
    );
    assert_eq!(
        CharStatements::SEL_GROUPS.sql(),
        "SELECT g.leaderGuid, g.lootMethod, g.looterGuid, g.lootThreshold, g.icon1, g.icon2, g.icon3, g.icon4, g.icon5, g.icon6, g.icon7, g.icon8, g.groupType, g.difficulty, g.raiddifficulty, g.legacyRaidDifficulty, g.masterLooterGuid, g.guid, lfg.dungeon, lfg.state FROM `groups` g LEFT JOIN lfg_data lfg ON lfg.guid = g.guid ORDER BY g.guid ASC"
    );
    assert_eq!(
        CharStatements::SEL_GROUP_MEMBERS.sql(),
        "SELECT guid, memberGuid, memberFlags, subgroup, roles FROM group_member ORDER BY guid"
    );
    assert_eq!(
        CharStatements::SEL_GROUP_MEMBER_CHARACTER_CACHE.sql(),
        "SELECT guid, name, race, class FROM characters WHERE guid IN (SELECT leaderGuid FROM `groups` UNION SELECT memberGuid FROM group_member)"
    );
    assert_eq!(
        CharStatements::DEL_GROUP_MEMBERS_WITHOUT_CHARACTER
            .sql()
            .matches('?')
            .count(),
        0
    );
    assert_eq!(CharStatements::SEL_GROUPS.sql().matches('?').count(), 0);
    assert_eq!(
        CharStatements::SEL_GROUP_MEMBERS.sql().matches('?').count(),
        0
    );
    assert_eq!(
        CharStatements::SEL_GROUP_MEMBER_CHARACTER_CACHE
            .sql()
            .matches('?')
            .count(),
        0
    );
}

#[test]
fn character_startup_and_lookup_statements_match_cpp_exactly() {
    assert_eq!(
        CharStatements::DEL_POOL_QUEST_SAVE.sql(),
        "DELETE FROM pool_quest_save WHERE pool_id = ?"
    );
    assert_eq!(
        CharStatements::INS_POOL_QUEST_SAVE.sql(),
        "INSERT INTO pool_quest_save (pool_id, quest_id) VALUES (?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_NONEXISTENT_GUILD_BANK_ITEM.sql(),
        "DELETE FROM guild_bank_item WHERE guildid = ? AND TabId = ? AND SlotId = ?"
    );
    assert_eq!(
        CharStatements::DEL_EXPIRED_BANS.sql(),
        "UPDATE character_banned SET active = 0 WHERE unbandate <= UNIX_TIMESTAMP() AND unbandate <> bandate"
    );
    assert_eq!(
        CharStatements::SEL_CHECK_NAME.sql(),
        "SELECT 1 FROM characters WHERE name = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHECK_GUID.sql(),
        "SELECT 1 FROM characters WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_SUM_CHARS.sql(),
        "SELECT COUNT(guid) FROM characters WHERE account = ? AND deleteDate IS NULL"
    );
    assert_eq!(
        CharStatements::SEL_CHAR_CREATE_INFO.sql(),
        "SELECT level, race, class FROM characters WHERE account = ? LIMIT 0, ?"
    );
}

#[test]
fn character_save_statements_match_cpp_sql_exactly() {
    assert_eq!(
        CharStatements::INS_CHARACTER.sql(),
        "INSERT INTO characters (guid, account, name, race, class, gender, level, xp, money, inventorySlots, bankSlots, restState, playerFlags, playerFlagsEx, map, instance_id, dungeonDifficulty, raidDifficulty, legacyRaidDifficulty, position_x, position_y, position_z, orientation, trans_x, trans_y, trans_z, trans_o, transguid, taximask, createTime, createMode, cinematic, totaltime, leveltime, rest_bonus, logout_time, is_logout_resting, resettalents_cost, resettalents_time, activeTalentGroup, bonusTalentGroups,extra_flags, summonedPetNumber, at_login, death_expire_time, taxi_path, totalKills, todayKills, yesterdayKills, chosenTitle, watchedFaction, drunk, health, power1, power2, power3, power4, power5, power6, power7, power8, power9, power10, latency, lootSpecId, exploredZones, equipmentCache, knownTitles, actionBars, lastLoginBuild) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"
    );
    assert_eq!(
        CharStatements::UPD_CHARACTER.sql(),
        "UPDATE characters SET name=?,race=?,class=?,gender=?,level=?,xp=?,money=?,inventorySlots=?,bankSlots=?,restState=?,playerFlags=?,playerFlagsEx=?,map=?,instance_id=?,dungeonDifficulty=?,raidDifficulty=?,legacyRaidDifficulty=?,position_x=?,position_y=?,position_z=?,orientation=?,trans_x=?,trans_y=?,trans_z=?,trans_o=?,transguid=?,taximask=?,cinematic=?,totaltime=?,leveltime=?,rest_bonus=?,logout_time=?,is_logout_resting=?,resettalents_cost=?,resettalents_time=?,numRespecs=?,activeTalentGroup=?,bonusTalentGroups=?,extra_flags=?,summonedPetNumber=?,at_login=?,zone=?,death_expire_time=?,taxi_path=?,totalKills=?,todayKills=?,yesterdayKills=?,chosenTitle=?,watchedFaction=?,drunk=?,health=?,power1=?,power2=?,power3=?,power4=?,power5=?,power6=?,power7=?,power8=?,power9=?,power10=?,latency=?,lootSpecId=?,exploredZones=?,equipmentCache=?,knownTitles=?,actionBars=?,online=?,honor=?,honorLevel=?,honorRestState=?,honorRestBonus=?,lastLoginBuild=? WHERE guid=?"
    );
    assert_eq!(
        CharStatements::UPD_CHAR_TALENT_RESET_STATE.sql(),
        "UPDATE characters SET resettalents_cost = ?, resettalents_time = ? WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHAR_MONEY.sql(),
        "SELECT money FROM characters WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_CHAR_EXPLORED_ZONES.sql(),
        "UPDATE characters SET exploredZones = ? WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_ADD_AT_LOGIN_FLAG.sql(),
        "UPDATE characters SET at_login = at_login | ? WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_REM_AT_LOGIN_FLAG.sql(),
        "UPDATE characters set at_login = at_login & ~ ? WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_ALL_AT_LOGIN_FLAGS.sql(),
        "UPDATE characters SET at_login = at_login | ?"
    );
    assert_eq!(
        CharStatements::INS_BUG_REPORT.sql(),
        "INSERT INTO bugreport (type, content) VALUES(?, ?)"
    );
    assert_eq!(
        CharStatements::UPD_PETITION_NAME.sql(),
        "UPDATE petition SET name = ? WHERE petitionguid = ?"
    );
    assert_eq!(
        CharStatements::INS_PETITION_SIGNATURE.sql(),
        "INSERT INTO petition_sign (ownerguid, petitionguid, playerguid, player_account) VALUES (?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::UPD_ACCOUNT_ONLINE.sql(),
        "UPDATE characters SET online = 0 WHERE account = ?"
    );
    assert_eq!(
        CharStatements::INS_CHAR_CUSTOMIZATION.sql(),
        "INSERT INTO character_customizations (guid, chrCustomizationOptionID, chrCustomizationChoiceID) VALUES (?, ?, ?)"
    );
    assert_eq!(
        CharStatements::INS_CHARACTER_CUSTOMIZATION.sql(),
        CharStatements::INS_CHAR_CUSTOMIZATION.sql()
    );
    assert_eq!(
        CharStatements::DEL_CHARACTER_CUSTOMIZATIONS.sql(),
        "DELETE FROM character_customizations WHERE guid = ?"
    );
}

#[test]
fn upd_char_difficulties_matches_cpp_saveback_columns() {
    assert_eq!(
        CharStatements::UPD_CHAR_DIFFICULTIES.sql(),
        "UPDATE characters SET dungeonDifficulty = ?, raidDifficulty = ?, legacyRaidDifficulty = ? WHERE guid = ?"
    );
}

#[test]
fn upd_char_explored_zones_matches_cpp_saveback_column() {
    assert_eq!(
        CharStatements::UPD_CHAR_EXPLORED_ZONES.sql(),
        "UPDATE characters SET exploredZones = ? WHERE guid = ?"
    );
}

#[test]
fn character_ban_and_mail_list_statements_match_cpp_exactly() {
    assert_eq!(
        CharStatements::INS_CHARACTER_BAN.sql(),
        "INSERT INTO character_banned (guid, bandate, unbandate, bannedby, banreason, active) VALUES (?, UNIX_TIMESTAMP(), UNIX_TIMESTAMP()+?, ?, ?, 1)"
    );
    assert_eq!(
        CharStatements::UPD_CHARACTER_BAN.sql(),
        "UPDATE character_banned SET active = 0 WHERE guid = ? AND active != 0"
    );
    assert_eq!(
        CharStatements::DEL_CHARACTER_BAN.sql(),
        "DELETE cb FROM character_banned cb INNER JOIN characters c ON c.guid = cb.guid WHERE c.account = ?"
    );
    assert_eq!(
        CharStatements::SEL_BANINFO.sql(),
        "SELECT bandate, unbandate-bandate, active, unbandate, banreason, bannedby FROM character_banned WHERE guid = ? ORDER BY bandate ASC"
    );
    assert_eq!(
        CharStatements::SEL_GUID_BY_NAME_FILTER.sql(),
        "SELECT guid, name FROM characters WHERE name LIKE CONCAT('%%', ?, '%%')"
    );
    assert_eq!(
        CharStatements::SEL_BANINFO_LIST.sql(),
        "SELECT bandate, unbandate, bannedby, banreason FROM character_banned WHERE guid = ? ORDER BY unbandate"
    );
    assert_eq!(
        CharStatements::SEL_BANNED_NAME.sql(),
        "SELECT characters.name FROM characters, character_banned WHERE character_banned.guid = ? AND character_banned.guid = characters.guid"
    );
    assert_eq!(
        CharStatements::SEL_MAIL_LIST_COUNT.sql(),
        "SELECT COUNT(id) FROM mail WHERE receiver = ? "
    );
    assert_eq!(
        CharStatements::SEL_MAIL_LIST_INFO.sql(),
        "SELECT id, sender, (SELECT name FROM characters WHERE guid = sender) AS sendername, receiver, (SELECT name FROM characters WHERE guid = receiver) AS receivername, subject, deliver_time, expire_time, money, has_items FROM mail WHERE receiver = ? "
    );
    assert_eq!(
        CharStatements::SEL_MAIL_LIST_ITEMS.sql(),
        "SELECT itemEntry,count FROM item_instance WHERE guid = ?"
    );
}

#[test]
fn character_enum_statement_matches_cpp_column_order_exactly() {
    assert_eq!(
        CharStatements::SEL_ENUM.sql(),
        "SELECT c.guid, c.name, c.race, c.class, c.gender, c.level, c.zone, c.map, c.position_x, c.position_y, c.position_z, gm.guildid, c.playerFlags, c.at_login, cp.entry, cp.modelid, cp.level, c.equipmentCache, cb.guid, c.slot, c.logout_time, c.activeTalentGroup, c.lastLoginBuild, c.personalTabardEmblemStyle, c.personalTabardEmblemColor, c.personalTabardBorderStyle, c.personalTabardBorderColor, c.personalTabardBackgroundColor FROM characters AS c LEFT JOIN character_pet AS cp ON c.summonedPetNumber = cp.id LEFT JOIN guild_member AS gm ON c.guid = gm.guid LEFT JOIN character_banned AS cb ON c.guid = cb.guid AND cb.active = 1 WHERE c.account = ? AND c.deleteInfos_Name IS NULL"
    );
    assert_eq!(CharStatements::SEL_ENUM.sql().matches('?').count(), 1);
}

#[test]
fn character_enum_variants_match_cpp_exactly() {
    assert_eq!(
        CharStatements::SEL_ENUM_DECLINED_NAME.sql(),
        "SELECT c.guid, c.name, c.race, c.class, c.gender, c.level, c.zone, c.map, c.position_x, c.position_y, c.position_z, gm.guildid, c.playerFlags, c.at_login, cp.entry, cp.modelid, cp.level, c.equipmentCache, cb.guid, c.slot, c.logout_time, c.activeTalentGroup, c.lastLoginBuild, c.personalTabardEmblemStyle, c.personalTabardEmblemColor, c.personalTabardBorderStyle, c.personalTabardBorderColor, c.personalTabardBackgroundColor, cd.genitive FROM characters AS c LEFT JOIN character_pet AS cp ON c.summonedPetNumber = cp.id LEFT JOIN guild_member AS gm ON c.guid = gm.guid LEFT JOIN character_banned AS cb ON c.guid = cb.guid AND cb.active = 1 LEFT JOIN character_declinedname AS cd ON c.guid = cd.guid WHERE c.account = ? AND c.deleteInfos_Name IS NULL"
    );
    assert_eq!(
        CharStatements::SEL_ENUM_CUSTOMIZATIONS.sql(),
        "SELECT cc.guid, cc.chrCustomizationOptionID, cc.chrCustomizationChoiceID FROM character_customizations cc LEFT JOIN characters c ON cc.guid = c.guid WHERE c.account = ? AND c.deleteInfos_Name IS NULL ORDER BY cc.guid, cc.chrCustomizationOptionID"
    );
    assert_eq!(
        CharStatements::SEL_UNDELETE_ENUM.sql(),
        "SELECT c.guid, c.deleteInfos_Name, c.race, c.class, c.gender, c.level, c.zone, c.map, c.position_x, c.position_y, c.position_z, gm.guildid, c.playerFlags, c.at_login, cp.entry, cp.modelid, cp.level, c.equipmentCache, cb.guid, c.slot, c.logout_time, c.activeTalentGroup, c.lastLoginBuild, c.personalTabardEmblemStyle, c.personalTabardEmblemColor, c.personalTabardBorderStyle, c.personalTabardBorderColor, c.personalTabardBackgroundColor FROM characters AS c LEFT JOIN character_pet AS cp ON c.summonedPetNumber = cp.id LEFT JOIN guild_member AS gm ON c.guid = gm.guid LEFT JOIN character_banned AS cb ON c.guid = cb.guid AND cb.active = 1 WHERE c.deleteInfos_Account = ? AND c.deleteInfos_Name IS NOT NULL"
    );
    assert_eq!(
        CharStatements::SEL_UNDELETE_ENUM_DECLINED_NAME.sql(),
        "SELECT c.guid, c.deleteInfos_Name, c.race, c.class, c.gender, c.level, c.zone, c.map, c.position_x, c.position_y, c.position_z, gm.guildid, c.playerFlags, c.at_login, cp.entry, cp.modelid, cp.level, c.equipmentCache, cb.guid, c.slot, c.logout_time, c.activeTalentGroup, c.lastLoginBuild, c.personalTabardEmblemStyle, c.personalTabardEmblemColor, c.personalTabardBorderStyle, c.personalTabardBorderColor, c.personalTabardBackgroundColor, cd.genitive FROM characters AS c LEFT JOIN character_pet AS cp ON c.summonedPetNumber = cp.id LEFT JOIN guild_member AS gm ON c.guid = gm.guid LEFT JOIN character_banned AS cb ON c.guid = cb.guid AND cb.active = 1 LEFT JOIN character_declinedname AS cd ON c.guid = cd.guid WHERE c.deleteInfos_Account = ? AND c.deleteInfos_Name IS NOT NULL"
    );
    assert_eq!(
        CharStatements::SEL_UNDELETE_ENUM_CUSTOMIZATIONS.sql(),
        "SELECT cc.guid, cc.chrCustomizationOptionID, cc.chrCustomizationChoiceID FROM character_customizations cc LEFT JOIN characters c ON cc.guid = c.guid WHERE c.deleteInfos_Account = ? AND c.deleteInfos_Name IS NOT NULL ORDER BY cc.guid, cc.chrCustomizationOptionID"
    );
}

#[test]
fn character_position_and_random_bg_statements_match_cpp_exactly() {
    assert_eq!(
        CharStatements::SEL_FREE_NAME.sql(),
        "SELECT name, at_login FROM characters WHERE guid = ? AND NOT EXISTS (SELECT NULL FROM characters WHERE name = ?)"
    );
    assert_eq!(
        CharStatements::SEL_CHAR_ZONE.sql(),
        "SELECT zone FROM characters WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHAR_POSITION_XYZ.sql(),
        "SELECT map, position_x, position_y, position_z FROM characters WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHAR_POSITION.sql(),
        "SELECT position_x, position_y, position_z, orientation, map, taxi_path FROM characters WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_BATTLEGROUND_RANDOM_ALL.sql(),
        "DELETE FROM character_battleground_random"
    );
    assert_eq!(
        CharStatements::DEL_BATTLEGROUND_RANDOM.sql(),
        "DELETE FROM character_battleground_random WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::INS_BATTLEGROUND_RANDOM.sql(),
        "INSERT INTO character_battleground_random (guid) VALUES (?)"
    );
}

#[test]
fn character_full_load_statement_matches_cpp_column_order_exactly() {
    assert_eq!(
        CharStatements::SEL_CHARACTER.sql(),
        "SELECT c.guid, account, name, race, class, gender, level, xp, money, inventorySlots, bankSlots, restState, playerFlags, playerFlagsEx, position_x, position_y, position_z, map, orientation, taximask, createTime, createMode, cinematic, totaltime, leveltime, rest_bonus, logout_time, is_logout_resting, resettalents_cost, resettalents_time, activeTalentGroup, bonusTalentGroups, trans_x, trans_y, trans_z, trans_o, transguid, extra_flags, summonedPetNumber, at_login, zone, online, death_expire_time, taxi_path, dungeonDifficulty, totalKills, todayKills, yesterdayKills, chosenTitle, watchedFaction, drunk, health, power1, power2, power3, power4, power5, power6, power7, power8, power9, power10, instance_id, lootSpecId, exploredZones, knownTitles, actionBars, raidDifficulty, legacyRaidDifficulty, fishingSteps, honor, honorLevel, honorRestState, honorRestBonus, numRespecs, personalTabardEmblemStyle, personalTabardEmblemColor, personalTabardBorderStyle, personalTabardBorderColor, personalTabardBackgroundColor FROM characters c LEFT JOIN character_fishingsteps cfs ON c.guid = cfs.guid WHERE c.guid = ?"
    );
    assert_eq!(CharStatements::SEL_CHARACTER.sql().matches('?').count(), 1);
}

#[test]
fn character_load_auxiliary_statements_match_cpp_exactly() {
    assert_eq!(
        CharStatements::SEL_CHARACTER_CUSTOMIZATIONS.sql(),
        "SELECT chrCustomizationOptionID, chrCustomizationChoiceID FROM character_customizations WHERE guid = ? ORDER BY chrCustomizationOptionID"
    );
    assert_eq!(
        CharStatements::SEL_GROUP_MEMBER.sql(),
        "SELECT guid FROM group_member WHERE memberGuid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHARACTER_AURAS.sql(),
        "SELECT casterGuid, itemGuid, spell, effectMask, recalculateMask, difficulty, stackCount, maxDuration, remainTime, remainCharges, castItemId, castItemLevel FROM character_aura WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHARACTER_AURA_EFFECTS.sql(),
        "SELECT casterGuid, itemGuid, spell, effectMask, effectIndex, amount, baseAmount FROM character_aura_effect WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHARACTER_SPELL_FAVORITES.sql(),
        "SELECT spell FROM character_spell_favorite WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHARACTER_REPUTATION.sql(),
        "SELECT faction, standing, flags FROM character_reputation WHERE guid = ?"
    );
}

#[test]
fn character_quest_load_statements_match_cpp_exactly() {
    assert_eq!(
        CharStatements::SEL_CHARACTER_QUESTSTATUS_OBJECTIVES_CRITERIA.sql(),
        "SELECT questObjectiveId FROM character_queststatus_objectives_criteria WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHARACTER_QUESTSTATUS_OBJECTIVES_CRITERIA_PROGRESS.sql(),
        "SELECT criteriaId, counter, date FROM character_queststatus_objectives_criteria_progress WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHARACTER_QUESTSTATUS_DAILY.sql(),
        "SELECT quest, time FROM character_queststatus_daily WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHARACTER_QUESTSTATUS_WEEKLY.sql(),
        "SELECT quest FROM character_queststatus_weekly WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHARACTER_QUESTSTATUS_MONTHLY.sql(),
        "SELECT quest FROM character_queststatus_monthly WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHARACTER_QUESTSTATUS_SEASONAL.sql(),
        "SELECT quest, event, completedTime FROM character_queststatus_seasonal WHERE guid = ?"
    );
}

#[test]
fn character_social_guild_bg_and_favorite_statements_match_cpp_exactly() {
    assert_eq!(
        CharStatements::SEL_MAIL_COUNT.sql(),
        "SELECT COUNT(*) FROM mail WHERE receiver = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHARACTER_SOCIALLIST.sql(),
        "SELECT cs.friend, c.account, cs.flags, cs.note FROM character_social cs JOIN characters c ON c.guid = cs.friend WHERE cs.guid = ? AND c.deleteinfos_name IS NULL LIMIT 255"
    );
    assert_eq!(
        CharStatements::SEL_CHARACTER_HOMEBIND.sql(),
        "SELECT mapId, zoneId, posX, posY, posZ, orientation FROM character_homebind WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHARACTER_SPELLCOOLDOWNS.sql(),
        "SELECT spell, item, time, categoryId, categoryEnd FROM character_spell_cooldown WHERE guid = ? AND time > UNIX_TIMESTAMP()"
    );
    assert_eq!(
        CharStatements::SEL_CHARACTER_SPELL_CHARGES.sql(),
        "SELECT categoryId, rechargeStart, rechargeEnd FROM character_spell_charges WHERE guid = ? AND rechargeEnd > UNIX_TIMESTAMP() ORDER BY rechargeEnd"
    );
    assert_eq!(
        CharStatements::SEL_CHARACTER_DECLINEDNAMES.sql(),
        "SELECT genitive, dative, accusative, instrumental, prepositional FROM character_declinedname WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_GUILD_MEMBER.sql(),
        "SELECT guildid, `rank` FROM guild_member WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_GUILD_MEMBER_EXTENDED.sql(),
        "SELECT g.guildid, g.name, gr.rname, gr.rid, gm.pnote, gm.offnote FROM guild g JOIN guild_member gm ON g.guildid = gm.guildid JOIN guild_rank gr ON g.guildid = gr.guildid AND gm.`rank` = gr.rid WHERE gm.guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHARACTER_ACHIEVEMENTS.sql(),
        "SELECT achievement, date FROM character_achievement WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHARACTER_CRITERIAPROGRESS.sql(),
        "SELECT criteria, counter, date FROM character_achievement_progress WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHARACTER_EQUIPMENTSETS.sql(),
        "SELECT setguid, setindex, name, iconname, ignore_mask, AssignedSpecIndex, item0, item1, item2, item3, item4, item5, item6, item7, item8, item9, item10, item11, item12, item13, item14, item15, item16, item17, item18 FROM character_equipmentsets WHERE guid = ? ORDER BY setindex"
    );
    assert_eq!(
        CharStatements::SEL_CHARACTER_TRANSMOG_OUTFITS.sql(),
        "SELECT setguid, setindex, name, iconname, ignore_mask, appearance0, appearance1, appearance2, appearance3, appearance4, appearance5, appearance6, appearance7, appearance8, appearance9, appearance10, appearance11, appearance12, appearance13, appearance14, appearance15, appearance16, appearance17, appearance18, mainHandEnchant, offHandEnchant FROM character_transmog_outfits WHERE guid = ? ORDER BY setindex"
    );
    assert_eq!(
        CharStatements::SEL_CHARACTER_BGDATA.sql(),
        "SELECT instanceId, team, joinX, joinY, joinZ, joinO, joinMapId, taxiStart, taxiEnd, mountSpell, queueId FROM character_battleground_data WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHARACTER_GLYPHS.sql(),
        "SELECT talentGroup, glyphSlot, glyphId FROM character_glyphs WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHARACTER_TALENTS.sql(),
        "SELECT talentId, talentRank, talentGroup FROM character_talent WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHARACTER_RANDOMBG.sql(),
        "SELECT guid FROM character_battleground_random WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHARACTER_BANNED.sql(),
        "SELECT guid FROM character_banned WHERE guid = ? AND active = 1"
    );
    assert_eq!(
        CharStatements::SEL_CHARACTER_QUESTSTATUSREW.sql(),
        "SELECT quest FROM character_queststatus_rewarded WHERE guid = ? AND active = 1"
    );
    assert_eq!(
        CharStatements::SEL_CHARACTER_FAVORITE_AUCTIONS.sql(),
        "SELECT `order`, itemId, itemLevel, battlePetSpeciesId, suffixItemNameDescriptionId FROM character_favorite_auctions WHERE guid = ? ORDER BY `order`"
    );
    assert_eq!(
        CharStatements::INS_CHARACTER_FAVORITE_AUCTION.sql(),
        "INSERT INTO character_favorite_auctions (guid, `order`, itemId, itemLevel, battlePetSpeciesId, suffixItemNameDescriptionId) VALUE (?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_CHARACTER_FAVORITE_AUCTION.sql(),
        "DELETE FROM character_favorite_auctions WHERE guid = ? AND `order` = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHARACTER_FAVORITE_AUCTIONS_BY_CHAR.sql(),
        "DELETE FROM character_favorite_auctions WHERE guid = ?"
    );
}

#[test]
fn character_auction_statements_match_cpp_exactly() {
    assert_eq!(
        CharStatements::SEL_AUCTIONS.sql(),
        "SELECT id, auctionHouseId, owner, bidder, minBid, buyoutOrUnitPrice, deposit, bidAmount, startTime, endTime, serverFlags FROM auctionhouse"
    );
    assert_eq!(
        CharStatements::INS_AUCTION_ITEMS.sql(),
        "INSERT INTO auction_items (auctionId, itemGuid) VALUES (?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_AUCTION_ITEMS_BY_ITEM.sql(),
        "DELETE FROM auction_items WHERE itemGuid = ?"
    );
    assert_eq!(
        CharStatements::SEL_AUCTION_BIDDERS.sql(),
        "SELECT auctionId, playerGuid FROM auction_bidders"
    );
    assert_eq!(
        CharStatements::INS_AUCTION_BIDDER.sql(),
        "INSERT INTO auction_bidders (auctionId, playerGuid) VALUES (?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_AUCTION_BIDDER_BY_PLAYER.sql(),
        "DELETE FROM auction_bidders WHERE playerGuid = ?"
    );
    assert_eq!(
        CharStatements::INS_AUCTION.sql(),
        "INSERT INTO auctionhouse (id, auctionHouseId, owner, bidder, minBid, buyoutOrUnitPrice, deposit, bidAmount, startTime, endTime, serverFlags) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_AUCTION.sql(),
        "DELETE a, ab, ai FROM auctionhouse a LEFT JOIN auction_items ai ON a.id = ai.auctionId LEFT JOIN auction_bidders ab ON a.id = ab.auctionId WHERE a.id = ?"
    );
    assert_eq!(
        CharStatements::UPD_AUCTION_BID.sql(),
        "UPDATE auctionhouse SET bidder = ?, bidAmount = ?, serverFlags = ? WHERE id = ?"
    );
    assert_eq!(
        CharStatements::UPD_AUCTION_EXPIRATION.sql(),
        "UPDATE auctionhouse SET endTime = ? WHERE id = ?"
    );
}

#[test]
fn character_mail_lifecycle_statements_match_cpp_exactly() {
    assert_eq!(
        CharStatements::INS_MAIL.sql(),
        "INSERT INTO mail(id, messageType, stationery, mailTemplateId, sender, receiver, subject, body, has_items, expire_time, deliver_time, money, cod, checked) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_MAIL_BY_ID.sql(),
        "DELETE FROM mail WHERE id = ?"
    );
    assert_eq!(
        CharStatements::INS_MAIL_ITEM.sql(),
        "INSERT INTO mail_items(mail_id, item_guid, receiver) VALUES (?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_MAIL_ITEM.sql(),
        "DELETE FROM mail_items WHERE item_guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_INVALID_MAIL_ITEM.sql(),
        "DELETE FROM mail_items WHERE item_guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_EMPTY_EXPIRED_MAIL.sql(),
        "DELETE FROM mail WHERE expire_time < ? AND has_items = 0 AND body = ''"
    );
    assert_eq!(
        CharStatements::SEL_EXPIRED_MAIL.sql(),
        "SELECT id, messageType, sender, receiver, has_items, expire_time, cod, checked, mailTemplateId FROM mail WHERE expire_time < ?"
    );
    assert_eq!(
        CharStatements::SEL_EXPIRED_MAIL_ITEMS.sql(),
        "SELECT item_guid, itemEntry, mail_id FROM mail_items mi INNER JOIN item_instance ii ON ii.guid = mi.item_guid LEFT JOIN mail mm ON mi.mail_id = mm.id WHERE mm.id IS NOT NULL AND mm.expire_time < ?"
    );
    assert_eq!(
        CharStatements::UPD_MAIL_RETURNED.sql(),
        "UPDATE mail SET sender = ?, receiver = ?, expire_time = ?, deliver_time = ?, cod = 0, checked = ? WHERE id = ?"
    );
    assert_eq!(
        CharStatements::UPD_MAIL_ITEM_RECEIVER.sql(),
        "UPDATE mail_items SET receiver = ? WHERE item_guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_ITEM_OWNER.sql(),
        "UPDATE item_instance SET owner_guid = ? WHERE guid = ?"
    );
}

#[test]
fn char_statements_have_sql() {
    assert!(!CharStatements::DEL_POOL_QUEST_SAVE.sql().is_empty());
    assert!(!CharStatements::INS_POOL_QUEST_SAVE.sql().is_empty());
    assert!(
        !CharStatements::DEL_NONEXISTENT_GUILD_BANK_ITEM
            .sql()
            .is_empty()
    );
    assert!(!CharStatements::DEL_EXPIRED_BANS.sql().is_empty());
    assert!(!CharStatements::SEL_ENUM.sql().is_empty());
    assert!(!CharStatements::SEL_ENUM_DECLINED_NAME.sql().is_empty());
    assert!(!CharStatements::SEL_ENUM_CUSTOMIZATIONS.sql().is_empty());
    assert!(!CharStatements::SEL_UNDELETE_ENUM.sql().is_empty());
    assert!(
        !CharStatements::SEL_UNDELETE_ENUM_DECLINED_NAME
            .sql()
            .is_empty()
    );
    assert!(
        !CharStatements::SEL_UNDELETE_ENUM_CUSTOMIZATIONS
            .sql()
            .is_empty()
    );
    assert!(!CharStatements::SEL_CHECK_NAME.sql().is_empty());
    assert!(!CharStatements::SEL_CHECK_GUID.sql().is_empty());
    assert!(!CharStatements::SEL_SUM_CHARS.sql().is_empty());
    assert!(!CharStatements::SEL_CHAR_CREATE_INFO.sql().is_empty());
    assert!(!CharStatements::INS_CHARACTER_BAN.sql().is_empty());
    assert!(!CharStatements::UPD_CHARACTER_BAN.sql().is_empty());
    assert!(!CharStatements::DEL_CHARACTER_BAN.sql().is_empty());
    assert!(!CharStatements::SEL_BANINFO.sql().is_empty());
    assert!(!CharStatements::SEL_GUID_BY_NAME_FILTER.sql().is_empty());
    assert!(!CharStatements::SEL_BANINFO_LIST.sql().is_empty());
    assert!(!CharStatements::SEL_BANNED_NAME.sql().is_empty());
    assert!(!CharStatements::SEL_MAIL_LIST_COUNT.sql().is_empty());
    assert!(!CharStatements::SEL_MAIL_LIST_INFO.sql().is_empty());
    assert!(!CharStatements::SEL_MAIL_LIST_ITEMS.sql().is_empty());
    assert!(!CharStatements::SEL_FREE_NAME.sql().is_empty());
    assert!(!CharStatements::SEL_CHAR_ZONE.sql().is_empty());
    assert!(!CharStatements::SEL_CHAR_POSITION_XYZ.sql().is_empty());
    assert!(!CharStatements::SEL_CHAR_POSITION.sql().is_empty());
    assert!(!CharStatements::DEL_BATTLEGROUND_RANDOM_ALL.sql().is_empty());
    assert!(!CharStatements::DEL_BATTLEGROUND_RANDOM.sql().is_empty());
    assert!(!CharStatements::INS_BATTLEGROUND_RANDOM.sql().is_empty());
    assert!(!CharStatements::INS_CHARACTER.sql().is_empty());
    assert!(!CharStatements::INS_CHAR_CUSTOMIZATION.sql().is_empty());
    assert!(!CharStatements::DEL_CHARACTER.sql().is_empty());
    assert!(!CharStatements::SEL_CHARACTER_REPUTATION.sql().is_empty());
    assert!(
        !CharStatements::DEL_CHAR_REPUTATION_BY_FACTION
            .sql()
            .is_empty()
    );
    assert!(
        !CharStatements::INS_CHAR_REPUTATION_BY_FACTION
            .sql()
            .is_empty()
    );
    assert!(!CharStatements::DEL_CHAR_REPUTATION.sql().is_empty());
    assert!(!CharStatements::SEL_CHARACTER.sql().is_empty());
    assert!(!CharStatements::UPD_CHAR_ONLINE.sql().is_empty());
    assert!(!CharStatements::UPD_CHAR_OFFLINE.sql().is_empty());
    assert!(!CharStatements::SEL_CHAR_DEL_CHECK.sql().is_empty());
    assert!(!CharStatements::SEL_MAX_GUID.sql().is_empty());
    assert!(!CharStatements::SEL_PLAYER_CURRENCY.sql().is_empty());
    assert!(!CharStatements::UPD_PLAYER_CURRENCY.sql().is_empty());
    assert!(!CharStatements::REP_PLAYER_CURRENCY.sql().is_empty());
    assert!(!CharStatements::UPD_GROUP_TYPE.sql().is_empty());
    assert!(!CharStatements::UPD_CHAR_PLAYED_TIME.sql().is_empty());
    assert!(!CharStatements::SEL_CHARACTER_INSTANCE_LOCK.sql().is_empty());
    assert!(!CharStatements::INS_CHARACTER_INSTANCE_LOCK.sql().is_empty());
    assert!(!CharStatements::INS_INSTANCE.sql().is_empty());
    assert!(!CharStatements::SEL_RESPAWNS.sql().is_empty());
    assert!(!CharStatements::SEL_ALL_RESPAWNS.sql().is_empty());
    assert!(!CharStatements::REP_RESPAWN.sql().is_empty());
    assert!(!CharStatements::DEL_RESPAWN.sql().is_empty());
    assert!(!CharStatements::DEL_ALL_RESPAWNS.sql().is_empty());
    assert!(!CharStatements::DEL_GAME_EVENT_SAVE.sql().is_empty());
    assert!(!CharStatements::INS_GAME_EVENT_SAVE.sql().is_empty());
    assert!(
        !CharStatements::SEL_GAME_EVENT_CONDITION_SAVES
            .sql()
            .is_empty()
    );
    assert!(
        !CharStatements::DEL_ALL_GAME_EVENT_CONDITION_SAVE
            .sql()
            .is_empty()
    );
    assert!(
        !CharStatements::DEL_GAME_EVENT_CONDITION_SAVE
            .sql()
            .is_empty()
    );
    assert!(
        !CharStatements::INS_GAME_EVENT_CONDITION_SAVE
            .sql()
            .is_empty()
    );
    assert!(
        !CharStatements::DEL_RESET_CHARACTER_QUESTSTATUS_SEASONAL_BY_EVENT
            .sql()
            .is_empty()
    );
    assert!(
        !CharStatements::DEL_CHARACTER_QUESTSTATUS_DAILY
            .sql()
            .is_empty()
    );
    assert!(
        !CharStatements::DEL_CHARACTER_QUESTSTATUS_WEEKLY
            .sql()
            .is_empty()
    );
    assert!(
        !CharStatements::DEL_CHARACTER_QUESTSTATUS_MONTHLY
            .sql()
            .is_empty()
    );
    assert!(
        !CharStatements::DEL_CHARACTER_QUESTSTATUS_SEASONAL
            .sql()
            .is_empty()
    );
    assert!(
        !CharStatements::INS_CHARACTER_QUESTSTATUS_DAILY
            .sql()
            .is_empty()
    );
    assert!(
        !CharStatements::INS_CHARACTER_QUESTSTATUS_WEEKLY
            .sql()
            .is_empty()
    );
    assert!(
        !CharStatements::INS_CHARACTER_QUESTSTATUS_MONTHLY
            .sql()
            .is_empty()
    );
    assert!(
        !CharStatements::INS_CHARACTER_QUESTSTATUS_SEASONAL
            .sql()
            .is_empty()
    );
    assert!(
        !CharStatements::SEL_CHAR_QUEST_STATUS_SEASONAL
            .sql()
            .is_empty()
    );
    assert!(!CharStatements::SEL_WORLD_STATE_VALUES.sql().is_empty());
    assert!(!CharStatements::REP_WORLD_STATE.sql().is_empty());
}

#[test]
fn game_event_save_statements_match_cpp_sql_exactly() {
    assert_eq!(
        CharStatements::DEL_GAME_EVENT_SAVE.sql(),
        "DELETE FROM game_event_save WHERE eventEntry = ?"
    );
    assert_eq!(
        CharStatements::INS_GAME_EVENT_SAVE.sql(),
        "INSERT INTO game_event_save (eventEntry, state, next_start) VALUES (?, ?, ?)"
    );
    assert_eq!(
        CharStatements::SEL_GAME_EVENT_CONDITION_SAVES.sql(),
        "SELECT eventEntry, condition_id, done FROM game_event_condition_save"
    );
    assert_eq!(
        CharStatements::DEL_ALL_GAME_EVENT_CONDITION_SAVE.sql(),
        "DELETE FROM game_event_condition_save WHERE eventEntry = ?"
    );
    assert_eq!(
        CharStatements::DEL_GAME_EVENT_CONDITION_SAVE.sql(),
        "DELETE FROM game_event_condition_save WHERE eventEntry = ? AND condition_id = ?"
    );
    assert_eq!(
        CharStatements::INS_GAME_EVENT_CONDITION_SAVE.sql(),
        "INSERT INTO game_event_condition_save (eventEntry, condition_id, done) VALUES (?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_RESET_CHARACTER_QUESTSTATUS_SEASONAL_BY_EVENT.sql(),
        "DELETE FROM character_queststatus_seasonal WHERE event = ? AND completedTime < ?"
    );
    assert_eq!(
        CharStatements::SEL_CHAR_QUEST_STATUS_SEASONAL.sql(),
        "SELECT quest, event, completedTime FROM character_queststatus_seasonal WHERE guid = ?"
    );
}

#[test]
fn character_reputation_statements_match_cpp_sql_exactly() {
    assert_eq!(
        CharStatements::SEL_CHARACTER_REPUTATION.sql(),
        "SELECT faction, standing, flags FROM character_reputation WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_REPUTATION_BY_FACTION.sql(),
        "DELETE FROM character_reputation WHERE guid = ? AND faction = ?"
    );
    assert_eq!(
        CharStatements::INS_CHAR_REPUTATION_BY_FACTION.sql(),
        "INSERT INTO character_reputation (guid, faction, standing, flags) VALUES (?, ?, ? , ?)"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_REPUTATION.sql(),
        "DELETE FROM character_reputation WHERE guid = ?"
    );
}

#[test]
fn quest_reward_lockout_status_save_statements_match_cpp_sql_exactly() {
    assert_eq!(
        CharStatements::DEL_CHARACTER_QUESTSTATUS_DAILY.sql(),
        "DELETE FROM character_queststatus_daily WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHARACTER_QUESTSTATUS_WEEKLY.sql(),
        "DELETE FROM character_queststatus_weekly WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHARACTER_QUESTSTATUS_MONTHLY.sql(),
        "DELETE FROM character_queststatus_monthly WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHARACTER_QUESTSTATUS_SEASONAL.sql(),
        "DELETE FROM character_queststatus_seasonal WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::INS_CHARACTER_QUESTSTATUS_DAILY.sql(),
        "INSERT INTO character_queststatus_daily (guid, quest, time) VALUES (?, ?, ?)"
    );
    assert_eq!(
        CharStatements::INS_CHARACTER_QUESTSTATUS_WEEKLY.sql(),
        "INSERT INTO character_queststatus_weekly (guid, quest) VALUES (?, ?)"
    );
    assert_eq!(
        CharStatements::INS_CHARACTER_QUESTSTATUS_MONTHLY.sql(),
        "INSERT INTO character_queststatus_monthly (guid, quest) VALUES (?, ?)"
    );
    assert_eq!(
        CharStatements::INS_CHARACTER_QUESTSTATUS_SEASONAL.sql(),
        "INSERT INTO character_queststatus_seasonal (guid, quest, event, completedTime) VALUES (?, ?, ?, ?)"
    );
}
