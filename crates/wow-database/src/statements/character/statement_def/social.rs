//! Character social, guild, group, channel, and petition SQL values.
pub(super) const DEL_NONEXISTENT_GUILD_BANK_ITEM: &str =
    { "DELETE FROM guild_bank_item WHERE guildid = ? AND TabId = ? AND SlotId = ?" };
pub(super) const UPD_PETITION_NAME: &str = "UPDATE petition SET name = ? WHERE petitionguid = ?";
pub(super) const INS_PETITION_SIGNATURE: &str = {
    "INSERT INTO petition_sign (ownerguid, petitionguid, playerguid, player_account) VALUES (?, ?, ?, ?)"
};
pub(super) const SEL_GROUP_MEMBER: &str = "SELECT guid FROM group_member WHERE memberGuid = ?";
pub(super) const SEL_CHARACTER_SOCIALLIST: &str = {
    "SELECT cs.friend, c.account, cs.flags, cs.note FROM character_social cs JOIN characters c ON c.guid = cs.friend WHERE cs.guid = ? AND c.deleteinfos_name IS NULL LIMIT 255"
};
pub(super) const SEL_GUILD_MEMBER: &str = "SELECT guildid, `rank` FROM guild_member WHERE guid = ?";
pub(super) const SEL_GUILD_MEMBER_EXTENDED: &str = {
    "SELECT g.guildid, g.name, gr.rname, gr.rid, gm.pnote, gm.offnote FROM guild g JOIN guild_member gm ON g.guildid = gm.guildid JOIN guild_rank gr ON g.guildid = gr.guildid AND gm.`rank` = gr.rid WHERE gm.guid = ?"
};
pub(super) const UPD_GROUP_TYPE: &str = "UPDATE `groups` SET groupType = ? WHERE guid = ?";
pub(super) const UPD_GROUP_LEADER: &str = "UPDATE `groups` SET leaderGuid = ? WHERE guid = ?";
pub(super) const INS_GROUP: &str = {
    "INSERT INTO `groups` (guid, leaderGuid, lootMethod, looterGuid, lootThreshold, icon1, icon2, icon3, icon4, icon5, icon6, icon7, icon8, groupType, difficulty, raidDifficulty, legacyRaidDifficulty, masterLooterGuid) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
};
pub(super) const INS_GROUP_MEMBER: &str = {
    "INSERT INTO group_member (guid, memberGuid, memberFlags, subgroup, roles) VALUES(?, ?, ?, ?, ?)"
};
pub(super) const UPD_GROUP_MEMBER_SUBGROUP: &str =
    { "UPDATE group_member SET subgroup = ? WHERE memberGuid = ?" };
pub(super) const UPD_GROUP_MEMBER_FLAG: &str =
    { "UPDATE group_member SET memberFlags = ? WHERE memberGuid = ?" };
pub(super) const UPD_GROUP_DIFFICULTY: &str = "UPDATE `groups` SET difficulty = ? WHERE guid = ?";
pub(super) const UPD_GROUP_RAID_DIFFICULTY: &str =
    { "UPDATE `groups` SET raidDifficulty = ? WHERE guid = ?" };
pub(super) const UPD_GROUP_LEGACY_RAID_DIFFICULTY: &str =
    { "UPDATE `groups` SET legacyRaidDifficulty = ? WHERE guid = ?" };
pub(super) const DEL_GROUP_MEMBER: &str = "DELETE FROM group_member WHERE memberGuid = ?";
pub(super) const DEL_GROUP: &str = "DELETE FROM `groups` WHERE guid = ?";
pub(super) const DEL_GROUP_MEMBER_ALL: &str = "DELETE FROM group_member WHERE guid = ?";
pub(super) const DEL_LFG_DATA: &str = "DELETE FROM lfg_data WHERE guid = ?";
pub(super) const DEL_GROUP_MEMBERS_WITHOUT_CHARACTER: &str =
    { "DELETE FROM group_member WHERE memberGuid NOT IN (SELECT guid FROM characters)" };
