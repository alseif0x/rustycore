//! Character progression, quest, spell, and aura SQL values.
pub(super) const DEL_POOL_QUEST_SAVE: &str = "DELETE FROM pool_quest_save WHERE pool_id = ?";
pub(super) const INS_POOL_QUEST_SAVE: &str = {
                "INSERT INTO pool_quest_save (pool_id, quest_id) VALUES (?, ?)"
            };
pub(super) const DEL_CHAR_REPUTATION_BY_FACTION: &str = {
                "DELETE FROM character_reputation WHERE guid = ? AND faction = ?"
            };
pub(super) const INS_CHAR_REPUTATION_BY_FACTION: &str = {
                "INSERT INTO character_reputation (guid, faction, standing, flags) VALUES (?, ?, ? , ?)"
            };
pub(super) const DEL_CHAR_REPUTATION: &str = "DELETE FROM character_reputation WHERE guid = ?";
pub(super) const SEL_CHARACTER_AURAS: &str = {
                "SELECT casterGuid, itemGuid, spell, effectMask, recalculateMask, difficulty, stackCount, maxDuration, remainTime, remainCharges, castItemId, castItemLevel FROM character_aura WHERE guid = ?"
            };
pub(super) const SEL_CHARACTER_AURA_EFFECTS: &str = {
                "SELECT casterGuid, itemGuid, spell, effectMask, effectIndex, amount, baseAmount FROM character_aura_effect WHERE guid = ?"
            };
pub(super) const SEL_CHARACTER_SKILLS: &str = {
                "SELECT skill, value, max, professionSlot FROM character_skills WHERE guid = ?"
            };
pub(super) const SEL_CHARACTER_SPELL: &str = {
                "SELECT spell, active, disabled FROM character_spell WHERE guid = ?"
            };
pub(super) const SEL_CHARACTER_SPELL_FAVORITES: &str = {
                "SELECT spell FROM character_spell_favorite WHERE guid = ?"
            };
pub(super) const SEL_CHARACTER_QUESTSTATUS_OBJECTIVES_CRITERIA: &str = {
                "SELECT questObjectiveId FROM character_queststatus_objectives_criteria WHERE guid = ?"
            };
pub(super) const SEL_CHARACTER_QUESTSTATUS_OBJECTIVES_CRITERIA_PROGRESS: &str = {
                "SELECT criteriaId, counter, date FROM character_queststatus_objectives_criteria_progress WHERE guid = ?"
            };
pub(super) const SEL_CHARACTER_QUESTSTATUS_DAILY: &str = {
                "SELECT quest, time FROM character_queststatus_daily WHERE guid = ?"
            };
pub(super) const SEL_CHARACTER_QUESTSTATUS_WEEKLY: &str = {
                "SELECT quest FROM character_queststatus_weekly WHERE guid = ?"
            };
pub(super) const SEL_CHARACTER_QUESTSTATUS_MONTHLY: &str = {
                "SELECT quest FROM character_queststatus_monthly WHERE guid = ?"
            };
pub(super) const SEL_CHARACTER_QUESTSTATUS_SEASONAL: &str = {
                "SELECT quest, event, completedTime FROM character_queststatus_seasonal WHERE guid = ?"
            };
pub(super) const SEL_CHARACTER_REPUTATION: &str = {
                "SELECT faction, standing, flags FROM character_reputation WHERE guid = ?"
            };
pub(super) const SEL_CHARACTER_SPELLCOOLDOWNS: &str = {
                "SELECT spell, item, time, categoryId, categoryEnd FROM character_spell_cooldown WHERE guid = ? AND time > UNIX_TIMESTAMP()"
            };
pub(super) const SEL_CHARACTER_SPELL_CHARGES: &str = {
                "SELECT categoryId, rechargeStart, rechargeEnd FROM character_spell_charges WHERE guid = ? AND rechargeEnd > UNIX_TIMESTAMP() ORDER BY rechargeEnd"
            };
