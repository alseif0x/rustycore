//! The character-database statement identity list.
//!
//! Separated from the character.rs root under #652. Behaviour is preserved.

use super::*;

/// Prepared statements for the character database.
///
/// Covers character list, creation, deletion, and login operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types)]
pub enum CharStatements {
    /// DELETE FROM pool_quest_save WHERE pool_id = ?
    DEL_POOL_QUEST_SAVE,

    /// INSERT INTO pool_quest_save (pool_id, quest_id) VALUES (?, ?)
    INS_POOL_QUEST_SAVE,

    /// DELETE FROM guild_bank_item WHERE guildid = ? AND TabId = ? AND SlotId = ?
    DEL_NONEXISTENT_GUILD_BANK_ITEM,

    /// UPDATE character_banned SET active = 0 WHERE unbandate <= UNIX_TIMESTAMP() AND unbandate <> bandate
    DEL_EXPIRED_BANS,

    /// C++ `CHAR_SEL_ENUM` character-list row query.
    SEL_ENUM,

    /// C++ `CHAR_SEL_ENUM_DECLINED_NAME` character-list row query with genitive declined name.
    SEL_ENUM_DECLINED_NAME,

    /// C++ `CHAR_SEL_ENUM_CUSTOMIZATIONS` character-list customizations query.
    SEL_ENUM_CUSTOMIZATIONS,

    /// C++ `CHAR_SEL_UNDELETE_ENUM` deleted-character list row query.
    SEL_UNDELETE_ENUM,

    /// C++ `CHAR_SEL_UNDELETE_ENUM_DECLINED_NAME` deleted-character list row query with genitive declined name.
    SEL_UNDELETE_ENUM_DECLINED_NAME,

    /// C++ `CHAR_SEL_UNDELETE_ENUM_CUSTOMIZATIONS` deleted-character customizations query.
    SEL_UNDELETE_ENUM_CUSTOMIZATIONS,

    /// SELECT 1 FROM characters WHERE name = ?
    SEL_CHECK_NAME,

    /// C++ `ObjectMgr::LoadReservedPlayersNames`.
    SEL_RESERVED_NAMES,

    /// SELECT 1 FROM characters WHERE guid = ?
    SEL_CHECK_GUID,

    /// SELECT COUNT(guid) FROM characters WHERE account = ? AND deleteDate IS NULL
    SEL_SUM_CHARS,

    /// SELECT level, race, class FROM characters WHERE account = ? LIMIT 0, ?
    SEL_CHAR_CREATE_INFO,

    /// INSERT INTO character_banned (guid, bandate, unbandate, bannedby, banreason, active) VALUES (?, UNIX_TIMESTAMP(), UNIX_TIMESTAMP()+?, ?, ?, 1)
    INS_CHARACTER_BAN,

    /// UPDATE character_banned SET active = 0 WHERE guid = ? AND active != 0
    UPD_CHARACTER_BAN,

    /// DELETE cb FROM character_banned cb INNER JOIN characters c ON c.guid = cb.guid WHERE c.account = ?
    DEL_CHARACTER_BAN,

    /// SELECT bandate, unbandate-bandate, active, unbandate, banreason, bannedby FROM character_banned WHERE guid = ? ORDER BY bandate ASC
    SEL_BANINFO,

    /// SELECT guid, name FROM characters WHERE name LIKE CONCAT('%%', ?, '%%')
    SEL_GUID_BY_NAME_FILTER,

    /// SELECT bandate, unbandate, bannedby, banreason FROM character_banned WHERE guid = ? ORDER BY unbandate
    SEL_BANINFO_LIST,

    /// SELECT characters.name FROM characters, character_banned WHERE character_banned.guid = ? AND character_banned.guid = characters.guid
    SEL_BANNED_NAME,

    /// SELECT COUNT(id) FROM mail WHERE receiver = ?
    SEL_MAIL_LIST_COUNT,

    /// SELECT mail list metadata for one receiver.
    SEL_MAIL_LIST_INFO,

    /// SELECT itemEntry,count FROM item_instance WHERE guid = ?
    SEL_MAIL_LIST_ITEMS,

    /// SELECT name, at_login FROM characters WHERE guid = ? AND NOT EXISTS (SELECT NULL FROM characters WHERE name = ?)
    SEL_FREE_NAME,

    /// SELECT zone FROM characters WHERE guid = ?
    SEL_CHAR_ZONE,

    /// SELECT map, position_x, position_y, position_z FROM characters WHERE guid = ?
    SEL_CHAR_POSITION_XYZ,

    /// SELECT position_x, position_y, position_z, orientation, map, taxi_path FROM characters WHERE guid = ?
    SEL_CHAR_POSITION,

    /// DELETE FROM character_battleground_random
    DEL_BATTLEGROUND_RANDOM_ALL,

    /// DELETE FROM character_battleground_random WHERE guid = ?
    DEL_BATTLEGROUND_RANDOM,

    /// INSERT INTO character_battleground_random (guid) VALUES (?)
    INS_BATTLEGROUND_RANDOM,

    /// C++ `CHAR_INS_CHARACTER` full character persistence insert.
    INS_CHARACTER,

    /// C++ `CHAR_UPD_CHARACTER` full character persistence update.
    UPD_CHARACTER,

    /// INSERT INTO character_customizations (guid, chrCustomizationOptionID,
    /// chrCustomizationChoiceID) VALUES (?,?,?)
    INS_CHAR_CUSTOMIZATION,

    /// C++ `CHAR_INS_CHARACTER_CUSTOMIZATION` alias for the same customization insert.
    INS_CHARACTER_CUSTOMIZATION,

    /// UPDATE characters SET at_login = at_login | ? WHERE guid = ?
    UPD_ADD_AT_LOGIN_FLAG,

    /// UPDATE characters set at_login = at_login & ~ ? WHERE guid = ?
    UPD_REM_AT_LOGIN_FLAG,

    /// UPDATE characters SET at_login = at_login | ?
    UPD_ALL_AT_LOGIN_FLAGS,

    /// INSERT INTO bugreport (type, content) VALUES(?, ?)
    INS_BUG_REPORT,

    /// UPDATE petition SET name = ? WHERE petitionguid = ?
    UPD_PETITION_NAME,

    /// INSERT INTO petition_sign.
    INS_PETITION_SIGNATURE,

    /// UPDATE characters SET online = 0 WHERE account = ?
    UPD_ACCOUNT_ONLINE,

    /// DELETE FROM character_customizations WHERE guid = ?
    DEL_CHARACTER_CUSTOMIZATIONS,

    /// DELETE FROM characters WHERE guid = ?
    DEL_CHARACTER,

    /// DELETE FROM character_reputation WHERE guid = ? AND faction = ?
    DEL_CHAR_REPUTATION_BY_FACTION,

    /// INSERT INTO character_reputation (guid, faction, standing, flags) VALUES (?, ?, ? , ?)
    INS_CHAR_REPUTATION_BY_FACTION,

    /// DELETE FROM character_reputation WHERE guid = ?
    DEL_CHAR_REPUTATION,

    /// C++ `CHAR_SEL_CHARACTER` full character load row.
    SEL_CHARACTER,

    /// SELECT chrCustomizationOptionID, chrCustomizationChoiceID FROM character_customizations WHERE guid = ? ORDER BY chrCustomizationOptionID
    SEL_CHARACTER_CUSTOMIZATIONS,

    /// SELECT guid FROM group_member WHERE memberGuid = ?
    SEL_GROUP_MEMBER,

    /// SELECT casterGuid, itemGuid, spell, effectMask, recalculateMask, difficulty, stackCount, maxDuration, remainTime, remainCharges, castItemId, castItemLevel FROM character_aura WHERE guid = ?
    SEL_CHARACTER_AURAS,

    /// SELECT casterGuid, itemGuid, spell, effectMask, effectIndex, amount, baseAmount FROM character_aura_effect WHERE guid = ?
    SEL_CHARACTER_AURA_EFFECTS,

    /// UPDATE characters SET online = 1 WHERE guid = ?
    UPD_CHAR_ONLINE,

    /// UPDATE characters SET online = 0 WHERE guid = ?
    UPD_CHAR_OFFLINE,

    /// SELECT guid, account FROM characters WHERE guid = ? AND account = ?
    SEL_CHAR_DEL_CHECK,

    /// SELECT MAX(guid) FROM characters
    SEL_MAX_GUID,

    /// SELECT ci.slot, ii.itemEntry, ci.item, ii.count, ii.durability, ii.context,
    /// ii.flags, ii.playedTime, ir.paidMoney, ir.paidExtendedCost
    /// FROM character_inventory ci
    /// JOIN item_instance ii ON ci.item = ii.guid
    /// LEFT JOIN item_refund_instance ir ON ir.item_guid = ci.item AND ir.player_guid = ci.guid
    /// WHERE ci.guid = ? AND ci.bag = 0
    SEL_CHAR_EQUIPMENT,

    /// UPDATE character_inventory SET slot = ? WHERE guid = ? AND item = ?
    UPD_CHAR_INVENTORY_SLOT,

    /// DELETE FROM character_inventory WHERE guid = ? AND item = ?
    DEL_CHAR_INVENTORY_ITEM,

    /// Delete a character-inventory link only while its item still has the expected owner.
    DEL_CHAR_INVENTORY_ITEM_BY_OWNER,

    /// SELECT skill, value, max, professionSlot FROM character_skills WHERE guid = ?
    SEL_CHARACTER_SKILLS,

    /// SELECT spell, active, disabled FROM character_spell WHERE guid = ?
    SEL_CHARACTER_SPELL,

    /// SELECT spell FROM character_spell_favorite WHERE guid = ?
    SEL_CHARACTER_SPELL_FAVORITES,

    /// SELECT questObjectiveId FROM character_queststatus_objectives_criteria WHERE guid = ?
    SEL_CHARACTER_QUESTSTATUS_OBJECTIVES_CRITERIA,

    /// SELECT criteriaId, counter, date FROM character_queststatus_objectives_criteria_progress WHERE guid = ?
    SEL_CHARACTER_QUESTSTATUS_OBJECTIVES_CRITERIA_PROGRESS,

    /// SELECT quest, time FROM character_queststatus_daily WHERE guid = ?
    SEL_CHARACTER_QUESTSTATUS_DAILY,

    /// SELECT quest FROM character_queststatus_weekly WHERE guid = ?
    SEL_CHARACTER_QUESTSTATUS_WEEKLY,

    /// SELECT quest FROM character_queststatus_monthly WHERE guid = ?
    SEL_CHARACTER_QUESTSTATUS_MONTHLY,

    /// SELECT quest, event, completedTime FROM character_queststatus_seasonal WHERE guid = ?
    SEL_CHARACTER_QUESTSTATUS_SEASONAL,

    /// SELECT faction, standing, flags FROM character_reputation WHERE guid = ?
    SEL_CHARACTER_REPUTATION,