pub(super) const DEL_GROUPS_WITHOUT_LEADER: &str =
    { "DELETE FROM `groups` WHERE leaderGuid NOT IN (SELECT guid FROM characters)" };
pub(super) const DEL_GROUPS_WITH_FEWER_THAN_TWO_MEMBERS: &str = {
    "DELETE FROM `groups` WHERE guid NOT IN (SELECT guid FROM group_member GROUP BY guid HAVING COUNT(guid) > 1)"
};
pub(super) const DEL_GROUP_MEMBERS_WITHOUT_GROUP: &str =
    { "DELETE FROM group_member WHERE guid NOT IN (SELECT guid FROM `groups`)" };
pub(super) const SEL_GROUPS: &str = {
    "SELECT g.leaderGuid, g.lootMethod, g.looterGuid, g.lootThreshold, g.icon1, g.icon2, g.icon3, g.icon4, g.icon5, g.icon6, g.icon7, g.icon8, g.groupType, g.difficulty, g.raiddifficulty, g.legacyRaidDifficulty, g.masterLooterGuid, g.guid, lfg.dungeon, lfg.state FROM `groups` g LEFT JOIN lfg_data lfg ON lfg.guid = g.guid ORDER BY g.guid ASC"
};
pub(super) const SEL_GROUP_MEMBERS: &str =
    { "SELECT guid, memberGuid, memberFlags, subgroup, roles FROM group_member ORDER BY guid" };
pub(super) const SEL_GROUP_MEMBER_CHARACTER_CACHE: &str = {
    "SELECT guid, name, race, class FROM characters WHERE guid IN (SELECT leaderGuid FROM `groups` UNION SELECT memberGuid FROM group_member)"
};
pub(super) const INS_LFG_DATA: &str =
    "INSERT INTO lfg_data (guid, dungeon, state) VALUES (?, ?, ?)";
pub(super) const DEL_INVALID_ACHIEV_PROGRESS_CRITERIA_GUILD: &str =
    { "DELETE FROM guild_achievement_progress WHERE criteria = ?" };
pub(super) const UPD_CHARACTER_SOCIAL_FLAGS: &str =
    { "UPDATE character_social SET flags = ? WHERE guid = ? AND friend = ?" };
pub(super) const INS_CHARACTER_SOCIAL: &str =
    { "INSERT INTO character_social (guid, friend, flags) VALUES (?, ?, ?)" };
pub(super) const DEL_CHARACTER_SOCIAL: &str =
    { "DELETE FROM character_social WHERE guid = ? AND friend = ?" };
pub(super) const UPD_CHARACTER_SOCIAL_NOTE: &str =
    { "UPDATE character_social SET note = ? WHERE guid = ? AND friend = ?" };
pub(super) const SEL_CHAR_SOCIAL: &str =
    "SELECT DISTINCT guid FROM character_social WHERE friend = ?";
pub(super) const SEL_GUILD_BANK_COUNT_ITEM: &str = {
    "SELECT COUNT(itemEntry) FROM guild_bank_item gbi INNER JOIN item_instance ii ON ii.guid = gbi.item_guid WHERE itemEntry = ?"
};
pub(super) const SEL_GUILD_BANK_ITEM_BY_ENTRY: &str = {
    "SELECT gi.item_guid, gi.guildid, g.name FROM guild_bank_item gi INNER JOIN guild g ON g.guildid = gi.guildid INNER JOIN item_instance ii ON ii.guid = gi.item_guid WHERE ii.itemEntry = ? LIMIT ?"
};
pub(super) const INS_PETITION: &str =
    { "INSERT INTO petition (ownerguid, petitionguid, name) VALUES (?, ?, ?)" };
pub(super) const DEL_PETITION_BY_GUID: &str = "DELETE FROM petition WHERE petitionguid = ?";
pub(super) const DEL_PETITION_SIGNATURE_BY_GUID: &str =
    { "DELETE FROM petition_sign WHERE petitionguid = ?" };
