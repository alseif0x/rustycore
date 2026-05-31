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
    SEL_LINKED_RESPAWNS,
    SEL_CREATURE_TEXT,
    SEL_SMART_SCRIPTS,
    DEL_GAMEOBJECT,
    DEL_EVENT_GAMEOBJECT,
    SEL_WORLD_SAFE_LOCS,
    SEL_GRAVEYARD_ZONE,
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
    SEL_WAYPOINT_PATHS,
    SEL_WAYPOINT_PATH_NODES,
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
    SEL_CREATURE_TEMPLATE_IDS,
    /// Load all creature spawn GUID/entry pairs for C++ ConditionMgr validation.
    SEL_CREATURE_SPAWN_IDS,
    /// Load all gameobject spawn GUID/entry pairs for C++ ConditionMgr validation.
    SEL_GAMEOBJECT_SPAWN_IDS,
    /// Load valid game event IDs for C++ ConditionMgr ActiveEvent validation.
    SEL_VALID_GAME_EVENT_IDS,
    /// Load world-state template IDs for C++ ConditionMgr WorldState validation.
    SEL_WORLD_STATE_IDS,
    /// Load C++ WorldStateMgr templates/default metadata.
    SEL_WORLD_STATES,
    /// C++ ObjectMgr::LoadReputationRewardRate startup query.
    SEL_REPUTATION_REWARD_RATE,
    /// C++ ObjectMgr::LoadReputationOnKill startup query.
    SEL_CREATURE_ONKILL_REPUTATION,
    /// C++ ObjectMgr::LoadReputationSpilloverTemplate startup query.
    SEL_REPUTATION_SPILLOVER_TEMPLATE,
    /// SELECT Experience FROM player_xp_for_level ORDER BY Level
    SEL_PLAYER_XP_FOR_LEVEL,
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
    /// Load all creature spawn rows into the C++ ObjectMgr-style spawn store.
    SEL_CREATURE_SPAWNS,
    /// C++ FormationMgr::LoadCreatureFormations startup query.
    SEL_CREATURE_FORMATIONS,
    /// Load creature template for query response (name, type, display, etc.).
    SEL_CREATURE_QUERY_RESPONSE,
    /// Load creature display models for a template entry.
    SEL_CREATURE_DISPLAY_MODELS,
    /// Load gameobjects in a bounding box around a position on a map.
    SEL_GAMEOBJECTS_IN_RANGE,
    /// Load all gameobject spawn rows into the C++ ObjectMgr-style spawn store.
    SEL_GAMEOBJECT_SPAWNS,
    /// Load all static areatrigger spawn rows into the C++ AreaTriggerDataStore-style spawn store.
    SEL_AREATRIGGER_SPAWNS,
    /// Load C++ terrain world map definitions.
    SEL_TERRAIN_WORLD_MAPS,
    /// Load C++ terrain swap default definitions.
    SEL_TERRAIN_SWAP_DEFAULTS,
    /// Load C++ phase area definitions.
    SEL_PHASE_AREAS,
    /// Load C++ spawn group templates.
    SEL_SPAWN_GROUP_TEMPLATES,
    /// Load C++ spawn group members.
    SEL_SPAWN_GROUP_MEMBERS,
    /// Load C++ PoolMgr pool templates.
    SEL_POOL_TEMPLATES,
    /// Load C++ PoolMgr pool members filtered by type.
    SEL_POOL_MEMBERS_BY_TYPE,
    /// Load C++ PoolMgr default autospawn candidates.
    SEL_POOL_AUTOSPAWN_CANDIDATES,
    /// C++ GameEventMgr::Initialize max game_event entry sizing query.
    SEL_MAX_GAME_EVENT_ENTRY,
    /// C++ GameEventMgr::LoadFromDB game_event master metadata query.
    SEL_GAME_EVENTS,
    /// C++ GameEventMgr::LoadFromDB game_event_prerequisite metadata query.
    SEL_GAME_EVENT_PREREQUISITES,
    /// C++ GameEventMgr::LoadFromDB game_event_condition metadata query.
    SEL_GAME_EVENT_CONDITIONS,
    /// C++ GameEventMgr::LoadFromDB game_event_quest_condition metadata query.
    SEL_GAME_EVENT_QUEST_CONDITIONS,
    /// C++ GameEventMgr::LoadFromDB game_event_pool metadata query.
    SEL_GAME_EVENT_POOLS,
    /// C++ GameEventMgr::LoadFromDB game_event_creature metadata query.
    SEL_GAME_EVENT_CREATURES,
    /// C++ GameEventMgr::LoadFromDB game_event_gameobject metadata query.
    SEL_GAME_EVENT_GAMEOBJECTS,
    /// C++ ObjectMgr::GetEquipmentInfo existence keys for game_event_model_equip validation.
    SEL_CREATURE_EQUIP_TEMPLATE_IDS,
    /// C++ GameEventMgr::LoadFromDB game_event_model_equip metadata query.
    SEL_GAME_EVENT_MODEL_EQUIP,
    /// C++ GameEventMgr::LoadFromDB game_event_creature_quest metadata query.
    SEL_GAME_EVENT_CREATURE_QUESTS,
    /// C++ GameEventMgr::LoadFromDB game_event_gameobject_quest metadata query.
    SEL_GAME_EVENT_GAMEOBJECT_QUESTS,
    /// C++ GameEventMgr::LoadFromDB game_event_npcflag metadata query.
    SEL_GAME_EVENT_NPC_FLAGS,
    /// C++ GameEventMgr::LoadFromDB game_event_npc_vendor metadata query.
    SEL_GAME_EVENT_NPC_VENDOR,
    /// C++ ObjectMgr::LoadNPCSpellClickSpells startup query.
    SEL_NPC_SPELLCLICK_SPELLS,
    /// Load C++ instance spawn groups.
    SEL_INSTANCE_SPAWN_GROUPS,
    /// Load gameobject template for query response.
    SEL_GAMEOBJECT_TEMPLATE_BY_ENTRY,
    /// Localized gameobject name/castbar/unk by entry and locale.
    SEL_GAMEOBJECT_TEMPLATE_LOCALE,
    /// C++ ObjectMgr gameobject quest item list by entry.
    SEL_GAMEOBJECT_QUEST_ITEMS,
    /// Static page text by page ID.
    SEL_PAGE_TEXT,
    /// Localized static page text by page ID and locale.
    SEL_PAGE_TEXT_LOCALE,
    SEL_GAMEOBJECT_TEMPLATE_IDS,
    /// SELECT InventoryType FROM item_template WHERE entry = ?
    SEL_ITEM_INVENTORY_TYPE,
    /// Load base stats for all race/class/level combos.
    SEL_PLAYER_LEVELSTATS,
    /// Load initial action buttons for character creation.
    SEL_PLAYER_CREATEINFO_ACTION,
    /// Gossip MenuID for a creature entry (creature_template_gossip).
    SEL_CREATURE_GOSSIP_MENU,
    /// Gossip menu text ID (gossip_menu).
    SEL_GOSSIP_MENU,
    /// Gossip menu text IDs (gossip_menu), used for C++ condition-based text selection.
    SEL_GOSSIP_MENU_TEXTS,
    /// Load all C++ ObjectMgr gossip_menu keys.
    SEL_GOSSIP_MENUS,
    /// NPC text BroadcastTextID by npc_text ID.
    SEL_NPC_TEXT,
    /// Gossip menu options (gossip_menu_option) — includes OptionBroadcastTextID for localization.
    SEL_GOSSIP_MENU_OPTIONS,
    /// Load all C++ ObjectMgr gossip_menu_option condition keys.
    SEL_GOSSIP_MENU_OPTION_KEYS,
    /// Localized text from broadcast_text_locale by ID and locale.
    SEL_BROADCAST_TEXT_LOCALE,
    /// Localized creature name/subname/title by entry and locale.
    SEL_CREATURE_TEMPLATE_LOCALE,
    /// Buy price + sell price + durability + vendor stack count for a specific item in a vendor's list.
    /// Args: npc_vendor.entry (u32), npc_vendor.item (u32).
    SEL_VENDOR_ITEM_PRICE,
    /// Sell price for any item directly from item_sparse (no vendor check).
    /// Args: item ID (u32).
    SEL_ITEM_SELL_PRICE,
    /// Min/max money loot bounds for an item from item_template_addon.
    /// Args: item ID (u32).
    SEL_ITEM_TEMPLATE_ADDON_MONEY_LOOT,
    /// Min/max money loot bounds for a gameobject from gameobject_template_addon.
    /// Args: gameobject template entry (u32).
    SEL_GAMEOBJECT_TEMPLATE_ADDON_MONEY_LOOT,
    /// FlagsCu and QuestLogItemId for loot eligibility from item_template_addon.
    /// Args: item ID (u32).
    SEL_ITEM_TEMPLATE_ADDON_LOOT_METADATA,
    /// Non-group item_loot_template rows for an item.
    /// Args: item ID (u32).
    SEL_ITEM_LOOT_TEMPLATE_ROWS,
    /// All item_loot_template rows for startup loading.
    SEL_ITEM_LOOT_TEMPLATE_ALL_ROWS,
    /// creature_loot_template rows for a creature loot ID.
    /// Args: creature loot ID (u32).
    SEL_CREATURE_LOOT_TEMPLATE_ROWS,
    /// All creature_loot_template rows for startup loading.
    SEL_CREATURE_LOOT_TEMPLATE_ALL_ROWS,
    /// fishing_loot_template rows for an area ID.
    /// Args: area ID (u32).
    SEL_FISHING_LOOT_TEMPLATE_ROWS,
    /// All fishing_loot_template rows for startup loading.
    SEL_FISHING_LOOT_TEMPLATE_ALL_ROWS,
    /// All C++ ObjectMgr fishing base skill levels by AreaTable ID.
    SEL_FISHING_BASE_SKILL_LEVELS,
    /// gameobject_loot_template rows for a gameobject loot ID.
    /// Args: gameobject loot ID (u32).
    SEL_GAMEOBJECT_LOOT_TEMPLATE_ROWS,
    /// All gameobject_loot_template rows for startup loading.
    SEL_GAMEOBJECT_LOOT_TEMPLATE_ALL_ROWS,
    /// mail_loot_template rows for a mail template ID.
    /// Args: mail template ID (u32).
    SEL_MAIL_LOOT_TEMPLATE_ROWS,
    /// All mail_loot_template rows for startup loading.
    SEL_MAIL_LOOT_TEMPLATE_ALL_ROWS,
    /// milling_loot_template rows for an herb item entry.
    /// Args: item ID (u32).
    SEL_MILLING_LOOT_TEMPLATE_ROWS,
    /// All milling_loot_template rows for startup loading.
    SEL_MILLING_LOOT_TEMPLATE_ALL_ROWS,
    /// pickpocketing_loot_template rows for a creature pickpocket loot ID.
    /// Args: creature pickpocket loot ID (u32).
    SEL_PICKPOCKETING_LOOT_TEMPLATE_ROWS,
    /// All pickpocketing_loot_template rows for startup loading.
    SEL_PICKPOCKETING_LOOT_TEMPLATE_ALL_ROWS,
    /// prospecting_loot_template rows for an ore item entry.
    /// Args: item ID (u32).
    SEL_PROSPECTING_LOOT_TEMPLATE_ROWS,
    /// All prospecting_loot_template rows for startup loading.
    SEL_PROSPECTING_LOOT_TEMPLATE_ALL_ROWS,
    /// Non-group reference_loot_template rows for a reference entry.
    /// Args: reference ID (u32).
    SEL_REFERENCE_LOOT_TEMPLATE_ROWS,
    /// All reference_loot_template rows for startup loading.
    SEL_REFERENCE_LOOT_TEMPLATE_ALL_ROWS,
    /// skinning_loot_template rows for a creature skinning loot ID.
    /// Args: creature skinning loot ID (u32).
    SEL_SKINNING_LOOT_TEMPLATE_ROWS,
    /// All skinning_loot_template rows for startup loading.
    SEL_SKINNING_LOOT_TEMPLATE_ALL_ROWS,
    /// disenchant_loot_template rows for an ItemDisenchantLoot.db2 ID.
    /// Args: disenchant loot ID (u32).
    SEL_DISENCHANT_LOOT_TEMPLATE_ROWS,
    /// All disenchant_loot_template rows for startup loading.
    SEL_DISENCHANT_LOOT_TEMPLATE_ALL_ROWS,
    /// spell_loot_template rows for a spell loot ID.
    /// Args: spell ID (u32).
    SEL_SPELL_LOOT_TEMPLATE_ROWS,
    /// All spell_loot_template rows for startup loading.
    SEL_SPELL_LOOT_TEMPLATE_ALL_ROWS,
    /// Load C++ ConditionMgr loot-template conditions.
    /// Args: SourceTypeOrReferenceId (i32), SourceGroup (u32), SourceEntry (u32).
    SEL_LOOT_TEMPLATE_CONDITION_ROWS,
    /// Load distinct C++ ConditionMgr loot-template condition IDs for startup validation.
    SEL_LOOT_TEMPLATE_CONDITION_IDS,
    /// Load distinct C++ ConditionMgr loot condition-reference uses for startup validation.
    SEL_LOOT_TEMPLATE_CONDITION_REFERENCE_USES,
    /// Load distinct C++ ConditionMgr reference-condition template IDs for startup validation.
    SEL_CONDITION_REFERENCE_TEMPLATE_IDS,
    /// Load all C++ ConditionMgr conditions rows.
    SEL_CONDITIONS,
    /// Load C++ ItemEnchantmentMgr random enchantment groups.
    SEL_ITEM_RANDOM_ENCHANTMENT_TEMPLATE,
    /// Load all area trigger teleport destinations.
    SEL_AREA_TRIGGER_TELEPORT,
    // Quest system
    SEL_QUEST_TEMPLATE,
    SEL_QUEST_OBJECTIVES,
    /// C++ GameEventMgr::LoadFromDB seasonal quest relation query.
    SEL_GAME_EVENT_SEASONAL_QUEST_RELATIONS,
    SEL_QUEST_STARTERS,
    SEL_QUEST_ENDERS,
    SEL_GAMEOBJECT_QUEST_STARTERS,
    SEL_GAMEOBJECT_QUEST_ENDERS,
    /// Get TrainerId from creature_trainer by creature entry (NPC template ID).
    SEL_TRAINER_BY_CREATURE,
    /// Load all spells for a trainer by TrainerId.
    SEL_TRAINER_SPELLS,
    /// Load trainer type and greeting by trainer ID.
    SEL_TRAINER_INFO,
    /// Load trainer IDs for C++ ConditionMgr source validation.
    SEL_TRAINER_IDS,
    /// Load conversation line template IDs for C++ ConditionMgr source validation.
    SEL_CONVERSATION_LINE_TEMPLATE_IDS,
    /// Load area-trigger template keys for C++ ConditionMgr source validation.
    SEL_AREA_TRIGGER_TEMPLATE_IDS,
}