    /// SELECT COUNT(*) FROM mail WHERE receiver = ?
    SEL_MAIL_COUNT,

    /// SELECT cs.friend, c.account, cs.flags, cs.note FROM character_social cs JOIN characters c ON c.guid = cs.friend WHERE cs.guid = ? AND c.deleteinfos_name IS NULL LIMIT 255
    SEL_CHARACTER_SOCIALLIST,

    /// SELECT mapId, zoneId, posX, posY, posZ, orientation FROM character_homebind WHERE guid = ?
    SEL_CHARACTER_HOMEBIND,

    /// SELECT spell, item, time, categoryId, categoryEnd FROM character_spell_cooldown WHERE guid = ? AND time > UNIX_TIMESTAMP()
    SEL_CHARACTER_SPELLCOOLDOWNS,

    /// SELECT categoryId, rechargeStart, rechargeEnd FROM character_spell_charges WHERE guid = ? AND rechargeEnd > UNIX_TIMESTAMP() ORDER BY rechargeEnd
    SEL_CHARACTER_SPELL_CHARGES,

    /// SELECT genitive, dative, accusative, instrumental, prepositional FROM character_declinedname WHERE guid = ?
    SEL_CHARACTER_DECLINEDNAMES,

    /// SELECT guildid, `rank` FROM guild_member WHERE guid = ?
    SEL_GUILD_MEMBER,

    /// SELECT extended guild membership data for one character.
    SEL_GUILD_MEMBER_EXTENDED,

    /// SELECT achievement, date FROM character_achievement WHERE guid = ?
    SEL_CHARACTER_ACHIEVEMENTS,

    /// SELECT criteria, counter, date FROM character_achievement_progress WHERE guid = ?
    SEL_CHARACTER_CRITERIAPROGRESS,

    /// SELECT character equipment sets.
    SEL_CHARACTER_EQUIPMENTSETS,

    /// SELECT character transmog outfits.
    SEL_CHARACTER_TRANSMOG_OUTFITS,

    /// SELECT instanceId, team, joinX, joinY, joinZ, joinO, joinMapId, taxiStart, taxiEnd, mountSpell, queueId FROM character_battleground_data WHERE guid = ?
    SEL_CHARACTER_BGDATA,

    /// SELECT talentGroup, glyphSlot, glyphId FROM character_glyphs WHERE guid = ?
    SEL_CHARACTER_GLYPHS,

    /// SELECT talentId, talentRank, talentGroup FROM character_talent WHERE guid = ?
    SEL_CHARACTER_TALENTS,

    /// SELECT guid FROM character_battleground_random WHERE guid = ?
    SEL_CHARACTER_RANDOMBG,

    /// SELECT guid FROM character_banned WHERE guid = ? AND active = 1
    SEL_CHARACTER_BANNED,

    /// SELECT quest FROM character_queststatus_rewarded WHERE guid = ? AND active = 1
    SEL_CHARACTER_QUESTSTATUSREW,

    /// SELECT `order`, itemId, itemLevel, battlePetSpeciesId, suffixItemNameDescriptionId FROM character_favorite_auctions WHERE guid = ? ORDER BY `order`
    SEL_CHARACTER_FAVORITE_AUCTIONS,

    /// INSERT INTO character_favorite_auctions (guid, `order`, itemId, itemLevel, battlePetSpeciesId, suffixItemNameDescriptionId) VALUE (?, ?, ?, ?, ?, ?)
    INS_CHARACTER_FAVORITE_AUCTION,

    /// DELETE FROM character_favorite_auctions WHERE guid = ? AND `order` = ?
    DEL_CHARACTER_FAVORITE_AUCTION,

    /// DELETE FROM character_favorite_auctions WHERE guid = ?
    DEL_CHARACTER_FAVORITE_AUCTIONS_BY_CHAR,

    /// SELECT Currency, Quantity, WeeklyQuantity, TrackedQuantity,
    /// IncreasedCapQuantity, EarnedQuantity, Flags FROM character_currency
    /// WHERE CharacterGuid = ?
    SEL_PLAYER_CURRENCY,

    /// UPDATE character_currency SET Quantity = ?, WeeklyQuantity = ?,
    /// TrackedQuantity = ?, IncreasedCapQuantity = ?, EarnedQuantity = ?,
    /// Flags = ? WHERE CharacterGuid = ? AND Currency = ?
    UPD_PLAYER_CURRENCY,

    /// REPLACE INTO character_currency (CharacterGuid, Currency, Quantity,
    /// WeeklyQuantity, TrackedQuantity, IncreasedCapQuantity, EarnedQuantity, Flags)
    /// VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    REP_PLAYER_CURRENCY,

    /// DELETE FROM character_currency WHERE CharacterGuid = ?
    DEL_PLAYER_CURRENCY,

    /// SELECT button, action, type FROM character_action
    /// WHERE guid = ? AND spec = ? AND traitConfigId = ? ORDER BY button
    SEL_CHARACTER_ACTIONS_SPEC,

    /// INSERT INTO character_action (guid, spec, traitConfigId, button, action, type)
    /// VALUES (?, 0, 0, ?, ?, ?)
    INS_CHARACTER_ACTION,

    /// UPDATE `groups` SET groupType = ? WHERE guid = ?
    UPD_GROUP_TYPE,
    /// UPDATE `groups` SET leaderGuid = ? WHERE guid = ?
    UPD_GROUP_LEADER,
    /// INSERT INTO `groups` (guid, leaderGuid, lootMethod, looterGuid, lootThreshold, icon1, icon2, icon3, icon4, icon5, icon6, icon7, icon8, groupType, difficulty, raidDifficulty, legacyRaidDifficulty, masterLooterGuid) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    INS_GROUP,
    /// INSERT INTO group_member (guid, memberGuid, memberFlags, subgroup, roles) VALUES(?, ?, ?, ?, ?)
    INS_GROUP_MEMBER,
    /// UPDATE group_member SET subgroup = ? WHERE memberGuid = ?
    UPD_GROUP_MEMBER_SUBGROUP,
    /// UPDATE group_member SET memberFlags = ? WHERE memberGuid = ?
    UPD_GROUP_MEMBER_FLAG,
    /// UPDATE `groups` SET difficulty = ? WHERE guid = ?
    UPD_GROUP_DIFFICULTY,
    /// UPDATE `groups` SET raidDifficulty = ? WHERE guid = ?
    UPD_GROUP_RAID_DIFFICULTY,
    /// UPDATE `groups` SET legacyRaidDifficulty = ? WHERE guid = ?
    UPD_GROUP_LEGACY_RAID_DIFFICULTY,
    /// DELETE FROM group_member WHERE memberGuid = ?
    DEL_GROUP_MEMBER,
    /// DELETE FROM `groups` WHERE guid = ?
    DEL_GROUP,
    /// DELETE FROM group_member WHERE guid = ?
    DEL_GROUP_MEMBER_ALL,
    /// DELETE FROM lfg_data WHERE guid = ?
    DEL_LFG_DATA,
    /// DELETE FROM group_member WHERE memberGuid NOT IN (SELECT guid FROM characters)
    DEL_GROUP_MEMBERS_WITHOUT_CHARACTER,
    /// DELETE FROM `groups` WHERE leaderGuid NOT IN (SELECT guid FROM characters)
    DEL_GROUPS_WITHOUT_LEADER,
    /// DELETE FROM `groups` WHERE guid NOT IN (SELECT guid FROM group_member GROUP BY guid HAVING COUNT(guid) > 1)
    DEL_GROUPS_WITH_FEWER_THAN_TWO_MEMBERS,
    /// DELETE FROM group_member WHERE guid NOT IN (SELECT guid FROM `groups`)
    DEL_GROUP_MEMBERS_WITHOUT_GROUP,
    /// SELECT C++ GroupMgr::LoadGroups group rows.
    SEL_GROUPS,
    /// SELECT C++ GroupMgr::LoadGroups member rows.
    SEL_GROUP_MEMBERS,
    /// SELECT minimal sCharacterCache projection needed by Group::LoadMemberFromDB.
    SEL_GROUP_MEMBER_CHARACTER_CACHE,

    /// UPDATE characters SET totaltime = ?, leveltime = ? WHERE guid = ?
    UPD_CHAR_PLAYED_TIME,

    /// SELECT instanceId, releaseTime FROM account_instance_times WHERE accountId = ?
    SEL_ACCOUNT_INSTANCELOCKTIMES,

    /// SELECT id, auctionHouseId, owner, bidder, minBid, buyoutOrUnitPrice, deposit, bidAmount, startTime, endTime, serverFlags FROM auctionhouse
    SEL_AUCTIONS,

    /// INSERT INTO auction_items (auctionId, itemGuid) VALUES (?, ?)
    INS_AUCTION_ITEMS,

    /// DELETE FROM auction_items WHERE itemGuid = ?
    DEL_AUCTION_ITEMS_BY_ITEM,

    /// SELECT auctionId, playerGuid FROM auction_bidders
    SEL_AUCTION_BIDDERS,

    /// INSERT INTO auction_bidders (auctionId, playerGuid) VALUES (?, ?)
    INS_AUCTION_BIDDER,

    /// DELETE FROM auction_bidders WHERE playerGuid = ?
    DEL_AUCTION_BIDDER_BY_PLAYER,

    /// INSERT INTO auctionhouse (id, auctionHouseId, owner, bidder, minBid, buyoutOrUnitPrice, deposit, bidAmount, startTime, endTime, serverFlags) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    INS_AUCTION,

    /// DELETE a, ab, ai FROM auctionhouse a LEFT JOIN auction_items ai ON a.id = ai.auctionId LEFT JOIN auction_bidders ab ON a.id = ab.auctionId WHERE a.id = ?
    DEL_AUCTION,

    /// UPDATE auctionhouse SET bidder = ?, bidAmount = ?, serverFlags = ? WHERE id = ?
    UPD_AUCTION_BID,

    /// UPDATE auctionhouse SET endTime = ? WHERE id = ?
    UPD_AUCTION_EXPIRATION,

    /// INSERT INTO mail(id, messageType, stationery, mailTemplateId, sender, receiver, subject, body, has_items, expire_time, deliver_time, money, cod, checked) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    INS_MAIL,

    /// DELETE FROM mail WHERE id = ?
    DEL_MAIL_BY_ID,

    /// INSERT INTO mail_items(mail_id, item_guid, receiver) VALUES (?, ?, ?)
    INS_MAIL_ITEM,

    /// DELETE FROM mail_items WHERE item_guid = ?
    DEL_MAIL_ITEM,

    /// DELETE FROM mail_items WHERE item_guid = ?
    DEL_INVALID_MAIL_ITEM,

    /// DELETE FROM mail WHERE expire_time < ? AND has_items = 0 AND body = ''
    DEL_EMPTY_EXPIRED_MAIL,

    /// SELECT id, messageType, sender, receiver, has_items, expire_time, cod, checked, mailTemplateId FROM mail WHERE expire_time < ?
    SEL_EXPIRED_MAIL,

