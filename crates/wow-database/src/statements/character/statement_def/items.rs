//! Character item, inventory, mail, and auction SQL values.
pub(super) const SEL_MAIL_LIST_COUNT: &str = "SELECT COUNT(id) FROM mail WHERE receiver = ? ";
pub(super) const SEL_MAIL_LIST_INFO: &str = {
    "SELECT id, sender, (SELECT name FROM characters WHERE guid = sender) AS sendername, receiver, (SELECT name FROM characters WHERE guid = receiver) AS receivername, subject, deliver_time, expire_time, money, has_items FROM mail WHERE receiver = ? "
};
pub(super) const SEL_MAIL_LIST_ITEMS: &str =
    "SELECT itemEntry,count FROM item_instance WHERE guid = ?";
pub(super) const SEL_CHAR_EQUIPMENT: &str = {
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
};
pub(super) const UPD_CHAR_INVENTORY_SLOT: &str =
    { "UPDATE character_inventory SET slot = ? WHERE guid = ? AND item = ?" };
pub(super) const DEL_CHAR_INVENTORY_ITEM: &str =
    { "DELETE FROM character_inventory WHERE guid = ? AND item = ?" };
pub(super) const DEL_CHAR_INVENTORY_ITEM_BY_OWNER: &str = {
    "DELETE ci FROM character_inventory ci INNER JOIN item_instance ii ON ii.guid = ci.item WHERE ci.guid = ? AND ci.item = ? AND ii.owner_guid = ?"
};
pub(super) const SEL_MAIL_COUNT: &str = "SELECT COUNT(*) FROM mail WHERE receiver = ?";
pub(super) const SEL_CHARACTER_EQUIPMENTSETS: &str = {
    "SELECT setguid, setindex, name, iconname, ignore_mask, AssignedSpecIndex, item0, item1, item2, item3, item4, item5, item6, item7, item8, item9, item10, item11, item12, item13, item14, item15, item16, item17, item18 FROM character_equipmentsets WHERE guid = ? ORDER BY setindex"
};
pub(super) const SEL_CHARACTER_TRANSMOG_OUTFITS: &str = {
    "SELECT setguid, setindex, name, iconname, ignore_mask, appearance0, appearance1, appearance2, appearance3, appearance4, appearance5, appearance6, appearance7, appearance8, appearance9, appearance10, appearance11, appearance12, appearance13, appearance14, appearance15, appearance16, appearance17, appearance18, mainHandEnchant, offHandEnchant FROM character_transmog_outfits WHERE guid = ? ORDER BY setindex"
};
pub(super) const SEL_CHARACTER_FAVORITE_AUCTIONS: &str = {
    "SELECT `order`, itemId, itemLevel, battlePetSpeciesId, suffixItemNameDescriptionId FROM character_favorite_auctions WHERE guid = ? ORDER BY `order`"
};
pub(super) const INS_CHARACTER_FAVORITE_AUCTION: &str = {
    "INSERT INTO character_favorite_auctions (guid, `order`, itemId, itemLevel, battlePetSpeciesId, suffixItemNameDescriptionId) VALUE (?, ?, ?, ?, ?, ?)"
};
pub(super) const DEL_CHARACTER_FAVORITE_AUCTION: &str =
    { "DELETE FROM character_favorite_auctions WHERE guid = ? AND `order` = ?" };
pub(super) const DEL_CHARACTER_FAVORITE_AUCTIONS_BY_CHAR: &str =
    { "DELETE FROM character_favorite_auctions WHERE guid = ?" };
pub(super) const SEL_AUCTIONS: &str = {
    "SELECT id, auctionHouseId, owner, bidder, minBid, buyoutOrUnitPrice, deposit, bidAmount, startTime, endTime, serverFlags FROM auctionhouse"
};
pub(super) const INS_AUCTION_ITEMS: &str =
    { "INSERT INTO auction_items (auctionId, itemGuid) VALUES (?, ?)" };
pub(super) const DEL_AUCTION_ITEMS_BY_ITEM: &str = "DELETE FROM auction_items WHERE itemGuid = ?";
pub(super) const SEL_AUCTION_BIDDERS: &str = "SELECT auctionId, playerGuid FROM auction_bidders";
pub(super) const INS_AUCTION_BIDDER: &str =
    { "INSERT INTO auction_bidders (auctionId, playerGuid) VALUES (?, ?)" };