pub(super) const DEL_CHAR_SOCIAL_BY_GUID: &str = "DELETE FROM character_social WHERE guid = ?";
pub(super) const DEL_CHAR_SOCIAL_BY_FRIEND: &str = "DELETE FROM character_social WHERE friend = ?";
pub(super) const DEL_GUILD_EVENTLOG_BY_PLAYER: &str =
    { "DELETE FROM guild_eventlog WHERE PlayerGuid1 = ? OR PlayerGuid2 = ?" };
pub(super) const DEL_GUILD_BANK_EVENTLOG_BY_PLAYER: &str =
    { "DELETE FROM guild_bank_eventlog WHERE PlayerGuid = ?" };
pub(super) const DEL_PETITION_BY_OWNER: &str = "DELETE FROM petition WHERE ownerguid = ?";
pub(super) const DEL_PETITION_SIGNATURE_BY_OWNER: &str =
    { "DELETE FROM petition_sign WHERE ownerguid = ?" };
pub(super) const REP_CALENDAR_EVENT: &str = {
    "REPLACE INTO calendar_events (EventID, Owner, Title, Description, EventType, TextureID, Date, Flags, LockDate) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
};
pub(super) const DEL_CALENDAR_EVENT: &str = "DELETE FROM calendar_events WHERE EventID = ?";
pub(super) const REP_CALENDAR_INVITE: &str = {
    "REPLACE INTO calendar_invites (InviteID, EventID, Invitee, Sender, Status, ResponseTime, ModerationRank, Note) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
};
pub(super) const DEL_CALENDAR_INVITE: &str = "DELETE FROM calendar_invites WHERE InviteID = ?";
pub(super) const SEL_GUILD_BANK_ITEMS: &str = {
    "SELECT ii.guid, ii.itemEntry, ii.creatorGuid, ii.giftCreatorGuid, ii.count, ii.duration, ii.charges, ii.flags, ii.enchantments, ii.durability, ii.playedTime, ii.text, ii.battlePetSpeciesId, ii.battlePetBreedData, ii.battlePetLevel, ii.battlePetDisplayId, ii.randomPropertiesId, ii.randomPropertiesSeed, ii.context, iit.itemModifiedAppearanceAllSpecs, iit.itemModifiedAppearanceSpec1, iit.itemModifiedAppearanceSpec2, iit.itemModifiedAppearanceSpec3, iit.itemModifiedAppearanceSpec4, iit.itemModifiedAppearanceSpec5, iit.spellItemEnchantmentAllSpecs, iit.spellItemEnchantmentSpec1, iit.spellItemEnchantmentSpec2, iit.spellItemEnchantmentSpec3, iit.spellItemEnchantmentSpec4, iit.spellItemEnchantmentSpec5, iit.secondaryItemModifiedAppearanceAllSpecs, iit.secondaryItemModifiedAppearanceSpec1, iit.secondaryItemModifiedAppearanceSpec2, iit.secondaryItemModifiedAppearanceSpec3, iit.secondaryItemModifiedAppearanceSpec4, iit.itemModifiedAppearanceSpec5, ig.gemItemId1, ig.gemBonuses1, ig.gemContext1, ig.gemItemId2, ig.gemBonuses2, ig.gemContext2, ig.gemItemId3, ig.gemBonuses3, ig.gemContext3, guildid, TabId, SlotId FROM guild_bank_item gbi INNER JOIN item_instance ii ON gbi.item_guid = ii.guid LEFT JOIN item_instance_gems ig ON ii.guid = ig.itemGuid LEFT JOIN item_instance_transmog iit ON ii.guid = iit.itemGuid"
};
pub(super) const DEL_INVALID_GUILD_BANK_ITEM_GUIDS: &str =
    { "DELETE FROM guild_bank_item WHERE item_guid >= ?" };
