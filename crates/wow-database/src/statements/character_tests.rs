//! Behaviour tests for [`super`].
//!
//! Extracted verbatim from `character.rs`, which was 6,684 lines of which
//! 3,289 — 49% — were this one `mod tests`. The production code and its
//! module boundaries are untouched: moving tests moves no invariant.

#![cfg(test)]

use super::*;

fn cpp_character_database_cpp() -> &'static str {
    "/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/CharacterDatabase.cpp"
}

fn cpp_string_literals(block: &str) -> String {
    let mut output = String::new();
    let bytes = block.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'"' {
            i += 1;
            continue;
        }

        i += 1;
        while i < bytes.len() {
            if bytes[i] == b'\\' {
                if i + 1 < bytes.len() {
                    output.push(bytes[i + 1] as char);
                    i += 2;
                    continue;
                }
            }
            if bytes[i] == b'"' {
                i += 1;
                break;
            }
            output.push(bytes[i] as char);
            i += 1;
        }
    }
    output
}

fn select_item_instance_content(cpp: &str) -> String {
    let start = cpp
        .find("#define SelectItemInstanceContent")
        .expect("C++ SelectItemInstanceContent macro must exist");
    let end = cpp[start..]
        .find("\n\n")
        .map(|offset| start + offset)
        .expect("C++ SelectItemInstanceContent macro block must end before statements");
    cpp_string_literals(&cpp[start..end])
}

fn cpp_character_sql() -> Vec<String> {
    let contents = std::fs::read_to_string(cpp_character_database_cpp())
        .expect("C++ CharacterDatabase.cpp must be available for parity tests");
    let item_content = select_item_instance_content(&contents);
    let mut sql = Vec::new();
    let mut offset = 0;
    while let Some(relative_start) = contents[offset..].find("PrepareStatement(CHAR_") {
        let start = offset + relative_start;
        let Some(relative_end) = contents[start..].find("CONNECTION_") else {
            break;
        };
        let after_connection = start + relative_end;
        let Some(relative_stmt_end) = contents[after_connection..].find(");") else {
            break;
        };
        let end = after_connection + relative_stmt_end + 2;
        let block = &contents[start..end];
        let mut statement_sql = cpp_string_literals(block);
        if block.contains("SelectItemInstanceContent") {
            statement_sql =
                statement_sql.replacen("SELECT ,", &format!("SELECT {item_content},"), 1);
        }
        sql.push(statement_sql);
        offset = end;
    }
    sql
}

