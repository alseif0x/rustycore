//! The world-database statement identity list.
//!
//! Separated from the world.rs root under #652. Behaviour is preserved.

use super::*;

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
    /// C++ `ObjectMgr::LoadGameTele` startup query.
    SEL_GAME_TELE,
    INS_NPC_VENDOR,
    DEL_NPC_VENDOR,
    SEL_NPC_VENDOR_REF,
    /// C++ `ObjectMgr::LoadVendors` full startup query.
    SEL_NPC_VENDORS_ALL,
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
    /// Joined transport materialization row used by the represented login path.
    SEL_LOGIN_TRANSPORTS,
    /// One joined transport materialization row by spawn GUID.
    SEL_LOGIN_TRANSPORT_BY_GUID,
    /// C++ World::LoadDBVersion startup query.
    SEL_WORLD_DB_VERSION,
    /// C++ ObjectMgr::LoadReputationRewardRate startup query.
    SEL_REPUTATION_REWARD_RATE,
    /// C++ ObjectMgr::LoadReputationOnKill startup query.
    SEL_CREATURE_ONKILL_REPUTATION,
    /// C++ ObjectMgr::LoadReputationSpilloverTemplate startup query.
    SEL_REPUTATION_SPILLOVER_TEMPLATE,
    /// SELECT Experience FROM player_xp_for_level ORDER BY Level
    SEL_PLAYER_XP_FOR_LEVEL,
    /// C++ ObjectMgr exploration base XP by area level.
    SEL_EXPLORATION_BASE_XP,
    /// C++ `CollectionMgr::LoadMountDefinitions` startup query.
    SEL_MOUNT_DEFINITIONS,
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
    /// C++ `LFGMgr::LoadLFGDungeons` template overlay query.
    SEL_LFG_DUNGEON_TEMPLATES,
    /// C++ `LFGMgr::LoadRewards` startup query.
    SEL_LFG_DUNGEON_REWARDS,
    /// Load C++ instance spawn groups.
    SEL_INSTANCE_SPAWN_GROUPS,
    /// Load gameobject template for query response.
    SEL_GAMEOBJECT_TEMPLATE_BY_ENTRY,
    /// Localized gameobject name/castbar/unk by entry and locale.
    SEL_GAMEOBJECT_TEMPLATE_LOCALE,
    /// C++ ObjectMgr gameobject quest item list by entry.
    SEL_GAMEOBJECT_QUEST_ITEMS,
    /// C++ `ObjectMgr::LoadGameObjectQuestItems`.
    SEL_GAMEOBJECT_QUEST_ITEM_ROWS,
    /// C++ `ObjectMgr::LoadCreatureQuestItems`.
    SEL_CREATURE_QUEST_ITEM_ROWS,
    /// Static page text by page ID.
    SEL_PAGE_TEXT,
    /// Localized static page text by page ID and locale.
    SEL_PAGE_TEXT_LOCALE,
    SEL_GAMEOBJECT_TEMPLATE_IDS,
    /// SELECT InventoryType FROM item_template WHERE entry = ?
    SEL_ITEM_INVENTORY_TYPE,
    /// C++ `ObjectMgr::LoadPlayerInfo` race stat modifiers.
    SEL_PLAYER_RACESTATS,
    /// C++ `ObjectMgr::LoadPlayerInfo` class/level base stats.
    SEL_PLAYER_CLASSLEVELSTATS,
    /// Load initial action buttons for character creation.
    SEL_PLAYER_CREATEINFO_ACTION,
    /// C++ `ObjectMgr::LoadPlayerInfo` base playercreateinfo startup query.
    SEL_PLAYER_CREATEINFO,
    /// C++ `ObjectMgr::LoadPlayerInfo` playercreateinfo_cast_spell startup query.
    SEL_PLAYER_CREATEINFO_CAST_SPELL,
    /// C++ `ObjectMgr::LoadPlayerInfo` playercreateinfo_spell_custom startup query.
    SEL_PLAYER_CREATEINFO_CUSTOM_SPELL,
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
    /// C++ `ObjectMgr::LoadGossipMenuItems` column order, filtered by `MenuID`.
    SEL_GOSSIP_MENU_OPTIONS,
    /// Load all C++ ObjectMgr gossip_menu_option condition keys.
    SEL_GOSSIP_MENU_OPTION_KEYS,
    /// C++ `ObjectMgr::LoadGossipMenuItems` all rows.
    SEL_GOSSIP_MENU_OPTIONS_ALL,
    /// C++ `ObjectMgr::LoadGossipMenuItemsLocales`.
    SEL_GOSSIP_MENU_OPTION_LOCALES,
    /// C++ `ObjectMgr::LoadGossipMenuAddon`.
    SEL_GOSSIP_MENU_ADDON,
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
    /// C++ ObjectMgr skill tier max values.
    SEL_SKILL_TIERS,
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
    /// C++ ObjectMgr level-dependent mail rewards.
    SEL_MAIL_LEVEL_REWARDS,
    /// C++ ObjectMgr points of interest.
    SEL_POINTS_OF_INTEREST,
    /// C++ ObjectMgr points of interest locale rows.
    SEL_POINTS_OF_INTEREST_LOCALES,
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
    /// C++ SpellMgr::LoadSpellPetAuras startup query.
    SEL_SPELL_PET_AURAS,
    /// #159 startup audit inputs for deterministic trainer wrapper casts.
    SEL_TRAINER_CAST_SCRIPT_BINDING_IDS,
    SEL_TRAINER_CAST_LEGACY_SCRIPT_IDS,
    SEL_TRAINER_CAST_CONDITION_IDS,
    /// C++ SpellMgr::LoadSpellThreats startup query.
    SEL_SPELL_THREATS,
    /// C++ SpellMgr::LoadSpellEnchantProcData startup query.
    SEL_SPELL_ENCHANT_PROC_DATA,
    /// C++ SpellMgr::LoadSpellLinked startup query.
    SEL_SPELL_LINKED,
    /// C++ SpellMgr::LoadSpellTotemModel startup query.
    SEL_SPELL_TOTEM_MODEL,
    /// C++ SpellMgr::LoadSpellRequired startup query.
    SEL_SPELL_REQUIRED,
    /// C++ SpellMgr::LoadSpellLearnSpells startup query.
    SEL_SPELL_LEARN_SPELL,
    /// C++ SpellMgr::LoadSpellTargetPositions startup query.
    SEL_SPELL_TARGET_POSITION,
    /// C++ SpellMgr::LoadSpellGroups startup query.
    SEL_SPELL_GROUP,
    /// C++ SpellMgr::LoadSpellGroupStackRules startup query.
    SEL_SPELL_GROUP_STACK_RULES,
    /// C++ SpellMgr::LoadSpellProcs startup query.
    SEL_SPELL_PROC,
    /// C++ SpellMgr::LoadSpellAreas startup query.
    SEL_SPELL_AREA,
    /// C++ SpellMgr::LoadSpellInfoCustomAttributes startup query.
    SEL_SPELL_CUSTOM_ATTR,
    /// C++ SpellMgr::LoadSpellInfoServerside spell-effect startup query.
    SEL_SERVERSIDE_SPELL_EFFECT,
    /// C++ SpellMgr::LoadSpellInfoServerside spell startup query.
    SEL_SERVERSIDE_SPELL,
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
    /// C++ `ObjectMgr::LoadAreaTriggerScripts` startup query.
    SEL_AREA_TRIGGER_SCRIPTS,
    /// C++ `ObjectMgr::LoadAreaTriggerTeleports` relation query.
    SEL_AREA_TRIGGER_TELEPORT_RELATIONS,
    /// C++ `ObjectMgr::LoadQuestAreaTriggers` relation query.
    SEL_QUEST_AREA_TRIGGER_RELATIONS,
    /// C++ `ObjectMgr::LoadTavernAreaTriggers` startup query.
    SEL_TAVERN_AREA_TRIGGERS,
    /// C++ `ObjectMgr::LoadPhaseNames`.
    SEL_PHASE_NAMES,
    /// C++ `ObjectMgr::LoadSceneTemplates`.
    SEL_SCENE_TEMPLATES,
    /// C++ `ObjectMgr::LoadPlayerChoices` base choice rows.
    SEL_PLAYER_CHOICES,
    /// C++ `ObjectMgr::LoadPlayerChoices` response rows.
    SEL_PLAYER_CHOICE_RESPONSES,
    /// C++ `ObjectMgr::LoadPlayerChoices` response reward rows.
    SEL_PLAYER_CHOICE_RESPONSE_REWARDS,
    /// C++ `ObjectMgr::LoadPlayerChoices` response reward item rows.
    SEL_PLAYER_CHOICE_RESPONSE_REWARD_ITEMS,
    /// C++ `ObjectMgr::LoadPlayerChoices` response reward currency rows.
    SEL_PLAYER_CHOICE_RESPONSE_REWARD_CURRENCIES,
    /// C++ `ObjectMgr::LoadPlayerChoices` response reward faction rows.
    SEL_PLAYER_CHOICE_RESPONSE_REWARD_FACTIONS,
    /// C++ `ObjectMgr::LoadPlayerChoices` response reward item-choice rows.
    SEL_PLAYER_CHOICE_RESPONSE_REWARD_ITEM_CHOICES,
    /// C++ `ObjectMgr::LoadPlayerChoices` response Maw Power rows.
    SEL_PLAYER_CHOICE_RESPONSE_MAW_POWERS,
    /// C++ `ObjectMgr::LoadPlayerChoicesLocale` choice locale rows.
    SEL_PLAYER_CHOICE_LOCALES,
    /// C++ `ObjectMgr::LoadPlayerChoicesLocale` response locale rows.
    SEL_PLAYER_CHOICE_RESPONSE_LOCALES,
    /// C++ `ObjectMgr::LoadJumpChargeParams`.
    SEL_JUMP_CHARGE_PARAMS,
    // Quest system
    SEL_QUEST_TEMPLATE,
    SEL_QUEST_OBJECTIVES,
    /// C++ GameEventMgr::LoadFromDB seasonal quest relation query.
    SEL_GAME_EVENT_SEASONAL_QUEST_RELATIONS,
    SEL_QUEST_STARTERS,
    SEL_QUEST_ENDERS,
    SEL_GAMEOBJECT_QUEST_STARTERS,
    SEL_GAMEOBJECT_QUEST_ENDERS,
    /// C++ `ObjectMgr::GetCreatureDefaultTrainer` (`MenuID=0`, `OptionID=0`).
    SEL_TRAINER_BY_CREATURE,
    /// C++ `ObjectMgr::GetCreatureTrainerForGossipOption`.
    SEL_TRAINER_BY_CREATURE_GOSSIP_OPTION,
    /// Load all spells for a trainer by TrainerId.
    SEL_TRAINER_SPELLS,
    /// Load trainer type and greeting by trainer ID.
    SEL_TRAINER_INFO,
    /// C++ `ObjectMgr::LoadTrainers` full trainer_spell query.
    SEL_TRAINER_SPELLS_ALL,
    /// C++ `ObjectMgr::LoadTrainers` full trainer query.
    SEL_TRAINERS_ALL,
    /// C++ `ObjectMgr::LoadTrainers` trainer_locale query.
    SEL_TRAINER_LOCALES,
    /// C++ `ObjectMgr::LoadCreatureTrainers` full creature_trainer query.
    SEL_CREATURE_TRAINERS_ALL,
    /// C++ `BattlePetMgr::LoadAvailablePetBreeds` full battle_pet_breeds query.
    SEL_BATTLE_PET_BREEDS,
    /// C++ `BattlePetMgr::LoadDefaultPetQualities` full battle_pet_quality query.
    SEL_BATTLE_PET_QUALITY,
    /// C++ `ObjectMgr::LoadFactionChangeAchievements` startup query.
    SEL_FACTION_CHANGE_ACHIEVEMENTS,
    /// C++ `ObjectMgr::LoadFactionChangeQuests` startup query.
    SEL_FACTION_CHANGE_QUESTS,
    /// C++ `ObjectMgr::LoadFactionChangeReputations` startup query.
    SEL_FACTION_CHANGE_REPUTATIONS,
    /// C++ `ObjectMgr::LoadFactionChangeSpells` startup query.
    SEL_FACTION_CHANGE_SPELLS,
    /// C++ `ObjectMgr::LoadFactionChangeTitles` startup query.
    SEL_FACTION_CHANGE_TITLES,
    /// Load trainer IDs for C++ ConditionMgr source validation.
    SEL_TRAINER_IDS,
    /// Load conversation line template IDs for C++ ConditionMgr source validation.
    SEL_CONVERSATION_LINE_TEMPLATE_IDS,
    /// Load area-trigger template keys for C++ ConditionMgr source validation.
    SEL_AREA_TRIGGER_TEMPLATE_IDS,
    /// C++ `AreaTriggerDataStore::LoadAreaTriggerTemplates` action rows.
    SEL_AREATRIGGER_TEMPLATE_ACTIONS,
    /// C++ `AreaTriggerDataStore::LoadAreaTriggerTemplates` polygon vertex rows.
    SEL_AREATRIGGER_CREATE_PROPERTIES_POLYGON_VERTICES,
    /// C++ `AreaTriggerDataStore::LoadAreaTriggerTemplates` spline point rows.
    SEL_AREATRIGGER_CREATE_PROPERTIES_SPLINE_POINTS,
    /// C++ `AreaTriggerDataStore::LoadAreaTriggerTemplates` create-properties rows.
    SEL_AREATRIGGER_CREATE_PROPERTIES,
    /// C++ `AreaTriggerDataStore::LoadAreaTriggerTemplates` create-properties orbit rows.
    SEL_AREATRIGGER_CREATE_PROPERTIES_ORBIT,
    /// C++ `AreaTriggerDataStore::LoadAreaTriggerTemplates` template rows.
    SEL_AREATRIGGER_TEMPLATES,
}
