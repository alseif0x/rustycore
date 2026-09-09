//! Character-statement C++ contrast regressions, part 3 of 4.
//!
//! Moved out of the character_tests.rs root under #652; every test is unchanged.

use super::*;

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