    /// SELECT item_guid, itemEntry, mail_id FROM mail_items mi INNER JOIN item_instance ii ON ii.guid = mi.item_guid LEFT JOIN mail mm ON mi.mail_id = mm.id WHERE mm.id IS NOT NULL AND mm.expire_time < ?
    SEL_EXPIRED_MAIL_ITEMS,

    /// UPDATE mail SET sender = ?, receiver = ?, expire_time = ?, deliver_time = ?, cod = 0, checked = ? WHERE id = ?
    UPD_MAIL_RETURNED,

    /// UPDATE mail_items SET receiver = ? WHERE item_guid = ?
    UPD_MAIL_ITEM_RECEIVER,

    /// UPDATE item_instance SET owner_guid = ? WHERE guid = ?
    UPD_ITEM_OWNER,

    /// DELETE FROM account_instance_times WHERE accountId = ?
    DEL_ACCOUNT_INSTANCE_LOCK_TIMES,
    /// INSERT INTO account_instance_times (accountId, instanceId, releaseTime) VALUES (?, ?, ?)
    INS_ACCOUNT_INSTANCE_LOCK_TIMES,

    /// SELECT instance rows used by C++ InstanceLockMgr::Load.
    SEL_INSTANCE,
    /// SELECT character_instance_lock rows used by C++ InstanceLockMgr::Load.
    SEL_CHARACTER_INSTANCE_LOCK,
    /// DELETE FROM character_instance_lock WHERE guid = ? AND mapId = ? AND lockId = ?
    DEL_CHARACTER_INSTANCE_LOCK,
    /// DELETE FROM character_instance_lock WHERE guid = ?
    DEL_CHARACTER_INSTANCE_LOCK_BY_GUID,
    /// INSERT INTO character_instance_lock C++ lock persistence row.
    INS_CHARACTER_INSTANCE_LOCK,
    /// UPDATE character_instance_lock SET extended = ? WHERE guid = ? AND mapId = ? AND lockId = ?
    UPD_CHARACTER_INSTANCE_LOCK_EXTENSION,
    /// UPDATE character_instance_lock SET expiryTime = ?, extended = 0 WHERE guid = ? AND mapId = ? AND lockId = ?
    UPD_CHARACTER_INSTANCE_LOCK_FORCE_EXPIRE,
    /// DELETE FROM instance WHERE instanceId = ?
    DEL_INSTANCE,
    /// INSERT INTO instance (instanceId, data, completedEncountersMask, entranceWorldSafeLocId) VALUES (?, ?, ?, ?)
    INS_INSTANCE,
    /// SELECT type, spawnId, respawnTime FROM respawn WHERE mapId = ? AND instanceId = ?
    SEL_RESPAWNS,
    /// SELECT type, spawnId, respawnTime, mapId, instanceId FROM respawn
    SEL_ALL_RESPAWNS,
    /// REPLACE INTO respawn (type, spawnId, respawnTime, mapId, instanceId) VALUES (?, ?, ?, ?, ?)
    REP_RESPAWN,
    /// DELETE FROM respawn WHERE type = ? AND spawnId = ? AND mapId = ? AND instanceId = ?
    DEL_RESPAWN,
    /// DELETE FROM respawn WHERE mapId = ? AND instanceId = ?
    DEL_ALL_RESPAWNS,

    /// SELECT id, playerGuid, note, createTime, mapId, posX, posY, posZ, facing, closedBy, assignedTo, comment FROM gm_bug
    SEL_GM_BUGS,

    /// REPLACE INTO gm_bug.
    REP_GM_BUG,

    /// DELETE FROM gm_bug WHERE id = ?
    DEL_GM_BUG,

    /// DELETE FROM gm_bug
    DEL_ALL_GM_BUGS,

    /// SELECT gm_complaint rows.
    SEL_GM_COMPLAINTS,

    /// REPLACE INTO gm_complaint.
    REP_GM_COMPLAINT,

    /// DELETE FROM gm_complaint WHERE id = ?
    DEL_GM_COMPLAINT,

    /// SELECT timestamp, text FROM gm_complaint_chatlog WHERE complaintId = ? ORDER BY lineId ASC
    SEL_GM_COMPLAINT_CHATLINES,

    /// INSERT INTO gm_complaint_chatlog.
    INS_GM_COMPLAINT_CHATLINE,

    /// DELETE FROM gm_complaint_chatlog WHERE complaintId = ?
    DEL_GM_COMPLAINT_CHATLOG,

    /// DELETE FROM gm_complaint
    DEL_ALL_GM_COMPLAINTS,

    /// DELETE FROM gm_complaint_chatlog
    DEL_ALL_GM_COMPLAINT_CHATLOGS,

    /// SELECT gm_suggestion rows.
    SEL_GM_SUGGESTIONS,

    /// REPLACE INTO gm_suggestion.
    REP_GM_SUGGESTION,

    /// DELETE FROM gm_suggestion WHERE id = ?
    DEL_GM_SUGGESTION,

    /// DELETE FROM gm_suggestion
    DEL_ALL_GM_SUGGESTIONS,

    /// INSERT INTO lfg_data (guid, dungeon, state) VALUES (?, ?, ?)
    INS_LFG_DATA,

    /// DELETE FROM game_event_save WHERE eventEntry = ?
    DEL_GAME_EVENT_SAVE,
    /// INSERT INTO game_event_save (eventEntry, state, next_start) VALUES (?, ?, ?)
    INS_GAME_EVENT_SAVE,
    /// SELECT eventEntry, condition_id, done FROM game_event_condition_save
    SEL_GAME_EVENT_CONDITION_SAVES,
    /// DELETE FROM game_event_condition_save WHERE eventEntry = ?
    DEL_ALL_GAME_EVENT_CONDITION_SAVE,
    /// DELETE FROM game_event_condition_save WHERE eventEntry = ? AND condition_id = ?
    DEL_GAME_EVENT_CONDITION_SAVE,
    /// INSERT INTO game_event_condition_save (eventEntry, condition_id, done) VALUES (?, ?, ?)
    INS_GAME_EVENT_CONDITION_SAVE,
    /// DELETE FROM character_queststatus_seasonal WHERE event = ? AND completedTime < ?
    DEL_RESET_CHARACTER_QUESTSTATUS_SEASONAL_BY_EVENT,
    /// DELETE FROM character_queststatus_daily WHERE guid = ?
    DEL_CHARACTER_QUESTSTATUS_DAILY,
    /// DELETE FROM character_queststatus_weekly WHERE guid = ?
    DEL_CHARACTER_QUESTSTATUS_WEEKLY,
    /// DELETE FROM character_queststatus_monthly WHERE guid = ?
    DEL_CHARACTER_QUESTSTATUS_MONTHLY,
    /// DELETE FROM character_queststatus_seasonal WHERE guid = ?
    DEL_CHARACTER_QUESTSTATUS_SEASONAL,
    /// INSERT INTO character_queststatus_daily (guid, quest, time) VALUES (?, ?, ?)
    INS_CHARACTER_QUESTSTATUS_DAILY,
    /// INSERT INTO character_queststatus_weekly (guid, quest) VALUES (?, ?)
    INS_CHARACTER_QUESTSTATUS_WEEKLY,
    /// INSERT INTO character_queststatus_monthly (guid, quest) VALUES (?, ?)
    INS_CHARACTER_QUESTSTATUS_MONTHLY,
    /// INSERT INTO character_queststatus_seasonal (guid, quest, event, completedTime) VALUES (?, ?, ?, ?)
    INS_CHARACTER_QUESTSTATUS_SEASONAL,
    /// SELECT Id, Value FROM world_state_value
    SEL_WORLD_STATE_VALUES,
    /// REPLACE INTO world_state_value (Id, Value) VALUES (?, ?)
    /// Future C++ SetValueAndSaveInDb persistence statement; not wired by #NEXT.R8.ENTITIES.575.
    REP_WORLD_STATE,

    /// REPLACE INTO world_variable (Id, Value) VALUES (?, ?)
    REP_WORLD_VARIABLE,

    /// DELETE FROM character_spell WHERE spell = ?
    DEL_INVALID_SPELL_SPELLS,

    /// UPDATE characters delete-info fields for soft deletion.
    UPD_DELETE_INFO,

    /// UPDATE characters restore delete-info fields.
    UPD_RESTORE_DELETE_INFO,

    /// UPDATE characters SET zone = ? WHERE guid = ?
    UPD_ZONE,

    /// UPDATE characters SET level = ?, xp = 0 WHERE guid = ?
    UPD_LEVEL,

    /// DELETE FROM character_achievement_progress WHERE criteria = ?
    DEL_INVALID_ACHIEV_PROGRESS_CRITERIA,

    /// DELETE FROM guild_achievement_progress WHERE criteria = ?
    DEL_INVALID_ACHIEV_PROGRESS_CRITERIA_GUILD,

    /// DELETE FROM character_achievement WHERE achievement = ?
    DEL_INVALID_ACHIEVMENT,

    /// DELETE FROM pet_spell WHERE spell = ?
    DEL_INVALID_PET_SPELL,

    /// UPDATE characters SET name = ?, at_login = ? WHERE guid = ?
    UPD_CHAR_NAME_AT_LOGIN,

    /// DELETE FROM character_skills WHERE guid = ? AND skill = ?
    DEL_CHARACTER_SKILL,

    /// UPDATE character_social SET flags = ? WHERE guid = ? AND friend = ?
    UPD_CHARACTER_SOCIAL_FLAGS,

    /// INSERT INTO character_social (guid, friend, flags) VALUES (?, ?, ?)
    INS_CHARACTER_SOCIAL,

    /// DELETE FROM character_social WHERE guid = ? AND friend = ?
    DEL_CHARACTER_SOCIAL,

    /// UPDATE character_social SET note = ? WHERE guid = ? AND friend = ?
    UPD_CHARACTER_SOCIAL_NOTE,

    /// UPDATE characters position by guid.
    UPD_CHARACTER_POSITION,

    /// UPDATE characters position by guid and current map.
    UPD_CHARACTER_POSITION_BY_MAPID,

    /// Update represented player position without clearing transport offsets or taxi path.
    UPD_CHARACTER_POSITION_PRESERVE_TRAVEL,

    /// SELECT frozen aura rows for spell 9454.
    SEL_CHARACTER_AURA_FROZEN,

    /// SELECT online character names/accounts/maps/zones.
    SEL_CHARACTER_ONLINE,

    /// SELECT deleted character info by guid.
    SEL_CHAR_DEL_INFO_BY_GUID,

    /// SELECT deleted character info by deleted name.
    SEL_CHAR_DEL_INFO_BY_NAME,

    /// SELECT deleted character info for all deleted characters.
    SEL_CHAR_DEL_INFO,

    /// SELECT guid FROM characters WHERE account = ?
    SEL_CHARS_BY_ACCOUNT_ID,