pub(super) const DEL_AUCTION_BIDDER_BY_PLAYER: &str =
    { "DELETE FROM auction_bidders WHERE playerGuid = ?" };
pub(super) const INS_AUCTION: &str = {
    "INSERT INTO auctionhouse (id, auctionHouseId, owner, bidder, minBid, buyoutOrUnitPrice, deposit, bidAmount, startTime, endTime, serverFlags) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
};
pub(super) const DEL_AUCTION: &str = {
    "DELETE a, ab, ai FROM auctionhouse a LEFT JOIN auction_items ai ON a.id = ai.auctionId LEFT JOIN auction_bidders ab ON a.id = ab.auctionId WHERE a.id = ?"
};
pub(super) const UPD_AUCTION_BID: &str =
    { "UPDATE auctionhouse SET bidder = ?, bidAmount = ?, serverFlags = ? WHERE id = ?" };
pub(super) const UPD_AUCTION_EXPIRATION: &str = "UPDATE auctionhouse SET endTime = ? WHERE id = ?";
pub(super) const INS_MAIL: &str = {
    "INSERT INTO mail(id, messageType, stationery, mailTemplateId, sender, receiver, subject, body, has_items, expire_time, deliver_time, money, cod, checked) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
};
pub(super) const DEL_MAIL_BY_ID: &str = "DELETE FROM mail WHERE id = ?";
pub(super) const INS_MAIL_ITEM: &str =
    { "INSERT INTO mail_items(mail_id, item_guid, receiver) VALUES (?, ?, ?)" };
pub(super) const DEL_MAIL_ITEM: &str = "DELETE FROM mail_items WHERE item_guid = ?";
pub(super) const DEL_INVALID_MAIL_ITEM: &str = "DELETE FROM mail_items WHERE item_guid = ?";
pub(super) const DEL_EMPTY_EXPIRED_MAIL: &str =
    { "DELETE FROM mail WHERE expire_time < ? AND has_items = 0 AND body = ''" };
pub(super) const SEL_EXPIRED_MAIL: &str = {
    "SELECT id, messageType, sender, receiver, has_items, expire_time, cod, checked, mailTemplateId FROM mail WHERE expire_time < ?"
};
pub(super) const SEL_EXPIRED_MAIL_ITEMS: &str = {
    "SELECT item_guid, itemEntry, mail_id FROM mail_items mi INNER JOIN item_instance ii ON ii.guid = mi.item_guid LEFT JOIN mail mm ON mi.mail_id = mm.id WHERE mm.id IS NOT NULL AND mm.expire_time < ?"
};
pub(super) const UPD_MAIL_RETURNED: &str = {
    "UPDATE mail SET sender = ?, receiver = ?, expire_time = ?, deliver_time = ?, cod = 0, checked = ? WHERE id = ?"
};
pub(super) const UPD_MAIL_ITEM_RECEIVER: &str =
    { "UPDATE mail_items SET receiver = ? WHERE item_guid = ?" };
