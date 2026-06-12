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

    // Quest status
    SEL_CHAR_QUEST_STATUS,
    /// SELECT quest, objective, data FROM character_queststatus_objectives WHERE guid = ?
    SEL_CHAR_QUEST_STATUS_OBJECTIVES,
    /// SELECT quest, event, completedTime FROM character_queststatus_seasonal WHERE guid = ?
    SEL_CHAR_QUEST_STATUS_SEASONAL,
    INS_CHAR_QUEST_STATUS,
    DEL_CHAR_QUEST_STATUS,
    DEL_CHAR_QUEST_STATUS_OBJECTIVES_BY_QUEST,
    REP_CHAR_QUEST_STATUS_OBJECTIVES,

    /// UPDATE characters SET money = ? WHERE guid = ?
    UPD_CHAR_MONEY,
    /// UPDATE characters SET xp = ? WHERE guid = ?
    UPD_CHAR_XP,
    /// UPDATE characters SET level = ?, xp = ? WHERE guid = ?
    UPD_CHAR_LEVEL,

    /// SELECT MAX(guid) FROM item_instance
    SEL_MAX_ITEM_GUID,

    /// INSERT INTO item_instance (guid, itemEntry, owner_guid, count, durability, enchantments, charges)
    /// VALUES (?, ?, ?, ?, ?, '', '')
    INS_ITEM_INSTANCE,

    /// INSERT INTO item_instance preserving generated loot random property/context metadata.
    INS_ITEM_INSTANCE_WITH_RANDOM_CONTEXT,

    /// INSERT INTO item_instance with the C++ Item::CloneItem persisted field subset.
    INS_ITEM_INSTANCE_CLONE,

    /// UPDATE item_instance SET count = ? WHERE guid = ?
    UPD_ITEM_INSTANCE_COUNT,

    /// UPDATE item_instance SET durability = ? WHERE guid = ?
    UPD_ITEM_INSTANCE_DURABILITY,

    /// UPDATE item_instance SET flags = ? WHERE guid = ?
    UPD_ITEM_INSTANCE_FLAGS,

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
        /// Exact SQL from C++ `PrepareStatement(CHAR_..., ...)`.
        sql: &'static str,
    },
}

impl CharStatements {
    /// Build a generated C++ CharacterDatabase statement from exact SQL.
    pub const fn cpp(sql: &'static str) -> Self {
        Self::GENERATED_CPP { sql }
    }
}