    /// SELECT character pinfo row.
    SEL_CHAR_PINFO,

    /// SELECT pinfo ban row.
    SEL_PINFO_BANS,

    /// SELECT pinfo mail counters.
    SEL_PINFO_MAILS,

    /// SELECT pinfo xp and guild row.
    SEL_PINFO_XP,

    /// SELECT homebind row using C++ `CHAR_SEL_CHAR_HOMEBIND` name.
    SEL_CHAR_HOMEBIND,

    /// SELECT guid, name, online FROM characters WHERE account = ?
    SEL_CHAR_GUID_NAME_BY_ACC,

    /// SELECT name, race, class, gender, at_login FROM characters WHERE guid = ?
    SEL_CHAR_CUSTOMIZE_INFO,

    /// SELECT race/faction change info.
    SEL_CHAR_RACE_OR_FACTION_CHANGE_INFOS,

    /// SELECT COD item mail rows.
    SEL_CHAR_COD_ITEM_MAIL,

    /// SELECT DISTINCT guid FROM character_social WHERE friend = ?
    SEL_CHAR_SOCIAL,

    /// SELECT old deleted character rows.
    SEL_CHAR_OLD_CHARS,

    /// SELECT full mail list rows ordered by id.
    SEL_MAIL,

    /// DELETE FROM character_aura WHERE spell = 9454 AND guid = ?
    DEL_CHAR_AURA_FROZEN,

    /// SELECT count of character inventory rows by item entry.
    SEL_CHAR_INVENTORY_COUNT_ITEM,

    /// SELECT count of mail item rows by item entry.
    SEL_MAIL_COUNT_ITEM,

    /// SELECT count of auction item rows by item entry.
    SEL_AUCTIONHOUSE_COUNT_ITEM,

    /// SELECT count of guild bank item rows by item entry.
    SEL_GUILD_BANK_COUNT_ITEM,

    /// SELECT character inventory item rows by entry.
    SEL_CHAR_INVENTORY_ITEM_BY_ENTRY,

    /// SELECT mail item rows by entry.
    SEL_MAIL_ITEMS_BY_ENTRY,

    /// SELECT auction item rows by entry.
    SEL_AUCTIONHOUSE_ITEM_BY_ENTRY,

    /// SELECT guild bank item rows by entry.
    SEL_GUILD_BANK_ITEM_BY_ENTRY,

    // Quest status
    SEL_CHAR_QUEST_STATUS,
    SEL_CHARACTER_QUESTSTATUS,
    /// SELECT quest, objective, data FROM character_queststatus_objectives WHERE guid = ?
    SEL_CHAR_QUEST_STATUS_OBJECTIVES,
    /// C++ `CHAR_SEL_CHARACTER_QUESTSTATUS_OBJECTIVES` alias.
    SEL_CHARACTER_QUESTSTATUS_OBJECTIVES,
    /// SELECT quest, event, completedTime FROM character_queststatus_seasonal WHERE guid = ?
    SEL_CHAR_QUEST_STATUS_SEASONAL,
    INS_CHAR_QUEST_STATUS,
    DEL_CHAR_QUEST_STATUS,
    DEL_CHAR_QUEST_STATUS_OBJECTIVES_BY_QUEST,
    DEL_CHAR_QUESTSTATUS_OBJECTIVES_BY_QUEST,
    REP_CHAR_QUEST_STATUS_OBJECTIVES,
    REP_CHAR_QUESTSTATUS_OBJECTIVES,

    /// DELETE FROM character_achievement WHERE guid = ?
    DEL_CHAR_ACHIEVEMENT,

    /// DELETE FROM character_achievement_progress WHERE guid = ?
    DEL_CHAR_ACHIEVEMENT_PROGRESS,

    /// INSERT INTO character_achievement (guid, achievement, date) VALUES (?, ?, ?)
    INS_CHAR_ACHIEVEMENT,

    /// DELETE FROM character_achievement_progress WHERE guid = ? AND criteria = ?
    DEL_CHAR_ACHIEVEMENT_PROGRESS_BY_CRITERIA,

    /// INSERT INTO character_achievement_progress (guid, criteria, counter, date) VALUES (?, ?, ?, ?)
    INS_CHAR_ACHIEVEMENT_PROGRESS,

    /// INSERT INTO character_gifts (guid, item_guid, entry, flags) VALUES (?, ?, ?, ?)
    INS_CHAR_GIFT,

    /// DELETE FROM mail_items WHERE mail_id = ?
    DEL_MAIL_ITEM_BY_ID,

    /// INSERT INTO petition (ownerguid, petitionguid, name) VALUES (?, ?, ?)
    INS_PETITION,

    /// DELETE FROM petition WHERE petitionguid = ?
    DEL_PETITION_BY_GUID,

    /// DELETE FROM petition_sign WHERE petitionguid = ?
    DEL_PETITION_SIGNATURE_BY_GUID,

    /// DELETE FROM character_declinedname WHERE guid = ?
    DEL_CHAR_DECLINED_NAME,

    /// INSERT INTO character_declinedname.
    INS_CHAR_DECLINED_NAME,

    /// UPDATE characters SET race = ?, extra_flags = extra_flags | ? WHERE guid = ?
    UPD_CHAR_RACE,

    /// DELETE language skills for a character.
    DEL_CHAR_SKILL_LANGUAGES,

    /// INSERT INTO `character_skills` language row.
    INS_CHAR_SKILL_LANGUAGE,

    /// UPDATE characters SET taxi_path = '' WHERE guid = ?
    UPD_CHAR_TAXI_PATH,

    /// UPDATE characters SET taximask = ? WHERE guid = ?
    UPD_CHAR_TAXIMASK,

    /// DELETE FROM character_queststatus WHERE guid = ?
    DEL_CHAR_QUESTSTATUS,

    /// DELETE FROM character_queststatus_objectives WHERE guid = ?
    DEL_CHAR_QUESTSTATUS_OBJECTIVES,

    /// DELETE FROM character_queststatus_objectives_criteria WHERE guid = ?
    DEL_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA,

    /// DELETE FROM character_queststatus_objectives_criteria_progress WHERE guid = ?
    DEL_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA_PROGRESS,

    /// DELETE FROM character_queststatus_objectives_criteria_progress WHERE guid = ? AND criteriaId = ?
    DEL_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA_PROGRESS_BY_CRITERIA,

    /// DELETE FROM character_social WHERE guid = ?
    DEL_CHAR_SOCIAL_BY_GUID,

    /// DELETE FROM character_social WHERE friend = ?
    DEL_CHAR_SOCIAL_BY_FRIEND,

    /// DELETE FROM character_achievement WHERE achievement = ? AND guid = ?
    DEL_CHAR_ACHIEVEMENT_BY_ACHIEVEMENT,

    /// UPDATE character_achievement SET achievement = ? where achievement = ? AND guid = ?
    UPD_CHAR_ACHIEVEMENT,

    /// UPDATE item_instance ii, character_inventory ci SET ii.itemEntry = ? WHERE ii.itemEntry = ? AND ci.guid = ? AND ci.item = ii.guid
    UPD_CHAR_INVENTORY_FACTION_CHANGE,

    /// DELETE FROM character_spell WHERE spell = ? AND guid = ?
    DEL_CHAR_SPELL_BY_SPELL,

    /// UPDATE character_spell SET spell = ? where spell = ? AND guid = ?
    UPD_CHAR_SPELL_FACTION_CHANGE,

    /// SELECT standing FROM character_reputation WHERE faction = ? AND guid = ?
    SEL_CHAR_REP_BY_FACTION,

    /// DELETE FROM character_reputation WHERE faction = ? AND guid = ?
    DEL_CHAR_REP_BY_FACTION,

    /// UPDATE character_reputation SET faction = ?, standing = ? WHERE faction = ? AND guid = ?
    UPD_CHAR_REP_FACTION_CHANGE,

    /// UPDATE characters SET knownTitles = ? WHERE guid = ?
    UPD_CHAR_TITLES_FACTION_CHANGE,

    /// UPDATE characters SET chosenTitle = 0 WHERE guid = ?
    RES_CHAR_TITLES_FACTION_CHANGE,

    /// DELETE FROM character_spell_cooldown WHERE guid = ?
    DEL_CHAR_SPELL_COOLDOWNS,

    /// INSERT INTO character_spell_cooldown (guid, spell, item, time, categoryId, categoryEnd) VALUES (?, ?, ?, ?, ?, ?)
    INS_CHAR_SPELL_COOLDOWN,

    /// DELETE FROM character_spell_charges WHERE guid = ?
    DEL_CHAR_SPELL_CHARGES,

    /// INSERT INTO character_spell_charges (guid, categoryId, rechargeStart, rechargeEnd) VALUES (?, ?, ?, ?)
    INS_CHAR_SPELL_CHARGES,

    /// DELETE FROM character_action WHERE guid = ?
    DEL_CHAR_ACTION,

    /// DELETE FROM character_aura WHERE guid = ?
    DEL_CHAR_AURA,

    /// DELETE FROM character_aura_effect WHERE guid = ?
    DEL_CHAR_AURA_EFFECT,

    /// DELETE FROM character_gifts WHERE guid = ?
    DEL_CHAR_GIFT,

    /// DELETE FROM character_inventory WHERE guid = ?
    DEL_CHAR_INVENTORY,

    /// DELETE FROM character_queststatus_rewarded WHERE guid = ?
    DEL_CHAR_QUESTSTATUS_REWARDED,

    /// DELETE FROM character_spell WHERE guid = ?
    DEL_CHAR_SPELL,

    /// DELETE FROM mail WHERE receiver = ?
    DEL_MAIL,

    /// DELETE FROM mail_items WHERE receiver = ?
    DEL_MAIL_ITEMS,

    /// DELETE FROM character_achievement WHERE guid = ? AND achievement NOT IN (...)
    DEL_CHAR_ACHIEVEMENTS,

    /// DELETE FROM character_equipmentsets WHERE guid = ?
    DEL_CHAR_EQUIPMENTSETS,

    /// DELETE FROM character_transmog_outfits WHERE guid = ?
    DEL_CHAR_TRANSMOG_OUTFITS,

    /// DELETE FROM guild_eventlog WHERE PlayerGuid1 = ? OR PlayerGuid2 = ?
    DEL_GUILD_EVENTLOG_BY_PLAYER,

    /// DELETE FROM guild_bank_eventlog WHERE PlayerGuid = ?
    DEL_GUILD_BANK_EVENTLOG_BY_PLAYER,

    /// DELETE FROM character_glyphs WHERE guid = ?
    DEL_CHAR_GLYPHS,

    /// DELETE FROM character_talent WHERE guid = ?
    DEL_CHAR_TALENT,

    /// DELETE FROM character_skills WHERE guid = ?
    DEL_CHAR_SKILLS,

    /// INSERT INTO character_action (guid, spec, traitConfigId, button, action, type) VALUES (?, ?, ?, ?, ?, ?)
    INS_CHAR_ACTION,