pub(super) const UPD_ITEM_OWNER: &str = "UPDATE item_instance SET owner_guid = ? WHERE guid = ?";
pub(super) const SEL_PINFO_MAILS: &str = {
    "SELECT SUM(CASE WHEN (checked & 1) THEN 1 ELSE 0 END) AS 'readmail', COUNT(*) AS 'totalmail' FROM mail WHERE `receiver` = ?"
};
pub(super) const SEL_CHAR_COD_ITEM_MAIL: &str = {
    "SELECT id, messageType, mailTemplateId, sender, subject, body, money, has_items FROM mail WHERE receiver = ? AND has_items <> 0 AND cod <> 0"
};
pub(super) const SEL_MAIL: &str = {
    "SELECT id, messageType, sender, receiver, subject, body, expire_time, deliver_time, money, cod, checked, stationery, mailTemplateId FROM mail WHERE receiver = ? ORDER BY id DESC"
};
pub(super) const SEL_CHAR_INVENTORY_COUNT_ITEM: &str = {
    "SELECT COUNT(itemEntry) FROM character_inventory ci INNER JOIN item_instance ii ON ii.guid = ci.item WHERE itemEntry = ?"
};
pub(super) const SEL_MAIL_COUNT_ITEM: &str = {
    "SELECT COUNT(itemEntry) FROM mail_items mi INNER JOIN item_instance ii ON ii.guid = mi.item_guid WHERE itemEntry = ?"
};
pub(super) const SEL_AUCTIONHOUSE_COUNT_ITEM: &str = {
    "SELECT COUNT(*) FROM auction_items ai INNER JOIN item_instance ii ON ii.guid = ai.itemGuid WHERE ii.itemEntry = ?"
};
pub(super) const SEL_CHAR_INVENTORY_ITEM_BY_ENTRY: &str = {
    "SELECT ci.item, cb.slot AS bag, ci.slot, ci.guid, c.account, c.name FROM characters c INNER JOIN character_inventory ci ON ci.guid = c.guid INNER JOIN item_instance ii ON ii.guid = ci.item LEFT JOIN character_inventory cb ON cb.item = ci.bag WHERE ii.itemEntry = ? LIMIT ?"
};
pub(super) const SEL_MAIL_ITEMS_BY_ENTRY: &str = {
    "SELECT mi.item_guid, m.sender, m.receiver, cs.account, cs.name, cr.account, cr.name FROM mail m INNER JOIN mail_items mi ON mi.mail_id = m.id INNER JOIN item_instance ii ON ii.guid = mi.item_guid INNER JOIN characters cs ON cs.guid = m.sender INNER JOIN characters cr ON cr.guid = m.receiver WHERE ii.itemEntry = ? LIMIT ?"
};
pub(super) const SEL_AUCTIONHOUSE_ITEM_BY_ENTRY: &str = {
    "SELECT ai.itemGuid, c.guid, c.account, c.name FROM auctionhouse ah INNER JOIN auction_items ai ON ah.id = ai.auctionId INNER JOIN characters c ON c.guid = ah.owner INNER JOIN item_instance ii ON ii.guid = ai.itemGuid WHERE ii.itemEntry = ? LIMIT ?"
};
pub(super) const INS_CHAR_GIFT: &str =
    { "INSERT INTO character_gifts (guid, item_guid, entry, flags) VALUES (?, ?, ?, ?)" };
pub(super) const DEL_MAIL_ITEM_BY_ID: &str = "DELETE FROM mail_items WHERE mail_id = ?";
pub(super) const UPD_CHAR_INVENTORY_FACTION_CHANGE: &str = {
    "UPDATE item_instance ii, character_inventory ci SET ii.itemEntry = ? WHERE ii.itemEntry = ? AND ci.guid = ? AND ci.item = ii.guid"
};
pub(super) const DEL_CHAR_GIFT: &str = "DELETE FROM character_gifts WHERE guid = ?";
pub(super) const DEL_CHAR_INVENTORY: &str = "DELETE FROM character_inventory WHERE guid = ?";
pub(super) const DEL_MAIL: &str = "DELETE FROM mail WHERE receiver = ?";
pub(super) const DEL_MAIL_ITEMS: &str = "DELETE FROM mail_items WHERE receiver = ?";
pub(super) const DEL_CHAR_EQUIPMENTSETS: &str =
    "DELETE FROM character_equipmentsets WHERE guid = ?";
pub(super) const DEL_CHAR_TRANSMOG_OUTFITS: &str =
    { "DELETE FROM character_transmog_outfits WHERE guid = ?" };
pub(super) const DEL_CHAR_INVENTORY_BY_ITEM: &str =
    "DELETE FROM character_inventory WHERE item = ?";
pub(super) const DEL_CHAR_INVENTORY_BY_BAG_SLOT: &str =
    { "DELETE FROM character_inventory WHERE bag = ? AND slot = ? AND guid = ?" };
pub(super) const UPD_MAIL: &str = {
    "UPDATE mail SET has_items = ?, expire_time = ?, deliver_time = ?, money = ?, cod = ?, checked = ? WHERE id = ?"
};
pub(super) const SEL_CHAR_VOID_STORAGE: &str = {
    "SELECT itemId, itemEntry, slot, creatorGuid, fixedScalingLevel, randomPropertiesId, randomPropertiesSeed, context FROM character_void_storage WHERE playerGuid = ?"
};
pub(super) const REP_CHAR_VOID_STORAGE_ITEM: &str = {
    "REPLACE INTO character_void_storage (itemId, playerGuid, itemEntry, slot, creatorGuid, fixedScalingLevel, randomPropertiesId, randomPropertiesSeed, context) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
};
pub(super) const DEL_CHAR_VOID_STORAGE_ITEM_BY_CHAR_GUID: &str =
    { "DELETE FROM character_void_storage WHERE playerGuid = ?" };