pub(super) const INS_GUILD: &str = {
    "INSERT INTO guild (guildid, name, leaderguid, info, motd, createdate, EmblemStyle, EmblemColor, BorderStyle, BorderColor, BackgroundColor, BankMoney) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
};
pub(super) const DEL_GUILD: &str = "DELETE FROM guild WHERE guildid = ?";
pub(super) const UPD_GUILD_NAME: &str = "UPDATE guild SET name = ? WHERE guildid = ?";
pub(super) const INS_GUILD_MEMBER: &str =
    { "INSERT INTO guild_member (guildid, guid, `rank`, pnote, offnote) VALUES (?, ?, ?, ?, ?)" };
pub(super) const DEL_GUILD_MEMBER: &str = "DELETE FROM guild_member WHERE guid = ?";
pub(super) const DEL_GUILD_MEMBERS: &str = "DELETE FROM guild_member WHERE guildid = ?";
pub(super) const INS_GUILD_RANK: &str = {
    "INSERT INTO guild_rank (guildid, rid, RankOrder, rname, rights, BankMoneyPerDay) VALUES (?, ?, ?, ?, ?, ?)"
};
pub(super) const DEL_GUILD_RANKS: &str = "DELETE FROM guild_rank WHERE guildid = ?";
pub(super) const DEL_GUILD_RANK: &str = "DELETE FROM guild_rank WHERE guildid = ? AND rid = ?";
pub(super) const INS_GUILD_BANK_TAB: &str =
    "INSERT INTO guild_bank_tab (guildid, TabId) VALUES (?, ?)";
pub(super) const DEL_GUILD_BANK_TAB: &str =
    { "DELETE FROM guild_bank_tab WHERE guildid = ? AND TabId = ?" };
pub(super) const DEL_GUILD_BANK_TABS: &str = "DELETE FROM guild_bank_tab WHERE guildid = ?";
pub(super) const INS_GUILD_BANK_ITEM: &str =
    { "INSERT INTO guild_bank_item (guildid, TabId, SlotId, item_guid) VALUES (?, ?, ?, ?)" };
pub(super) const DEL_GUILD_BANK_ITEM: &str =
    { "DELETE FROM guild_bank_item WHERE guildid = ? AND TabId = ? AND SlotId = ?" };
pub(super) const DEL_GUILD_BANK_ITEMS: &str = "DELETE FROM guild_bank_item WHERE guildid = ?";
pub(super) const INS_GUILD_BANK_RIGHT: &str = {
    "INSERT INTO guild_bank_right (guildid, TabId, rid, gbright, SlotPerDay) VALUES (?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE gbright = VALUES(gbright), SlotPerDay = VALUES(SlotPerDay)"
};
pub(super) const DEL_GUILD_BANK_RIGHTS: &str = "DELETE FROM guild_bank_right WHERE guildid = ?";
pub(super) const DEL_GUILD_BANK_RIGHTS_FOR_RANK: &str =
    { "DELETE FROM guild_bank_right WHERE guildid = ? AND rid = ?" };
pub(super) const INS_GUILD_BANK_EVENTLOG: &str = {
    "INSERT INTO guild_bank_eventlog (guildid, LogGuid, TabId, EventType, PlayerGuid, ItemOrMoney, ItemStackCount, DestTabId, TimeStamp) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
};
pub(super) const DEL_GUILD_BANK_EVENTLOG: &str =
    { "DELETE FROM guild_bank_eventlog WHERE guildid = ? AND LogGuid = ? AND TabId = ?" };
pub(super) const DEL_GUILD_BANK_EVENTLOGS: &str =
    "DELETE FROM guild_bank_eventlog WHERE guildid = ?";
pub(super) const INS_GUILD_EVENTLOG: &str = {
    "INSERT INTO guild_eventlog (guildid, LogGuid, EventType, PlayerGuid1, PlayerGuid2, NewRank, TimeStamp) VALUES (?, ?, ?, ?, ?, ?, ?)"
};
pub(super) const DEL_GUILD_EVENTLOG: &str =
    { "DELETE FROM guild_eventlog WHERE guildid = ? AND LogGuid = ?" };