    /// UPDATE character_action SET action = ?, type = ? WHERE guid = ? AND button = ? AND spec = ? AND traitConfigId = ?
    UPD_CHAR_ACTION,

    /// DELETE FROM character_action WHERE guid = ? and button = ? and spec = ? AND traitConfigId = ?
    DEL_CHAR_ACTION_BY_BUTTON_SPEC,

    /// DELETE FROM character_action WHERE guid = ? AND spec = ? AND traitConfigId = ?
    DEL_CHAR_ACTION_BY_SPEC,

    /// DELETE FROM character_action WHERE guid = ? AND traitConfigId = ?
    DEL_CHAR_ACTION_BY_TRAIT_CONFIG,

    /// DELETE FROM character_inventory WHERE item = ?
    DEL_CHAR_INVENTORY_BY_ITEM,

    /// DELETE FROM character_inventory WHERE bag = ? AND slot = ? AND guid = ?
    DEL_CHAR_INVENTORY_BY_BAG_SLOT,

    /// UPDATE mail SET has_items = ?, expire_time = ?, deliver_time = ?, money = ?, cod = ?, checked = ? WHERE id = ?
    UPD_MAIL,

    /// REPLACE INTO character_queststatus (guid, quest, status, explored, acceptTime, endTime) VALUES (?, ?, ?, ?, ?, ?)
    REP_CHAR_QUESTSTATUS,

    /// DELETE FROM character_queststatus WHERE guid = ? AND quest = ?
    DEL_CHAR_QUESTSTATUS_BY_QUEST,

    /// INSERT INTO character_queststatus_objectives_criteria (guid, questObjectiveId) VALUES (?, ?)
    INS_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA,

    /// INSERT INTO character_queststatus_objectives_criteria_progress (guid, criteriaId, counter, date) VALUES (?, ?, ?, ?)
    INS_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA_PROGRESS,

    /// INSERT IGNORE INTO character_queststatus_rewarded (guid, quest, active) VALUES (?, ?, 1)
    INS_CHAR_QUESTSTATUS_REWARDED,

    /// DELETE FROM character_queststatus_rewarded WHERE guid = ? AND quest = ?
    DEL_CHAR_QUESTSTATUS_REWARDED_BY_QUEST,

    /// UPDATE character_queststatus_rewarded SET quest = ? WHERE quest = ? AND guid = ?
    UPD_CHAR_QUESTSTATUS_REWARDED_FACTION_CHANGE,

    /// UPDATE character_queststatus_rewarded SET active = 1 WHERE guid = ?
    UPD_CHAR_QUESTSTATUS_REWARDED_ACTIVE,

    /// UPDATE character_queststatus_rewarded SET active = 0 WHERE quest = ? AND guid = ?
    UPD_CHAR_QUESTSTATUS_REWARDED_ACTIVE_BY_QUEST,

    /// DELETE FROM character_queststatus_objectives_criteria WHERE questObjectiveId = ?
    DEL_INVALID_QUEST_PROGRESS_CRITERIA,

    /// DELETE FROM character_skills WHERE guid = ? AND skill = ?
    DEL_CHAR_SKILL_BY_SKILL,

    /// INSERT INTO character_skills (guid, skill, value, max, professionSlot) VALUES (?, ?, ?, ?, ?)
    INS_CHAR_SKILLS,

    /// UPDATE character_skills SET value = ?, max = ?, professionSlot = ? WHERE guid = ? AND skill = ?
    UPD_CHAR_SKILLS,

    /// INSERT INTO character_spell (guid, spell, active, disabled) VALUES (?, ?, ?, ?)
    INS_CHAR_SPELL,

    /// DB-safe fallback for a runtime LearnSpell grant made before the complete PlayerSpellMap is available.
    UPSERT_CHAR_SPELL_LEARN_FALLBACK,

    /// DELETE FROM character_spell_favorite WHERE guid = ? AND spell = ?
    DEL_CHAR_SPELL_FAVORITE,

    /// DELETE FROM character_spell_favorite WHERE guid = ?
    DEL_CHAR_SPELL_FAVORITE_BY_CHAR,

    /// INSERT INTO character_spell_favorite (guid, spell) VALUES (?, ?)
    INS_CHAR_SPELL_FAVORITE,

    /// DELETE FROM character_stats WHERE guid = ?
    DEL_CHAR_STATS,

    /// INSERT INTO character_stats full save row.
    INS_CHAR_STATS,

    /// DELETE FROM petition WHERE ownerguid = ?
    DEL_PETITION_BY_OWNER,

    /// DELETE FROM petition_sign WHERE ownerguid = ?
    DEL_PETITION_SIGNATURE_BY_OWNER,

    /// INSERT INTO character_glyphs (guid, talentGroup, glyphSlot, glyphId) VALUES(?, ?, ?, ?)
    INS_CHAR_GLYPHS,

    /// INSERT INTO character_talent (guid, talentId, talentRank, talentGroup) VALUES (?, ?, ?, ?)
    INS_CHAR_TALENT,

    /// UPDATE characters SET slot = ? WHERE guid = ? AND account = ?
    UPD_CHAR_LIST_SLOT,

    /// INSERT INTO character_fishingsteps (guid, fishingSteps) VALUES (?, ?)
    INS_CHAR_FISHINGSTEPS,

    /// DELETE FROM character_fishingsteps WHERE guid = ?
    DEL_CHAR_FISHINGSTEPS,

    /// SELECT traitConfigId, traitNodeId, traitNodeEntryId, `rank`, grantedRanks FROM character_trait_entry WHERE guid = ?
    SEL_CHAR_TRAIT_ENTRIES,

    /// INSERT INTO character_trait_entry (guid, traitConfigId, traitNodeId, traitNodeEntryId, `rank`, grantedRanks) VALUES (?, ?, ?, ?, ?, ?)
    INS_CHAR_TRAIT_ENTRIES,

    /// DELETE FROM character_trait_entry WHERE guid = ? AND traitConfigId = ?
    DEL_CHAR_TRAIT_ENTRIES,

    /// DELETE FROM character_trait_entry WHERE guid = ?
    DEL_CHAR_TRAIT_ENTRIES_BY_CHAR,

    /// SELECT traitConfigId, type, chrSpecializationId, combatConfigFlags, localIdentifier, skillLineId, traitSystemId, `name` FROM character_trait_config WHERE guid = ?
    SEL_CHAR_TRAIT_CONFIGS,

    /// INSERT INTO character_trait_config (guid, traitConfigId, type, chrSpecializationId, combatConfigFlags, localIdentifier, skillLineId, traitSystemId, `name`) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
    INS_CHAR_TRAIT_CONFIGS,

    /// DELETE FROM character_trait_config WHERE guid = ? AND traitConfigId = ?
    DEL_CHAR_TRAIT_CONFIGS,

    /// DELETE FROM character_trait_config WHERE guid = ?
    DEL_CHAR_TRAIT_CONFIGS_BY_CHAR,

    /// DELETE FROM character_queststatus_daily
    DEL_RESET_CHARACTER_QUESTSTATUS_DAILY,

    /// DELETE FROM character_queststatus_weekly
    DEL_RESET_CHARACTER_QUESTSTATUS_WEEKLY,

    /// DELETE FROM character_queststatus_monthly
    DEL_RESET_CHARACTER_QUESTSTATUS_MONTHLY,

    /// SELECT itemId, itemEntry, slot, creatorGuid, fixedScalingLevel, randomPropertiesId, randomPropertiesSeed, context FROM character_void_storage WHERE playerGuid = ?
    SEL_CHAR_VOID_STORAGE,

    /// REPLACE INTO character_void_storage (itemId, playerGuid, itemEntry, slot, creatorGuid, fixedScalingLevel, randomPropertiesId, randomPropertiesSeed, context) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
    REP_CHAR_VOID_STORAGE_ITEM,

    /// DELETE FROM character_void_storage WHERE playerGuid = ?
    DEL_CHAR_VOID_STORAGE_ITEM_BY_CHAR_GUID,

    /// DELETE FROM character_void_storage WHERE slot = ? AND playerGuid = ?
    DEL_CHAR_VOID_STORAGE_ITEM_BY_SLOT,

    /// SELECT character_cuf_profiles rows.
    SEL_CHAR_CUF_PROFILES,

    /// REPLACE INTO character_cuf_profiles.
    REP_CHAR_CUF_PROFILES,

    /// DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?
    DEL_CHAR_CUF_PROFILES_BY_ID,

    /// DELETE FROM character_cuf_profiles WHERE guid = ?
    DEL_CHAR_CUF_PROFILES,

    /// REPLACE INTO calendar_events.
    REP_CALENDAR_EVENT,

    /// DELETE FROM calendar_events WHERE EventID = ?
    DEL_CALENDAR_EVENT,

    /// REPLACE INTO calendar_invites.
    REP_CALENDAR_INVITE,

    /// DELETE FROM calendar_invites WHERE InviteID = ?
    DEL_CALENDAR_INVITE,

    /// SELECT id FROM character_pet WHERE owner = ?
    SEL_CHAR_PET_IDS,

    /// DELETE FROM character_pet_declinedname WHERE owner = ?
    DEL_CHAR_PET_DECLINEDNAME_BY_OWNER,

    /// DELETE FROM character_pet_declinedname WHERE id = ?
    DEL_CHAR_PET_DECLINEDNAME,

    /// INSERT INTO character_pet_declinedname.
    INS_CHAR_PET_DECLINEDNAME,

    /// SELECT casterGuid, spell, effectMask, recalculateMask, difficulty, stackCount, maxDuration, remainTime, remainCharges FROM pet_aura WHERE guid = ?
    SEL_PET_AURA,

    /// SELECT casterGuid, spell, effectMask, effectIndex, amount, baseAmount FROM pet_aura_effect WHERE guid = ?
    SEL_PET_AURA_EFFECT,

    /// SELECT spell, active FROM pet_spell WHERE guid = ?
    SEL_PET_SPELL,

    /// SELECT spell, time, categoryId, categoryEnd FROM pet_spell_cooldown WHERE guid = ? AND time > UNIX_TIMESTAMP()
    SEL_PET_SPELL_COOLDOWN,

    /// SELECT genitive, dative, accusative, instrumental, prepositional FROM character_pet_declinedname WHERE owner = ? AND id = ?
    SEL_PET_DECLINED_NAME,

    /// DELETE FROM pet_aura WHERE guid = ?
    DEL_PET_AURAS,

    /// DELETE FROM pet_aura_effect WHERE guid = ?
    DEL_PET_AURA_EFFECTS,

    /// DELETE FROM pet_spell WHERE guid = ?
    DEL_PET_SPELLS,

    /// DELETE FROM pet_spell_cooldown WHERE guid = ?
    DEL_PET_SPELL_COOLDOWNS,

    /// INSERT INTO pet_spell_cooldown (guid, spell, time, categoryId, categoryEnd) VALUES (?, ?, ?, ?, ?)
    INS_PET_SPELL_COOLDOWN,