pub(super) const SEL_CHARACTER_ACHIEVEMENTS: &str = {
                "SELECT achievement, date FROM character_achievement WHERE guid = ?"
            };
pub(super) const SEL_CHARACTER_CRITERIAPROGRESS: &str = {
                "SELECT criteria, counter, date FROM character_achievement_progress WHERE guid = ?"
            };
pub(super) const SEL_CHARACTER_GLYPHS: &str = {
                "SELECT talentGroup, glyphSlot, glyphId FROM character_glyphs WHERE guid = ?"
            };
pub(super) const SEL_CHARACTER_TALENTS: &str = {
                "SELECT talentId, talentRank, talentGroup FROM character_talent WHERE guid = ?"
            };
pub(super) const SEL_CHARACTER_QUESTSTATUSREW: &str = {
                "SELECT quest FROM character_queststatus_rewarded WHERE guid = ? AND active = 1"
            };
pub(super) const SEL_PLAYER_CURRENCY: &str = {
                "SELECT Currency, Quantity, WeeklyQuantity, TrackedQuantity, \
                 IncreasedCapQuantity, EarnedQuantity, Flags \
                 FROM character_currency WHERE CharacterGuid = ?"
            };
pub(super) const UPD_PLAYER_CURRENCY: &str = {
                "UPDATE character_currency SET Quantity = ?, WeeklyQuantity = ?, \
                 TrackedQuantity = ?, IncreasedCapQuantity = ?, EarnedQuantity = ?, Flags = ? \
                 WHERE CharacterGuid = ? AND Currency = ?"
            };
pub(super) const REP_PLAYER_CURRENCY: &str = {
                "REPLACE INTO character_currency \
                 (CharacterGuid, Currency, Quantity, WeeklyQuantity, TrackedQuantity, \
                  IncreasedCapQuantity, EarnedQuantity, Flags) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
            };
pub(super) const DEL_PLAYER_CURRENCY: &str = "DELETE FROM character_currency WHERE CharacterGuid = ?";
pub(super) const SEL_CHARACTER_ACTIONS_SPEC: &str = {
                "SELECT button, action, type FROM character_action \
                 WHERE guid = ? AND spec = ? AND traitConfigId = ? ORDER BY button"
            };
pub(super) const INS_CHARACTER_ACTION: &str = {
                "INSERT INTO character_action (guid, spec, traitConfigId, button, action, type) \
                 VALUES (?, 0, 0, ?, ?, ?)"
            };
pub(super) const UPD_CHAR_PLAYED_TIME: &str = {
                "UPDATE characters SET totaltime = ?, leveltime = ? WHERE guid = ?"
            };
pub(super) const DEL_RESET_CHARACTER_QUESTSTATUS_SEASONAL_BY_EVENT: &str = {
                "DELETE FROM character_queststatus_seasonal WHERE event = ? AND completedTime < ?"
            };
pub(super) const DEL_CHARACTER_QUESTSTATUS_DAILY: &str = {
                "DELETE FROM character_queststatus_daily WHERE guid = ?"
            };
pub(super) const DEL_CHARACTER_QUESTSTATUS_WEEKLY: &str = {
                "DELETE FROM character_queststatus_weekly WHERE guid = ?"
            };
pub(super) const DEL_CHARACTER_QUESTSTATUS_MONTHLY: &str = {
                "DELETE FROM character_queststatus_monthly WHERE guid = ?"
            };
pub(super) const DEL_CHARACTER_QUESTSTATUS_SEASONAL: &str = {
                "DELETE FROM character_queststatus_seasonal WHERE guid = ?"
            };
pub(super) const INS_CHARACTER_QUESTSTATUS_DAILY: &str = {
                "INSERT INTO character_queststatus_daily (guid, quest, time) VALUES (?, ?, ?)"
            };
pub(super) const INS_CHARACTER_QUESTSTATUS_WEEKLY: &str = {
                "INSERT INTO character_queststatus_weekly (guid, quest) VALUES (?, ?)"
            };
