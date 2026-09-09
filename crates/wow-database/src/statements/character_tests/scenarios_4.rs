//! Character-statement C++ contrast regressions, part 4 of 4.
//!
//! Moved out of the character_tests.rs root under #652; every test is unchanged.

use super::*;

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