    /// SELECT categoryId, rechargeStart, rechargeEnd FROM pet_spell_charges WHERE guid = ? AND rechargeEnd > UNIX_TIMESTAMP() ORDER BY rechargeEnd
    SEL_PET_SPELL_CHARGES,

    /// DELETE FROM pet_spell_charges WHERE guid = ?
    DEL_PET_SPELL_CHARGES,

    /// INSERT INTO pet_spell_charges (guid, categoryId, rechargeStart, rechargeEnd) VALUES (?, ?, ?, ?)
    INS_PET_SPELL_CHARGES,

    /// DELETE FROM pet_spell WHERE guid = ? and spell = ?
    DEL_PET_SPELL_BY_SPELL,

    /// INSERT INTO pet_spell (guid, spell, active) VALUES (?, ?, ?)
    INS_PET_SPELL,

    /// INSERT INTO pet_aura full row.
    INS_PET_AURA,

    /// INSERT INTO pet_aura_effect full row.
    INS_PET_AURA_EFFECT,

    /// SELECT character_pet rows by owner.
    SEL_CHAR_PETS,

    /// C++ `CHAR_SEL_CHARACTER_INVENTORY` with `SelectItemInstanceContent` expanded.
    SEL_CHARACTER_INVENTORY,

    /// C++ `CHAR_SEL_MAILITEMS` with `SelectItemInstanceContent` expanded.
    SEL_MAILITEMS,

    /// C++ `CHAR_SEL_AUCTION_ITEMS` with `SelectItemInstanceContent` expanded.
    SEL_AUCTION_ITEMS,

    /// C++ `CHAR_SEL_GUILD_BANK_ITEMS` with `SelectItemInstanceContent` expanded.
    SEL_GUILD_BANK_ITEMS,

    /// DELETE FROM character_pet WHERE owner = ?
    DEL_CHAR_PET_BY_OWNER,

    /// UPDATE character_pet SET name = ?, renamed = 1 WHERE owner = ? AND id = ?
    UPD_CHAR_PET_NAME,

    /// UPDATE character_pet SET slot = ? WHERE owner = ? AND id = ?
    UPD_CHAR_PET_SLOT_BY_ID,

    /// DELETE FROM character_pet WHERE id = ?
    DEL_CHAR_PET_BY_ID,

    /// DELETE FROM pet_spell WHERE guid in (SELECT id FROM character_pet WHERE owner=?)
    DEL_ALL_PET_SPELLS_BY_OWNER,

    /// UPDATE character_pet SET specialization = 0 WHERE owner=?
    UPD_PET_SPECS_BY_OWNER,

    /// INSERT INTO character_pet full row.
    INS_PET,

    /// SELECT MAX(id) FROM pvpstats_battlegrounds
    SEL_PVPSTATS_MAXID,

    /// INSERT INTO pvpstats_battlegrounds.
    INS_PVPSTATS_BATTLEGROUND,

    /// INSERT INTO pvpstats_players.
    INS_PVPSTATS_PLAYER,

    /// SELECT winner_faction, COUNT(*) AS count FROM pvpstats_battlegrounds WHERE DATEDIFF(NOW(), date) < 7 GROUP BY winner_faction ORDER BY winner_faction ASC
    SEL_PVPSTATS_FACTIONS_OVERALL,

    /// INSERT INTO quest_tracker (id, character_guid, quest_accept_time, core_hash, core_revision) VALUES (?, ?, NOW(), ?, ?)
    INS_QUEST_TRACK,

    /// UPDATE quest_tracker SET completed_by_gm = 1 WHERE id = ? AND character_guid = ? ORDER BY quest_accept_time DESC LIMIT 1
    UPD_QUEST_TRACK_GM_COMPLETE,

    /// UPDATE quest_tracker SET quest_complete_time = NOW() WHERE id = ? AND character_guid = ? ORDER BY quest_accept_time DESC LIMIT 1
    UPD_QUEST_TRACK_COMPLETE_TIME,

    /// UPDATE quest_tracker SET quest_abandon_time = NOW() WHERE id = ? AND character_guid = ? ORDER BY quest_accept_time DESC LIMIT 1
    UPD_QUEST_TRACK_ABANDON_TIME,

    /// SELECT Spell, MapId, PositionX, PositionY, PositionZ, Orientation FROM character_aura_stored_location WHERE Guid = ?
    SEL_CHARACTER_AURA_STORED_LOCATIONS,

    /// DELETE FROM character_aura_stored_location WHERE Guid = ?
    DEL_CHARACTER_AURA_STORED_LOCATIONS_BY_GUID,

    /// DELETE FROM character_aura_stored_location WHERE Guid = ? AND Spell = ?
    DEL_CHARACTER_AURA_STORED_LOCATION,

    /// INSERT INTO character_aura_stored_location.
    INS_CHARACTER_AURA_STORED_LOCATION,

    /// SELECT race, COUNT(guid) FROM characters WHERE ((playerFlags & ?) = ?) AND logout_time >= (UNIX_TIMESTAMP() - 604800) GROUP BY race
    SEL_WAR_MODE_TUNING,

    /// UPDATE characters SET money = ? WHERE guid = ?
    UPD_CHAR_MONEY,
    /// C++ `Player::UnlockVoidStorage` changes `PLAYER_FLAGS_VOID_UNLOCKED`;
    /// Rust persists that flag in the same transaction as its unlock cost.
    /// UPDATE characters SET playerFlags = ? WHERE guid = ?
    UPD_CHAR_PLAYER_FLAGS,
    /// SELECT money FROM characters WHERE guid = ? FOR UPDATE
    SEL_CHAR_MONEY_FOR_UPDATE,
    /// Reconcile a lost COMMIT reply from an absolute-money transaction.
    /// SELECT money FROM characters WHERE guid = ?
    SEL_CHAR_MONEY,
    /// C++ `CHAR_UPD_CHARACTER` persists this field immediately before powers.
    /// UPDATE characters SET health = ? WHERE guid = ?
    UPD_CHAR_HEALTH,
    /// C++ `CHAR_UPD_CHARACTER` persists these fields in the full save.
    /// UPDATE characters SET power1 = ?, ..., power10 = ? WHERE guid = ?
    UPD_CHAR_POWERS,
    /// C++ `CHAR_UPD_CHARACTER` persists these fields in the full save.
    /// UPDATE characters SET restState = ?, playerFlags = ?, rest_bonus = ?, logout_time = ?, is_logout_resting = ? WHERE guid = ?
    UPD_CHAR_REST_STATE,
    /// C++ `RestMgr::GetRestBonusFor` updates in-memory rest state during online XP gain; Rust
    /// persists only those fields here and leaves logout_time/is_logout_resting untouched.
    /// UPDATE characters SET restState = ?, playerFlags = ?, rest_bonus = ? WHERE guid = ?
    UPD_CHAR_ONLINE_REST_STATE,
    /// C++ `CHAR_UPD_CHARACTER` persists these fields in the full save.
    /// UPDATE characters SET resettalents_cost = ?, resettalents_time = ? WHERE guid = ?
    UPD_CHAR_TALENT_RESET_STATE,
    /// UPDATE characters SET xp = ? WHERE guid = ?
    UPD_CHAR_XP,
    /// UPDATE characters SET level = ?, xp = ? WHERE guid = ?
    UPD_CHAR_LEVEL,
    /// C++ `CHAR_UPD_CHARACTER` persists these fields in the full save.
    /// UPDATE characters SET dungeonDifficulty = ?, raidDifficulty = ?, legacyRaidDifficulty = ? WHERE guid = ?
    UPD_CHAR_DIFFICULTIES,
    /// C++ `CHAR_UPD_CHARACTER` persists this field in the full save.
    /// UPDATE characters SET exploredZones = ? WHERE guid = ?
    UPD_CHAR_EXPLORED_ZONES,

    /// SELECT MAX(guid) FROM item_instance
    SEL_MAX_ITEM_GUID,
    /// C++ `ObjectMgr::SetHighestGuids` shared equipment/transmog set GUID maximum.
    SEL_MAX_EQUIPMENT_SET_GUID,
    /// C++ `ObjectMgr::SetHighestGuids` void-storage raw item-ID maximum.
    SEL_MAX_VOID_STORAGE_ITEM_ID,
    /// C++ ObjectMgr::SetHighestGuids startup cleanup.
    DEL_INVALID_CHAR_INVENTORY_ITEM_GUIDS,
    DEL_INVALID_MAIL_ITEM_GUIDS,
    DEL_INVALID_AUCTION_ITEM_GUIDS,
    DEL_INVALID_GUILD_BANK_ITEM_GUIDS,
    /// Rust safety extension to C++ `ObjectMgr::SetHighestGuids`: stored loot
    /// has no foreign key to `item_instance`, so orphan rows at or above the
    /// next allocator value must not be inherited by a reused item GUID.
    DEL_INVALID_ITEM_LOOT_ITEMS_GUIDS,
    DEL_INVALID_ITEM_LOOT_MONEY_GUIDS,

    /// INSERT INTO item_instance (guid, itemEntry, owner_guid, count, durability, enchantments, charges)
    /// VALUES (?, ?, ?, ?, ?, '', '')
    INS_ITEM_INSTANCE,

    /// INSERT INTO item_instance preserving generated loot flags/random/context metadata.
    INS_ITEM_INSTANCE_WITH_RANDOM_CONTEXT,

    /// INSERT INTO item_instance with the C++ Item::CloneItem persisted field subset.
    INS_ITEM_INSTANCE_CLONE,

    /// UPDATE item_instance SET count = ? WHERE guid = ?
    UPD_ITEM_INSTANCE_COUNT,

    /// UPDATE item_instance SET durability = ? WHERE guid = ?
    UPD_ITEM_INSTANCE_DURABILITY,

    /// UPDATE item_instance SET flags = ? WHERE guid = ?
    UPD_ITEM_INSTANCE_FLAGS,

    /// UPDATE item_instance SET enchantments = ? WHERE guid = ?
    UPD_ITEM_INSTANCE_ENCHANTMENTS,

    /// Persist the mutable `Item::SaveToDB` fields affected by storage moves.
    /// UPDATE item_instance SET count = ?, duration = ?, charges = ?, flags = ?,
    /// enchantments = ?, durability = ?, playedTime = ? WHERE guid = ?
    UPD_ITEM_INSTANCE_STORAGE_MUTABLE,

    /// SELECT entry, flags FROM character_gifts WHERE item_guid = ?
    SEL_CHARACTER_GIFT_BY_ITEM,

    /// DELETE FROM character_gifts WHERE item_guid = ?
    DEL_GIFT,

    /// UPDATE item_instance after opening a wrapped gift.
    UPD_ITEM_INSTANCE_OPEN_GIFT,

    /// INSERT INTO character_inventory (guid, bag, slot, item) VALUES (?, 0, ?, ?)
    INS_CHAR_INVENTORY,

