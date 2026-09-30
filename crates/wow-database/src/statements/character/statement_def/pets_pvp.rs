//! Character pet, arena, battleground, and PvP SQL values.
pub(super) const DEL_BATTLEGROUND_RANDOM_ALL: &str = "DELETE FROM character_battleground_random";
pub(super) const DEL_BATTLEGROUND_RANDOM: &str = {
                "DELETE FROM character_battleground_random WHERE guid = ?"
            };
pub(super) const INS_BATTLEGROUND_RANDOM: &str = {
                "INSERT INTO character_battleground_random (guid) VALUES (?)"
            };
pub(super) const SEL_CHARACTER_BGDATA: &str = {
                "SELECT instanceId, team, joinX, joinY, joinZ, joinO, joinMapId, taxiStart, taxiEnd, mountSpell, queueId FROM character_battleground_data WHERE guid = ?"
            };
pub(super) const SEL_CHARACTER_RANDOMBG: &str = {
                "SELECT guid FROM character_battleground_random WHERE guid = ?"
            };
pub(super) const DEL_INVALID_PET_SPELL: &str = "DELETE FROM pet_spell WHERE spell = ?";
pub(super) const SEL_CHAR_PET_IDS: &str = "SELECT id FROM character_pet WHERE owner = ?";
pub(super) const DEL_CHAR_PET_DECLINEDNAME_BY_OWNER: &str = {
                "DELETE FROM character_pet_declinedname WHERE owner = ?"
            };
pub(super) const DEL_CHAR_PET_DECLINEDNAME: &str = {
                "DELETE FROM character_pet_declinedname WHERE id = ?"
            };
pub(super) const INS_CHAR_PET_DECLINEDNAME: &str = {
                "INSERT INTO character_pet_declinedname (id, owner, genitive, dative, accusative, instrumental, prepositional) VALUES (?, ?, ?, ?, ?, ?, ?)"
            };
pub(super) const SEL_PET_AURA: &str = {
                "SELECT casterGuid, spell, effectMask, recalculateMask, difficulty, stackCount, maxDuration, remainTime, remainCharges FROM pet_aura WHERE guid = ?"
            };
pub(super) const SEL_PET_AURA_EFFECT: &str = {
                "SELECT casterGuid, spell, effectMask, effectIndex, amount, baseAmount FROM pet_aura_effect WHERE guid = ?"
            };
pub(super) const SEL_PET_SPELL: &str = "SELECT spell, active FROM pet_spell WHERE guid = ?";
pub(super) const SEL_PET_SPELL_COOLDOWN: &str = {
                "SELECT spell, time, categoryId, categoryEnd FROM pet_spell_cooldown WHERE guid = ? AND time > UNIX_TIMESTAMP()"
            };
pub(super) const SEL_PET_DECLINED_NAME: &str = {
                "SELECT genitive, dative, accusative, instrumental, prepositional FROM character_pet_declinedname WHERE owner = ? AND id = ?"
            };
pub(super) const DEL_PET_AURAS: &str = "DELETE FROM pet_aura WHERE guid = ?";
pub(super) const DEL_PET_AURA_EFFECTS: &str = "DELETE FROM pet_aura_effect WHERE guid = ?";
pub(super) const DEL_PET_SPELLS: &str = "DELETE FROM pet_spell WHERE guid = ?";
pub(super) const DEL_PET_SPELL_COOLDOWNS: &str = "DELETE FROM pet_spell_cooldown WHERE guid = ?";
pub(super) const INS_PET_SPELL_COOLDOWN: &str = {
                "INSERT INTO pet_spell_cooldown (guid, spell, time, categoryId, categoryEnd) VALUES (?, ?, ?, ?, ?)"
            };
pub(super) const SEL_PET_SPELL_CHARGES: &str = {
                "SELECT categoryId, rechargeStart, rechargeEnd FROM pet_spell_charges WHERE guid = ? AND rechargeEnd > UNIX_TIMESTAMP() ORDER BY rechargeEnd"
            };
pub(super) const DEL_PET_SPELL_CHARGES: &str = "DELETE FROM pet_spell_charges WHERE guid = ?";
pub(super) const INS_PET_SPELL_CHARGES: &str = {
                "INSERT INTO pet_spell_charges (guid, categoryId, rechargeStart, rechargeEnd) VALUES (?, ?, ?, ?)"
            };