pub(super) const DEL_GUILD_EVENTLOGS: &str = "DELETE FROM guild_eventlog WHERE guildid = ?";
pub(super) const UPD_GUILD_MEMBER_PNOTE: &str = "UPDATE guild_member SET pnote = ? WHERE guid = ?";
pub(super) const UPD_GUILD_MEMBER_OFFNOTE: &str =
    "UPDATE guild_member SET offnote = ? WHERE guid = ?";
pub(super) const UPD_GUILD_MEMBER_RANK: &str = "UPDATE guild_member SET `rank` = ? WHERE guid = ?";
pub(super) const UPD_GUILD_MOTD: &str = "UPDATE guild SET motd = ? WHERE guildid = ?";
pub(super) const UPD_GUILD_INFO: &str = "UPDATE guild SET info = ? WHERE guildid = ?";
pub(super) const UPD_GUILD_LEADER: &str = "UPDATE guild SET leaderguid = ? WHERE guildid = ?";
pub(super) const UPD_GUILD_RANK_ORDER: &str =
    { "UPDATE guild_rank SET RankOrder = ? WHERE rid = ? AND guildid = ?" };
pub(super) const UPD_GUILD_RANK_NAME: &str =
    { "UPDATE guild_rank SET rname = ? WHERE rid = ? AND guildid = ?" };
pub(super) const UPD_GUILD_RANK_RIGHTS: &str =
    { "UPDATE guild_rank SET rights = ? WHERE rid = ? AND guildid = ?" };
pub(super) const UPD_GUILD_EMBLEM_INFO: &str = {
    "UPDATE guild SET EmblemStyle = ?, EmblemColor = ?, BorderStyle = ?, BorderColor = ?, BackgroundColor = ? WHERE guildid = ?"
};
pub(super) const UPD_GUILD_BANK_TAB_INFO: &str =
    { "UPDATE guild_bank_tab SET TabName = ?, TabIcon = ? WHERE guildid = ? AND TabId = ?" };
pub(super) const UPD_GUILD_BANK_MONEY: &str = "UPDATE guild SET BankMoney = ? WHERE guildid = ?";
pub(super) const UPD_GUILD_RANK_BANK_MONEY: &str =
    { "UPDATE guild_rank SET BankMoneyPerDay = ? WHERE rid = ? AND guildid = ?" };
pub(super) const UPD_GUILD_BANK_TAB_TEXT: &str =
    { "UPDATE guild_bank_tab SET TabText = ? WHERE guildid = ? AND TabId = ?" };
pub(super) const INS_GUILD_MEMBER_WITHDRAW_TABS: &str = {
    "INSERT INTO guild_member_withdraw (guid, tab0, tab1, tab2, tab3, tab4, tab5, tab6, tab7) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE tab0 = VALUES (tab0), tab1 = VALUES (tab1), tab2 = VALUES (tab2), tab3 = VALUES (tab3), tab4 = VALUES (tab4), tab5 = VALUES (tab5), tab6 = VALUES (tab6), tab7 = VALUES (tab7)"
};
pub(super) const INS_GUILD_MEMBER_WITHDRAW_MONEY: &str = {
    "INSERT INTO guild_member_withdraw (guid, money) VALUES (?, ?) ON DUPLICATE KEY UPDATE money = VALUES (money)"
};
pub(super) const DEL_GUILD_MEMBER_WITHDRAW: &str = "DELETE FROM guild_member_withdraw";
pub(super) const SEL_CHAR_DATA_FOR_GUILD: &str =
    { "SELECT name, level, race, class, gender, zone, account FROM characters WHERE guid = ?" };
pub(super) const DEL_GUILD_ACHIEVEMENT: &str =
    { "DELETE FROM guild_achievement WHERE guildId = ? AND achievement = ?" };
pub(super) const INS_GUILD_ACHIEVEMENT: &str =
    { "INSERT INTO guild_achievement (guildId, achievement, date, guids) VALUES (?, ?, ?, ?)" };
pub(super) const DEL_GUILD_ACHIEVEMENT_CRITERIA: &str =
    { "DELETE FROM guild_achievement_progress WHERE guildId = ? AND criteria = ?" };
