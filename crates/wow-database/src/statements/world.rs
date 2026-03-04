//! World database prepared statement definitions.
//!
//! These correspond to the `world` database and the C# `WorldStatements` enum.

use super::StatementDef;

/// Prepared statements for the world database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types)]
pub enum WorldStatements {
    DEL_LINKED_RESPAWN,
    DEL_LINKED_RESPAWN_MASTER,
    REP_LINKED_RESPAWN,
    SEL_CREATURE_TEXT,
    SEL_SMART_SCRIPTS,
    DEL_GAMEOBJECT,
    DEL_EVENT_GAMEOBJECT,
    INS_GRAVEYARD_ZONE,
    DEL_GRAVEYARD_ZONE,
    INS_GAME_TELE,
    DEL_GAME_TELE,
    INS_NPC_VENDOR,
    DEL_NPC_VENDOR,
    SEL_NPC_VENDOR_REF,
    SEL_VENDOR_ITEMS,
    UPD_CREATURE_MOVEMENT_TYPE,
    UPD_CREATURE_FACTION,
    UPD_CREATURE_NPCFLAG,
    UPD_CREATURE_POSITION,
    UPD_CREATURE_MAP_POSITION,
    UPD_CREATURE_WANDER_DISTANCE,
    UPD_CREATURE_SPAWN_TIME_SECS,
    INS_CREATURE_FORMATION,
    SEL_WAYPOINT_PATH_BY_PATHID,
    INS_WAYPOINT_PATH_NODE,
    DEL_WAYPOINT_PATH_NODE,
    UPD_WAYPOINT_PATH_NODE,
    UPD_WAYPOINT_PATH_NODE_POSITION,
    SEL_WAYPOINT_PATH_NODE_MAX_PATHID,
    SEL_WAYPOINT_PATH_NODE_BY_PATHID,
    SEL_WAYPOINT_PATH_NODE_POS_BY_PATHID,
    SEL_WAYPOINT_PATH_NODE_POS_FIRST_BY_PATHID,
    SEL_WAYPOINT_PATH_NODE_POS_LAST_BY_PATHID,
    SEL_WAYPOINT_PATH_NODE_MAX_NODEID,
    SEL_WAYPOINT_PATH_NODE_BY_POS,
    UPD_CREATURE_ADDON_PATH,
    INS_CREATURE_ADDON,
    DEL_CREATURE_ADDON,
    SEL_CREATURE_ADDON_BY_GUID,
    DEL_CREATURE,
    SEL_COMMANDS,
    SEL_CREATURE_TEMPLATE,
    SEL_CREATURE_BY_ID,
    /// Creature template entry by spawn GUID (for vendor/trainer when not in visibility tracker).
    SEL_CREATURE_ENTRY_BY_GUID,
    SEL_GAMEOBJECT_NEAREST,
    SEL_CREATURE_NEAREST,
    SEL_GAMEOBJECT_TARGET,
    INS_CREATURE,
    DEL_GAME_EVENT_CREATURE,
    DEL_GAME_EVENT_MODEL_EQUIP,
    INS_GAMEOBJECT,
    SEL_DISABLES,
    INS_DISABLES,
    DEL_DISABLES,
    UPD_CREATURE_ZONE_AREA_DATA,
    UPD_GAMEOBJECT_ZONE_AREA_DATA,
    DEL_SPAWNGROUP_MEMBER,
    DEL_GAMEOBJECT_ADDON,
    SEL_GUILD_REWARDS_REQ_ACHIEVEMENTS,
    INS_CONDITION,
    /// Load creatures in a bounding box around a position on a map.
    SEL_CREATURES_IN_RANGE,
    /// Load creature template for query response (name, type, display, etc.).
    SEL_CREATURE_QUERY_RESPONSE,
    /// Load creature display models for a template entry.
    SEL_CREATURE_DISPLAY_MODELS,
    /// Load gameobjects in a bounding box around a position on a map.
    SEL_GAMEOBJECTS_IN_RANGE,
    /// Load gameobject template for query response.
    SEL_GAMEOBJECT_TEMPLATE_BY_ENTRY,
    /// SELECT InventoryType FROM item_template WHERE entry = ?
    SEL_ITEM_INVENTORY_TYPE,
    /// Load base stats for all race/class/level combos.
    SEL_PLAYER_LEVELSTATS,
    /// Load initial action buttons for character creation.
    SEL_PLAYER_CREATEINFO_ACTION,
    /// Fetch raw DB2 blob for a specific (TableHash, RecordId) from hotfix_blob.
    /// Used to serve CMSG_DB_QUERY_BULK responses with server-side overrides.
    SEL_HOTFIX_BLOB,
    /// Gossip MenuID for a creature entry (creature_template_gossip).
    SEL_CREATURE_GOSSIP_MENU,
    /// Gossip menu text ID (gossip_menu).
    SEL_GOSSIP_MENU,
    /// NPC text BroadcastTextID by npc_text ID.
    SEL_NPC_TEXT,
    /// Gossip menu options (gossip_menu_option) — includes OptionBroadcastTextID for localization.
    SEL_GOSSIP_MENU_OPTIONS,
    /// Localized text from broadcast_text_locale by ID and locale.
    SEL_BROADCAST_TEXT_LOCALE,
    /// Localized creature name/subname/title by entry and locale.
    SEL_CREATURE_TEMPLATE_LOCALE,
    /// Buy price + sell price + durability for a specific item in a vendor's list.
    /// Args: npc_vendor.entry (u32), npc_vendor.item (u32).
    SEL_VENDOR_ITEM_PRICE,
    /// Sell price for any item directly from item_sparse (no vendor check).
    /// Args: item ID (u32).
    SEL_ITEM_SELL_PRICE,
    /// Load all area trigger teleport destinations.
    SEL_AREA_TRIGGER_TELEPORT,
    // Quest system
    SEL_QUEST_TEMPLATE,
    SEL_QUEST_OBJECTIVES,
    SEL_QUEST_STARTERS,
    SEL_QUEST_ENDERS,
    /// Get TrainerId from creature_trainer by creature entry (NPC template ID).
    SEL_TRAINER_BY_CREATURE,
    /// Load all spells for a trainer by TrainerId.
    SEL_TRAINER_SPELLS,
    /// Load trainer type and greeting by trainer ID.
    SEL_TRAINER_INFO,
}