pub(super) const INS_CHARACTER_QUESTSTATUS_MONTHLY: &str = {
                "INSERT INTO character_queststatus_monthly (guid, quest) VALUES (?, ?)"
            };
pub(super) const INS_CHARACTER_QUESTSTATUS_SEASONAL: &str = {
                "INSERT INTO character_queststatus_seasonal (guid, quest, event, completedTime) VALUES (?, ?, ?, ?)"
            };
pub(super) const DEL_INVALID_SPELL_SPELLS: &str = "DELETE FROM character_spell WHERE spell = ?";
pub(super) const UPD_LEVEL: &str = "UPDATE characters SET level = ?, xp = 0 WHERE guid = ?";
pub(super) const DEL_INVALID_ACHIEV_PROGRESS_CRITERIA: &str = {
                "DELETE FROM character_achievement_progress WHERE criteria = ?"
            };
pub(super) const DEL_INVALID_ACHIEVMENT: &str = {
                "DELETE FROM character_achievement WHERE achievement = ?"
            };
pub(super) const DEL_CHARACTER_SKILL: &str = {
                "DELETE FROM character_skills WHERE guid = ? AND skill = ?"
            };
pub(super) const SEL_CHARACTER_AURA_FROZEN: &str = {
                "SELECT characters.name, character_aura.remainTime FROM characters LEFT JOIN character_aura ON (characters.guid = character_aura.guid) WHERE character_aura.spell = 9454"
            };
pub(super) const SEL_PINFO_XP: &str = {
                "SELECT a.xp, b.guid FROM characters a LEFT JOIN guild_member b ON a.guid = b.guid WHERE a.guid = ?"
            };
pub(super) const DEL_CHAR_AURA_FROZEN: &str = {
                "DELETE FROM character_aura WHERE spell = 9454 AND guid = ?"
            };
pub(super) const DEL_CHAR_ACHIEVEMENT: &str = "DELETE FROM character_achievement WHERE guid = ?";
pub(super) const DEL_CHAR_ACHIEVEMENT_PROGRESS: &str = {
                "DELETE FROM character_achievement_progress WHERE guid = ?"
            };
pub(super) const INS_CHAR_ACHIEVEMENT: &str = {
                "INSERT INTO character_achievement (guid, achievement, date) VALUES (?, ?, ?)"
            };
pub(super) const DEL_CHAR_ACHIEVEMENT_PROGRESS_BY_CRITERIA: &str = {
                "DELETE FROM character_achievement_progress WHERE guid = ? AND criteria = ?"
            };
pub(super) const INS_CHAR_ACHIEVEMENT_PROGRESS: &str = {
                "INSERT INTO character_achievement_progress (guid, criteria, counter, date) VALUES (?, ?, ?, ?)"
            };
pub(super) const DEL_CHAR_SKILL_LANGUAGES: &str = {
                "DELETE FROM character_skills WHERE skill IN (98, 113, 759, 111, 313, 109, 115, 315, 673, 137) AND guid = ?"
            };
pub(super) const INS_CHAR_SKILL_LANGUAGE: &str = {
                "INSERT INTO `character_skills` (guid, skill, value, max) VALUES (?, ?, 300, 300)"
            };
pub(super) const DEL_CHAR_QUESTSTATUS: &str = "DELETE FROM character_queststatus WHERE guid = ?";
pub(super) const DEL_CHAR_QUESTSTATUS_OBJECTIVES: &str = {
                "DELETE FROM character_queststatus_objectives WHERE guid = ?"
            };
pub(super) const DEL_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA: &str = {
                "DELETE FROM character_queststatus_objectives_criteria WHERE guid = ?"
            };
pub(super) const DEL_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA_PROGRESS: &str = {
                "DELETE FROM character_queststatus_objectives_criteria_progress WHERE guid = ?"
            };
pub(super) const DEL_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA_PROGRESS_BY_CRITERIA: &str = {
                "DELETE FROM character_queststatus_objectives_criteria_progress WHERE guid = ? AND criteriaId = ?"
            };
