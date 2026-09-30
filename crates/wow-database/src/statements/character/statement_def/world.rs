//! Character world, instance, respawn, and corpse SQL values.
pub(super) const SEL_CHAR_ZONE: &str = "SELECT zone FROM characters WHERE guid = ?";
pub(super) const SEL_CHAR_POSITION_XYZ: &str = {
                "SELECT map, position_x, position_y, position_z FROM characters WHERE guid = ?"
            };
pub(super) const SEL_CHAR_POSITION: &str = {
                "SELECT position_x, position_y, position_z, orientation, map, taxi_path FROM characters WHERE guid = ?"
            };
pub(super) const SEL_CHARACTER_HOMEBIND: &str = {
                "SELECT mapId, zoneId, posX, posY, posZ, orientation FROM character_homebind WHERE guid = ?"
            };
pub(super) const SEL_ACCOUNT_INSTANCELOCKTIMES: &str = {
                "SELECT instanceId, releaseTime FROM account_instance_times WHERE accountId = ?"
            };
pub(super) const DEL_ACCOUNT_INSTANCE_LOCK_TIMES: &str = {
                "DELETE FROM account_instance_times WHERE accountId = ?"
            };
pub(super) const INS_ACCOUNT_INSTANCE_LOCK_TIMES: &str = {
                "INSERT INTO account_instance_times (accountId, instanceId, releaseTime) VALUES (?, ?, ?)"
            };
pub(super) const SEL_INSTANCE: &str = {
                "SELECT instanceId, data, completedEncountersMask, entranceWorldSafeLocId FROM instance"
            };
pub(super) const SEL_CHARACTER_INSTANCE_LOCK: &str = {
                "SELECT guid, mapId, lockId, instanceId, difficulty, data, completedEncountersMask, \
                 entranceWorldSafeLocId, expiryTime, extended FROM character_instance_lock ORDER BY instanceId"
            };
pub(super) const DEL_CHARACTER_INSTANCE_LOCK: &str = {
                "DELETE FROM character_instance_lock WHERE guid = ? AND mapId = ? AND lockId = ?"
            };
pub(super) const DEL_CHARACTER_INSTANCE_LOCK_BY_GUID: &str = {
                "DELETE FROM character_instance_lock WHERE guid = ?"
            };