pub(super) const DEL_CHAR_VOID_STORAGE_ITEM_BY_SLOT: &str =
    { "DELETE FROM character_void_storage WHERE slot = ? AND playerGuid = ?" };
pub(super) const SEL_CHARACTER_INVENTORY: &str = {
    "SELECT ii.guid, ii.itemEntry, ii.creatorGuid, ii.giftCreatorGuid, ii.count, ii.duration, ii.charges, ii.flags, ii.enchantments, ii.durability, ii.playedTime, ii.text, ii.battlePetSpeciesId, ii.battlePetBreedData, ii.battlePetLevel, ii.battlePetDisplayId, ii.randomPropertiesId, ii.randomPropertiesSeed, ii.context, iit.itemModifiedAppearanceAllSpecs, iit.itemModifiedAppearanceSpec1, iit.itemModifiedAppearanceSpec2, iit.itemModifiedAppearanceSpec3, iit.itemModifiedAppearanceSpec4, iit.itemModifiedAppearanceSpec5, iit.spellItemEnchantmentAllSpecs, iit.spellItemEnchantmentSpec1, iit.spellItemEnchantmentSpec2, iit.spellItemEnchantmentSpec3, iit.spellItemEnchantmentSpec4, iit.spellItemEnchantmentSpec5, iit.secondaryItemModifiedAppearanceAllSpecs, iit.secondaryItemModifiedAppearanceSpec1, iit.secondaryItemModifiedAppearanceSpec2, iit.secondaryItemModifiedAppearanceSpec3, iit.secondaryItemModifiedAppearanceSpec4, iit.itemModifiedAppearanceSpec5, ig.gemItemId1, ig.gemBonuses1, ig.gemContext1, ig.gemItemId2, ig.gemBonuses2, ig.gemContext2, ig.gemItemId3, ig.gemBonuses3, ig.gemContext3, bag, slot FROM character_inventory ci JOIN item_instance ii ON ci.item = ii.guid LEFT JOIN item_instance_gems ig ON ii.guid = ig.itemGuid LEFT JOIN item_instance_transmog iit ON ii.guid = iit.itemGuid WHERE ci.guid = ? ORDER BY (ii.flags & 0x80000) ASC, bag ASC, slot ASC"
};
pub(super) const SEL_MAILITEMS: &str = {
    "SELECT ii.guid, ii.itemEntry, ii.creatorGuid, ii.giftCreatorGuid, ii.count, ii.duration, ii.charges, ii.flags, ii.enchantments, ii.durability, ii.playedTime, ii.text, ii.battlePetSpeciesId, ii.battlePetBreedData, ii.battlePetLevel, ii.battlePetDisplayId, ii.randomPropertiesId, ii.randomPropertiesSeed, ii.context, iit.itemModifiedAppearanceAllSpecs, iit.itemModifiedAppearanceSpec1, iit.itemModifiedAppearanceSpec2, iit.itemModifiedAppearanceSpec3, iit.itemModifiedAppearanceSpec4, iit.itemModifiedAppearanceSpec5, iit.spellItemEnchantmentAllSpecs, iit.spellItemEnchantmentSpec1, iit.spellItemEnchantmentSpec2, iit.spellItemEnchantmentSpec3, iit.spellItemEnchantmentSpec4, iit.spellItemEnchantmentSpec5, iit.secondaryItemModifiedAppearanceAllSpecs, iit.secondaryItemModifiedAppearanceSpec1, iit.secondaryItemModifiedAppearanceSpec2, iit.secondaryItemModifiedAppearanceSpec3, iit.secondaryItemModifiedAppearanceSpec4, iit.itemModifiedAppearanceSpec5, ig.gemItemId1, ig.gemBonuses1, ig.gemContext1, ig.gemItemId2, ig.gemBonuses2, ig.gemContext2, ig.gemItemId3, ig.gemBonuses3, ig.gemContext3, ii.owner_guid, m.id FROM mail_items mi INNER JOIN mail m ON mi.mail_id = m.id LEFT JOIN item_instance ii ON mi.item_guid = ii.guid LEFT JOIN item_instance_gems ig ON ii.guid = ig.itemGuid LEFT JOIN item_instance_transmog iit ON ii.guid = iit.itemGuid WHERE m.receiver = ?"
};
pub(super) const SEL_AUCTION_ITEMS: &str = {
    "SELECT ii.guid, ii.itemEntry, ii.creatorGuid, ii.giftCreatorGuid, ii.count, ii.duration, ii.charges, ii.flags, ii.enchantments, ii.durability, ii.playedTime, ii.text, ii.battlePetSpeciesId, ii.battlePetBreedData, ii.battlePetLevel, ii.battlePetDisplayId, ii.randomPropertiesId, ii.randomPropertiesSeed, ii.context, iit.itemModifiedAppearanceAllSpecs, iit.itemModifiedAppearanceSpec1, iit.itemModifiedAppearanceSpec2, iit.itemModifiedAppearanceSpec3, iit.itemModifiedAppearanceSpec4, iit.itemModifiedAppearanceSpec5, iit.spellItemEnchantmentAllSpecs, iit.spellItemEnchantmentSpec1, iit.spellItemEnchantmentSpec2, iit.spellItemEnchantmentSpec3, iit.spellItemEnchantmentSpec4, iit.spellItemEnchantmentSpec5, iit.secondaryItemModifiedAppearanceAllSpecs, iit.secondaryItemModifiedAppearanceSpec1, iit.secondaryItemModifiedAppearanceSpec2, iit.secondaryItemModifiedAppearanceSpec3, iit.secondaryItemModifiedAppearanceSpec4, iit.itemModifiedAppearanceSpec5, ig.gemItemId1, ig.gemBonuses1, ig.gemContext1, ig.gemItemId2, ig.gemBonuses2, ig.gemContext2, ig.gemItemId3, ig.gemBonuses3, ig.gemContext3, ii.owner_guid, ai.auctionId FROM auction_items ai INNER JOIN item_instance ii ON ai.itemGuid = ii.guid LEFT JOIN item_instance_gems ig ON ii.guid = ig.itemGuid LEFT JOIN item_instance_transmog iit ON ii.guid = iit.itemGuid"
};
pub(super) const SEL_MAX_ITEM_GUID: &str = "SELECT MAX(guid) FROM item_instance";
pub(super) const SEL_MAX_EQUIPMENT_SET_GUID: &str = {
    // The equipment table uses BIGINT UNSIGNED while the canonical
    // transmog table uses signed BIGINT. MariaDB can promote their
    // UNION/MAX result to DECIMAL; pin the wire type so startup can
    // decode the shared raw uint64 namespace without driver-specific
    // signed/decimal coercion.
    "SELECT CAST(MAX(maxguid) AS UNSIGNED) FROM ((SELECT MAX(setguid) AS maxguid FROM character_equipmentsets) UNION (SELECT MAX(setguid) AS maxguid FROM character_transmog_outfits)) allsets"
};
pub(super) const SEL_MAX_VOID_STORAGE_ITEM_ID: &str =
    "SELECT MAX(itemId) FROM character_void_storage";