pub(super) const DEL_CHAR_ACHIEVEMENT_BY_ACHIEVEMENT: &str = {
                "DELETE FROM character_achievement WHERE achievement = ? AND guid = ?"
            };
pub(super) const UPD_CHAR_ACHIEVEMENT: &str = {
                "UPDATE character_achievement SET achievement = ? where achievement = ? AND guid = ?"
            };
pub(super) const DEL_CHAR_SPELL_BY_SPELL: &str = {
                "DELETE FROM character_spell WHERE spell = ? AND guid = ?"
            };
pub(super) const UPD_CHAR_SPELL_FACTION_CHANGE: &str = {
                "UPDATE character_spell SET spell = ? where spell = ? AND guid = ?"
            };
pub(super) const SEL_CHAR_REP_BY_FACTION: &str = {
                "SELECT standing FROM character_reputation WHERE faction = ? AND guid = ?"
            };
pub(super) const DEL_CHAR_REP_BY_FACTION: &str = {
                "DELETE FROM character_reputation WHERE faction = ? AND guid = ?"
            };
pub(super) const UPD_CHAR_REP_FACTION_CHANGE: &str = {
                "UPDATE character_reputation SET faction = ?, standing = ? WHERE faction = ? AND guid = ?"
            };
pub(super) const UPD_CHAR_TITLES_FACTION_CHANGE: &str = {
                "UPDATE characters SET knownTitles = ? WHERE guid = ?"
            };
pub(super) const RES_CHAR_TITLES_FACTION_CHANGE: &str = {
                "UPDATE characters SET chosenTitle = 0 WHERE guid = ?"
            };
pub(super) const DEL_CHAR_SPELL_COOLDOWNS: &str = "DELETE FROM character_spell_cooldown WHERE guid = ?";
pub(super) const INS_CHAR_SPELL_COOLDOWN: &str = {
                "INSERT INTO character_spell_cooldown (guid, spell, item, time, categoryId, categoryEnd) VALUES (?, ?, ?, ?, ?, ?)"
            };
pub(super) const DEL_CHAR_SPELL_CHARGES: &str = "DELETE FROM character_spell_charges WHERE guid = ?";
pub(super) const INS_CHAR_SPELL_CHARGES: &str = {
                "INSERT INTO character_spell_charges (guid, categoryId, rechargeStart, rechargeEnd) VALUES (?, ?, ?, ?)"
            };
pub(super) const DEL_CHAR_ACTION: &str = "DELETE FROM character_action WHERE guid = ?";
pub(super) const DEL_CHAR_AURA: &str = "DELETE FROM character_aura WHERE guid = ?";
pub(super) const DEL_CHAR_AURA_EFFECT: &str = "DELETE FROM character_aura_effect WHERE guid = ?";
pub(super) const DEL_CHAR_QUESTSTATUS_REWARDED: &str = {
                "DELETE FROM character_queststatus_rewarded WHERE guid = ?"
            };
pub(super) const DEL_CHAR_SPELL: &str = "DELETE FROM character_spell WHERE guid = ?";
pub(super) const DEL_CHAR_ACHIEVEMENTS: &str = {
                "DELETE FROM character_achievement WHERE guid = ? AND achievement NOT IN (456,457,458,459,460,461,462,463,464,465,466,467,1400,1402,1404,1405,1406,1407,1408,1409,1410,1411,1412,1413,1414,1415,1416,1417,1418,1419,1420,1421,1422,1423,1424,1425,1426,1427,1463,3117,3259,4078,4576,4998,4999,5000,5001,5002,5003,5004,5005,5006,5007,5008,5381,5382,5383,5384,5385,5386,5387,5388,5389,5390,5391,5392,5393,5394,5395,5396,6433,6523,6524,6743,6744,6745,6746,6747,6748,6749,6750,6751,6752,6829,6859,6860,6861,6862,6863,6864,6865,6866,6867,6868,6869,6870,6871,6872,6873)"
            };