pub(super) const INS_CHARACTER_INSTANCE_LOCK: &str = {
                "INSERT INTO character_instance_lock \
                 (guid, mapId, lockId, instanceId, difficulty, data, completedEncountersMask, \
                  entranceWorldSafeLocId, expiryTime, extended) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            };
pub(super) const UPD_CHARACTER_INSTANCE_LOCK_EXTENSION: &str = {
                "UPDATE character_instance_lock SET extended = ? WHERE guid = ? AND mapId = ? AND lockId = ?"
            };
pub(super) const UPD_CHARACTER_INSTANCE_LOCK_FORCE_EXPIRE: &str = {
                "UPDATE character_instance_lock SET expiryTime = ?, extended = 0 WHERE guid = ? AND mapId = ? AND lockId = ?"
            };
pub(super) const DEL_INSTANCE: &str = "DELETE FROM instance WHERE instanceId = ?";
pub(super) const INS_INSTANCE: &str = {
                "INSERT INTO instance (instanceId, data, completedEncountersMask, entranceWorldSafeLocId) VALUES (?, ?, ?, ?)"
            };
pub(super) const SEL_RESPAWNS: &str = {
                "SELECT type, spawnId, respawnTime FROM respawn WHERE mapId = ? AND instanceId = ?"
            };
pub(super) const SEL_ALL_RESPAWNS: &str = {
                "SELECT type, spawnId, respawnTime, mapId, instanceId FROM respawn"
            };
pub(super) const REP_RESPAWN: &str = {
                "REPLACE INTO respawn (type, spawnId, respawnTime, mapId, instanceId) VALUES (?, ?, ?, ?, ?)"
            };
pub(super) const DEL_RESPAWN: &str = {
                "DELETE FROM respawn WHERE type = ? AND spawnId = ? AND mapId = ? AND instanceId = ?"
            };
pub(super) const DEL_ALL_RESPAWNS: &str = "DELETE FROM respawn WHERE mapId = ? AND instanceId = ?";
pub(super) const DEL_GAME_EVENT_SAVE: &str = "DELETE FROM game_event_save WHERE eventEntry = ?";
pub(super) const INS_GAME_EVENT_SAVE: &str = {
                "INSERT INTO game_event_save (eventEntry, state, next_start) VALUES (?, ?, ?)"
            };
pub(super) const SEL_GAME_EVENT_CONDITION_SAVES: &str = {
                "SELECT eventEntry, condition_id, done FROM game_event_condition_save"
            };
pub(super) const DEL_ALL_GAME_EVENT_CONDITION_SAVE: &str = {
                "DELETE FROM game_event_condition_save WHERE eventEntry = ?"
            };
pub(super) const DEL_GAME_EVENT_CONDITION_SAVE: &str = {
                "DELETE FROM game_event_condition_save WHERE eventEntry = ? AND condition_id = ?"
            };
pub(super) const INS_GAME_EVENT_CONDITION_SAVE: &str = {
                "INSERT INTO game_event_condition_save (eventEntry, condition_id, done) VALUES (?, ?, ?)"
            };
pub(super) const SEL_WORLD_STATE_VALUES: &str = "SELECT Id, Value FROM world_state_value";
pub(super) const REP_WORLD_STATE: &str = "REPLACE INTO world_state_value (Id, Value) VALUES (?, ?)";
pub(super) const REP_WORLD_VARIABLE: &str = "REPLACE INTO world_variable (Id, Value) VALUES (?, ?)";
pub(super) const UPD_ZONE: &str = "UPDATE characters SET zone = ? WHERE guid = ?";
pub(super) const UPD_CHARACTER_POSITION: &str = {
                "UPDATE characters SET position_x = ?, position_y = ?, position_z = ?, orientation = ?, map = ?, instance_id = ?, zone = ?, trans_x = 0, trans_y = 0, trans_z = 0, transguid = 0, taxi_path = '', cinematic = 1 WHERE guid = ?"
            };
pub(super) const UPD_CHARACTER_POSITION_BY_MAPID: &str = {
                "UPDATE characters SET position_x = ?, position_y = ?, position_z = ?, orientation = ?, map = ?, zone = ?, trans_x = 0, trans_y = 0, trans_z = 0, transguid = 0, taxi_path = '', cinematic = 1 WHERE guid = ? AND map = ?"
            };
pub(super) const UPD_CHARACTER_POSITION_PRESERVE_TRAVEL: &str = {
                "UPDATE characters SET position_x = ?, position_y = ?, position_z = ?, orientation = ?, map = ?, instance_id = ?, zone = ? WHERE guid = ?"
            };
pub(super) const SEL_CHAR_HOMEBIND: &str = {
                "SELECT mapId, zoneId, posX, posY, posZ, orientation FROM character_homebind WHERE guid = ?"
            };
pub(super) const UPD_CHAR_TAXI_PATH: &str = "UPDATE characters SET taxi_path = '' WHERE guid = ?";
pub(super) const UPD_CHAR_TAXIMASK: &str = "UPDATE characters SET taximask = ? WHERE guid = ?";
pub(super) const UPD_CHAR_DIFFICULTIES: &str = {
                "UPDATE characters SET dungeonDifficulty = ?, raidDifficulty = ?, legacyRaidDifficulty = ? WHERE guid = ?"
            };
pub(super) const UPD_CHAR_EXPLORED_ZONES: &str = {
                "UPDATE characters SET exploredZones = ? WHERE guid = ?"
            };
pub(super) const INS_PLAYER_HOMEBIND: &str = {
                "INSERT INTO character_homebind (guid, mapId, zoneId, posX, posY, posZ, orientation) VALUES (?, ?, ?, ?, ?, ?, ?)"
            };
pub(super) const UPD_PLAYER_HOMEBIND: &str = {
                "UPDATE character_homebind SET mapId = ?, zoneId = ?, posX = ?, posY = ?, posZ = ?, orientation = ? WHERE guid = ?"
            };
pub(super) const DEL_PLAYER_HOMEBIND: &str = "DELETE FROM character_homebind WHERE guid = ?";
pub(super) const SEL_CORPSES: &str = {
                "SELECT posX, posY, posZ, orientation, mapId, displayId, itemCache, race, class, gender, flags, dynFlags, time, corpseType, instanceId, guid FROM corpse WHERE mapId = ? AND instanceId = ?"
            };
pub(super) const INS_CORPSE: &str = {
                "INSERT INTO corpse (guid, posX, posY, posZ, orientation, mapId, displayId, itemCache, race, class, gender, flags, dynFlags, time, corpseType, instanceId) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            };
pub(super) const DEL_CORPSE: &str = "DELETE FROM corpse WHERE guid = ?";
pub(super) const DEL_CORPSES_FROM_MAP: &str = {
                "DELETE c, cc, cp FROM corpse c LEFT JOIN corpse_customizations cc ON c.guid = cc.ownerGuid LEFT JOIN corpse_phases cp ON c.guid = cp.OwnerGuid WHERE c.mapId = ? AND c.instanceId = ?"
            };
pub(super) const SEL_CORPSE_PHASES: &str = {
                "SELECT cp.OwnerGuid, cp.PhaseId FROM corpse_phases cp LEFT JOIN corpse c ON cp.OwnerGuid = c.guid WHERE c.mapId = ? AND c.instanceId = ?"
            };
pub(super) const INS_CORPSE_PHASES: &str = {
                "INSERT INTO corpse_phases (OwnerGuid, PhaseId) VALUES (?, ?)"
            };
pub(super) const DEL_CORPSE_PHASES: &str = "DELETE FROM corpse_phases WHERE OwnerGuid = ?";
pub(super) const SEL_CORPSE_CUSTOMIZATIONS: &str = {
                "SELECT cc.ownerGuid, cc.chrCustomizationOptionID, cc.chrCustomizationChoiceID FROM corpse_customizations cc LEFT JOIN corpse c ON cc.ownerGuid = c.guid WHERE c.mapId = ? AND c.instanceId = ? ORDER BY cc.ownerGuid, cc.chrCustomizationOptionID"
            };
pub(super) const INS_CORPSE_CUSTOMIZATIONS: &str = {
                "INSERT INTO corpse_customizations (ownerGuid, chrCustomizationOptionID, chrCustomizationChoiceID) VALUES (?, ?, ?)"
            };
pub(super) const DEL_CORPSE_CUSTOMIZATIONS: &str = {
                "DELETE FROM corpse_customizations WHERE ownerGuid = ?"
            };
pub(super) const SEL_CORPSE_LOCATION: &str = {
                "SELECT mapId, posX, posY, posZ, orientation FROM corpse WHERE guid = ?"
            };