impl StatementDef for CharStatements {
    fn sql(self) -> &'static str {
        match self {
            Self::DEL_POOL_QUEST_SAVE => "DELETE FROM pool_quest_save WHERE pool_id = ?",
            Self::INS_POOL_QUEST_SAVE => {
                "INSERT INTO pool_quest_save (pool_id, quest_id) VALUES (?, ?)"
            }
            Self::DEL_NONEXISTENT_GUILD_BANK_ITEM => {
                "DELETE FROM guild_bank_item WHERE guildid = ? AND TabId = ? AND SlotId = ?"
            }
            Self::DEL_EXPIRED_BANS => {
                "UPDATE character_banned SET active = 0 WHERE unbandate <= UNIX_TIMESTAMP() AND unbandate <> bandate"
            }
            Self::SEL_ENUM => {
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
            }
            Self::SEL_ENUM_DECLINED_NAME => {
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
            }
            Self::SEL_ENUM_CUSTOMIZATIONS => {
                "SELECT cc.guid, cc.chrCustomizationOptionID, cc.chrCustomizationChoiceID FROM character_customizations cc \
                 LEFT JOIN characters c ON cc.guid = c.guid WHERE c.account = ? AND c.deleteInfos_Name IS NULL ORDER BY cc.guid, cc.chrCustomizationOptionID"
            }
            Self::SEL_UNDELETE_ENUM => {
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
            }
            Self::SEL_UNDELETE_ENUM_DECLINED_NAME => {
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
            }
            Self::SEL_UNDELETE_ENUM_CUSTOMIZATIONS => {
                "SELECT cc.guid, cc.chrCustomizationOptionID, cc.chrCustomizationChoiceID FROM character_customizations cc \
                 LEFT JOIN characters c ON cc.guid = c.guid WHERE c.deleteInfos_Account = ? AND c.deleteInfos_Name IS NOT NULL ORDER BY cc.guid, cc.chrCustomizationOptionID"
            }
            Self::SEL_CHECK_NAME => "SELECT 1 FROM characters WHERE name = ?",
            Self::SEL_CHECK_GUID => "SELECT 1 FROM characters WHERE guid = ?",
            Self::SEL_SUM_CHARS => {
                "SELECT COUNT(guid) FROM characters WHERE account = ? AND deleteDate IS NULL"
            }
            Self::SEL_CHAR_CREATE_INFO => {
                "SELECT level, race, class FROM characters WHERE account = ? LIMIT 0, ?"
            }
            Self::INS_CHARACTER_BAN => {
                "INSERT INTO character_banned (guid, bandate, unbandate, bannedby, banreason, active) VALUES (?, UNIX_TIMESTAMP(), UNIX_TIMESTAMP()+?, ?, ?, 1)"
            }
            Self::UPD_CHARACTER_BAN => {
                "UPDATE character_banned SET active = 0 WHERE guid = ? AND active != 0"
            }
            Self::DEL_CHARACTER_BAN => {
                "DELETE cb FROM character_banned cb INNER JOIN characters c ON c.guid = cb.guid WHERE c.account = ?"
            }
            Self::SEL_BANINFO => {
                "SELECT bandate, unbandate-bandate, active, unbandate, banreason, bannedby FROM character_banned WHERE guid = ? ORDER BY bandate ASC"
            }
            Self::SEL_GUID_BY_NAME_FILTER => {
                "SELECT guid, name FROM characters WHERE name LIKE CONCAT('%%', ?, '%%')"
            }
            Self::SEL_BANINFO_LIST => {
                "SELECT bandate, unbandate, bannedby, banreason FROM character_banned WHERE guid = ? ORDER BY unbandate"
            }
            Self::SEL_BANNED_NAME => {
                "SELECT characters.name FROM characters, character_banned WHERE character_banned.guid = ? AND character_banned.guid = characters.guid"
            }
            Self::SEL_MAIL_LIST_COUNT => "SELECT COUNT(id) FROM mail WHERE receiver = ? ",
            Self::SEL_MAIL_LIST_INFO => {
                "SELECT id, sender, (SELECT name FROM characters WHERE guid = sender) AS sendername, receiver, (SELECT name FROM characters WHERE guid = receiver) AS receivername, subject, deliver_time, expire_time, money, has_items FROM mail WHERE receiver = ? "
            }
            Self::SEL_MAIL_LIST_ITEMS => "SELECT itemEntry,count FROM item_instance WHERE guid = ?",
            Self::SEL_FREE_NAME => {
                "SELECT name, at_login FROM characters WHERE guid = ? AND NOT EXISTS (SELECT NULL FROM characters WHERE name = ?)"
            }
            Self::SEL_CHAR_ZONE => "SELECT zone FROM characters WHERE guid = ?",
            Self::SEL_CHAR_POSITION_XYZ => {
                "SELECT map, position_x, position_y, position_z FROM characters WHERE guid = ?"
            }
            Self::SEL_CHAR_POSITION => {
                "SELECT position_x, position_y, position_z, orientation, map, taxi_path FROM characters WHERE guid = ?"
            }
            Self::DEL_BATTLEGROUND_RANDOM_ALL => "DELETE FROM character_battleground_random",
            Self::DEL_BATTLEGROUND_RANDOM => {
                "DELETE FROM character_battleground_random WHERE guid = ?"
            }
            Self::INS_BATTLEGROUND_RANDOM => {
                "INSERT INTO character_battleground_random (guid) VALUES (?)"
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
            Self::DEL_CHARACTER => "DELETE FROM characters WHERE guid = ?",
            Self::DEL_CHAR_REPUTATION_BY_FACTION => {
                "DELETE FROM character_reputation WHERE guid = ? AND faction = ?"
            }
            Self::INS_CHAR_REPUTATION_BY_FACTION => {
                "INSERT INTO character_reputation (guid, faction, standing, flags) VALUES (?, ?, ? , ?)"
            }
            Self::DEL_CHAR_REPUTATION => "DELETE FROM character_reputation WHERE guid = ?",
            Self::SEL_CHARACTER => {
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
            }
            Self::SEL_CHARACTER_CUSTOMIZATIONS => {
                "SELECT chrCustomizationOptionID, chrCustomizationChoiceID FROM character_customizations WHERE guid = ? ORDER BY chrCustomizationOptionID"
            }
            Self::SEL_GROUP_MEMBER => "SELECT guid FROM group_member WHERE memberGuid = ?",
            Self::SEL_CHARACTER_AURAS => {
                "SELECT casterGuid, itemGuid, spell, effectMask, recalculateMask, difficulty, stackCount, maxDuration, remainTime, remainCharges, castItemId, castItemLevel FROM character_aura WHERE guid = ?"
            }
            Self::SEL_CHARACTER_AURA_EFFECTS => {
                "SELECT casterGuid, itemGuid, spell, effectMask, effectIndex, amount, baseAmount FROM character_aura_effect WHERE guid = ?"
            }
            Self::UPD_CHAR_ONLINE => "UPDATE characters SET online = 1 WHERE guid = ?",
            Self::UPD_CHAR_OFFLINE => "UPDATE characters SET online = 0 WHERE guid = ?",
            Self::SEL_CHAR_DEL_CHECK => {
                "SELECT guid, account FROM characters WHERE guid = ? AND account = ?"
            }
            Self::SEL_MAX_GUID => "SELECT MAX(guid) FROM characters",
            Self::SEL_CHAR_EQUIPMENT => {
                "SELECT ci.slot, ii.itemEntry, ci.item, ii.count, ii.durability, ii.context, \
                 ii.flags, ii.playedTime, ir.paidMoney, ir.paidExtendedCost \
                 FROM character_inventory ci \
                 JOIN item_instance ii ON ci.item = ii.guid \
                 LEFT JOIN item_refund_instance ir \
                   ON ir.item_guid = ci.item AND ir.player_guid = ci.guid \
                 WHERE ci.guid = ? AND ci.bag = 0"
            }
            Self::UPD_CHAR_INVENTORY_SLOT => {
                "UPDATE character_inventory SET slot = ? WHERE guid = ? AND item = ?"
            }
            Self::DEL_CHAR_INVENTORY_ITEM => {
                "DELETE FROM character_inventory WHERE guid = ? AND item = ?"
            }
            Self::SEL_CHARACTER_SKILLS => {
                "SELECT skill, value, max, professionSlot FROM character_skills WHERE guid = ?"
            }
            Self::SEL_CHARACTER_SPELL => {
                "SELECT spell, active, disabled FROM character_spell WHERE guid = ?"
            }
            Self::SEL_CHARACTER_SPELL_FAVORITES => {
                "SELECT spell FROM character_spell_favorite WHERE guid = ?"
            }
            Self::SEL_CHARACTER_QUESTSTATUS_OBJECTIVES_CRITERIA => {
                "SELECT questObjectiveId FROM character_queststatus_objectives_criteria WHERE guid = ?"
            }
            Self::SEL_CHARACTER_QUESTSTATUS_OBJECTIVES_CRITERIA_PROGRESS => {
                "SELECT criteriaId, counter, date FROM character_queststatus_objectives_criteria_progress WHERE guid = ?"
            }
            Self::SEL_CHARACTER_QUESTSTATUS_DAILY => {
                "SELECT quest, time FROM character_queststatus_daily WHERE guid = ?"
            }
            Self::SEL_CHARACTER_QUESTSTATUS_WEEKLY => {
                "SELECT quest FROM character_queststatus_weekly WHERE guid = ?"
            }
            Self::SEL_CHARACTER_QUESTSTATUS_MONTHLY => {
                "SELECT quest FROM character_queststatus_monthly WHERE guid = ?"
            }
            Self::SEL_CHARACTER_QUESTSTATUS_SEASONAL => {
                "SELECT quest, event, completedTime FROM character_queststatus_seasonal WHERE guid = ?"
            }
            Self::SEL_CHARACTER_REPUTATION => {
                "SELECT faction, standing, flags FROM character_reputation WHERE guid = ?"
            }
            Self::SEL_MAIL_COUNT => "SELECT COUNT(*) FROM mail WHERE receiver = ?",
            Self::SEL_CHARACTER_SOCIALLIST => {
                "SELECT cs.friend, c.account, cs.flags, cs.note FROM character_social cs JOIN characters c ON c.guid = cs.friend WHERE cs.guid = ? AND c.deleteinfos_name IS NULL LIMIT 255"
            }
            Self::SEL_CHARACTER_HOMEBIND => {
                "SELECT mapId, zoneId, posX, posY, posZ, orientation FROM character_homebind WHERE guid = ?"
            }
            Self::SEL_CHARACTER_SPELLCOOLDOWNS => {
                "SELECT spell, item, time, categoryId, categoryEnd FROM character_spell_cooldown WHERE guid = ? AND time > UNIX_TIMESTAMP()"
            }
            Self::SEL_CHARACTER_SPELL_CHARGES => {
                "SELECT categoryId, rechargeStart, rechargeEnd FROM character_spell_charges WHERE guid = ? AND rechargeEnd > UNIX_TIMESTAMP() ORDER BY rechargeEnd"
            }
            Self::SEL_CHARACTER_DECLINEDNAMES => {
                "SELECT genitive, dative, accusative, instrumental, prepositional FROM character_declinedname WHERE guid = ?"
            }
            Self::SEL_GUILD_MEMBER => "SELECT guildid, `rank` FROM guild_member WHERE guid = ?",
            Self::SEL_GUILD_MEMBER_EXTENDED => {
                "SELECT g.guildid, g.name, gr.rname, gr.rid, gm.pnote, gm.offnote FROM guild g JOIN guild_member gm ON g.guildid = gm.guildid JOIN guild_rank gr ON g.guildid = gr.guildid AND gm.`rank` = gr.rid WHERE gm.guid = ?"
            }
            Self::SEL_CHARACTER_ACHIEVEMENTS => {
                "SELECT achievement, date FROM character_achievement WHERE guid = ?"
            }
            Self::SEL_CHARACTER_CRITERIAPROGRESS => {
                "SELECT criteria, counter, date FROM character_achievement_progress WHERE guid = ?"
            }
            Self::SEL_CHARACTER_EQUIPMENTSETS => {
                "SELECT setguid, setindex, name, iconname, ignore_mask, AssignedSpecIndex, item0, item1, item2, item3, item4, item5, item6, item7, item8, item9, item10, item11, item12, item13, item14, item15, item16, item17, item18 FROM character_equipmentsets WHERE guid = ? ORDER BY setindex"
            }
            Self::SEL_CHARACTER_TRANSMOG_OUTFITS => {
                "SELECT setguid, setindex, name, iconname, ignore_mask, appearance0, appearance1, appearance2, appearance3, appearance4, appearance5, appearance6, appearance7, appearance8, appearance9, appearance10, appearance11, appearance12, appearance13, appearance14, appearance15, appearance16, appearance17, appearance18, mainHandEnchant, offHandEnchant FROM character_transmog_outfits WHERE guid = ? ORDER BY setindex"
            }
            Self::SEL_CHARACTER_BGDATA => {
                "SELECT instanceId, team, joinX, joinY, joinZ, joinO, joinMapId, taxiStart, taxiEnd, mountSpell, queueId FROM character_battleground_data WHERE guid = ?"
            }
            Self::SEL_CHARACTER_GLYPHS => {
                "SELECT talentGroup, glyphSlot, glyphId FROM character_glyphs WHERE guid = ?"
            }
            Self::SEL_CHARACTER_TALENTS => {
                "SELECT talentId, talentRank, talentGroup FROM character_talent WHERE guid = ?"
            }
            Self::SEL_CHARACTER_RANDOMBG => {
                "SELECT guid FROM character_battleground_random WHERE guid = ?"
            }
            Self::SEL_CHARACTER_BANNED => {
                "SELECT guid FROM character_banned WHERE guid = ? AND active = 1"
            }
            Self::SEL_CHARACTER_QUESTSTATUSREW => {
                "SELECT quest FROM character_queststatus_rewarded WHERE guid = ? AND active = 1"
            }
            Self::SEL_CHARACTER_FAVORITE_AUCTIONS => {
                "SELECT `order`, itemId, itemLevel, battlePetSpeciesId, suffixItemNameDescriptionId FROM character_favorite_auctions WHERE guid = ? ORDER BY `order`"
            }
            Self::INS_CHARACTER_FAVORITE_AUCTION => {
                "INSERT INTO character_favorite_auctions (guid, `order`, itemId, itemLevel, battlePetSpeciesId, suffixItemNameDescriptionId) VALUE (?, ?, ?, ?, ?, ?)"
            }
            Self::DEL_CHARACTER_FAVORITE_AUCTION => {
                "DELETE FROM character_favorite_auctions WHERE guid = ? AND `order` = ?"
            }
            Self::DEL_CHARACTER_FAVORITE_AUCTIONS_BY_CHAR => {
                "DELETE FROM character_favorite_auctions WHERE guid = ?"
            }
            Self::SEL_PLAYER_CURRENCY => {
                "SELECT Currency, Quantity, WeeklyQuantity, TrackedQuantity, \
                 IncreasedCapQuantity, EarnedQuantity, Flags \
                 FROM character_currency WHERE CharacterGuid = ?"
            }
            Self::UPD_PLAYER_CURRENCY => {
                "UPDATE character_currency SET Quantity = ?, WeeklyQuantity = ?, \
                 TrackedQuantity = ?, IncreasedCapQuantity = ?, EarnedQuantity = ?, Flags = ? \
                 WHERE CharacterGuid = ? AND Currency = ?"
            }
            Self::REP_PLAYER_CURRENCY => {
                "REPLACE INTO character_currency \
                 (CharacterGuid, Currency, Quantity, WeeklyQuantity, TrackedQuantity, \
                  IncreasedCapQuantity, EarnedQuantity, Flags) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::SEL_CHARACTER_ACTIONS_SPEC => {
                "SELECT button, action, type FROM character_action \
                 WHERE guid = ? AND spec = ? AND traitConfigId = ? ORDER BY button"
            }
            Self::INS_CHARACTER_ACTION => {
                "INSERT INTO character_action (guid, spec, traitConfigId, button, action, type) \
                 VALUES (?, 0, 0, ?, ?, ?)"
            }
            Self::UPD_GROUP_TYPE => "UPDATE `groups` SET groupType = ? WHERE guid = ?",
            Self::UPD_GROUP_LEADER => "UPDATE `groups` SET leaderGuid = ? WHERE guid = ?",
            Self::INS_GROUP => {
                "INSERT INTO `groups` (guid, leaderGuid, lootMethod, looterGuid, lootThreshold, icon1, icon2, icon3, icon4, icon5, icon6, icon7, icon8, groupType, difficulty, raidDifficulty, legacyRaidDifficulty, masterLooterGuid) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::INS_GROUP_MEMBER => {
                "INSERT INTO group_member (guid, memberGuid, memberFlags, subgroup, roles) VALUES(?, ?, ?, ?, ?)"
            }
            Self::UPD_GROUP_MEMBER_SUBGROUP => {
                "UPDATE group_member SET subgroup = ? WHERE memberGuid = ?"
            }
            Self::UPD_GROUP_MEMBER_FLAG => {
                "UPDATE group_member SET memberFlags = ? WHERE memberGuid = ?"
            }
            Self::DEL_GROUP_MEMBER => "DELETE FROM group_member WHERE memberGuid = ?",
            Self::DEL_GROUP => "DELETE FROM `groups` WHERE guid = ?",
            Self::DEL_GROUP_MEMBER_ALL => "DELETE FROM group_member WHERE guid = ?",
            Self::DEL_LFG_DATA => "DELETE FROM lfg_data WHERE guid = ?",
            Self::DEL_GROUP_MEMBERS_WITHOUT_CHARACTER => {
                "DELETE FROM group_member WHERE memberGuid NOT IN (SELECT guid FROM characters)"
            }
            Self::DEL_GROUPS_WITHOUT_LEADER => {
                "DELETE FROM `groups` WHERE leaderGuid NOT IN (SELECT guid FROM characters)"
            }
            Self::DEL_GROUPS_WITH_FEWER_THAN_TWO_MEMBERS => {
                "DELETE FROM `groups` WHERE guid NOT IN (SELECT guid FROM group_member GROUP BY guid HAVING COUNT(guid) > 1)"
            }
            Self::DEL_GROUP_MEMBERS_WITHOUT_GROUP => {
                "DELETE FROM group_member WHERE guid NOT IN (SELECT guid FROM `groups`)"
            }
            Self::SEL_GROUPS => {
                "SELECT g.leaderGuid, g.lootMethod, g.looterGuid, g.lootThreshold, g.icon1, g.icon2, g.icon3, g.icon4, g.icon5, g.icon6, g.icon7, g.icon8, g.groupType, g.difficulty, g.raiddifficulty, g.legacyRaidDifficulty, g.masterLooterGuid, g.guid, lfg.dungeon, lfg.state FROM `groups` g LEFT JOIN lfg_data lfg ON lfg.guid = g.guid ORDER BY g.guid ASC"
            }
            Self::SEL_GROUP_MEMBERS => {
                "SELECT guid, memberGuid, memberFlags, subgroup, roles FROM group_member ORDER BY guid"
            }
            Self::UPD_CHAR_PLAYED_TIME => {
                "UPDATE characters SET totaltime = ?, leveltime = ? WHERE guid = ?"
            }
            Self::SEL_ACCOUNT_INSTANCELOCKTIMES => {
                "SELECT instanceId, releaseTime FROM account_instance_times WHERE accountId = ?"
            }
            Self::SEL_AUCTIONS => {
                "SELECT id, auctionHouseId, owner, bidder, minBid, buyoutOrUnitPrice, deposit, bidAmount, startTime, endTime, serverFlags FROM auctionhouse"
            }
            Self::INS_AUCTION_ITEMS => {
                "INSERT INTO auction_items (auctionId, itemGuid) VALUES (?, ?)"
            }
            Self::DEL_AUCTION_ITEMS_BY_ITEM => "DELETE FROM auction_items WHERE itemGuid = ?",
            Self::SEL_AUCTION_BIDDERS => "SELECT auctionId, playerGuid FROM auction_bidders",
            Self::INS_AUCTION_BIDDER => {
                "INSERT INTO auction_bidders (auctionId, playerGuid) VALUES (?, ?)"
            }
            Self::DEL_AUCTION_BIDDER_BY_PLAYER => {
                "DELETE FROM auction_bidders WHERE playerGuid = ?"
            }
            Self::INS_AUCTION => {
                "INSERT INTO auctionhouse (id, auctionHouseId, owner, bidder, minBid, buyoutOrUnitPrice, deposit, bidAmount, startTime, endTime, serverFlags) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::DEL_AUCTION => {
                "DELETE a, ab, ai FROM auctionhouse a LEFT JOIN auction_items ai ON a.id = ai.auctionId LEFT JOIN auction_bidders ab ON a.id = ab.auctionId WHERE a.id = ?"
            }
            Self::UPD_AUCTION_BID => {
                "UPDATE auctionhouse SET bidder = ?, bidAmount = ?, serverFlags = ? WHERE id = ?"
            }
            Self::UPD_AUCTION_EXPIRATION => "UPDATE auctionhouse SET endTime = ? WHERE id = ?",
            Self::INS_MAIL => {
                "INSERT INTO mail(id, messageType, stationery, mailTemplateId, sender, receiver, subject, body, has_items, expire_time, deliver_time, money, cod, checked) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::DEL_MAIL_BY_ID => "DELETE FROM mail WHERE id = ?",
            Self::INS_MAIL_ITEM => {
                "INSERT INTO mail_items(mail_id, item_guid, receiver) VALUES (?, ?, ?)"
            }
            Self::DEL_MAIL_ITEM => "DELETE FROM mail_items WHERE item_guid = ?",
            Self::DEL_INVALID_MAIL_ITEM => "DELETE FROM mail_items WHERE item_guid = ?",
            Self::DEL_EMPTY_EXPIRED_MAIL => {
                "DELETE FROM mail WHERE expire_time < ? AND has_items = 0 AND body = ''"
            }
            Self::SEL_EXPIRED_MAIL => {
                "SELECT id, messageType, sender, receiver, has_items, expire_time, cod, checked, mailTemplateId FROM mail WHERE expire_time < ?"
            }
            Self::SEL_EXPIRED_MAIL_ITEMS => {
                "SELECT item_guid, itemEntry, mail_id FROM mail_items mi INNER JOIN item_instance ii ON ii.guid = mi.item_guid LEFT JOIN mail mm ON mi.mail_id = mm.id WHERE mm.id IS NOT NULL AND mm.expire_time < ?"
            }
            Self::UPD_MAIL_RETURNED => {
                "UPDATE mail SET sender = ?, receiver = ?, expire_time = ?, deliver_time = ?, cod = 0, checked = ? WHERE id = ?"
            }
            Self::UPD_MAIL_ITEM_RECEIVER => {
                "UPDATE mail_items SET receiver = ? WHERE item_guid = ?"
            }
            Self::UPD_ITEM_OWNER => "UPDATE item_instance SET owner_guid = ? WHERE guid = ?",
            Self::DEL_ACCOUNT_INSTANCE_LOCK_TIMES => {
                "DELETE FROM account_instance_times WHERE accountId = ?"
            }
            Self::INS_ACCOUNT_INSTANCE_LOCK_TIMES => {
                "INSERT INTO account_instance_times (accountId, instanceId, releaseTime) VALUES (?, ?, ?)"
            }
            Self::SEL_INSTANCE => {
                "SELECT instanceId, data, completedEncountersMask, entranceWorldSafeLocId FROM instance"
            }
            Self::SEL_CHARACTER_INSTANCE_LOCK => {
                "SELECT guid, mapId, lockId, instanceId, difficulty, data, completedEncountersMask, \
                 entranceWorldSafeLocId, expiryTime, extended FROM character_instance_lock ORDER BY instanceId"
            }
            Self::DEL_CHARACTER_INSTANCE_LOCK => {
                "DELETE FROM character_instance_lock WHERE guid = ? AND mapId = ? AND lockId = ?"
            }
            Self::DEL_CHARACTER_INSTANCE_LOCK_BY_GUID => {
                "DELETE FROM character_instance_lock WHERE guid = ?"
            }
            Self::INS_CHARACTER_INSTANCE_LOCK => {
                "INSERT INTO character_instance_lock \
                 (guid, mapId, lockId, instanceId, difficulty, data, completedEncountersMask, \
                  entranceWorldSafeLocId, expiryTime, extended) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::UPD_CHARACTER_INSTANCE_LOCK_EXTENSION => {
                "UPDATE character_instance_lock SET extended = ? WHERE guid = ? AND mapId = ? AND lockId = ?"
            }
            Self::UPD_CHARACTER_INSTANCE_LOCK_FORCE_EXPIRE => {
                "UPDATE character_instance_lock SET expiryTime = ?, extended = 0 WHERE guid = ? AND mapId = ? AND lockId = ?"
            }
            Self::DEL_INSTANCE => "DELETE FROM instance WHERE instanceId = ?",
            Self::INS_INSTANCE => {
                "INSERT INTO instance (instanceId, data, completedEncountersMask, entranceWorldSafeLocId) VALUES (?, ?, ?, ?)"
            }
            Self::SEL_RESPAWNS => {
                "SELECT type, spawnId, respawnTime FROM respawn WHERE mapId = ? AND instanceId = ?"
            }
            Self::SEL_ALL_RESPAWNS => {
                "SELECT type, spawnId, respawnTime, mapId, instanceId FROM respawn"
            }
            Self::REP_RESPAWN => {
                "REPLACE INTO respawn (type, spawnId, respawnTime, mapId, instanceId) VALUES (?, ?, ?, ?, ?)"
            }
            Self::DEL_RESPAWN => {
                "DELETE FROM respawn WHERE type = ? AND spawnId = ? AND mapId = ? AND instanceId = ?"
            }
            Self::DEL_ALL_RESPAWNS => "DELETE FROM respawn WHERE mapId = ? AND instanceId = ?",
            Self::DEL_GAME_EVENT_SAVE => "DELETE FROM game_event_save WHERE eventEntry = ?",
            Self::INS_GAME_EVENT_SAVE => {
                "INSERT INTO game_event_save (eventEntry, state, next_start) VALUES (?, ?, ?)"
            }
            Self::SEL_GAME_EVENT_CONDITION_SAVES => {
                "SELECT eventEntry, condition_id, done FROM game_event_condition_save"
            }
            Self::DEL_ALL_GAME_EVENT_CONDITION_SAVE => {
                "DELETE FROM game_event_condition_save WHERE eventEntry = ?"
            }
            Self::DEL_GAME_EVENT_CONDITION_SAVE => {
                "DELETE FROM game_event_condition_save WHERE eventEntry = ? AND condition_id = ?"
            }
            Self::INS_GAME_EVENT_CONDITION_SAVE => {
                "INSERT INTO game_event_condition_save (eventEntry, condition_id, done) VALUES (?, ?, ?)"
            }
            Self::DEL_RESET_CHARACTER_QUESTSTATUS_SEASONAL_BY_EVENT => {
                "DELETE FROM character_queststatus_seasonal WHERE event = ? AND completedTime < ?"
            }
            Self::DEL_CHARACTER_QUESTSTATUS_DAILY => {
                "DELETE FROM character_queststatus_daily WHERE guid = ?"
            }
            Self::DEL_CHARACTER_QUESTSTATUS_WEEKLY => {
                "DELETE FROM character_queststatus_weekly WHERE guid = ?"
            }
            Self::DEL_CHARACTER_QUESTSTATUS_MONTHLY => {
                "DELETE FROM character_queststatus_monthly WHERE guid = ?"
            }
            Self::DEL_CHARACTER_QUESTSTATUS_SEASONAL => {
                "DELETE FROM character_queststatus_seasonal WHERE guid = ?"
            }
            Self::INS_CHARACTER_QUESTSTATUS_DAILY => {
                "INSERT INTO character_queststatus_daily (guid, quest, time) VALUES (?, ?, ?)"
            }
            Self::INS_CHARACTER_QUESTSTATUS_WEEKLY => {
                "INSERT INTO character_queststatus_weekly (guid, quest) VALUES (?, ?)"
            }
            Self::INS_CHARACTER_QUESTSTATUS_MONTHLY => {
                "INSERT INTO character_queststatus_monthly (guid, quest) VALUES (?, ?)"
            }
            Self::INS_CHARACTER_QUESTSTATUS_SEASONAL => {
                "INSERT INTO character_queststatus_seasonal (guid, quest, event, completedTime) VALUES (?, ?, ?, ?)"
            }
            Self::SEL_WORLD_STATE_VALUES => "SELECT Id, Value FROM world_state_value",
            Self::REP_WORLD_STATE => "REPLACE INTO world_state_value (Id, Value) VALUES (?, ?)",
            Self::UPD_CHAR_XP => "UPDATE characters SET xp = ? WHERE guid = ?",
            Self::UPD_CHAR_LEVEL => "UPDATE characters SET level = ?, xp = ? WHERE guid = ?",
            Self::UPD_CHAR_MONEY => "UPDATE characters SET money = ? WHERE guid = ?",
            Self::SEL_MAX_ITEM_GUID => "SELECT MAX(guid) FROM item_instance",
            Self::INS_ITEM_INSTANCE => {
                "INSERT INTO item_instance \
                 (guid, itemEntry, owner_guid, creatorGuid, giftCreatorGuid, count, \
                  durability, enchantments, charges, flags, randomPropertiesId, \
                  randomPropertiesSeed, context) \
                 VALUES (?, ?, ?, 0, 0, ?, ?, '', '', 0, 0, 0, 0)"
            }
            Self::INS_ITEM_INSTANCE_WITH_RANDOM_CONTEXT => {
                "INSERT INTO item_instance \
                 (guid, itemEntry, owner_guid, creatorGuid, giftCreatorGuid, count, \
                  durability, enchantments, charges, flags, randomPropertiesId, \
                  randomPropertiesSeed, context) \
                 VALUES (?, ?, ?, 0, 0, ?, ?, '', '', 0, ?, ?, ?)"
            }
            Self::INS_ITEM_INSTANCE_CLONE => {
                "INSERT INTO item_instance \
                 (guid, itemEntry, owner_guid, creatorGuid, giftCreatorGuid, count, \
                  duration, charges, flags, durability, playedTime, randomPropertiesId, \
                  randomPropertiesSeed, context) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::UPD_ITEM_INSTANCE_COUNT => "UPDATE item_instance SET count = ? WHERE guid = ?",
            Self::UPD_ITEM_INSTANCE_DURABILITY => {
                "UPDATE item_instance SET durability = ? WHERE guid = ?"
            }
            Self::UPD_ITEM_INSTANCE_FLAGS => "UPDATE item_instance SET flags = ? WHERE guid = ?",
            Self::SEL_CHARACTER_GIFT_BY_ITEM => {
                "SELECT entry, flags FROM character_gifts WHERE item_guid = ?"
            }
            Self::DEL_GIFT => "DELETE FROM character_gifts WHERE item_guid = ?",
            Self::UPD_ITEM_INSTANCE_OPEN_GIFT => {
                "UPDATE item_instance SET itemEntry = ?, giftCreatorGuid = 0, flags = ?, durability = ? WHERE guid = ?"
            }
            Self::INS_CHAR_INVENTORY => {
                "INSERT INTO character_inventory (guid, bag, slot, item) VALUES (?, 0, ?, ?)"
            }
            Self::REP_CHAR_INVENTORY_ITEM => {
                "REPLACE INTO character_inventory (guid, bag, slot, item) VALUES (?, ?, ?, ?)"
            }
            Self::DEL_ITEM_INSTANCE => "DELETE FROM item_instance WHERE guid = ?",
            Self::SEL_ITEM_REFUNDS => {
                "SELECT paidMoney, paidExtendedCost \
                 FROM item_refund_instance WHERE item_guid = ? AND player_guid = ? LIMIT 1"
            }
            Self::SEL_ITEM_BOP_TRADE => {
                "SELECT allowedPlayers FROM item_soulbound_trade_data WHERE itemGuid = ? LIMIT 1"
            }
            Self::DEL_ITEM_BOP_TRADE => {
                "DELETE FROM item_soulbound_trade_data WHERE itemGuid = ? LIMIT 1"
            }
            Self::INS_ITEM_BOP_TRADE => "INSERT INTO item_soulbound_trade_data VALUES (?, ?)",
            Self::REP_INVENTORY_ITEM => {
                "REPLACE INTO character_inventory (guid, bag, slot, item) VALUES (?, ?, ?, ?)"
            }
            Self::REP_ITEM_INSTANCE => {
                "REPLACE INTO item_instance (itemEntry, owner_guid, creatorGuid, giftCreatorGuid, count, duration, charges, flags, enchantments, durability, playedTime, text, battlePetSpeciesId, battlePetBreedData, battlePetLevel, battlePetDisplayId, randomPropertiesId, randomPropertiesSeed, context, guid) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::UPD_ITEM_INSTANCE => {
                "UPDATE item_instance SET itemEntry = ?, owner_guid = ?, creatorGuid = ?, giftCreatorGuid = ?, count = ?, duration = ?, charges = ?, flags = ?, enchantments = ?, durability = ?, playedTime = ?, text = ?, battlePetSpeciesId = ?, battlePetBreedData = ?, battlePetLevel = ?, battlePetDisplayId = ?, randomPropertiesId = ?, randomPropertiesSeed = ?, context = ? WHERE guid = ?"
            }
            Self::UPD_ITEM_INSTANCE_ON_LOAD => {
                "UPDATE item_instance SET duration = ?, flags = ?, durability = ? WHERE guid = ?"
            }
            Self::DEL_ITEM_INSTANCE_BY_OWNER => "DELETE FROM item_instance WHERE owner_guid = ?",
            Self::INS_ITEM_INSTANCE_GEMS => {
                "INSERT INTO item_instance_gems (itemGuid, gemItemId1, gemBonuses1, gemContext1, gemItemId2, gemBonuses2, gemContext2, gemItemId3, gemBonuses3, gemContext3) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::DEL_ITEM_INSTANCE_GEMS => "DELETE FROM item_instance_gems WHERE itemGuid = ?",
            Self::DEL_ITEM_INSTANCE_GEMS_BY_OWNER => {
                "DELETE iig FROM item_instance_gems iig LEFT JOIN item_instance ii ON iig.itemGuid = ii.guid WHERE ii.owner_guid = ?"
            }
            Self::INS_ITEM_INSTANCE_TRANSMOG => {
                "INSERT INTO item_instance_transmog (itemGuid, itemModifiedAppearanceAllSpecs, itemModifiedAppearanceSpec1, itemModifiedAppearanceSpec2, itemModifiedAppearanceSpec3, itemModifiedAppearanceSpec4, itemModifiedAppearanceSpec5, spellItemEnchantmentAllSpecs, spellItemEnchantmentSpec1, spellItemEnchantmentSpec2, spellItemEnchantmentSpec3, spellItemEnchantmentSpec4, spellItemEnchantmentSpec5, secondaryItemModifiedAppearanceAllSpecs, secondaryItemModifiedAppearanceSpec1, secondaryItemModifiedAppearanceSpec2, secondaryItemModifiedAppearanceSpec3, secondaryItemModifiedAppearanceSpec4, secondaryItemModifiedAppearanceSpec5) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::DEL_ITEM_INSTANCE_TRANSMOG => {
                "DELETE FROM item_instance_transmog WHERE itemGuid = ?"
            }
            Self::DEL_ITEM_INSTANCE_TRANSMOG_BY_OWNER => {
                "DELETE iit FROM item_instance_transmog iit LEFT JOIN item_instance ii ON iit.itemGuid = ii.guid WHERE ii.owner_guid = ?"
            }
            Self::UPD_GIFT_OWNER => "UPDATE character_gifts SET guid = ? WHERE item_guid = ?",
            Self::SEL_ACCOUNT_BY_NAME => "SELECT account FROM characters WHERE name = ?",
            Self::UPD_ACCOUNT_BY_GUID => "UPDATE characters SET account = ? WHERE guid = ?",
            Self::SEL_MATCH_MAKER_RATING => {
                "SELECT matchMakerRating FROM character_arena_stats WHERE guid = ? AND slot = ?"
            }
            Self::SEL_CHARACTER_COUNT => {
                "SELECT account, COUNT(guid) FROM characters WHERE account = ? GROUP BY account"
            }
            Self::UPD_NAME_BY_GUID => "UPDATE characters SET name = ? WHERE guid = ?",
            Self::SEL_CHAR_BAG_CONTENTS => {
                "SELECT bag_ci.slot, ci.slot, ii.itemEntry, ci.item, ii.count, ii.durability, ii.context, \
                 ii.flags, ii.playedTime, ir.paidMoney, ir.paidExtendedCost \
                 FROM character_inventory ci \
                 JOIN character_inventory bag_ci \
                   ON bag_ci.guid = ci.guid AND bag_ci.item = ci.bag \
                 JOIN item_instance ii ON ci.item = ii.guid \
                 LEFT JOIN item_refund_instance ir \
                   ON ir.item_guid = ci.item AND ir.player_guid = ci.guid \
                 WHERE ci.guid = ? AND bag_ci.bag = 0 AND ((bag_ci.slot >= 30 AND bag_ci.slot < 34) OR \
                 (bag_ci.slot >= 87 AND bag_ci.slot < 94) OR \
                 (bag_ci.slot >= 34 AND bag_ci.slot < 35))"
            }
            Self::DEL_ITEM_REFUND_INSTANCE => {
                "DELETE FROM item_refund_instance WHERE item_guid = ?"
            }
            Self::DEL_ITEMCONTAINER_MONEY => "DELETE FROM item_loot_money WHERE container_id = ?",
            Self::DEL_ITEMCONTAINER_ITEMS => "DELETE FROM item_loot_items WHERE container_id = ?",
            Self::DEL_ITEMCONTAINER_ITEM => {
                "DELETE FROM item_loot_items WHERE container_id = ? AND item_id = ? AND item_count = ? AND item_index = ?"
            }
            Self::SEL_ITEMCONTAINER_MONEY => {
                "SELECT money FROM item_loot_money WHERE container_id = ? LIMIT 1"
            }
            Self::INS_ITEMCONTAINER_MONEY => {
                "INSERT INTO item_loot_money (container_id, money) VALUES (?, ?)"
            }
            Self::SEL_ITEMCONTAINER_ITEMS => {
                "SELECT item_id, item_count, item_index, follow_rules, ffa, blocked, counted, under_threshold, needs_quest, random_properties_id, random_properties_seed, context \
                 FROM item_loot_items WHERE container_id = ? ORDER BY item_index"
            }
            Self::INS_ITEMCONTAINER_ITEMS => {
                "INSERT INTO item_loot_items \
                 (container_id, item_id, item_count, item_index, follow_rules, ffa, blocked, counted, under_threshold, needs_quest, random_properties_id, random_properties_seed, context) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::INS_ITEM_REFUND_INSTANCE => {
                "INSERT INTO item_refund_instance \
                 (item_guid, player_guid, paidMoney, paidExtendedCost) \
                 VALUES (?, ?, ?, ?)"
            }
            Self::INS_CHARACTER_SPELL => {
                "INSERT IGNORE INTO character_spell (guid, spell, active, disabled) VALUES (?, ?, 1, 0)"
            }
            Self::GENERATED_CPP { sql } => sql,
            Self::SEL_CHAR_QUEST_STATUS => {
                "SELECT quest, status, explored, acceptTime, endTime FROM character_queststatus WHERE guid = ? AND status <> 0"
            }
            Self::SEL_CHAR_QUEST_STATUS_OBJECTIVES => {
                "SELECT quest, objective, data FROM character_queststatus_objectives WHERE guid = ?"
            }
            Self::SEL_CHAR_QUEST_STATUS_SEASONAL => {
                "SELECT quest, event, completedTime FROM character_queststatus_seasonal WHERE guid = ?"
            }
            Self::INS_CHAR_QUEST_STATUS => {
                "REPLACE INTO character_queststatus (guid, quest, status, explored, acceptTime, endTime) VALUES (?, ?, ?, ?, ?, ?)"
            }
            Self::DEL_CHAR_QUEST_STATUS => {
                "DELETE FROM character_queststatus WHERE guid = ? AND quest = ?"
            }
            Self::DEL_CHAR_QUEST_STATUS_OBJECTIVES_BY_QUEST => {
                "DELETE FROM character_queststatus_objectives WHERE guid = ? AND quest = ?"
            }
            Self::REP_CHAR_QUEST_STATUS_OBJECTIVES => {
                "REPLACE INTO character_queststatus_objectives (guid, quest, objective, data) VALUES (?, ?, ?, ?)"
            }
        }
    }
}

#[cfg(test)]
mod tests {
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
        // INS_CHARACTER has 22 placeholders
        assert_eq!(CharStatements::INS_CHARACTER.sql().matches('?').count(), 22);
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
        assert_eq!(
            CharStatements::INS_ITEM_INSTANCE_WITH_RANDOM_CONTEXT
                .sql()
                .matches('?')
                .count(),
            8
        );
        assert_eq!(
            CharStatements::INS_ITEM_INSTANCE_CLONE
                .sql()
                .matches('?')
                .count(),
            14
        );
        assert_eq!(
            CharStatements::UPD_ITEM_INSTANCE_FLAGS
                .sql()
                .matches('?')
                .count(),
            2
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
}