pub(super) const DEL_CHAR_GLYPHS: &str = "DELETE FROM character_glyphs WHERE guid = ?";
pub(super) const DEL_CHAR_TALENT: &str = "DELETE FROM character_talent WHERE guid = ?";
pub(super) const DEL_CHAR_SKILLS: &str = "DELETE FROM character_skills WHERE guid = ?";
pub(super) const INS_CHAR_ACTION: &str = {
                "INSERT INTO character_action (guid, spec, traitConfigId, button, action, type) VALUES (?, ?, ?, ?, ?, ?)"
            };
pub(super) const UPD_CHAR_ACTION: &str = {
                "UPDATE character_action SET action = ?, type = ? WHERE guid = ? AND button = ? AND spec = ? AND traitConfigId = ?"
            };
pub(super) const DEL_CHAR_ACTION_BY_BUTTON_SPEC: &str = {
                "DELETE FROM character_action WHERE guid = ? and button = ? and spec = ? AND traitConfigId = ?"
            };
pub(super) const DEL_CHAR_ACTION_BY_SPEC: &str = {
                "DELETE FROM character_action WHERE guid = ? AND spec = ? AND traitConfigId = ?"
            };
pub(super) const DEL_CHAR_ACTION_BY_TRAIT_CONFIG: &str = {
                "DELETE FROM character_action WHERE guid = ? AND traitConfigId = ?"
            };
pub(super) const REP_CHAR_QUESTSTATUS: &str = {
                "REPLACE INTO character_queststatus (guid, quest, status, explored, acceptTime, endTime) VALUES (?, ?, ?, ?, ?, ?)"
            };
pub(super) const DEL_CHAR_QUESTSTATUS_BY_QUEST: &str = {
                "DELETE FROM character_queststatus WHERE guid = ? AND quest = ?"
            };
pub(super) const INS_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA: &str = {
                "INSERT INTO character_queststatus_objectives_criteria (guid, questObjectiveId) VALUES (?, ?)"
            };
pub(super) const INS_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA_PROGRESS: &str = {
                "INSERT INTO character_queststatus_objectives_criteria_progress (guid, criteriaId, counter, date) VALUES (?, ?, ?, ?)"
            };
pub(super) const INS_CHAR_QUESTSTATUS_REWARDED: &str = {
                "INSERT IGNORE INTO character_queststatus_rewarded (guid, quest, active) VALUES (?, ?, 1)"
            };
pub(super) const DEL_CHAR_QUESTSTATUS_REWARDED_BY_QUEST: &str = {
                "DELETE FROM character_queststatus_rewarded WHERE guid = ? AND quest = ?"
            };
pub(super) const UPD_CHAR_QUESTSTATUS_REWARDED_FACTION_CHANGE: &str = {
                "UPDATE character_queststatus_rewarded SET quest = ? WHERE quest = ? AND guid = ?"
            };
pub(super) const UPD_CHAR_QUESTSTATUS_REWARDED_ACTIVE: &str = {
                "UPDATE character_queststatus_rewarded SET active = 1 WHERE guid = ?"
            };
pub(super) const UPD_CHAR_QUESTSTATUS_REWARDED_ACTIVE_BY_QUEST: &str = {
                "UPDATE character_queststatus_rewarded SET active = 0 WHERE quest = ? AND guid = ?"
            };
pub(super) const DEL_INVALID_QUEST_PROGRESS_CRITERIA: &str = {
                "DELETE FROM character_queststatus_objectives_criteria WHERE questObjectiveId = ?"
            };
pub(super) const DEL_CHAR_SKILL_BY_SKILL: &str = {
                "DELETE FROM character_skills WHERE guid = ? AND skill = ?"
            };
pub(super) const INS_CHAR_SKILLS: &str = {
                "INSERT INTO character_skills (guid, skill, value, max, professionSlot) VALUES (?, ?, ?, ?, ?)"
            };
pub(super) const UPD_CHAR_SKILLS: &str = {
                "UPDATE character_skills SET value = ?, max = ?, professionSlot = ? WHERE guid = ? AND skill = ?"
            };