impl StatementDef for WorldStatements {
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
            Self::INS_NPC_VENDOR => {
                "INSERT INTO npc_vendor (entry, item, maxcount, incrtime, extendedcost, type) VALUES(?, ?, ?, ?, ?, ?)"
            }
            Self::DEL_NPC_VENDOR => {
                "DELETE FROM npc_vendor WHERE entry = ? AND item = ? AND type = ?"
            }
            Self::SEL_NPC_VENDOR_REF => {
                "SELECT item, maxcount, incrtime, ExtendedCost, type, BonusListIDs, PlayerConditionID, IgnoreFiltering FROM npc_vendor WHERE entry = ? ORDER BY slot ASC"
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
                "CASE WHEN ca.guid IS NOT NULL AND ca.PathId = 0 AND c.MovementType = 2 THEN 0 ELSE c.MovementType END, ",
                "COALESCE(ca.PathId, cta.PathId, 0) ",
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
                "COALESCE(cmo.Ground, ctm.Ground, 1), COALESCE(cmo.Swim, ctm.Swim, 1), COALESCE(cmo.Flight, ctm.Flight, 0) ",
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
                "CASE WHEN goo.spawnId IS NOT NULL OR gta.entry IS NOT NULL THEN 1 ELSE 0 END ",
                "FROM gameobject g ",
                "JOIN gameobject_template gt ON g.id = gt.entry ",
                "LEFT JOIN gameobject_template_addon gta ON gta.entry = g.id ",
                "LEFT JOIN gameobject_overrides goo ON goo.spawnId = g.guid ",
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
            Self::SEL_INSTANCE_SPAWN_GROUPS => {
                "SELECT instanceMapId, bossStateId, bossStates, spawnGroupId, flags FROM instance_spawn_groups"
            }
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
                "Data32, Data33, Data34, ContentTuningId ",
                "FROM gameobject_template WHERE entry = ?",
            ),
            Self::SEL_GAMEOBJECT_TEMPLATE_LOCALE => {
                "SELECT Name, CastBarCaption, Unk1 FROM gameobject_template_locale WHERE entry = ? AND locale = ?"
            }
            Self::SEL_GAMEOBJECT_QUEST_ITEMS => {
                "SELECT ItemId FROM gameobject_questitem WHERE GameObjectEntry = ? ORDER BY Idx ASC"
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
                "SELECT GossipOptionID, OptionID, OptionNpc, OptionText, ",
                "ActionMenuID, BoxCoded, BoxMoney, BoxText, SpellID, OverrideIconID, ",
                "OptionBroadcastTextID ",
                "FROM gossip_menu_option WHERE MenuID = ? ORDER BY OptionID ASC",
            ),
            Self::SEL_GOSSIP_MENU_OPTION_KEYS => {
                "SELECT MenuID, OptionID FROM gossip_menu_option ORDER BY MenuID, OptionID"
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
                "SELECT LEAST(COALESCE(MinMoneyLoot, 0), COALESCE(MaxMoneyLoot, 0)), ",
                "GREATEST(COALESCE(MinMoneyLoot, 0), COALESCE(MaxMoneyLoot, 0)) ",
                "FROM item_template_addon WHERE Id = ? LIMIT 1",
            ),
            Self::SEL_GAMEOBJECT_TEMPLATE_ADDON_MONEY_LOOT => concat!(
                "SELECT LEAST(COALESCE(mingold, 0), COALESCE(maxgold, 0)), ",
                "GREATEST(COALESCE(mingold, 0), COALESCE(maxgold, 0)) ",
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
            Self::SEL_TRAINER_BY_CREATURE => {
                "SELECT TrainerId FROM creature_trainer WHERE CreatureID = ?"
            }
            Self::SEL_TRAINER_SPELLS => {
                "SELECT SpellId, MoneyCost, ReqSkillLine, ReqSkillRank, \
                 ReqAbility1, ReqAbility2, ReqAbility3, ReqLevel \
                 FROM trainer_spell WHERE TrainerId = ?"
            }
            Self::SEL_TRAINER_INFO => "SELECT Id, Type, Greeting FROM trainer WHERE Id = ?",
            Self::SEL_TRAINER_IDS => "SELECT Id FROM trainer",
            Self::SEL_CONVERSATION_LINE_TEMPLATE_IDS => "SELECT Id FROM conversation_line_template",
            Self::SEL_AREA_TRIGGER_TEMPLATE_IDS => "SELECT Id, IsCustom FROM areatrigger_template",
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
                "COALESCE(qt.AllowableRaces, 0) AS AllowableRaces, ",
                "COALESCE(qta.AllowableClasses, 0) AS AllowableClasses, ",
                "COALESCE(qta.MaxLevel, 0) AS MaxLevel, ",
                "COALESCE(qta.PrevQuestID, 0) AS PrevQuestID, ",
                "COALESCE(qta.RequiredMinRepFaction, 0) AS RequiredMinRepFaction, ",
                "COALESCE(qta.RequiredMinRepValue, 0) AS RequiredMinRepValue, ",
                "COALESCE(qta.RequiredMaxRepFaction, 0) AS RequiredMaxRepFaction, ",
                "COALESCE(qta.RequiredMaxRepValue, 0) AS RequiredMaxRepValue, ",
                "qt.RewardChoiceItemID1, qt.RewardChoiceItemQuantity1, ",
                "qt.RewardChoiceItemID2, qt.RewardChoiceItemQuantity2, ",
                "qt.RewardChoiceItemID3, qt.RewardChoiceItemQuantity3, ",
                "qt.RewardChoiceItemID4, qt.RewardChoiceItemQuantity4, ",
                "qt.RewardChoiceItemID5, qt.RewardChoiceItemQuantity5, ",
                "qt.RewardChoiceItemID6, qt.RewardChoiceItemQuantity6, ",
                "COALESCE(qta.NextQuestID, 0) AS NextQuestID, ",
                "COALESCE(qta.ExclusiveGroup, 0) AS ExclusiveGroup, ",
                "COALESCE(qta.BreadcrumbForQuestId, 0) AS BreadcrumbForQuestId, ",
                "COALESCE(qta.SpecialFlags, 0) AS SpecialFlags, ",
                "qt.Expansion, ",
                "qt.StartItem, ",
                "COALESCE(qta.SourceSpellID, 0) AS SourceSpellID, ",
                "COALESCE(qta.ProvidedItemCount, 0) AS ProvidedItemCount, ",
                "qt.TimeAllowed, ",
                "COALESCE(qrci.Type1, 0) AS RewardChoiceItemType1, ",
                "COALESCE(qrci.Type2, 0) AS RewardChoiceItemType2, ",
                "COALESCE(qrci.Type3, 0) AS RewardChoiceItemType3, ",
                "COALESCE(qrci.Type4, 0) AS RewardChoiceItemType4, ",
                "COALESCE(qrci.Type5, 0) AS RewardChoiceItemType5, ",
                "COALESCE(qrci.Type6, 0) AS RewardChoiceItemType6, ",
                "qt.RewardCurrencyID1, qt.RewardCurrencyQty1, ",
                "qt.RewardCurrencyID2, qt.RewardCurrencyQty2, ",
                "qt.RewardCurrencyID3, qt.RewardCurrencyQty3, ",
                "qt.RewardCurrencyID4, qt.RewardCurrencyQty4, ",
                "qt.RewardSkillLineID, qt.RewardNumSkillUps, qt.RewardTitle, ",
                "COALESCE(qta.RewardMailTemplateID, 0) AS RewardMailTemplateID, ",
                "COALESCE(qta.RewardMailDelay, 0) AS RewardMailDelay, ",
                "COALESCE(qms.RewardMailSenderEntry, 0) AS RewardMailSenderEntry, ",
                "qt.RewardFactionID1, qt.RewardFactionValue1, qt.RewardFactionOverride1, qt.RewardFactionCapIn1, ",
                "qt.RewardFactionID2, qt.RewardFactionValue2, qt.RewardFactionOverride2, qt.RewardFactionCapIn2, ",
                "qt.RewardFactionID3, qt.RewardFactionValue3, qt.RewardFactionOverride3, qt.RewardFactionCapIn3, ",
                "qt.RewardFactionID4, qt.RewardFactionValue4, qt.RewardFactionOverride4, qt.RewardFactionCapIn4, ",
                "qt.RewardFactionID5, qt.RewardFactionValue5, qt.RewardFactionOverride5, qt.RewardFactionCapIn5, ",
                "qt.RewardFactionFlags, ",
                "COALESCE(qta.RequiredSkillID, 0) AS RequiredSkillID, ",
                "COALESCE(qta.RequiredSkillPoints, 0) AS RequiredSkillPoints ",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creatures_in_range_selects_addon_path_and_effective_movement_like_cpp() {
        let sql = WorldStatements::SEL_CREATURES_IN_RANGE.sql();

        assert!(sql.contains("LEFT JOIN creature_addon ca ON ca.guid = c.guid"));
        assert!(sql.contains("LEFT JOIN creature_template_addon cta ON cta.entry = c.id"));
        assert!(
            sql.contains(
                "CASE WHEN ca.guid IS NOT NULL AND ca.PathId = 0 AND c.MovementType = 2 THEN 0 ELSE c.MovementType END"
            ),
            "C++ ObjectMgr::LoadCreatureAddons downgrades spawn waypoint movement when a spawn addon has PathId=0"
        );
        assert!(sql.contains("COALESCE(ca.PathId, cta.PathId, 0)"));
        assert_eq!(sql.matches('?').count(), 5);
    }

    #[test]
    fn world_state_load_statement_matches_cpp_sql_exactly() {
        let sql = WorldStatements::SEL_WORLD_STATES.sql();
        assert_eq!(
            sql,
            "SELECT ID, DefaultValue, MapIDs, AreaIDs, ScriptName FROM world_state"
        );
        assert_eq!(sql.matches('?').count(), 0);
        assert_eq!(
            WorldStatements::SEL_WORLD_STATE_IDS.sql(),
            "SELECT ID FROM world_state"
        );
    }

    #[test]
    fn game_event_condition_statement_matches_cpp_sql_exactly() {
        let sql = WorldStatements::SEL_GAME_EVENT_CONDITIONS.sql();
        assert_eq!(
            sql,
            "SELECT eventEntry, condition_id, req_num, max_world_state_field, done_world_state_field FROM game_event_condition"
        );
        assert_eq!(sql.matches('?').count(), 0);
    }

    #[test]
    fn reputation_reward_rate_statement_matches_cpp_sql_exactly() {
        let sql = WorldStatements::SEL_REPUTATION_REWARD_RATE.sql();
        assert_eq!(
            sql,
            "SELECT faction, quest_rate, quest_daily_rate, quest_weekly_rate, quest_monthly_rate, quest_repeatable_rate, creature_rate, spell_rate FROM reputation_reward_rate"
        );
        assert_eq!(sql.matches('?').count(), 0);
    }

    #[test]
    fn creature_onkill_reputation_statement_matches_cpp_sql_exactly() {
        let sql = WorldStatements::SEL_CREATURE_ONKILL_REPUTATION.sql();
        assert_eq!(
            sql,
            "SELECT creature_id, RewOnKillRepFaction1, RewOnKillRepFaction2, IsTeamAward1, MaxStanding1, RewOnKillRepValue1, IsTeamAward2, MaxStanding2, RewOnKillRepValue2, TeamDependent FROM creature_onkill_reputation"
        );
        assert_eq!(sql.matches('?').count(), 0);
    }

    #[test]
    fn reputation_spillover_template_statement_matches_cpp_sql_exactly() {
        let sql = WorldStatements::SEL_REPUTATION_SPILLOVER_TEMPLATE.sql();
        assert_eq!(
            sql,
            "SELECT faction, faction1, rate_1, rank_1, faction2, rate_2, rank_2, faction3, rate_3, rank_3, faction4, rate_4, rank_4, faction5, rate_5, rank_5 FROM reputation_spillover_template"
        );
        assert_eq!(sql.matches('?').count(), 0);
    }

    #[test]
    fn game_event_quest_condition_statement_matches_cpp_sql_exactly() {
        let sql = WorldStatements::SEL_GAME_EVENT_QUEST_CONDITIONS.sql();
        assert_eq!(
            sql,
            "SELECT quest, eventEntry, condition_id, num FROM game_event_quest_condition"
        );
        assert_eq!(sql.matches('?').count(), 0);
    }

    #[test]
    fn game_event_seasonal_questrelation_statement_matches_cpp_sql_exactly() {
        let sql = WorldStatements::SEL_GAME_EVENT_SEASONAL_QUEST_RELATIONS.sql();
        assert_eq!(
            sql,
            "SELECT questId, eventEntry FROM game_event_seasonal_questrelation"
        );
        assert_eq!(sql.matches('?').count(), 0);
    }

    #[test]
    fn npc_spellclick_spells_statement_matches_cpp_sql_exactly() {
        let sql = WorldStatements::SEL_NPC_SPELLCLICK_SPELLS.sql();
        assert_eq!(
            sql,
            "SELECT npc_entry, spell_id, cast_flags, user_type FROM npc_spellclick_spells"
        );
        assert_eq!(sql.matches('?').count(), 0);
    }

    #[test]
    fn quest_template_statement_loads_reward_choice_item_types_like_cpp() {
        let sql = WorldStatements::SEL_QUEST_TEMPLATE.sql();

        assert!(sql.contains("qt.QuestPackageID"));
        assert!(sql.contains("COALESCE(qrci.Type1, 0) AS RewardChoiceItemType1"));
        assert!(sql.contains("COALESCE(qrci.Type6, 0) AS RewardChoiceItemType6"));
        assert!(sql.contains("LEFT JOIN quest_reward_choice_items qrci ON qt.ID = qrci.QuestID"));
        assert!(sql.contains("qt.RewardCurrencyID1, qt.RewardCurrencyQty1"));
        assert!(sql.contains("qt.RewardCurrencyID4, qt.RewardCurrencyQty4"));
        assert!(sql.contains("qt.RewardSkillLineID, qt.RewardNumSkillUps, qt.RewardTitle"));
        assert!(sql.contains("COALESCE(qta.RewardMailTemplateID, 0) AS RewardMailTemplateID"));
        assert!(sql.contains("COALESCE(qta.RewardMailDelay, 0) AS RewardMailDelay"));
        assert!(sql.contains("COALESCE(qms.RewardMailSenderEntry, 0) AS RewardMailSenderEntry"));
        assert!(sql.contains("LEFT JOIN quest_mail_sender qms ON qt.ID = qms.QuestId"));
        assert!(sql.contains("qt.RewardFactionID1, qt.RewardFactionValue1"));
        assert!(sql.contains("qt.RewardFactionID5, qt.RewardFactionValue5"));
        assert!(sql.contains("qt.RewardFactionFlags"));
        assert!(sql.contains("COALESCE(qta.RequiredSkillID, 0) AS RequiredSkillID"));
        assert!(sql.contains("COALESCE(qta.RequiredSkillPoints, 0) AS RequiredSkillPoints"));
    }

    #[test]
    fn gameobject_quest_relation_statements_match_cpp_sql_exactly() {
        let starter_sql = WorldStatements::SEL_GAMEOBJECT_QUEST_STARTERS.sql();
        let ender_sql = WorldStatements::SEL_GAMEOBJECT_QUEST_ENDERS.sql();

        assert_eq!(starter_sql, "SELECT id, quest FROM gameobject_queststarter");
        assert_eq!(ender_sql, "SELECT id, quest FROM gameobject_questender");
        assert_eq!(starter_sql.matches('?').count(), 0);
        assert_eq!(ender_sql.matches('?').count(), 0);
    }
}