    /// REPLACE INTO character_inventory (guid, bag, slot, item) VALUES (?, ?, ?, ?)
    REP_CHAR_INVENTORY_ITEM,

    /// DELETE FROM item_instance WHERE guid = ?
    DEL_ITEM_INSTANCE,

    /// DELETE FROM item_instance WHERE guid = ? AND owner_guid = ?
    DEL_ITEM_INSTANCE_BY_GUID_AND_OWNER,

    /// SELECT the durable owner and the character-inventory link for an uncaged item.
    SEL_UNCAGE_ITEM_STATE,

    /// INSERT one pending battle-pet trainer purchase saga command (issue #161).
    /// INSERT INTO character_battle_pet_purchase (request_key, guid, account_id,
    /// trainer_id, spell_id, species, breed, quality, display_id, level, price,
    /// money_before, money_after, status) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)
    INS_BATTLE_PET_PURCHASE,

    /// SELECT request_key, guid, account_id, trainer_id, spell_id, species, breed,
    /// quality, display_id, level, price, money_before, money_after, status,
    /// failure_reason, published FROM character_battle_pet_purchase WHERE request_key = ?
    SEL_BATTLE_PET_PURCHASE_BY_KEY,

    /// SELECT the same columns for every unconverged command of one character:
    /// pending/compensation rows plus Completed rows still owed a publication.
    /// ... WHERE guid = ? AND (status IN (0, 2) OR (status = 1 AND published = 0)) ... LIMIT ?
    SEL_BATTLE_PET_PURCHASE_PENDING,

    /// UPDATE character_battle_pet_purchase SET published = 1
    /// WHERE request_key = ? AND published = 0 AND status IN (0, 1, 2).
    /// Completed-but-unpublished rows are the recovery-publication signal.
    UPD_BATTLE_PET_PURCHASE_PUBLISHED,

    /// UPDATE character_battle_pet_purchase SET status = 1,
    /// failure_reason = NULL WHERE request_key = ? AND status IN (0, 2). The
    /// wider source guard also closes a recorded compensation decision once
    /// the Login DB receipt has proven the pet durable (issue #161 T3/T3').
    /// Completion deliberately does NOT set the publication marker: a
    /// Completed row with published = 0 is the recovery-publication signal.
    UPD_BATTLE_PET_PURCHASE_COMPLETED,

    /// UPDATE character_battle_pet_purchase SET status = 2, failure_reason = ?
    /// WHERE request_key = ? AND status = 0
    UPD_BATTLE_PET_PURCHASE_COMPENSATION_PENDING,

    /// UPDATE character_battle_pet_purchase SET status = 3
    /// WHERE request_key = ? AND status = 2
    UPD_BATTLE_PET_PURCHASE_COMPENSATED,

    /// UPDATE character_battle_pet_purchase SET status = 4, failure_reason = ?
    /// WHERE request_key = ? AND status = 2
    UPD_BATTLE_PET_PURCHASE_TERMINAL_FAILURE,

    /// Guarded saga charge: UPDATE characters SET money = ? WHERE guid = ? AND money = ?
    UPD_CHARACTER_MONEY_GUARDED,

    /// Idempotent saga refund: UPDATE characters SET money = LEAST(money + ?, ?) WHERE guid = ?
    UPD_CHARACTER_MONEY_REFUND,

    /// SELECT paidMoney, paidExtendedCost FROM item_refund_instance
    /// WHERE item_guid = ? AND player_guid = ? LIMIT 1
    SEL_ITEM_REFUNDS,

    /// SELECT allowedPlayers FROM item_soulbound_trade_data WHERE itemGuid = ? LIMIT 1
    SEL_ITEM_BOP_TRADE,

    /// DELETE FROM item_soulbound_trade_data WHERE itemGuid = ? LIMIT 1
    DEL_ITEM_BOP_TRADE,

    /// INSERT INTO item_soulbound_trade_data VALUES (?, ?)
    INS_ITEM_BOP_TRADE,

    /// C++ `CHAR_REP_INVENTORY_ITEM` canonical statement name.
    REP_INVENTORY_ITEM,

    /// C++ `CHAR_REP_ITEM_INSTANCE` full item persistence replace statement.
    REP_ITEM_INSTANCE,

    /// C++ `CHAR_UPD_ITEM_INSTANCE` full item persistence update statement.
    UPD_ITEM_INSTANCE,

    /// UPDATE item_instance SET duration = ?, flags = ?, durability = ? WHERE guid = ?
    UPD_ITEM_INSTANCE_ON_LOAD,

    /// DELETE FROM item_instance WHERE owner_guid = ?
    DEL_ITEM_INSTANCE_BY_OWNER,

    /// INSERT INTO item_instance_gems.
    INS_ITEM_INSTANCE_GEMS,

    /// DELETE FROM item_instance_gems WHERE itemGuid = ?
    DEL_ITEM_INSTANCE_GEMS,

    /// DELETE item gems by item owner.
    DEL_ITEM_INSTANCE_GEMS_BY_OWNER,

    /// INSERT INTO item_instance_transmog.
    INS_ITEM_INSTANCE_TRANSMOG,

    /// DELETE FROM item_instance_transmog WHERE itemGuid = ?
    DEL_ITEM_INSTANCE_TRANSMOG,

    /// DELETE item transmogs by item owner.
    DEL_ITEM_INSTANCE_TRANSMOG_BY_OWNER,

    /// UPDATE character_gifts SET guid = ? WHERE item_guid = ?
    UPD_GIFT_OWNER,

    /// SELECT account FROM characters WHERE name = ?
    SEL_ACCOUNT_BY_NAME,

    /// UPDATE characters SET account = ? WHERE guid = ?
    UPD_ACCOUNT_BY_GUID,

    /// SELECT matchMakerRating FROM character_arena_stats WHERE guid = ? AND slot = ?
    SEL_MATCH_MAKER_RATING,

    /// SELECT account, COUNT(guid) FROM characters WHERE account = ? GROUP BY account
    SEL_CHARACTER_COUNT,

    /// UPDATE characters SET name = ? WHERE guid = ?
    UPD_NAME_BY_GUID,

    /// INSERT INTO guild.
    INS_GUILD,

    /// DELETE FROM guild WHERE guildid = ?
    DEL_GUILD,

    /// UPDATE guild SET name = ? WHERE guildid = ?
    UPD_GUILD_NAME,

    /// INSERT INTO guild_member.
    INS_GUILD_MEMBER,

    /// DELETE FROM guild_member WHERE guid = ?
    DEL_GUILD_MEMBER,

    /// DELETE FROM guild_member WHERE guildid = ?
    DEL_GUILD_MEMBERS,

    /// INSERT INTO guild_rank.
    INS_GUILD_RANK,

    /// DELETE FROM guild_rank WHERE guildid = ?
    DEL_GUILD_RANKS,

    /// DELETE FROM guild_rank WHERE guildid = ? AND rid = ?
    DEL_GUILD_RANK,

    /// INSERT INTO guild_bank_tab.
    INS_GUILD_BANK_TAB,

    /// DELETE FROM guild_bank_tab WHERE guildid = ? AND TabId = ?
    DEL_GUILD_BANK_TAB,

    /// DELETE FROM guild_bank_tab WHERE guildid = ?
    DEL_GUILD_BANK_TABS,

    /// INSERT INTO guild_bank_item.
    INS_GUILD_BANK_ITEM,

    /// DELETE FROM guild_bank_item WHERE guildid = ? AND TabId = ? AND SlotId = ?
    DEL_GUILD_BANK_ITEM,

    /// DELETE FROM guild_bank_item WHERE guildid = ?
    DEL_GUILD_BANK_ITEMS,

    /// INSERT INTO guild_bank_right.
    INS_GUILD_BANK_RIGHT,

    /// DELETE FROM guild_bank_right WHERE guildid = ?
    DEL_GUILD_BANK_RIGHTS,

    /// DELETE FROM guild_bank_right WHERE guildid = ? AND rid = ?
    DEL_GUILD_BANK_RIGHTS_FOR_RANK,

    /// INSERT INTO guild_bank_eventlog.
    INS_GUILD_BANK_EVENTLOG,

    /// DELETE FROM guild_bank_eventlog WHERE guildid = ? AND LogGuid = ? AND TabId = ?
    DEL_GUILD_BANK_EVENTLOG,

    /// DELETE FROM guild_bank_eventlog WHERE guildid = ?
    DEL_GUILD_BANK_EVENTLOGS,

    /// INSERT INTO guild_eventlog.
    INS_GUILD_EVENTLOG,

    /// DELETE FROM guild_eventlog WHERE guildid = ? AND LogGuid = ?
    DEL_GUILD_EVENTLOG,

    /// DELETE FROM guild_eventlog WHERE guildid = ?
    DEL_GUILD_EVENTLOGS,

    /// UPDATE guild_member SET pnote = ? WHERE guid = ?
    UPD_GUILD_MEMBER_PNOTE,

    /// UPDATE guild_member SET offnote = ? WHERE guid = ?
    UPD_GUILD_MEMBER_OFFNOTE,

    /// UPDATE guild_member SET `rank` = ? WHERE guid = ?
    UPD_GUILD_MEMBER_RANK,

    /// UPDATE guild SET motd = ? WHERE guildid = ?
    UPD_GUILD_MOTD,

    /// UPDATE guild SET info = ? WHERE guildid = ?
    UPD_GUILD_INFO,

    /// UPDATE guild SET leaderguid = ? WHERE guildid = ?
    UPD_GUILD_LEADER,

    /// UPDATE guild_rank SET RankOrder = ? WHERE rid = ? AND guildid = ?
    UPD_GUILD_RANK_ORDER,

    /// UPDATE guild_rank SET rname = ? WHERE rid = ? AND guildid = ?
    UPD_GUILD_RANK_NAME,

    /// UPDATE guild_rank SET rights = ? WHERE rid = ? AND guildid = ?
    UPD_GUILD_RANK_RIGHTS,

    /// UPDATE guild emblem fields.
    UPD_GUILD_EMBLEM_INFO,

    /// UPDATE guild_bank_tab SET TabName = ?, TabIcon = ? WHERE guildid = ? AND TabId = ?
    UPD_GUILD_BANK_TAB_INFO,

    /// UPDATE guild SET BankMoney = ? WHERE guildid = ?
    UPD_GUILD_BANK_MONEY,

    /// UPDATE guild_rank SET BankMoneyPerDay = ? WHERE rid = ? AND guildid = ?
    UPD_GUILD_RANK_BANK_MONEY,

    /// UPDATE guild_bank_tab SET TabText = ? WHERE guildid = ? AND TabId = ?
    UPD_GUILD_BANK_TAB_TEXT,

    /// INSERT/UPDATE guild_member_withdraw tab limits.
    INS_GUILD_MEMBER_WITHDRAW_TABS,

    /// INSERT/UPDATE guild_member_withdraw money limit.
    INS_GUILD_MEMBER_WITHDRAW_MONEY,