pub(super) const DEL_INVALID_CHAR_INVENTORY_ITEM_GUIDS: &str =
    { "DELETE FROM character_inventory WHERE item >= ?" };
pub(super) const DEL_INVALID_MAIL_ITEM_GUIDS: &str = "DELETE FROM mail_items WHERE item_guid >= ?";
pub(super) const DEL_INVALID_AUCTION_ITEM_GUIDS: &str = {
    "DELETE a, ab, ai FROM auctionhouse a LEFT JOIN auction_bidders ab ON ab.auctionId = a.id LEFT JOIN auction_items ai ON ai.auctionId = a.id WHERE ai.itemGuid >= ?"
};
pub(super) const DEL_INVALID_ITEM_LOOT_ITEMS_GUIDS: &str =
    { "DELETE FROM item_loot_items WHERE container_id >= ?" };
pub(super) const DEL_INVALID_ITEM_LOOT_MONEY_GUIDS: &str =
    { "DELETE FROM item_loot_money WHERE container_id >= ?" };
pub(super) const INS_ITEM_INSTANCE: &str = {
    "INSERT INTO item_instance \
                 (guid, itemEntry, owner_guid, creatorGuid, giftCreatorGuid, count, \
                  durability, enchantments, charges, flags, randomPropertiesId, \
                  randomPropertiesSeed, context) \
                 VALUES (?, ?, ?, 0, 0, ?, ?, '', '', 0, 0, 0, 0)"
};
pub(super) const INS_ITEM_INSTANCE_WITH_RANDOM_CONTEXT: &str = {
    "INSERT INTO item_instance \
                 (guid, itemEntry, owner_guid, creatorGuid, giftCreatorGuid, count, \
                  durability, enchantments, charges, flags, randomPropertiesId, \
                  randomPropertiesSeed, context) \
                 VALUES (?, ?, ?, 0, 0, ?, ?, '', '', ?, ?, ?, ?)"
};
pub(super) const INS_ITEM_INSTANCE_CLONE: &str = {
    "INSERT INTO item_instance \
                 (guid, itemEntry, owner_guid, creatorGuid, giftCreatorGuid, count, \
                  duration, charges, enchantments, flags, durability, playedTime, \
                  randomPropertiesId, randomPropertiesSeed, context) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
};
pub(super) const UPD_ITEM_INSTANCE_COUNT: &str =
    "UPDATE item_instance SET count = ? WHERE guid = ?";