pub(super) const INS_GUILD_ACHIEVEMENT_CRITERIA: &str = {
    "INSERT INTO guild_achievement_progress (guildId, criteria, counter, date, completedGuid) VALUES (?, ?, ?, ?, ?)"
};
pub(super) const DEL_ALL_GUILD_ACHIEVEMENTS: &str = {
    "DELETE FROM guild_achievement WHERE guildId = ? AND achievement NOT IN (5407,5408,5409,5410,5411,5985,6126,6628,6678,6679,6680,8257,8512,8513,9397,9399,10380)"
};
pub(super) const DEL_ALL_GUILD_ACHIEVEMENT_CRITERIA: &str =
    { "DELETE FROM guild_achievement_progress WHERE guildId = ?" };
pub(super) const SEL_GUILD_ACHIEVEMENT: &str =
    { "SELECT achievement, date, guids FROM guild_achievement WHERE guildId = ?" };
pub(super) const SEL_GUILD_ACHIEVEMENT_CRITERIA: &str = {
    "SELECT criteria, counter, date, completedGuid FROM guild_achievement_progress WHERE guildId = ?"
};
pub(super) const INS_GUILD_NEWS: &str = {
    "INSERT INTO guild_newslog (guildid, LogGuid, EventType, PlayerGuid, Flags, Value, Timestamp) VALUES (?, ?, ?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE LogGuid = VALUES (LogGuid), EventType = VALUES (EventType), PlayerGuid = VALUES (PlayerGuid), Flags = VALUES (Flags), Value = VALUES (Value), Timestamp = VALUES (Timestamp)"
};
pub(super) const UPD_CHANNEL: &str = {
    "INSERT INTO channels (name, team, announce, ownership, password, bannedList, lastUsed) VALUES (?, ?, ?, ?, ?, ?, UNIX_TIMESTAMP()) ON DUPLICATE KEY UPDATE announce=VALUES(announce), ownership=VALUES(ownership), password=VALUES(password), bannedList=VALUES(bannedList), lastUsed=VALUES(lastUsed)"
};
pub(super) const UPD_CHANNEL_USAGE: &str =
    { "UPDATE channels SET lastUsed = UNIX_TIMESTAMP() WHERE name = ? AND team = ?" };
pub(super) const UPD_CHANNEL_OWNERSHIP: &str =
    "UPDATE channels SET ownership = ? WHERE name LIKE ?";
pub(super) const DEL_CHANNEL: &str = "DELETE FROM channels WHERE name = ? AND team = ?";
pub(super) const DEL_OLD_CHANNELS: &str =
    { "DELETE FROM channels WHERE ownership = 1 AND lastUsed + ? < UNIX_TIMESTAMP()" };
pub(super) const SEL_PETITION: &str = "SELECT ownerguid, name FROM petition WHERE petitionguid = ?";
pub(super) const SEL_PETITION_SIGNATURE: &str =
    { "SELECT playerguid FROM petition_sign WHERE petitionguid = ?" };
pub(super) const DEL_ALL_PETITION_SIGNATURES: &str =
    "DELETE FROM petition_sign WHERE playerguid = ?";
pub(super) const SEL_PETITION_BY_OWNER: &str =
    "SELECT petitionguid FROM petition WHERE ownerguid = ?";
pub(super) const SEL_PETITION_SIGNATURES: &str = {
    "SELECT ownerguid, (SELECT COUNT(playerguid) FROM petition_sign WHERE petition_sign.petitionguid = ?) AS signs FROM petition WHERE petitionguid = ?"
};
pub(super) const SEL_PETITION_SIG_BY_ACCOUNT: &str =
    { "SELECT playerguid FROM petition_sign WHERE player_account = ? AND petitionguid = ?" };
pub(super) const SEL_PETITION_OWNER_BY_GUID: &str =
    { "SELECT ownerguid FROM petition WHERE petitionguid = ?" };
pub(super) const SEL_PETITION_SIG_BY_GUID: &str =
    { "SELECT ownerguid, petitionguid FROM petition_sign WHERE playerguid = ?" };