    /// DELETE FROM guild_member_withdraw
    DEL_GUILD_MEMBER_WITHDRAW,

    /// SELECT name, level, race, class, gender, zone, account FROM characters WHERE guid = ?
    SEL_CHAR_DATA_FOR_GUILD,

    /// DELETE FROM guild_achievement WHERE guildId = ? AND achievement = ?
    DEL_GUILD_ACHIEVEMENT,

    /// INSERT INTO guild_achievement.
    INS_GUILD_ACHIEVEMENT,

    /// DELETE FROM guild_achievement_progress WHERE guildId = ? AND criteria = ?
    DEL_GUILD_ACHIEVEMENT_CRITERIA,

    /// INSERT INTO guild_achievement_progress.
    INS_GUILD_ACHIEVEMENT_CRITERIA,

    /// DELETE non-static guild achievements by guild id.
    DEL_ALL_GUILD_ACHIEVEMENTS,

    /// DELETE FROM guild_achievement_progress WHERE guildId = ?
    DEL_ALL_GUILD_ACHIEVEMENT_CRITERIA,

    /// SELECT achievement, date, guids FROM guild_achievement WHERE guildId = ?
    SEL_GUILD_ACHIEVEMENT,

    /// SELECT criteria, counter, date, completedGuid FROM guild_achievement_progress WHERE guildId = ?
    SEL_GUILD_ACHIEVEMENT_CRITERIA,

    /// INSERT/UPDATE guild_newslog.
    INS_GUILD_NEWS,

    /// INSERT/UPDATE channel row.
    UPD_CHANNEL,

    /// UPDATE channels SET lastUsed = UNIX_TIMESTAMP() WHERE name = ? AND team = ?
    UPD_CHANNEL_USAGE,

    /// UPDATE channels SET ownership = ? WHERE name LIKE ?
    UPD_CHANNEL_OWNERSHIP,

    /// DELETE FROM channels WHERE name = ? AND team = ?
    DEL_CHANNEL,

    /// DELETE old owned custom channels.
    DEL_OLD_CHANNELS,

    /// UPDATE character_equipmentsets.
    UPD_EQUIP_SET,

    /// INSERT INTO character_equipmentsets.
    INS_EQUIP_SET,

    /// DELETE FROM character_equipmentsets WHERE setguid=?
    DEL_EQUIP_SET,

    /// UPDATE character_transmog_outfits.
    UPD_TRANSMOG_OUTFIT,

    /// INSERT INTO character_transmog_outfits.
    INS_TRANSMOG_OUTFIT,

    /// DELETE FROM character_transmog_outfits WHERE setguid=?
    DEL_TRANSMOG_OUTFIT,

    /// INSERT INTO character_aura.
    INS_AURA,

    /// INSERT INTO character_aura_effect.
    INS_AURA_EFFECT,

    /// SELECT type, time, data FROM account_data WHERE accountId = ?
    SEL_ACCOUNT_DATA,

    /// REPLACE INTO account_data.
    REP_ACCOUNT_DATA,

    /// DELETE FROM account_data WHERE accountId = ?
    DEL_ACCOUNT_DATA,

    /// SELECT type, time, data FROM character_account_data WHERE guid = ?
    SEL_PLAYER_ACCOUNT_DATA,

    /// REPLACE INTO character_account_data.
    REP_PLAYER_ACCOUNT_DATA,

    /// DELETE FROM character_account_data WHERE guid = ?
    DEL_PLAYER_ACCOUNT_DATA,

    /// SELECT tutorials row for account.
    SEL_TUTORIALS,

    /// INSERT INTO account_tutorial.
    INS_TUTORIALS,

    /// UPDATE account_tutorial.
    UPD_TUTORIALS,

    /// DELETE FROM account_tutorial WHERE accountId = ?
    DEL_TUTORIALS,

    /// SELECT ownerguid, name FROM petition WHERE petitionguid = ?
    SEL_PETITION,

    /// SELECT playerguid FROM petition_sign WHERE petitionguid = ?
    SEL_PETITION_SIGNATURE,

    /// DELETE FROM petition_sign WHERE playerguid = ?
    DEL_ALL_PETITION_SIGNATURES,

    /// SELECT petitionguid FROM petition WHERE ownerguid = ?
    SEL_PETITION_BY_OWNER,

    /// SELECT ownerguid plus signature count for a petition.
    SEL_PETITION_SIGNATURES,

    /// SELECT playerguid FROM petition_sign WHERE player_account = ? AND petitionguid = ?
    SEL_PETITION_SIG_BY_ACCOUNT,

    /// SELECT ownerguid FROM petition WHERE petitionguid = ?
    SEL_PETITION_OWNER_BY_GUID,

    /// SELECT ownerguid, petitionguid FROM petition_sign WHERE playerguid = ?
    SEL_PETITION_SIG_BY_GUID,

    /// SELECT arenaTeamId, weekGames, seasonGames, seasonWins, personalRating FROM arena_team_member WHERE guid = ?
    SEL_CHARACTER_ARENAINFO,

    /// INSERT INTO arena_team.
    INS_ARENA_TEAM,

    /// INSERT INTO arena_team_member.
    INS_ARENA_TEAM_MEMBER,

    /// DELETE FROM arena_team where arenaTeamId = ?
    DEL_ARENA_TEAM,

    /// DELETE FROM arena_team_member WHERE arenaTeamId = ?
    DEL_ARENA_TEAM_MEMBERS,

    /// UPDATE arena_team SET captainGuid = ? WHERE arenaTeamId = ?
    UPD_ARENA_TEAM_CAPTAIN,

    /// DELETE FROM arena_team_member WHERE arenaTeamId = ? AND guid = ?
    DEL_ARENA_TEAM_MEMBER,

    /// UPDATE arena_team SET rating/week/season stats.
    UPD_ARENA_TEAM_STATS,

    /// UPDATE arena_team_member personal and weekly stats.
    UPD_ARENA_TEAM_MEMBER,

    /// DELETE FROM character_arena_stats WHERE guid = ?
    DEL_CHARACTER_ARENA_STATS,

    /// REPLACE INTO character_arena_stats.
    REP_CHARACTER_ARENA_STATS,

    /// UPDATE arena_team SET name = ? WHERE arenaTeamId = ?
    UPD_ARENA_TEAM_NAME,

    /// INSERT INTO character_battleground_data.
    INS_PLAYER_BGDATA,

    /// DELETE FROM character_battleground_data WHERE guid = ?
    DEL_PLAYER_BGDATA,

    /// INSERT INTO character_homebind.
    INS_PLAYER_HOMEBIND,

    /// UPDATE character_homebind SET map/zone/position.
    UPD_PLAYER_HOMEBIND,

    /// DELETE FROM character_homebind WHERE guid = ?
    DEL_PLAYER_HOMEBIND,

    /// SELECT corpse rows for one map and instance.
    SEL_CORPSES,

    /// INSERT INTO corpse.
    INS_CORPSE,

    /// DELETE FROM corpse WHERE guid = ?
    DEL_CORPSE,

    /// DELETE corpses and auxiliary rows for one map and instance.
    DEL_CORPSES_FROM_MAP,

    /// SELECT corpse phases for one map and instance.
    SEL_CORPSE_PHASES,

    /// INSERT INTO corpse_phases.
    INS_CORPSE_PHASES,

    /// DELETE FROM corpse_phases WHERE OwnerGuid = ?
    DEL_CORPSE_PHASES,

    /// SELECT corpse customizations for one map and instance.
    SEL_CORPSE_CUSTOMIZATIONS,

    /// INSERT INTO corpse_customizations.
    INS_CORPSE_CUSTOMIZATIONS,

    /// DELETE FROM corpse_customizations WHERE ownerGuid = ?
    DEL_CORPSE_CUSTOMIZATIONS,

    /// SELECT mapId, posX, posY, posZ, orientation FROM corpse WHERE guid = ?
    SEL_CORPSE_LOCATION,

    /// SELECT bag_ci.slot, ci.slot, ii.itemEntry, ci.item, ii.count, ii.durability, ii.context,
    /// ii.flags, ii.playedTime, ir.paidMoney, ir.paidExtendedCost
    /// FROM character_inventory ci
    /// JOIN character_inventory bag_ci ON bag_ci.guid = ci.guid AND bag_ci.item = ci.bag
    /// JOIN item_instance ii ON ci.item = ii.guid
    /// LEFT JOIN item_refund_instance ir ON ir.item_guid = ci.item AND ir.player_guid = ci.guid
    /// WHERE ci.guid = ? AND bag_ci.bag = 0 AND bag_ci.slot >= 30 AND bag_ci.slot < 34
    SEL_CHAR_BAG_CONTENTS,

    /// DELETE FROM item_refund_instance WHERE item_guid = ?
    DEL_ITEM_REFUND_INSTANCE,

    /// DELETE FROM item_loot_money WHERE container_id = ?
    DEL_ITEMCONTAINER_MONEY,

    /// DELETE FROM item_loot_items WHERE container_id = ?
    DEL_ITEMCONTAINER_ITEMS,

    /// DELETE FROM item_loot_items WHERE container_id = ? AND item_id = ? AND item_count = ? AND item_index = ?
    DEL_ITEMCONTAINER_ITEM,

    /// SELECT money FROM item_loot_money WHERE container_id = ?
    SEL_ITEMCONTAINER_MONEY,
    /// SELECT money FROM item_loot_money WHERE container_id = ? FOR UPDATE
    SEL_ITEMCONTAINER_MONEY_FOR_UPDATE,

    /// INSERT INTO item_loot_money (container_id, money) VALUES (?, ?)
    INS_ITEMCONTAINER_MONEY,

    /// SELECT item_loot_items rows for one container_id.
    SEL_ITEMCONTAINER_ITEMS,

    /// INSERT INTO item_loot_items with Trinity's stored item loot shape.
    INS_ITEMCONTAINER_ITEMS,

    /// INSERT INTO item_refund_instance (item_guid, player_guid, paidMoney, paidExtendedCost)
    /// VALUES (?, ?, ?, ?)
    INS_ITEM_REFUND_INSTANCE,

    /// INSERT IGNORE INTO character_spell (guid, spell, active, disabled) VALUES (?, ?, 1, 0)
    INS_CHARACTER_SPELL,

    /// Generated C++ `CharacterDatabase` prepared statement.
    GENERATED_CPP {
        /// C++ statement identifier, e.g. `CHAR_SEL_CHARACTER_MONEY`.
        ///
        /// Carried separately from the SQL so a persistence trace can name the
        /// statement without embedding its text. `Debug` on this variant would
        /// otherwise render the whole query, which would identify the
        /// statement by SQL and move every golden on a reformat — the exact
        /// coupling the trace contract exists to avoid.
        name: &'static str,
        /// Exact SQL from C++ `PrepareStatement(CHAR_..., ...)`.
        sql: &'static str,
    },
}