pub(super) const UPD_ITEM_INSTANCE_DURABILITY: &str =
    { "UPDATE item_instance SET durability = ? WHERE guid = ?" };
pub(super) const UPD_ITEM_INSTANCE_FLAGS: &str =
    "UPDATE item_instance SET flags = ? WHERE guid = ?";
pub(super) const UPD_ITEM_INSTANCE_ENCHANTMENTS: &str =
    { "UPDATE item_instance SET enchantments = ? WHERE guid = ?" };
pub(super) const UPD_ITEM_INSTANCE_STORAGE_MUTABLE: &str = {
    "UPDATE item_instance SET count = ?, duration = ?, charges = ?, flags = ?, enchantments = ?, durability = ?, playedTime = ? WHERE guid = ?"
};
pub(super) const SEL_CHARACTER_GIFT_BY_ITEM: &str =
    { "SELECT entry, flags FROM character_gifts WHERE item_guid = ?" };
pub(super) const DEL_GIFT: &str = "DELETE FROM character_gifts WHERE item_guid = ?";
pub(super) const UPD_ITEM_INSTANCE_OPEN_GIFT: &str = {
    "UPDATE item_instance SET itemEntry = ?, giftCreatorGuid = 0, flags = ?, durability = ? WHERE guid = ?"
};
pub(super) const INS_CHAR_INVENTORY: &str =
    { "INSERT INTO character_inventory (guid, bag, slot, item) VALUES (?, 0, ?, ?)" };
pub(super) const REP_CHAR_INVENTORY_ITEM: &str =
    { "REPLACE INTO character_inventory (guid, bag, slot, item) VALUES (?, ?, ?, ?)" };
pub(super) const DEL_ITEM_INSTANCE: &str = "DELETE FROM item_instance WHERE guid = ?";
pub(super) const DEL_ITEM_INSTANCE_BY_GUID_AND_OWNER: &str =
    { "DELETE FROM item_instance WHERE guid = ? AND owner_guid = ?" };
pub(super) const SEL_UNCAGE_ITEM_STATE: &str = {
    "SELECT (SELECT owner_guid FROM item_instance WHERE guid = ? LIMIT 1), EXISTS(SELECT 1 FROM character_inventory WHERE guid = ? AND item = ?)"
};
pub(super) const UPD_CHARACTER_MONEY_REFUND: &str =
    { "UPDATE characters SET money = LEAST(money + ?, ?) WHERE guid = ?" };
pub(super) const SEL_ITEM_REFUNDS: &str = {
    "SELECT paidMoney, paidExtendedCost \
                 FROM item_refund_instance WHERE item_guid = ? AND player_guid = ? LIMIT 1"
};
pub(super) const SEL_ITEM_BOP_TRADE: &str =
    { "SELECT allowedPlayers FROM item_soulbound_trade_data WHERE itemGuid = ? LIMIT 1" };