pub(super) const DEL_PET_SPELL_BY_SPELL: &str = "DELETE FROM pet_spell WHERE guid = ? and spell = ?";
pub(super) const INS_PET_SPELL: &str = "INSERT INTO pet_spell (guid, spell, active) VALUES (?, ?, ?)";
pub(super) const INS_PET_AURA: &str = {
                "INSERT INTO pet_aura (guid, casterGuid, spell, effectMask, recalculateMask, difficulty, stackCount, maxDuration, remainTime, remainCharges) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            };
pub(super) const INS_PET_AURA_EFFECT: &str = {
                "INSERT INTO pet_aura_effect (guid, casterGuid, spell, effectMask, effectIndex, amount, baseAmount) VALUES (?, ?, ?, ?, ?, ?, ?)"
            };
pub(super) const SEL_CHAR_PETS: &str = {
                "SELECT id, entry, modelid, level, exp, Reactstate, slot, name, renamed, curhealth, curmana, abdata, savetime, CreatedBySpell, PetType, specialization FROM character_pet WHERE owner = ?"
            };
pub(super) const DEL_CHAR_PET_BY_OWNER: &str = "DELETE FROM character_pet WHERE owner = ?";
pub(super) const UPD_CHAR_PET_NAME: &str = {
                "UPDATE character_pet SET name = ?, renamed = 1 WHERE owner = ? AND id = ?"
            };
pub(super) const UPD_CHAR_PET_SLOT_BY_ID: &str = {
                "UPDATE character_pet SET slot = ? WHERE owner = ? AND id = ?"
            };
pub(super) const DEL_CHAR_PET_BY_ID: &str = "DELETE FROM character_pet WHERE id = ?";
pub(super) const DEL_ALL_PET_SPELLS_BY_OWNER: &str = {
                "DELETE FROM pet_spell WHERE guid in (SELECT id FROM character_pet WHERE owner=?)"
            };
pub(super) const UPD_PET_SPECS_BY_OWNER: &str = {
                "UPDATE character_pet SET specialization = 0 WHERE owner=?"
            };
pub(super) const INS_PET: &str = {
                "INSERT INTO character_pet (id, entry, owner, modelid, level, exp, Reactstate, slot, name, renamed, curhealth, curmana, abdata, savetime, CreatedBySpell, PetType, specialization) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            };
pub(super) const SEL_PVPSTATS_MAXID: &str = "SELECT MAX(id) FROM pvpstats_battlegrounds";
pub(super) const INS_PVPSTATS_BATTLEGROUND: &str = {
                "INSERT INTO pvpstats_battlegrounds (id, winner_faction, bracket_id, type, date) VALUES (?, ?, ?, ?, NOW())"
            };
pub(super) const INS_PVPSTATS_PLAYER: &str = {
                "INSERT INTO pvpstats_players (battleground_id, character_guid, winner, score_killing_blows, score_deaths, score_honorable_kills, score_bonus_honor, score_damage_done, score_healing_done, attr_1, attr_2, attr_3, attr_4, attr_5) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            };
pub(super) const SEL_PVPSTATS_FACTIONS_OVERALL: &str = {
                "SELECT winner_faction, COUNT(*) AS count FROM pvpstats_battlegrounds WHERE DATEDIFF(NOW(), date) < 7 GROUP BY winner_faction ORDER BY winner_faction ASC"
            };
pub(super) const INS_BATTLE_PET_PURCHASE: &str = {
                "INSERT INTO character_battle_pet_purchase (request_key, guid, account_id, trainer_id, spell_id, species, breed, quality, display_id, level, price, money_before, money_after, status) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            };
pub(super) const SEL_BATTLE_PET_PURCHASE_BY_KEY: &str = {
                "SELECT request_key, guid, account_id, trainer_id, spell_id, species, breed, quality, display_id, level, price, money_before, money_after, status, failure_reason, published FROM character_battle_pet_purchase WHERE request_key = ?"
            };
pub(super) const SEL_BATTLE_PET_PURCHASE_PENDING: &str = {
                "SELECT request_key, guid, account_id, trainer_id, spell_id, species, breed, quality, display_id, level, price, money_before, money_after, status, failure_reason, published FROM character_battle_pet_purchase WHERE guid = ? AND (status IN (0, 2) OR (status = 1 AND published = 0)) ORDER BY created_at ASC, request_key ASC LIMIT ?"
            };
pub(super) const UPD_BATTLE_PET_PURCHASE_PUBLISHED: &str = {
                "UPDATE character_battle_pet_purchase SET published = 1 WHERE request_key = ? AND published = 0 AND status IN (0, 1, 2)"
            };
pub(super) const UPD_BATTLE_PET_PURCHASE_COMPLETED: &str = {
                "UPDATE character_battle_pet_purchase SET status = 1, failure_reason = NULL WHERE request_key = ? AND status IN (0, 2)"
            };
pub(super) const UPD_BATTLE_PET_PURCHASE_COMPENSATION_PENDING: &str = {
                "UPDATE character_battle_pet_purchase SET status = 2, failure_reason = ? WHERE request_key = ? AND status = 0"
            };
pub(super) const UPD_BATTLE_PET_PURCHASE_COMPENSATED: &str = {
                "UPDATE character_battle_pet_purchase SET status = 3 WHERE request_key = ? AND status = 2"
            };
pub(super) const UPD_BATTLE_PET_PURCHASE_TERMINAL_FAILURE: &str = {
                "UPDATE character_battle_pet_purchase SET status = 4, failure_reason = ? WHERE request_key = ? AND status = 2"
            };
pub(super) const SEL_MATCH_MAKER_RATING: &str = {
                "SELECT matchMakerRating FROM character_arena_stats WHERE guid = ? AND slot = ?"
            };
pub(super) const SEL_CHARACTER_ARENAINFO: &str = {
                "SELECT arenaTeamId, weekGames, seasonGames, seasonWins, personalRating FROM arena_team_member WHERE guid = ?"
            };
pub(super) const INS_ARENA_TEAM: &str = {
                "INSERT INTO arena_team (arenaTeamId, name, captainGuid, type, rating, backgroundColor, emblemStyle, emblemColor, borderStyle, borderColor) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            };
pub(super) const INS_ARENA_TEAM_MEMBER: &str = {
                "INSERT INTO arena_team_member (arenaTeamId, guid, personalRating) VALUES (?, ?, ?)"
            };
pub(super) const DEL_ARENA_TEAM: &str = "DELETE FROM arena_team where arenaTeamId = ?";
pub(super) const DEL_ARENA_TEAM_MEMBERS: &str = "DELETE FROM arena_team_member WHERE arenaTeamId = ?";
pub(super) const UPD_ARENA_TEAM_CAPTAIN: &str = {
                "UPDATE arena_team SET captainGuid = ? WHERE arenaTeamId = ?"
            };
pub(super) const DEL_ARENA_TEAM_MEMBER: &str = {
                "DELETE FROM arena_team_member WHERE arenaTeamId = ? AND guid = ?"
            };
pub(super) const UPD_ARENA_TEAM_STATS: &str = {
                "UPDATE arena_team SET rating = ?, weekGames = ?, weekWins = ?, seasonGames = ?, seasonWins = ?, `rank` = ? WHERE arenaTeamId = ?"
            };
pub(super) const UPD_ARENA_TEAM_MEMBER: &str = {
                "UPDATE arena_team_member SET personalRating = ?, weekGames = ?, weekWins = ?, seasonGames = ?, seasonWins = ? WHERE arenaTeamId = ? AND guid = ?"
            };
pub(super) const DEL_CHARACTER_ARENA_STATS: &str = "DELETE FROM character_arena_stats WHERE guid = ?";
pub(super) const REP_CHARACTER_ARENA_STATS: &str = {
                "REPLACE INTO character_arena_stats (guid, slot, matchMakerRating) VALUES (?, ?, ?)"
            };
pub(super) const UPD_ARENA_TEAM_NAME: &str = "UPDATE arena_team SET name = ? WHERE arenaTeamId = ?";
pub(super) const INS_PLAYER_BGDATA: &str = {
                "INSERT INTO character_battleground_data (guid, instanceId, team, joinX, joinY, joinZ, joinO, joinMapId, taxiStart, taxiEnd, mountSpell, queueId) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            };
pub(super) const DEL_PLAYER_BGDATA: &str = "DELETE FROM character_battleground_data WHERE guid = ?";