pub(super) const INS_CHAR_SPELL: &str = {
                "INSERT INTO character_spell (guid, spell, active, disabled) VALUES (?, ?, ?, ?)"
            };
pub(super) const UPSERT_CHAR_SPELL_LEARN_FALLBACK: &str = {
                "INSERT INTO character_spell (guid, spell, active, disabled) VALUES (?, ?, ?, ?) ON DUPLICATE KEY UPDATE active = IF(character_spell.disabled, character_spell.active, VALUES(active)), disabled = VALUES(disabled)"
            };
pub(super) const DEL_CHAR_SPELL_FAVORITE: &str = {
                "DELETE FROM character_spell_favorite WHERE guid = ? AND spell = ?"
            };
pub(super) const DEL_CHAR_SPELL_FAVORITE_BY_CHAR: &str = {
                "DELETE FROM character_spell_favorite WHERE guid = ?"
            };
pub(super) const INS_CHAR_SPELL_FAVORITE: &str = {
                "INSERT INTO character_spell_favorite (guid, spell) VALUES (?, ?)"
            };
pub(super) const DEL_CHAR_STATS: &str = "DELETE FROM character_stats WHERE guid = ?";
pub(super) const INS_CHAR_STATS: &str = {
                "INSERT INTO character_stats (guid, maxhealth, maxpower1, maxpower2, maxpower3, maxpower4, maxpower5, maxpower6, maxpower7, maxpower8, maxpower9, maxpower10, strength, agility, stamina, intellect, armor, resHoly, resFire, resNature, resFrost, resShadow, resArcane, blockPct, dodgePct, parryPct, critPct, rangedCritPct, spellCritPct, attackPower, rangedAttackPower, spellPower, resilience, mastery, versatility) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            };
pub(super) const INS_CHAR_GLYPHS: &str = {
                "INSERT INTO character_glyphs (guid, talentGroup, glyphSlot, glyphId) VALUES(?, ?, ?, ?)"
            };
pub(super) const INS_CHAR_TALENT: &str = {
                "INSERT INTO character_talent (guid, talentId, talentRank, talentGroup) VALUES (?, ?, ?, ?)"
            };
pub(super) const INS_CHAR_FISHINGSTEPS: &str = {
                "INSERT INTO character_fishingsteps (guid, fishingSteps) VALUES (?, ?)"
            };
pub(super) const DEL_CHAR_FISHINGSTEPS: &str = "DELETE FROM character_fishingsteps WHERE guid = ?";
pub(super) const SEL_CHAR_TRAIT_ENTRIES: &str = {
                "SELECT traitConfigId, traitNodeId, traitNodeEntryId, `rank`, grantedRanks FROM character_trait_entry WHERE guid = ?"
            };
pub(super) const INS_CHAR_TRAIT_ENTRIES: &str = {
                "INSERT INTO character_trait_entry (guid, traitConfigId, traitNodeId, traitNodeEntryId, `rank`, grantedRanks) VALUES (?, ?, ?, ?, ?, ?)"
            };
pub(super) const DEL_CHAR_TRAIT_ENTRIES: &str = {
                "DELETE FROM character_trait_entry WHERE guid = ? AND traitConfigId = ?"
            };
pub(super) const DEL_CHAR_TRAIT_ENTRIES_BY_CHAR: &str = {
                "DELETE FROM character_trait_entry WHERE guid = ?"
            };
pub(super) const SEL_CHAR_TRAIT_CONFIGS: &str = {
                "SELECT traitConfigId, type, chrSpecializationId, combatConfigFlags, localIdentifier, skillLineId, traitSystemId, `name` FROM character_trait_config WHERE guid = ?"
            };
pub(super) const INS_CHAR_TRAIT_CONFIGS: &str = {
                "INSERT INTO character_trait_config (guid, traitConfigId, type, chrSpecializationId, combatConfigFlags, localIdentifier, skillLineId, traitSystemId, `name`) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
            };
pub(super) const DEL_CHAR_TRAIT_CONFIGS: &str = {
                "DELETE FROM character_trait_config WHERE guid = ? AND traitConfigId = ?"
            };
pub(super) const DEL_CHAR_TRAIT_CONFIGS_BY_CHAR: &str = {
                "DELETE FROM character_trait_config WHERE guid = ?"
            };
pub(super) const DEL_RESET_CHARACTER_QUESTSTATUS_DAILY: &str = {
                "DELETE FROM character_queststatus_daily"
            };
pub(super) const DEL_RESET_CHARACTER_QUESTSTATUS_WEEKLY: &str = {
                "DELETE FROM character_queststatus_weekly"
            };
pub(super) const DEL_RESET_CHARACTER_QUESTSTATUS_MONTHLY: &str = {
                "DELETE FROM character_queststatus_monthly"
            };
pub(super) const INS_QUEST_TRACK: &str = {
                "INSERT INTO quest_tracker (id, character_guid, quest_accept_time, core_hash, core_revision) VALUES (?, ?, NOW(), ?, ?)"
            };
pub(super) const UPD_QUEST_TRACK_GM_COMPLETE: &str = {
                "UPDATE quest_tracker SET completed_by_gm = 1 WHERE id = ? AND character_guid = ? ORDER BY quest_accept_time DESC LIMIT 1"
            };
pub(super) const UPD_QUEST_TRACK_COMPLETE_TIME: &str = {
                "UPDATE quest_tracker SET quest_complete_time = NOW() WHERE id = ? AND character_guid = ? ORDER BY quest_accept_time DESC LIMIT 1"
            };
pub(super) const UPD_QUEST_TRACK_ABANDON_TIME: &str = {
                "UPDATE quest_tracker SET quest_abandon_time = NOW() WHERE id = ? AND character_guid = ? ORDER BY quest_accept_time DESC LIMIT 1"
            };
pub(super) const SEL_CHARACTER_AURA_STORED_LOCATIONS: &str = {
                "SELECT Spell, MapId, PositionX, PositionY, PositionZ, Orientation FROM character_aura_stored_location WHERE Guid = ?"
            };
pub(super) const DEL_CHARACTER_AURA_STORED_LOCATIONS_BY_GUID: &str = {
                "DELETE FROM character_aura_stored_location WHERE Guid = ?"
            };
pub(super) const DEL_CHARACTER_AURA_STORED_LOCATION: &str = {
                "DELETE FROM character_aura_stored_location WHERE Guid = ? AND Spell = ?"
            };
pub(super) const INS_CHARACTER_AURA_STORED_LOCATION: &str = {
                "INSERT INTO character_aura_stored_location (Guid, Spell, MapId, PositionX, PositionY, PositionZ, Orientation) VALUES (?, ?, ?, ?, ?, ?, ?)"
            };
pub(super) const SEL_WAR_MODE_TUNING: &str = {
                "SELECT race, COUNT(guid) FROM characters WHERE ((playerFlags & ?) = ?) AND logout_time >= (UNIX_TIMESTAMP() - 604800) GROUP BY race"
            };
pub(super) const UPD_CHAR_XP: &str = "UPDATE characters SET xp = ? WHERE guid = ?";
pub(super) const UPD_CHAR_LEVEL: &str = "UPDATE characters SET level = ?, xp = ? WHERE guid = ?";
pub(super) const UPD_CHAR_MONEY: &str = "UPDATE characters SET money = ? WHERE guid = ?";
pub(super) const SEL_CHAR_MONEY_FOR_UPDATE: &str = {
                "SELECT money FROM characters WHERE guid = ? FOR UPDATE"
            };
pub(super) const SEL_CHAR_MONEY: &str = "SELECT money FROM characters WHERE guid = ?";
pub(super) const UPD_CHAR_HEALTH: &str = "UPDATE characters SET health = ? WHERE guid = ?";
pub(super) const UPD_CHAR_POWERS: &str = {
                "UPDATE characters SET power1 = ?, power2 = ?, power3 = ?, power4 = ?, power5 = ?, power6 = ?, power7 = ?, power8 = ?, power9 = ?, power10 = ? WHERE guid = ?"
            };
pub(super) const UPD_CHAR_REST_STATE: &str = {
                "UPDATE characters SET restState = ?, playerFlags = ?, rest_bonus = ?, logout_time = ?, is_logout_resting = ? WHERE guid = ?"
            };
pub(super) const UPD_CHAR_ONLINE_REST_STATE: &str = {
                "UPDATE characters SET restState = ?, playerFlags = ?, rest_bonus = ? WHERE guid = ?"
            };
pub(super) const UPD_CHAR_TALENT_RESET_STATE: &str = {
                "UPDATE characters SET resettalents_cost = ?, resettalents_time = ? WHERE guid = ?"
            };
pub(super) const UPD_CHARACTER_MONEY_GUARDED: &str = {
                "UPDATE characters SET money = ? WHERE guid = ? AND money = ?"
            };
pub(super) const INS_AURA: &str = {
                "INSERT INTO character_aura (guid, casterGuid, itemGuid, spell, effectMask, recalculateMask, difficulty, stackCount, maxDuration, remainTime, remainCharges, castItemId, castItemLevel) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            };
pub(super) const INS_AURA_EFFECT: &str = {
                "INSERT INTO character_aura_effect (guid, casterGuid, itemGuid, spell, effectMask, effectIndex, amount, baseAmount) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
            };
pub(super) const INS_CHARACTER_SPELL: &str = {
                "INSERT IGNORE INTO character_spell (guid, spell, active, disabled) VALUES (?, ?, 1, 0)"
            };
pub(super) const SEL_CHAR_QUEST_STATUS: &str = {
                "SELECT quest, status, explored, acceptTime, endTime FROM character_queststatus WHERE guid = ? AND status <> 0"
            };
pub(super) const SEL_CHARACTER_QUESTSTATUS: &str = {
                "SELECT quest, status, explored, acceptTime, endTime FROM character_queststatus WHERE guid = ? AND status <> 0"
            };
pub(super) const SEL_CHAR_QUEST_STATUS_OBJECTIVES: &str = {
                "SELECT quest, objective, data FROM character_queststatus_objectives WHERE guid = ?"
            };
pub(super) const SEL_CHARACTER_QUESTSTATUS_OBJECTIVES: &str = {
                "SELECT quest, objective, data FROM character_queststatus_objectives WHERE guid = ?"
            };
pub(super) const SEL_CHAR_QUEST_STATUS_SEASONAL: &str = {
                "SELECT quest, event, completedTime FROM character_queststatus_seasonal WHERE guid = ?"
            };
pub(super) const INS_CHAR_QUEST_STATUS: &str = {
                "REPLACE INTO character_queststatus (guid, quest, status, explored, acceptTime, endTime) VALUES (?, ?, ?, ?, ?, ?)"
            };
pub(super) const DEL_CHAR_QUEST_STATUS: &str = {
                "DELETE FROM character_queststatus WHERE guid = ? AND quest = ?"
            };
pub(super) const DEL_CHAR_QUEST_STATUS_OBJECTIVES_BY_QUEST: &str = {
                "DELETE FROM character_queststatus_objectives WHERE guid = ? AND quest = ?"
            };
pub(super) const DEL_CHAR_QUESTSTATUS_OBJECTIVES_BY_QUEST: &str = {
                "DELETE FROM character_queststatus_objectives WHERE guid = ? AND quest = ?"
            };
pub(super) const REP_CHAR_QUEST_STATUS_OBJECTIVES: &str = {
                "REPLACE INTO character_queststatus_objectives (guid, quest, objective, data) VALUES (?, ?, ?, ?)"
            };
pub(super) const REP_CHAR_QUESTSTATUS_OBJECTIVES: &str = {
                "REPLACE INTO character_queststatus_objectives (guid, quest, objective, data) VALUES (?, ?, ?, ?)"
            };
