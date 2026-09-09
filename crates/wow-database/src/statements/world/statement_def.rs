//! The `StatementDef` impl for the world statements; a trait impl cannot be split.
//!
//! Separated from the world.rs root under #652. Behaviour is preserved.

use super::*;

impl StatementDef for WorldStatements {
    fn database() -> crate::persistence_trace::LogicalDatabase {
        crate::persistence_trace::LogicalDatabase::World
    }

    fn sql(self) -> &'static str {
        match self {
            Self::DEL_LINKED_RESPAWN => {
                "DELETE FROM linked_respawn WHERE guid = ? AND linkType  = ?"
            }
            Self::DEL_LINKED_RESPAWN_MASTER => {
                "DELETE FROM linked_respawn WHERE linkedGuid = ? AND linkType = ?"
            }
            Self::REP_LINKED_RESPAWN => {
                "REPLACE INTO linked_respawn (guid, linkedGuid, linkType) VALUES (?, ?, ?)"
            }
            Self::SEL_LINKED_RESPAWNS => {
                "SELECT guid, linkedGuid, linkType FROM linked_respawn ORDER BY guid ASC"
            }
            Self::SEL_CREATURE_TEXT => {
                "SELECT CreatureID, GroupID, ID, Text, Type, Language, Probability, Emote, Duration, Sound, SoundPlayType, BroadcastTextId, TextRange FROM creature_text"
            }
            Self::SEL_SMART_SCRIPTS => concat!(
                "SELECT entryorguid, source_type, id, link, Difficulties, event_type, event_phase_mask, event_chance, event_flags, ",
                "event_param1, event_param2, event_param3, event_param4, event_param5, event_param_string, ",
                "action_type, action_param1, action_param2, action_param3, action_param4, action_param5, action_param6, action_param7, ",
                "target_type, target_param1, target_param2, target_param3, target_param4, target_x, target_y, target_z, target_o ",
                "FROM smart_scripts ORDER BY entryorguid, source_type, id, link",
            ),
            Self::DEL_GAMEOBJECT => "DELETE FROM gameobject WHERE guid = ?",
            Self::DEL_EVENT_GAMEOBJECT => "DELETE FROM game_event_gameobject WHERE guid = ?",
            Self::SEL_WORLD_SAFE_LOCS => {
                "SELECT ID, MapID, LocX, LocY, LocZ, Facing FROM world_safe_locs"
            }
            Self::SEL_GRAVEYARD_ZONE => "SELECT ID, GhostZone FROM graveyard_zone",
            Self::INS_GRAVEYARD_ZONE => "INSERT INTO graveyard_zone (ID, GhostZone) VALUES (?, ?)",
            Self::DEL_GRAVEYARD_ZONE => "DELETE FROM graveyard_zone WHERE ID = ? AND GhostZone = ?",
            Self::INS_GAME_TELE => {
                "INSERT INTO game_tele (id, position_x, position_y, position_z, orientation, map, name) VALUES (?, ?, ?, ?, ?, ?, ?)"
            }
            Self::DEL_GAME_TELE => "DELETE FROM game_tele WHERE name = ?",
            Self::SEL_GAME_TELE => {
                "SELECT id, position_x, position_y, position_z, orientation, map, name FROM game_tele"
            }
            Self::SEL_MOUNT_DEFINITIONS => {
                "SELECT spellId, otherFactionSpellId FROM mount_definitions"
            }
            Self::INS_NPC_VENDOR => {
                "INSERT INTO npc_vendor (entry, item, maxcount, incrtime, extendedcost, type) VALUES(?, ?, ?, ?, ?, ?)"
            }
            Self::DEL_NPC_VENDOR => {
                "DELETE FROM npc_vendor WHERE entry = ? AND item = ? AND type = ?"
            }
            Self::SEL_NPC_VENDOR_REF => {
                "SELECT item, maxcount, incrtime, ExtendedCost, type, BonusListIDs, PlayerConditionID, IgnoreFiltering FROM npc_vendor WHERE entry = ? ORDER BY slot ASC"
            }
            Self::SEL_NPC_VENDORS_ALL => {
                "SELECT entry, item, maxcount, incrtime, ExtendedCost, type, BonusListIDs, PlayerConditionID, IgnoreFiltering FROM npc_vendor ORDER BY entry, slot ASC"
            }
            // Cols: 0=item, 1=maxcount, 2=ExtendedCost, 3=type, 4=slot,
            //       5=BuyPrice, 6=SellPrice, 7=MaxDurability, 8=VendorStackCount,
            //       9=IgnoreFiltering, 10=incrtime, 11=PlayerConditionID,
            //       12=HasVendorConditions. Param 0 is root creature entry for
            //       CONDITION_SOURCE_TYPE_NPC_VENDOR; param 1 is the expanded
            //       npc_vendor entry being read.
            Self::SEL_VENDOR_ITEMS => concat!(
                "SELECT nv.item, nv.maxcount, nv.ExtendedCost, nv.type, nv.slot, ",
                "COALESCE(isp.BuyPrice, 0), COALESCE(isp.SellPrice, 0), ",
                "COALESCE(isp.MaxDurability, 0), COALESCE(isp.VendorStackCount, 1), ",
                "nv.IgnoreFiltering, nv.incrtime, nv.PlayerConditionID, ",
                "EXISTS(SELECT 1 FROM conditions c ",
                "WHERE c.SourceTypeOrReferenceId = 23 AND c.SourceGroup = ? ",
                "AND c.SourceEntry = nv.item AND c.SourceId = 0) ",
                "FROM npc_vendor nv ",
                "LEFT JOIN hotfixes.item_sparse isp ON nv.item = isp.ID ",
                "WHERE nv.entry = ? ORDER BY nv.slot ASC"
            ),
            Self::UPD_CREATURE_MOVEMENT_TYPE => {
                "UPDATE creature SET MovementType = ? WHERE guid = ?"
            }
            Self::UPD_CREATURE_FACTION => {
                "UPDATE creature_template SET faction = ? WHERE entry = ?"
            }
            Self::UPD_CREATURE_NPCFLAG => {
                "UPDATE creature_template SET npcflag = ? WHERE entry = ?"
            }
            Self::UPD_CREATURE_POSITION => {
                "UPDATE creature SET position_x = ?, position_y = ?, position_z = ?, orientation = ? WHERE guid = ?"
            }
            Self::UPD_CREATURE_MAP_POSITION => {
                "UPDATE creature SET map = ?, position_x = ?, position_y = ?, position_z = ?, orientation = ? WHERE guid = ?"
            }
            Self::UPD_CREATURE_WANDER_DISTANCE => {
                "UPDATE creature SET wander_distance = ?, MovementType = ? WHERE guid = ?"
            }
            Self::UPD_CREATURE_SPAWN_TIME_SECS => {
                "UPDATE creature SET spawntimesecs = ? WHERE guid = ?"
            }
            Self::INS_CREATURE_FORMATION => {
                "INSERT INTO creature_formations (leaderGUID, memberGUID, dist, angle, groupAI) VALUES (?, ?, ?, ?, ?)"
            }
            Self::SEL_WAYPOINT_PATHS => "SELECT PathId, MoveType, Flags FROM waypoint_path",
            Self::SEL_WAYPOINT_PATH_NODES => {
                "SELECT PathId, NodeId, PositionX, PositionY, PositionZ, Orientation, Delay FROM waypoint_path_node ORDER BY PathId, NodeId"
            }
            Self::SEL_WAYPOINT_PATH_BY_PATHID => {
                "SELECT PathId, MoveType, Flags FROM waypoint_path WHERE PathId = ?"
            }
            Self::INS_WAYPOINT_PATH_NODE => {
                "INSERT INTO waypoint_path_node (PathId, NodeId, PositionX, PositionY, PositionZ, Orientation) VALUES (?, ?, ?, ?, ?, ?)"
            }
            Self::DEL_WAYPOINT_PATH_NODE => {
                "DELETE FROM waypoint_path_node WHERE PathId = ? AND NodeId = ?"
            }
            Self::UPD_WAYPOINT_PATH_NODE => {
                "UPDATE waypoint_path_node SET NodeId = NodeId - 1 WHERE PathId = ? AND NodeId > ?"
            }
            Self::UPD_WAYPOINT_PATH_NODE_POSITION => {
                "UPDATE waypoint_path_node SET PositionX = ?, PositionY = ?, PositionZ = ?, Orientation = ? WHERE PathId = ? AND NodeId = ?"
            }
            Self::SEL_WAYPOINT_PATH_NODE_MAX_PATHID => "SELECT MAX(PathId) FROM waypoint_path_node",
            Self::SEL_WAYPOINT_PATH_NODE_BY_PATHID => {
                "SELECT PathId, NodeId, PositionX, PositionY, PositionZ, Orientation, Delay FROM waypoint_path_node WHERE PathId = ? ORDER BY NodeId"
            }
            Self::SEL_WAYPOINT_PATH_NODE_POS_BY_PATHID => {
                "SELECT NodeId, PositionX, PositionY, PositionZ, Orientation FROM waypoint_path_node WHERE PathId = ?"
            }
            Self::SEL_WAYPOINT_PATH_NODE_POS_FIRST_BY_PATHID => {
                "SELECT PositionX, PositionY, PositionZ, Orientation FROM waypoint_path_node WHERE NodeId = 1 AND PathId = ?"
            }
            Self::SEL_WAYPOINT_PATH_NODE_POS_LAST_BY_PATHID => {
                "SELECT PositionX, PositionY, PositionZ, Orientation FROM waypoint_path_node WHERE PathId = ? ORDER BY NodeId DESC LIMIT 1"
            }
            Self::SEL_WAYPOINT_PATH_NODE_MAX_NODEID => {
                "SELECT MAX(NodeId) FROM waypoint_path_node WHERE PathId = ?"
            }
            Self::SEL_WAYPOINT_PATH_NODE_BY_POS => {
                "SELECT PathId, NodeId FROM waypoint_path_node WHERE (abs(PositionX - ?) <= ?) and (abs(PositionY - ?) <= ?) and (abs(PositionZ - ?) <= ?)"
            }
            Self::UPD_CREATURE_ADDON_PATH => "UPDATE creature_addon SET PathId = ? WHERE guid = ?",
            Self::INS_CREATURE_ADDON => "INSERT INTO creature_addon(guid, PathId) VALUES (?, ?)",
            Self::DEL_CREATURE_ADDON => "DELETE FROM creature_addon WHERE guid = ?",
            Self::SEL_CREATURE_ADDON_BY_GUID => "SELECT guid FROM creature_addon WHERE guid = ?",
            Self::DEL_CREATURE => "DELETE FROM creature WHERE guid = ?",
            Self::SEL_COMMANDS => "SELECT name, help FROM command",
            Self::SEL_PLAYER_XP_FOR_LEVEL => {
                "SELECT Level, Experience FROM player_xp_for_level ORDER BY Level"
            }
            Self::SEL_EXPLORATION_BASE_XP => "SELECT level, basexp FROM exploration_basexp",
            Self::SEL_CREATURE_TEMPLATE => concat!(
                "SELECT entry, KillCredit1, KillCredit2, name, femaleName, subname, TitleAlt, IconName, ",
                "RequiredExpansion, VignetteID, faction, npcflag, speed_walk, speed_run, scale, Classification, ",
                "dmgschool, BaseAttackTime, RangeAttackTime, BaseVariance, RangeVariance, unit_class, unit_flags, ",
                "unit_flags2, unit_flags3, family, trainer_class, type, PetSpellDataId, VehicleId, AIName, ",
                "MovementType, ctm.Ground, ctm.Swim, ctm.Flight, ctm.Rooted, ctm.Chase, ctm.Random, ",
                "ctm.InteractionPauseTimer, ExperienceModifier, Civilian, RacialLeader, movementId, WidgetSetID, ",
                "WidgetSetUnitConditionID, RegenHealth, mechanic_immune_mask, spell_school_immune_mask, flags_extra, ",
                "ScriptName, StringId FROM creature_template ct ",
                "LEFT JOIN creature_template_movement ctm ON ct.entry = ctm.CreatureId WHERE entry = ? OR 1 = ?",
            ),
            Self::SEL_CREATURE_TEMPLATE_IDS => "SELECT entry FROM creature_template",
            Self::SEL_CREATURE_SPAWN_IDS => "SELECT guid, id FROM creature",
            Self::SEL_GAMEOBJECT_SPAWN_IDS => "SELECT guid, id FROM gameobject",
            Self::SEL_VALID_GAME_EVENT_IDS => {
                "SELECT eventEntry FROM game_event WHERE eventEntry <> 0 AND (`length` > 0 OR world_event > 0)"
            }
            Self::SEL_WORLD_STATE_IDS => "SELECT ID FROM world_state",
            Self::SEL_WORLD_STATES => {
                "SELECT ID, DefaultValue, MapIDs, AreaIDs, ScriptName FROM world_state"
            }
            Self::SEL_LOGIN_TRANSPORTS => concat!(
                "SELECT t.guid, t.entry, t.phaseUseFlags, t.phaseid, t.phasegroup, ",
                "gt.displayId, gt.size, gt.Data0, gt.Data1, gt.Data2, gt.Data8, ",
                "COALESCE(goo.flags, gta.flags, 0), COALESCE(goo.faction, gta.faction, 0) ",
                "FROM transports t ",
                "JOIN gameobject_template gt ON gt.entry = t.entry ",
                "LEFT JOIN gameobject_template_addon gta ON gta.entry = t.entry ",
                "LEFT JOIN gameobject_overrides goo ON goo.spawnId = t.guid ",
                "WHERE gt.type = 15 ORDER BY t.guid"
            ),
            Self::SEL_LOGIN_TRANSPORT_BY_GUID => concat!(
                "SELECT t.guid, t.entry, t.phaseUseFlags, t.phaseid, t.phasegroup, ",
                "gt.displayId, gt.size, gt.Data0, gt.Data1, gt.Data2, gt.Data8, ",
                "COALESCE(goo.flags, gta.flags, 0), COALESCE(goo.faction, gta.faction, 0) ",
                "FROM transports t ",
                "JOIN gameobject_template gt ON gt.entry = t.entry ",
                "LEFT JOIN gameobject_template_addon gta ON gta.entry = t.entry ",
                "LEFT JOIN gameobject_overrides goo ON goo.spawnId = t.guid ",
                "WHERE gt.type = 15 AND t.guid = ? LIMIT 1"
            ),
            Self::SEL_WORLD_DB_VERSION => "SELECT db_version, cache_id FROM version LIMIT 1",
            Self::SEL_REPUTATION_REWARD_RATE => {
                "SELECT faction, quest_rate, quest_daily_rate, quest_weekly_rate, quest_monthly_rate, quest_repeatable_rate, creature_rate, spell_rate FROM reputation_reward_rate"
            }
            Self::SEL_CREATURE_ONKILL_REPUTATION => {
                "SELECT creature_id, RewOnKillRepFaction1, RewOnKillRepFaction2, IsTeamAward1, MaxStanding1, RewOnKillRepValue1, IsTeamAward2, MaxStanding2, RewOnKillRepValue2, TeamDependent FROM creature_onkill_reputation"
            }
            Self::SEL_REPUTATION_SPILLOVER_TEMPLATE => {
                "SELECT faction, faction1, rate_1, rank_1, faction2, rate_2, rank_2, faction3, rate_3, rank_3, faction4, rate_4, rank_4, faction5, rate_5, rank_5 FROM reputation_spillover_template"
            }
            Self::SEL_CREATURE_BY_ID => "SELECT guid FROM creature WHERE id = ?",
            Self::SEL_CREATURE_ENTRY_BY_GUID => "SELECT id FROM creature WHERE guid = ?",
            Self::SEL_GAMEOBJECT_NEAREST => {
                "SELECT guid, id, position_x, position_y, position_z, map, (POW(position_x - ?, 2) + POW(position_y - ?, 2) + POW(position_z - ?, 2)) AS order_ FROM gameobject WHERE map = ? AND (POW(position_x - ?, 2) + POW(position_y - ?, 2) + POW(position_z - ?, 2)) <= ? ORDER BY order_"
            }
            Self::SEL_CREATURE_NEAREST => {
                "SELECT guid, id, position_x, position_y, position_z, map, (POW(position_x - ?, 2) + POW(position_y - ?, 2) + POW(position_z - ?, 2)) AS order_ FROM creature WHERE map = ? AND (POW(position_x - ?, 2) + POW(position_y - ?, 2) + POW(position_z - ?, 2)) <= ? ORDER BY order_"
            }
            Self::SEL_GAMEOBJECT_TARGET => "", // C++ enum exists, but WorldDatabase.cpp does not prepare it.
            Self::INS_CREATURE => concat!(
                "INSERT INTO creature (guid, id , map, spawnDifficulties, PhaseId, PhaseGroup, modelid, equipment_id, ",
                "position_x, position_y, position_z, orientation, spawntimesecs, wander_distance, currentwaypoint, ",
                "curhealth, curmana, MovementType, npcflag, unit_flags, unit_flags2, unit_flags3) ",
                "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            ),
            Self::DEL_GAME_EVENT_CREATURE => "DELETE FROM game_event_creature WHERE guid = ?",
            Self::DEL_GAME_EVENT_MODEL_EQUIP => "DELETE FROM game_event_model_equip WHERE guid = ?",
            Self::INS_GAMEOBJECT => concat!(
                "INSERT INTO gameobject (guid, id, map, spawnDifficulties, PhaseId, PhaseGroup, ",
                "position_x, position_y, position_z, orientation, rotation0, rotation1, rotation2, rotation3, ",
                "spawntimesecs, animprogress, state) ",
                "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            ),
            Self::SEL_DISABLES => "SELECT entry FROM disables WHERE entry = ? AND sourceType = ?",
            Self::INS_DISABLES => {
                "INSERT INTO disables (entry, sourceType, flags, comment) VALUES (?, ?, ?, ?)"
            }
            Self::DEL_DISABLES => "DELETE FROM disables WHERE entry = ? AND sourceType = ?",
            Self::UPD_CREATURE_ZONE_AREA_DATA => {
                "UPDATE creature SET zoneId = ?, areaId = ? WHERE guid = ?"
            }
            Self::UPD_GAMEOBJECT_ZONE_AREA_DATA => {
                "UPDATE gameobject SET zoneId = ?, areaId = ? WHERE guid = ?"
            }
            Self::DEL_SPAWNGROUP_MEMBER => {
                "DELETE FROM spawn_group WHERE spawnType = ? AND spawnId = ?"
            }
            Self::DEL_GAMEOBJECT_ADDON => "DELETE FROM gameobject_addon WHERE guid = ?",
            Self::SEL_GUILD_REWARDS_REQ_ACHIEVEMENTS => {
                "SELECT AchievementRequired FROM guild_rewards_req_achievements WHERE ItemID = ?"
            }
            Self::INS_CONDITION => concat!(
                "INSERT INTO conditions (SourceTypeOrReferenceId, SourceGroup, SourceEntry, SourceId, ElseGroup, ",
                "ConditionTypeOrReference, ConditionTarget, ConditionValue1, ConditionValue2, ConditionValue3, ",
                "NegativeCondition, ErrorType, ErrorTextId, ScriptName, Comment) ",
                "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            ),
            Self::SEL_CREATURES_IN_RANGE => concat!(
                "SELECT c.guid, c.id, c.position_x, c.position_y, c.position_z, c.orientation, ",
                "c.curhealth, c.curmana, c.modelid, ",
                "ctdiff.MinLevel, ctdiff.MaxLevel, ",
                "ct.faction, ct.npcflag, ",
                "ct.unit_flags, ct.unit_flags2, ct.unit_flags3, ",
                "ct.speed_walk, ct.speed_run, ct.scale, ct.unit_class, ",
                "ct.flags_extra, ",
                "ct.BaseAttackTime, ct.RangeAttackTime, ",
                "ctm.CreatureDisplayID, ",
                "ctdiff.LootID, ctdiff.SkinLootID, ctdiff.GoldMin, ctdiff.GoldMax, ",
                "c.phaseUseFlags, c.phaseid, c.phasegroup, c.terrainSwapMap, ",
                "COALESCE(cmo.Ground, ctmv.Ground, 1), COALESCE(cmo.Swim, ctmv.Swim, 1), COALESCE(cmo.Flight, ctmv.Flight, 0), ",
                "COALESCE(cmo.Rooted, ctmv.Rooted, 0), COALESCE(cmo.Chase, ctmv.Chase, 0), COALESCE(cmo.Random, ctmv.Random, 0), ",
                "COALESCE(cmo.InteractionPauseTimer, ctmv.InteractionPauseTimer, 180000), ",
                "c.wander_distance, ",
                "CASE WHEN ca.guid IS NOT NULL AND ca.PathId = 0 AND c.MovementType = 2 THEN 0 ELSE c.MovementType END, ",
                "COALESCE(ca.PathId, cta.PathId, 0), ",
                "COALESCE(ctm.DisplayScale, 1.0), ",
                "ct.Classification, ct.RegenHealth, ",
                "c.npcflag, c.unit_flags, c.unit_flags2, c.unit_flags3, ",
                "c.equipment_id, c.spawntimesecs, c.spawnDifficulties, c.ScriptName, c.StringId, ct.VehicleId ",
                "FROM creature c ",
                "JOIN creature_template ct ON c.id = ct.entry ",
                "LEFT JOIN creature_template_difficulty ctdiff ON ct.entry = ctdiff.Entry AND ctdiff.DifficultyID = 0 ",
                "LEFT JOIN creature_template_model ctm ON ct.entry = ctm.CreatureID AND ctm.Idx = 0 ",
                "LEFT JOIN creature_template_movement ctmv ON ct.entry = ctmv.CreatureId ",
                "LEFT JOIN creature_movement_override cmo ON c.guid = cmo.SpawnId ",
                "LEFT JOIN creature_addon ca ON ca.guid = c.guid ",
                "LEFT JOIN creature_template_addon cta ON cta.entry = c.id ",
                "WHERE c.map = ? AND c.position_x BETWEEN ? AND ? AND c.position_y BETWEEN ? AND ?",
            ),
            Self::SEL_CREATURE_SPAWNS => concat!(
                "SELECT creature.guid, id, map, position_x, position_y, position_z, orientation, modelid, equipment_id, spawntimesecs, wander_distance, ",
                "currentwaypoint, curhealth, curmana, MovementType, spawnDifficulties, eventEntry, poolSpawnId, creature.npcflag, creature.unit_flags, creature.unit_flags2, creature.unit_flags3, ",
                "creature.phaseUseFlags, creature.phaseid, creature.phasegroup, creature.terrainSwapMap, creature.ScriptName, creature.StringId, ",
                "COALESCE(cmo.Ground, ctm.Ground, 1), COALESCE(cmo.Swim, ctm.Swim, 1), COALESCE(cmo.Flight, ctm.Flight, 0), ",
                "COALESCE(cmo.Rooted, ctm.Rooted, 0), COALESCE(cmo.Chase, ctm.Chase, 0), COALESCE(cmo.Random, ctm.Random, 0), ",
                "COALESCE(cmo.InteractionPauseTimer, ctm.InteractionPauseTimer, 180000) ",
                "FROM creature ",
                "LEFT JOIN creature_template_movement ctm ON creature.id = ctm.CreatureId ",
                "LEFT JOIN creature_movement_override cmo ON creature.guid = cmo.SpawnId ",
                "LEFT OUTER JOIN game_event_creature ON creature.guid = game_event_creature.guid ",
                "LEFT OUTER JOIN pool_members ON pool_members.type = 0 AND creature.guid = pool_members.spawnId",
            ),
            Self::SEL_CREATURE_FORMATIONS => {
                "SELECT leaderGUID, memberGUID, dist, angle, groupAI, point_1, point_2 FROM creature_formations ORDER BY leaderGUID"
            }
            Self::SEL_CREATURE_QUERY_RESPONSE => concat!(
                "SELECT ct.entry, ct.name, ct.femaleName, ct.subname, ct.TitleAlt, ct.IconName, ",
                "ct.type, ct.family, ct.Classification, ct.KillCredit1, ct.KillCredit2, ",
                "ct.Civilian, ct.RacialLeader, ct.movementId, ct.RequiredExpansion, ct.VignetteID, ",
                "ct.unit_class, ct.WidgetSetID, ct.WidgetSetUnitConditionID, ",
                "ctdiff.HealthModifier, ctdiff.ManaModifier, ctdiff.CreatureDifficultyID, ",
                "ctdiff.TypeFlags, ctdiff.TypeFlags2 ",
                "FROM creature_template ct ",
                "LEFT JOIN creature_template_difficulty ctdiff ON ct.entry = ctdiff.Entry AND ctdiff.DifficultyID = 0 ",
                "WHERE ct.entry = ?",
            ),
            Self::SEL_CREATURE_DISPLAY_MODELS => concat!(
                "SELECT CreatureDisplayID, DisplayScale, Probability ",
                "FROM creature_template_model WHERE CreatureID = ? ORDER BY Idx",
            ),
            Self::SEL_GAMEOBJECTS_IN_RANGE => concat!(
                "SELECT g.guid, g.id, g.position_x, g.position_y, g.position_z, g.orientation, ",
                "g.rotation0, g.rotation1, g.rotation2, g.rotation3, ",
                "g.animprogress, g.state, ",
                "gt.type, gt.displayId, gt.name, gt.size, ",
                "gt.Data0, gt.Data1, gt.Data2, gt.Data3, gt.Data4, gt.Data5, gt.Data6, gt.Data7, ",
                "gt.Data8, gt.Data9, gt.Data10, gt.Data11, gt.Data12, gt.Data13, gt.Data14, gt.Data15, ",
                "gt.Data16, gt.Data17, gt.Data18, gt.Data19, gt.Data20, gt.Data21, gt.Data22, gt.Data23, ",
                "gt.Data24, gt.Data25, gt.Data26, gt.Data27, gt.Data28, gt.Data29, gt.Data30, gt.Data31, ",
                "gt.Data32, gt.Data33, gt.Data34, ",
                "g.phaseUseFlags, g.phaseid, g.phasegroup, g.terrainSwapMap, ",
                "COALESCE(goo.flags, gta.flags, 0), COALESCE(goo.faction, gta.faction, 0), ",
                "CASE WHEN goo.spawnId IS NOT NULL OR gta.entry IS NOT NULL THEN 1 ELSE 0 END, ",
                // #NEXT.R8.ENTITIES.1216: GameObjectData.ParentRotation from per-spawn
                // gameobject_addon (GameObject::Create, GameObject.cpp:1003-1008). NULL (no
                // addon row) -> identity quaternion in the reader.
                "ga.parent_rotation0, ga.parent_rotation1, ga.parent_rotation2, ga.parent_rotation3 ",
                "FROM gameobject g ",
                "JOIN gameobject_template gt ON g.id = gt.entry ",
                "LEFT JOIN gameobject_template_addon gta ON gta.entry = g.id ",
                "LEFT JOIN gameobject_overrides goo ON goo.spawnId = g.guid ",
                "LEFT JOIN gameobject_addon ga ON ga.guid = g.guid ",
                "WHERE g.map = ? AND g.position_x BETWEEN ? AND ? AND g.position_y BETWEEN ? AND ?",
            ),
            Self::SEL_GAMEOBJECT_SPAWNS => concat!(
                "SELECT gameobject.guid, id, map, position_x, position_y, position_z, orientation, ",
                "rotation0, rotation1, rotation2, rotation3, spawntimesecs, animprogress, state, spawnDifficulties, eventEntry, poolSpawnId, ",
                "phaseUseFlags, phaseid, phasegroup, terrainSwapMap, ScriptName, StringId ",
                "FROM gameobject LEFT OUTER JOIN game_event_gameobject ON gameobject.guid = game_event_gameobject.guid ",
                "LEFT OUTER JOIN pool_members ON pool_members.type = 1 AND gameobject.guid = pool_members.spawnId",
            ),
            Self::SEL_AREATRIGGER_SPAWNS => {
                "SELECT SpawnId, AreaTriggerCreatePropertiesId, IsCustom, MapId, SpawnDifficulties, PosX, PosY, PosZ, Orientation, PhaseUseFlags, PhaseId, PhaseGroup, SpellForVisuals, ScriptName FROM `areatrigger`"
            }
            Self::SEL_TERRAIN_WORLD_MAPS => {
                "SELECT TerrainSwapMap, UiMapPhaseId FROM `terrain_worldmap`"
            }
            Self::SEL_TERRAIN_SWAP_DEFAULTS => {
                "SELECT MapId, TerrainSwapMap FROM `terrain_swap_defaults`"
            }
            Self::SEL_PHASE_AREAS => "SELECT AreaId, PhaseId FROM `phase_area`",
            Self::SEL_SPAWN_GROUP_TEMPLATES => {
                "SELECT groupId, groupName, groupFlags FROM spawn_group_template"
            }
            Self::SEL_SPAWN_GROUP_MEMBERS => "SELECT groupId, spawnType, spawnId FROM spawn_group",
            Self::SEL_POOL_TEMPLATES => "SELECT entry, max_limit FROM pool_template",
            Self::SEL_POOL_MEMBERS_BY_TYPE => {
                "SELECT spawnId, poolSpawnId, chance FROM pool_members WHERE type = ?"
            }
            Self::SEL_POOL_AUTOSPAWN_CANDIDATES => concat!(
                "SELECT DISTINCT pool_template.entry, pool_members.spawnId, pool_members.poolSpawnId FROM pool_template",
                " LEFT JOIN game_event_pool ON pool_template.entry = game_event_pool.pool_entry",
                " LEFT JOIN pool_members ON pool_members.type = 2 AND pool_template.entry = pool_members.spawnId WHERE game_event_pool.pool_entry IS NULL",
            ),
            Self::SEL_MAX_GAME_EVENT_ENTRY => "SELECT MAX(eventEntry) FROM game_event",
            Self::SEL_GAME_EVENTS => {
                "SELECT eventEntry, UNIX_TIMESTAMP(start_time), UNIX_TIMESTAMP(end_time), occurence, length, holiday, holidayStage, description, world_event, announce FROM game_event"
            }
            Self::SEL_GAME_EVENT_PREREQUISITES => {
                "SELECT eventEntry, prerequisite_event FROM game_event_prerequisite"
            }
            Self::SEL_GAME_EVENT_CONDITIONS => {
                "SELECT eventEntry, condition_id, req_num, max_world_state_field, done_world_state_field FROM game_event_condition"
            }
            Self::SEL_GAME_EVENT_QUEST_CONDITIONS => {
                "SELECT quest, eventEntry, condition_id, num FROM game_event_quest_condition"
            }
            Self::SEL_GAME_EVENT_POOLS => concat!(
                "SELECT pool_template.entry, game_event_pool.eventEntry FROM pool_template",
                " JOIN game_event_pool ON pool_template.entry = game_event_pool.pool_entry",
            ),
            Self::SEL_GAME_EVENT_CREATURES => "SELECT guid, eventEntry FROM game_event_creature",
            Self::SEL_GAME_EVENT_GAMEOBJECTS => {
                "SELECT guid, eventEntry FROM game_event_gameobject"
            }
            Self::SEL_CREATURE_EQUIP_TEMPLATE_IDS => {
                "SELECT CreatureID, ID FROM creature_equip_template"
            }
            Self::SEL_GAME_EVENT_MODEL_EQUIP => concat!(
                "SELECT creature.guid, creature.id, game_event_model_equip.eventEntry, ",
                "game_event_model_equip.modelid, game_event_model_equip.equipment_id ",
                "FROM creature JOIN game_event_model_equip ON creature.guid = game_event_model_equip.guid",
            ),
            Self::SEL_GAME_EVENT_CREATURE_QUESTS => {
                "SELECT id, quest, eventEntry FROM game_event_creature_quest"
            }
            Self::SEL_GAME_EVENT_GAMEOBJECT_QUESTS => {
                "SELECT id, quest, eventEntry FROM game_event_gameobject_quest"
            }
            Self::SEL_GAME_EVENT_NPC_FLAGS => {
                "SELECT guid, eventEntry, npcflag FROM game_event_npcflag"
            }
            Self::SEL_GAME_EVENT_NPC_VENDOR => concat!(
                "SELECT eventEntry, guid, item, maxcount, incrtime, ExtendedCost, type, ",
                "BonusListIDs, PlayerConditionId, IgnoreFiltering FROM game_event_npc_vendor ",
                "ORDER BY guid, slot ASC",
            ),
            Self::SEL_NPC_SPELLCLICK_SPELLS => {
                "SELECT npc_entry, spell_id, cast_flags, user_type FROM npc_spellclick_spells"
            }
            Self::SEL_LFG_DUNGEON_TEMPLATES => {
                "SELECT dungeonId, position_x, position_y, position_z, orientation, requiredItemLevel FROM lfg_dungeon_template"
            }
            Self::SEL_LFG_DUNGEON_REWARDS => {
                "SELECT dungeonId, maxLevel, firstQuestId, otherQuestId FROM lfg_dungeon_rewards ORDER BY dungeonId, maxLevel ASC"
            }
            Self::SEL_INSTANCE_SPAWN_GROUPS => {
                "SELECT instanceMapId, bossStateId, bossStates, spawnGroupId, flags FROM instance_spawn_groups"
            }
            Self::SEL_ITEM_INVENTORY_TYPE => {
                "SELECT InventoryType FROM item_template WHERE entry = ?"
            }
            Self::SEL_PLAYER_RACESTATS => {
                "SELECT race, str, agi, sta, inte, spi FROM player_racestats"
            }
            Self::SEL_PLAYER_CLASSLEVELSTATS => {
                "SELECT class, level, str, agi, sta, inte, spi FROM player_classlevelstats"
            }
            Self::SEL_PLAYER_CREATEINFO_ACTION => {
                "SELECT race, class, button, action, Type FROM playercreateinfo_action"
            }
            Self::SEL_PLAYER_CREATEINFO => concat!(
                "SELECT p.race, p.class, p.map, p.position_x, p.position_y, p.position_z, p.orientation, ",
                "p.npe_map, p.npe_position_x, p.npe_position_y, p.npe_position_z, ",
                "p.npe_orientation, p.npe_transport_guid, ",
                "(SELECT t.entry FROM transports t WHERE t.guid = p.npe_transport_guid LIMIT 1) ",
                "FROM playercreateinfo p",
            ),
            Self::SEL_PLAYER_CREATEINFO_CAST_SPELL => {
                "SELECT raceMask, classMask, spell, createMode FROM playercreateinfo_cast_spell"
            }
            Self::SEL_PLAYER_CREATEINFO_CUSTOM_SPELL => {
                "SELECT racemask, classmask, Spell FROM playercreateinfo_spell_custom"
            }
            Self::SEL_GAMEOBJECT_TEMPLATE_BY_ENTRY => concat!(
                "SELECT entry, type, displayId, name, IconName, castBarCaption, unk1, ",
                "size, Data0, Data1, Data2, Data3, Data4, Data5, Data6, Data7, ",
                "Data8, Data9, Data10, Data11, Data12, Data13, Data14, Data15, ",
                "Data16, Data17, Data18, Data19, Data20, Data21, Data22, Data23, ",
                "Data24, Data25, Data26, Data27, Data28, Data29, Data30, Data31, ",
                "Data32, Data33, Data34, ContentTuningId ",
                "FROM gameobject_template WHERE entry = ?",
            ),
            Self::SEL_GAMEOBJECT_TEMPLATE_LOCALE => {
                "SELECT Name, CastBarCaption, Unk1 FROM gameobject_template_locale WHERE entry = ? AND locale = ?"
            }
            Self::SEL_GAMEOBJECT_QUEST_ITEMS => {
                "SELECT ItemId FROM gameobject_questitem WHERE GameObjectEntry = ? ORDER BY Idx ASC"
            }
            Self::SEL_GAMEOBJECT_QUEST_ITEM_ROWS => {
                "SELECT GameObjectEntry, ItemId, Idx FROM gameobject_questitem ORDER BY Idx ASC"
            }
            Self::SEL_CREATURE_QUEST_ITEM_ROWS => {
                "SELECT CreatureEntry, DifficultyID, ItemId, Idx FROM creature_questitem ORDER BY Idx ASC"
            }
            Self::SEL_PAGE_TEXT => {
                "SELECT ID, `Text`, NextPageID, PlayerConditionID, Flags FROM page_text WHERE ID = ?"
            }
            Self::SEL_PAGE_TEXT_LOCALE => {
                "SELECT `Text` FROM page_text_locale WHERE ID = ? AND locale = ?"
            }
            Self::SEL_GAMEOBJECT_TEMPLATE_IDS => "SELECT entry FROM gameobject_template",
            Self::SEL_CREATURE_GOSSIP_MENU => {
                "SELECT MenuID FROM creature_template_gossip WHERE CreatureID = ?"
            }
            Self::SEL_GOSSIP_MENU => "SELECT TextID FROM gossip_menu WHERE MenuID = ? LIMIT 1",
            Self::SEL_GOSSIP_MENU_TEXTS => "SELECT TextID FROM gossip_menu WHERE MenuID = ?",
            Self::SEL_GOSSIP_MENUS => "SELECT MenuID, TextID FROM gossip_menu",
            Self::SEL_NPC_TEXT => "SELECT BroadcastTextID0 FROM npc_text WHERE ID = ? LIMIT 1",
            Self::SEL_GOSSIP_MENU_OPTIONS => concat!(
                "SELECT MenuID, GossipOptionID, OptionID, OptionNpc, OptionText, OptionBroadcastTextID, ",
                "Language, Flags, ActionMenuID, ActionPoiID, GossipNpcOptionID, BoxCoded, BoxMoney, ",
                "BoxText, BoxBroadcastTextID, SpellID, OverrideIconID FROM gossip_menu_option ",
                "WHERE MenuID = ? ORDER BY MenuID, OptionID"
            ),
            Self::SEL_GOSSIP_MENU_OPTION_KEYS => {
                "SELECT MenuID, OptionID FROM gossip_menu_option ORDER BY MenuID, OptionID"
            }
            Self::SEL_GOSSIP_MENU_OPTIONS_ALL => concat!(
                "SELECT MenuID, GossipOptionID, OptionID, OptionNpc, OptionText, OptionBroadcastTextID, ",
                "Language, Flags, ActionMenuID, ActionPoiID, GossipNpcOptionID, BoxCoded, BoxMoney, ",
                "BoxText, BoxBroadcastTextID, SpellID, OverrideIconID FROM gossip_menu_option ORDER BY MenuID, OptionID"
            ),
            Self::SEL_GOSSIP_MENU_OPTION_LOCALES => {
                "SELECT MenuID, OptionID, Locale, OptionText, BoxText FROM gossip_menu_option_locale"
            }
            Self::SEL_GOSSIP_MENU_ADDON => {
                "SELECT MenuID, FriendshipFactionID FROM gossip_menu_addon"
            }
            Self::SEL_BROADCAST_TEXT_LOCALE => {
                "SELECT Text_lang FROM hotfixes.broadcast_text_locale WHERE ID = ? AND locale = ?"
            }
            Self::SEL_CREATURE_TEMPLATE_LOCALE => {
                "SELECT Name, NameAlt, Title, TitleAlt FROM creature_template_locale WHERE entry = ? AND locale = ?"
            }
            Self::SEL_VENDOR_ITEM_PRICE => concat!(
                "SELECT COALESCE(isp.BuyPrice, 0), COALESCE(isp.SellPrice, 0), ",
                "COALESCE(isp.MaxDurability, 0), COALESCE(isp.VendorStackCount, 1) ",
                "FROM npc_vendor nv ",
                "LEFT JOIN hotfixes.item_sparse isp ON nv.item = isp.ID ",
                "WHERE nv.entry = ? AND nv.item = ? LIMIT 1",
            ),
            Self::SEL_ITEM_SELL_PRICE => {
                "SELECT COALESCE(SellPrice, 0) FROM hotfixes.item_sparse WHERE ID = ? LIMIT 1"
            }
            Self::SEL_ITEM_TEMPLATE_ADDON_MONEY_LOOT => concat!(
                "SELECT MinMoneyLoot, MaxMoneyLoot ",
                "FROM item_template_addon WHERE Id = ? LIMIT 1",
            ),
            Self::SEL_GAMEOBJECT_TEMPLATE_ADDON_MONEY_LOOT => concat!(
                "SELECT mingold, maxgold ",
                "FROM gameobject_template_addon WHERE entry = ? LIMIT 1",
            ),
            Self::SEL_ITEM_TEMPLATE_ADDON_LOOT_METADATA => concat!(
                "SELECT COALESCE(FlagsCu, 0), COALESCE(QuestLogItemId, 0) ",
                "FROM item_template_addon WHERE Id = ? LIMIT 1",
            ),
            Self::SEL_ITEM_LOOT_TEMPLATE_ROWS => concat!(
                "SELECT Item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount ",
                "FROM item_loot_template WHERE Entry = ?",
            ),
            Self::SEL_ITEM_LOOT_TEMPLATE_ALL_ROWS => concat!(
                "SELECT Entry, Item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount ",
                "FROM item_loot_template",
            ),
            Self::SEL_CREATURE_LOOT_TEMPLATE_ROWS => concat!(
                "SELECT Item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount ",
                "FROM creature_loot_template WHERE Entry = ?",
            ),
            Self::SEL_CREATURE_LOOT_TEMPLATE_ALL_ROWS => concat!(
                "SELECT Entry, Item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount ",
                "FROM creature_loot_template",
            ),
            Self::SEL_FISHING_LOOT_TEMPLATE_ROWS => concat!(
                "SELECT Item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount ",
                "FROM fishing_loot_template WHERE Entry = ?",
            ),
            Self::SEL_FISHING_LOOT_TEMPLATE_ALL_ROWS => concat!(
                "SELECT Entry, Item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount ",
                "FROM fishing_loot_template",
            ),
            Self::SEL_FISHING_BASE_SKILL_LEVELS => {
                "SELECT entry, skill FROM skill_fishing_base_level"
            }
            Self::SEL_SKILL_TIERS => concat!(
                "SELECT ID, Value1, Value2, Value3, Value4, Value5, Value6, Value7, Value8, ",
                "Value9, Value10, Value11, Value12, Value13, Value14, Value15, Value16 ",
                "FROM skill_tiers",
            ),
            Self::SEL_GAMEOBJECT_LOOT_TEMPLATE_ROWS => concat!(
                "SELECT Item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount ",
                "FROM gameobject_loot_template WHERE Entry = ?",
            ),
            Self::SEL_GAMEOBJECT_LOOT_TEMPLATE_ALL_ROWS => concat!(
                "SELECT Entry, Item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount ",
                "FROM gameobject_loot_template",
            ),
            Self::SEL_MAIL_LOOT_TEMPLATE_ROWS => concat!(
                "SELECT Item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount ",
                "FROM mail_loot_template WHERE Entry = ?",
            ),
            Self::SEL_MAIL_LOOT_TEMPLATE_ALL_ROWS => concat!(
                "SELECT Entry, Item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount ",
                "FROM mail_loot_template",
            ),
            Self::SEL_MAIL_LEVEL_REWARDS => {
                "SELECT level, raceMask, mailTemplateId, senderEntry FROM mail_level_reward"
            }
            Self::SEL_POINTS_OF_INTEREST => concat!(
                "SELECT ID, PositionX, PositionY, PositionZ, Icon, Flags, Importance, Name, WMOGroupID ",
                "FROM points_of_interest",
            ),
            Self::SEL_POINTS_OF_INTEREST_LOCALES => {
                "SELECT ID, locale, Name FROM points_of_interest_locale"
            }
            Self::SEL_MILLING_LOOT_TEMPLATE_ROWS => concat!(
                "SELECT Item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount ",
                "FROM milling_loot_template WHERE Entry = ?",
            ),
            Self::SEL_MILLING_LOOT_TEMPLATE_ALL_ROWS => concat!(
                "SELECT Entry, Item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount ",
                "FROM milling_loot_template",
            ),
            Self::SEL_PICKPOCKETING_LOOT_TEMPLATE_ROWS => concat!(
                "SELECT Item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount ",
                "FROM pickpocketing_loot_template WHERE Entry = ?",
            ),
            Self::SEL_PICKPOCKETING_LOOT_TEMPLATE_ALL_ROWS => concat!(
                "SELECT Entry, Item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount ",
                "FROM pickpocketing_loot_template",
            ),
            Self::SEL_PROSPECTING_LOOT_TEMPLATE_ROWS => concat!(
                "SELECT Item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount ",
                "FROM prospecting_loot_template WHERE Entry = ?",
            ),
            Self::SEL_PROSPECTING_LOOT_TEMPLATE_ALL_ROWS => concat!(
                "SELECT Entry, Item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount ",
                "FROM prospecting_loot_template",
            ),
            Self::SEL_REFERENCE_LOOT_TEMPLATE_ROWS => concat!(
                "SELECT Item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount ",
                "FROM reference_loot_template WHERE Entry = ?",
            ),
            Self::SEL_REFERENCE_LOOT_TEMPLATE_ALL_ROWS => concat!(
                "SELECT Entry, Item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount ",
                "FROM reference_loot_template",
            ),
            Self::SEL_SKINNING_LOOT_TEMPLATE_ROWS => concat!(
                "SELECT Item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount ",
                "FROM skinning_loot_template WHERE Entry = ?",
            ),
            Self::SEL_SKINNING_LOOT_TEMPLATE_ALL_ROWS => concat!(
                "SELECT Entry, Item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount ",
                "FROM skinning_loot_template",
            ),
            Self::SEL_DISENCHANT_LOOT_TEMPLATE_ROWS => concat!(
                "SELECT Item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount ",
                "FROM disenchant_loot_template WHERE Entry = ?",
            ),
            Self::SEL_DISENCHANT_LOOT_TEMPLATE_ALL_ROWS => concat!(
                "SELECT Entry, Item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount ",
                "FROM disenchant_loot_template",
            ),
            Self::SEL_SPELL_LOOT_TEMPLATE_ROWS => concat!(
                "SELECT Item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount ",
                "FROM spell_loot_template WHERE Entry = ?",
            ),
            Self::SEL_SPELL_LOOT_TEMPLATE_ALL_ROWS => concat!(
                "SELECT Entry, Item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount ",
                "FROM spell_loot_template",
            ),
            Self::SEL_SPELL_PET_AURAS => "SELECT spell, effectId, pet, aura FROM spell_pet_auras",
            Self::SEL_TRAINER_CAST_SCRIPT_BINDING_IDS => {
                "SELECT DISTINCT spell_id FROM spell_script_names"
            }
            Self::SEL_TRAINER_CAST_LEGACY_SCRIPT_IDS => {
                "SELECT DISTINCT (id & 16777215) FROM spell_scripts"
            }
            Self::SEL_TRAINER_CAST_CONDITION_IDS => {
                "SELECT DISTINCT SourceEntry FROM conditions WHERE SourceTypeOrReferenceId IN (13, 17)"
            }
            Self::SEL_SPELL_THREATS => "SELECT entry, flatMod, pctMod, apPctMod FROM spell_threat",
            Self::SEL_SPELL_ENCHANT_PROC_DATA => {
                "SELECT EnchantID, Chance, ProcsPerMinute, HitMask, AttributesMask FROM spell_enchant_proc_data"
            }
            Self::SEL_SPELL_LINKED => {
                "SELECT spell_trigger, spell_effect, type FROM spell_linked_spell"
            }
            Self::SEL_SPELL_TOTEM_MODEL => {
                "SELECT SpellID, RaceID, DisplayID from spell_totem_model"
            }
            Self::SEL_SPELL_REQUIRED => "SELECT spell_id, req_spell from spell_required",
            Self::SEL_SPELL_LEARN_SPELL => "SELECT entry, SpellID, Active FROM spell_learn_spell",
            Self::SEL_SPELL_TARGET_POSITION => {
                "SELECT ID, EffectIndex, MapID, PositionX, PositionY, PositionZ, Orientation FROM spell_target_position"
            }
            Self::SEL_SPELL_GROUP => "SELECT id, spell_id FROM spell_group",
            Self::SEL_SPELL_GROUP_STACK_RULES => {
                "SELECT group_id, stack_rule FROM spell_group_stack_rules"
            }
            Self::SEL_SPELL_PROC => {
                "SELECT SpellId, SchoolMask, SpellFamilyName, SpellFamilyMask0, SpellFamilyMask1, SpellFamilyMask2, SpellFamilyMask3, ProcFlags, ProcFlags2, SpellTypeMask, SpellPhaseMask, HitMask, AttributesMask, DisableEffectsMask, ProcsPerMinute, Chance, Cooldown, Charges FROM spell_proc"
            }
            Self::SEL_SPELL_AREA => {
                "SELECT spell, area, quest_start, quest_start_status, quest_end_status, quest_end, aura_spell, racemask, gender, flags FROM spell_area"
            }
            Self::SEL_SPELL_CUSTOM_ATTR => "SELECT entry, attributes FROM spell_custom_attr",
            Self::SEL_SERVERSIDE_SPELL_EFFECT => concat!(
                "SELECT SpellID, EffectIndex, DifficultyID, Effect, EffectAura, EffectAmplitude, EffectAttributes, ",
                "EffectAuraPeriod, EffectBonusCoefficient, EffectChainAmplitude, EffectChainTargets, EffectItemType, EffectMechanic, EffectPointsPerResource, ",
                "EffectPosFacing, EffectRealPointsPerLevel, EffectTriggerSpell, BonusCoefficientFromAP, PvpMultiplier, Coefficient, Variance, ",
                "ResourceCoefficient, GroupSizeBasePointsCoefficient, EffectBasePoints, EffectMiscValue1, EffectMiscValue2, EffectRadiusIndex1, ",
                "EffectRadiusIndex2, EffectSpellClassMask1, EffectSpellClassMask2, EffectSpellClassMask3, EffectSpellClassMask4, ImplicitTarget1, ",
                "ImplicitTarget2 FROM serverside_spell_effect"
            ),
            Self::SEL_SERVERSIDE_SPELL => concat!(
                "SELECT Id, DifficultyID, CategoryId, Dispel, Mechanic, Attributes, AttributesEx, AttributesEx2, AttributesEx3, ",
                "AttributesEx4, AttributesEx5, AttributesEx6, AttributesEx7, AttributesEx8, AttributesEx9, AttributesEx10, AttributesEx11, AttributesEx12, AttributesEx13, ",
                "AttributesEx14, Stances, StancesNot, Targets, TargetCreatureType, RequiresSpellFocus, FacingCasterFlags, CasterAuraState, TargetAuraState, ",
                "ExcludeCasterAuraState, ExcludeTargetAuraState, CasterAuraSpell, TargetAuraSpell, ExcludeCasterAuraSpell, ExcludeTargetAuraSpell, ",
                "CasterAuraType, TargetAuraType, ExcludeCasterAuraType, ExcludeTargetAuraType, CastingTimeIndex, ",
                "RecoveryTime, CategoryRecoveryTime, StartRecoveryCategory, StartRecoveryTime, InterruptFlags, AuraInterruptFlags1, AuraInterruptFlags2, ",
                "ChannelInterruptFlags1, ChannelInterruptFlags2, ProcFlags, ProcFlags2, ProcChance, ProcCharges, ProcCooldown, ProcBasePPM, MaxLevel, BaseLevel, SpellLevel, ",
                "DurationIndex, RangeIndex, Speed, LaunchDelay, StackAmount, EquippedItemClass, EquippedItemSubClassMask, EquippedItemInventoryTypeMask, ContentTuningId, ",
                "SpellName, ConeAngle, ConeWidth, MaxTargetLevel, MaxAffectedTargets, SpellFamilyName, SpellFamilyFlags1, SpellFamilyFlags2, SpellFamilyFlags3, SpellFamilyFlags4, ",
                "DmgClass, PreventionType, AreaGroupId, SchoolMask, ChargeCategoryId FROM serverside_spell"
            ),
            Self::SEL_LOOT_TEMPLATE_CONDITION_ROWS => concat!(
                "SELECT ElseGroup, ConditionTypeOrReference, ConditionTarget, ",
                "ConditionValue1, ConditionValue2, ConditionValue3, ",
                "COALESCE(ConditionStringValue1, ''), NegativeCondition, COALESCE(ScriptName, '') ",
                "FROM conditions ",
                "WHERE SourceTypeOrReferenceId = ? AND SourceGroup = ? AND SourceEntry = ? AND SourceId = 0 ",
                "ORDER BY ElseGroup, ConditionTypeOrReference, ConditionTarget, ConditionValue1, ConditionValue2, ConditionValue3",
            ),
            Self::SEL_LOOT_TEMPLATE_CONDITION_IDS => concat!(
                "SELECT DISTINCT SourceTypeOrReferenceId, SourceGroup, SourceEntry ",
                "FROM conditions ",
                "WHERE SourceTypeOrReferenceId BETWEEN 1 AND 12 AND SourceId = 0 ",
                "ORDER BY SourceTypeOrReferenceId, SourceGroup, SourceEntry",
            ),
            Self::SEL_LOOT_TEMPLATE_CONDITION_REFERENCE_USES => concat!(
                "SELECT DISTINCT SourceTypeOrReferenceId, SourceGroup, SourceEntry, -ConditionTypeOrReference ",
                "FROM conditions ",
                "WHERE SourceId = 0 AND ConditionTypeOrReference < 0 ",
                "AND ConditionTypeOrReference <> SourceTypeOrReferenceId ",
                "AND (SourceTypeOrReferenceId BETWEEN 1 AND 12 OR SourceTypeOrReferenceId < 0) ",
                "ORDER BY SourceTypeOrReferenceId, SourceGroup, SourceEntry, -ConditionTypeOrReference",
            ),
            Self::SEL_CONDITION_REFERENCE_TEMPLATE_IDS => concat!(
                "SELECT DISTINCT -SourceTypeOrReferenceId ",
                "FROM conditions ",
                "WHERE SourceTypeOrReferenceId < 0 AND SourceGroup = 0 AND SourceEntry = 0 AND SourceId = 0 ",
                "ORDER BY -SourceTypeOrReferenceId",
            ),
            Self::SEL_CONDITIONS => concat!(
                "SELECT SourceTypeOrReferenceId, SourceGroup, SourceEntry, SourceId, ElseGroup, ",
                "ConditionTypeOrReference, ConditionTarget, ConditionValue1, ConditionValue2, ConditionValue3, ",
                "COALESCE(ConditionStringValue1, ''), NegativeCondition, ErrorType, ErrorTextId, COALESCE(ScriptName, '') ",
                "FROM conditions",
            ),
            Self::SEL_ITEM_RANDOM_ENCHANTMENT_TEMPLATE => {
                "SELECT Id, EnchantmentId, Chance FROM item_random_enchantment_template"
            }
            Self::SEL_AREA_TRIGGER_TELEPORT => {
                "SELECT at.ID, wsl.MapID, wsl.LocX, wsl.LocY, wsl.LocZ, wsl.Facing FROM areatrigger_teleport at LEFT JOIN world_safe_locs wsl ON at.PortLocID = wsl.ID"
            }
            Self::SEL_AREA_TRIGGER_SCRIPTS => "SELECT entry, ScriptName FROM areatrigger_scripts",
            Self::SEL_AREA_TRIGGER_TELEPORT_RELATIONS => {
                "SELECT ID, PortLocID FROM areatrigger_teleport"
            }
            Self::SEL_QUEST_AREA_TRIGGER_RELATIONS => {
                "SELECT id, quest FROM areatrigger_involvedrelation"
            }
            Self::SEL_TAVERN_AREA_TRIGGERS => "SELECT id FROM areatrigger_tavern",
            Self::SEL_PHASE_NAMES => "SELECT `ID`, `Name` FROM `phase_name`",
            Self::SEL_SCENE_TEMPLATES => {
                "SELECT SceneId, Flags, ScriptPackageID, Encrypted, ScriptName FROM scene_template"
            }
            Self::SEL_PLAYER_CHOICES => {
                "SELECT ChoiceId, UiTextureKitId, SoundKitId, CloseSoundKitId, Duration, Question, PendingChoiceText, HideWarboardHeader, KeepOpenAfterChoice FROM playerchoice"
            }
            Self::SEL_PLAYER_CHOICE_RESPONSES => {
                "SELECT ChoiceId, ResponseId, ResponseIdentifier, ChoiceArtFileId, Flags, WidgetSetID, UiTextureAtlasElementID, SoundKitID, GroupID, UiTextureKitID, Answer, Header, SubHeader, ButtonTooltip, Description, Confirmation, RewardQuestID FROM playerchoice_response ORDER BY `Index` ASC"
            }
            Self::SEL_PLAYER_CHOICE_RESPONSE_REWARDS => {
                "SELECT ChoiceId, ResponseId, TitleId, PackageId, SkillLineId, SkillPointCount, ArenaPointCount, HonorPointCount, Money, Xp FROM playerchoice_response_reward"
            }
            Self::SEL_PLAYER_CHOICE_RESPONSE_REWARD_ITEMS => {
                "SELECT ChoiceId, ResponseId, ItemId, BonusListIDs, Quantity FROM playerchoice_response_reward_item ORDER BY `Index` ASC"
            }
            Self::SEL_PLAYER_CHOICE_RESPONSE_REWARD_CURRENCIES => {
                "SELECT ChoiceId, ResponseId, CurrencyId, Quantity FROM playerchoice_response_reward_currency ORDER BY `Index` ASC"
            }
            Self::SEL_PLAYER_CHOICE_RESPONSE_REWARD_FACTIONS => {
                "SELECT ChoiceId, ResponseId, FactionId, Quantity FROM playerchoice_response_reward_faction ORDER BY `Index` ASC"
            }
            Self::SEL_PLAYER_CHOICE_RESPONSE_REWARD_ITEM_CHOICES => {
                "SELECT ChoiceId, ResponseId, ItemId, BonusListIDs, Quantity FROM playerchoice_response_reward_item_choice ORDER BY `Index` ASC"
            }
            Self::SEL_PLAYER_CHOICE_RESPONSE_MAW_POWERS => {
                "SELECT ChoiceId, ResponseId, TypeArtFileID, Rarity, RarityColor, SpellID, MaxStacks FROM playerchoice_response_maw_power"
            }
            Self::SEL_PLAYER_CHOICE_LOCALES => {
                "SELECT ChoiceId, locale, Question FROM playerchoice_locale"
            }
            Self::SEL_PLAYER_CHOICE_RESPONSE_LOCALES => {
                "SELECT ChoiceID, ResponseID, locale, Answer, Header, SubHeader, ButtonTooltip, Description, Confirmation FROM playerchoice_response_locale"
            }
            Self::SEL_JUMP_CHARGE_PARAMS => {
                "SELECT id, speed, treatSpeedAsMoveTimeSeconds, jumpGravity, spellVisualId, progressCurveId, parabolicCurveId FROM jump_charge_params"
            }
            Self::SEL_TRAINER_BY_CREATURE => {
                "SELECT TrainerID FROM creature_trainer WHERE CreatureID = ? AND MenuID = 0 AND OptionID = 0"
            }
            Self::SEL_TRAINER_BY_CREATURE_GOSSIP_OPTION => {
                "SELECT TrainerID FROM creature_trainer WHERE CreatureID = ? AND MenuID = ? AND OptionID = ?"
            }
            Self::SEL_TRAINER_SPELLS => {
                "SELECT SpellId, MoneyCost, ReqSkillLine, ReqSkillRank, \
                 ReqAbility1, ReqAbility2, ReqAbility3, ReqLevel \
                 FROM trainer_spell WHERE TrainerId = ?"
            }
            Self::SEL_TRAINER_INFO => "SELECT Id, Type, Greeting FROM trainer WHERE Id = ?",
            Self::SEL_TRAINER_SPELLS_ALL => {
                "SELECT TrainerId, SpellId, MoneyCost, ReqSkillLine, ReqSkillRank, ReqAbility1, ReqAbility2, ReqAbility3, ReqLevel FROM trainer_spell"
            }
            Self::SEL_TRAINERS_ALL => "SELECT Id, Type, Greeting FROM trainer",
            Self::SEL_TRAINER_LOCALES => "SELECT Id, locale, Greeting_lang FROM trainer_locale",
            Self::SEL_CREATURE_TRAINERS_ALL => {
                "SELECT CreatureID, TrainerID, MenuID, OptionID FROM creature_trainer"
            }
            Self::SEL_BATTLE_PET_BREEDS => "SELECT speciesId, breedId FROM battle_pet_breeds",
            Self::SEL_BATTLE_PET_QUALITY => "SELECT speciesId, quality FROM battle_pet_quality",
            Self::SEL_FACTION_CHANGE_ACHIEVEMENTS => {
                "SELECT alliance_id, horde_id FROM player_factionchange_achievement"
            }
            Self::SEL_FACTION_CHANGE_QUESTS => {
                "SELECT alliance_id, horde_id FROM player_factionchange_quests"
            }
            Self::SEL_FACTION_CHANGE_REPUTATIONS => {
                "SELECT alliance_id, horde_id FROM player_factionchange_reputations"
            }
            Self::SEL_FACTION_CHANGE_SPELLS => {
                "SELECT alliance_id, horde_id FROM player_factionchange_spells"
            }
            Self::SEL_FACTION_CHANGE_TITLES => {
                "SELECT alliance_id, horde_id FROM player_factionchange_titles"
            }
            Self::SEL_TRAINER_IDS => "SELECT Id FROM trainer",
            Self::SEL_CONVERSATION_LINE_TEMPLATE_IDS => "SELECT Id FROM conversation_line_template",
            Self::SEL_AREA_TRIGGER_TEMPLATE_IDS => "SELECT Id, IsCustom FROM areatrigger_template",
            Self::SEL_AREATRIGGER_TEMPLATE_ACTIONS => {
                "SELECT AreaTriggerId, IsCustom, ActionType, ActionParam, TargetType FROM `areatrigger_template_actions`"
            }
            Self::SEL_AREATRIGGER_CREATE_PROPERTIES_POLYGON_VERTICES => {
                "SELECT AreaTriggerCreatePropertiesId, IsCustom, Idx, VerticeX, VerticeY, VerticeTargetX, VerticeTargetY FROM `areatrigger_create_properties_polygon_vertex` ORDER BY `AreaTriggerCreatePropertiesId`, `IsCustom`, `Idx`"
            }
            Self::SEL_AREATRIGGER_CREATE_PROPERTIES_SPLINE_POINTS => {
                "SELECT AreaTriggerCreatePropertiesId, IsCustom, X, Y, Z FROM `areatrigger_create_properties_spline_point` ORDER BY `AreaTriggerCreatePropertiesId`, `IsCustom`, `Idx`"
            }
            Self::SEL_AREATRIGGER_CREATE_PROPERTIES => {
                "SELECT Id, IsCustom, AreaTriggerId, IsAreatriggerCustom, Flags, MoveCurveId, ScaleCurveId, MorphCurveId, FacingCurveId, AnimId, AnimKitId, DecalPropertiesId, TimeToTarget, TimeToTargetScale, Shape, ShapeData0, ShapeData1, ShapeData2, ShapeData3, ShapeData4, ShapeData5, ShapeData6, ShapeData7, ScriptName FROM `areatrigger_create_properties`"
            }
            Self::SEL_AREATRIGGER_CREATE_PROPERTIES_ORBIT => {
                "SELECT AreaTriggerCreatePropertiesId, IsCustom, StartDelay, CircleRadius, BlendFromRadius, InitialAngle, ZOffset, CounterClockwise, CanLoop FROM `areatrigger_create_properties_orbit`"
            }
            Self::SEL_AREATRIGGER_TEMPLATES => {
                "SELECT Id, IsCustom, Flags FROM `areatrigger_template`"
            }
            Self::SEL_QUEST_TEMPLATE => concat!(
                "SELECT qt.ID, qt.QuestType, qt.QuestLevel, qt.QuestMaxScalingLevel, qt.QuestPackageID, qt.MinLevel, qt.QuestSortID, ",
                "qt.QuestInfoID, qt.SuggestedGroupNum, qt.RewardNextQuest, qt.RewardXPDifficulty, qt.RewardXPMultiplier, ",
                "qt.RewardMoneyDifficulty, qt.RewardMoneyMultiplier, qt.RewardBonusMoney, ",
                "qt.RewardDisplaySpell1, qt.RewardDisplaySpell2, qt.RewardDisplaySpell3, ",
                "qt.RewardSpell, qt.RewardHonor, qt.Flags, qt.FlagsEx, qt.FlagsEx2, ",
                "qt.RewardItem1, qt.RewardAmount1, qt.ItemDrop1, qt.ItemDropQuantity1, ",
                "qt.RewardItem2, qt.RewardAmount2, qt.ItemDrop2, qt.ItemDropQuantity2, ",
                "qt.RewardItem3, qt.RewardAmount3, qt.ItemDrop3, qt.ItemDropQuantity3, ",
                "qt.RewardItem4, qt.RewardAmount4, qt.ItemDrop4, qt.ItemDropQuantity4, ",
                "qt.LogTitle, qt.LogDescription, qt.QuestDescription, qt.AreaDescription, qt.QuestCompletionLog, ",
                "qt.AllowableRaces AS AllowableRaces, ",
                "CAST(COALESCE(qta.AllowableClasses, 0) AS UNSIGNED) AS AllowableClasses, ",
                "CAST(COALESCE(qta.MaxLevel, 0) AS UNSIGNED) AS MaxLevel, ",
                "CAST(COALESCE(qta.PrevQuestID, 0) AS SIGNED) AS PrevQuestID, ",
                "CAST(COALESCE(qta.RequiredMinRepFaction, 0) AS UNSIGNED) AS RequiredMinRepFaction, ",
                "CAST(COALESCE(qta.RequiredMinRepValue, 0) AS SIGNED) AS RequiredMinRepValue, ",
                "CAST(COALESCE(qta.RequiredMaxRepFaction, 0) AS UNSIGNED) AS RequiredMaxRepFaction, ",
                "CAST(COALESCE(qta.RequiredMaxRepValue, 0) AS SIGNED) AS RequiredMaxRepValue, ",
                "qt.RewardChoiceItemID1, qt.RewardChoiceItemQuantity1, ",
                "qt.RewardChoiceItemID2, qt.RewardChoiceItemQuantity2, ",
                "qt.RewardChoiceItemID3, qt.RewardChoiceItemQuantity3, ",
                "qt.RewardChoiceItemID4, qt.RewardChoiceItemQuantity4, ",
                "qt.RewardChoiceItemID5, qt.RewardChoiceItemQuantity5, ",
                "qt.RewardChoiceItemID6, qt.RewardChoiceItemQuantity6, ",
                "CAST(COALESCE(qta.NextQuestID, 0) AS UNSIGNED) AS NextQuestID, ",
                "CAST(COALESCE(qta.ExclusiveGroup, 0) AS SIGNED) AS ExclusiveGroup, ",
                "CAST(COALESCE(qta.BreadcrumbForQuestId, 0) AS SIGNED) AS BreadcrumbForQuestId, ",
                "CAST(COALESCE(qta.SpecialFlags, 0) AS UNSIGNED) AS SpecialFlags, ",
                "qt.Expansion, ",
                "qt.StartItem, ",
                "CAST(COALESCE(qta.SourceSpellID, 0) AS UNSIGNED) AS SourceSpellID, ",
                "CAST(COALESCE(qta.ProvidedItemCount, 0) AS UNSIGNED) AS ProvidedItemCount, ",
                "qt.TimeAllowed, ",
                "CAST(COALESCE(qrci.Type1, 0) AS UNSIGNED) AS RewardChoiceItemType1, ",
                "CAST(COALESCE(qrci.Type2, 0) AS UNSIGNED) AS RewardChoiceItemType2, ",
                "CAST(COALESCE(qrci.Type3, 0) AS UNSIGNED) AS RewardChoiceItemType3, ",
                "CAST(COALESCE(qrci.Type4, 0) AS UNSIGNED) AS RewardChoiceItemType4, ",
                "CAST(COALESCE(qrci.Type5, 0) AS UNSIGNED) AS RewardChoiceItemType5, ",
                "CAST(COALESCE(qrci.Type6, 0) AS UNSIGNED) AS RewardChoiceItemType6, ",
                "qt.RewardCurrencyID1, qt.RewardCurrencyQty1, ",
                "qt.RewardCurrencyID2, qt.RewardCurrencyQty2, ",
                "qt.RewardCurrencyID3, qt.RewardCurrencyQty3, ",
                "qt.RewardCurrencyID4, qt.RewardCurrencyQty4, ",
                "qt.RewardSkillLineID, qt.RewardNumSkillUps, qt.RewardTitle, ",
                "CAST(COALESCE(qta.RewardMailTemplateID, 0) AS UNSIGNED) AS RewardMailTemplateID, ",
                "CAST(COALESCE(qta.RewardMailDelay, 0) AS UNSIGNED) AS RewardMailDelay, ",
                "CAST(COALESCE(qms.RewardMailSenderEntry, 0) AS UNSIGNED) AS RewardMailSenderEntry, ",
                "qt.RewardFactionID1, qt.RewardFactionValue1, qt.RewardFactionOverride1, qt.RewardFactionCapIn1, ",
                "qt.RewardFactionID2, qt.RewardFactionValue2, qt.RewardFactionOverride2, qt.RewardFactionCapIn2, ",
                "qt.RewardFactionID3, qt.RewardFactionValue3, qt.RewardFactionOverride3, qt.RewardFactionCapIn3, ",
                "qt.RewardFactionID4, qt.RewardFactionValue4, qt.RewardFactionOverride4, qt.RewardFactionCapIn4, ",
                "qt.RewardFactionID5, qt.RewardFactionValue5, qt.RewardFactionOverride5, qt.RewardFactionCapIn5, ",
                "qt.RewardFactionFlags, ",
                "CAST(COALESCE(qta.RequiredSkillID, 0) AS UNSIGNED) AS RequiredSkillID, ",
                "CAST(COALESCE(qta.RequiredSkillPoints, 0) AS UNSIGNED) AS RequiredSkillPoints ",
                "FROM quest_template qt ",
                "LEFT JOIN quest_template_addon qta ON qt.ID = qta.ID ",
                "LEFT JOIN quest_reward_choice_items qrci ON qt.ID = qrci.QuestID ",
                "LEFT JOIN quest_mail_sender qms ON qt.ID = qms.QuestId"
            ),
            Self::SEL_QUEST_OBJECTIVES => {
                "SELECT ID, QuestID, Type, `Order`, StorageIndex, ObjectID, Amount, Flags, Flags2, ProgressBarWeight, Description FROM quest_objectives ORDER BY QuestID, `Order`"
            }
            Self::SEL_GAME_EVENT_SEASONAL_QUEST_RELATIONS => {
                "SELECT questId, eventEntry FROM game_event_seasonal_questrelation"
            }
            Self::SEL_QUEST_STARTERS => "SELECT id, quest FROM creature_queststarter",
            Self::SEL_QUEST_ENDERS => "SELECT id, quest FROM creature_questender",
            Self::SEL_GAMEOBJECT_QUEST_STARTERS => "SELECT id, quest FROM gameobject_queststarter",
            Self::SEL_GAMEOBJECT_QUEST_ENDERS => "SELECT id, quest FROM gameobject_questender",
        }
    }
}