impl StatementDef for WorldStatements {
    fn sql(self) -> &'static str {
        match self {
            Self::DEL_LINKED_RESPAWN => "DELETE FROM linked_respawn WHERE guid = ? AND linkType  = ?",
            Self::DEL_LINKED_RESPAWN_MASTER => "DELETE FROM linked_respawn WHERE linkedGuid = ? AND linkType = ?",
            Self::REP_LINKED_RESPAWN => "REPLACE INTO linked_respawn (guid, linkedGuid, linkType) VALUES (?, ?, ?)",
            Self::SEL_CREATURE_TEXT => "SELECT CreatureID, GroupID, ID, Text, Type, Language, Probability, Emote, Duration, Sound, SoundPlayType, BroadcastTextId, TextRange FROM creature_text",
            Self::SEL_SMART_SCRIPTS => concat!(
                "SELECT entryorguid, source_type, id, link, Difficulties, event_type, event_phase_mask, event_chance, event_flags, ",
                "event_param1, event_param2, event_param3, event_param4, event_param5, event_param_string, ",
                "action_type, action_param1, action_param2, action_param3, action_param4, action_param5, action_param6, action_param7, ",
                "target_type, target_param1, target_param2, target_param3, target_param4, target_x, target_y, target_z, target_o ",
                "FROM smart_scripts ORDER BY entryorguid, source_type, id, link",
            ),
            Self::DEL_GAMEOBJECT => "DELETE FROM gameobject WHERE guid = ?",
            Self::DEL_EVENT_GAMEOBJECT => "DELETE FROM game_event_gameobject WHERE guid = ?",
            Self::INS_GRAVEYARD_ZONE => "INSERT INTO graveyard_zone (ID, GhostZone) VALUES (?, ?)",
            Self::DEL_GRAVEYARD_ZONE => "DELETE FROM graveyard_zone WHERE ID = ? AND GhostZone = ?",
            Self::INS_GAME_TELE => "INSERT INTO game_tele (id, position_x, position_y, position_z, orientation, map, name) VALUES (?, ?, ?, ?, ?, ?, ?)",
            Self::DEL_GAME_TELE => "DELETE FROM game_tele WHERE name = ?",
            Self::INS_NPC_VENDOR => "INSERT INTO npc_vendor (entry, item, maxcount, incrtime, extendedcost, type) VALUES(?, ?, ?, ?, ?, ?)",
            Self::DEL_NPC_VENDOR => "DELETE FROM npc_vendor WHERE entry = ? AND item = ? AND type = ?",
            Self::SEL_NPC_VENDOR_REF => "SELECT item, maxcount, incrtime, ExtendedCost, type, BonusListIDs, PlayerConditionID, IgnoreFiltering FROM npc_vendor WHERE entry = ? ORDER BY slot ASC",
            // Cols: 0=item, 1=maxcount, 2=ExtendedCost, 3=type, 4=slot,
            //       5=BuyPrice, 6=SellPrice, 7=MaxDurability, 8=VendorStackCount, 9=IgnoreFiltering
            Self::SEL_VENDOR_ITEMS => concat!(
                "SELECT nv.item, nv.maxcount, nv.ExtendedCost, nv.type, nv.slot, ",
                "COALESCE(isp.BuyPrice, 0), COALESCE(isp.SellPrice, 0), ",
                "COALESCE(isp.MaxDurability, 0), COALESCE(isp.VendorStackCount, 1), ",
                "nv.IgnoreFiltering ",
                "FROM npc_vendor nv ",
                "LEFT JOIN hotfixes.item_sparse isp ON nv.item = isp.ID ",
                "WHERE nv.entry = ? ORDER BY nv.slot ASC"
            ),
            Self::UPD_CREATURE_MOVEMENT_TYPE => "UPDATE creature SET MovementType = ? WHERE guid = ?",
            Self::UPD_CREATURE_FACTION => "UPDATE creature_template SET faction = ? WHERE entry = ?",
            Self::UPD_CREATURE_NPCFLAG => "UPDATE creature_template SET npcflag = ? WHERE entry = ?",
            Self::UPD_CREATURE_POSITION => "UPDATE creature SET position_x = ?, position_y = ?, position_z = ?, orientation = ? WHERE guid = ?",
            Self::UPD_CREATURE_MAP_POSITION => "UPDATE creature SET map = ?, position_x = ?, position_y = ?, position_z = ?, orientation = ? WHERE guid = ?",
            Self::UPD_CREATURE_WANDER_DISTANCE => "UPDATE creature SET wander_distance = ?, MovementType = ? WHERE guid = ?",
            Self::UPD_CREATURE_SPAWN_TIME_SECS => "UPDATE creature SET spawntimesecs = ? WHERE guid = ?",
            Self::INS_CREATURE_FORMATION => "INSERT INTO creature_formations (leaderGUID, memberGUID, dist, angle, groupAI) VALUES (?, ?, ?, ?, ?)",
            Self::SEL_WAYPOINT_PATH_BY_PATHID => "SELECT PathId, MoveType, Flags FROM waypoint_path WHERE PathId = ?",
            Self::INS_WAYPOINT_PATH_NODE => "INSERT INTO waypoint_path_node (PathId, NodeId, PositionX, PositionY, PositionZ, Orientation) VALUES (?, ?, ?, ?, ?, ?)",
            Self::DEL_WAYPOINT_PATH_NODE => "DELETE FROM waypoint_path_node WHERE PathId = ? AND NodeId = ?",
            Self::UPD_WAYPOINT_PATH_NODE => "UPDATE waypoint_path_node SET NodeId = NodeId - 1 WHERE PathId = ? AND NodeId > ?",
            Self::UPD_WAYPOINT_PATH_NODE_POSITION => "UPDATE waypoint_path_node SET PositionX = ?, PositionY = ?, PositionZ = ?, Orientation = ? WHERE PathId = ? AND NodeId = ?",
            Self::SEL_WAYPOINT_PATH_NODE_MAX_PATHID => "SELECT MAX(PathId) FROM waypoint_path_node",
            Self::SEL_WAYPOINT_PATH_NODE_BY_PATHID => "SELECT PathId, NodeId, PositionX, PositionY, PositionZ, Orientation, Delay FROM waypoint_path_node WHERE PathId = ? ORDER BY NodeId",
            Self::SEL_WAYPOINT_PATH_NODE_POS_BY_PATHID => "SELECT NodeId, PositionX, PositionY, PositionZ, Orientation FROM waypoint_path_node WHERE PathId = ?",
            Self::SEL_WAYPOINT_PATH_NODE_POS_FIRST_BY_PATHID => "SELECT PositionX, PositionY, PositionZ, Orientation FROM waypoint_path_node WHERE NodeId = 1 AND PathId = ?",
            Self::SEL_WAYPOINT_PATH_NODE_POS_LAST_BY_PATHID => "SELECT PositionX, PositionY, PositionZ, Orientation FROM waypoint_path_node WHERE PathId = ? ORDER BY NodeId DESC LIMIT 1",
            Self::SEL_WAYPOINT_PATH_NODE_MAX_NODEID => "SELECT MAX(NodeId) FROM waypoint_path_node WHERE PathId = ?",
            Self::SEL_WAYPOINT_PATH_NODE_BY_POS => "SELECT PathId, NodeId FROM waypoint_path_node WHERE (abs(PositionX - ?) <= ?) and (abs(PositionY - ?) <= ?) and (abs(PositionZ - ?) <= ?)",
            Self::UPD_CREATURE_ADDON_PATH => "UPDATE creature_addon SET PathId = ? WHERE guid = ?",
            Self::INS_CREATURE_ADDON => "INSERT INTO creature_addon(guid, PathId) VALUES (?, ?)",
            Self::DEL_CREATURE_ADDON => "DELETE FROM creature_addon WHERE guid = ?",
            Self::SEL_CREATURE_ADDON_BY_GUID => "SELECT guid FROM creature_addon WHERE guid = ?",
            Self::DEL_CREATURE => "DELETE FROM creature WHERE guid = ?",
            Self::SEL_COMMANDS => "SELECT name, help FROM command",
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
            Self::SEL_CREATURE_BY_ID => "SELECT guid FROM creature WHERE id = ?",
            Self::SEL_CREATURE_ENTRY_BY_GUID => "SELECT id FROM creature WHERE guid = ?",
            Self::SEL_GAMEOBJECT_NEAREST => "SELECT guid, id, position_x, position_y, position_z, map, (POW(position_x - ?, 2) + POW(position_y - ?, 2) + POW(position_z - ?, 2)) AS order_ FROM gameobject WHERE map = ? AND (POW(position_x - ?, 2) + POW(position_y - ?, 2) + POW(position_z - ?, 2)) <= ? ORDER BY order_",
            Self::SEL_CREATURE_NEAREST => "SELECT guid, id, position_x, position_y, position_z, map, (POW(position_x - ?, 2) + POW(position_y - ?, 2) + POW(position_z - ?, 2)) AS order_ FROM creature WHERE map = ? AND (POW(position_x - ?, 2) + POW(position_y - ?, 2) + POW(position_z - ?, 2)) <= ? ORDER BY order_",
            Self::SEL_GAMEOBJECT_TARGET => "", // No SQL registered in C# source
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
            Self::INS_DISABLES => "INSERT INTO disables (entry, sourceType, flags, comment) VALUES (?, ?, ?, ?)",
            Self::DEL_DISABLES => "DELETE FROM disables WHERE entry = ? AND sourceType = ?",
            Self::UPD_CREATURE_ZONE_AREA_DATA => "UPDATE creature SET zoneId = ?, areaId = ? WHERE guid = ?",
            Self::UPD_GAMEOBJECT_ZONE_AREA_DATA => "UPDATE gameobject SET zoneId = ?, areaId = ? WHERE guid = ?",
            Self::DEL_SPAWNGROUP_MEMBER => "DELETE FROM spawn_group WHERE spawnType = ? AND spawnId = ?",
            Self::DEL_GAMEOBJECT_ADDON => "DELETE FROM gameobject_addon WHERE guid = ?",
            Self::SEL_GUILD_REWARDS_REQ_ACHIEVEMENTS => "SELECT AchievementRequired FROM guild_rewards_req_achievements WHERE ItemID = ?",
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
                "ct.BaseAttackTime, ct.RangeAttackTime, ",
                "ctm.CreatureDisplayID ",
                "FROM creature c ",
                "JOIN creature_template ct ON c.id = ct.entry ",
                "LEFT JOIN creature_template_difficulty ctdiff ON ct.entry = ctdiff.Entry AND ctdiff.DifficultyID = 0 ",
                "LEFT JOIN creature_template_model ctm ON ct.entry = ctm.CreatureID AND ctm.Idx = 0 ",
                "WHERE c.map = ? AND c.position_x BETWEEN ? AND ? AND c.position_y BETWEEN ? AND ?",
            ),
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
                "gt.type, gt.displayId, gt.name, gt.size, gt.Data0, gt.Data1 ",
                "FROM gameobject g ",
                "JOIN gameobject_template gt ON g.id = gt.entry ",
                "WHERE g.map = ? AND g.position_x BETWEEN ? AND ? AND g.position_y BETWEEN ? AND ?",
            ),
            Self::SEL_ITEM_INVENTORY_TYPE => {
                "SELECT InventoryType FROM item_template WHERE entry = ?"
            }
            Self::SEL_PLAYER_LEVELSTATS => {
                "SELECT race, class, level, str, agi, sta, inte, spi, basehp, basemana FROM player_levelstats"
            }
            Self::SEL_PLAYER_CREATEINFO_ACTION => {
                "SELECT race, class, button, action, Type FROM playercreateinfo_action"
            }
            Self::SEL_GAMEOBJECT_TEMPLATE_BY_ENTRY => concat!(
                "SELECT entry, type, displayId, name, IconName, castBarCaption, unk1, ",
                "size, Data0, Data1, Data2, Data3, Data4, Data5, Data6, Data7, ",
                "Data8, Data9, Data10, Data11, Data12, Data13, Data14, Data15, ",
                "Data16, Data17, Data18, Data19, Data20, Data21, Data22, Data23, ",
                "Data24, Data25, Data26, Data27, Data28, Data29, Data30, Data31, ",
                "Data32, Data33 ",
                "FROM gameobject_template WHERE entry = ?",
            ),
            Self::SEL_HOTFIX_BLOB => {
                "SELECT Blob FROM hotfixes.hotfix_blob \
                 WHERE TableHash = ? AND RecordId = ? AND locale = 'enUS'"
            }
            Self::SEL_CREATURE_GOSSIP_MENU => {
                "SELECT MenuID FROM creature_template_gossip WHERE CreatureID = ?"
            }
            Self::SEL_GOSSIP_MENU => {
                "SELECT TextID FROM gossip_menu WHERE MenuID = ? LIMIT 1"
            }
            Self::SEL_NPC_TEXT => {
                "SELECT BroadcastTextID0 FROM npc_text WHERE ID = ? LIMIT 1"
            }
            Self::SEL_GOSSIP_MENU_OPTIONS => concat!(
                "SELECT GossipOptionID, OptionID, OptionNpc, OptionText, ",
                "ActionMenuID, BoxCoded, BoxMoney, BoxText, SpellID, OverrideIconID, ",
                "OptionBroadcastTextID ",
                "FROM gossip_menu_option WHERE MenuID = ? ORDER BY OptionID ASC",
            ),
            Self::SEL_BROADCAST_TEXT_LOCALE => {
                "SELECT Text_lang FROM hotfixes.broadcast_text_locale WHERE ID = ? AND locale = ?"
            }
            Self::SEL_CREATURE_TEMPLATE_LOCALE => {
                "SELECT Name, NameAlt, Title, TitleAlt FROM creature_template_locale WHERE entry = ? AND locale = ?"
            }
            Self::SEL_VENDOR_ITEM_PRICE => concat!(
                "SELECT COALESCE(isp.BuyPrice, 0), COALESCE(isp.SellPrice, 0), ",
                "COALESCE(isp.MaxDurability, 0) ",
                "FROM npc_vendor nv ",
                "LEFT JOIN hotfixes.item_sparse isp ON nv.item = isp.ID ",
                "WHERE nv.entry = ? AND nv.item = ? LIMIT 1",
            ),
            Self::SEL_ITEM_SELL_PRICE => {
                "SELECT COALESCE(SellPrice, 0) FROM hotfixes.item_sparse WHERE ID = ? LIMIT 1"
            }
            Self::SEL_AREA_TRIGGER_TELEPORT => {
                "SELECT at.ID, wsl.MapID, wsl.LocX, wsl.LocY, wsl.LocZ, wsl.Facing FROM areatrigger_teleport at LEFT JOIN world_safe_locs wsl ON at.PortLocID = wsl.ID"
            }
            Self::SEL_TRAINER_BY_CREATURE => {
                "SELECT TrainerId FROM creature_trainer WHERE CreatureID = ?"
            }
            Self::SEL_TRAINER_SPELLS => {
                "SELECT SpellId, MoneyCost, ReqSkillLine, ReqSkillRank, \
                 ReqAbility1, ReqAbility2, ReqAbility3, ReqLevel \
                 FROM trainer_spell WHERE TrainerId = ?"
            }
            Self::SEL_TRAINER_INFO => {
                "SELECT Id, Type, Greeting FROM trainer WHERE Id = ?"
            }
            Self::SEL_QUEST_TEMPLATE => concat!(
                "SELECT qt.ID, qt.QuestType, qt.QuestLevel, qt.QuestMaxScalingLevel, qt.MinLevel, qt.QuestSortID, ",
                "qt.QuestInfoID, qt.SuggestedGroupNum, qt.RewardNextQuest, qt.RewardXPDifficulty, qt.RewardXPMultiplier, ",
                "qt.RewardMoneyDifficulty, qt.RewardMoneyMultiplier, qt.RewardBonusMoney, ",
                "qt.RewardDisplaySpell1, qt.RewardDisplaySpell2, qt.RewardDisplaySpell3, ",
                "qt.RewardSpell, qt.RewardHonor, qt.Flags, qt.FlagsEx, qt.FlagsEx2, ",
                "qt.RewardItem1, qt.RewardAmount1, qt.RewardItem2, qt.RewardAmount2, ",
                "qt.RewardItem3, qt.RewardAmount3, qt.RewardItem4, qt.RewardAmount4, ",
                "qt.LogTitle, qt.LogDescription, qt.QuestDescription, qt.AreaDescription, qt.QuestCompletionLog, ",
                "COALESCE(qt.AllowableRaces, 0) AS AllowableRaces, ",
                "COALESCE(qta.AllowableClasses, 0) AS AllowableClasses, ",
                "COALESCE(qta.MaxLevel, 0) AS MaxLevel, ",
                "COALESCE(qta.PrevQuestID, 0) AS PrevQuestID, ",
                "qt.RewardChoiceItemID1, qt.RewardChoiceItemQuantity1, ",
                "qt.RewardChoiceItemID2, qt.RewardChoiceItemQuantity2, ",
                "qt.RewardChoiceItemID3, qt.RewardChoiceItemQuantity3, ",
                "qt.RewardChoiceItemID4, qt.RewardChoiceItemQuantity4, ",
                "qt.RewardChoiceItemID5, qt.RewardChoiceItemQuantity5, ",
                "qt.RewardChoiceItemID6, qt.RewardChoiceItemQuantity6 ",
                "FROM quest_template qt LEFT JOIN quest_template_addon qta ON qt.ID = qta.ID"
            ),
            Self::SEL_QUEST_OBJECTIVES => {
                "SELECT ID, QuestID, Type, `Order`, StorageIndex, ObjectID, Amount, Flags, Flags2, ProgressBarWeight, Description FROM quest_objectives ORDER BY QuestID, `Order`"
            }
            Self::SEL_QUEST_STARTERS => {
                "SELECT id, quest FROM creature_queststarter"
            }
            Self::SEL_QUEST_ENDERS => {
                "SELECT id, quest FROM creature_questender"
            }
        }
    }
}