pub(super) const DEL_ITEM_BOP_TRADE: &str =
    { "DELETE FROM item_soulbound_trade_data WHERE itemGuid = ? LIMIT 1" };
pub(super) const INS_ITEM_BOP_TRADE: &str = "INSERT INTO item_soulbound_trade_data VALUES (?, ?)";
pub(super) const REP_INVENTORY_ITEM: &str =
    { "REPLACE INTO character_inventory (guid, bag, slot, item) VALUES (?, ?, ?, ?)" };
pub(super) const REP_ITEM_INSTANCE: &str = {
    "REPLACE INTO item_instance (itemEntry, owner_guid, creatorGuid, giftCreatorGuid, count, duration, charges, flags, enchantments, durability, playedTime, text, battlePetSpeciesId, battlePetBreedData, battlePetLevel, battlePetDisplayId, randomPropertiesId, randomPropertiesSeed, context, guid) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
};
pub(super) const UPD_ITEM_INSTANCE: &str = {
    "UPDATE item_instance SET itemEntry = ?, owner_guid = ?, creatorGuid = ?, giftCreatorGuid = ?, count = ?, duration = ?, charges = ?, flags = ?, enchantments = ?, durability = ?, playedTime = ?, text = ?, battlePetSpeciesId = ?, battlePetBreedData = ?, battlePetLevel = ?, battlePetDisplayId = ?, randomPropertiesId = ?, randomPropertiesSeed = ?, context = ? WHERE guid = ?"
};
pub(super) const UPD_ITEM_INSTANCE_ON_LOAD: &str =
    { "UPDATE item_instance SET duration = ?, flags = ?, durability = ? WHERE guid = ?" };
pub(super) const DEL_ITEM_INSTANCE_BY_OWNER: &str =
    "DELETE FROM item_instance WHERE owner_guid = ?";
pub(super) const INS_ITEM_INSTANCE_GEMS: &str = {
    "INSERT INTO item_instance_gems (itemGuid, gemItemId1, gemBonuses1, gemContext1, gemItemId2, gemBonuses2, gemContext2, gemItemId3, gemBonuses3, gemContext3) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
};
pub(super) const DEL_ITEM_INSTANCE_GEMS: &str = "DELETE FROM item_instance_gems WHERE itemGuid = ?";
pub(super) const DEL_ITEM_INSTANCE_GEMS_BY_OWNER: &str = {
    "DELETE iig FROM item_instance_gems iig LEFT JOIN item_instance ii ON iig.itemGuid = ii.guid WHERE ii.owner_guid = ?"
};
pub(super) const INS_ITEM_INSTANCE_TRANSMOG: &str = {
    "INSERT INTO item_instance_transmog (itemGuid, itemModifiedAppearanceAllSpecs, itemModifiedAppearanceSpec1, itemModifiedAppearanceSpec2, itemModifiedAppearanceSpec3, itemModifiedAppearanceSpec4, itemModifiedAppearanceSpec5, spellItemEnchantmentAllSpecs, spellItemEnchantmentSpec1, spellItemEnchantmentSpec2, spellItemEnchantmentSpec3, spellItemEnchantmentSpec4, spellItemEnchantmentSpec5, secondaryItemModifiedAppearanceAllSpecs, secondaryItemModifiedAppearanceSpec1, secondaryItemModifiedAppearanceSpec2, secondaryItemModifiedAppearanceSpec3, secondaryItemModifiedAppearanceSpec4, secondaryItemModifiedAppearanceSpec5) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
};
pub(super) const DEL_ITEM_INSTANCE_TRANSMOG: &str =
    { "DELETE FROM item_instance_transmog WHERE itemGuid = ?" };
