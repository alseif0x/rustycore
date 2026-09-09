//! The `StatementDef` impl for the character statements; a trait impl cannot be split.
//!
//! Separated from the character.rs root under #652. Behaviour is preserved.

use super::*;

impl StatementDef for CharStatements {
    fn database() -> crate::persistence_trace::LogicalDatabase {
        crate::persistence_trace::LogicalDatabase::Character
    }

    fn trace_identity(self) -> String {
        // The default derives identity from `Debug`, which for this data
        // carrying variant would embed the SQL text.
        match self {
            Self::GENERATED_CPP { name, .. } => name.to_owned(),
            other => format!("{other:?}"),
        }
    }

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
            Self::SEL_RESERVED_NAMES => "SELECT name FROM reserved_name",
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
                "INSERT INTO characters (guid, account, name, race, class, gender, level, xp, money, inventorySlots, bankSlots, restState, playerFlags, playerFlagsEx, map, instance_id, dungeonDifficulty, raidDifficulty, legacyRaidDifficulty, position_x, position_y, position_z, orientation, trans_x, trans_y, trans_z, trans_o, transguid, taximask, createTime, createMode, cinematic, totaltime, leveltime, rest_bonus, logout_time, is_logout_resting, resettalents_cost, resettalents_time, activeTalentGroup, bonusTalentGroups,extra_flags, summonedPetNumber, at_login, death_expire_time, taxi_path, totalKills, todayKills, yesterdayKills, chosenTitle, watchedFaction, drunk, health, power1, power2, power3, power4, power5, power6, power7, power8, power9, power10, latency, lootSpecId, exploredZones, equipmentCache, knownTitles, actionBars, lastLoginBuild) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"
            }
            Self::UPD_CHARACTER => {
                "UPDATE characters SET name=?,race=?,class=?,gender=?,level=?,xp=?,money=?,inventorySlots=?,bankSlots=?,restState=?,playerFlags=?,playerFlagsEx=?,map=?,instance_id=?,dungeonDifficulty=?,raidDifficulty=?,legacyRaidDifficulty=?,position_x=?,position_y=?,position_z=?,orientation=?,trans_x=?,trans_y=?,trans_z=?,trans_o=?,transguid=?,taximask=?,cinematic=?,totaltime=?,leveltime=?,rest_bonus=?,logout_time=?,is_logout_resting=?,resettalents_cost=?,resettalents_time=?,numRespecs=?,activeTalentGroup=?,bonusTalentGroups=?,extra_flags=?,summonedPetNumber=?,at_login=?,zone=?,death_expire_time=?,taxi_path=?,totalKills=?,todayKills=?,yesterdayKills=?,chosenTitle=?,watchedFaction=?,drunk=?,health=?,power1=?,power2=?,power3=?,power4=?,power5=?,power6=?,power7=?,power8=?,power9=?,power10=?,latency=?,lootSpecId=?,exploredZones=?,equipmentCache=?,knownTitles=?,actionBars=?,online=?,honor=?,honorLevel=?,honorRestState=?,honorRestBonus=?,lastLoginBuild=? WHERE guid=?"
            }
            Self::INS_CHAR_CUSTOMIZATION => {
                "INSERT INTO character_customizations (guid, chrCustomizationOptionID, chrCustomizationChoiceID) VALUES (?, ?, ?)"
            }
            Self::INS_CHARACTER_CUSTOMIZATION => {
                "INSERT INTO character_customizations (guid, chrCustomizationOptionID, chrCustomizationChoiceID) VALUES (?, ?, ?)"
            }
            Self::UPD_ADD_AT_LOGIN_FLAG => {
                "UPDATE characters SET at_login = at_login | ? WHERE guid = ?"
            }
            Self::UPD_REM_AT_LOGIN_FLAG => {
                "UPDATE characters set at_login = at_login & ~ ? WHERE guid = ?"
            }
            Self::UPD_ALL_AT_LOGIN_FLAGS => "UPDATE characters SET at_login = at_login | ?",
            Self::INS_BUG_REPORT => "INSERT INTO bugreport (type, content) VALUES(?, ?)",
            Self::UPD_PETITION_NAME => "UPDATE petition SET name = ? WHERE petitionguid = ?",
            Self::INS_PETITION_SIGNATURE => {
                "INSERT INTO petition_sign (ownerguid, petitionguid, playerguid, player_account) VALUES (?, ?, ?, ?)"
            }
            Self::UPD_ACCOUNT_ONLINE => "UPDATE characters SET online = 0 WHERE account = ?",
            Self::DEL_CHARACTER_CUSTOMIZATIONS => {
                "DELETE FROM character_customizations WHERE guid = ?"
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
                 ii.flags, ii.playedTime, ii.enchantments, ii.randomPropertiesId, \
                 ii.randomPropertiesSeed, ig.gemItemId1, ig.gemBonuses1, ig.gemContext1, \
                 ig.gemItemId2, ig.gemBonuses2, ig.gemContext2, \
                 ig.gemItemId3, ig.gemBonuses3, ig.gemContext3, \
                 ir.paidMoney, ir.paidExtendedCost, ii.duration, ii.charges \
                 FROM character_inventory ci \
                 JOIN item_instance ii ON ci.item = ii.guid \
                 LEFT JOIN item_instance_gems ig ON ii.guid = ig.itemGuid \
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
            Self::DEL_CHAR_INVENTORY_ITEM_BY_OWNER => {
                "DELETE ci FROM character_inventory ci INNER JOIN item_instance ii ON ii.guid = ci.item WHERE ci.guid = ? AND ci.item = ? AND ii.owner_guid = ?"
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
            Self::DEL_PLAYER_CURRENCY => "DELETE FROM character_currency WHERE CharacterGuid = ?",
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
            Self::UPD_GROUP_DIFFICULTY => "UPDATE `groups` SET difficulty = ? WHERE guid = ?",
            Self::UPD_GROUP_RAID_DIFFICULTY => {
                "UPDATE `groups` SET raidDifficulty = ? WHERE guid = ?"
            }
            Self::UPD_GROUP_LEGACY_RAID_DIFFICULTY => {
                "UPDATE `groups` SET legacyRaidDifficulty = ? WHERE guid = ?"
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
            Self::SEL_GROUP_MEMBER_CHARACTER_CACHE => {
                "SELECT guid, name, race, class FROM characters WHERE guid IN (SELECT leaderGuid FROM `groups` UNION SELECT memberGuid FROM group_member)"
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
            Self::SEL_GM_BUGS => {
                "SELECT id, playerGuid, note, createTime, mapId, posX, posY, posZ, facing, closedBy, assignedTo, comment FROM gm_bug"
            }
            Self::REP_GM_BUG => {
                "REPLACE INTO gm_bug (id, playerGuid, note, createTime, mapId, posX, posY, posZ, facing, closedBy, assignedTo, comment) VALUES (?, ?, ?, UNIX_TIMESTAMP(NOW()), ?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::DEL_GM_BUG => "DELETE FROM gm_bug WHERE id = ?",
            Self::DEL_ALL_GM_BUGS => "DELETE FROM gm_bug",
            Self::SEL_GM_COMPLAINTS => {
                "SELECT id, playerGuid, note, createTime, mapId, posX, posY, posZ, facing, targetCharacterGuid, reportType, reportMajorCategory, reportMinorCategoryFlags, reportLineIndex, assignedTo, closedBy, comment FROM gm_complaint"
            }
            Self::REP_GM_COMPLAINT => {
                "REPLACE INTO gm_complaint (id, playerGuid, note, createTime, mapId, posX, posY, posZ, facing, targetCharacterGuid, reportType, reportMajorCategory, reportMinorCategoryFlags, reportLineIndex, assignedTo, closedBy, comment) VALUES (?, ?, ?, UNIX_TIMESTAMP(NOW()), ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::DEL_GM_COMPLAINT => "DELETE FROM gm_complaint WHERE id = ?",
            Self::SEL_GM_COMPLAINT_CHATLINES => {
                "SELECT timestamp, text FROM gm_complaint_chatlog WHERE complaintId = ? ORDER BY lineId ASC"
            }
            Self::INS_GM_COMPLAINT_CHATLINE => {
                "INSERT INTO gm_complaint_chatlog (complaintId, lineId, timestamp, text) VALUES (?, ?, ?, ?)"
            }
            Self::DEL_GM_COMPLAINT_CHATLOG => {
                "DELETE FROM gm_complaint_chatlog WHERE complaintId = ?"
            }
            Self::DEL_ALL_GM_COMPLAINTS => "DELETE FROM gm_complaint",
            Self::DEL_ALL_GM_COMPLAINT_CHATLOGS => "DELETE FROM gm_complaint_chatlog",
            Self::SEL_GM_SUGGESTIONS => {
                "SELECT id, playerGuid, note, createTime, mapId, posX, posY, posZ, facing, closedBy, assignedTo, comment FROM gm_suggestion"
            }
            Self::REP_GM_SUGGESTION => {
                "REPLACE INTO gm_suggestion (id, playerGuid, note, createTime, mapId, posX, posY, posZ, facing, closedBy, assignedTo, comment) VALUES (?, ?, ?, UNIX_TIMESTAMP(NOW()), ?, ?, ?, ?, ?, ? ,? ,?)"
            }
            Self::DEL_GM_SUGGESTION => "DELETE FROM gm_suggestion WHERE id = ?",
            Self::DEL_ALL_GM_SUGGESTIONS => "DELETE FROM gm_suggestion",
            Self::INS_LFG_DATA => "INSERT INTO lfg_data (guid, dungeon, state) VALUES (?, ?, ?)",
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
            Self::REP_WORLD_VARIABLE => "REPLACE INTO world_variable (Id, Value) VALUES (?, ?)",
            Self::DEL_INVALID_SPELL_SPELLS => "DELETE FROM character_spell WHERE spell = ?",
            Self::UPD_DELETE_INFO => {
                "UPDATE characters SET deleteInfos_Name = name, deleteInfos_Account = account, deleteDate = UNIX_TIMESTAMP(), name = '', account = 0 WHERE guid = ?"
            }
            Self::UPD_RESTORE_DELETE_INFO => {
                "UPDATE characters SET name = ?, account = ?, deleteDate = NULL, deleteInfos_Name = NULL, deleteInfos_Account = NULL WHERE deleteDate IS NOT NULL AND guid = ?"
            }
            Self::UPD_ZONE => "UPDATE characters SET zone = ? WHERE guid = ?",
            Self::UPD_LEVEL => "UPDATE characters SET level = ?, xp = 0 WHERE guid = ?",
            Self::DEL_INVALID_ACHIEV_PROGRESS_CRITERIA => {
                "DELETE FROM character_achievement_progress WHERE criteria = ?"
            }
            Self::DEL_INVALID_ACHIEV_PROGRESS_CRITERIA_GUILD => {
                "DELETE FROM guild_achievement_progress WHERE criteria = ?"
            }
            Self::DEL_INVALID_ACHIEVMENT => {
                "DELETE FROM character_achievement WHERE achievement = ?"
            }
            Self::DEL_INVALID_PET_SPELL => "DELETE FROM pet_spell WHERE spell = ?",
            Self::UPD_CHAR_NAME_AT_LOGIN => {
                "UPDATE characters SET name = ?, at_login = ? WHERE guid = ?"
            }
            Self::DEL_CHARACTER_SKILL => {
                "DELETE FROM character_skills WHERE guid = ? AND skill = ?"
            }
            Self::UPD_CHARACTER_SOCIAL_FLAGS => {
                "UPDATE character_social SET flags = ? WHERE guid = ? AND friend = ?"
            }
            Self::INS_CHARACTER_SOCIAL => {
                "INSERT INTO character_social (guid, friend, flags) VALUES (?, ?, ?)"
            }
            Self::DEL_CHARACTER_SOCIAL => {
                "DELETE FROM character_social WHERE guid = ? AND friend = ?"
            }
            Self::UPD_CHARACTER_SOCIAL_NOTE => {
                "UPDATE character_social SET note = ? WHERE guid = ? AND friend = ?"
            }
            Self::UPD_CHARACTER_POSITION => {
                "UPDATE characters SET position_x = ?, position_y = ?, position_z = ?, orientation = ?, map = ?, instance_id = ?, zone = ?, trans_x = 0, trans_y = 0, trans_z = 0, transguid = 0, taxi_path = '', cinematic = 1 WHERE guid = ?"
            }
            Self::UPD_CHARACTER_POSITION_BY_MAPID => {
                "UPDATE characters SET position_x = ?, position_y = ?, position_z = ?, orientation = ?, map = ?, zone = ?, trans_x = 0, trans_y = 0, trans_z = 0, transguid = 0, taxi_path = '', cinematic = 1 WHERE guid = ? AND map = ?"
            }
            Self::UPD_CHARACTER_POSITION_PRESERVE_TRAVEL => {
                "UPDATE characters SET position_x = ?, position_y = ?, position_z = ?, orientation = ?, map = ?, instance_id = ?, zone = ? WHERE guid = ?"
            }
            Self::SEL_CHARACTER_AURA_FROZEN => {
                "SELECT characters.name, character_aura.remainTime FROM characters LEFT JOIN character_aura ON (characters.guid = character_aura.guid) WHERE character_aura.spell = 9454"
            }
            Self::SEL_CHARACTER_ONLINE => {
                "SELECT name, account, map, zone FROM characters WHERE online > 0"
            }
            Self::SEL_CHAR_DEL_INFO_BY_GUID => {
                "SELECT guid, deleteInfos_Name, deleteInfos_Account, deleteDate FROM characters WHERE deleteDate IS NOT NULL AND guid = ?"
            }
            Self::SEL_CHAR_DEL_INFO_BY_NAME => {
                "SELECT guid, deleteInfos_Name, deleteInfos_Account, deleteDate FROM characters WHERE deleteDate IS NOT NULL AND deleteInfos_Name LIKE CONCAT('%%', ?, '%%')"
            }
            Self::SEL_CHAR_DEL_INFO => {
                "SELECT guid, deleteInfos_Name, deleteInfos_Account, deleteDate FROM characters WHERE deleteDate IS NOT NULL"
            }
            Self::SEL_CHARS_BY_ACCOUNT_ID => "SELECT guid FROM characters WHERE account = ?",
            Self::SEL_CHAR_PINFO => {
                "SELECT totaltime, level, money, account, race, class, map, zone, gender, health, playerFlags FROM characters WHERE guid = ?"
            }
            Self::SEL_PINFO_BANS => {
                "SELECT unbandate, bandate = unbandate, bannedby, banreason FROM character_banned WHERE guid = ? AND active ORDER BY bandate ASC LIMIT 1"
            }
            Self::SEL_PINFO_MAILS => {
                "SELECT SUM(CASE WHEN (checked & 1) THEN 1 ELSE 0 END) AS 'readmail', COUNT(*) AS 'totalmail' FROM mail WHERE `receiver` = ?"
            }
            Self::SEL_PINFO_XP => {
                "SELECT a.xp, b.guid FROM characters a LEFT JOIN guild_member b ON a.guid = b.guid WHERE a.guid = ?"
            }
            Self::SEL_CHAR_HOMEBIND => {
                "SELECT mapId, zoneId, posX, posY, posZ, orientation FROM character_homebind WHERE guid = ?"
            }
            Self::SEL_CHAR_GUID_NAME_BY_ACC => {
                "SELECT guid, name, online FROM characters WHERE account = ?"
            }
            Self::SEL_CHAR_CUSTOMIZE_INFO => {
                "SELECT name, race, class, gender, at_login FROM characters WHERE guid = ?"
            }
            Self::SEL_CHAR_RACE_OR_FACTION_CHANGE_INFOS => {
                "SELECT c.at_login, c.knownTitles, gm.guid FROM characters c LEFT JOIN group_member gm ON c.guid = gm.memberGuid WHERE c.guid = ?"
            }
            Self::SEL_CHAR_COD_ITEM_MAIL => {
                "SELECT id, messageType, mailTemplateId, sender, subject, body, money, has_items FROM mail WHERE receiver = ? AND has_items <> 0 AND cod <> 0"
            }
            Self::SEL_CHAR_SOCIAL => "SELECT DISTINCT guid FROM character_social WHERE friend = ?",
            Self::SEL_CHAR_OLD_CHARS => {
                "SELECT guid, deleteInfos_Account FROM characters WHERE deleteDate IS NOT NULL AND deleteDate < ?"
            }
            Self::SEL_MAIL => {
                "SELECT id, messageType, sender, receiver, subject, body, expire_time, deliver_time, money, cod, checked, stationery, mailTemplateId FROM mail WHERE receiver = ? ORDER BY id DESC"
            }
            Self::DEL_CHAR_AURA_FROZEN => {
                "DELETE FROM character_aura WHERE spell = 9454 AND guid = ?"
            }
            Self::SEL_CHAR_INVENTORY_COUNT_ITEM => {
                "SELECT COUNT(itemEntry) FROM character_inventory ci INNER JOIN item_instance ii ON ii.guid = ci.item WHERE itemEntry = ?"
            }
            Self::SEL_MAIL_COUNT_ITEM => {
                "SELECT COUNT(itemEntry) FROM mail_items mi INNER JOIN item_instance ii ON ii.guid = mi.item_guid WHERE itemEntry = ?"
            }
            Self::SEL_AUCTIONHOUSE_COUNT_ITEM => {
                "SELECT COUNT(*) FROM auction_items ai INNER JOIN item_instance ii ON ii.guid = ai.itemGuid WHERE ii.itemEntry = ?"
            }
            Self::SEL_GUILD_BANK_COUNT_ITEM => {
                "SELECT COUNT(itemEntry) FROM guild_bank_item gbi INNER JOIN item_instance ii ON ii.guid = gbi.item_guid WHERE itemEntry = ?"
            }
            Self::SEL_CHAR_INVENTORY_ITEM_BY_ENTRY => {
                "SELECT ci.item, cb.slot AS bag, ci.slot, ci.guid, c.account, c.name FROM characters c INNER JOIN character_inventory ci ON ci.guid = c.guid INNER JOIN item_instance ii ON ii.guid = ci.item LEFT JOIN character_inventory cb ON cb.item = ci.bag WHERE ii.itemEntry = ? LIMIT ?"
            }
            Self::SEL_MAIL_ITEMS_BY_ENTRY => {
                "SELECT mi.item_guid, m.sender, m.receiver, cs.account, cs.name, cr.account, cr.name FROM mail m INNER JOIN mail_items mi ON mi.mail_id = m.id INNER JOIN item_instance ii ON ii.guid = mi.item_guid INNER JOIN characters cs ON cs.guid = m.sender INNER JOIN characters cr ON cr.guid = m.receiver WHERE ii.itemEntry = ? LIMIT ?"
            }
            Self::SEL_AUCTIONHOUSE_ITEM_BY_ENTRY => {
                "SELECT ai.itemGuid, c.guid, c.account, c.name FROM auctionhouse ah INNER JOIN auction_items ai ON ah.id = ai.auctionId INNER JOIN characters c ON c.guid = ah.owner INNER JOIN item_instance ii ON ii.guid = ai.itemGuid WHERE ii.itemEntry = ? LIMIT ?"
            }
            Self::SEL_GUILD_BANK_ITEM_BY_ENTRY => {
                "SELECT gi.item_guid, gi.guildid, g.name FROM guild_bank_item gi INNER JOIN guild g ON g.guildid = gi.guildid INNER JOIN item_instance ii ON ii.guid = gi.item_guid WHERE ii.itemEntry = ? LIMIT ?"
            }
            Self::DEL_CHAR_ACHIEVEMENT => "DELETE FROM character_achievement WHERE guid = ?",
            Self::DEL_CHAR_ACHIEVEMENT_PROGRESS => {
                "DELETE FROM character_achievement_progress WHERE guid = ?"
            }
            Self::INS_CHAR_ACHIEVEMENT => {
                "INSERT INTO character_achievement (guid, achievement, date) VALUES (?, ?, ?)"
            }
            Self::DEL_CHAR_ACHIEVEMENT_PROGRESS_BY_CRITERIA => {
                "DELETE FROM character_achievement_progress WHERE guid = ? AND criteria = ?"
            }
            Self::INS_CHAR_ACHIEVEMENT_PROGRESS => {
                "INSERT INTO character_achievement_progress (guid, criteria, counter, date) VALUES (?, ?, ?, ?)"
            }
            Self::INS_CHAR_GIFT => {
                "INSERT INTO character_gifts (guid, item_guid, entry, flags) VALUES (?, ?, ?, ?)"
            }
            Self::DEL_MAIL_ITEM_BY_ID => "DELETE FROM mail_items WHERE mail_id = ?",
            Self::INS_PETITION => {
                "INSERT INTO petition (ownerguid, petitionguid, name) VALUES (?, ?, ?)"
            }
            Self::DEL_PETITION_BY_GUID => "DELETE FROM petition WHERE petitionguid = ?",
            Self::DEL_PETITION_SIGNATURE_BY_GUID => {
                "DELETE FROM petition_sign WHERE petitionguid = ?"
            }
            Self::DEL_CHAR_DECLINED_NAME => "DELETE FROM character_declinedname WHERE guid = ?",
            Self::INS_CHAR_DECLINED_NAME => {
                "INSERT INTO character_declinedname (guid, genitive, dative, accusative, instrumental, prepositional) VALUES (?, ?, ?, ?, ?, ?)"
            }
            Self::UPD_CHAR_RACE => {
                "UPDATE characters SET race = ?, extra_flags = extra_flags | ? WHERE guid = ?"
            }
            Self::DEL_CHAR_SKILL_LANGUAGES => {
                "DELETE FROM character_skills WHERE skill IN (98, 113, 759, 111, 313, 109, 115, 315, 673, 137) AND guid = ?"
            }
            Self::INS_CHAR_SKILL_LANGUAGE => {
                "INSERT INTO `character_skills` (guid, skill, value, max) VALUES (?, ?, 300, 300)"
            }
            Self::UPD_CHAR_TAXI_PATH => "UPDATE characters SET taxi_path = '' WHERE guid = ?",
            Self::UPD_CHAR_TAXIMASK => "UPDATE characters SET taximask = ? WHERE guid = ?",
            Self::DEL_CHAR_QUESTSTATUS => "DELETE FROM character_queststatus WHERE guid = ?",
            Self::DEL_CHAR_QUESTSTATUS_OBJECTIVES => {
                "DELETE FROM character_queststatus_objectives WHERE guid = ?"
            }
            Self::DEL_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA => {
                "DELETE FROM character_queststatus_objectives_criteria WHERE guid = ?"
            }
            Self::DEL_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA_PROGRESS => {
                "DELETE FROM character_queststatus_objectives_criteria_progress WHERE guid = ?"
            }
            Self::DEL_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA_PROGRESS_BY_CRITERIA => {
                "DELETE FROM character_queststatus_objectives_criteria_progress WHERE guid = ? AND criteriaId = ?"
            }
            Self::DEL_CHAR_SOCIAL_BY_GUID => "DELETE FROM character_social WHERE guid = ?",
            Self::DEL_CHAR_SOCIAL_BY_FRIEND => "DELETE FROM character_social WHERE friend = ?",
            Self::DEL_CHAR_ACHIEVEMENT_BY_ACHIEVEMENT => {
                "DELETE FROM character_achievement WHERE achievement = ? AND guid = ?"
            }
            Self::UPD_CHAR_ACHIEVEMENT => {
                "UPDATE character_achievement SET achievement = ? where achievement = ? AND guid = ?"
            }
            Self::UPD_CHAR_INVENTORY_FACTION_CHANGE => {
                "UPDATE item_instance ii, character_inventory ci SET ii.itemEntry = ? WHERE ii.itemEntry = ? AND ci.guid = ? AND ci.item = ii.guid"
            }
            Self::DEL_CHAR_SPELL_BY_SPELL => {
                "DELETE FROM character_spell WHERE spell = ? AND guid = ?"
            }
            Self::UPD_CHAR_SPELL_FACTION_CHANGE => {
                "UPDATE character_spell SET spell = ? where spell = ? AND guid = ?"
            }
            Self::SEL_CHAR_REP_BY_FACTION => {
                "SELECT standing FROM character_reputation WHERE faction = ? AND guid = ?"
            }
            Self::DEL_CHAR_REP_BY_FACTION => {
                "DELETE FROM character_reputation WHERE faction = ? AND guid = ?"
            }
            Self::UPD_CHAR_REP_FACTION_CHANGE => {
                "UPDATE character_reputation SET faction = ?, standing = ? WHERE faction = ? AND guid = ?"
            }
            Self::UPD_CHAR_TITLES_FACTION_CHANGE => {
                "UPDATE characters SET knownTitles = ? WHERE guid = ?"
            }
            Self::RES_CHAR_TITLES_FACTION_CHANGE => {
                "UPDATE characters SET chosenTitle = 0 WHERE guid = ?"
            }
            Self::DEL_CHAR_SPELL_COOLDOWNS => "DELETE FROM character_spell_cooldown WHERE guid = ?",
            Self::INS_CHAR_SPELL_COOLDOWN => {
                "INSERT INTO character_spell_cooldown (guid, spell, item, time, categoryId, categoryEnd) VALUES (?, ?, ?, ?, ?, ?)"
            }
            Self::DEL_CHAR_SPELL_CHARGES => "DELETE FROM character_spell_charges WHERE guid = ?",
            Self::INS_CHAR_SPELL_CHARGES => {
                "INSERT INTO character_spell_charges (guid, categoryId, rechargeStart, rechargeEnd) VALUES (?, ?, ?, ?)"
            }
            Self::DEL_CHAR_ACTION => "DELETE FROM character_action WHERE guid = ?",
            Self::DEL_CHAR_AURA => "DELETE FROM character_aura WHERE guid = ?",
            Self::DEL_CHAR_AURA_EFFECT => "DELETE FROM character_aura_effect WHERE guid = ?",
            Self::DEL_CHAR_GIFT => "DELETE FROM character_gifts WHERE guid = ?",
            Self::DEL_CHAR_INVENTORY => "DELETE FROM character_inventory WHERE guid = ?",
            Self::DEL_CHAR_QUESTSTATUS_REWARDED => {
                "DELETE FROM character_queststatus_rewarded WHERE guid = ?"
            }
            Self::DEL_CHAR_SPELL => "DELETE FROM character_spell WHERE guid = ?",
            Self::DEL_MAIL => "DELETE FROM mail WHERE receiver = ?",
            Self::DEL_MAIL_ITEMS => "DELETE FROM mail_items WHERE receiver = ?",
            Self::DEL_CHAR_ACHIEVEMENTS => {
                "DELETE FROM character_achievement WHERE guid = ? AND achievement NOT IN (456,457,458,459,460,461,462,463,464,465,466,467,1400,1402,1404,1405,1406,1407,1408,1409,1410,1411,1412,1413,1414,1415,1416,1417,1418,1419,1420,1421,1422,1423,1424,1425,1426,1427,1463,3117,3259,4078,4576,4998,4999,5000,5001,5002,5003,5004,5005,5006,5007,5008,5381,5382,5383,5384,5385,5386,5387,5388,5389,5390,5391,5392,5393,5394,5395,5396,6433,6523,6524,6743,6744,6745,6746,6747,6748,6749,6750,6751,6752,6829,6859,6860,6861,6862,6863,6864,6865,6866,6867,6868,6869,6870,6871,6872,6873)"
            }
            Self::DEL_CHAR_EQUIPMENTSETS => "DELETE FROM character_equipmentsets WHERE guid = ?",
            Self::DEL_CHAR_TRANSMOG_OUTFITS => {
                "DELETE FROM character_transmog_outfits WHERE guid = ?"
            }
            Self::DEL_GUILD_EVENTLOG_BY_PLAYER => {
                "DELETE FROM guild_eventlog WHERE PlayerGuid1 = ? OR PlayerGuid2 = ?"
            }
            Self::DEL_GUILD_BANK_EVENTLOG_BY_PLAYER => {
                "DELETE FROM guild_bank_eventlog WHERE PlayerGuid = ?"
            }
            Self::DEL_CHAR_GLYPHS => "DELETE FROM character_glyphs WHERE guid = ?",
            Self::DEL_CHAR_TALENT => "DELETE FROM character_talent WHERE guid = ?",
            Self::DEL_CHAR_SKILLS => "DELETE FROM character_skills WHERE guid = ?",
            Self::INS_CHAR_ACTION => {
                "INSERT INTO character_action (guid, spec, traitConfigId, button, action, type) VALUES (?, ?, ?, ?, ?, ?)"
            }
            Self::UPD_CHAR_ACTION => {
                "UPDATE character_action SET action = ?, type = ? WHERE guid = ? AND button = ? AND spec = ? AND traitConfigId = ?"
            }
            Self::DEL_CHAR_ACTION_BY_BUTTON_SPEC => {
                "DELETE FROM character_action WHERE guid = ? and button = ? and spec = ? AND traitConfigId = ?"
            }
            Self::DEL_CHAR_ACTION_BY_SPEC => {
                "DELETE FROM character_action WHERE guid = ? AND spec = ? AND traitConfigId = ?"
            }
            Self::DEL_CHAR_ACTION_BY_TRAIT_CONFIG => {
                "DELETE FROM character_action WHERE guid = ? AND traitConfigId = ?"
            }
            Self::DEL_CHAR_INVENTORY_BY_ITEM => "DELETE FROM character_inventory WHERE item = ?",
            Self::DEL_CHAR_INVENTORY_BY_BAG_SLOT => {
                "DELETE FROM character_inventory WHERE bag = ? AND slot = ? AND guid = ?"
            }
            Self::UPD_MAIL => {
                "UPDATE mail SET has_items = ?, expire_time = ?, deliver_time = ?, money = ?, cod = ?, checked = ? WHERE id = ?"
            }
            Self::REP_CHAR_QUESTSTATUS => {
                "REPLACE INTO character_queststatus (guid, quest, status, explored, acceptTime, endTime) VALUES (?, ?, ?, ?, ?, ?)"
            }
            Self::DEL_CHAR_QUESTSTATUS_BY_QUEST => {
                "DELETE FROM character_queststatus WHERE guid = ? AND quest = ?"
            }
            Self::INS_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA => {
                "INSERT INTO character_queststatus_objectives_criteria (guid, questObjectiveId) VALUES (?, ?)"
            }
            Self::INS_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA_PROGRESS => {
                "INSERT INTO character_queststatus_objectives_criteria_progress (guid, criteriaId, counter, date) VALUES (?, ?, ?, ?)"
            }
            Self::INS_CHAR_QUESTSTATUS_REWARDED => {
                "INSERT IGNORE INTO character_queststatus_rewarded (guid, quest, active) VALUES (?, ?, 1)"
            }
            Self::DEL_CHAR_QUESTSTATUS_REWARDED_BY_QUEST => {
                "DELETE FROM character_queststatus_rewarded WHERE guid = ? AND quest = ?"
            }
            Self::UPD_CHAR_QUESTSTATUS_REWARDED_FACTION_CHANGE => {
                "UPDATE character_queststatus_rewarded SET quest = ? WHERE quest = ? AND guid = ?"
            }
            Self::UPD_CHAR_QUESTSTATUS_REWARDED_ACTIVE => {
                "UPDATE character_queststatus_rewarded SET active = 1 WHERE guid = ?"
            }
            Self::UPD_CHAR_QUESTSTATUS_REWARDED_ACTIVE_BY_QUEST => {
                "UPDATE character_queststatus_rewarded SET active = 0 WHERE quest = ? AND guid = ?"
            }
            Self::DEL_INVALID_QUEST_PROGRESS_CRITERIA => {
                "DELETE FROM character_queststatus_objectives_criteria WHERE questObjectiveId = ?"
            }
            Self::DEL_CHAR_SKILL_BY_SKILL => {
                "DELETE FROM character_skills WHERE guid = ? AND skill = ?"
            }
            Self::INS_CHAR_SKILLS => {
                "INSERT INTO character_skills (guid, skill, value, max, professionSlot) VALUES (?, ?, ?, ?, ?)"
            }
            Self::UPD_CHAR_SKILLS => {
                "UPDATE character_skills SET value = ?, max = ?, professionSlot = ? WHERE guid = ? AND skill = ?"
            }
            Self::INS_CHAR_SPELL => {
                "INSERT INTO character_spell (guid, spell, active, disabled) VALUES (?, ?, ?, ?)"
            }
            Self::UPSERT_CHAR_SPELL_LEARN_FALLBACK => {
                "INSERT INTO character_spell (guid, spell, active, disabled) VALUES (?, ?, ?, ?) ON DUPLICATE KEY UPDATE active = IF(character_spell.disabled, character_spell.active, VALUES(active)), disabled = VALUES(disabled)"
            }
            Self::DEL_CHAR_SPELL_FAVORITE => {
                "DELETE FROM character_spell_favorite WHERE guid = ? AND spell = ?"
            }
            Self::DEL_CHAR_SPELL_FAVORITE_BY_CHAR => {
                "DELETE FROM character_spell_favorite WHERE guid = ?"
            }
            Self::INS_CHAR_SPELL_FAVORITE => {
                "INSERT INTO character_spell_favorite (guid, spell) VALUES (?, ?)"
            }
            Self::DEL_CHAR_STATS => "DELETE FROM character_stats WHERE guid = ?",
            Self::INS_CHAR_STATS => {
                "INSERT INTO character_stats (guid, maxhealth, maxpower1, maxpower2, maxpower3, maxpower4, maxpower5, maxpower6, maxpower7, maxpower8, maxpower9, maxpower10, strength, agility, stamina, intellect, armor, resHoly, resFire, resNature, resFrost, resShadow, resArcane, blockPct, dodgePct, parryPct, critPct, rangedCritPct, spellCritPct, attackPower, rangedAttackPower, spellPower, resilience, mastery, versatility) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::DEL_PETITION_BY_OWNER => "DELETE FROM petition WHERE ownerguid = ?",
            Self::DEL_PETITION_SIGNATURE_BY_OWNER => {
                "DELETE FROM petition_sign WHERE ownerguid = ?"
            }
            Self::INS_CHAR_GLYPHS => {
                "INSERT INTO character_glyphs (guid, talentGroup, glyphSlot, glyphId) VALUES(?, ?, ?, ?)"
            }
            Self::INS_CHAR_TALENT => {
                "INSERT INTO character_talent (guid, talentId, talentRank, talentGroup) VALUES (?, ?, ?, ?)"
            }
            Self::UPD_CHAR_LIST_SLOT => {
                "UPDATE characters SET slot = ? WHERE guid = ? AND account = ?"
            }
            Self::INS_CHAR_FISHINGSTEPS => {
                "INSERT INTO character_fishingsteps (guid, fishingSteps) VALUES (?, ?)"
            }
            Self::DEL_CHAR_FISHINGSTEPS => "DELETE FROM character_fishingsteps WHERE guid = ?",
            Self::SEL_CHAR_TRAIT_ENTRIES => {
                "SELECT traitConfigId, traitNodeId, traitNodeEntryId, `rank`, grantedRanks FROM character_trait_entry WHERE guid = ?"
            }
            Self::INS_CHAR_TRAIT_ENTRIES => {
                "INSERT INTO character_trait_entry (guid, traitConfigId, traitNodeId, traitNodeEntryId, `rank`, grantedRanks) VALUES (?, ?, ?, ?, ?, ?)"
            }
            Self::DEL_CHAR_TRAIT_ENTRIES => {
                "DELETE FROM character_trait_entry WHERE guid = ? AND traitConfigId = ?"
            }
            Self::DEL_CHAR_TRAIT_ENTRIES_BY_CHAR => {
                "DELETE FROM character_trait_entry WHERE guid = ?"
            }
            Self::SEL_CHAR_TRAIT_CONFIGS => {
                "SELECT traitConfigId, type, chrSpecializationId, combatConfigFlags, localIdentifier, skillLineId, traitSystemId, `name` FROM character_trait_config WHERE guid = ?"
            }
            Self::INS_CHAR_TRAIT_CONFIGS => {
                "INSERT INTO character_trait_config (guid, traitConfigId, type, chrSpecializationId, combatConfigFlags, localIdentifier, skillLineId, traitSystemId, `name`) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::DEL_CHAR_TRAIT_CONFIGS => {
                "DELETE FROM character_trait_config WHERE guid = ? AND traitConfigId = ?"
            }
            Self::DEL_CHAR_TRAIT_CONFIGS_BY_CHAR => {
                "DELETE FROM character_trait_config WHERE guid = ?"
            }
            Self::DEL_RESET_CHARACTER_QUESTSTATUS_DAILY => {
                "DELETE FROM character_queststatus_daily"
            }
            Self::DEL_RESET_CHARACTER_QUESTSTATUS_WEEKLY => {
                "DELETE FROM character_queststatus_weekly"
            }
            Self::DEL_RESET_CHARACTER_QUESTSTATUS_MONTHLY => {
                "DELETE FROM character_queststatus_monthly"
            }
            Self::SEL_CHAR_VOID_STORAGE => {
                "SELECT itemId, itemEntry, slot, creatorGuid, fixedScalingLevel, randomPropertiesId, randomPropertiesSeed, context FROM character_void_storage WHERE playerGuid = ?"
            }
            Self::REP_CHAR_VOID_STORAGE_ITEM => {
                "REPLACE INTO character_void_storage (itemId, playerGuid, itemEntry, slot, creatorGuid, fixedScalingLevel, randomPropertiesId, randomPropertiesSeed, context) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::DEL_CHAR_VOID_STORAGE_ITEM_BY_CHAR_GUID => {
                "DELETE FROM character_void_storage WHERE playerGuid = ?"
            }
            Self::DEL_CHAR_VOID_STORAGE_ITEM_BY_SLOT => {
                "DELETE FROM character_void_storage WHERE slot = ? AND playerGuid = ?"
            }
            Self::SEL_CHAR_CUF_PROFILES => {
                "SELECT id, name, frameHeight, frameWidth, sortBy, healthText, boolOptions, topPoint, bottomPoint, leftPoint, topOffset, bottomOffset, leftOffset FROM character_cuf_profiles WHERE guid = ?"
            }
            Self::REP_CHAR_CUF_PROFILES => {
                "REPLACE INTO character_cuf_profiles (guid, id, name, frameHeight, frameWidth, sortBy, healthText, boolOptions, topPoint, bottomPoint, leftPoint, topOffset, bottomOffset, leftOffset) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::DEL_CHAR_CUF_PROFILES_BY_ID => {
                "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?"
            }
            Self::DEL_CHAR_CUF_PROFILES => "DELETE FROM character_cuf_profiles WHERE guid = ?",
            Self::REP_CALENDAR_EVENT => {
                "REPLACE INTO calendar_events (EventID, Owner, Title, Description, EventType, TextureID, Date, Flags, LockDate) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::DEL_CALENDAR_EVENT => "DELETE FROM calendar_events WHERE EventID = ?",
            Self::REP_CALENDAR_INVITE => {
                "REPLACE INTO calendar_invites (InviteID, EventID, Invitee, Sender, Status, ResponseTime, ModerationRank, Note) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::DEL_CALENDAR_INVITE => "DELETE FROM calendar_invites WHERE InviteID = ?",
            Self::SEL_CHAR_PET_IDS => "SELECT id FROM character_pet WHERE owner = ?",
            Self::DEL_CHAR_PET_DECLINEDNAME_BY_OWNER => {
                "DELETE FROM character_pet_declinedname WHERE owner = ?"
            }
            Self::DEL_CHAR_PET_DECLINEDNAME => {
                "DELETE FROM character_pet_declinedname WHERE id = ?"
            }
            Self::INS_CHAR_PET_DECLINEDNAME => {
                "INSERT INTO character_pet_declinedname (id, owner, genitive, dative, accusative, instrumental, prepositional) VALUES (?, ?, ?, ?, ?, ?, ?)"
            }
            Self::SEL_PET_AURA => {
                "SELECT casterGuid, spell, effectMask, recalculateMask, difficulty, stackCount, maxDuration, remainTime, remainCharges FROM pet_aura WHERE guid = ?"
            }
            Self::SEL_PET_AURA_EFFECT => {
                "SELECT casterGuid, spell, effectMask, effectIndex, amount, baseAmount FROM pet_aura_effect WHERE guid = ?"
            }
            Self::SEL_PET_SPELL => "SELECT spell, active FROM pet_spell WHERE guid = ?",
            Self::SEL_PET_SPELL_COOLDOWN => {
                "SELECT spell, time, categoryId, categoryEnd FROM pet_spell_cooldown WHERE guid = ? AND time > UNIX_TIMESTAMP()"
            }
            Self::SEL_PET_DECLINED_NAME => {
                "SELECT genitive, dative, accusative, instrumental, prepositional FROM character_pet_declinedname WHERE owner = ? AND id = ?"
            }
            Self::DEL_PET_AURAS => "DELETE FROM pet_aura WHERE guid = ?",
            Self::DEL_PET_AURA_EFFECTS => "DELETE FROM pet_aura_effect WHERE guid = ?",
            Self::DEL_PET_SPELLS => "DELETE FROM pet_spell WHERE guid = ?",
            Self::DEL_PET_SPELL_COOLDOWNS => "DELETE FROM pet_spell_cooldown WHERE guid = ?",
            Self::INS_PET_SPELL_COOLDOWN => {
                "INSERT INTO pet_spell_cooldown (guid, spell, time, categoryId, categoryEnd) VALUES (?, ?, ?, ?, ?)"
            }
            Self::SEL_PET_SPELL_CHARGES => {
                "SELECT categoryId, rechargeStart, rechargeEnd FROM pet_spell_charges WHERE guid = ? AND rechargeEnd > UNIX_TIMESTAMP() ORDER BY rechargeEnd"
            }
            Self::DEL_PET_SPELL_CHARGES => "DELETE FROM pet_spell_charges WHERE guid = ?",
            Self::INS_PET_SPELL_CHARGES => {
                "INSERT INTO pet_spell_charges (guid, categoryId, rechargeStart, rechargeEnd) VALUES (?, ?, ?, ?)"
            }
            Self::DEL_PET_SPELL_BY_SPELL => "DELETE FROM pet_spell WHERE guid = ? and spell = ?",
            Self::INS_PET_SPELL => "INSERT INTO pet_spell (guid, spell, active) VALUES (?, ?, ?)",
            Self::INS_PET_AURA => {
                "INSERT INTO pet_aura (guid, casterGuid, spell, effectMask, recalculateMask, difficulty, stackCount, maxDuration, remainTime, remainCharges) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::INS_PET_AURA_EFFECT => {
                "INSERT INTO pet_aura_effect (guid, casterGuid, spell, effectMask, effectIndex, amount, baseAmount) VALUES (?, ?, ?, ?, ?, ?, ?)"
            }
            Self::SEL_CHAR_PETS => {
                "SELECT id, entry, modelid, level, exp, Reactstate, slot, name, renamed, curhealth, curmana, abdata, savetime, CreatedBySpell, PetType, specialization FROM character_pet WHERE owner = ?"
            }
            Self::SEL_CHARACTER_INVENTORY => {
                "SELECT ii.guid, ii.itemEntry, ii.creatorGuid, ii.giftCreatorGuid, ii.count, ii.duration, ii.charges, ii.flags, ii.enchantments, ii.durability, ii.playedTime, ii.text, ii.battlePetSpeciesId, ii.battlePetBreedData, ii.battlePetLevel, ii.battlePetDisplayId, ii.randomPropertiesId, ii.randomPropertiesSeed, ii.context, iit.itemModifiedAppearanceAllSpecs, iit.itemModifiedAppearanceSpec1, iit.itemModifiedAppearanceSpec2, iit.itemModifiedAppearanceSpec3, iit.itemModifiedAppearanceSpec4, iit.itemModifiedAppearanceSpec5, iit.spellItemEnchantmentAllSpecs, iit.spellItemEnchantmentSpec1, iit.spellItemEnchantmentSpec2, iit.spellItemEnchantmentSpec3, iit.spellItemEnchantmentSpec4, iit.spellItemEnchantmentSpec5, iit.secondaryItemModifiedAppearanceAllSpecs, iit.secondaryItemModifiedAppearanceSpec1, iit.secondaryItemModifiedAppearanceSpec2, iit.secondaryItemModifiedAppearanceSpec3, iit.secondaryItemModifiedAppearanceSpec4, iit.itemModifiedAppearanceSpec5, ig.gemItemId1, ig.gemBonuses1, ig.gemContext1, ig.gemItemId2, ig.gemBonuses2, ig.gemContext2, ig.gemItemId3, ig.gemBonuses3, ig.gemContext3, bag, slot FROM character_inventory ci JOIN item_instance ii ON ci.item = ii.guid LEFT JOIN item_instance_gems ig ON ii.guid = ig.itemGuid LEFT JOIN item_instance_transmog iit ON ii.guid = iit.itemGuid WHERE ci.guid = ? ORDER BY (ii.flags & 0x80000) ASC, bag ASC, slot ASC"
            }
            Self::SEL_MAILITEMS => {
                "SELECT ii.guid, ii.itemEntry, ii.creatorGuid, ii.giftCreatorGuid, ii.count, ii.duration, ii.charges, ii.flags, ii.enchantments, ii.durability, ii.playedTime, ii.text, ii.battlePetSpeciesId, ii.battlePetBreedData, ii.battlePetLevel, ii.battlePetDisplayId, ii.randomPropertiesId, ii.randomPropertiesSeed, ii.context, iit.itemModifiedAppearanceAllSpecs, iit.itemModifiedAppearanceSpec1, iit.itemModifiedAppearanceSpec2, iit.itemModifiedAppearanceSpec3, iit.itemModifiedAppearanceSpec4, iit.itemModifiedAppearanceSpec5, iit.spellItemEnchantmentAllSpecs, iit.spellItemEnchantmentSpec1, iit.spellItemEnchantmentSpec2, iit.spellItemEnchantmentSpec3, iit.spellItemEnchantmentSpec4, iit.spellItemEnchantmentSpec5, iit.secondaryItemModifiedAppearanceAllSpecs, iit.secondaryItemModifiedAppearanceSpec1, iit.secondaryItemModifiedAppearanceSpec2, iit.secondaryItemModifiedAppearanceSpec3, iit.secondaryItemModifiedAppearanceSpec4, iit.itemModifiedAppearanceSpec5, ig.gemItemId1, ig.gemBonuses1, ig.gemContext1, ig.gemItemId2, ig.gemBonuses2, ig.gemContext2, ig.gemItemId3, ig.gemBonuses3, ig.gemContext3, ii.owner_guid, m.id FROM mail_items mi INNER JOIN mail m ON mi.mail_id = m.id LEFT JOIN item_instance ii ON mi.item_guid = ii.guid LEFT JOIN item_instance_gems ig ON ii.guid = ig.itemGuid LEFT JOIN item_instance_transmog iit ON ii.guid = iit.itemGuid WHERE m.receiver = ?"
            }
            Self::SEL_AUCTION_ITEMS => {
                "SELECT ii.guid, ii.itemEntry, ii.creatorGuid, ii.giftCreatorGuid, ii.count, ii.duration, ii.charges, ii.flags, ii.enchantments, ii.durability, ii.playedTime, ii.text, ii.battlePetSpeciesId, ii.battlePetBreedData, ii.battlePetLevel, ii.battlePetDisplayId, ii.randomPropertiesId, ii.randomPropertiesSeed, ii.context, iit.itemModifiedAppearanceAllSpecs, iit.itemModifiedAppearanceSpec1, iit.itemModifiedAppearanceSpec2, iit.itemModifiedAppearanceSpec3, iit.itemModifiedAppearanceSpec4, iit.itemModifiedAppearanceSpec5, iit.spellItemEnchantmentAllSpecs, iit.spellItemEnchantmentSpec1, iit.spellItemEnchantmentSpec2, iit.spellItemEnchantmentSpec3, iit.spellItemEnchantmentSpec4, iit.spellItemEnchantmentSpec5, iit.secondaryItemModifiedAppearanceAllSpecs, iit.secondaryItemModifiedAppearanceSpec1, iit.secondaryItemModifiedAppearanceSpec2, iit.secondaryItemModifiedAppearanceSpec3, iit.secondaryItemModifiedAppearanceSpec4, iit.itemModifiedAppearanceSpec5, ig.gemItemId1, ig.gemBonuses1, ig.gemContext1, ig.gemItemId2, ig.gemBonuses2, ig.gemContext2, ig.gemItemId3, ig.gemBonuses3, ig.gemContext3, ii.owner_guid, ai.auctionId FROM auction_items ai INNER JOIN item_instance ii ON ai.itemGuid = ii.guid LEFT JOIN item_instance_gems ig ON ii.guid = ig.itemGuid LEFT JOIN item_instance_transmog iit ON ii.guid = iit.itemGuid"
            }
            Self::SEL_GUILD_BANK_ITEMS => {
                "SELECT ii.guid, ii.itemEntry, ii.creatorGuid, ii.giftCreatorGuid, ii.count, ii.duration, ii.charges, ii.flags, ii.enchantments, ii.durability, ii.playedTime, ii.text, ii.battlePetSpeciesId, ii.battlePetBreedData, ii.battlePetLevel, ii.battlePetDisplayId, ii.randomPropertiesId, ii.randomPropertiesSeed, ii.context, iit.itemModifiedAppearanceAllSpecs, iit.itemModifiedAppearanceSpec1, iit.itemModifiedAppearanceSpec2, iit.itemModifiedAppearanceSpec3, iit.itemModifiedAppearanceSpec4, iit.itemModifiedAppearanceSpec5, iit.spellItemEnchantmentAllSpecs, iit.spellItemEnchantmentSpec1, iit.spellItemEnchantmentSpec2, iit.spellItemEnchantmentSpec3, iit.spellItemEnchantmentSpec4, iit.spellItemEnchantmentSpec5, iit.secondaryItemModifiedAppearanceAllSpecs, iit.secondaryItemModifiedAppearanceSpec1, iit.secondaryItemModifiedAppearanceSpec2, iit.secondaryItemModifiedAppearanceSpec3, iit.secondaryItemModifiedAppearanceSpec4, iit.itemModifiedAppearanceSpec5, ig.gemItemId1, ig.gemBonuses1, ig.gemContext1, ig.gemItemId2, ig.gemBonuses2, ig.gemContext2, ig.gemItemId3, ig.gemBonuses3, ig.gemContext3, guildid, TabId, SlotId FROM guild_bank_item gbi INNER JOIN item_instance ii ON gbi.item_guid = ii.guid LEFT JOIN item_instance_gems ig ON ii.guid = ig.itemGuid LEFT JOIN item_instance_transmog iit ON ii.guid = iit.itemGuid"
            }
            Self::DEL_CHAR_PET_BY_OWNER => "DELETE FROM character_pet WHERE owner = ?",
            Self::UPD_CHAR_PET_NAME => {
                "UPDATE character_pet SET name = ?, renamed = 1 WHERE owner = ? AND id = ?"
            }
            Self::UPD_CHAR_PET_SLOT_BY_ID => {
                "UPDATE character_pet SET slot = ? WHERE owner = ? AND id = ?"
            }
            Self::DEL_CHAR_PET_BY_ID => "DELETE FROM character_pet WHERE id = ?",
            Self::DEL_ALL_PET_SPELLS_BY_OWNER => {
                "DELETE FROM pet_spell WHERE guid in (SELECT id FROM character_pet WHERE owner=?)"
            }
            Self::UPD_PET_SPECS_BY_OWNER => {
                "UPDATE character_pet SET specialization = 0 WHERE owner=?"
            }
            Self::INS_PET => {
                "INSERT INTO character_pet (id, entry, owner, modelid, level, exp, Reactstate, slot, name, renamed, curhealth, curmana, abdata, savetime, CreatedBySpell, PetType, specialization) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::SEL_PVPSTATS_MAXID => "SELECT MAX(id) FROM pvpstats_battlegrounds",
            Self::INS_PVPSTATS_BATTLEGROUND => {
                "INSERT INTO pvpstats_battlegrounds (id, winner_faction, bracket_id, type, date) VALUES (?, ?, ?, ?, NOW())"
            }
            Self::INS_PVPSTATS_PLAYER => {
                "INSERT INTO pvpstats_players (battleground_id, character_guid, winner, score_killing_blows, score_deaths, score_honorable_kills, score_bonus_honor, score_damage_done, score_healing_done, attr_1, attr_2, attr_3, attr_4, attr_5) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::SEL_PVPSTATS_FACTIONS_OVERALL => {
                "SELECT winner_faction, COUNT(*) AS count FROM pvpstats_battlegrounds WHERE DATEDIFF(NOW(), date) < 7 GROUP BY winner_faction ORDER BY winner_faction ASC"
            }
            Self::INS_QUEST_TRACK => {
                "INSERT INTO quest_tracker (id, character_guid, quest_accept_time, core_hash, core_revision) VALUES (?, ?, NOW(), ?, ?)"
            }
            Self::UPD_QUEST_TRACK_GM_COMPLETE => {
                "UPDATE quest_tracker SET completed_by_gm = 1 WHERE id = ? AND character_guid = ? ORDER BY quest_accept_time DESC LIMIT 1"
            }
            Self::UPD_QUEST_TRACK_COMPLETE_TIME => {
                "UPDATE quest_tracker SET quest_complete_time = NOW() WHERE id = ? AND character_guid = ? ORDER BY quest_accept_time DESC LIMIT 1"
            }
            Self::UPD_QUEST_TRACK_ABANDON_TIME => {
                "UPDATE quest_tracker SET quest_abandon_time = NOW() WHERE id = ? AND character_guid = ? ORDER BY quest_accept_time DESC LIMIT 1"
            }
            Self::SEL_CHARACTER_AURA_STORED_LOCATIONS => {
                "SELECT Spell, MapId, PositionX, PositionY, PositionZ, Orientation FROM character_aura_stored_location WHERE Guid = ?"
            }
            Self::DEL_CHARACTER_AURA_STORED_LOCATIONS_BY_GUID => {
                "DELETE FROM character_aura_stored_location WHERE Guid = ?"
            }
            Self::DEL_CHARACTER_AURA_STORED_LOCATION => {
                "DELETE FROM character_aura_stored_location WHERE Guid = ? AND Spell = ?"
            }
            Self::INS_CHARACTER_AURA_STORED_LOCATION => {
                "INSERT INTO character_aura_stored_location (Guid, Spell, MapId, PositionX, PositionY, PositionZ, Orientation) VALUES (?, ?, ?, ?, ?, ?, ?)"
            }
            Self::SEL_WAR_MODE_TUNING => {
                "SELECT race, COUNT(guid) FROM characters WHERE ((playerFlags & ?) = ?) AND logout_time >= (UNIX_TIMESTAMP() - 604800) GROUP BY race"
            }
            Self::UPD_CHAR_XP => "UPDATE characters SET xp = ? WHERE guid = ?",
            Self::UPD_CHAR_LEVEL => "UPDATE characters SET level = ?, xp = ? WHERE guid = ?",
            Self::UPD_CHAR_MONEY => "UPDATE characters SET money = ? WHERE guid = ?",
            Self::UPD_CHAR_PLAYER_FLAGS => "UPDATE characters SET playerFlags = ? WHERE guid = ?",
            Self::SEL_CHAR_MONEY_FOR_UPDATE => {
                "SELECT money FROM characters WHERE guid = ? FOR UPDATE"
            }
            Self::SEL_CHAR_MONEY => "SELECT money FROM characters WHERE guid = ?",
            Self::UPD_CHAR_HEALTH => "UPDATE characters SET health = ? WHERE guid = ?",
            Self::UPD_CHAR_POWERS => {
                "UPDATE characters SET power1 = ?, power2 = ?, power3 = ?, power4 = ?, power5 = ?, power6 = ?, power7 = ?, power8 = ?, power9 = ?, power10 = ? WHERE guid = ?"
            }
            Self::UPD_CHAR_REST_STATE => {
                "UPDATE characters SET restState = ?, playerFlags = ?, rest_bonus = ?, logout_time = ?, is_logout_resting = ? WHERE guid = ?"
            }
            Self::UPD_CHAR_ONLINE_REST_STATE => {
                "UPDATE characters SET restState = ?, playerFlags = ?, rest_bonus = ? WHERE guid = ?"
            }
            Self::UPD_CHAR_TALENT_RESET_STATE => {
                "UPDATE characters SET resettalents_cost = ?, resettalents_time = ? WHERE guid = ?"
            }
            Self::UPD_CHAR_DIFFICULTIES => {
                "UPDATE characters SET dungeonDifficulty = ?, raidDifficulty = ?, legacyRaidDifficulty = ? WHERE guid = ?"
            }
            Self::UPD_CHAR_EXPLORED_ZONES => {
                "UPDATE characters SET exploredZones = ? WHERE guid = ?"
            }
            Self::SEL_MAX_ITEM_GUID => "SELECT MAX(guid) FROM item_instance",
            Self::SEL_MAX_EQUIPMENT_SET_GUID => {
                // The equipment table uses BIGINT UNSIGNED while the canonical
                // transmog table uses signed BIGINT. MariaDB can promote their
                // UNION/MAX result to DECIMAL; pin the wire type so startup can
                // decode the shared raw uint64 namespace without driver-specific
                // signed/decimal coercion.
                "SELECT CAST(MAX(maxguid) AS UNSIGNED) FROM ((SELECT MAX(setguid) AS maxguid FROM character_equipmentsets) UNION (SELECT MAX(setguid) AS maxguid FROM character_transmog_outfits)) allsets"
            }
            Self::SEL_MAX_VOID_STORAGE_ITEM_ID => "SELECT MAX(itemId) FROM character_void_storage",
            Self::DEL_INVALID_CHAR_INVENTORY_ITEM_GUIDS => {
                "DELETE FROM character_inventory WHERE item >= ?"
            }
            Self::DEL_INVALID_MAIL_ITEM_GUIDS => "DELETE FROM mail_items WHERE item_guid >= ?",
            Self::DEL_INVALID_AUCTION_ITEM_GUIDS => {
                "DELETE a, ab, ai FROM auctionhouse a LEFT JOIN auction_bidders ab ON ab.auctionId = a.id LEFT JOIN auction_items ai ON ai.auctionId = a.id WHERE ai.itemGuid >= ?"
            }
            Self::DEL_INVALID_GUILD_BANK_ITEM_GUIDS => {
                "DELETE FROM guild_bank_item WHERE item_guid >= ?"
            }
            Self::DEL_INVALID_ITEM_LOOT_ITEMS_GUIDS => {
                "DELETE FROM item_loot_items WHERE container_id >= ?"
            }
            Self::DEL_INVALID_ITEM_LOOT_MONEY_GUIDS => {
                "DELETE FROM item_loot_money WHERE container_id >= ?"
            }
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
                 VALUES (?, ?, ?, 0, 0, ?, ?, '', '', ?, ?, ?, ?)"
            }
            Self::INS_ITEM_INSTANCE_CLONE => {
                "INSERT INTO item_instance \
                 (guid, itemEntry, owner_guid, creatorGuid, giftCreatorGuid, count, \
                  duration, charges, enchantments, flags, durability, playedTime, \
                  randomPropertiesId, randomPropertiesSeed, context) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::UPD_ITEM_INSTANCE_COUNT => "UPDATE item_instance SET count = ? WHERE guid = ?",
            Self::UPD_ITEM_INSTANCE_DURABILITY => {
                "UPDATE item_instance SET durability = ? WHERE guid = ?"
            }
            Self::UPD_ITEM_INSTANCE_FLAGS => "UPDATE item_instance SET flags = ? WHERE guid = ?",
            Self::UPD_ITEM_INSTANCE_ENCHANTMENTS => {
                "UPDATE item_instance SET enchantments = ? WHERE guid = ?"
            }
            Self::UPD_ITEM_INSTANCE_STORAGE_MUTABLE => {
                "UPDATE item_instance SET count = ?, duration = ?, charges = ?, flags = ?, enchantments = ?, durability = ?, playedTime = ? WHERE guid = ?"
            }
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
            Self::DEL_ITEM_INSTANCE_BY_GUID_AND_OWNER => {
                "DELETE FROM item_instance WHERE guid = ? AND owner_guid = ?"
            }
            Self::SEL_UNCAGE_ITEM_STATE => {
                "SELECT (SELECT owner_guid FROM item_instance WHERE guid = ? LIMIT 1), EXISTS(SELECT 1 FROM character_inventory WHERE guid = ? AND item = ?)"
            }
            Self::INS_BATTLE_PET_PURCHASE => {
                "INSERT INTO character_battle_pet_purchase (request_key, guid, account_id, trainer_id, spell_id, species, breed, quality, display_id, level, price, money_before, money_after, status) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::SEL_BATTLE_PET_PURCHASE_BY_KEY => {
                "SELECT request_key, guid, account_id, trainer_id, spell_id, species, breed, quality, display_id, level, price, money_before, money_after, status, failure_reason, published FROM character_battle_pet_purchase WHERE request_key = ?"
            }
            Self::SEL_BATTLE_PET_PURCHASE_PENDING => {
                "SELECT request_key, guid, account_id, trainer_id, spell_id, species, breed, quality, display_id, level, price, money_before, money_after, status, failure_reason, published FROM character_battle_pet_purchase WHERE guid = ? AND (status IN (0, 2) OR (status = 1 AND published = 0)) ORDER BY created_at ASC, request_key ASC LIMIT ?"
            }
            Self::UPD_BATTLE_PET_PURCHASE_PUBLISHED => {
                "UPDATE character_battle_pet_purchase SET published = 1 WHERE request_key = ? AND published = 0 AND status IN (0, 1, 2)"
            }
            Self::UPD_BATTLE_PET_PURCHASE_COMPLETED => {
                "UPDATE character_battle_pet_purchase SET status = 1, failure_reason = NULL WHERE request_key = ? AND status IN (0, 2)"
            }
            Self::UPD_BATTLE_PET_PURCHASE_COMPENSATION_PENDING => {
                "UPDATE character_battle_pet_purchase SET status = 2, failure_reason = ? WHERE request_key = ? AND status = 0"
            }
            Self::UPD_BATTLE_PET_PURCHASE_COMPENSATED => {
                "UPDATE character_battle_pet_purchase SET status = 3 WHERE request_key = ? AND status = 2"
            }
            Self::UPD_BATTLE_PET_PURCHASE_TERMINAL_FAILURE => {
                "UPDATE character_battle_pet_purchase SET status = 4, failure_reason = ? WHERE request_key = ? AND status = 2"
            }
            Self::UPD_CHARACTER_MONEY_GUARDED => {
                "UPDATE characters SET money = ? WHERE guid = ? AND money = ?"
            }
            Self::UPD_CHARACTER_MONEY_REFUND => {
                "UPDATE characters SET money = LEAST(money + ?, ?) WHERE guid = ?"
            }
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
            Self::INS_GUILD => {
                "INSERT INTO guild (guildid, name, leaderguid, info, motd, createdate, EmblemStyle, EmblemColor, BorderStyle, BorderColor, BackgroundColor, BankMoney) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::DEL_GUILD => "DELETE FROM guild WHERE guildid = ?",
            Self::UPD_GUILD_NAME => "UPDATE guild SET name = ? WHERE guildid = ?",
            Self::INS_GUILD_MEMBER => {
                "INSERT INTO guild_member (guildid, guid, `rank`, pnote, offnote) VALUES (?, ?, ?, ?, ?)"
            }
            Self::DEL_GUILD_MEMBER => "DELETE FROM guild_member WHERE guid = ?",
            Self::DEL_GUILD_MEMBERS => "DELETE FROM guild_member WHERE guildid = ?",
            Self::INS_GUILD_RANK => {
                "INSERT INTO guild_rank (guildid, rid, RankOrder, rname, rights, BankMoneyPerDay) VALUES (?, ?, ?, ?, ?, ?)"
            }
            Self::DEL_GUILD_RANKS => "DELETE FROM guild_rank WHERE guildid = ?",
            Self::DEL_GUILD_RANK => "DELETE FROM guild_rank WHERE guildid = ? AND rid = ?",
            Self::INS_GUILD_BANK_TAB => "INSERT INTO guild_bank_tab (guildid, TabId) VALUES (?, ?)",
            Self::DEL_GUILD_BANK_TAB => {
                "DELETE FROM guild_bank_tab WHERE guildid = ? AND TabId = ?"
            }
            Self::DEL_GUILD_BANK_TABS => "DELETE FROM guild_bank_tab WHERE guildid = ?",
            Self::INS_GUILD_BANK_ITEM => {
                "INSERT INTO guild_bank_item (guildid, TabId, SlotId, item_guid) VALUES (?, ?, ?, ?)"
            }
            Self::DEL_GUILD_BANK_ITEM => {
                "DELETE FROM guild_bank_item WHERE guildid = ? AND TabId = ? AND SlotId = ?"
            }
            Self::DEL_GUILD_BANK_ITEMS => "DELETE FROM guild_bank_item WHERE guildid = ?",
            Self::INS_GUILD_BANK_RIGHT => {
                "INSERT INTO guild_bank_right (guildid, TabId, rid, gbright, SlotPerDay) VALUES (?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE gbright = VALUES(gbright), SlotPerDay = VALUES(SlotPerDay)"
            }
            Self::DEL_GUILD_BANK_RIGHTS => "DELETE FROM guild_bank_right WHERE guildid = ?",
            Self::DEL_GUILD_BANK_RIGHTS_FOR_RANK => {
                "DELETE FROM guild_bank_right WHERE guildid = ? AND rid = ?"
            }
            Self::INS_GUILD_BANK_EVENTLOG => {
                "INSERT INTO guild_bank_eventlog (guildid, LogGuid, TabId, EventType, PlayerGuid, ItemOrMoney, ItemStackCount, DestTabId, TimeStamp) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::DEL_GUILD_BANK_EVENTLOG => {
                "DELETE FROM guild_bank_eventlog WHERE guildid = ? AND LogGuid = ? AND TabId = ?"
            }
            Self::DEL_GUILD_BANK_EVENTLOGS => "DELETE FROM guild_bank_eventlog WHERE guildid = ?",
            Self::INS_GUILD_EVENTLOG => {
                "INSERT INTO guild_eventlog (guildid, LogGuid, EventType, PlayerGuid1, PlayerGuid2, NewRank, TimeStamp) VALUES (?, ?, ?, ?, ?, ?, ?)"
            }
            Self::DEL_GUILD_EVENTLOG => {
                "DELETE FROM guild_eventlog WHERE guildid = ? AND LogGuid = ?"
            }
            Self::DEL_GUILD_EVENTLOGS => "DELETE FROM guild_eventlog WHERE guildid = ?",
            Self::UPD_GUILD_MEMBER_PNOTE => "UPDATE guild_member SET pnote = ? WHERE guid = ?",
            Self::UPD_GUILD_MEMBER_OFFNOTE => "UPDATE guild_member SET offnote = ? WHERE guid = ?",
            Self::UPD_GUILD_MEMBER_RANK => "UPDATE guild_member SET `rank` = ? WHERE guid = ?",
            Self::UPD_GUILD_MOTD => "UPDATE guild SET motd = ? WHERE guildid = ?",
            Self::UPD_GUILD_INFO => "UPDATE guild SET info = ? WHERE guildid = ?",
            Self::UPD_GUILD_LEADER => "UPDATE guild SET leaderguid = ? WHERE guildid = ?",
            Self::UPD_GUILD_RANK_ORDER => {
                "UPDATE guild_rank SET RankOrder = ? WHERE rid = ? AND guildid = ?"
            }
            Self::UPD_GUILD_RANK_NAME => {
                "UPDATE guild_rank SET rname = ? WHERE rid = ? AND guildid = ?"
            }
            Self::UPD_GUILD_RANK_RIGHTS => {
                "UPDATE guild_rank SET rights = ? WHERE rid = ? AND guildid = ?"
            }
            Self::UPD_GUILD_EMBLEM_INFO => {
                "UPDATE guild SET EmblemStyle = ?, EmblemColor = ?, BorderStyle = ?, BorderColor = ?, BackgroundColor = ? WHERE guildid = ?"
            }
            Self::UPD_GUILD_BANK_TAB_INFO => {
                "UPDATE guild_bank_tab SET TabName = ?, TabIcon = ? WHERE guildid = ? AND TabId = ?"
            }
            Self::UPD_GUILD_BANK_MONEY => "UPDATE guild SET BankMoney = ? WHERE guildid = ?",
            Self::UPD_GUILD_RANK_BANK_MONEY => {
                "UPDATE guild_rank SET BankMoneyPerDay = ? WHERE rid = ? AND guildid = ?"
            }
            Self::UPD_GUILD_BANK_TAB_TEXT => {
                "UPDATE guild_bank_tab SET TabText = ? WHERE guildid = ? AND TabId = ?"
            }
            Self::INS_GUILD_MEMBER_WITHDRAW_TABS => {
                "INSERT INTO guild_member_withdraw (guid, tab0, tab1, tab2, tab3, tab4, tab5, tab6, tab7) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE tab0 = VALUES (tab0), tab1 = VALUES (tab1), tab2 = VALUES (tab2), tab3 = VALUES (tab3), tab4 = VALUES (tab4), tab5 = VALUES (tab5), tab6 = VALUES (tab6), tab7 = VALUES (tab7)"
            }
            Self::INS_GUILD_MEMBER_WITHDRAW_MONEY => {
                "INSERT INTO guild_member_withdraw (guid, money) VALUES (?, ?) ON DUPLICATE KEY UPDATE money = VALUES (money)"
            }
            Self::DEL_GUILD_MEMBER_WITHDRAW => "DELETE FROM guild_member_withdraw",
            Self::SEL_CHAR_DATA_FOR_GUILD => {
                "SELECT name, level, race, class, gender, zone, account FROM characters WHERE guid = ?"
            }
            Self::DEL_GUILD_ACHIEVEMENT => {
                "DELETE FROM guild_achievement WHERE guildId = ? AND achievement = ?"
            }
            Self::INS_GUILD_ACHIEVEMENT => {
                "INSERT INTO guild_achievement (guildId, achievement, date, guids) VALUES (?, ?, ?, ?)"
            }
            Self::DEL_GUILD_ACHIEVEMENT_CRITERIA => {
                "DELETE FROM guild_achievement_progress WHERE guildId = ? AND criteria = ?"
            }
            Self::INS_GUILD_ACHIEVEMENT_CRITERIA => {
                "INSERT INTO guild_achievement_progress (guildId, criteria, counter, date, completedGuid) VALUES (?, ?, ?, ?, ?)"
            }
            Self::DEL_ALL_GUILD_ACHIEVEMENTS => {
                "DELETE FROM guild_achievement WHERE guildId = ? AND achievement NOT IN (5407,5408,5409,5410,5411,5985,6126,6628,6678,6679,6680,8257,8512,8513,9397,9399,10380)"
            }
            Self::DEL_ALL_GUILD_ACHIEVEMENT_CRITERIA => {
                "DELETE FROM guild_achievement_progress WHERE guildId = ?"
            }
            Self::SEL_GUILD_ACHIEVEMENT => {
                "SELECT achievement, date, guids FROM guild_achievement WHERE guildId = ?"
            }
            Self::SEL_GUILD_ACHIEVEMENT_CRITERIA => {
                "SELECT criteria, counter, date, completedGuid FROM guild_achievement_progress WHERE guildId = ?"
            }
            Self::INS_GUILD_NEWS => {
                "INSERT INTO guild_newslog (guildid, LogGuid, EventType, PlayerGuid, Flags, Value, Timestamp) VALUES (?, ?, ?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE LogGuid = VALUES (LogGuid), EventType = VALUES (EventType), PlayerGuid = VALUES (PlayerGuid), Flags = VALUES (Flags), Value = VALUES (Value), Timestamp = VALUES (Timestamp)"
            }
            Self::UPD_CHANNEL => {
                "INSERT INTO channels (name, team, announce, ownership, password, bannedList, lastUsed) VALUES (?, ?, ?, ?, ?, ?, UNIX_TIMESTAMP()) ON DUPLICATE KEY UPDATE announce=VALUES(announce), ownership=VALUES(ownership), password=VALUES(password), bannedList=VALUES(bannedList), lastUsed=VALUES(lastUsed)"
            }
            Self::UPD_CHANNEL_USAGE => {
                "UPDATE channels SET lastUsed = UNIX_TIMESTAMP() WHERE name = ? AND team = ?"
            }
            Self::UPD_CHANNEL_OWNERSHIP => "UPDATE channels SET ownership = ? WHERE name LIKE ?",
            Self::DEL_CHANNEL => "DELETE FROM channels WHERE name = ? AND team = ?",
            Self::DEL_OLD_CHANNELS => {
                "DELETE FROM channels WHERE ownership = 1 AND lastUsed + ? < UNIX_TIMESTAMP()"
            }
            Self::UPD_EQUIP_SET => {
                "UPDATE character_equipmentsets SET name=?, iconname=?, ignore_mask=?, AssignedSpecIndex=?, item0=?, item1=?, item2=?, item3=?, item4=?, item5=?, item6=?, item7=?, item8=?, item9=?, item10=?, item11=?, item12=?, item13=?, item14=?, item15=?, item16=?, item17=?, item18=? WHERE guid=? AND setguid=? AND setindex=?"
            }
            Self::INS_EQUIP_SET => {
                "INSERT INTO character_equipmentsets (guid, setguid, setindex, name, iconname, ignore_mask, AssignedSpecIndex, item0, item1, item2, item3, item4, item5, item6, item7, item8, item9, item10, item11, item12, item13, item14, item15, item16, item17, item18) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::DEL_EQUIP_SET => "DELETE FROM character_equipmentsets WHERE setguid=?",
            Self::UPD_TRANSMOG_OUTFIT => {
                "UPDATE character_transmog_outfits SET name=?, iconname=?, ignore_mask=?, appearance0=?, appearance1=?, appearance2=?, appearance3=?, appearance4=?, appearance5=?, appearance6=?, appearance7=?, appearance8=?, appearance9=?, appearance10=?, appearance11=?, appearance12=?, appearance13=?, appearance14=?, appearance15=?, appearance16=?, appearance17=?, appearance18=?, mainHandEnchant=?, offHandEnchant=? WHERE guid=? AND setguid=? AND setindex=?"
            }
            Self::INS_TRANSMOG_OUTFIT => {
                "INSERT INTO character_transmog_outfits (guid, setguid, setindex, name, iconname, ignore_mask, appearance0, appearance1, appearance2, appearance3, appearance4, appearance5, appearance6, appearance7, appearance8, appearance9, appearance10, appearance11, appearance12, appearance13, appearance14, appearance15, appearance16, appearance17, appearance18, mainHandEnchant, offHandEnchant) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::DEL_TRANSMOG_OUTFIT => "DELETE FROM character_transmog_outfits WHERE setguid=?",
            Self::INS_AURA => {
                "INSERT INTO character_aura (guid, casterGuid, itemGuid, spell, effectMask, recalculateMask, difficulty, stackCount, maxDuration, remainTime, remainCharges, castItemId, castItemLevel) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::INS_AURA_EFFECT => {
                "INSERT INTO character_aura_effect (guid, casterGuid, itemGuid, spell, effectMask, effectIndex, amount, baseAmount) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::SEL_ACCOUNT_DATA => {
                "SELECT type, time, data FROM account_data WHERE accountId = ?"
            }
            Self::REP_ACCOUNT_DATA => {
                "REPLACE INTO account_data (accountId, type, time, data) VALUES (?, ?, ?, ?)"
            }
            Self::DEL_ACCOUNT_DATA => "DELETE FROM account_data WHERE accountId = ?",
            Self::SEL_PLAYER_ACCOUNT_DATA => {
                "SELECT type, time, data FROM character_account_data WHERE guid = ?"
            }
            Self::REP_PLAYER_ACCOUNT_DATA => {
                "REPLACE INTO character_account_data(guid, type, time, data) VALUES (?, ?, ?, ?)"
            }
            Self::DEL_PLAYER_ACCOUNT_DATA => "DELETE FROM character_account_data WHERE guid = ?",
            Self::SEL_TUTORIALS => {
                "SELECT tut0, tut1, tut2, tut3, tut4, tut5, tut6, tut7 FROM account_tutorial WHERE accountId = ?"
            }
            Self::INS_TUTORIALS => {
                "INSERT INTO account_tutorial(tut0, tut1, tut2, tut3, tut4, tut5, tut6, tut7, accountId) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::UPD_TUTORIALS => {
                "UPDATE account_tutorial SET tut0 = ?, tut1 = ?, tut2 = ?, tut3 = ?, tut4 = ?, tut5 = ?, tut6 = ?, tut7 = ? WHERE accountId = ?"
            }
            Self::DEL_TUTORIALS => "DELETE FROM account_tutorial WHERE accountId = ?",
            Self::SEL_PETITION => "SELECT ownerguid, name FROM petition WHERE petitionguid = ?",
            Self::SEL_PETITION_SIGNATURE => {
                "SELECT playerguid FROM petition_sign WHERE petitionguid = ?"
            }
            Self::DEL_ALL_PETITION_SIGNATURES => "DELETE FROM petition_sign WHERE playerguid = ?",
            Self::SEL_PETITION_BY_OWNER => "SELECT petitionguid FROM petition WHERE ownerguid = ?",
            Self::SEL_PETITION_SIGNATURES => {
                "SELECT ownerguid, (SELECT COUNT(playerguid) FROM petition_sign WHERE petition_sign.petitionguid = ?) AS signs FROM petition WHERE petitionguid = ?"
            }
            Self::SEL_PETITION_SIG_BY_ACCOUNT => {
                "SELECT playerguid FROM petition_sign WHERE player_account = ? AND petitionguid = ?"
            }
            Self::SEL_PETITION_OWNER_BY_GUID => {
                "SELECT ownerguid FROM petition WHERE petitionguid = ?"
            }
            Self::SEL_PETITION_SIG_BY_GUID => {
                "SELECT ownerguid, petitionguid FROM petition_sign WHERE playerguid = ?"
            }
            Self::SEL_CHARACTER_ARENAINFO => {
                "SELECT arenaTeamId, weekGames, seasonGames, seasonWins, personalRating FROM arena_team_member WHERE guid = ?"
            }
            Self::INS_ARENA_TEAM => {
                "INSERT INTO arena_team (arenaTeamId, name, captainGuid, type, rating, backgroundColor, emblemStyle, emblemColor, borderStyle, borderColor) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::INS_ARENA_TEAM_MEMBER => {
                "INSERT INTO arena_team_member (arenaTeamId, guid, personalRating) VALUES (?, ?, ?)"
            }
            Self::DEL_ARENA_TEAM => "DELETE FROM arena_team where arenaTeamId = ?",
            Self::DEL_ARENA_TEAM_MEMBERS => "DELETE FROM arena_team_member WHERE arenaTeamId = ?",
            Self::UPD_ARENA_TEAM_CAPTAIN => {
                "UPDATE arena_team SET captainGuid = ? WHERE arenaTeamId = ?"
            }
            Self::DEL_ARENA_TEAM_MEMBER => {
                "DELETE FROM arena_team_member WHERE arenaTeamId = ? AND guid = ?"
            }
            Self::UPD_ARENA_TEAM_STATS => {
                "UPDATE arena_team SET rating = ?, weekGames = ?, weekWins = ?, seasonGames = ?, seasonWins = ?, `rank` = ? WHERE arenaTeamId = ?"
            }
            Self::UPD_ARENA_TEAM_MEMBER => {
                "UPDATE arena_team_member SET personalRating = ?, weekGames = ?, weekWins = ?, seasonGames = ?, seasonWins = ? WHERE arenaTeamId = ? AND guid = ?"
            }
            Self::DEL_CHARACTER_ARENA_STATS => "DELETE FROM character_arena_stats WHERE guid = ?",
            Self::REP_CHARACTER_ARENA_STATS => {
                "REPLACE INTO character_arena_stats (guid, slot, matchMakerRating) VALUES (?, ?, ?)"
            }
            Self::UPD_ARENA_TEAM_NAME => "UPDATE arena_team SET name = ? WHERE arenaTeamId = ?",
            Self::INS_PLAYER_BGDATA => {
                "INSERT INTO character_battleground_data (guid, instanceId, team, joinX, joinY, joinZ, joinO, joinMapId, taxiStart, taxiEnd, mountSpell, queueId) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::DEL_PLAYER_BGDATA => "DELETE FROM character_battleground_data WHERE guid = ?",
            Self::INS_PLAYER_HOMEBIND => {
                "INSERT INTO character_homebind (guid, mapId, zoneId, posX, posY, posZ, orientation) VALUES (?, ?, ?, ?, ?, ?, ?)"
            }
            Self::UPD_PLAYER_HOMEBIND => {
                "UPDATE character_homebind SET mapId = ?, zoneId = ?, posX = ?, posY = ?, posZ = ?, orientation = ? WHERE guid = ?"
            }
            Self::DEL_PLAYER_HOMEBIND => "DELETE FROM character_homebind WHERE guid = ?",
            Self::SEL_CORPSES => {
                "SELECT posX, posY, posZ, orientation, mapId, displayId, itemCache, race, class, gender, flags, dynFlags, time, corpseType, instanceId, guid FROM corpse WHERE mapId = ? AND instanceId = ?"
            }
            Self::INS_CORPSE => {
                "INSERT INTO corpse (guid, posX, posY, posZ, orientation, mapId, displayId, itemCache, race, class, gender, flags, dynFlags, time, corpseType, instanceId) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            }
            Self::DEL_CORPSE => "DELETE FROM corpse WHERE guid = ?",
            Self::DEL_CORPSES_FROM_MAP => {
                "DELETE c, cc, cp FROM corpse c LEFT JOIN corpse_customizations cc ON c.guid = cc.ownerGuid LEFT JOIN corpse_phases cp ON c.guid = cp.OwnerGuid WHERE c.mapId = ? AND c.instanceId = ?"
            }
            Self::SEL_CORPSE_PHASES => {
                "SELECT cp.OwnerGuid, cp.PhaseId FROM corpse_phases cp LEFT JOIN corpse c ON cp.OwnerGuid = c.guid WHERE c.mapId = ? AND c.instanceId = ?"
            }
            Self::INS_CORPSE_PHASES => {
                "INSERT INTO corpse_phases (OwnerGuid, PhaseId) VALUES (?, ?)"
            }
            Self::DEL_CORPSE_PHASES => "DELETE FROM corpse_phases WHERE OwnerGuid = ?",
            Self::SEL_CORPSE_CUSTOMIZATIONS => {
                "SELECT cc.ownerGuid, cc.chrCustomizationOptionID, cc.chrCustomizationChoiceID FROM corpse_customizations cc LEFT JOIN corpse c ON cc.ownerGuid = c.guid WHERE c.mapId = ? AND c.instanceId = ? ORDER BY cc.ownerGuid, cc.chrCustomizationOptionID"
            }
            Self::INS_CORPSE_CUSTOMIZATIONS => {
                "INSERT INTO corpse_customizations (ownerGuid, chrCustomizationOptionID, chrCustomizationChoiceID) VALUES (?, ?, ?)"
            }
            Self::DEL_CORPSE_CUSTOMIZATIONS => {
                "DELETE FROM corpse_customizations WHERE ownerGuid = ?"
            }
            Self::SEL_CORPSE_LOCATION => {
                "SELECT mapId, posX, posY, posZ, orientation FROM corpse WHERE guid = ?"
            }
            Self::SEL_CHAR_BAG_CONTENTS => {
                "SELECT bag_ci.slot, ci.slot, ii.itemEntry, ci.item, ii.count, ii.durability, ii.context, \
                 ii.flags, ii.playedTime, ii.enchantments, ii.randomPropertiesId, \
                 ii.randomPropertiesSeed, ig.gemItemId1, ig.gemBonuses1, ig.gemContext1, \
                 ig.gemItemId2, ig.gemBonuses2, ig.gemContext2, \
                 ig.gemItemId3, ig.gemBonuses3, ig.gemContext3, \
                 ir.paidMoney, ir.paidExtendedCost, ii.duration, ii.charges \
                 FROM character_inventory ci \
                 JOIN character_inventory bag_ci \
                   ON bag_ci.guid = ci.guid AND bag_ci.item = ci.bag \
                 JOIN item_instance ii ON ci.item = ii.guid \
                 LEFT JOIN item_instance_gems ig ON ii.guid = ig.itemGuid \
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
            Self::SEL_ITEMCONTAINER_MONEY_FOR_UPDATE => {
                "SELECT money FROM item_loot_money WHERE container_id = ? FOR UPDATE"
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
            Self::GENERATED_CPP { sql, .. } => sql,
            Self::SEL_CHAR_QUEST_STATUS => {
                "SELECT quest, status, explored, acceptTime, endTime FROM character_queststatus WHERE guid = ? AND status <> 0"
            }
            Self::SEL_CHARACTER_QUESTSTATUS => {
                "SELECT quest, status, explored, acceptTime, endTime FROM character_queststatus WHERE guid = ? AND status <> 0"
            }
            Self::SEL_CHAR_QUEST_STATUS_OBJECTIVES => {
                "SELECT quest, objective, data FROM character_queststatus_objectives WHERE guid = ?"
            }
            Self::SEL_CHARACTER_QUESTSTATUS_OBJECTIVES => {
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
            Self::DEL_CHAR_QUESTSTATUS_OBJECTIVES_BY_QUEST => {
                "DELETE FROM character_queststatus_objectives WHERE guid = ? AND quest = ?"
            }
            Self::REP_CHAR_QUEST_STATUS_OBJECTIVES => {
                "REPLACE INTO character_queststatus_objectives (guid, quest, objective, data) VALUES (?, ?, ?, ?)"
            }
            Self::REP_CHAR_QUESTSTATUS_OBJECTIVES => {
                "REPLACE INTO character_queststatus_objectives (guid, quest, objective, data) VALUES (?, ?, ?, ?)"
            }
        }
    }
}