#[test]
fn generated_cpp_statements_cover_character_database() {
    let statements = cpp_character_sql();
    assert_eq!(statements.len(), 523);

    for cpp_sql in statements {
        let sql: &'static str = Box::leak(cpp_sql.into_boxed_str());
        assert_eq!(CharStatements::cpp(sql).sql(), sql);
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

#[test]
fn character_void_calendar_pet_pvp_questtrack_spell_location_statements_match_cpp_sql_exactly() {
    assert_eq!(
        CharStatements::DEL_RESET_CHARACTER_QUESTSTATUS_DAILY.sql(),
        "DELETE FROM character_queststatus_daily"
    );
    assert_eq!(
        CharStatements::DEL_RESET_CHARACTER_QUESTSTATUS_WEEKLY.sql(),
        "DELETE FROM character_queststatus_weekly"
    );
    assert_eq!(
        CharStatements::DEL_RESET_CHARACTER_QUESTSTATUS_MONTHLY.sql(),
        "DELETE FROM character_queststatus_monthly"
    );
    assert_eq!(
        CharStatements::SEL_CHAR_VOID_STORAGE.sql(),
        "SELECT itemId, itemEntry, slot, creatorGuid, fixedScalingLevel, randomPropertiesId, randomPropertiesSeed, context FROM character_void_storage WHERE playerGuid = ?"
    );
    assert_eq!(
        CharStatements::REP_CHAR_VOID_STORAGE_ITEM.sql(),
        "REPLACE INTO character_void_storage (itemId, playerGuid, itemEntry, slot, creatorGuid, fixedScalingLevel, randomPropertiesId, randomPropertiesSeed, context) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_VOID_STORAGE_ITEM_BY_CHAR_GUID.sql(),
        "DELETE FROM character_void_storage WHERE playerGuid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_VOID_STORAGE_ITEM_BY_SLOT.sql(),
        "DELETE FROM character_void_storage WHERE slot = ? AND playerGuid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHAR_CUF_PROFILES.sql(),
        "SELECT id, name, frameHeight, frameWidth, sortBy, healthText, boolOptions, topPoint, bottomPoint, leftPoint, topOffset, bottomOffset, leftOffset FROM character_cuf_profiles WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::REP_CHAR_CUF_PROFILES.sql(),
        "REPLACE INTO character_cuf_profiles (guid, id, name, frameHeight, frameWidth, sortBy, healthText, boolOptions, topPoint, bottomPoint, leftPoint, topOffset, bottomOffset, leftOffset) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::REP_CHAR_CUF_PROFILES
            .sql()
            .matches('?')
            .count(),
        14
    );
    assert_eq!(
        CharStatements::DEL_CHAR_CUF_PROFILES_BY_ID.sql(),
        "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_CUF_PROFILES.sql(),
        "DELETE FROM character_cuf_profiles WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::REP_CALENDAR_EVENT.sql(),
        "REPLACE INTO calendar_events (EventID, Owner, Title, Description, EventType, TextureID, Date, Flags, LockDate) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_CALENDAR_EVENT.sql(),
        "DELETE FROM calendar_events WHERE EventID = ?"
    );
    assert_eq!(
        CharStatements::REP_CALENDAR_INVITE.sql(),
        "REPLACE INTO calendar_invites (InviteID, EventID, Invitee, Sender, Status, ResponseTime, ModerationRank, Note) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_CALENDAR_INVITE.sql(),
        "DELETE FROM calendar_invites WHERE InviteID = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHAR_PET_IDS.sql(),
        "SELECT id FROM character_pet WHERE owner = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_PET_DECLINEDNAME_BY_OWNER.sql(),
        "DELETE FROM character_pet_declinedname WHERE owner = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_PET_DECLINEDNAME.sql(),
        "DELETE FROM character_pet_declinedname WHERE id = ?"
    );
    assert_eq!(
        CharStatements::INS_CHAR_PET_DECLINEDNAME.sql(),
        "INSERT INTO character_pet_declinedname (id, owner, genitive, dative, accusative, instrumental, prepositional) VALUES (?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::SEL_PET_AURA.sql(),
        "SELECT casterGuid, spell, effectMask, recalculateMask, difficulty, stackCount, maxDuration, remainTime, remainCharges FROM pet_aura WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_PET_AURA_EFFECT.sql(),
        "SELECT casterGuid, spell, effectMask, effectIndex, amount, baseAmount FROM pet_aura_effect WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_PET_SPELL.sql(),
        "SELECT spell, active FROM pet_spell WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_PET_SPELL_COOLDOWN.sql(),
        "SELECT spell, time, categoryId, categoryEnd FROM pet_spell_cooldown WHERE guid = ? AND time > UNIX_TIMESTAMP()"
    );
    assert_eq!(
        CharStatements::SEL_PET_DECLINED_NAME.sql(),
        "SELECT genitive, dative, accusative, instrumental, prepositional FROM character_pet_declinedname WHERE owner = ? AND id = ?"
    );
    assert_eq!(
        CharStatements::DEL_PET_AURAS.sql(),
        "DELETE FROM pet_aura WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_PET_AURA_EFFECTS.sql(),
        "DELETE FROM pet_aura_effect WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_PET_SPELLS.sql(),
        "DELETE FROM pet_spell WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_PET_SPELL_COOLDOWNS.sql(),
        "DELETE FROM pet_spell_cooldown WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::INS_PET_SPELL_COOLDOWN.sql(),
        "INSERT INTO pet_spell_cooldown (guid, spell, time, categoryId, categoryEnd) VALUES (?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::SEL_PET_SPELL_CHARGES.sql(),
        "SELECT categoryId, rechargeStart, rechargeEnd FROM pet_spell_charges WHERE guid = ? AND rechargeEnd > UNIX_TIMESTAMP() ORDER BY rechargeEnd"
    );
    assert_eq!(
        CharStatements::DEL_PET_SPELL_CHARGES.sql(),
        "DELETE FROM pet_spell_charges WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::INS_PET_SPELL_CHARGES.sql(),
        "INSERT INTO pet_spell_charges (guid, categoryId, rechargeStart, rechargeEnd) VALUES (?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_PET_SPELL_BY_SPELL.sql(),
        "DELETE FROM pet_spell WHERE guid = ? and spell = ?"
    );
    assert_eq!(
        CharStatements::INS_PET_SPELL.sql(),
        "INSERT INTO pet_spell (guid, spell, active) VALUES (?, ?, ?)"
    );
    assert_eq!(
        CharStatements::INS_PET_AURA.sql(),
        "INSERT INTO pet_aura (guid, casterGuid, spell, effectMask, recalculateMask, difficulty, stackCount, maxDuration, remainTime, remainCharges) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::INS_PET_AURA_EFFECT.sql(),
        "INSERT INTO pet_aura_effect (guid, casterGuid, spell, effectMask, effectIndex, amount, baseAmount) VALUES (?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::SEL_CHAR_PETS.sql(),
        "SELECT id, entry, modelid, level, exp, Reactstate, slot, name, renamed, curhealth, curmana, abdata, savetime, CreatedBySpell, PetType, specialization FROM character_pet WHERE owner = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_PET_BY_OWNER.sql(),
        "DELETE FROM character_pet WHERE owner = ?"
    );
    assert_eq!(
        CharStatements::UPD_CHAR_PET_NAME.sql(),
        "UPDATE character_pet SET name = ?, renamed = 1 WHERE owner = ? AND id = ?"
    );
    assert_eq!(
        CharStatements::UPD_CHAR_PET_SLOT_BY_ID.sql(),
        "UPDATE character_pet SET slot = ? WHERE owner = ? AND id = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_PET_BY_ID.sql(),
        "DELETE FROM character_pet WHERE id = ?"
    );
    assert_eq!(
        CharStatements::DEL_ALL_PET_SPELLS_BY_OWNER.sql(),
        "DELETE FROM pet_spell WHERE guid in (SELECT id FROM character_pet WHERE owner=?)"
    );
    assert_eq!(
        CharStatements::UPD_PET_SPECS_BY_OWNER.sql(),
        "UPDATE character_pet SET specialization = 0 WHERE owner=?"
    );
    assert_eq!(
        CharStatements::INS_PET.sql(),
        "INSERT INTO character_pet (id, entry, owner, modelid, level, exp, Reactstate, slot, name, renamed, curhealth, curmana, abdata, savetime, CreatedBySpell, PetType, specialization) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(CharStatements::INS_PET.sql().matches('?').count(), 17);
    assert_eq!(
        CharStatements::SEL_PVPSTATS_MAXID.sql(),
        "SELECT MAX(id) FROM pvpstats_battlegrounds"
    );
    assert_eq!(
        CharStatements::INS_PVPSTATS_BATTLEGROUND.sql(),
        "INSERT INTO pvpstats_battlegrounds (id, winner_faction, bracket_id, type, date) VALUES (?, ?, ?, ?, NOW())"
    );
    assert_eq!(
        CharStatements::INS_PVPSTATS_PLAYER.sql(),
        "INSERT INTO pvpstats_players (battleground_id, character_guid, winner, score_killing_blows, score_deaths, score_honorable_kills, score_bonus_honor, score_damage_done, score_healing_done, attr_1, attr_2, attr_3, attr_4, attr_5) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::SEL_PVPSTATS_FACTIONS_OVERALL.sql(),
        "SELECT winner_faction, COUNT(*) AS count FROM pvpstats_battlegrounds WHERE DATEDIFF(NOW(), date) < 7 GROUP BY winner_faction ORDER BY winner_faction ASC"
    );
    assert_eq!(
        CharStatements::INS_QUEST_TRACK.sql(),
        "INSERT INTO quest_tracker (id, character_guid, quest_accept_time, core_hash, core_revision) VALUES (?, ?, NOW(), ?, ?)"
    );
    assert_eq!(
        CharStatements::UPD_QUEST_TRACK_GM_COMPLETE.sql(),
        "UPDATE quest_tracker SET completed_by_gm = 1 WHERE id = ? AND character_guid = ? ORDER BY quest_accept_time DESC LIMIT 1"
    );
    assert_eq!(
        CharStatements::UPD_QUEST_TRACK_COMPLETE_TIME.sql(),
        "UPDATE quest_tracker SET quest_complete_time = NOW() WHERE id = ? AND character_guid = ? ORDER BY quest_accept_time DESC LIMIT 1"
    );
    assert_eq!(
        CharStatements::UPD_QUEST_TRACK_ABANDON_TIME.sql(),
        "UPDATE quest_tracker SET quest_abandon_time = NOW() WHERE id = ? AND character_guid = ? ORDER BY quest_accept_time DESC LIMIT 1"
    );
    assert_eq!(
        CharStatements::SEL_CHARACTER_AURA_STORED_LOCATIONS.sql(),
        "SELECT Spell, MapId, PositionX, PositionY, PositionZ, Orientation FROM character_aura_stored_location WHERE Guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHARACTER_AURA_STORED_LOCATIONS_BY_GUID.sql(),
        "DELETE FROM character_aura_stored_location WHERE Guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHARACTER_AURA_STORED_LOCATION.sql(),
        "DELETE FROM character_aura_stored_location WHERE Guid = ? AND Spell = ?"
    );
    assert_eq!(
        CharStatements::INS_CHARACTER_AURA_STORED_LOCATION.sql(),
        "INSERT INTO character_aura_stored_location (Guid, Spell, MapId, PositionX, PositionY, PositionZ, Orientation) VALUES (?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::SEL_WAR_MODE_TUNING.sql(),
        "SELECT race, COUNT(guid) FROM characters WHERE ((playerFlags & ?) = ?) AND logout_time >= (UNIX_TIMESTAMP() - 604800) GROUP BY race"
    );
}

#[test]
fn character_select_item_instance_content_aliases_match_cpp_expansion_exactly() {
    let cpp_sql = cpp_character_sql();
    let aliases = [
        CharStatements::SEL_CHARACTER_INVENTORY,
        CharStatements::SEL_MAILITEMS,
        CharStatements::SEL_AUCTION_ITEMS,
        CharStatements::SEL_GUILD_BANK_ITEMS,
    ];

    for statement in aliases {
        assert!(
            cpp_sql.iter().any(|sql| sql == statement.sql()),
            "{} must match expanded C++ SelectItemInstanceContent SQL",
            statement.sql()
        );
        assert!(
            statement.sql().contains(
                "iit.secondaryItemModifiedAppearanceSpec4, iit.itemModifiedAppearanceSpec5"
            ),
            "port preserves the exact C++ macro tail, including the suspicious Spec5 column"
        );
    }

    assert_eq!(
        CharStatements::SEL_CHARACTER_INVENTORY
            .sql()
            .matches('?')
            .count(),
        1
    );
    assert_eq!(CharStatements::SEL_MAILITEMS.sql().matches('?').count(), 1);
    assert_eq!(
        CharStatements::SEL_AUCTION_ITEMS.sql().matches('?').count(),
        0
    );
    assert_eq!(
        CharStatements::SEL_GUILD_BANK_ITEMS
            .sql()
            .matches('?')
            .count(),
        0
    );
    assert!(
        CharStatements::SEL_CHARACTER_INVENTORY
            .sql()
            .contains(", bag, slot FROM character_inventory")
    );
    assert!(
        CharStatements::SEL_MAILITEMS
            .sql()
            .contains(", ii.owner_guid, m.id FROM mail_items")
    );
    assert!(
        CharStatements::SEL_AUCTION_ITEMS
            .sql()
            .contains(", ii.owner_guid, ai.auctionId FROM auction_items")
    );
    assert!(
        CharStatements::SEL_GUILD_BANK_ITEMS
            .sql()
            .contains(", guildid, TabId, SlotId FROM guild_bank_item")
    );
}

#[test]
fn gm_ticket_and_lfg_statements_match_cpp_sql_exactly() {
    assert_eq!(
        CharStatements::SEL_GM_BUGS.sql(),
        "SELECT id, playerGuid, note, createTime, mapId, posX, posY, posZ, facing, closedBy, assignedTo, comment FROM gm_bug"
    );
    assert_eq!(
        CharStatements::REP_GM_BUG.sql(),
        "REPLACE INTO gm_bug (id, playerGuid, note, createTime, mapId, posX, posY, posZ, facing, closedBy, assignedTo, comment) VALUES (?, ?, ?, UNIX_TIMESTAMP(NOW()), ?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_GM_BUG.sql(),
        "DELETE FROM gm_bug WHERE id = ?"
    );
    assert_eq!(CharStatements::DEL_ALL_GM_BUGS.sql(), "DELETE FROM gm_bug");
    assert_eq!(
        CharStatements::SEL_GM_COMPLAINTS.sql(),
        "SELECT id, playerGuid, note, createTime, mapId, posX, posY, posZ, facing, targetCharacterGuid, reportType, reportMajorCategory, reportMinorCategoryFlags, reportLineIndex, assignedTo, closedBy, comment FROM gm_complaint"
    );
    assert_eq!(
        CharStatements::REP_GM_COMPLAINT.sql(),
        "REPLACE INTO gm_complaint (id, playerGuid, note, createTime, mapId, posX, posY, posZ, facing, targetCharacterGuid, reportType, reportMajorCategory, reportMinorCategoryFlags, reportLineIndex, assignedTo, closedBy, comment) VALUES (?, ?, ?, UNIX_TIMESTAMP(NOW()), ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_GM_COMPLAINT.sql(),
        "DELETE FROM gm_complaint WHERE id = ?"
    );
    assert_eq!(
        CharStatements::SEL_GM_COMPLAINT_CHATLINES.sql(),
        "SELECT timestamp, text FROM gm_complaint_chatlog WHERE complaintId = ? ORDER BY lineId ASC"
    );
    assert_eq!(
        CharStatements::INS_GM_COMPLAINT_CHATLINE.sql(),
        "INSERT INTO gm_complaint_chatlog (complaintId, lineId, timestamp, text) VALUES (?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_GM_COMPLAINT_CHATLOG.sql(),
        "DELETE FROM gm_complaint_chatlog WHERE complaintId = ?"
    );
    assert_eq!(
        CharStatements::DEL_ALL_GM_COMPLAINTS.sql(),
        "DELETE FROM gm_complaint"
    );
    assert_eq!(
        CharStatements::DEL_ALL_GM_COMPLAINT_CHATLOGS.sql(),
        "DELETE FROM gm_complaint_chatlog"
    );
    assert_eq!(
        CharStatements::SEL_GM_SUGGESTIONS.sql(),
        "SELECT id, playerGuid, note, createTime, mapId, posX, posY, posZ, facing, closedBy, assignedTo, comment FROM gm_suggestion"
    );
    assert_eq!(
        CharStatements::REP_GM_SUGGESTION.sql(),
        "REPLACE INTO gm_suggestion (id, playerGuid, note, createTime, mapId, posX, posY, posZ, facing, closedBy, assignedTo, comment) VALUES (?, ?, ?, UNIX_TIMESTAMP(NOW()), ?, ?, ?, ?, ?, ? ,? ,?)"
    );
    assert_eq!(
        CharStatements::DEL_GM_SUGGESTION.sql(),
        "DELETE FROM gm_suggestion WHERE id = ?"
    );
    assert_eq!(
        CharStatements::DEL_ALL_GM_SUGGESTIONS.sql(),
        "DELETE FROM gm_suggestion"
    );
    assert_eq!(
        CharStatements::INS_LFG_DATA.sql(),
        "INSERT INTO lfg_data (guid, dungeon, state) VALUES (?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_LFG_DATA.sql(),
        "DELETE FROM lfg_data WHERE guid = ?"
    );
}

#[test]
fn seasonal_quest_status_load_statement_matches_cpp_sql_exactly() {
    assert_eq!(
        CharStatements::SEL_CHAR_QUEST_STATUS_SEASONAL.sql(),
        "SELECT quest, event, completedTime FROM character_queststatus_seasonal WHERE guid = ?"
    );
}

#[test]
fn quest_status_load_statement_matches_cpp_sql_exactly() {
    assert_eq!(
        CharStatements::SEL_CHAR_QUEST_STATUS.sql(),
        "SELECT quest, status, explored, acceptTime, endTime FROM character_queststatus WHERE guid = ? AND status <> 0"
    );
    assert_eq!(
        CharStatements::SEL_CHAR_QUEST_STATUS_OBJECTIVES.sql(),
        "SELECT quest, objective, data FROM character_queststatus_objectives WHERE guid = ?"
    );
}

#[test]
fn quest_status_objective_save_statements_match_cpp_sql_exactly() {
    assert_eq!(
        CharStatements::DEL_CHAR_QUEST_STATUS_OBJECTIVES_BY_QUEST.sql(),
        "DELETE FROM character_queststatus_objectives WHERE guid = ? AND quest = ?"
    );
    assert_eq!(
        CharStatements::REP_CHAR_QUEST_STATUS_OBJECTIVES.sql(),
        "REPLACE INTO character_queststatus_objectives (guid, quest, objective, data) VALUES (?, ?, ?, ?)"
    );
}

#[test]
fn quest_status_save_statement_matches_cpp_replace_sql_exactly() {
    let sql = CharStatements::INS_CHAR_QUEST_STATUS.sql();
    assert_eq!(
        sql,
        "REPLACE INTO character_queststatus (guid, quest, status, explored, acceptTime, endTime) VALUES (?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(sql.matches('?').count(), 6);
}

#[test]
fn inventory_item_replace_statement_matches_cpp_sql_exactly() {
    let sql = CharStatements::REP_CHAR_INVENTORY_ITEM.sql();
    assert_eq!(
        sql,
        "REPLACE INTO character_inventory (guid, bag, slot, item) VALUES (?, ?, ?, ?)"
    );
    assert_eq!(sql.matches('?').count(), 4);
}

#[test]
fn item_trade_and_persistence_statements_match_cpp_sql_exactly() {
    assert_eq!(
        CharStatements::SEL_ITEM_REFUNDS.sql(),
        "SELECT paidMoney, paidExtendedCost FROM item_refund_instance WHERE item_guid = ? AND player_guid = ? LIMIT 1"
    );
    assert_eq!(
        CharStatements::SEL_ITEM_BOP_TRADE.sql(),
        "SELECT allowedPlayers FROM item_soulbound_trade_data WHERE itemGuid = ? LIMIT 1"
    );
    assert_eq!(
        CharStatements::DEL_ITEM_BOP_TRADE.sql(),
        "DELETE FROM item_soulbound_trade_data WHERE itemGuid = ? LIMIT 1"
    );
    assert_eq!(
        CharStatements::INS_ITEM_BOP_TRADE.sql(),
        "INSERT INTO item_soulbound_trade_data VALUES (?, ?)"
    );
    assert_eq!(
        CharStatements::REP_INVENTORY_ITEM.sql(),
        "REPLACE INTO character_inventory (guid, bag, slot, item) VALUES (?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::REP_ITEM_INSTANCE.sql(),
        "REPLACE INTO item_instance (itemEntry, owner_guid, creatorGuid, giftCreatorGuid, count, duration, charges, flags, enchantments, durability, playedTime, text, battlePetSpeciesId, battlePetBreedData, battlePetLevel, battlePetDisplayId, randomPropertiesId, randomPropertiesSeed, context, guid) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::UPD_ITEM_INSTANCE.sql(),
        "UPDATE item_instance SET itemEntry = ?, owner_guid = ?, creatorGuid = ?, giftCreatorGuid = ?, count = ?, duration = ?, charges = ?, flags = ?, enchantments = ?, durability = ?, playedTime = ?, text = ?, battlePetSpeciesId = ?, battlePetBreedData = ?, battlePetLevel = ?, battlePetDisplayId = ?, randomPropertiesId = ?, randomPropertiesSeed = ?, context = ? WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_ITEM_INSTANCE_ON_LOAD.sql(),
        "UPDATE item_instance SET duration = ?, flags = ?, durability = ? WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_ITEM_INSTANCE_BY_OWNER.sql(),
        "DELETE FROM item_instance WHERE owner_guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_ITEM_INSTANCE_BY_GUID_AND_OWNER.sql(),
        "DELETE FROM item_instance WHERE guid = ? AND owner_guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHAR_INVENTORY_ITEM_BY_OWNER.sql(),
        "DELETE ci FROM character_inventory ci INNER JOIN item_instance ii ON ii.guid = ci.item WHERE ci.guid = ? AND ci.item = ? AND ii.owner_guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_UNCAGE_ITEM_STATE.sql(),
        "SELECT (SELECT owner_guid FROM item_instance WHERE guid = ? LIMIT 1), EXISTS(SELECT 1 FROM character_inventory WHERE guid = ? AND item = ?)"
    );
}

#[test]
fn battle_pet_purchase_saga_statements_match_their_contract_exactly() {
    assert_eq!(
        CharStatements::INS_BATTLE_PET_PURCHASE.sql(),
        "INSERT INTO character_battle_pet_purchase (request_key, guid, account_id, trainer_id, spell_id, species, breed, quality, display_id, level, price, money_before, money_after, status) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::SEL_BATTLE_PET_PURCHASE_BY_KEY.sql(),
        "SELECT request_key, guid, account_id, trainer_id, spell_id, species, breed, quality, display_id, level, price, money_before, money_after, status, failure_reason, published FROM character_battle_pet_purchase WHERE request_key = ?"
    );
    assert_eq!(
        CharStatements::SEL_BATTLE_PET_PURCHASE_PENDING.sql(),
        "SELECT request_key, guid, account_id, trainer_id, spell_id, species, breed, quality, display_id, level, price, money_before, money_after, status, failure_reason, published FROM character_battle_pet_purchase WHERE guid = ? AND (status IN (0, 2) OR (status = 1 AND published = 0)) ORDER BY created_at ASC, request_key ASC LIMIT ?"
    );
    assert_eq!(
        CharStatements::UPD_BATTLE_PET_PURCHASE_PUBLISHED.sql(),
        "UPDATE character_battle_pet_purchase SET published = 1 WHERE request_key = ? AND published = 0 AND status IN (0, 1, 2)"
    );
    assert_eq!(
        CharStatements::UPD_BATTLE_PET_PURCHASE_COMPLETED.sql(),
        "UPDATE character_battle_pet_purchase SET status = 1, failure_reason = NULL WHERE request_key = ? AND status IN (0, 2)"
    );
    assert_eq!(
        CharStatements::UPD_BATTLE_PET_PURCHASE_COMPENSATION_PENDING.sql(),
        "UPDATE character_battle_pet_purchase SET status = 2, failure_reason = ? WHERE request_key = ? AND status = 0"
    );
    assert_eq!(
        CharStatements::UPD_BATTLE_PET_PURCHASE_COMPENSATED.sql(),
        "UPDATE character_battle_pet_purchase SET status = 3 WHERE request_key = ? AND status = 2"
    );
    assert_eq!(
        CharStatements::UPD_BATTLE_PET_PURCHASE_TERMINAL_FAILURE.sql(),
        "UPDATE character_battle_pet_purchase SET status = 4, failure_reason = ? WHERE request_key = ? AND status = 2"
    );
    assert_eq!(
        CharStatements::UPD_CHARACTER_MONEY_GUARDED.sql(),
        "UPDATE characters SET money = ? WHERE guid = ? AND money = ?"
    );
    assert_eq!(
        CharStatements::UPD_CHARACTER_MONEY_REFUND.sql(),
        "UPDATE characters SET money = LEAST(money + ?, ?) WHERE guid = ?"
    );
}

#[test]
fn item_gem_transmog_and_character_transfer_statements_match_cpp_sql_exactly() {
    assert_eq!(
        CharStatements::INS_ITEM_INSTANCE_GEMS.sql(),
        "INSERT INTO item_instance_gems (itemGuid, gemItemId1, gemBonuses1, gemContext1, gemItemId2, gemBonuses2, gemContext2, gemItemId3, gemBonuses3, gemContext3) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_ITEM_INSTANCE_GEMS.sql(),
        "DELETE FROM item_instance_gems WHERE itemGuid = ?"
    );
    assert_eq!(
        CharStatements::DEL_ITEM_INSTANCE_GEMS_BY_OWNER.sql(),
        "DELETE iig FROM item_instance_gems iig LEFT JOIN item_instance ii ON iig.itemGuid = ii.guid WHERE ii.owner_guid = ?"
    );
    assert_eq!(
        CharStatements::INS_ITEM_INSTANCE_TRANSMOG.sql(),
        "INSERT INTO item_instance_transmog (itemGuid, itemModifiedAppearanceAllSpecs, itemModifiedAppearanceSpec1, itemModifiedAppearanceSpec2, itemModifiedAppearanceSpec3, itemModifiedAppearanceSpec4, itemModifiedAppearanceSpec5, spellItemEnchantmentAllSpecs, spellItemEnchantmentSpec1, spellItemEnchantmentSpec2, spellItemEnchantmentSpec3, spellItemEnchantmentSpec4, spellItemEnchantmentSpec5, secondaryItemModifiedAppearanceAllSpecs, secondaryItemModifiedAppearanceSpec1, secondaryItemModifiedAppearanceSpec2, secondaryItemModifiedAppearanceSpec3, secondaryItemModifiedAppearanceSpec4, secondaryItemModifiedAppearanceSpec5) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_ITEM_INSTANCE_TRANSMOG.sql(),
        "DELETE FROM item_instance_transmog WHERE itemGuid = ?"
    );
    assert_eq!(
        CharStatements::DEL_ITEM_INSTANCE_TRANSMOG_BY_OWNER.sql(),
        "DELETE iit FROM item_instance_transmog iit LEFT JOIN item_instance ii ON iit.itemGuid = ii.guid WHERE ii.owner_guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_GIFT_OWNER.sql(),
        "UPDATE character_gifts SET guid = ? WHERE item_guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_ACCOUNT_BY_NAME.sql(),
        "SELECT account FROM characters WHERE name = ?"
    );
    assert_eq!(
        CharStatements::UPD_ACCOUNT_BY_GUID.sql(),
        "UPDATE characters SET account = ? WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_MATCH_MAKER_RATING.sql(),
        "SELECT matchMakerRating FROM character_arena_stats WHERE guid = ? AND slot = ?"
    );
    assert_eq!(
        CharStatements::SEL_CHARACTER_COUNT.sql(),
        "SELECT account, COUNT(guid) FROM characters WHERE account = ? GROUP BY account"
    );
    assert_eq!(
        CharStatements::UPD_NAME_BY_GUID.sql(),
        "UPDATE characters SET name = ? WHERE guid = ?"
    );
}

#[test]
fn guild_core_and_rank_statements_match_cpp_sql_exactly() {
    assert_eq!(
        CharStatements::INS_GUILD.sql(),
        "INSERT INTO guild (guildid, name, leaderguid, info, motd, createdate, EmblemStyle, EmblemColor, BorderStyle, BorderColor, BackgroundColor, BankMoney) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_GUILD.sql(),
        "DELETE FROM guild WHERE guildid = ?"
    );
    assert_eq!(
        CharStatements::UPD_GUILD_NAME.sql(),
        "UPDATE guild SET name = ? WHERE guildid = ?"
    );
    assert_eq!(
        CharStatements::INS_GUILD_MEMBER.sql(),
        "INSERT INTO guild_member (guildid, guid, `rank`, pnote, offnote) VALUES (?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_GUILD_MEMBER.sql(),
        "DELETE FROM guild_member WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_GUILD_MEMBERS.sql(),
        "DELETE FROM guild_member WHERE guildid = ?"
    );
    assert_eq!(
        CharStatements::INS_GUILD_RANK.sql(),
        "INSERT INTO guild_rank (guildid, rid, RankOrder, rname, rights, BankMoneyPerDay) VALUES (?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_GUILD_RANKS.sql(),
        "DELETE FROM guild_rank WHERE guildid = ?"
    );
    assert_eq!(
        CharStatements::DEL_GUILD_RANK.sql(),
        "DELETE FROM guild_rank WHERE guildid = ? AND rid = ?"
    );
}

#[test]
fn guild_bank_and_log_statements_match_cpp_sql_exactly() {
    assert_eq!(
        CharStatements::INS_GUILD_BANK_TAB.sql(),
        "INSERT INTO guild_bank_tab (guildid, TabId) VALUES (?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_GUILD_BANK_TAB.sql(),
        "DELETE FROM guild_bank_tab WHERE guildid = ? AND TabId = ?"
    );
    assert_eq!(
        CharStatements::DEL_GUILD_BANK_TABS.sql(),
        "DELETE FROM guild_bank_tab WHERE guildid = ?"
    );
    assert_eq!(
        CharStatements::INS_GUILD_BANK_ITEM.sql(),
        "INSERT INTO guild_bank_item (guildid, TabId, SlotId, item_guid) VALUES (?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_GUILD_BANK_ITEM.sql(),
        "DELETE FROM guild_bank_item WHERE guildid = ? AND TabId = ? AND SlotId = ?"
    );
    assert_eq!(
        CharStatements::DEL_GUILD_BANK_ITEMS.sql(),
        "DELETE FROM guild_bank_item WHERE guildid = ?"
    );
    assert_eq!(
        CharStatements::INS_GUILD_BANK_RIGHT.sql(),
        "INSERT INTO guild_bank_right (guildid, TabId, rid, gbright, SlotPerDay) VALUES (?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE gbright = VALUES(gbright), SlotPerDay = VALUES(SlotPerDay)"
    );
    assert_eq!(
        CharStatements::DEL_GUILD_BANK_RIGHTS.sql(),
        "DELETE FROM guild_bank_right WHERE guildid = ?"
    );
    assert_eq!(
        CharStatements::DEL_GUILD_BANK_RIGHTS_FOR_RANK.sql(),
        "DELETE FROM guild_bank_right WHERE guildid = ? AND rid = ?"
    );
    assert_eq!(
        CharStatements::INS_GUILD_BANK_EVENTLOG.sql(),
        "INSERT INTO guild_bank_eventlog (guildid, LogGuid, TabId, EventType, PlayerGuid, ItemOrMoney, ItemStackCount, DestTabId, TimeStamp) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_GUILD_BANK_EVENTLOG.sql(),
        "DELETE FROM guild_bank_eventlog WHERE guildid = ? AND LogGuid = ? AND TabId = ?"
    );
    assert_eq!(
        CharStatements::DEL_GUILD_BANK_EVENTLOGS.sql(),
        "DELETE FROM guild_bank_eventlog WHERE guildid = ?"
    );
    assert_eq!(
        CharStatements::INS_GUILD_EVENTLOG.sql(),
        "INSERT INTO guild_eventlog (guildid, LogGuid, EventType, PlayerGuid1, PlayerGuid2, NewRank, TimeStamp) VALUES (?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_GUILD_EVENTLOG.sql(),
        "DELETE FROM guild_eventlog WHERE guildid = ? AND LogGuid = ?"
    );
    assert_eq!(
        CharStatements::DEL_GUILD_EVENTLOGS.sql(),
        "DELETE FROM guild_eventlog WHERE guildid = ?"
    );
}

#[test]
fn guild_update_and_withdraw_statements_match_cpp_sql_exactly() {
    assert_eq!(
        CharStatements::UPD_GUILD_MEMBER_PNOTE.sql(),
        "UPDATE guild_member SET pnote = ? WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_GUILD_MEMBER_OFFNOTE.sql(),
        "UPDATE guild_member SET offnote = ? WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_GUILD_MEMBER_RANK.sql(),
        "UPDATE guild_member SET `rank` = ? WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_GUILD_MOTD.sql(),
        "UPDATE guild SET motd = ? WHERE guildid = ?"
    );
    assert_eq!(
        CharStatements::UPD_GUILD_INFO.sql(),
        "UPDATE guild SET info = ? WHERE guildid = ?"
    );
    assert_eq!(
        CharStatements::UPD_GUILD_LEADER.sql(),
        "UPDATE guild SET leaderguid = ? WHERE guildid = ?"
    );
    assert_eq!(
        CharStatements::UPD_GUILD_RANK_ORDER.sql(),
        "UPDATE guild_rank SET RankOrder = ? WHERE rid = ? AND guildid = ?"
    );
    assert_eq!(
        CharStatements::UPD_GUILD_RANK_NAME.sql(),
        "UPDATE guild_rank SET rname = ? WHERE rid = ? AND guildid = ?"
    );
    assert_eq!(
        CharStatements::UPD_GUILD_RANK_RIGHTS.sql(),
        "UPDATE guild_rank SET rights = ? WHERE rid = ? AND guildid = ?"
    );
    assert_eq!(
        CharStatements::UPD_GUILD_EMBLEM_INFO.sql(),
        "UPDATE guild SET EmblemStyle = ?, EmblemColor = ?, BorderStyle = ?, BorderColor = ?, BackgroundColor = ? WHERE guildid = ?"
    );
    assert_eq!(
        CharStatements::UPD_GUILD_BANK_TAB_INFO.sql(),
        "UPDATE guild_bank_tab SET TabName = ?, TabIcon = ? WHERE guildid = ? AND TabId = ?"
    );
    assert_eq!(
        CharStatements::UPD_GUILD_BANK_MONEY.sql(),
        "UPDATE guild SET BankMoney = ? WHERE guildid = ?"
    );
    assert_eq!(
        CharStatements::UPD_GUILD_RANK_BANK_MONEY.sql(),
        "UPDATE guild_rank SET BankMoneyPerDay = ? WHERE rid = ? AND guildid = ?"
    );
    assert_eq!(
        CharStatements::UPD_GUILD_BANK_TAB_TEXT.sql(),
        "UPDATE guild_bank_tab SET TabText = ? WHERE guildid = ? AND TabId = ?"
    );
    assert_eq!(
        CharStatements::INS_GUILD_MEMBER_WITHDRAW_TABS.sql(),
        "INSERT INTO guild_member_withdraw (guid, tab0, tab1, tab2, tab3, tab4, tab5, tab6, tab7) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE tab0 = VALUES (tab0), tab1 = VALUES (tab1), tab2 = VALUES (tab2), tab3 = VALUES (tab3), tab4 = VALUES (tab4), tab5 = VALUES (tab5), tab6 = VALUES (tab6), tab7 = VALUES (tab7)"
    );
    assert_eq!(
        CharStatements::INS_GUILD_MEMBER_WITHDRAW_MONEY.sql(),
        "INSERT INTO guild_member_withdraw (guid, money) VALUES (?, ?) ON DUPLICATE KEY UPDATE money = VALUES (money)"
    );
    assert_eq!(
        CharStatements::DEL_GUILD_MEMBER_WITHDRAW.sql(),
        "DELETE FROM guild_member_withdraw"
    );
}

#[test]
fn guild_achievement_and_news_statements_match_cpp_sql_exactly() {
    assert_eq!(
        CharStatements::SEL_CHAR_DATA_FOR_GUILD.sql(),
        "SELECT name, level, race, class, gender, zone, account FROM characters WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_GUILD_ACHIEVEMENT.sql(),
        "DELETE FROM guild_achievement WHERE guildId = ? AND achievement = ?"
    );
    assert_eq!(
        CharStatements::INS_GUILD_ACHIEVEMENT.sql(),
        "INSERT INTO guild_achievement (guildId, achievement, date, guids) VALUES (?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_GUILD_ACHIEVEMENT_CRITERIA.sql(),
        "DELETE FROM guild_achievement_progress WHERE guildId = ? AND criteria = ?"
    );
    assert_eq!(
        CharStatements::INS_GUILD_ACHIEVEMENT_CRITERIA.sql(),
        "INSERT INTO guild_achievement_progress (guildId, criteria, counter, date, completedGuid) VALUES (?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_ALL_GUILD_ACHIEVEMENTS.sql(),
        "DELETE FROM guild_achievement WHERE guildId = ? AND achievement NOT IN (5407,5408,5409,5410,5411,5985,6126,6628,6678,6679,6680,8257,8512,8513,9397,9399,10380)"
    );
    assert_eq!(
        CharStatements::DEL_ALL_GUILD_ACHIEVEMENT_CRITERIA.sql(),
        "DELETE FROM guild_achievement_progress WHERE guildId = ?"
    );
    assert_eq!(
        CharStatements::SEL_GUILD_ACHIEVEMENT.sql(),
        "SELECT achievement, date, guids FROM guild_achievement WHERE guildId = ?"
    );
    assert_eq!(
        CharStatements::SEL_GUILD_ACHIEVEMENT_CRITERIA.sql(),
        "SELECT criteria, counter, date, completedGuid FROM guild_achievement_progress WHERE guildId = ?"
    );
    assert_eq!(
        CharStatements::INS_GUILD_NEWS.sql(),
        "INSERT INTO guild_newslog (guildid, LogGuid, EventType, PlayerGuid, Flags, Value, Timestamp) VALUES (?, ?, ?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE LogGuid = VALUES (LogGuid), EventType = VALUES (EventType), PlayerGuid = VALUES (PlayerGuid), Flags = VALUES (Flags), Value = VALUES (Value), Timestamp = VALUES (Timestamp)"
    );
}

#[test]
fn channel_equipment_transmog_aura_statements_match_cpp_sql_exactly() {
    assert_eq!(
        CharStatements::UPD_CHANNEL.sql(),
        "INSERT INTO channels (name, team, announce, ownership, password, bannedList, lastUsed) VALUES (?, ?, ?, ?, ?, ?, UNIX_TIMESTAMP()) ON DUPLICATE KEY UPDATE announce=VALUES(announce), ownership=VALUES(ownership), password=VALUES(password), bannedList=VALUES(bannedList), lastUsed=VALUES(lastUsed)"
    );
    assert_eq!(
        CharStatements::UPD_CHANNEL_USAGE.sql(),
        "UPDATE channels SET lastUsed = UNIX_TIMESTAMP() WHERE name = ? AND team = ?"
    );
    assert_eq!(
        CharStatements::UPD_CHANNEL_OWNERSHIP.sql(),
        "UPDATE channels SET ownership = ? WHERE name LIKE ?"
    );
    assert_eq!(
        CharStatements::DEL_CHANNEL.sql(),
        "DELETE FROM channels WHERE name = ? AND team = ?"
    );
    assert_eq!(
        CharStatements::DEL_OLD_CHANNELS.sql(),
        "DELETE FROM channels WHERE ownership = 1 AND lastUsed + ? < UNIX_TIMESTAMP()"
    );
    assert_eq!(
        CharStatements::UPD_EQUIP_SET.sql(),
        "UPDATE character_equipmentsets SET name=?, iconname=?, ignore_mask=?, AssignedSpecIndex=?, item0=?, item1=?, item2=?, item3=?, item4=?, item5=?, item6=?, item7=?, item8=?, item9=?, item10=?, item11=?, item12=?, item13=?, item14=?, item15=?, item16=?, item17=?, item18=? WHERE guid=? AND setguid=? AND setindex=?"
    );
    assert_eq!(
        CharStatements::INS_EQUIP_SET.sql(),
        "INSERT INTO character_equipmentsets (guid, setguid, setindex, name, iconname, ignore_mask, AssignedSpecIndex, item0, item1, item2, item3, item4, item5, item6, item7, item8, item9, item10, item11, item12, item13, item14, item15, item16, item17, item18) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_EQUIP_SET.sql(),
        "DELETE FROM character_equipmentsets WHERE setguid=?"
    );
    assert_eq!(
        CharStatements::UPD_TRANSMOG_OUTFIT.sql(),
        "UPDATE character_transmog_outfits SET name=?, iconname=?, ignore_mask=?, appearance0=?, appearance1=?, appearance2=?, appearance3=?, appearance4=?, appearance5=?, appearance6=?, appearance7=?, appearance8=?, appearance9=?, appearance10=?, appearance11=?, appearance12=?, appearance13=?, appearance14=?, appearance15=?, appearance16=?, appearance17=?, appearance18=?, mainHandEnchant=?, offHandEnchant=? WHERE guid=? AND setguid=? AND setindex=?"
    );
    assert_eq!(
        CharStatements::INS_TRANSMOG_OUTFIT.sql(),
        "INSERT INTO character_transmog_outfits (guid, setguid, setindex, name, iconname, ignore_mask, appearance0, appearance1, appearance2, appearance3, appearance4, appearance5, appearance6, appearance7, appearance8, appearance9, appearance10, appearance11, appearance12, appearance13, appearance14, appearance15, appearance16, appearance17, appearance18, mainHandEnchant, offHandEnchant) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_TRANSMOG_OUTFIT.sql(),
        "DELETE FROM character_transmog_outfits WHERE setguid=?"
    );
    assert_eq!(
        CharStatements::INS_AURA.sql(),
        "INSERT INTO character_aura (guid, casterGuid, itemGuid, spell, effectMask, recalculateMask, difficulty, stackCount, maxDuration, remainTime, remainCharges, castItemId, castItemLevel) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::INS_AURA_EFFECT.sql(),
        "INSERT INTO character_aura_effect (guid, casterGuid, itemGuid, spell, effectMask, effectIndex, amount, baseAmount) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    );
}

#[test]
fn currency_account_data_and_tutorial_statements_match_cpp_sql_exactly() {
    assert_eq!(
        CharStatements::DEL_PLAYER_CURRENCY.sql(),
        "DELETE FROM character_currency WHERE CharacterGuid = ?"
    );
    assert_eq!(
        CharStatements::SEL_ACCOUNT_DATA.sql(),
        "SELECT type, time, data FROM account_data WHERE accountId = ?"
    );
    assert_eq!(
        CharStatements::REP_ACCOUNT_DATA.sql(),
        "REPLACE INTO account_data (accountId, type, time, data) VALUES (?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_ACCOUNT_DATA.sql(),
        "DELETE FROM account_data WHERE accountId = ?"
    );
    assert_eq!(
        CharStatements::SEL_PLAYER_ACCOUNT_DATA.sql(),
        "SELECT type, time, data FROM character_account_data WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::REP_PLAYER_ACCOUNT_DATA.sql(),
        "REPLACE INTO character_account_data(guid, type, time, data) VALUES (?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_PLAYER_ACCOUNT_DATA.sql(),
        "DELETE FROM character_account_data WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::SEL_TUTORIALS.sql(),
        "SELECT tut0, tut1, tut2, tut3, tut4, tut5, tut6, tut7 FROM account_tutorial WHERE accountId = ?"
    );
    assert_eq!(
        CharStatements::INS_TUTORIALS.sql(),
        "INSERT INTO account_tutorial(tut0, tut1, tut2, tut3, tut4, tut5, tut6, tut7, accountId) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::UPD_TUTORIALS.sql(),
        "UPDATE account_tutorial SET tut0 = ?, tut1 = ?, tut2 = ?, tut3 = ?, tut4 = ?, tut5 = ?, tut6 = ?, tut7 = ? WHERE accountId = ?"
    );
    assert_eq!(
        CharStatements::DEL_TUTORIALS.sql(),
        "DELETE FROM account_tutorial WHERE accountId = ?"
    );
}

#[test]
fn petition_statements_match_cpp_sql_exactly() {
    assert_eq!(
        CharStatements::SEL_PETITION.sql(),
        "SELECT ownerguid, name FROM petition WHERE petitionguid = ?"
    );
    assert_eq!(
        CharStatements::SEL_PETITION_SIGNATURE.sql(),
        "SELECT playerguid FROM petition_sign WHERE petitionguid = ?"
    );
    assert_eq!(
        CharStatements::DEL_ALL_PETITION_SIGNATURES.sql(),
        "DELETE FROM petition_sign WHERE playerguid = ?"
    );
    assert_eq!(
        CharStatements::SEL_PETITION_BY_OWNER.sql(),
        "SELECT petitionguid FROM petition WHERE ownerguid = ?"
    );
    assert_eq!(
        CharStatements::SEL_PETITION_SIGNATURES.sql(),
        "SELECT ownerguid, (SELECT COUNT(playerguid) FROM petition_sign WHERE petition_sign.petitionguid = ?) AS signs FROM petition WHERE petitionguid = ?"
    );
    assert_eq!(
        CharStatements::SEL_PETITION_SIG_BY_ACCOUNT.sql(),
        "SELECT playerguid FROM petition_sign WHERE player_account = ? AND petitionguid = ?"
    );
    assert_eq!(
        CharStatements::SEL_PETITION_OWNER_BY_GUID.sql(),
        "SELECT ownerguid FROM petition WHERE petitionguid = ?"
    );
    assert_eq!(
        CharStatements::SEL_PETITION_SIG_BY_GUID.sql(),
        "SELECT ownerguid, petitionguid FROM petition_sign WHERE playerguid = ?"
    );
}

#[test]
fn arena_team_statements_match_cpp_sql_exactly() {
    assert_eq!(
        CharStatements::SEL_CHARACTER_ARENAINFO.sql(),
        "SELECT arenaTeamId, weekGames, seasonGames, seasonWins, personalRating FROM arena_team_member WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::INS_ARENA_TEAM.sql(),
        "INSERT INTO arena_team (arenaTeamId, name, captainGuid, type, rating, backgroundColor, emblemStyle, emblemColor, borderStyle, borderColor) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::INS_ARENA_TEAM_MEMBER.sql(),
        "INSERT INTO arena_team_member (arenaTeamId, guid, personalRating) VALUES (?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_ARENA_TEAM.sql(),
        "DELETE FROM arena_team where arenaTeamId = ?"
    );
    assert_eq!(
        CharStatements::DEL_ARENA_TEAM_MEMBERS.sql(),
        "DELETE FROM arena_team_member WHERE arenaTeamId = ?"
    );
    assert_eq!(
        CharStatements::UPD_ARENA_TEAM_CAPTAIN.sql(),
        "UPDATE arena_team SET captainGuid = ? WHERE arenaTeamId = ?"
    );
    assert_eq!(
        CharStatements::DEL_ARENA_TEAM_MEMBER.sql(),
        "DELETE FROM arena_team_member WHERE arenaTeamId = ? AND guid = ?"
    );
    assert_eq!(
        CharStatements::UPD_ARENA_TEAM_STATS.sql(),
        "UPDATE arena_team SET rating = ?, weekGames = ?, weekWins = ?, seasonGames = ?, seasonWins = ?, `rank` = ? WHERE arenaTeamId = ?"
    );
    assert_eq!(
        CharStatements::UPD_ARENA_TEAM_MEMBER.sql(),
        "UPDATE arena_team_member SET personalRating = ?, weekGames = ?, weekWins = ?, seasonGames = ?, seasonWins = ? WHERE arenaTeamId = ? AND guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CHARACTER_ARENA_STATS.sql(),
        "DELETE FROM character_arena_stats WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::REP_CHARACTER_ARENA_STATS.sql(),
        "REPLACE INTO character_arena_stats (guid, slot, matchMakerRating) VALUES (?, ?, ?)"
    );
    assert_eq!(
        CharStatements::UPD_ARENA_TEAM_NAME.sql(),
        "UPDATE arena_team SET name = ? WHERE arenaTeamId = ?"
    );
}

#[test]
fn equipment_set_guid_max_query_matches_cpp_shared_namespace() {
    assert_eq!(
        CharStatements::SEL_MAX_EQUIPMENT_SET_GUID.sql(),
        "SELECT CAST(MAX(maxguid) AS UNSIGNED) FROM ((SELECT MAX(setguid) AS maxguid FROM character_equipmentsets) UNION (SELECT MAX(setguid) AS maxguid FROM character_transmog_outfits)) allsets"
    );
}

#[test]
fn void_storage_item_id_max_query_matches_cpp() {
    assert_eq!(
        CharStatements::SEL_MAX_VOID_STORAGE_ITEM_ID.sql(),
        "SELECT MAX(itemId) FROM character_void_storage"
    );
}

#[test]
fn battleground_and_homebind_statements_match_cpp_sql_exactly() {
    assert_eq!(
        CharStatements::INS_PLAYER_BGDATA.sql(),
        "INSERT INTO character_battleground_data (guid, instanceId, team, joinX, joinY, joinZ, joinO, joinMapId, taxiStart, taxiEnd, mountSpell, queueId) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_PLAYER_BGDATA.sql(),
        "DELETE FROM character_battleground_data WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::INS_PLAYER_HOMEBIND.sql(),
        "INSERT INTO character_homebind (guid, mapId, zoneId, posX, posY, posZ, orientation) VALUES (?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::UPD_PLAYER_HOMEBIND.sql(),
        "UPDATE character_homebind SET mapId = ?, zoneId = ?, posX = ?, posY = ?, posZ = ?, orientation = ? WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_PLAYER_HOMEBIND.sql(),
        "DELETE FROM character_homebind WHERE guid = ?"
    );
}

#[test]
fn corpse_statements_match_cpp_sql_exactly() {
    assert_eq!(
        CharStatements::SEL_CORPSES.sql(),
        "SELECT posX, posY, posZ, orientation, mapId, displayId, itemCache, race, class, gender, flags, dynFlags, time, corpseType, instanceId, guid FROM corpse WHERE mapId = ? AND instanceId = ?"
    );
    assert_eq!(
        CharStatements::INS_CORPSE.sql(),
        "INSERT INTO corpse (guid, posX, posY, posZ, orientation, mapId, displayId, itemCache, race, class, gender, flags, dynFlags, time, corpseType, instanceId) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_CORPSE.sql(),
        "DELETE FROM corpse WHERE guid = ?"
    );
    assert_eq!(
        CharStatements::DEL_CORPSES_FROM_MAP.sql(),
        "DELETE c, cc, cp FROM corpse c LEFT JOIN corpse_customizations cc ON c.guid = cc.ownerGuid LEFT JOIN corpse_phases cp ON c.guid = cp.OwnerGuid WHERE c.mapId = ? AND c.instanceId = ?"
    );
    assert_eq!(
        CharStatements::SEL_CORPSE_PHASES.sql(),
        "SELECT cp.OwnerGuid, cp.PhaseId FROM corpse_phases cp LEFT JOIN corpse c ON cp.OwnerGuid = c.guid WHERE c.mapId = ? AND c.instanceId = ?"
    );
    assert_eq!(
        CharStatements::INS_CORPSE_PHASES.sql(),
        "INSERT INTO corpse_phases (OwnerGuid, PhaseId) VALUES (?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_CORPSE_PHASES.sql(),
        "DELETE FROM corpse_phases WHERE OwnerGuid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CORPSE_CUSTOMIZATIONS.sql(),
        "SELECT cc.ownerGuid, cc.chrCustomizationOptionID, cc.chrCustomizationChoiceID FROM corpse_customizations cc LEFT JOIN corpse c ON cc.ownerGuid = c.guid WHERE c.mapId = ? AND c.instanceId = ? ORDER BY cc.ownerGuid, cc.chrCustomizationOptionID"
    );
    assert_eq!(
        CharStatements::INS_CORPSE_CUSTOMIZATIONS.sql(),
        "INSERT INTO corpse_customizations (ownerGuid, chrCustomizationOptionID, chrCustomizationChoiceID) VALUES (?, ?, ?)"
    );
    assert_eq!(
        CharStatements::DEL_CORPSE_CUSTOMIZATIONS.sql(),
        "DELETE FROM corpse_customizations WHERE ownerGuid = ?"
    );
    assert_eq!(
        CharStatements::SEL_CORPSE_LOCATION.sql(),
        "SELECT mapId, posX, posY, posZ, orientation FROM corpse WHERE guid = ?"
    );
}

#[test]
fn char_sql_contains_expected_tables() {
    assert!(CharStatements::SEL_ENUM.sql().contains("characters"));
    assert!(CharStatements::INS_CHARACTER.sql().contains("characters"));
    assert!(
        CharStatements::INS_CHAR_CUSTOMIZATION
            .sql()
            .contains("character_customizations")
    );
    assert!(CharStatements::DEL_CHARACTER.sql().contains("characters"));
    assert!(
        CharStatements::SEL_CHARACTER_INSTANCE_LOCK
            .sql()
            .contains("character_instance_lock")
    );
    assert!(CharStatements::SEL_INSTANCE.sql().contains("instance"));
    assert!(
        CharStatements::SEL_ACCOUNT_INSTANCELOCKTIMES
            .sql()
            .contains("account_instance_times")
    );
    assert!(CharStatements::SEL_RESPAWNS.sql().contains("respawn"));
    assert!(CharStatements::DEL_ALL_RESPAWNS.sql().contains("respawn"));
    assert!(
        CharStatements::DEL_GAME_EVENT_SAVE
            .sql()
            .contains("game_event_save")
    );
    assert!(
        CharStatements::INS_GAME_EVENT_SAVE
            .sql()
            .contains("game_event_save")
    );
    assert!(
        CharStatements::DEL_ALL_GAME_EVENT_CONDITION_SAVE
            .sql()
            .contains("game_event_condition_save")
    );
    assert!(
        CharStatements::SEL_GAME_EVENT_CONDITION_SAVES
            .sql()
            .contains("game_event_condition_save")
    );
    assert!(
        CharStatements::DEL_GAME_EVENT_CONDITION_SAVE
            .sql()
            .contains("game_event_condition_save")
    );
    assert!(
        CharStatements::INS_GAME_EVENT_CONDITION_SAVE
            .sql()
            .contains("game_event_condition_save")
    );
    assert!(
        CharStatements::DEL_RESET_CHARACTER_QUESTSTATUS_SEASONAL_BY_EVENT
            .sql()
            .contains("character_queststatus_seasonal")
    );
    assert!(
        CharStatements::SEL_CHAR_QUEST_STATUS_SEASONAL
            .sql()
            .contains("character_queststatus_seasonal")
    );
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
    // INS_CHARACTER follows the full Trinity character row.
    assert_eq!(CharStatements::INS_CHARACTER.sql().matches('?').count(), 70);
    // INS_CHAR_CUSTOMIZATION has 3 placeholders
    assert_eq!(
        CharStatements::INS_CHAR_CUSTOMIZATION
            .sql()
            .matches('?')
            .count(),
        3
    );
    // DEL_CHARACTER has 1 placeholder
    assert_eq!(CharStatements::DEL_CHARACTER.sql().matches('?').count(), 1);
    // SEL_CHARACTER has 1 placeholder
    assert_eq!(CharStatements::SEL_CHARACTER.sql().matches('?').count(), 1);
    // SEL_CHAR_DEL_CHECK has 2 placeholders
    assert_eq!(
        CharStatements::SEL_CHAR_DEL_CHECK
            .sql()
            .matches('?')
            .count(),
        2
    );
    // Player currency save/load statements mirror C++ CharacterDatabase.cpp.
    assert_eq!(
        CharStatements::SEL_PLAYER_CURRENCY
            .sql()
            .matches('?')
            .count(),
        1
    );
    assert_eq!(
        CharStatements::UPD_PLAYER_CURRENCY
            .sql()
            .matches('?')
            .count(),
        8
    );
    assert_eq!(
        CharStatements::REP_PLAYER_CURRENCY
            .sql()
            .matches('?')
            .count(),
        8
    );
    assert_eq!(
        CharStatements::DEL_CHARACTER_QUESTSTATUS_DAILY
            .sql()
            .matches('?')
            .count(),
        1
    );
    assert_eq!(
        CharStatements::DEL_CHARACTER_QUESTSTATUS_WEEKLY
            .sql()
            .matches('?')
            .count(),
        1
    );
    assert_eq!(
        CharStatements::DEL_CHARACTER_QUESTSTATUS_MONTHLY
            .sql()
            .matches('?')
            .count(),
        1
    );
    assert_eq!(
        CharStatements::DEL_CHARACTER_QUESTSTATUS_SEASONAL
            .sql()
            .matches('?')
            .count(),
        1
    );
    assert_eq!(
        CharStatements::INS_CHARACTER_QUESTSTATUS_DAILY
            .sql()
            .matches('?')
            .count(),
        3
    );
    assert_eq!(
        CharStatements::INS_CHARACTER_QUESTSTATUS_WEEKLY
            .sql()
            .matches('?')
            .count(),
        2
    );
    assert_eq!(
        CharStatements::INS_CHARACTER_QUESTSTATUS_MONTHLY
            .sql()
            .matches('?')
            .count(),
        2
    );
    assert_eq!(
        CharStatements::INS_CHARACTER_QUESTSTATUS_SEASONAL
            .sql()
            .matches('?')
            .count(),
        4
    );
    assert_eq!(
        CharStatements::SEL_CHAR_EQUIPMENT
            .sql()
            .matches('?')
            .count(),
        1
    );
    assert!(
        CharStatements::SEL_CHAR_EQUIPMENT
            .sql()
            .contains("ii.enchantments")
    );
    assert!(
        CharStatements::SEL_CHAR_EQUIPMENT
            .sql()
            .contains("ii.randomPropertiesId")
    );
    assert!(
        CharStatements::SEL_CHAR_EQUIPMENT
            .sql()
            .contains("ii.randomPropertiesSeed")
    );
    assert!(
        CharStatements::SEL_CHAR_EQUIPMENT
            .sql()
            .contains("ii.duration, ii.charges")
    );
    assert!(
        CharStatements::SEL_CHAR_EQUIPMENT
            .sql()
            .contains("LEFT JOIN item_instance_gems ig ON ii.guid = ig.itemGuid")
    );
    assert!(
        CharStatements::SEL_CHAR_EQUIPMENT
            .sql()
            .contains("ig.gemItemId3")
    );
    assert_eq!(
        CharStatements::INS_ITEM_INSTANCE_WITH_RANDOM_CONTEXT
            .sql()
            .matches('?')
            .count(),
        9
    );
    assert_eq!(
        CharStatements::INS_ITEM_INSTANCE_CLONE
            .sql()
            .matches('?')
            .count(),
        15
    );
    assert!(
        CharStatements::INS_ITEM_INSTANCE_CLONE
            .sql()
            .contains("charges, enchantments, flags")
    );
    assert_eq!(
        CharStatements::UPD_ITEM_INSTANCE_FLAGS
            .sql()
            .matches('?')
            .count(),
        2
    );
    assert_eq!(
        CharStatements::UPD_ITEM_INSTANCE_ENCHANTMENTS
            .sql()
            .matches('?')
            .count(),
        2
    );
    assert_eq!(
        CharStatements::UPD_ITEM_INSTANCE_STORAGE_MUTABLE
            .sql()
            .matches('?')
            .count(),
        8
    );
    assert_eq!(
        CharStatements::SEL_CHARACTER_GIFT_BY_ITEM
            .sql()
            .matches('?')
            .count(),
        1
    );
    assert_eq!(CharStatements::DEL_GIFT.sql().matches('?').count(), 1);
    assert_eq!(
        CharStatements::UPD_ITEM_INSTANCE_OPEN_GIFT
            .sql()
            .matches('?')
            .count(),
        4
    );
    assert_eq!(
        CharStatements::SEL_ITEM_REFUNDS.sql().matches('?').count(),
        2
    );
    assert_eq!(
        CharStatements::SEL_CHAR_BAG_CONTENTS
            .sql()
            .matches('?')
            .count(),
        1
    );
    assert!(
        CharStatements::SEL_CHAR_BAG_CONTENTS
            .sql()
            .contains("ii.enchantments")
    );
    assert!(
        CharStatements::SEL_CHAR_BAG_CONTENTS
            .sql()
            .contains("ii.randomPropertiesId")
    );
    assert!(
        CharStatements::SEL_CHAR_BAG_CONTENTS
            .sql()
            .contains("ii.randomPropertiesSeed")
    );
    assert!(
        CharStatements::SEL_CHAR_BAG_CONTENTS
            .sql()
            .contains("ii.duration, ii.charges")
    );
    assert!(
        CharStatements::SEL_CHAR_BAG_CONTENTS
            .sql()
            .contains("LEFT JOIN item_instance_gems ig ON ii.guid = ig.itemGuid")
    );
    assert!(
        CharStatements::SEL_CHAR_BAG_CONTENTS
            .sql()
            .contains("ig.gemItemId3")
    );
    assert_eq!(
        CharStatements::DEL_ITEM_REFUND_INSTANCE
            .sql()
            .matches('?')
            .count(),
        1
    );
    assert_eq!(
        CharStatements::DEL_ITEMCONTAINER_MONEY
            .sql()
            .matches('?')
            .count(),
        1
    );
    assert_eq!(
        CharStatements::DEL_INVALID_ITEM_LOOT_MONEY_GUIDS
            .sql()
            .matches('?')
            .count(),
        1
    );
    assert_eq!(
        CharStatements::DEL_INVALID_ITEM_LOOT_ITEMS_GUIDS
            .sql()
            .matches('?')
            .count(),
        1
    );
    assert_eq!(
        CharStatements::DEL_ITEMCONTAINER_ITEMS
            .sql()
            .matches('?')
            .count(),
        1
    );
    assert_eq!(
        CharStatements::DEL_ITEMCONTAINER_ITEM
            .sql()
            .matches('?')
            .count(),
        4
    );
    assert_eq!(
        CharStatements::SEL_ITEMCONTAINER_MONEY
            .sql()
            .matches('?')
            .count(),
        1
    );
    assert_eq!(
        CharStatements::INS_ITEMCONTAINER_MONEY
            .sql()
            .matches('?')
            .count(),
        2
    );
    assert_eq!(
        CharStatements::SEL_ITEMCONTAINER_ITEMS
            .sql()
            .matches('?')
            .count(),
        1
    );
    assert_eq!(
        CharStatements::INS_ITEMCONTAINER_ITEMS
            .sql()
            .matches('?')
            .count(),
        13
    );
    assert_eq!(
        CharStatements::INS_ITEM_REFUND_INSTANCE
            .sql()
            .matches('?')
            .count(),
        4
    );
    assert_eq!(
        CharStatements::SEL_ACCOUNT_INSTANCELOCKTIMES
            .sql()
            .matches('?')
            .count(),
        1
    );
    assert_eq!(
        CharStatements::DEL_ACCOUNT_INSTANCE_LOCK_TIMES
            .sql()
            .matches('?')
            .count(),
        1
    );
    assert_eq!(
        CharStatements::INS_ACCOUNT_INSTANCE_LOCK_TIMES
            .sql()
            .matches('?')
            .count(),
        3
    );
    assert_eq!(CharStatements::SEL_INSTANCE.sql().matches('?').count(), 0);
    assert_eq!(
        CharStatements::SEL_CHARACTER_INSTANCE_LOCK
            .sql()
            .matches('?')
            .count(),
        0
    );
    assert_eq!(
        CharStatements::DEL_CHARACTER_INSTANCE_LOCK
            .sql()
            .matches('?')
            .count(),
        3
    );
    assert_eq!(
        CharStatements::DEL_CHARACTER_INSTANCE_LOCK_BY_GUID
            .sql()
            .matches('?')
            .count(),
        1
    );
    assert_eq!(
        CharStatements::INS_CHARACTER_INSTANCE_LOCK
            .sql()
            .matches('?')
            .count(),
        10
    );
    assert_eq!(
        CharStatements::UPD_CHARACTER_INSTANCE_LOCK_EXTENSION
            .sql()
            .matches('?')
            .count(),
        4
    );
    assert_eq!(
        CharStatements::UPD_CHARACTER_INSTANCE_LOCK_FORCE_EXPIRE
            .sql()
            .matches('?')
            .count(),
        4
    );
    assert_eq!(CharStatements::DEL_INSTANCE.sql().matches('?').count(), 1);
    assert_eq!(CharStatements::INS_INSTANCE.sql().matches('?').count(), 4);
    assert_eq!(CharStatements::SEL_RESPAWNS.sql().matches('?').count(), 2);
    assert_eq!(CharStatements::REP_RESPAWN.sql().matches('?').count(), 5);
    assert_eq!(CharStatements::DEL_RESPAWN.sql().matches('?').count(), 4);
    assert_eq!(
        CharStatements::UPD_GROUP_LEADER.sql().matches('?').count(),
        2
    );
    assert_eq!(CharStatements::INS_GROUP.sql().matches('?').count(), 18);
    assert_eq!(
        CharStatements::INS_GROUP_MEMBER.sql().matches('?').count(),
        5
    );
    assert_eq!(
        CharStatements::UPD_GROUP_MEMBER_SUBGROUP
            .sql()
            .matches('?')
            .count(),
        2
    );
    assert_eq!(
        CharStatements::DEL_GROUP_MEMBER.sql().matches('?').count(),
        1
    );
    assert_eq!(CharStatements::DEL_GROUP.sql().matches('?').count(), 1);
    assert_eq!(
        CharStatements::DEL_GROUP_MEMBER_ALL
            .sql()
            .matches('?')
            .count(),
        1
    );
    assert_eq!(CharStatements::DEL_LFG_DATA.sql().matches('?').count(), 1);
    assert_eq!(
        CharStatements::DEL_ALL_RESPAWNS.sql().matches('?').count(),
        2
    );
    assert_eq!(
        CharStatements::DEL_GROUP_MEMBERS_WITHOUT_CHARACTER
            .sql()
            .matches('?')
            .count(),
        0
    );
    assert_eq!(
        CharStatements::DEL_GROUPS_WITHOUT_LEADER
            .sql()
            .matches('?')
            .count(),
        0
    );
    assert_eq!(
        CharStatements::DEL_GROUPS_WITH_FEWER_THAN_TWO_MEMBERS
            .sql()
            .matches('?')
            .count(),
        0
    );
    assert_eq!(
        CharStatements::DEL_GROUP_MEMBERS_WITHOUT_GROUP
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
        CharStatements::SEL_GAME_EVENT_CONDITION_SAVES
            .sql()
            .matches('?')
            .count(),
        0
    );
    assert_eq!(
        CharStatements::DEL_GAME_EVENT_CONDITION_SAVE
            .sql()
            .matches('?')
            .count(),
        2
    );
    assert_eq!(
        CharStatements::INS_GAME_EVENT_CONDITION_SAVE
            .sql()
            .matches('?')
            .count(),
        3
    );
    assert_eq!(
        CharStatements::DEL_RESET_CHARACTER_QUESTSTATUS_SEASONAL_BY_EVENT
            .sql()
            .matches('?')
            .count(),
        2
    );
}