pub(super) const DEL_ITEM_INSTANCE_TRANSMOG_BY_OWNER: &str = {
    "DELETE iit FROM item_instance_transmog iit LEFT JOIN item_instance ii ON iit.itemGuid = ii.guid WHERE ii.owner_guid = ?"
};
pub(super) const UPD_GIFT_OWNER: &str = "UPDATE character_gifts SET guid = ? WHERE item_guid = ?";
pub(super) const UPD_EQUIP_SET: &str = {
    "UPDATE character_equipmentsets SET name=?, iconname=?, ignore_mask=?, AssignedSpecIndex=?, item0=?, item1=?, item2=?, item3=?, item4=?, item5=?, item6=?, item7=?, item8=?, item9=?, item10=?, item11=?, item12=?, item13=?, item14=?, item15=?, item16=?, item17=?, item18=? WHERE guid=? AND setguid=? AND setindex=?"
};
pub(super) const INS_EQUIP_SET: &str = {
    "INSERT INTO character_equipmentsets (guid, setguid, setindex, name, iconname, ignore_mask, AssignedSpecIndex, item0, item1, item2, item3, item4, item5, item6, item7, item8, item9, item10, item11, item12, item13, item14, item15, item16, item17, item18) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
};
pub(super) const DEL_EQUIP_SET: &str = "DELETE FROM character_equipmentsets WHERE setguid=?";
pub(super) const UPD_TRANSMOG_OUTFIT: &str = {
    "UPDATE character_transmog_outfits SET name=?, iconname=?, ignore_mask=?, appearance0=?, appearance1=?, appearance2=?, appearance3=?, appearance4=?, appearance5=?, appearance6=?, appearance7=?, appearance8=?, appearance9=?, appearance10=?, appearance11=?, appearance12=?, appearance13=?, appearance14=?, appearance15=?, appearance16=?, appearance17=?, appearance18=?, mainHandEnchant=?, offHandEnchant=? WHERE guid=? AND setguid=? AND setindex=?"
};
pub(super) const INS_TRANSMOG_OUTFIT: &str = {
    "INSERT INTO character_transmog_outfits (guid, setguid, setindex, name, iconname, ignore_mask, appearance0, appearance1, appearance2, appearance3, appearance4, appearance5, appearance6, appearance7, appearance8, appearance9, appearance10, appearance11, appearance12, appearance13, appearance14, appearance15, appearance16, appearance17, appearance18, mainHandEnchant, offHandEnchant) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
};
pub(super) const DEL_TRANSMOG_OUTFIT: &str =
    "DELETE FROM character_transmog_outfits WHERE setguid=?";
pub(super) const SEL_CHAR_BAG_CONTENTS: &str = {
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
};
pub(super) const DEL_ITEM_REFUND_INSTANCE: &str =
    { "DELETE FROM item_refund_instance WHERE item_guid = ?" };
pub(super) const DEL_ITEMCONTAINER_MONEY: &str =
    "DELETE FROM item_loot_money WHERE container_id = ?";
pub(super) const DEL_ITEMCONTAINER_ITEMS: &str =
    "DELETE FROM item_loot_items WHERE container_id = ?";
pub(super) const DEL_ITEMCONTAINER_ITEM: &str = {
    "DELETE FROM item_loot_items WHERE container_id = ? AND item_id = ? AND item_count = ? AND item_index = ?"
};
pub(super) const SEL_ITEMCONTAINER_MONEY: &str =
    { "SELECT money FROM item_loot_money WHERE container_id = ? LIMIT 1" };
pub(super) const SEL_ITEMCONTAINER_MONEY_FOR_UPDATE: &str =
    { "SELECT money FROM item_loot_money WHERE container_id = ? FOR UPDATE" };
pub(super) const INS_ITEMCONTAINER_MONEY: &str =
    { "INSERT INTO item_loot_money (container_id, money) VALUES (?, ?)" };
pub(super) const SEL_ITEMCONTAINER_ITEMS: &str = {
    "SELECT item_id, item_count, item_index, follow_rules, ffa, blocked, counted, under_threshold, needs_quest, random_properties_id, random_properties_seed, context \
                 FROM item_loot_items WHERE container_id = ? ORDER BY item_index"
};
pub(super) const INS_ITEMCONTAINER_ITEMS: &str = {
    "INSERT INTO item_loot_items \
                 (container_id, item_id, item_count, item_index, follow_rules, ffa, blocked, counted, under_threshold, needs_quest, random_properties_id, random_properties_seed, context) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
};
pub(super) const INS_ITEM_REFUND_INSTANCE: &str = {
    "INSERT INTO item_refund_instance \
                 (item_guid, player_guid, paidMoney, paidExtendedCost) \
                 VALUES (?, ?, ?, ?)"
};
