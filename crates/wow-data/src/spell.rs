// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spell.db2 and related spell data loading.
//!
//! Loads spell metadata from hotfixes database or DB2 files:
//! - Cast time (milliseconds)
//! - Global cooldown
//! - Per-spell cooldown
//! - Effect type (heal, damage, apply aura, etc.)
//! - Effect parameters (base points, bonus coefficients)

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::f32::consts::TAU;

use anyhow::Result;
use tracing::info;
use wow_database::{HotfixDatabase, StatementDef, WorldDatabase, WorldStatements};
use wow_entities::PetAuraLikeCpp;

use crate::{
    ConditionEntriesByTypeStore, ConditionsReference, conditions::RACEMASK_ALL_PLAYABLE_LIKE_CPP,
};

/// Spell effect types (from SpellEffectType enum)
pub mod spell_effect_types {
    pub const SPELL_EFFECT_NONE: u32 = 0;
    pub const SPELL_EFFECT_INSTAKILL: u32 = 1;
    pub const SPELL_EFFECT_SCHOOL_DAMAGE: u32 = 2;
    pub const SPELL_EFFECT_DUMMY: u32 = 3;
    pub const SPELL_EFFECT_PORTAL_TELEPORT: u32 = 4;
    pub const SPELL_EFFECT_APPLY_AURA: u32 = 6;
    pub const SPELL_EFFECT_ENVIRONMENTAL_DAMAGE: u32 = 7;
    pub const SPELL_EFFECT_POWER_DRAIN: u32 = 8;
    pub const SPELL_EFFECT_HEALTH_LEECH: u32 = 9;
    pub const SPELL_EFFECT_HEAL: u32 = 10;
    pub const SPELL_EFFECT_BIND: u32 = 11;
    pub const SPELL_EFFECT_PORTAL: u32 = 12;
    pub const SPELL_EFFECT_RITUAL_BASE: u32 = 13;
    pub const SPELL_EFFECT_RITUAL_SPECIALIZE: u32 = 14;
    pub const SPELL_EFFECT_RITUAL_ACTIVATE_PORTAL: u32 = 15;
    pub const SPELL_EFFECT_QUEST_COMPLETE: u32 = 16;
    pub const SPELL_EFFECT_ADD_EXTRA_ATTACKS: u32 = 19;
    pub const SPELL_EFFECT_DODGE: u32 = 20;
    pub const SPELL_EFFECT_EVADE: u32 = 21;
    pub const SPELL_EFFECT_PARRY: u32 = 22;
    pub const SPELL_EFFECT_BLOCK: u32 = 23;
    pub const SPELL_EFFECT_WEAPON: u32 = 25;
    pub const SPELL_EFFECT_DEFENSE: u32 = 26;
    pub const SPELL_EFFECT_PERSISTENT_AREA_AURA: u32 = 27;
    pub const SPELL_EFFECT_ENERGIZE: u32 = 30;
    pub const SPELL_EFFECT_APPLY_AREA_AURA_PARTY: u32 = 35;
    pub const SPELL_EFFECT_LEARN_SPELL: u32 = 36;
    pub const SPELL_EFFECT_SPELL_DEFENSE: u32 = 37;
    pub const SPELL_EFFECT_LANGUAGE: u32 = 39;
    pub const SPELL_EFFECT_DUAL_WIELD: u32 = 40;
    pub const SPELL_EFFECT_PLAY_MOVIE: u32 = 45;
    pub const SPELL_EFFECT_SPAWN: u32 = 46;
    pub const SPELL_EFFECT_TRADE_SKILL: u32 = 47;
    pub const SPELL_EFFECT_STEALTH: u32 = 48;
    pub const SPELL_EFFECT_DETECT: u32 = 49;
    pub const SPELL_EFFECT_FORCE_CRITICAL_HIT: u32 = 51;
    pub const SPELL_EFFECT_GUARANTEE_HIT: u32 = 52;
    pub const SPELL_EFFECT_POWER_BURN: u32 = 62;
    pub const SPELL_EFFECT_THREAT: u32 = 63;
    pub const SPELL_EFFECT_APPLY_AREA_AURA_RAID: u32 = 65;
    pub const SPELL_EFFECT_HEAL_MAX_HEALTH: u32 = 67;
    pub const SPELL_EFFECT_DISTRACT: u32 = 69;
    pub const SPELL_EFFECT_PULL: u32 = 70;
    pub const SPELL_EFFECT_ADD_FARSIGHT: u32 = 72;
    pub const SPELL_EFFECT_HEAL_MECHANICAL: u32 = 75;
    /// C++ `SPELL_EFFECT_SUMMON_OBJECT_WILD`; see
    /// `Spell::EffectSummonObjectWild` (`SpellEffects.cpp:2937-2986`).
    pub const SPELL_EFFECT_SUMMON_OBJECT_WILD: u32 = 76;
    pub const SPELL_EFFECT_ATTACK: u32 = 78;
    pub const SPELL_EFFECT_SANCTUARY: u32 = 79;
    /// C++ `SPELL_EFFECT_ADD_COMBO_POINTS`; in the current legacy source
    /// `Spell::EffectAddComboPoints` validates the hit/unit target and then
    /// has its combo-point mutation commented out (`SpellEffects.cpp:3164`).
    pub const SPELL_EFFECT_ADD_COMBO_POINTS: u32 = 80;
    pub const SPELL_EFFECT_CREATE_HOUSE: u32 = 81;
    pub const SPELL_EFFECT_BIND_SIGHT: u32 = 82;
    pub const SPELL_EFFECT_DUEL: u32 = 83;
    pub const SPELL_EFFECT_STUCK: u32 = 84;
    pub const SPELL_EFFECT_KILL_CREDIT: u32 = 90;
    pub const SPELL_EFFECT_THREAT_ALL: u32 = 91;
    pub const SPELL_EFFECT_FORCE_DESELECT: u32 = 93;
    pub const SPELL_EFFECT_SELF_RESURRECT: u32 = 94;
    pub const SPELL_EFFECT_INEBRIATE: u32 = 100;
    pub const SPELL_EFFECT_DISMISS_PET: u32 = 102;
    pub const SPELL_EFFECT_REPUTATION: u32 = 103;
    /// First C++ `SPELL_EFFECT_SUMMON_OBJECT_SLOT*` value; see
    /// `Spell::EffectSummonObject` (`SpellEffects.cpp:3541-3597`).
    pub const SPELL_EFFECT_SUMMON_OBJECT_SLOT1: u32 = 104;
    pub const SPELL_EFFECT_SURVEY: u32 = 105;
    pub const SPELL_EFFECT_CHANGE_RAID_MARKER: u32 = 106;
    pub const SPELL_EFFECT_SHOW_CORPSE_LOOT: u32 = 107;
    pub const SPELL_EFFECT_112: u32 = 112;
    pub const SPELL_EFFECT_ATTACK_ME: u32 = 114;
    /// C++ `SPELL_EFFECT_SKILL`; `SpellMgr::LoadSpellLearnSkills` derives
    /// `mSpellLearnSkills` from this effect.
    pub const SPELL_EFFECT_SKILL: u32 = 118;
    pub const SPELL_EFFECT_APPLY_AREA_AURA_PET: u32 = 119;
    pub const SPELL_EFFECT_122: u32 = 122;
    pub const SPELL_EFFECT_MODIFY_THREAT_PERCENT: u32 = 125;
    pub const SPELL_EFFECT_APPLY_AREA_AURA_FRIEND: u32 = 128;
    pub const SPELL_EFFECT_APPLY_AREA_AURA_ENEMY: u32 = 129;
    pub const SPELL_EFFECT_KILL_CREDIT2: u32 = 134;
    pub const SPELL_EFFECT_CALL_PET: u32 = 135;
    pub const SPELL_EFFECT_HEAL_PCT: u32 = 136;
    pub const SPELL_EFFECT_ENERGIZE_PCT: u32 = 137;
    pub const SPELL_EFFECT_APPLY_AREA_AURA_OWNER: u32 = 143;
    pub const SPELL_EFFECT_OBLITERATE_ITEM: u32 = 163;
    pub const SPELL_EFFECT_ALLOW_CONTROL_PET: u32 = 168;
    pub const SPELL_EFFECT_APPLY_AURA_ON_PET: u32 = 174;
    pub const SPELL_EFFECT_175: u32 = 175;
    pub const SPELL_EFFECT_DESPAWN_PERSISTENT_AREA_AURA: u32 = 177;
    pub const SPELL_EFFECT_178: u32 = 178;
    pub const SPELL_EFFECT_UPDATE_AREATRIGGER: u32 = 180;
    pub const SPELL_EFFECT_DESPAWN_AREATRIGGER: u32 = 182;
    pub const SPELL_EFFECT_183: u32 = 183;
    pub const SPELL_EFFECT_REPUTATION_2: u32 = 184;
    pub const SPELL_EFFECT_185: u32 = 185;
    pub const SPELL_EFFECT_186: u32 = 186;
    pub const SPELL_EFFECT_RANDOMIZE_ARCHAEOLOGY_DIGSITES: u32 = 187;
    pub const SPELL_EFFECT_SUMMON_STABLED_PET_AS_GUARDIAN: u32 = 188;
    pub const SPELL_EFFECT_LOOT: u32 = 189;
    pub const SPELL_EFFECT_CHANGE_PARTY_MEMBERS: u32 = 190;
    pub const SPELL_EFFECT_TELEPORT_TO_DIGSITE: u32 = 191;
    pub const SPELL_EFFECT_UNCAGE_BATTLEPET: u32 = 192;
    pub const SPELL_EFFECT_START_PET_BATTLE: u32 = 193;
    pub const SPELL_EFFECT_194: u32 = 194;
    pub const SPELL_EFFECT_DESPAWN_SUMMON: u32 = 199;
    pub const SPELL_EFFECT_APPLY_AREA_AURA_SUMMONS: u32 = 202;
    pub const SPELL_EFFECT_CHANGE_BATTLEPET_QUALITY: u32 = 204;
    pub const SPELL_EFFECT_ALTER_ITEM: u32 = 206;
    pub const SPELL_EFFECT_LAUNCH_QUEST_TASK: u32 = 207;
    pub const SPELL_EFFECT_SET_REPUTATION: u32 = 208;
    pub const SPELL_EFFECT_209: u32 = 209;
    pub const SPELL_EFFECT_LEARN_GARRISON_BUILDING: u32 = 210;
    pub const SPELL_EFFECT_LEARN_GARRISON_SPECIALIZATION: u32 = 211;
    pub const SPELL_EFFECT_CREATE_GARRISON: u32 = 214;
    pub const SPELL_EFFECT_UPGRADE_CHARACTER_SPELLS: u32 = 215;
    pub const SPELL_EFFECT_CREATE_SHIPMENT: u32 = 216;
    pub const SPELL_EFFECT_UPGRADE_GARRISON: u32 = 217;
    pub const SPELL_EFFECT_218: u32 = 218;
    pub const SPELL_EFFECT_ADD_GARRISON_FOLLOWER: u32 = 220;
    pub const SPELL_EFFECT_ADD_GARRISON_MISSION: u32 = 221;
    pub const SPELL_EFFECT_CHANGE_ITEM_BONUSES: u32 = 223;
    pub const SPELL_EFFECT_ACTIVATE_GARRISON_BUILDING: u32 = 224;
    pub const SPELL_EFFECT_GRANT_BATTLEPET_LEVEL: u32 = 225;
    pub const SPELL_EFFECT_TRIGGER_ACTION_SET: u32 = 226;
    pub const SPELL_EFFECT_TELEPORT_TO_LFG_DUNGEON: u32 = 227;
    pub const SPELL_EFFECT_228: u32 = 228;
    pub const SPELL_EFFECT_SET_FOLLOWER_QUALITY: u32 = 229;
    pub const SPELL_EFFECT_230: u32 = 230;
    pub const SPELL_EFFECT_INCREASE_FOLLOWER_EXPERIENCE: u32 = 231;
    pub const SPELL_EFFECT_REMOVE_PHASE: u32 = 232;
    pub const SPELL_EFFECT_RANDOMIZE_FOLLOWER_ABILITIES: u32 = 233;
    pub const SPELL_EFFECT_234: u32 = 234;
    pub const SPELL_EFFECT_235: u32 = 235;
    pub const SPELL_EFFECT_INCREASE_SKILL: u32 = 238;
    pub const SPELL_EFFECT_END_GARRISON_BUILDING_CONSTRUCTION: u32 = 239;
    pub const SPELL_EFFECT_GIVE_ARTIFACT_POWER: u32 = 240;
    pub const SPELL_EFFECT_241: u32 = 241;
    pub const SPELL_EFFECT_GIVE_ARTIFACT_POWER_NO_BONUS: u32 = 242;
    pub const SPELL_EFFECT_LEARN_FOLLOWER_ABILITY: u32 = 244;
    pub const SPELL_EFFECT_UPGRADE_HEIRLOOM: u32 = 245;
    pub const SPELL_EFFECT_FINISH_GARRISON_MISSION: u32 = 246;
    pub const SPELL_EFFECT_ADD_GARRISON_MISSION_SET: u32 = 247;
    pub const SPELL_EFFECT_FINISH_SHIPMENT: u32 = 248;
    pub const SPELL_EFFECT_FORCE_EQUIP_ITEM: u32 = 249;
    pub const SPELL_EFFECT_TAKE_SCREENSHOT: u32 = 250;
    pub const SPELL_EFFECT_SET_GARRISON_CACHE_SIZE: u32 = 251;
    pub const SPELL_EFFECT_TELEPORT_UNITS: u32 = 252;
    pub const SPELL_EFFECT_GIVE_HONOR: u32 = 253;
    pub const SPELL_EFFECT_JUMP_CHARGE: u32 = 254;
    pub const SPELL_EFFECT_LEARN_TRANSMOG_SET: u32 = 255;
    pub const SPELL_EFFECT_256: u32 = 256;
    pub const SPELL_EFFECT_257: u32 = 257;
    pub const SPELL_EFFECT_MODIFY_KEYSTONE: u32 = 258;
    pub const SPELL_EFFECT_RESPEC_AZERITE_EMPOWERED_ITEM: u32 = 259;
    pub const SPELL_EFFECT_SUMMON_STABLED_PET: u32 = 260;
    pub const SPELL_EFFECT_SCRAP_ITEM: u32 = 261;
    pub const SPELL_EFFECT_262: u32 = 262;
    pub const SPELL_EFFECT_REPAIR_ITEM: u32 = 263;
    pub const SPELL_EFFECT_REMOVE_GEM: u32 = 264;
    pub const SPELL_EFFECT_LEARN_AZERITE_ESSENCE_POWER: u32 = 265;
    pub const SPELL_EFFECT_SET_ITEM_BONUS_LIST_GROUP_ENTRY: u32 = 266;
    pub const SPELL_EFFECT_APPLY_MOUNT_EQUIPMENT: u32 = 268;
    pub const SPELL_EFFECT_INCREASE_ITEM_BONUS_LIST_GROUP_STEP: u32 = 269;
    pub const SPELL_EFFECT_270: u32 = 270;
    pub const SPELL_EFFECT_APPLY_AREA_AURA_PARTY_NONRANDOM: u32 = 271;
    pub const SPELL_EFFECT_SET_COVENANT: u32 = 272;
    pub const SPELL_EFFECT_CRAFT_RUNEFORGE_LEGENDARY: u32 = 273;
    pub const SPELL_EFFECT_274: u32 = 274;
    pub const SPELL_EFFECT_275: u32 = 275;
    pub const SPELL_EFFECT_LEARN_TRANSMOG_ILLUSION: u32 = 276;
    pub const SPELL_EFFECT_SET_CHROMIE_TIME: u32 = 277;
    pub const SPELL_EFFECT_278: u32 = 278;
    pub const SPELL_EFFECT_LEARN_GARR_TALENT: u32 = 279;
    pub const SPELL_EFFECT_280: u32 = 280;
    pub const SPELL_EFFECT_LEARN_SOULBIND_CONDUIT: u32 = 281;
    pub const SPELL_EFFECT_CONVERT_ITEMS_TO_CURRENCY: u32 = 282;
    pub const SPELL_EFFECT_COMPLETE_CAMPAIGN: u32 = 283;
    pub const SPELL_EFFECT_MODIFY_KEYSTONE_2: u32 = 285;
    pub const SPELL_EFFECT_GRANT_BATTLEPET_EXPERIENCE: u32 = 286;
    pub const SPELL_EFFECT_SET_GARRISON_FOLLOWER_LEVEL: u32 = 287;
    pub const SPELL_EFFECT_CRAFT_ITEM: u32 = 288;
    pub const SPELL_EFFECT_MODIFY_AURA_STACKS: u32 = 289;
    pub const SPELL_EFFECT_MODIFY_COOLDOWN: u32 = 290;
    pub const SPELL_EFFECT_MODIFY_COOLDOWNS: u32 = 291;
    pub const SPELL_EFFECT_MODIFY_COOLDOWNS_BY_CATEGORY: u32 = 292;
    pub const SPELL_EFFECT_MODIFY_CHARGES: u32 = 293;
    pub const SPELL_EFFECT_CRAFT_LOOT: u32 = 294;
    pub const SPELL_EFFECT_SALVAGE_ITEM: u32 = 295;
    pub const SPELL_EFFECT_CRAFT_SALVAGE_ITEM: u32 = 296;
    pub const SPELL_EFFECT_RECRAFT_ITEM: u32 = 297;
    pub const SPELL_EFFECT_CANCEL_ALL_PRIVATE_CONVERSATIONS: u32 = 298;
    pub const SPELL_EFFECT_299: u32 = 299;
    pub const SPELL_EFFECT_300: u32 = 300;
    pub const SPELL_EFFECT_CRAFT_ENCHANT: u32 = 301;
    pub const SPELL_EFFECT_GATHERING: u32 = 302;
    pub const SPELL_EFFECT_305: u32 = 305;
    pub const SPELL_EFFECT_UPDATE_INTERACTIONS: u32 = 306;
    pub const SPELL_EFFECT_307: u32 = 307;
    pub const SPELL_EFFECT_CANCEL_PRELOAD_WORLD: u32 = 308;
    pub const SPELL_EFFECT_PRELOAD_WORLD: u32 = 309;
    pub const SPELL_EFFECT_310: u32 = 310;
    pub const SPELL_EFFECT_ENSURE_WORLD_LOADED: u32 = 311;
    pub const SPELL_EFFECT_312: u32 = 312;
    pub const SPELL_EFFECT_CHANGE_ITEM_BONUSES_2: u32 = 313;
    pub const SPELL_EFFECT_ADD_SOCKET_BONUS: u32 = 314;
    pub const SPELL_EFFECT_LEARN_TRANSMOG_APPEARANCE_FROM_ITEM_MOD_APPEARANCE_GROUP: u32 = 315;

    /// C++ dispatch entries that intentionally run as represented no-ops in
    /// `SpellEffects.cpp` for the covered effect range: `EffectNULL`,
    /// `EffectUnused`, or a concrete handler whose mutation is disabled in
    /// this legacy source. This deliberately excludes `SPELL_EFFECT_DUMMY`,
    /// whose behavior is script-driven through `ScriptMgr::OnSpellEffectDummy`.
    pub fn is_cpp_null_or_unused_noop(effect: u32) -> bool {
        matches!(
            effect,
            SPELL_EFFECT_NONE
                | SPELL_EFFECT_PORTAL_TELEPORT
                | SPELL_EFFECT_PORTAL
                | SPELL_EFFECT_RITUAL_BASE
                | SPELL_EFFECT_RITUAL_SPECIALIZE
                | SPELL_EFFECT_RITUAL_ACTIVATE_PORTAL
                | SPELL_EFFECT_DODGE
                | SPELL_EFFECT_EVADE
                | SPELL_EFFECT_WEAPON
                | SPELL_EFFECT_DEFENSE
                | SPELL_EFFECT_APPLY_AREA_AURA_PARTY
                | SPELL_EFFECT_SPELL_DEFENSE
                | SPELL_EFFECT_LANGUAGE
                | SPELL_EFFECT_SPAWN
                | SPELL_EFFECT_STEALTH
                | SPELL_EFFECT_DETECT
                | SPELL_EFFECT_FORCE_CRITICAL_HIT
                | SPELL_EFFECT_GUARANTEE_HIT
                | SPELL_EFFECT_APPLY_AREA_AURA_RAID
                | SPELL_EFFECT_ATTACK
                | SPELL_EFFECT_ADD_COMBO_POINTS
                | SPELL_EFFECT_CREATE_HOUSE
                | SPELL_EFFECT_BIND_SIGHT
                | SPELL_EFFECT_THREAT_ALL
                | SPELL_EFFECT_SURVEY
                | SPELL_EFFECT_SHOW_CORPSE_LOOT
                | SPELL_EFFECT_112
                | SPELL_EFFECT_APPLY_AREA_AURA_PET
                | SPELL_EFFECT_122
                | SPELL_EFFECT_APPLY_AREA_AURA_FRIEND
                | SPELL_EFFECT_APPLY_AREA_AURA_ENEMY
                | SPELL_EFFECT_CALL_PET
                | SPELL_EFFECT_APPLY_AREA_AURA_OWNER
                | SPELL_EFFECT_OBLITERATE_ITEM
                | SPELL_EFFECT_ALLOW_CONTROL_PET
                | SPELL_EFFECT_175
                | SPELL_EFFECT_DESPAWN_PERSISTENT_AREA_AURA
                | SPELL_EFFECT_178
                | SPELL_EFFECT_UPDATE_AREATRIGGER
                | SPELL_EFFECT_DESPAWN_AREATRIGGER
                | SPELL_EFFECT_183
                | SPELL_EFFECT_REPUTATION_2
                | SPELL_EFFECT_185
                | SPELL_EFFECT_186
                | SPELL_EFFECT_RANDOMIZE_ARCHAEOLOGY_DIGSITES
                | SPELL_EFFECT_SUMMON_STABLED_PET_AS_GUARDIAN
                | SPELL_EFFECT_LOOT
                | SPELL_EFFECT_CHANGE_PARTY_MEMBERS
                | SPELL_EFFECT_TELEPORT_TO_DIGSITE
                | SPELL_EFFECT_START_PET_BATTLE
                | SPELL_EFFECT_194
                | SPELL_EFFECT_DESPAWN_SUMMON
                | SPELL_EFFECT_APPLY_AREA_AURA_SUMMONS
                | SPELL_EFFECT_ALTER_ITEM
                | SPELL_EFFECT_LAUNCH_QUEST_TASK
                | SPELL_EFFECT_SET_REPUTATION
                | SPELL_EFFECT_209
                | SPELL_EFFECT_LEARN_GARRISON_BUILDING
                | SPELL_EFFECT_LEARN_GARRISON_SPECIALIZATION
                | SPELL_EFFECT_CREATE_GARRISON
                | SPELL_EFFECT_UPGRADE_CHARACTER_SPELLS
                | SPELL_EFFECT_CREATE_SHIPMENT
                | SPELL_EFFECT_UPGRADE_GARRISON
                | SPELL_EFFECT_218
                | SPELL_EFFECT_ADD_GARRISON_FOLLOWER
                | SPELL_EFFECT_ADD_GARRISON_MISSION
                | SPELL_EFFECT_CHANGE_ITEM_BONUSES
                | SPELL_EFFECT_ACTIVATE_GARRISON_BUILDING
                | SPELL_EFFECT_TRIGGER_ACTION_SET
                | SPELL_EFFECT_TELEPORT_TO_LFG_DUNGEON
                | SPELL_EFFECT_228
                | SPELL_EFFECT_SET_FOLLOWER_QUALITY
                | SPELL_EFFECT_230
                | SPELL_EFFECT_INCREASE_FOLLOWER_EXPERIENCE
                | SPELL_EFFECT_REMOVE_PHASE
                | SPELL_EFFECT_RANDOMIZE_FOLLOWER_ABILITIES
                | SPELL_EFFECT_234
                | SPELL_EFFECT_235
                | SPELL_EFFECT_INCREASE_SKILL
                | SPELL_EFFECT_END_GARRISON_BUILDING_CONSTRUCTION
                | SPELL_EFFECT_GIVE_ARTIFACT_POWER
                | SPELL_EFFECT_241
                | SPELL_EFFECT_GIVE_ARTIFACT_POWER_NO_BONUS
                | SPELL_EFFECT_LEARN_FOLLOWER_ABILITY
                | SPELL_EFFECT_FINISH_GARRISON_MISSION
                | SPELL_EFFECT_ADD_GARRISON_MISSION_SET
                | SPELL_EFFECT_FINISH_SHIPMENT
                | SPELL_EFFECT_FORCE_EQUIP_ITEM
                | SPELL_EFFECT_TAKE_SCREENSHOT
                | SPELL_EFFECT_SET_GARRISON_CACHE_SIZE
                | SPELL_EFFECT_256
                | SPELL_EFFECT_257
                | SPELL_EFFECT_MODIFY_KEYSTONE
                | SPELL_EFFECT_RESPEC_AZERITE_EMPOWERED_ITEM
                | SPELL_EFFECT_SUMMON_STABLED_PET
                | SPELL_EFFECT_SCRAP_ITEM
                | SPELL_EFFECT_262
                | SPELL_EFFECT_REPAIR_ITEM
                | SPELL_EFFECT_REMOVE_GEM
                | SPELL_EFFECT_LEARN_AZERITE_ESSENCE_POWER
                | SPELL_EFFECT_SET_ITEM_BONUS_LIST_GROUP_ENTRY
                | SPELL_EFFECT_APPLY_MOUNT_EQUIPMENT
                | SPELL_EFFECT_INCREASE_ITEM_BONUS_LIST_GROUP_STEP
                | SPELL_EFFECT_270
                | SPELL_EFFECT_APPLY_AREA_AURA_PARTY_NONRANDOM
                | SPELL_EFFECT_SET_COVENANT
                | SPELL_EFFECT_CRAFT_RUNEFORGE_LEGENDARY
                | SPELL_EFFECT_274
                | SPELL_EFFECT_275
                | SPELL_EFFECT_SET_CHROMIE_TIME
                | SPELL_EFFECT_278
                | SPELL_EFFECT_LEARN_GARR_TALENT
                | SPELL_EFFECT_280
                | SPELL_EFFECT_LEARN_SOULBIND_CONDUIT
                | SPELL_EFFECT_CONVERT_ITEMS_TO_CURRENCY
                | SPELL_EFFECT_COMPLETE_CAMPAIGN
                | SPELL_EFFECT_MODIFY_KEYSTONE_2
                | SPELL_EFFECT_SET_GARRISON_FOLLOWER_LEVEL
                | SPELL_EFFECT_CRAFT_ITEM
                | SPELL_EFFECT_CRAFT_LOOT
                | SPELL_EFFECT_SALVAGE_ITEM
                | SPELL_EFFECT_CRAFT_SALVAGE_ITEM
                | SPELL_EFFECT_RECRAFT_ITEM
                | SPELL_EFFECT_CANCEL_ALL_PRIVATE_CONVERSATIONS
                | SPELL_EFFECT_299
                | SPELL_EFFECT_300
                | SPELL_EFFECT_CRAFT_ENCHANT
                | SPELL_EFFECT_GATHERING
                | SPELL_EFFECT_305
                | SPELL_EFFECT_UPDATE_INTERACTIONS
                | SPELL_EFFECT_307
                | SPELL_EFFECT_CANCEL_PRELOAD_WORLD
                | SPELL_EFFECT_PRELOAD_WORLD
                | SPELL_EFFECT_310
                | SPELL_EFFECT_ENSURE_WORLD_LOADED
                | SPELL_EFFECT_312
                | SPELL_EFFECT_CHANGE_ITEM_BONUSES_2
                | SPELL_EFFECT_ADD_SOCKET_BONUS
                | SPELL_EFFECT_LEARN_TRANSMOG_APPEARANCE_FROM_ITEM_MOD_APPEARANCE_GROUP
        )
    }
}

/// Aura types (from AuraType enum)
pub mod aura_types {
    pub const SPELL_AURA_CONTROL_VEHICLE: i32 = 236;
    pub const SPELL_AURA_DUMMY: i32 = 0;
    pub const SPELL_AURA_SCHOOL_ABSORB: i32 = 1;
    pub const SPELL_AURA_SCHOOL_IMMUNITY: i32 = 2;
    pub const SPELL_AURA_DUMMY_ABSORB: i32 = 3;
    pub const SPELL_AURA_MOD_CONFUSE: i32 = 5;
    pub const SPELL_AURA_MOD_FEAR: i32 = 7;
    pub const SPELL_AURA_MOD_THREAT: i32 = 10;
    pub const SPELL_AURA_MOD_TAUNT: i32 = 11;
    pub const SPELL_AURA_MOD_STUN: i32 = 12;
    pub const SPELL_AURA_MOD_DAMAGE_DONE: i32 = 13;
    pub const SPELL_AURA_MOD_DAMAGE_TAKEN: i32 = 14;
    pub const SPELL_AURA_MOD_STEALTH: i32 = 16;
    pub const SPELL_AURA_MOD_INVISIBILITY: i32 = 18;
    pub const SPELL_AURA_MOD_RESISTANCE: i32 = 22;
    pub const SPELL_AURA_MOD_ROOT: i32 = 26;
    pub const SPELL_AURA_REFLECT_SPELLS: i32 = 28;
    pub const SPELL_AURA_MODIFY_DAMAGE_PERCENT_TAKEN: i32 = 31;
    pub const SPELL_AURA_DAMAGE_IMMUNITY: i32 = 40;
    pub const SPELL_AURA_PROC_TRIGGER_SPELL: i32 = 42;
    pub const SPELL_AURA_PROC_TRIGGER_DAMAGE: i32 = 43;
    pub const SPELL_AURA_MOD_BLOCK_PERCENT: i32 = 51;
    pub const SPELL_AURA_MOD_WEAPON_CRIT_PERCENT: i32 = 52;
    pub const SPELL_AURA_MOD_HIT_CHANCE: i32 = 54;
    pub const SPELL_AURA_TRANSFORM: i32 = 56;
    pub const SPELL_AURA_MOD_SPELL_CRIT_CHANCE: i32 = 57;
    pub const SPELL_AURA_MOD_CASTING_SPEED_NOT_STACK: i32 = 65;
    pub const SPELL_AURA_MOD_POWER_COST_SCHOOL_PCT: i32 = 72;
    pub const SPELL_AURA_HASTE_SPELLS: i32 = 73;
    pub const SPELL_AURA_MOD_POWER_COST_SCHOOL: i32 = 73;
    pub const SPELL_AURA_REFLECT_SPELLS_SCHOOL: i32 = 74;
    pub const SPELL_AURA_MECHANIC_IMMUNITY: i32 = 77;
    pub const SPELL_AURA_MOUNTED: i32 = 78;
    pub const SPELL_AURA_MOD_DAMAGE_PERCENT_DONE: i32 = 79;
    pub const SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN: i32 = 87;
    pub const SPELL_AURA_MOD_DETECT_RANGE: i32 = 91;
    pub const SPELL_AURA_SPELL_MAGNET: i32 = 96;
    pub const SPELL_AURA_MOD_ATTACK_POWER: i32 = 99;
    pub const SPELL_AURA_ADD_FLAT_MODIFIER: i32 = 107;
    pub const SPELL_AURA_ADD_PCT_MODIFIER: i32 = 108;
    pub const SPELL_AURA_MOD_POWER_REGEN_PERCENT: i32 = 110;
    pub const SPELL_AURA_INTERCEPT_MELEE_RANGED_ATTACKS: i32 = 111;
    pub const SPELL_AURA_OVERRIDE_CLASS_SCRIPTS: i32 = 112;
    pub const SPELL_AURA_MOD_MECHANIC_RESISTANCE: i32 = 117;
    pub const SPELL_AURA_RANGED_ATTACK_POWER_ATTACKER_BONUS: i32 = 127;
    pub const SPELL_AURA_MOD_MELEE_HASTE: i32 = 138;
    pub const SPELL_AURA_FORCE_REACTION: i32 = 139;
    pub const SPELL_AURA_MOD_RANGED_HASTE: i32 = 140;
    pub const SPELL_AURA_MOD_DETECTED_RANGE: i32 = 152;
    pub const SPELL_AURA_MOD_ATTACK_POWER_PCT: i32 = 166;
    pub const SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE: i32 = 184;
    pub const SPELL_AURA_MOD_MELEE_RANGED_HASTE: i32 = 192;
    pub const SPELL_AURA_ADD_PCT_MODIFIER_BY_SPELL_LABEL: i32 = 218;
    pub const SPELL_AURA_MOD_DETAUNT: i32 = 221;
    pub const SPELL_AURA_PERIODIC_DUMMY: i32 = 226;
    pub const SPELL_AURA_PROC_TRIGGER_SPELL_WITH_VALUE: i32 = 231;
    pub const SPELL_AURA_ABILITY_IGNORE_AURASTATE: i32 = 262;
    pub const SPELL_AURA_MOD_SCHOOL_MASK_DAMAGE_FROM_CASTER: i32 = 270;
    pub const SPELL_AURA_MOD_SPELL_DAMAGE_FROM_CASTER: i32 = 271;
    pub const SPELL_AURA_PROVIDE_SPELL_FOCUS: i32 = 281;
    pub const SPELL_AURA_MOD_MELEE_HASTE_3: i32 = 319;
    pub const SPELL_AURA_IGNORE_SPELL_COOLDOWN: i32 = 383;
    pub const SPELL_AURA_MOD_BATTLE_PET_XP_PCT: i32 = 420;
    pub const SPELL_AURA_MOD_ROOT_2: i32 = 455;
}

/// Selected `Targets` ids from C++ `SpellImplicitTargetInfo::_data`.
pub mod implicit_targets {
    pub const TARGET_DEST_DB: u32 = 17;
    pub const TARGET_DEST_NEARBY_ENTRY: u32 = 46;
    pub const TARGET_DEST_NEARBY_ENTRY_2: u32 = 107;
    pub const TARGET_DEST_NEARBY_ENTRY_OR_DB: u32 = 142;
}

/// C++ `MAX_SPELL_EFFECTS` (`DBCEnums.h`).
pub const MAX_SPELL_EFFECTS_LIKE_CPP: i32 = 32;
/// C++ `TOTAL_SPELL_EFFECTS` (`SharedDefines.h`): last effect id 315 + sentinel.
pub const TOTAL_SPELL_EFFECTS_LIKE_CPP: i32 = 316;
/// C++ `TOTAL_AURAS` (`SpellAuraDefines.h`): last aura id 544 + sentinel.
pub const TOTAL_AURAS_LIKE_CPP: i32 = 545;
/// C++ `TOTAL_SPELL_TARGETS` (`SharedDefines.h`): last target id 152 + sentinel.
pub const TOTAL_SPELL_TARGETS_LIKE_CPP: i32 = 153;

pub mod attributes {
    /// C++ `SPELL_ATTR0_PASSIVE` (`SharedDefines.h`).
    pub const SPELL_ATTR0_PASSIVE: u32 = 0x0000_0040;
    /// C++ `SPELL_ATTR1_NO_AUTOCAST_AI` (`SharedDefines.h`).
    pub const SPELL_ATTR1_NO_AUTOCAST_AI: u32 = 0x0002_0000;
    /// C++ `SPELL_ATTR3_CAN_PROC_FROM_PROCS` (`SharedDefines.h`).
    pub const SPELL_ATTR3_CAN_PROC_FROM_PROCS: u32 = 0x0400_0000;
    /// C++ `SPELL_ATTR4_AURA_EXPIRES_OFFLINE` (`SharedDefines.h`).
    pub const SPELL_ATTR4_AURA_EXPIRES_OFFLINE: u32 = 0x0000_0004;
    pub const SPELL_ATTR4_USE_FACING_FROM_SPELL: u32 = 0x8000_0000;
}

/// Metadata for a spell from Spell.db2 and related tables.
#[derive(Debug, Clone)]
pub struct SpellInfo {
    /// Spell ID
    pub spell_id: i32,
    /// Cast time in milliseconds (0 = instant)
    pub cast_time_ms: u32,
    /// Global cooldown in milliseconds
    pub cooldown_ms: u32,
    /// Per-spell cooldown in milliseconds (0 = no per-spell cooldown)
    pub recovery_time_ms: u32,
    /// First effect type (primary effect) — e.g., 2 (damage), 6 (aura), 10 (heal)
    pub effect_type: u32,
    /// Base damage/healing before bonuses
    pub effect_base_points: i32,
    /// Spell power / attack power coefficient (0.0 = no scaling)
    pub effect_bonus_coefficient: f32,
    /// Aura type if effect_type == SPELL_EFFECT_APPLY_AURA
    pub aura_type: Option<i32>,
    /// Display flags (channelled, etc.)
    pub display_flags: u32,
    /// C++ `SpellInfo::RequiresSpellFocus`, hydrated from
    /// `SpellCastingRequirementsEntry::RequiresSpellFocus`.
    pub requires_spell_focus: u32,
    /// Spell effects keyed by C++ `SpellEffectInfo::EffectIndex`.
    pub effects: Vec<SpellEffectInfo>,
}

/// Minimal `SpellEffectInfo` fields needed by C++ ConditionMgr validation.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpellEffectInfo {
    pub effect_index: u32,
    pub effect: u32,
    pub effect_aura: i32,
    pub effect_base_points: i32,
    pub effect_die_sides: i32,
    pub effect_spell_class_mask: [u32; 4],
    pub effect_misc_value_1: i32,
    pub effect_misc_value_2: i32,
    pub effect_trigger_spell: i32,
    /// C++ `SpellEffectEntry::EffectRadiusIndex[0]` / TargetA radius index.
    pub effect_radius_index_1: u32,
    pub position_facing: f32,
    pub chain_targets: i32,
    pub implicit_target_1: u32,
    pub implicit_target_2: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServersideSpellEffectRowLikeCpp {
    pub spell_id: u32,
    pub effect_index: i32,
    pub difficulty_id: u32,
    pub effect: i32,
    pub effect_aura: i32,
    pub effect_amplitude: f32,
    pub effect_attributes: i32,
    pub effect_aura_period: i32,
    pub effect_bonus_coefficient: f32,
    pub effect_chain_amplitude: f32,
    pub effect_chain_targets: i32,
    pub effect_item_type: i32,
    pub effect_mechanic: i32,
    pub effect_points_per_resource: f32,
    pub effect_pos_facing: f32,
    pub effect_real_points_per_level: f32,
    pub effect_trigger_spell: i32,
    pub bonus_coefficient_from_ap: f32,
    pub pvp_multiplier: f32,
    pub coefficient: f32,
    pub variance: f32,
    pub resource_coefficient: f32,
    pub group_size_base_points_coefficient: f32,
    pub effect_base_points: f32,
    pub effect_misc_value_1: i32,
    pub effect_misc_value_2: i32,
    pub effect_radius_index_1: u32,
    pub effect_radius_index_2: u32,
    pub effect_spell_class_mask: [i32; 4],
    pub implicit_target_1: i32,
    pub implicit_target_2: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServersideSpellEffectLikeCpp {
    pub effect_index: i32,
    pub difficulty_id: u32,
    pub effect: i32,
    pub effect_aura: i32,
    pub effect_amplitude: f32,
    pub effect_attributes: i32,
    pub effect_aura_period: i32,
    pub effect_bonus_coefficient: f32,
    pub effect_chain_amplitude: f32,
    pub effect_chain_targets: i32,
    pub effect_item_type: i32,
    pub effect_mechanic: i32,
    pub effect_points_per_resource: f32,
    pub effect_pos_facing: f32,
    pub effect_real_points_per_level: f32,
    pub effect_trigger_spell: i32,
    pub bonus_coefficient_from_ap: f32,
    pub pvp_multiplier: f32,
    pub coefficient: f32,
    pub variance: f32,
    pub resource_coefficient: f32,
    pub group_size_base_points_coefficient: f32,
    pub effect_base_points: f32,
    pub effect_misc_value: [i32; 2],
    pub effect_radius_index: [u32; 2],
    pub effect_spell_class_mask: [i32; 4],
    pub implicit_target: [i32; 2],
}

impl ServersideSpellEffectRowLikeCpp {
    pub fn into_effect_like_cpp(self) -> ServersideSpellEffectLikeCpp {
        ServersideSpellEffectLikeCpp {
            effect_index: self.effect_index,
            difficulty_id: self.difficulty_id,
            effect: self.effect,
            effect_aura: self.effect_aura,
            effect_amplitude: self.effect_amplitude,
            effect_attributes: self.effect_attributes,
            effect_aura_period: self.effect_aura_period,
            effect_bonus_coefficient: self.effect_bonus_coefficient,
            effect_chain_amplitude: self.effect_chain_amplitude,
            effect_chain_targets: self.effect_chain_targets,
            effect_item_type: self.effect_item_type,
            effect_mechanic: self.effect_mechanic,
            effect_points_per_resource: self.effect_points_per_resource,
            effect_pos_facing: self.effect_pos_facing,
            effect_real_points_per_level: self.effect_real_points_per_level,
            effect_trigger_spell: self.effect_trigger_spell,
            bonus_coefficient_from_ap: self.bonus_coefficient_from_ap,
            pvp_multiplier: self.pvp_multiplier,
            coefficient: self.coefficient,
            variance: self.variance,
            resource_coefficient: self.resource_coefficient,
            group_size_base_points_coefficient: self.group_size_base_points_coefficient,
            effect_base_points: self.effect_base_points,
            effect_misc_value: [self.effect_misc_value_1, self.effect_misc_value_2],
            effect_radius_index: [self.effect_radius_index_1, self.effect_radius_index_2],
            effect_spell_class_mask: self.effect_spell_class_mask,
            implicit_target: [self.implicit_target_1, self.implicit_target_2],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ServersideSpellEffectKeyLikeCpp {
    pub spell_id: u32,
    pub difficulty_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServersideSpellEffectLoadErrorKindLikeCpp {
    RegularSpellAlreadyLoaded,
    DifficultyMissing,
    EffectIndexOutOfRange,
    EffectTypeOutOfRange,
    AuraTypeOutOfRange,
    ImplicitTarget1OutOfRange,
    ImplicitTarget2OutOfRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServersideSpellEffectLoadErrorLikeCpp {
    pub row: ServersideSpellEffectRowLikeCpp,
    pub kind: ServersideSpellEffectLoadErrorKindLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServersideSpellEffectLoadWarningKindLikeCpp {
    EffectRadius1Missing,
    EffectRadius2Missing,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServersideSpellEffectLoadWarningLikeCpp {
    pub row: ServersideSpellEffectRowLikeCpp,
    pub kind: ServersideSpellEffectLoadWarningKindLikeCpp,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ServersideSpellEffectStoreLikeCpp {
    pub effects_by_spell_and_difficulty:
        BTreeMap<ServersideSpellEffectKeyLikeCpp, Vec<ServersideSpellEffectLikeCpp>>,
}

impl ServersideSpellEffectStoreLikeCpp {
    pub fn from_rows_like_cpp<I, RegularSpellExists, DifficultyExists, RadiusExists>(
        rows: I,
        mut regular_spell_exists: RegularSpellExists,
        mut difficulty_exists: DifficultyExists,
        mut radius_exists: RadiusExists,
    ) -> ServersideSpellEffectLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = ServersideSpellEffectRowLikeCpp>,
        RegularSpellExists: FnMut(u32) -> bool,
        DifficultyExists: FnMut(u32) -> bool,
        RadiusExists: FnMut(u32) -> bool,
    {
        let mut store = Self::default();
        let mut loaded_effect_count = 0;
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        for row in rows {
            if regular_spell_exists(row.spell_id) {
                errors.push(ServersideSpellEffectLoadErrorLikeCpp {
                    row,
                    kind: ServersideSpellEffectLoadErrorKindLikeCpp::RegularSpellAlreadyLoaded,
                });
                continue;
            }

            if row.difficulty_id != 0 && !difficulty_exists(row.difficulty_id) {
                errors.push(ServersideSpellEffectLoadErrorLikeCpp {
                    row,
                    kind: ServersideSpellEffectLoadErrorKindLikeCpp::DifficultyMissing,
                });
                continue;
            }

            if row.effect_index >= MAX_SPELL_EFFECTS_LIKE_CPP {
                errors.push(ServersideSpellEffectLoadErrorLikeCpp {
                    row,
                    kind: ServersideSpellEffectLoadErrorKindLikeCpp::EffectIndexOutOfRange,
                });
                continue;
            }

            if row.effect >= TOTAL_SPELL_EFFECTS_LIKE_CPP {
                errors.push(ServersideSpellEffectLoadErrorLikeCpp {
                    row,
                    kind: ServersideSpellEffectLoadErrorKindLikeCpp::EffectTypeOutOfRange,
                });
                continue;
            }

            if row.effect_aura >= TOTAL_AURAS_LIKE_CPP {
                errors.push(ServersideSpellEffectLoadErrorLikeCpp {
                    row,
                    kind: ServersideSpellEffectLoadErrorKindLikeCpp::AuraTypeOutOfRange,
                });
                continue;
            }

            if row.implicit_target_1 >= TOTAL_SPELL_TARGETS_LIKE_CPP {
                errors.push(ServersideSpellEffectLoadErrorLikeCpp {
                    row,
                    kind: ServersideSpellEffectLoadErrorKindLikeCpp::ImplicitTarget1OutOfRange,
                });
                continue;
            }

            if row.implicit_target_2 >= TOTAL_SPELL_TARGETS_LIKE_CPP {
                errors.push(ServersideSpellEffectLoadErrorLikeCpp {
                    row,
                    kind: ServersideSpellEffectLoadErrorKindLikeCpp::ImplicitTarget2OutOfRange,
                });
                continue;
            }

            if row.effect_radius_index_1 != 0 && !radius_exists(row.effect_radius_index_1) {
                warnings.push(ServersideSpellEffectLoadWarningLikeCpp {
                    row: row.clone(),
                    kind: ServersideSpellEffectLoadWarningKindLikeCpp::EffectRadius1Missing,
                });
            }

            if row.effect_radius_index_2 != 0 && !radius_exists(row.effect_radius_index_2) {
                warnings.push(ServersideSpellEffectLoadWarningLikeCpp {
                    row: row.clone(),
                    kind: ServersideSpellEffectLoadWarningKindLikeCpp::EffectRadius2Missing,
                });
            }

            let key = ServersideSpellEffectKeyLikeCpp {
                spell_id: row.spell_id,
                difficulty_id: row.difficulty_id,
            };
            let effect = row.into_effect_like_cpp();
            store
                .effects_by_spell_and_difficulty
                .entry(key)
                .or_default()
                .push(effect);
            loaded_effect_count += 1;
        }

        ServersideSpellEffectLoadOutcomeLikeCpp {
            store,
            loaded_effect_count,
            errors,
            warnings,
        }
    }

    pub fn effects_for_spell_difficulty_like_cpp(
        &self,
        spell_id: u32,
        difficulty_id: u32,
    ) -> Option<&[ServersideSpellEffectLikeCpp]> {
        self.effects_by_spell_and_difficulty
            .get(&ServersideSpellEffectKeyLikeCpp {
                spell_id,
                difficulty_id,
            })
            .map(Vec::as_slice)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServersideSpellEffectLoadOutcomeLikeCpp {
    pub store: ServersideSpellEffectStoreLikeCpp,
    pub loaded_effect_count: usize,
    pub errors: Vec<ServersideSpellEffectLoadErrorLikeCpp>,
    pub warnings: Vec<ServersideSpellEffectLoadWarningLikeCpp>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServersideSpellRowLikeCpp {
    pub spell_id: u32,
    pub difficulty_id: u32,
    pub category_id: u32,
    pub dispel: u32,
    pub mechanic: u32,
    pub attributes: u32,
    pub attributes_ex: [u32; 14],
    pub stances: u64,
    pub stances_not: u64,
    pub targets: u32,
    pub target_creature_type: u32,
    pub requires_spell_focus: u32,
    pub facing_caster_flags: u32,
    pub caster_aura_state: u32,
    pub target_aura_state: u32,
    pub exclude_caster_aura_state: u32,
    pub exclude_target_aura_state: u32,
    pub caster_aura_spell: u32,
    pub target_aura_spell: u32,
    pub exclude_caster_aura_spell: u32,
    pub exclude_target_aura_spell: u32,
    pub caster_aura_type: i32,
    pub target_aura_type: i32,
    pub exclude_caster_aura_type: i32,
    pub exclude_target_aura_type: i32,
    pub casting_time_index: u32,
    pub recovery_time: u32,
    pub category_recovery_time: u32,
    pub start_recovery_category: u32,
    pub start_recovery_time: u32,
    pub interrupt_flags: u32,
    pub aura_interrupt_flags: [u32; 2],
    pub channel_interrupt_flags: [u32; 2],
    pub proc_flags: [u32; 2],
    pub proc_chance: u32,
    pub proc_charges: u32,
    pub proc_cooldown: u32,
    pub proc_base_ppm: f32,
    pub max_level: u32,
    pub base_level: u32,
    pub spell_level: u32,
    pub duration_index: u32,
    pub range_index: u32,
    pub speed: f32,
    pub launch_delay: f32,
    pub stack_amount: u32,
    pub equipped_item_class: i32,
    pub equipped_item_sub_class_mask: i32,
    pub equipped_item_inventory_type_mask: i32,
    pub content_tuning_id: u32,
    pub spell_name: String,
    pub cone_angle: f32,
    pub cone_width: f32,
    pub max_target_level: u32,
    pub max_affected_targets: u32,
    pub spell_family_name: u32,
    pub spell_family_flags: [u32; 4],
    pub dmg_class: u32,
    pub prevention_type: u32,
    pub area_group_id: i32,
    pub school_mask: u32,
    pub charge_category_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServersideSpellInfoLikeCpp {
    pub row: ServersideSpellRowLikeCpp,
    pub effects: Vec<ServersideSpellEffectLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServersideSpellLoadErrorKindLikeCpp {
    RegularSpellAlreadyLoaded,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServersideSpellLoadErrorLikeCpp {
    pub row: ServersideSpellRowLikeCpp,
    pub kind: ServersideSpellLoadErrorKindLikeCpp,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ServersideSpellStoreLikeCpp {
    pub spell_infos_by_spell_and_difficulty:
        BTreeMap<ServersideSpellEffectKeyLikeCpp, ServersideSpellInfoLikeCpp>,
    pub serverside_spell_names: Vec<(u32, String)>,
}

impl ServersideSpellStoreLikeCpp {
    pub fn from_rows_like_cpp<I, RegularSpellExists>(
        rows: I,
        effects: &ServersideSpellEffectStoreLikeCpp,
        mut regular_spell_exists: RegularSpellExists,
    ) -> ServersideSpellLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = ServersideSpellRowLikeCpp>,
        RegularSpellExists: FnMut(u32) -> bool,
    {
        let mut store = Self::default();
        let mut loaded_spell_count = 0;
        let mut errors = Vec::new();

        for row in rows {
            if regular_spell_exists(row.spell_id) {
                errors.push(ServersideSpellLoadErrorLikeCpp {
                    row,
                    kind: ServersideSpellLoadErrorKindLikeCpp::RegularSpellAlreadyLoaded,
                });
                continue;
            }

            let key = ServersideSpellEffectKeyLikeCpp {
                spell_id: row.spell_id,
                difficulty_id: row.difficulty_id,
            };
            let staged_effects = effects
                .effects_for_spell_difficulty_like_cpp(row.spell_id, row.difficulty_id)
                .map(|effects| effects.to_vec())
                .unwrap_or_default();

            store
                .serverside_spell_names
                .push((row.spell_id, row.spell_name.clone()));
            store.spell_infos_by_spell_and_difficulty.insert(
                key,
                ServersideSpellInfoLikeCpp {
                    row,
                    effects: staged_effects,
                },
            );
            loaded_spell_count += 1;
        }

        ServersideSpellLoadOutcomeLikeCpp {
            store,
            loaded_spell_count,
            errors,
        }
    }

    pub fn get_serverside_spell_like_cpp(
        &self,
        spell_id: u32,
        difficulty_id: u32,
    ) -> Option<&ServersideSpellInfoLikeCpp> {
        self.spell_infos_by_spell_and_difficulty
            .get(&ServersideSpellEffectKeyLikeCpp {
                spell_id,
                difficulty_id,
            })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServersideSpellLoadOutcomeLikeCpp {
    pub store: ServersideSpellStoreLikeCpp,
    pub loaded_spell_count: usize,
    pub errors: Vec<ServersideSpellLoadErrorLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpellTargetPositionLikeCpp {
    pub target_map_id: u16,
    pub position: wow_core::Position,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpellTargetPositionRowLikeCpp {
    pub spell_id: u32,
    pub effect_index: u32,
    pub target_map_id: u16,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub orientation: Option<f32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellTargetPositionLoadReportLikeCpp {
    pub loaded: usize,
    pub skipped_missing_map: usize,
    pub skipped_missing_spell: usize,
    pub skipped_missing_effect: usize,
    pub skipped_zero_position: usize,
    pub skipped_unsupported_target: usize,
}

#[derive(Debug, Clone, Default)]
pub struct SpellTargetPositionStoreLikeCpp {
    positions: HashMap<(u32, u32), SpellTargetPositionLikeCpp>,
    load_report: SpellTargetPositionLoadReportLikeCpp,
}

pub const SPELL_AURA_DUMMY_LIKE_CPP: i32 = 0;
pub const TARGET_UNIT_PET_LIKE_CPP: u32 = 5;
pub const SKILL_DUAL_WIELD_LIKE_CPP: u16 = 118;
pub const SPELL_GROUP_CORE_RANGE_MAX_LIKE_CPP: u32 = 5;
pub const SPELL_GROUP_DB_RANGE_MIN_LIKE_CPP: u32 = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellPetAuraRowLikeCpp {
    pub spell_id: u32,
    pub effect_index: u8,
    pub pet_entry: u32,
    pub aura_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellPetAuraSourceEffectLikeCpp {
    pub effect: u32,
    pub apply_aura_name: i32,
    pub target_a: u32,
    pub calc_value: i32,
}

impl SpellPetAuraSourceEffectLikeCpp {
    pub const fn is_valid_pet_aura_source_like_cpp(self) -> bool {
        self.effect == spell_effect_types::SPELL_EFFECT_DUMMY
            || (self.effect == spell_effect_types::SPELL_EFFECT_APPLY_AURA
                && self.apply_aura_name == SPELL_AURA_DUMMY_LIKE_CPP)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellPetAuraSourceLookupLikeCpp {
    SpellMissing,
    EffectIndexMissing,
    Found(SpellPetAuraSourceEffectLikeCpp),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellPetAuraLoadErrorKindLikeCpp {
    SpellMissing,
    EffectIndexMissing,
    SourceEffectNotDummy,
    AuraSpellMissing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellPetAuraLoadErrorLikeCpp {
    pub row: SpellPetAuraRowLikeCpp,
    pub kind: SpellPetAuraLoadErrorKindLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SpellPetAuraStoreLikeCpp {
    pub auras_by_spell_effect_key: BTreeMap<u32, PetAuraLikeCpp>,
}

impl SpellPetAuraStoreLikeCpp {
    pub const fn key_like_cpp(spell_id: u32, effect_index: u8) -> u32 {
        (spell_id << 8) + effect_index as u32
    }

    pub fn get_pet_aura_like_cpp(
        &self,
        spell_id: u32,
        effect_index: u8,
    ) -> Option<&PetAuraLikeCpp> {
        self.auras_by_spell_effect_key
            .get(&Self::key_like_cpp(spell_id, effect_index))
    }

    pub fn load_spell_pet_auras_like_cpp<I, SourceEffect, AuraExists>(
        rows: I,
        mut source_effect_lookup: SourceEffect,
        mut aura_spell_exists: AuraExists,
    ) -> SpellPetAuraLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellPetAuraRowLikeCpp>,
        SourceEffect: FnMut(u32, u8) -> SpellPetAuraSourceLookupLikeCpp,
        AuraExists: FnMut(u32) -> bool,
    {
        let mut store = Self::default();
        let mut loaded_row_count = 0;
        let mut errors = Vec::new();

        for row in rows {
            let key = Self::key_like_cpp(row.spell_id, row.effect_index);
            if let Some(pet_aura) = store.auras_by_spell_effect_key.get_mut(&key) {
                pet_aura.add_aura_like_cpp(row.pet_entry, row.aura_id);
                loaded_row_count += 1;
                continue;
            }

            let source_effect = match source_effect_lookup(row.spell_id, row.effect_index) {
                SpellPetAuraSourceLookupLikeCpp::SpellMissing => {
                    errors.push(SpellPetAuraLoadErrorLikeCpp {
                        row,
                        kind: SpellPetAuraLoadErrorKindLikeCpp::SpellMissing,
                    });
                    continue;
                }
                SpellPetAuraSourceLookupLikeCpp::EffectIndexMissing => {
                    errors.push(SpellPetAuraLoadErrorLikeCpp {
                        row,
                        kind: SpellPetAuraLoadErrorKindLikeCpp::EffectIndexMissing,
                    });
                    continue;
                }
                SpellPetAuraSourceLookupLikeCpp::Found(effect) => effect,
            };

            if !source_effect.is_valid_pet_aura_source_like_cpp() {
                errors.push(SpellPetAuraLoadErrorLikeCpp {
                    row,
                    kind: SpellPetAuraLoadErrorKindLikeCpp::SourceEffectNotDummy,
                });
                continue;
            }

            if !aura_spell_exists(row.aura_id) {
                errors.push(SpellPetAuraLoadErrorLikeCpp {
                    row,
                    kind: SpellPetAuraLoadErrorKindLikeCpp::AuraSpellMissing,
                });
                continue;
            }

            let pet_aura = PetAuraLikeCpp::new(
                row.pet_entry,
                row.aura_id,
                source_effect.target_a == TARGET_UNIT_PET_LIKE_CPP,
                source_effect.calc_value,
            );
            store.auras_by_spell_effect_key.insert(key, pet_aura);
            loaded_row_count += 1;
        }

        SpellPetAuraLoadOutcomeLikeCpp {
            store,
            loaded_row_count,
            errors,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellPetAuraLoadOutcomeLikeCpp {
    pub store: SpellPetAuraStoreLikeCpp,
    pub loaded_row_count: usize,
    pub errors: Vec<SpellPetAuraLoadErrorLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpellThreatRowLikeCpp {
    pub spell_id: u32,
    pub flat_mod: i32,
    pub pct_mod: f32,
    pub ap_pct_mod: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpellThreatEntryLikeCpp {
    pub flat_mod: i32,
    pub pct_mod: f32,
    pub ap_pct_mod: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpellThreatLoadErrorLikeCpp {
    pub row: SpellThreatRowLikeCpp,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpellThreatStoreLikeCpp {
    pub entries_by_spell_id: HashMap<u32, SpellThreatEntryLikeCpp>,
}

impl SpellThreatStoreLikeCpp {
    pub async fn load_like_cpp(
        db: &WorldDatabase,
        spells: &SpellStore,
    ) -> Result<SpellThreatLoadOutcomeLikeCpp> {
        let stmt = db.prepare(WorldStatements::SEL_SPELL_THREATS);
        let mut result = db.query(&stmt).await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(SpellThreatRowLikeCpp {
                    spell_id: result.try_read::<u32>(0).unwrap_or(0),
                    flat_mod: result.try_read::<i32>(1).unwrap_or(0),
                    pct_mod: result.try_read::<f32>(2).unwrap_or(0.0),
                    ap_pct_mod: result.try_read::<f32>(3).unwrap_or(0.0),
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_rows_like_cpp(rows, |spell_id| {
            spells.get(spell_id as i32).is_some()
        }))
    }

    pub fn from_rows_like_cpp<I, SpellExists>(
        rows: I,
        mut spell_exists: SpellExists,
    ) -> SpellThreatLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellThreatRowLikeCpp>,
        SpellExists: FnMut(u32) -> bool,
    {
        let mut store = Self::default();
        let mut loaded_row_count = 0;
        let mut errors = Vec::new();

        for row in rows {
            if !spell_exists(row.spell_id) {
                errors.push(SpellThreatLoadErrorLikeCpp { row });
                continue;
            }

            store.entries_by_spell_id.insert(
                row.spell_id,
                SpellThreatEntryLikeCpp {
                    flat_mod: row.flat_mod,
                    pct_mod: row.pct_mod,
                    ap_pct_mod: row.ap_pct_mod,
                },
            );
            loaded_row_count += 1;
        }

        SpellThreatLoadOutcomeLikeCpp {
            store,
            loaded_row_count,
            errors,
        }
    }

    pub fn get_spell_threat_entry_like_cpp<FirstSpellInChain>(
        &self,
        spell_id: u32,
        mut first_spell_in_chain: FirstSpellInChain,
    ) -> Option<&SpellThreatEntryLikeCpp>
    where
        FirstSpellInChain: FnMut(u32) -> u32,
    {
        self.entries_by_spell_id.get(&spell_id).or_else(|| {
            self.entries_by_spell_id
                .get(&first_spell_in_chain(spell_id))
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellThreatLoadOutcomeLikeCpp {
    pub store: SpellThreatStoreLikeCpp,
    pub loaded_row_count: usize,
    pub errors: Vec<SpellThreatLoadErrorLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SpellLinkedTypeLikeCpp {
    Cast,
    Hit,
    Aura,
    Remove,
}

impl SpellLinkedTypeLikeCpp {
    pub fn from_u8_like_cpp(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Cast),
            1 => Some(Self::Hit),
            2 => Some(Self::Aura),
            3 => Some(Self::Remove),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellLinkedRowLikeCpp {
    pub spell_trigger: i32,
    pub spell_effect: i32,
    pub link_type: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellLinkedSpellInfoLikeCpp {
    /// Precomputed C++ `SpellEffectInfo::CalcValue()` values paired with
    /// `EffectIndex`. Rust does not have full CalcValue yet, so callers must
    /// pass authoritative values when this warning needs exact parity.
    pub effect_calc_values_by_index: Vec<(u32, i32)>,
}

impl SpellLinkedSpellInfoLikeCpp {
    pub fn from_represented_spell_info_base_points(spell_info: &SpellInfo) -> Self {
        Self {
            effect_calc_values_by_index: spell_info
                .effects()
                .iter()
                .map(|effect| (effect.effect_index, effect.effect_base_points))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellLinkedLoadErrorKindLikeCpp {
    TriggerSpellMissing,
    EffectSpellMissing,
    InvalidLinkType,
    SelfTriggerLoop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellLinkedLoadErrorLikeCpp {
    pub row: SpellLinkedRowLikeCpp,
    pub kind: SpellLinkedLoadErrorKindLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellLinkedLoadWarningKindLikeCpp {
    TriggerEffectSameBasePoint { effect_index: u32 },
    NegativeTriggerLinkTypeCoercedToRemove,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellLinkedLoadWarningLikeCpp {
    pub row: SpellLinkedRowLikeCpp,
    pub kind: SpellLinkedLoadWarningKindLikeCpp,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellLinkedStoreLikeCpp {
    pub effects_by_type_and_trigger: BTreeMap<(SpellLinkedTypeLikeCpp, u32), Vec<i32>>,
}

impl SpellLinkedStoreLikeCpp {
    pub fn from_rows_like_cpp<I, SpellLookup>(
        rows: I,
        mut spell_lookup: SpellLookup,
    ) -> SpellLinkedLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellLinkedRowLikeCpp>,
        SpellLookup: FnMut(u32) -> Option<SpellLinkedSpellInfoLikeCpp>,
    {
        let mut store = Self::default();
        let mut loaded_row_count = 0;
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        for row in rows {
            let trigger_spell_id = row.spell_trigger.unsigned_abs();
            let effect_spell_id = row.spell_effect.unsigned_abs();
            let Some(trigger_spell) = spell_lookup(trigger_spell_id) else {
                errors.push(SpellLinkedLoadErrorLikeCpp {
                    row,
                    kind: SpellLinkedLoadErrorKindLikeCpp::TriggerSpellMissing,
                });
                continue;
            };

            if row.spell_effect >= 0 {
                for (effect_index, calc_value) in trigger_spell.effect_calc_values_by_index {
                    if calc_value == row.spell_effect.abs() {
                        warnings.push(SpellLinkedLoadWarningLikeCpp {
                            row: row.clone(),
                            kind: SpellLinkedLoadWarningKindLikeCpp::TriggerEffectSameBasePoint {
                                effect_index,
                            },
                        });
                    }
                }
            }

            if spell_lookup(effect_spell_id).is_none() {
                errors.push(SpellLinkedLoadErrorLikeCpp {
                    row,
                    kind: SpellLinkedLoadErrorKindLikeCpp::EffectSpellMissing,
                });
                continue;
            }

            let Some(mut link_type) = SpellLinkedTypeLikeCpp::from_u8_like_cpp(row.link_type)
            else {
                errors.push(SpellLinkedLoadErrorLikeCpp {
                    row,
                    kind: SpellLinkedLoadErrorKindLikeCpp::InvalidLinkType,
                });
                continue;
            };

            let trigger_key = if row.spell_trigger < 0 {
                if link_type != SpellLinkedTypeLikeCpp::Cast {
                    warnings.push(SpellLinkedLoadWarningLikeCpp {
                        row: row.clone(),
                        kind: SpellLinkedLoadWarningKindLikeCpp::NegativeTriggerLinkTypeCoercedToRemove,
                    });
                }
                link_type = SpellLinkedTypeLikeCpp::Remove;
                trigger_spell_id
            } else {
                row.spell_trigger as u32
            };

            if link_type != SpellLinkedTypeLikeCpp::Aura
                && trigger_key <= i32::MAX as u32
                && trigger_key as i32 == row.spell_effect
            {
                errors.push(SpellLinkedLoadErrorLikeCpp {
                    row,
                    kind: SpellLinkedLoadErrorKindLikeCpp::SelfTriggerLoop,
                });
                continue;
            }

            store
                .effects_by_type_and_trigger
                .entry((link_type, trigger_key))
                .or_default()
                .push(row.spell_effect);
            loaded_row_count += 1;
        }

        SpellLinkedLoadOutcomeLikeCpp {
            store,
            loaded_row_count,
            errors,
            warnings,
        }
    }

    pub fn get_spell_linked_like_cpp(
        &self,
        link_type: SpellLinkedTypeLikeCpp,
        spell_id: u32,
    ) -> Option<&[i32]> {
        self.effects_by_type_and_trigger
            .get(&(link_type, spell_id))
            .map(Vec::as_slice)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellLinkedLoadOutcomeLikeCpp {
    pub store: SpellLinkedStoreLikeCpp,
    pub loaded_row_count: usize,
    pub errors: Vec<SpellLinkedLoadErrorLikeCpp>,
    pub warnings: Vec<SpellLinkedLoadWarningLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellTotemModelRowLikeCpp {
    pub spell_id: u32,
    pub race_id: u8,
    pub display_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellTotemModelLoadErrorKindLikeCpp {
    SpellMissing,
    RaceMissing,
    DisplayMissing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellTotemModelLoadErrorLikeCpp {
    pub row: SpellTotemModelRowLikeCpp,
    pub kind: SpellTotemModelLoadErrorKindLikeCpp,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellTotemModelStoreLikeCpp {
    pub display_id_by_spell_and_race: BTreeMap<(u32, u8), u32>,
}

impl SpellTotemModelStoreLikeCpp {
    pub fn from_rows_like_cpp<I, SpellExists, RaceExists, DisplayExists>(
        rows: I,
        mut spell_exists: SpellExists,
        mut race_exists: RaceExists,
        mut display_exists: DisplayExists,
    ) -> SpellTotemModelLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellTotemModelRowLikeCpp>,
        SpellExists: FnMut(u32) -> bool,
        RaceExists: FnMut(u8) -> bool,
        DisplayExists: FnMut(u32) -> bool,
    {
        let mut store = Self::default();
        let mut loaded_row_count = 0;
        let mut errors = Vec::new();

        for row in rows {
            if !spell_exists(row.spell_id) {
                errors.push(SpellTotemModelLoadErrorLikeCpp {
                    row,
                    kind: SpellTotemModelLoadErrorKindLikeCpp::SpellMissing,
                });
                continue;
            }

            if !race_exists(row.race_id) {
                errors.push(SpellTotemModelLoadErrorLikeCpp {
                    row,
                    kind: SpellTotemModelLoadErrorKindLikeCpp::RaceMissing,
                });
                continue;
            }

            if !display_exists(row.display_id) {
                errors.push(SpellTotemModelLoadErrorLikeCpp {
                    row,
                    kind: SpellTotemModelLoadErrorKindLikeCpp::DisplayMissing,
                });
                continue;
            }

            store
                .display_id_by_spell_and_race
                .insert((row.spell_id, row.race_id), row.display_id);
            loaded_row_count += 1;
        }

        SpellTotemModelLoadOutcomeLikeCpp {
            store,
            loaded_row_count,
            errors,
        }
    }

    pub fn get_model_for_totem_like_cpp(&self, spell_id: u32, race_id: u8) -> u32 {
        self.display_id_by_spell_and_race
            .get(&(spell_id, race_id))
            .copied()
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellTotemModelLoadOutcomeLikeCpp {
    pub store: SpellTotemModelStoreLikeCpp,
    pub loaded_row_count: usize,
    pub errors: Vec<SpellTotemModelLoadErrorLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellRequiredRowLikeCpp {
    pub spell_id: u32,
    pub req_spell: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellRequiredLoadErrorKindLikeCpp {
    SpellMissing,
    RequiredSpellMissing,
    SameRankChain,
    Duplicate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellRequiredLoadErrorLikeCpp {
    pub row: SpellRequiredRowLikeCpp,
    pub kind: SpellRequiredLoadErrorKindLikeCpp,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellRequiredStoreLikeCpp {
    pub required_by_spell_id: BTreeMap<u32, Vec<u32>>,
    pub requiring_by_required_spell_id: BTreeMap<u32, Vec<u32>>,
}

impl SpellRequiredStoreLikeCpp {
    pub async fn load_like_cpp(
        db: &WorldDatabase,
        spells: &SpellStore,
        spell_chains: &SpellChainStoreLikeCpp,
    ) -> Result<SpellRequiredLoadOutcomeLikeCpp> {
        let stmt = db.prepare(WorldStatements::SEL_SPELL_REQUIRED);
        let mut result = db.query(&stmt).await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(SpellRequiredRowLikeCpp {
                    spell_id: result.try_read::<u32>(0).unwrap_or(0),
                    req_spell: result.try_read::<u32>(1).unwrap_or(0),
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_rows_like_cpp(
            rows,
            |spell_id| spells.get(spell_id as i32).is_some(),
            |spell_id, req_spell| spell_chains.is_rank_of_like_cpp(spell_id, req_spell),
        ))
    }

    pub fn from_rows_like_cpp<I, SpellExists, SameRankChain>(
        rows: I,
        mut spell_exists: SpellExists,
        mut same_rank_chain: SameRankChain,
    ) -> SpellRequiredLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellRequiredRowLikeCpp>,
        SpellExists: FnMut(u32) -> bool,
        SameRankChain: FnMut(u32, u32) -> bool,
    {
        let mut store = Self::default();
        let mut loaded_row_count = 0;
        let mut errors = Vec::new();

        for row in rows {
            if !spell_exists(row.spell_id) {
                errors.push(SpellRequiredLoadErrorLikeCpp {
                    row,
                    kind: SpellRequiredLoadErrorKindLikeCpp::SpellMissing,
                });
                continue;
            }

            if !spell_exists(row.req_spell) {
                errors.push(SpellRequiredLoadErrorLikeCpp {
                    row,
                    kind: SpellRequiredLoadErrorKindLikeCpp::RequiredSpellMissing,
                });
                continue;
            }

            if same_rank_chain(row.spell_id, row.req_spell) {
                errors.push(SpellRequiredLoadErrorLikeCpp {
                    row,
                    kind: SpellRequiredLoadErrorKindLikeCpp::SameRankChain,
                });
                continue;
            }

            if store.is_spell_requiring_spell_like_cpp(row.spell_id, row.req_spell) {
                errors.push(SpellRequiredLoadErrorLikeCpp {
                    row,
                    kind: SpellRequiredLoadErrorKindLikeCpp::Duplicate,
                });
                continue;
            }

            store
                .required_by_spell_id
                .entry(row.spell_id)
                .or_default()
                .push(row.req_spell);
            store
                .requiring_by_required_spell_id
                .entry(row.req_spell)
                .or_default()
                .push(row.spell_id);
            loaded_row_count += 1;
        }

        SpellRequiredLoadOutcomeLikeCpp {
            store,
            loaded_row_count,
            errors,
        }
    }

    pub fn spells_required_for_spell_like_cpp(&self, spell_id: u32) -> &[u32] {
        self.required_by_spell_id
            .get(&spell_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn spells_requiring_spell_like_cpp(&self, req_spell: u32) -> &[u32] {
        self.requiring_by_required_spell_id
            .get(&req_spell)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn is_spell_requiring_spell_like_cpp(&self, spell_id: u32, req_spell: u32) -> bool {
        self.spells_requiring_spell_like_cpp(req_spell)
            .contains(&spell_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellRequiredLoadOutcomeLikeCpp {
    pub store: SpellRequiredStoreLikeCpp,
    pub loaded_row_count: usize,
    pub errors: Vec<SpellRequiredLoadErrorLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellLearnSkillNodeLikeCpp {
    pub skill: u16,
    pub step: u16,
    pub value: u16,
    pub maxvalue: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellLearnSkillEffectLikeCpp {
    pub effect: u32,
    pub misc_value: i32,
    /// Precomputed C++ `SpellEffectInfo::CalcValue()` for
    /// `SPELL_EFFECT_SKILL`.
    pub calc_value: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellLearnSkillSourceSpellInfoLikeCpp {
    pub spell_id: u32,
    pub difficulty_none: bool,
    pub effects: Vec<SpellLearnSkillEffectLikeCpp>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellLearnSkillStoreLikeCpp {
    pub skill_by_spell_id: BTreeMap<u32, SpellLearnSkillNodeLikeCpp>,
}

impl SpellLearnSkillStoreLikeCpp {
    pub fn from_spell_infos_like_cpp<I>(source_spells: I) -> SpellLearnSkillLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellLearnSkillSourceSpellInfoLikeCpp>,
    {
        let mut store = Self::default();
        let mut dbc_loaded_row_count = 0;

        for source_spell in source_spells {
            if !source_spell.difficulty_none {
                continue;
            }

            for effect in source_spell.effects {
                let node = match effect.effect {
                    spell_effect_types::SPELL_EFFECT_SKILL => SpellLearnSkillNodeLikeCpp {
                        skill: effect.misc_value as u16,
                        step: effect.calc_value as u16,
                        value: 0,
                        maxvalue: 0,
                    },
                    spell_effect_types::SPELL_EFFECT_DUAL_WIELD => SpellLearnSkillNodeLikeCpp {
                        skill: SKILL_DUAL_WIELD_LIKE_CPP,
                        step: 1,
                        value: 1,
                        maxvalue: 1,
                    },
                    _ => continue,
                };

                store.skill_by_spell_id.insert(source_spell.spell_id, node);
                dbc_loaded_row_count += 1;
                break;
            }
        }

        SpellLearnSkillLoadOutcomeLikeCpp {
            store,
            dbc_loaded_row_count,
        }
    }

    pub fn get_spell_learn_skill_like_cpp(
        &self,
        spell_id: u32,
    ) -> Option<&SpellLearnSkillNodeLikeCpp> {
        self.skill_by_spell_id.get(&spell_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellLearnSkillLoadOutcomeLikeCpp {
    pub store: SpellLearnSkillStoreLikeCpp,
    pub dbc_loaded_row_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellRankEdgeLikeCpp {
    pub spell_id: u32,
    pub supercedes_spell_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellChainNodeLikeCpp {
    pub prev_spell_id: Option<u32>,
    pub next_spell_id: Option<u32>,
    pub first_spell_id: u32,
    pub last_spell_id: u32,
    pub rank: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellChainStoreLikeCpp {
    pub chains_by_spell_id: BTreeMap<u32, SpellChainNodeLikeCpp>,
}

impl SpellChainStoreLikeCpp {
    pub fn from_skill_line_ability_supercedes_like_cpp<I, SpellExists>(
        rows: I,
        mut spell_exists: SpellExists,
    ) -> Self
    where
        I: IntoIterator<Item = SpellRankEdgeLikeCpp>,
        SpellExists: FnMut(u32) -> bool,
    {
        let mut chain_next_by_spell_id = BTreeMap::new();
        let mut has_prev = BTreeSet::new();

        for row in rows {
            if row.supercedes_spell_id == 0 {
                continue;
            }

            if !spell_exists(row.supercedes_spell_id) || !spell_exists(row.spell_id) {
                continue;
            }

            chain_next_by_spell_id.insert(row.supercedes_spell_id, row.spell_id);
            has_prev.insert(row.spell_id);
        }

        let mut store = Self::default();
        for (spell_id, next_spell_id) in chain_next_by_spell_id.clone() {
            if has_prev.contains(&spell_id) {
                continue;
            }

            let first_spell_id = spell_id;
            store.chains_by_spell_id.insert(
                spell_id,
                SpellChainNodeLikeCpp {
                    prev_spell_id: None,
                    next_spell_id: Some(next_spell_id),
                    first_spell_id,
                    last_spell_id: next_spell_id,
                    rank: 1,
                },
            );
            store.chains_by_spell_id.insert(
                next_spell_id,
                SpellChainNodeLikeCpp {
                    prev_spell_id: Some(first_spell_id),
                    next_spell_id: None,
                    first_spell_id,
                    last_spell_id: next_spell_id,
                    rank: 2,
                },
            );

            let mut rank = 3;
            let mut current_spell_id = next_spell_id;
            while let Some(last_spell_id) = chain_next_by_spell_id.get(&current_spell_id).copied() {
                if let Some(current_node) = store.chains_by_spell_id.get_mut(&current_spell_id) {
                    current_node.next_spell_id = Some(last_spell_id);
                }

                store.chains_by_spell_id.insert(
                    last_spell_id,
                    SpellChainNodeLikeCpp {
                        prev_spell_id: Some(current_spell_id),
                        next_spell_id: None,
                        first_spell_id,
                        last_spell_id,
                        rank,
                    },
                );
                rank = rank.saturating_add(1);

                let mut prev_to_update = Some(current_spell_id);
                while let Some(prev_spell_id) = prev_to_update {
                    prev_to_update = store
                        .chains_by_spell_id
                        .get(&prev_spell_id)
                        .and_then(|node| node.prev_spell_id);
                    if let Some(prev_node) = store.chains_by_spell_id.get_mut(&prev_spell_id) {
                        prev_node.last_spell_id = last_spell_id;
                    }
                }

                current_spell_id = last_spell_id;
            }
        }

        store
    }

    pub fn spell_chain_node_like_cpp(&self, spell_id: u32) -> Option<&SpellChainNodeLikeCpp> {
        self.chains_by_spell_id.get(&spell_id)
    }

    pub fn first_spell_in_chain_like_cpp(&self, spell_id: u32) -> u32 {
        self.spell_chain_node_like_cpp(spell_id)
            .map(|node| node.first_spell_id)
            .unwrap_or(spell_id)
    }

    pub fn is_rank_of_like_cpp(&self, spell_id: u32, other_spell_id: u32) -> bool {
        self.first_spell_in_chain_like_cpp(spell_id)
            == self.first_spell_in_chain_like_cpp(other_spell_id)
    }

    pub fn last_spell_in_chain_like_cpp(&self, spell_id: u32) -> u32 {
        self.spell_chain_node_like_cpp(spell_id)
            .map(|node| node.last_spell_id)
            .unwrap_or(spell_id)
    }

    pub fn next_spell_in_chain_like_cpp(&self, spell_id: u32) -> u32 {
        self.spell_chain_node_like_cpp(spell_id)
            .and_then(|node| node.next_spell_id)
            .unwrap_or(0)
    }

    pub fn prev_spell_in_chain_like_cpp(&self, spell_id: u32) -> u32 {
        self.spell_chain_node_like_cpp(spell_id)
            .and_then(|node| node.prev_spell_id)
            .unwrap_or(0)
    }

    pub fn spell_rank_like_cpp(&self, spell_id: u32) -> u8 {
        self.spell_chain_node_like_cpp(spell_id)
            .map(|node| node.rank)
            .unwrap_or(0)
    }

    pub fn spell_with_rank_like_cpp(&self, spell_id: u32, rank: u32, strict: bool) -> u32 {
        let mut current_spell_id = spell_id;
        let mut seen = BTreeSet::new();

        loop {
            let Some(node) = self.spell_chain_node_like_cpp(current_spell_id) else {
                return if strict && rank > 1 {
                    0
                } else {
                    current_spell_id
                };
            };

            if u32::from(node.rank) == rank {
                return current_spell_id;
            }

            let next = if u32::from(node.rank) < rank {
                node.next_spell_id
            } else {
                node.prev_spell_id
            };

            let Some(next_spell_id) = next else {
                return if strict { 0 } else { current_spell_id };
            };

            if !seen.insert(current_spell_id) {
                return if strict { 0 } else { current_spell_id };
            }

            current_spell_id = next_spell_id;
        }
    }
}

pub const SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP: u8 = 0x1;
pub const SPELL_AREA_FLAG_AUTOREMOVE_LIKE_CPP: u8 = 0x2;
pub const SPELL_AREA_FLAG_IGNORE_AUTOCAST_ON_QUEST_STATUS_CHANGE_LIKE_CPP: u8 = 0x4;
pub const GENDER_MALE_LIKE_CPP: u8 = 0;
pub const GENDER_FEMALE_LIKE_CPP: u8 = 1;
pub const GENDER_NONE_LIKE_CPP: u8 = 2;
pub const SPELL_ATTR0_CU_SHARE_DAMAGE_LIKE_CPP: u32 = 0x0000_0008;
pub const SPELL_ATTR0_CU_NO_INITIAL_THREAT_LIKE_CPP: u32 = 0x0000_0010;
pub const SPELL_ATTR0_CU_CAN_CRIT_LIKE_CPP: u32 = 0x0000_0080;
pub const SPELL_ATTR0_CU_DIRECT_DAMAGE_LIKE_CPP: u32 = 0x0000_0100;
pub const SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP: u32 = 0x0080_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellAreaRowLikeCpp {
    pub spell_id: u32,
    pub area_id: u32,
    pub quest_start: u32,
    pub quest_start_status: u32,
    pub quest_end_status: u32,
    pub quest_end: u32,
    pub aura_spell: i32,
    pub race_mask: u64,
    pub gender: u8,
    pub flags: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellAreaLikeCpp {
    pub spell_id: u32,
    pub area_id: u32,
    pub quest_start: u32,
    pub quest_end: u32,
    pub aura_spell: i32,
    pub race_mask: u64,
    pub gender: u8,
    pub quest_start_status: u32,
    pub quest_end_status: u32,
    pub flags: u8,
}

impl From<SpellAreaRowLikeCpp> for SpellAreaLikeCpp {
    fn from(row: SpellAreaRowLikeCpp) -> Self {
        Self {
            spell_id: row.spell_id,
            area_id: row.area_id,
            quest_start: row.quest_start,
            quest_end: row.quest_end,
            aura_spell: row.aura_spell,
            race_mask: row.race_mask,
            gender: row.gender,
            quest_start_status: row.quest_start_status,
            quest_end_status: row.quest_end_status,
            flags: row.flags,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellAreaLoadErrorKindLikeCpp {
    SpellMissing,
    DuplicateSimilarRequirements,
    AreaMissing,
    QuestStartMissing,
    QuestEndMissing,
    AuraSpellMissing,
    AuraSpellSelfRequirement,
    AuraAutocastChain,
    InvalidRaceMask,
    InvalidGender,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellAreaLoadErrorLikeCpp {
    pub row: SpellAreaRowLikeCpp,
    pub kind: SpellAreaLoadErrorKindLikeCpp,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellAreaStoreLikeCpp {
    areas: Vec<SpellAreaLikeCpp>,
    area_indices_by_spell_id: BTreeMap<u32, Vec<usize>>,
    area_indices_by_quest_start_or_end: BTreeMap<u32, Vec<usize>>,
    area_indices_by_quest_end: BTreeMap<u32, Vec<usize>>,
    area_indices_by_aura_spell: BTreeMap<u32, Vec<usize>>,
    area_indices_by_area_id: BTreeMap<u32, Vec<usize>>,
}

impl SpellAreaStoreLikeCpp {
    pub fn from_rows_like_cpp<I, SpellExists, AreaExists, QuestExists>(
        rows: I,
        mut spell_exists: SpellExists,
        mut area_exists: AreaExists,
        mut quest_exists: QuestExists,
    ) -> SpellAreaLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellAreaRowLikeCpp>,
        SpellExists: FnMut(u32) -> bool,
        AreaExists: FnMut(u32) -> bool,
        QuestExists: FnMut(u32) -> bool,
    {
        let mut store = Self::default();
        let mut errors = Vec::new();

        for row in rows {
            let spell_area = SpellAreaLikeCpp::from(row);

            if !spell_exists(spell_area.spell_id) {
                errors.push(SpellAreaLoadErrorLikeCpp {
                    row,
                    kind: SpellAreaLoadErrorKindLikeCpp::SpellMissing,
                });
                continue;
            }

            if store.has_similar_requirements_like_cpp(&spell_area) {
                errors.push(SpellAreaLoadErrorLikeCpp {
                    row,
                    kind: SpellAreaLoadErrorKindLikeCpp::DuplicateSimilarRequirements,
                });
                continue;
            }

            if spell_area.area_id != 0 && !area_exists(spell_area.area_id) {
                errors.push(SpellAreaLoadErrorLikeCpp {
                    row,
                    kind: SpellAreaLoadErrorKindLikeCpp::AreaMissing,
                });
                continue;
            }

            if spell_area.quest_start != 0 && !quest_exists(spell_area.quest_start) {
                errors.push(SpellAreaLoadErrorLikeCpp {
                    row,
                    kind: SpellAreaLoadErrorKindLikeCpp::QuestStartMissing,
                });
                continue;
            }

            if spell_area.quest_end != 0 && !quest_exists(spell_area.quest_end) {
                errors.push(SpellAreaLoadErrorLikeCpp {
                    row,
                    kind: SpellAreaLoadErrorKindLikeCpp::QuestEndMissing,
                });
                continue;
            }

            if spell_area.aura_spell != 0 {
                let aura_spell_id = spell_area.aura_spell.unsigned_abs();
                if !spell_exists(aura_spell_id) {
                    errors.push(SpellAreaLoadErrorLikeCpp {
                        row,
                        kind: SpellAreaLoadErrorKindLikeCpp::AuraSpellMissing,
                    });
                    continue;
                }

                if aura_spell_id == spell_area.spell_id {
                    errors.push(SpellAreaLoadErrorLikeCpp {
                        row,
                        kind: SpellAreaLoadErrorKindLikeCpp::AuraSpellSelfRequirement,
                    });
                    continue;
                }

                if spell_area.flags & SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP != 0
                    && spell_area.aura_spell > 0
                    && store.has_autocast_aura_chain_like_cpp(&spell_area)
                {
                    errors.push(SpellAreaLoadErrorLikeCpp {
                        row,
                        kind: SpellAreaLoadErrorKindLikeCpp::AuraAutocastChain,
                    });
                    continue;
                }
            }

            if spell_area.race_mask != 0
                && (spell_area.race_mask & RACEMASK_ALL_PLAYABLE_LIKE_CPP) == 0
            {
                errors.push(SpellAreaLoadErrorLikeCpp {
                    row,
                    kind: SpellAreaLoadErrorKindLikeCpp::InvalidRaceMask,
                });
                continue;
            }

            if !matches!(
                spell_area.gender,
                GENDER_NONE_LIKE_CPP | GENDER_FEMALE_LIKE_CPP | GENDER_MALE_LIKE_CPP
            ) {
                errors.push(SpellAreaLoadErrorLikeCpp {
                    row,
                    kind: SpellAreaLoadErrorKindLikeCpp::InvalidGender,
                });
                continue;
            }

            store.insert_like_cpp(spell_area);
        }

        SpellAreaLoadOutcomeLikeCpp {
            loaded_row_count: store.areas.len(),
            store,
            errors,
        }
    }

    pub fn spell_area_map_bounds_like_cpp(&self, spell_id: u32) -> Vec<&SpellAreaLikeCpp> {
        self.lookup_indices_like_cpp(&self.area_indices_by_spell_id, spell_id)
    }

    pub fn spell_area_for_quest_map_bounds_like_cpp(
        &self,
        quest_id: u32,
    ) -> Vec<&SpellAreaLikeCpp> {
        self.lookup_indices_like_cpp(&self.area_indices_by_quest_start_or_end, quest_id)
    }

    pub fn spell_area_for_quest_end_map_bounds_like_cpp(
        &self,
        quest_id: u32,
    ) -> Vec<&SpellAreaLikeCpp> {
        self.lookup_indices_like_cpp(&self.area_indices_by_quest_end, quest_id)
    }

    pub fn spell_area_for_aura_map_bounds_like_cpp(&self, spell_id: u32) -> Vec<&SpellAreaLikeCpp> {
        self.lookup_indices_like_cpp(&self.area_indices_by_aura_spell, spell_id)
    }

    pub fn spell_area_for_area_map_bounds_like_cpp(&self, area_id: u32) -> Vec<&SpellAreaLikeCpp> {
        self.lookup_indices_like_cpp(&self.area_indices_by_area_id, area_id)
    }

    pub fn areas_like_cpp(&self) -> &[SpellAreaLikeCpp] {
        &self.areas
    }

    fn lookup_indices_like_cpp(
        &self,
        index: &BTreeMap<u32, Vec<usize>>,
        key: u32,
    ) -> Vec<&SpellAreaLikeCpp> {
        index
            .get(&key)
            .into_iter()
            .flat_map(|indices| indices.iter())
            .filter_map(|idx| self.areas.get(*idx))
            .collect()
    }

    fn has_similar_requirements_like_cpp(&self, spell_area: &SpellAreaLikeCpp) -> bool {
        self.spell_area_map_bounds_like_cpp(spell_area.spell_id)
            .into_iter()
            .any(|existing| {
                spell_area.spell_id == existing.spell_id
                    && spell_area.area_id == existing.area_id
                    && spell_area.quest_start == existing.quest_start
                    && spell_area.aura_spell == existing.aura_spell
                    && (spell_area.race_mask & existing.race_mask) != 0
                    && spell_area.gender == existing.gender
            })
    }

    fn has_autocast_aura_chain_like_cpp(&self, spell_area: &SpellAreaLikeCpp) -> bool {
        self.spell_area_for_aura_map_bounds_like_cpp(spell_area.spell_id)
            .into_iter()
            .any(|existing| {
                existing.flags & SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP != 0 && existing.aura_spell > 0
            })
            || self
                .spell_area_map_bounds_like_cpp(spell_area.aura_spell as u32)
                .into_iter()
                .any(|existing| {
                    existing.flags & SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP != 0
                        && existing.aura_spell > 0
                })
    }

    fn insert_like_cpp(&mut self, spell_area: SpellAreaLikeCpp) {
        let idx = self.areas.len();
        self.areas.push(spell_area);
        self.area_indices_by_spell_id
            .entry(spell_area.spell_id)
            .or_default()
            .push(idx);

        if spell_area.area_id != 0 {
            self.area_indices_by_area_id
                .entry(spell_area.area_id)
                .or_default()
                .push(idx);
        }

        if spell_area.quest_start != 0 || spell_area.quest_end != 0 {
            if spell_area.quest_start == spell_area.quest_end {
                self.area_indices_by_quest_start_or_end
                    .entry(spell_area.quest_start)
                    .or_default()
                    .push(idx);
            } else {
                if spell_area.quest_start != 0 {
                    self.area_indices_by_quest_start_or_end
                        .entry(spell_area.quest_start)
                        .or_default()
                        .push(idx);
                }
                if spell_area.quest_end != 0 {
                    self.area_indices_by_quest_start_or_end
                        .entry(spell_area.quest_end)
                        .or_default()
                        .push(idx);
                }
            }
        }

        if spell_area.quest_end != 0 {
            self.area_indices_by_quest_end
                .entry(spell_area.quest_end)
                .or_default()
                .push(idx);
        }

        if spell_area.aura_spell != 0 {
            self.area_indices_by_aura_spell
                .entry(spell_area.aura_spell.unsigned_abs())
                .or_default()
                .push(idx);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellAreaLoadOutcomeLikeCpp {
    pub store: SpellAreaStoreLikeCpp,
    pub loaded_row_count: usize,
    pub errors: Vec<SpellAreaLoadErrorLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellCustomAttributeRowLikeCpp {
    pub spell_id: u32,
    pub attributes: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellCustomAttributeSourceSpellInfoLikeCpp {
    pub spell_id: u32,
    pub difficulty: u32,
    pub effects: Vec<SpellEffectInfo>,
}

impl SpellCustomAttributeSourceSpellInfoLikeCpp {
    fn has_effect_like_cpp(&self, effect_type: u32) -> bool {
        self.effects
            .iter()
            .any(|effect| effect.effect == effect_type)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SpellCustomAttributeKeyLikeCpp {
    pub spell_id: u32,
    pub difficulty: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellCustomAttributeLoadErrorKindLikeCpp {
    SpellMissing,
    ShareDamageWithoutSchoolDamage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellCustomAttributeLoadErrorLikeCpp {
    pub spell_id: u32,
    pub difficulty: Option<u32>,
    pub kind: SpellCustomAttributeLoadErrorKindLikeCpp,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellCustomAttributeStoreLikeCpp {
    pub attributes_by_spell_and_difficulty: BTreeMap<SpellCustomAttributeKeyLikeCpp, u32>,
}

impl SpellCustomAttributeStoreLikeCpp {
    pub fn from_sql_rows_like_cpp<I, SpellInfosById>(
        rows: I,
        mut spell_infos_by_id: SpellInfosById,
    ) -> SpellCustomAttributeLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellCustomAttributeRowLikeCpp>,
        SpellInfosById: FnMut(u32) -> Vec<SpellCustomAttributeSourceSpellInfoLikeCpp>,
    {
        let mut store = Self::default();
        let mut loaded_row_count = 0;
        let mut applied_variant_count = 0;
        let mut errors = Vec::new();

        for row in rows {
            let spell_infos = spell_infos_by_id(row.spell_id);
            if spell_infos.is_empty() {
                errors.push(SpellCustomAttributeLoadErrorLikeCpp {
                    spell_id: row.spell_id,
                    difficulty: None,
                    kind: SpellCustomAttributeLoadErrorKindLikeCpp::SpellMissing,
                });
                continue;
            }

            for spell_info in spell_infos {
                if row.attributes & SPELL_ATTR0_CU_SHARE_DAMAGE_LIKE_CPP != 0
                    && !spell_info
                        .has_effect_like_cpp(spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE)
                {
                    errors.push(SpellCustomAttributeLoadErrorLikeCpp {
                        spell_id: row.spell_id,
                        difficulty: Some(spell_info.difficulty),
                        kind: SpellCustomAttributeLoadErrorKindLikeCpp::ShareDamageWithoutSchoolDamage,
                    });
                    continue;
                }

                let key = SpellCustomAttributeKeyLikeCpp {
                    spell_id: spell_info.spell_id,
                    difficulty: spell_info.difficulty,
                };
                *store
                    .attributes_by_spell_and_difficulty
                    .entry(key)
                    .or_default() |= row.attributes;
                applied_variant_count += 1;
            }

            loaded_row_count += 1;
        }

        SpellCustomAttributeLoadOutcomeLikeCpp {
            store,
            loaded_row_count,
            applied_variant_count,
            errors,
        }
    }

    pub fn attributes_for_spell_difficulty_like_cpp(&self, spell_id: u32, difficulty: u32) -> u32 {
        self.attributes_by_spell_and_difficulty
            .get(&SpellCustomAttributeKeyLikeCpp {
                spell_id,
                difficulty,
            })
            .copied()
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellCustomAttributeLoadOutcomeLikeCpp {
    pub store: SpellCustomAttributeStoreLikeCpp,
    pub loaded_row_count: usize,
    pub applied_variant_count: usize,
    pub errors: Vec<SpellCustomAttributeLoadErrorLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellGroupRowLikeCpp {
    pub group_id: u32,
    pub spell_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellGroupLoadErrorKindLikeCpp {
    CoreRangeGroupMissing,
    ReferencedGroupMissing,
    SpellMissing,
    SpellNotFirstRank,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellGroupLoadErrorLikeCpp {
    pub row: SpellGroupRowLikeCpp,
    pub kind: SpellGroupLoadErrorKindLikeCpp,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellGroupStoreLikeCpp {
    pub spell_entries_by_group_id: BTreeMap<u32, Vec<i32>>,
    pub group_ids_by_spell_id: BTreeMap<u32, Vec<u32>>,
}

impl SpellGroupStoreLikeCpp {
    pub fn from_rows_like_cpp<I, SpellExists, SpellRank>(
        rows: I,
        mut spell_exists: SpellExists,
        mut spell_rank: SpellRank,
    ) -> SpellGroupLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellGroupRowLikeCpp>,
        SpellExists: FnMut(u32) -> bool,
        SpellRank: FnMut(u32) -> u32,
    {
        let mut store = Self::default();
        let mut group_ids = BTreeSet::new();
        let mut errors = Vec::new();

        for row in rows {
            if row.group_id <= SPELL_GROUP_DB_RANGE_MIN_LIKE_CPP
                && row.group_id >= SPELL_GROUP_CORE_RANGE_MAX_LIKE_CPP
            {
                errors.push(SpellGroupLoadErrorLikeCpp {
                    row,
                    kind: SpellGroupLoadErrorKindLikeCpp::CoreRangeGroupMissing,
                });
                continue;
            }

            group_ids.insert(row.group_id);
            store
                .spell_entries_by_group_id
                .entry(row.group_id)
                .or_default()
                .push(row.spell_id);
        }

        for (group_id, entries) in store.spell_entries_by_group_id.clone() {
            let mut retained_entries = Vec::new();

            for spell_id in entries {
                let row = SpellGroupRowLikeCpp { group_id, spell_id };
                if spell_id < 0 {
                    if !group_ids.contains(&spell_id.unsigned_abs()) {
                        errors.push(SpellGroupLoadErrorLikeCpp {
                            row,
                            kind: SpellGroupLoadErrorKindLikeCpp::ReferencedGroupMissing,
                        });
                        continue;
                    }
                } else {
                    let spell_id_u32 = spell_id as u32;
                    if !spell_exists(spell_id_u32) {
                        errors.push(SpellGroupLoadErrorLikeCpp {
                            row,
                            kind: SpellGroupLoadErrorKindLikeCpp::SpellMissing,
                        });
                        continue;
                    }

                    if spell_rank(spell_id_u32) > 1 {
                        errors.push(SpellGroupLoadErrorLikeCpp {
                            row,
                            kind: SpellGroupLoadErrorKindLikeCpp::SpellNotFirstRank,
                        });
                        continue;
                    }
                }

                retained_entries.push(spell_id);
            }

            if retained_entries.is_empty() {
                store.spell_entries_by_group_id.remove(&group_id);
            } else {
                store
                    .spell_entries_by_group_id
                    .insert(group_id, retained_entries);
            }
        }

        let mut loaded_row_count = 0;
        for group_id in group_ids {
            let spells = store.set_of_spells_in_spell_group_like_cpp(group_id);
            for spell_id in spells {
                store
                    .group_ids_by_spell_id
                    .entry(spell_id)
                    .or_default()
                    .push(group_id);
                loaded_row_count += 1;
            }
        }

        SpellGroupLoadOutcomeLikeCpp {
            store,
            loaded_row_count,
            errors,
        }
    }

    pub fn spell_group_spell_map_bounds_like_cpp(&self, group_id: u32) -> &[i32] {
        self.spell_entries_by_group_id
            .get(&group_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn spell_spell_group_map_bounds_like_cpp<FirstSpellInChain>(
        &self,
        spell_id: u32,
        mut first_spell_in_chain: FirstSpellInChain,
    ) -> &[u32]
    where
        FirstSpellInChain: FnMut(u32) -> u32,
    {
        let first_spell_id = first_spell_in_chain(spell_id);
        self.group_ids_by_spell_id
            .get(&first_spell_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn is_spell_member_of_spell_group_like_cpp<FirstSpellInChain>(
        &self,
        spell_id: u32,
        group_id: u32,
        first_spell_in_chain: FirstSpellInChain,
    ) -> bool
    where
        FirstSpellInChain: FnMut(u32) -> u32,
    {
        self.spell_spell_group_map_bounds_like_cpp(spell_id, first_spell_in_chain)
            .contains(&group_id)
    }

    pub fn set_of_spells_in_spell_group_like_cpp(&self, group_id: u32) -> BTreeSet<u32> {
        let mut found_spells = BTreeSet::new();
        let mut used_groups = BTreeSet::new();
        self.collect_spells_in_group_like_cpp(group_id, &mut found_spells, &mut used_groups);
        found_spells
    }

    fn collect_spells_in_group_like_cpp(
        &self,
        group_id: u32,
        found_spells: &mut BTreeSet<u32>,
        used_groups: &mut BTreeSet<u32>,
    ) {
        if !used_groups.insert(group_id) {
            return;
        }

        for spell_id in self.spell_group_spell_map_bounds_like_cpp(group_id) {
            if *spell_id < 0 {
                self.collect_spells_in_group_like_cpp(
                    spell_id.unsigned_abs(),
                    found_spells,
                    used_groups,
                );
            } else {
                found_spells.insert(*spell_id as u32);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellGroupLoadOutcomeLikeCpp {
    pub store: SpellGroupStoreLikeCpp,
    pub loaded_row_count: usize,
    pub errors: Vec<SpellGroupLoadErrorLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum SpellGroupStackRuleLikeCpp {
    Default = 0,
    Exclusive = 1,
    ExclusiveFromSameCaster = 2,
    ExclusiveSameEffect = 3,
    ExclusiveHighest = 4,
}

impl SpellGroupStackRuleLikeCpp {
    pub const MAX_LIKE_CPP: u8 = 5;

    pub const fn from_u8_like_cpp(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Default),
            1 => Some(Self::Exclusive),
            2 => Some(Self::ExclusiveFromSameCaster),
            3 => Some(Self::ExclusiveSameEffect),
            4 => Some(Self::ExclusiveHighest),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellGroupStackRuleRowLikeCpp {
    pub group_id: u32,
    pub stack_rule: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellGroupStackRuleLoadErrorKindLikeCpp {
    StackRuleMissing,
    GroupMissing,
    SameEffectSpellMissing,
    SameEffectSpellAuraMissing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellGroupStackRuleLoadErrorLikeCpp {
    pub row: SpellGroupStackRuleRowLikeCpp,
    pub spell_id: Option<u32>,
    pub kind: SpellGroupStackRuleLoadErrorKindLikeCpp,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellGroupStackRuleStoreLikeCpp {
    pub stack_rule_by_group_id: BTreeMap<u32, SpellGroupStackRuleLikeCpp>,
    pub same_effect_stack_by_group_id: BTreeMap<u32, BTreeSet<i32>>,
}

impl SpellGroupStackRuleStoreLikeCpp {
    pub fn from_rows_like_cpp<I, SpellInfoById, NextRankSpell>(
        rows: I,
        spell_groups: &SpellGroupStoreLikeCpp,
        mut spell_info_by_id: SpellInfoById,
        mut next_rank_spell: NextRankSpell,
    ) -> SpellGroupStackRuleLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellGroupStackRuleRowLikeCpp>,
        SpellInfoById: FnMut(u32) -> Option<SpellInfo>,
        NextRankSpell: FnMut(u32) -> Option<u32>,
    {
        let mut store = Self::default();
        let mut same_effect_groups = Vec::new();
        let mut errors = Vec::new();
        let mut loaded_row_count = 0;

        for row in rows {
            let Some(stack_rule) = SpellGroupStackRuleLikeCpp::from_u8_like_cpp(row.stack_rule)
            else {
                errors.push(SpellGroupStackRuleLoadErrorLikeCpp {
                    row,
                    spell_id: None,
                    kind: SpellGroupStackRuleLoadErrorKindLikeCpp::StackRuleMissing,
                });
                continue;
            };

            if spell_groups
                .spell_group_spell_map_bounds_like_cpp(row.group_id)
                .is_empty()
            {
                errors.push(SpellGroupStackRuleLoadErrorLikeCpp {
                    row,
                    spell_id: None,
                    kind: SpellGroupStackRuleLoadErrorKindLikeCpp::GroupMissing,
                });
                continue;
            }

            store
                .stack_rule_by_group_id
                .entry(row.group_id)
                .or_insert(stack_rule);

            if stack_rule == SpellGroupStackRuleLikeCpp::ExclusiveSameEffect {
                same_effect_groups.push(row.group_id);
            }

            loaded_row_count += 1;
        }

        let mut same_effect_parsed_count = 0;
        for group_id in same_effect_groups {
            let spell_ids = spell_groups.set_of_spells_in_spell_group_like_cpp(group_id);
            let aura_types =
                infer_same_effect_stack_aura_types_like_cpp(&spell_ids, &mut spell_info_by_id);

            for spell_id in spell_ids {
                if !spell_rank_chain_has_any_aura_like_cpp(
                    spell_id,
                    &aura_types,
                    &mut spell_info_by_id,
                    &mut next_rank_spell,
                ) {
                    let kind = if spell_info_by_id(spell_id).is_some() {
                        SpellGroupStackRuleLoadErrorKindLikeCpp::SameEffectSpellAuraMissing
                    } else {
                        SpellGroupStackRuleLoadErrorKindLikeCpp::SameEffectSpellMissing
                    };
                    errors.push(SpellGroupStackRuleLoadErrorLikeCpp {
                        row: SpellGroupStackRuleRowLikeCpp {
                            group_id,
                            stack_rule: SpellGroupStackRuleLikeCpp::ExclusiveSameEffect as u8,
                        },
                        spell_id: Some(spell_id),
                        kind,
                    });
                }
            }

            store
                .same_effect_stack_by_group_id
                .insert(group_id, aura_types);
            same_effect_parsed_count += 1;
        }

        SpellGroupStackRuleLoadOutcomeLikeCpp {
            store,
            loaded_row_count,
            same_effect_parsed_count,
            errors,
        }
    }

    pub fn spell_group_stack_rule_like_cpp(&self, group_id: u32) -> SpellGroupStackRuleLikeCpp {
        self.stack_rule_by_group_id
            .get(&group_id)
            .copied()
            .unwrap_or(SpellGroupStackRuleLikeCpp::Default)
    }

    pub fn same_effect_stack_rule_aura_types_like_cpp(
        &self,
        group_id: u32,
    ) -> Option<&BTreeSet<i32>> {
        self.same_effect_stack_by_group_id.get(&group_id)
    }

    pub fn check_spell_group_stack_rules_like_cpp(
        &self,
        spell_groups: &SpellGroupStoreLikeCpp,
        first_rank_spell_id_1: u32,
        first_rank_spell_id_2: u32,
    ) -> SpellGroupStackRuleLikeCpp {
        let mut common_groups = BTreeSet::new();

        for group_id in spell_groups
            .spell_spell_group_map_bounds_like_cpp(first_rank_spell_id_1, |spell_id| spell_id)
        {
            if spell_groups.is_spell_member_of_spell_group_like_cpp(
                first_rank_spell_id_2,
                *group_id,
                |spell_id| spell_id,
            ) {
                let mut add = true;
                for entry in spell_groups.spell_group_spell_map_bounds_like_cpp(*group_id) {
                    if *entry < 0 {
                        let nested_group_id = entry.unsigned_abs();
                        if spell_groups.is_spell_member_of_spell_group_like_cpp(
                            first_rank_spell_id_1,
                            nested_group_id,
                            |spell_id| spell_id,
                        ) && spell_groups.is_spell_member_of_spell_group_like_cpp(
                            first_rank_spell_id_2,
                            nested_group_id,
                            |spell_id| spell_id,
                        ) {
                            add = false;
                            break;
                        }
                    }
                }

                if add {
                    common_groups.insert(*group_id);
                }
            }
        }

        let mut rule = SpellGroupStackRuleLikeCpp::Default;
        for group_id in common_groups {
            rule = self.spell_group_stack_rule_like_cpp(group_id);
            if rule != SpellGroupStackRuleLikeCpp::Default {
                break;
            }
        }
        rule
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellGroupStackRuleLoadOutcomeLikeCpp {
    pub store: SpellGroupStackRuleStoreLikeCpp,
    pub loaded_row_count: usize,
    pub same_effect_parsed_count: usize,
    pub errors: Vec<SpellGroupStackRuleLoadErrorLikeCpp>,
}

pub const SPELL_SCHOOL_MASK_ALL_LIKE_CPP: u8 = 0x7F;
pub const PROC_FLAG_HEARTBEAT_LIKE_CPP: u32 = 0x0000_0001;
pub const PROC_FLAG_KILL_LIKE_CPP: u32 = 0x0000_0002;
pub const PROC_FLAG_DEAL_MELEE_SWING_LIKE_CPP: u32 = 0x0000_0004;
pub const PROC_FLAG_TAKE_MELEE_SWING_LIKE_CPP: u32 = 0x0000_0008;
pub const PROC_FLAG_DEAL_MELEE_ABILITY_LIKE_CPP: u32 = 0x0000_0010;
pub const PROC_FLAG_TAKE_MELEE_ABILITY_LIKE_CPP: u32 = 0x0000_0020;
pub const PROC_FLAG_DEAL_RANGED_ATTACK_LIKE_CPP: u32 = 0x0000_0040;
pub const PROC_FLAG_TAKE_RANGED_ATTACK_LIKE_CPP: u32 = 0x0000_0080;
pub const PROC_FLAG_DEAL_RANGED_ABILITY_LIKE_CPP: u32 = 0x0000_0100;
pub const PROC_FLAG_TAKE_RANGED_ABILITY_LIKE_CPP: u32 = 0x0000_0200;
pub const PROC_FLAG_DEAL_HELPFUL_ABILITY_LIKE_CPP: u32 = 0x0000_0400;
pub const PROC_FLAG_TAKE_HELPFUL_ABILITY_LIKE_CPP: u32 = 0x0000_0800;
pub const PROC_FLAG_DEAL_HARMFUL_ABILITY_LIKE_CPP: u32 = 0x0000_1000;
pub const PROC_FLAG_TAKE_HARMFUL_ABILITY_LIKE_CPP: u32 = 0x0000_2000;
pub const PROC_FLAG_DEAL_HELPFUL_SPELL_LIKE_CPP: u32 = 0x0000_4000;
pub const PROC_FLAG_TAKE_HELPFUL_SPELL_LIKE_CPP: u32 = 0x0000_8000;
pub const PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP: u32 = 0x0001_0000;
pub const PROC_FLAG_TAKE_HARMFUL_SPELL_LIKE_CPP: u32 = 0x0002_0000;
pub const PROC_FLAG_DEAL_HARMFUL_PERIODIC_LIKE_CPP: u32 = 0x0004_0000;
pub const PROC_FLAG_TAKE_HARMFUL_PERIODIC_LIKE_CPP: u32 = 0x0008_0000;
pub const PROC_FLAG_TAKE_ANY_DAMAGE_LIKE_CPP: u32 = 0x0010_0000;
pub const PROC_FLAG_DEAL_HELPFUL_PERIODIC_LIKE_CPP: u32 = 0x0020_0000;
pub const PROC_FLAG_MAIN_HAND_WEAPON_SWING_LIKE_CPP: u32 = 0x0040_0000;
pub const PROC_FLAG_OFF_HAND_WEAPON_SWING_LIKE_CPP: u32 = 0x0080_0000;
pub const PROC_FLAG_TAKE_HELPFUL_PERIODIC_LIKE_CPP: u32 = 0x8000_0000;
pub const PROC_FLAG_2_CAST_SUCCESSFUL_LIKE_CPP: u32 = 0x0000_0004;
pub const PROC_SPELL_TYPE_DAMAGE_LIKE_CPP: u32 = 0x0000_0001;
pub const PROC_SPELL_TYPE_HEAL_LIKE_CPP: u32 = 0x0000_0002;
pub const PROC_SPELL_TYPE_NO_DMG_HEAL_LIKE_CPP: u32 = 0x0000_0004;
pub const PROC_SPELL_TYPE_MASK_ALL_LIKE_CPP: u32 = PROC_SPELL_TYPE_DAMAGE_LIKE_CPP
    | PROC_SPELL_TYPE_HEAL_LIKE_CPP
    | PROC_SPELL_TYPE_NO_DMG_HEAL_LIKE_CPP;
pub const PROC_SPELL_PHASE_CAST_LIKE_CPP: u32 = 0x0000_0001;
pub const PROC_SPELL_PHASE_HIT_LIKE_CPP: u32 = 0x0000_0002;
pub const PROC_SPELL_PHASE_FINISH_LIKE_CPP: u32 = 0x0000_0004;
pub const PROC_SPELL_PHASE_MASK_ALL_LIKE_CPP: u32 = PROC_SPELL_PHASE_CAST_LIKE_CPP
    | PROC_SPELL_PHASE_HIT_LIKE_CPP
    | PROC_SPELL_PHASE_FINISH_LIKE_CPP;
pub const PROC_HIT_NORMAL_LIKE_CPP: u32 = 0x0000_0001;
pub const PROC_HIT_CRITICAL_LIKE_CPP: u32 = 0x0000_0002;
pub const PROC_HIT_MISS_LIKE_CPP: u32 = 0x0000_0004;
pub const PROC_HIT_BLOCK_LIKE_CPP: u32 = 0x0000_0040;
pub const PROC_HIT_ABSORB_LIKE_CPP: u32 = 0x0000_0400;
pub const PROC_HIT_REFLECT_LIKE_CPP: u32 = 0x0000_0800;
pub const PROC_HIT_MASK_ALL_LIKE_CPP: u32 = 0x0007_FFFF;
pub const PROC_ATTR_REQ_SPELLMOD_LIKE_CPP: u32 = 0x0000_0008;
pub const PROC_ATTR_REQ_EXP_OR_HONOR_LIKE_CPP: u32 = 0x0000_0001;
pub const PROC_ATTR_TRIGGERED_CAN_PROC_LIKE_CPP: u32 = 0x0000_0002;
pub const PROC_ATTR_REQ_POWER_COST_LIKE_CPP: u32 = 0x0000_0004;
pub const PROC_ATTR_USE_STACKS_FOR_CHARGES_LIKE_CPP: u32 = 0x0000_0010;
pub const PROC_ATTR_REDUCE_PROC_60_LIKE_CPP: u32 = 0x0000_0080;
pub const PROC_ATTR_ALL_ALLOWED_LIKE_CPP: u32 = PROC_ATTR_REQ_EXP_OR_HONOR_LIKE_CPP
    | PROC_ATTR_TRIGGERED_CAN_PROC_LIKE_CPP
    | PROC_ATTR_REQ_POWER_COST_LIKE_CPP
    | PROC_ATTR_REQ_SPELLMOD_LIKE_CPP
    | PROC_ATTR_USE_STACKS_FOR_CHARGES_LIKE_CPP
    | PROC_ATTR_REDUCE_PROC_60_LIKE_CPP;
pub const SPELL_PROC_FLAG_MASK_LIKE_CPP: u32 = PROC_FLAG_DEAL_MELEE_ABILITY_LIKE_CPP
    | PROC_FLAG_TAKE_MELEE_ABILITY_LIKE_CPP
    | PROC_FLAG_DEAL_RANGED_ATTACK_LIKE_CPP
    | PROC_FLAG_TAKE_RANGED_ATTACK_LIKE_CPP
    | PROC_FLAG_DEAL_RANGED_ABILITY_LIKE_CPP
    | PROC_FLAG_TAKE_RANGED_ABILITY_LIKE_CPP
    | PROC_FLAG_DEAL_HELPFUL_ABILITY_LIKE_CPP
    | PROC_FLAG_TAKE_HELPFUL_ABILITY_LIKE_CPP
    | PROC_FLAG_DEAL_HARMFUL_ABILITY_LIKE_CPP
    | PROC_FLAG_TAKE_HARMFUL_ABILITY_LIKE_CPP
    | PROC_FLAG_DEAL_HELPFUL_SPELL_LIKE_CPP
    | PROC_FLAG_TAKE_HELPFUL_SPELL_LIKE_CPP
    | PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP
    | PROC_FLAG_TAKE_HARMFUL_SPELL_LIKE_CPP
    | PROC_FLAG_DEAL_HARMFUL_PERIODIC_LIKE_CPP
    | PROC_FLAG_TAKE_HARMFUL_PERIODIC_LIKE_CPP
    | PROC_FLAG_DEAL_HELPFUL_PERIODIC_LIKE_CPP
    | PROC_FLAG_TAKE_HELPFUL_PERIODIC_LIKE_CPP;
pub const DONE_HIT_PROC_FLAG_MASK_LIKE_CPP: u32 = PROC_FLAG_DEAL_MELEE_SWING_LIKE_CPP
    | PROC_FLAG_DEAL_RANGED_ATTACK_LIKE_CPP
    | PROC_FLAG_DEAL_MELEE_ABILITY_LIKE_CPP
    | PROC_FLAG_DEAL_RANGED_ABILITY_LIKE_CPP
    | PROC_FLAG_DEAL_HELPFUL_ABILITY_LIKE_CPP
    | PROC_FLAG_DEAL_HARMFUL_ABILITY_LIKE_CPP
    | PROC_FLAG_DEAL_HELPFUL_SPELL_LIKE_CPP
    | PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP
    | PROC_FLAG_DEAL_HARMFUL_PERIODIC_LIKE_CPP
    | PROC_FLAG_DEAL_HELPFUL_PERIODIC_LIKE_CPP
    | PROC_FLAG_MAIN_HAND_WEAPON_SWING_LIKE_CPP
    | PROC_FLAG_OFF_HAND_WEAPON_SWING_LIKE_CPP;
pub const TAKEN_HIT_PROC_FLAG_MASK_LIKE_CPP: u32 = PROC_FLAG_TAKE_MELEE_SWING_LIKE_CPP
    | PROC_FLAG_TAKE_RANGED_ATTACK_LIKE_CPP
    | PROC_FLAG_TAKE_MELEE_ABILITY_LIKE_CPP
    | PROC_FLAG_TAKE_RANGED_ABILITY_LIKE_CPP
    | PROC_FLAG_TAKE_HELPFUL_ABILITY_LIKE_CPP
    | PROC_FLAG_TAKE_HARMFUL_ABILITY_LIKE_CPP
    | PROC_FLAG_TAKE_HELPFUL_SPELL_LIKE_CPP
    | PROC_FLAG_TAKE_HARMFUL_SPELL_LIKE_CPP
    | PROC_FLAG_TAKE_HARMFUL_PERIODIC_LIKE_CPP
    | PROC_FLAG_TAKE_HELPFUL_PERIODIC_LIKE_CPP
    | PROC_FLAG_TAKE_ANY_DAMAGE_LIKE_CPP;
pub const REQ_SPELL_PHASE_PROC_FLAG_MASK_LIKE_CPP: u32 =
    SPELL_PROC_FLAG_MASK_LIKE_CPP & DONE_HIT_PROC_FLAG_MASK_LIKE_CPP;
pub const PROC_FLAG_DEATH_LIKE_CPP: u32 = 0x0100_0000;
pub const CAN_PROC_FROM_PROCS_UNRESTRICTED_DONE_FLAGS_LIKE_CPP: u32 =
    PROC_FLAG_DEAL_MELEE_ABILITY_LIKE_CPP
        | PROC_FLAG_DEAL_RANGED_ATTACK_LIKE_CPP
        | PROC_FLAG_DEAL_RANGED_ABILITY_LIKE_CPP
        | PROC_FLAG_DEAL_HELPFUL_ABILITY_LIKE_CPP
        | PROC_FLAG_DEAL_HARMFUL_ABILITY_LIKE_CPP
        | PROC_FLAG_DEAL_HELPFUL_SPELL_LIKE_CPP
        | PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP
        | PROC_FLAG_DEAL_HARMFUL_PERIODIC_LIKE_CPP
        | PROC_FLAG_DEAL_HELPFUL_PERIODIC_LIKE_CPP;

#[derive(Debug, Clone, PartialEq)]
pub struct SpellProcRowLikeCpp {
    pub spell_id: i32,
    pub school_mask: u8,
    pub spell_family_name: u16,
    pub spell_family_mask: [u32; 4],
    pub proc_flags: [u32; 2],
    pub spell_type_mask: u32,
    pub spell_phase_mask: u32,
    pub hit_mask: u32,
    pub attributes_mask: u32,
    pub disable_effects_mask: u32,
    pub procs_per_minute: f32,
    pub chance: f32,
    pub cooldown_ms: u32,
    pub charges: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellProcEntryLikeCpp {
    pub school_mask: u8,
    pub spell_family_name: u16,
    pub spell_family_mask: [u32; 4],
    pub proc_flags: [u32; 2],
    pub spell_type_mask: u32,
    pub spell_phase_mask: u32,
    pub hit_mask: u32,
    pub attributes_mask: u32,
    pub disable_effects_mask: u32,
    pub procs_per_minute: f32,
    pub chance: f32,
    pub cooldown_ms: u32,
    pub charges: u32,
}

impl SpellProcEntryLikeCpp {
    fn from_row_like_cpp(row: &SpellProcRowLikeCpp) -> Self {
        Self {
            school_mask: row.school_mask,
            spell_family_name: row.spell_family_name,
            spell_family_mask: row.spell_family_mask,
            proc_flags: row.proc_flags,
            spell_type_mask: row.spell_type_mask,
            spell_phase_mask: row.spell_phase_mask,
            hit_mask: row.hit_mask,
            attributes_mask: row.attributes_mask,
            disable_effects_mask: row.disable_effects_mask,
            procs_per_minute: row.procs_per_minute,
            chance: row.chance,
            cooldown_ms: row.cooldown_ms,
            charges: u32::from(row.charges),
        }
    }

    pub fn proc_flags_any_like_cpp(&self) -> bool {
        self.proc_flags[0] != 0 || self.proc_flags[1] != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellProcEventSpellInfoLikeCpp {
    pub spell_family_name: u16,
    pub spell_family_mask: [u32; 4],
}

impl SpellProcEventSpellInfoLikeCpp {
    pub fn is_affected_like_cpp(&self, family_name: u16, family_mask: [u32; 4]) -> bool {
        if family_name == 0 {
            return true;
        }

        if family_name != self.spell_family_name {
            return false;
        }

        if family_mask.iter().any(|mask| *mask != 0)
            && !family_mask
                .iter()
                .zip(self.spell_family_mask.iter())
                .any(|(required, actual)| required & actual != 0)
        {
            return false;
        }

        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellProcEventInfoLikeCpp {
    pub type_mask: [u32; 2],
    pub actor_is_player: bool,
    pub action_target_exists: bool,
    pub action_target_is_honor_or_xp: bool,
    pub proc_spell_has_positive_power_cost: Option<bool>,
    pub school_mask: u8,
    pub spell_info: Option<SpellProcEventSpellInfoLikeCpp>,
    pub spell_type_mask: u32,
    pub spell_phase_mask: u32,
    pub hit_mask: u32,
}

pub fn can_spell_trigger_proc_on_event_like_cpp(
    proc_entry: &SpellProcEntryLikeCpp,
    event_info: &SpellProcEventInfoLikeCpp,
) -> bool {
    if !proc_flags_intersect_like_cpp(event_info.type_mask, proc_entry.proc_flags) {
        return false;
    }

    if proc_entry.attributes_mask & PROC_ATTR_REQ_EXP_OR_HONOR_LIKE_CPP != 0
        && event_info.actor_is_player
        && event_info.action_target_exists
        && !event_info.action_target_is_honor_or_xp
    {
        return false;
    }

    if proc_entry.attributes_mask & PROC_ATTR_REQ_POWER_COST_LIKE_CPP != 0
        && event_info.proc_spell_has_positive_power_cost != Some(true)
    {
        return false;
    }

    if event_info.type_mask[0]
        & (PROC_FLAG_HEARTBEAT_LIKE_CPP | PROC_FLAG_KILL_LIKE_CPP | PROC_FLAG_DEATH_LIKE_CPP)
        != 0
    {
        return true;
    }

    if proc_entry.school_mask != 0 && event_info.school_mask & proc_entry.school_mask == 0 {
        return false;
    }

    if event_info.type_mask[0] & SPELL_PROC_FLAG_MASK_LIKE_CPP != 0 {
        if let Some(event_spell_info) = event_info.spell_info {
            if !event_spell_info
                .is_affected_like_cpp(proc_entry.spell_family_name, proc_entry.spell_family_mask)
            {
                return false;
            }
        }

        if proc_entry.spell_type_mask != 0
            && event_info.spell_type_mask & proc_entry.spell_type_mask == 0
        {
            return false;
        }
    }

    if event_info.type_mask[0] & REQ_SPELL_PHASE_PROC_FLAG_MASK_LIKE_CPP != 0
        && event_info.spell_phase_mask & proc_entry.spell_phase_mask == 0
    {
        return false;
    }

    if event_info.type_mask[0] & TAKEN_HIT_PROC_FLAG_MASK_LIKE_CPP != 0
        || (event_info.type_mask[0] & DONE_HIT_PROC_FLAG_MASK_LIKE_CPP != 0
            && event_info.spell_phase_mask & PROC_SPELL_PHASE_CAST_LIKE_CPP == 0)
    {
        let mut hit_mask = proc_entry.hit_mask;
        if hit_mask == 0 {
            hit_mask = PROC_HIT_NORMAL_LIKE_CPP | PROC_HIT_CRITICAL_LIKE_CPP;
            if event_info.type_mask[0] & TAKEN_HIT_PROC_FLAG_MASK_LIKE_CPP == 0 {
                hit_mask |= PROC_HIT_ABSORB_LIKE_CPP;
            }
        }

        if event_info.hit_mask & hit_mask == 0 {
            return false;
        }
    }

    true
}

fn proc_flags_intersect_like_cpp(lhs: [u32; 2], rhs: [u32; 2]) -> bool {
    lhs[0] & rhs[0] != 0 || lhs[1] & rhs[1] != 0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImplicitProcAuraInfoLikeCpp {
    pub spell_type_mask: u32,
    pub triggered_can_proc: bool,
}

pub fn implicit_proc_aura_info_like_cpp(aura_type: i32) -> Option<ImplicitProcAuraInfoLikeCpp> {
    if !implicit_proc_aura_can_trigger_like_cpp(aura_type) {
        return None;
    }

    Some(ImplicitProcAuraInfoLikeCpp {
        spell_type_mask: implicit_proc_aura_spell_type_mask_like_cpp(aura_type),
        triggered_can_proc: implicit_proc_aura_is_always_triggered_like_cpp(aura_type),
    })
}

fn implicit_proc_aura_can_trigger_like_cpp(aura_type: i32) -> bool {
    matches!(
        aura_type,
        aura_types::SPELL_AURA_DUMMY
            | aura_types::SPELL_AURA_PERIODIC_DUMMY
            | aura_types::SPELL_AURA_MOD_CONFUSE
            | aura_types::SPELL_AURA_MOD_THREAT
            | aura_types::SPELL_AURA_MOD_STUN
            | aura_types::SPELL_AURA_MOD_DAMAGE_DONE
            | aura_types::SPELL_AURA_MOD_DAMAGE_TAKEN
            | aura_types::SPELL_AURA_MOD_RESISTANCE
            | aura_types::SPELL_AURA_MOD_STEALTH
            | aura_types::SPELL_AURA_MOD_FEAR
            | aura_types::SPELL_AURA_MOD_ROOT
            | aura_types::SPELL_AURA_TRANSFORM
            | aura_types::SPELL_AURA_REFLECT_SPELLS
            | aura_types::SPELL_AURA_DAMAGE_IMMUNITY
            | aura_types::SPELL_AURA_PROC_TRIGGER_SPELL
            | aura_types::SPELL_AURA_PROC_TRIGGER_DAMAGE
            | aura_types::SPELL_AURA_MOD_CASTING_SPEED_NOT_STACK
            | aura_types::SPELL_AURA_SCHOOL_ABSORB
            | aura_types::SPELL_AURA_MOD_POWER_COST_SCHOOL_PCT
            | aura_types::SPELL_AURA_MOD_POWER_COST_SCHOOL
            | aura_types::SPELL_AURA_REFLECT_SPELLS_SCHOOL
            | aura_types::SPELL_AURA_MECHANIC_IMMUNITY
            | aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN
            | aura_types::SPELL_AURA_SPELL_MAGNET
            | aura_types::SPELL_AURA_MOD_ATTACK_POWER
            | aura_types::SPELL_AURA_MOD_POWER_REGEN_PERCENT
            | aura_types::SPELL_AURA_INTERCEPT_MELEE_RANGED_ATTACKS
            | aura_types::SPELL_AURA_OVERRIDE_CLASS_SCRIPTS
            | aura_types::SPELL_AURA_MOD_MECHANIC_RESISTANCE
            | aura_types::SPELL_AURA_RANGED_ATTACK_POWER_ATTACKER_BONUS
            | aura_types::SPELL_AURA_MOD_MELEE_HASTE
            | aura_types::SPELL_AURA_MOD_MELEE_HASTE_3
            | aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE
            | aura_types::SPELL_AURA_PROC_TRIGGER_SPELL_WITH_VALUE
            | aura_types::SPELL_AURA_MOD_SCHOOL_MASK_DAMAGE_FROM_CASTER
            | aura_types::SPELL_AURA_MOD_SPELL_DAMAGE_FROM_CASTER
            | aura_types::SPELL_AURA_MOD_SPELL_CRIT_CHANCE
            | aura_types::SPELL_AURA_ABILITY_IGNORE_AURASTATE
            | aura_types::SPELL_AURA_MOD_INVISIBILITY
            | aura_types::SPELL_AURA_FORCE_REACTION
            | aura_types::SPELL_AURA_MOD_TAUNT
            | aura_types::SPELL_AURA_MOD_DETAUNT
            | aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE
            | aura_types::SPELL_AURA_MOD_ATTACK_POWER_PCT
            | aura_types::SPELL_AURA_MOD_HIT_CHANCE
            | aura_types::SPELL_AURA_MOD_WEAPON_CRIT_PERCENT
            | aura_types::SPELL_AURA_MOD_BLOCK_PERCENT
            | aura_types::SPELL_AURA_MOD_ROOT_2
            | aura_types::SPELL_AURA_IGNORE_SPELL_COOLDOWN
    )
}

fn implicit_proc_aura_is_always_triggered_like_cpp(aura_type: i32) -> bool {
    matches!(
        aura_type,
        aura_types::SPELL_AURA_OVERRIDE_CLASS_SCRIPTS
            | aura_types::SPELL_AURA_MOD_STEALTH
            | aura_types::SPELL_AURA_MOD_CONFUSE
            | aura_types::SPELL_AURA_MOD_FEAR
            | aura_types::SPELL_AURA_MOD_ROOT
            | aura_types::SPELL_AURA_MOD_STUN
            | aura_types::SPELL_AURA_TRANSFORM
            | aura_types::SPELL_AURA_MOD_INVISIBILITY
            | aura_types::SPELL_AURA_SPELL_MAGNET
            | aura_types::SPELL_AURA_SCHOOL_ABSORB
            | aura_types::SPELL_AURA_MOD_ROOT_2
    )
}

fn implicit_proc_aura_spell_type_mask_like_cpp(aura_type: i32) -> u32 {
    match aura_type {
        aura_types::SPELL_AURA_MOD_STEALTH => {
            PROC_SPELL_TYPE_DAMAGE_LIKE_CPP | PROC_SPELL_TYPE_NO_DMG_HEAL_LIKE_CPP
        }
        aura_types::SPELL_AURA_MOD_CONFUSE
        | aura_types::SPELL_AURA_MOD_FEAR
        | aura_types::SPELL_AURA_MOD_ROOT
        | aura_types::SPELL_AURA_MOD_ROOT_2
        | aura_types::SPELL_AURA_MOD_STUN
        | aura_types::SPELL_AURA_TRANSFORM
        | aura_types::SPELL_AURA_MOD_INVISIBILITY => PROC_SPELL_TYPE_DAMAGE_LIKE_CPP,
        _ => PROC_SPELL_TYPE_MASK_ALL_LIKE_CPP,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImplicitSpellProcEffectLikeCpp {
    pub effect_index: u32,
    pub is_effect: bool,
    pub is_aura: bool,
    pub aura_type: i32,
    pub spell_class_mask: [u32; 4],
    pub calc_value: i32,
    pub trigger_spell: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImplicitSpellProcSourceLikeCpp {
    pub spell_id: u32,
    pub difficulty: u32,
    pub spell_family_name: u16,
    pub proc_flags: [u32; 2],
    pub proc_chance: f32,
    pub proc_cooldown_ms: u32,
    pub proc_charges: u32,
    pub proc_base_ppm: f32,
    pub attributes3: u32,
    pub effects: Vec<ImplicitSpellProcEffectLikeCpp>,
}

pub fn implicit_spell_proc_entry_like_cpp(
    spell_info: &ImplicitSpellProcSourceLikeCpp,
) -> Option<SpellProcEntryLikeCpp> {
    if spell_info.proc_flags[0] == 0 && spell_info.proc_flags[1] == 0 {
        return None;
    }

    let mut add_trigger_flag = false;
    let mut proc_spell_type_mask = 0;
    let mut non_proc_mask = 0;

    for effect in &spell_info.effects {
        if !effect.is_effect || effect.aura_type == 0 {
            continue;
        }

        let Some(proc_aura_info) = implicit_proc_aura_info_like_cpp(effect.aura_type) else {
            non_proc_mask |= 1_u32.checked_shl(effect.effect_index).unwrap_or(0);
            continue;
        };

        proc_spell_type_mask |= proc_aura_info.spell_type_mask;
        add_trigger_flag |= proc_aura_info.triggered_can_proc;

        if !add_trigger_flag
            && spell_info.proc_flags[0] & TAKEN_HIT_PROC_FLAG_MASK_LIKE_CPP != 0
            && matches!(
                effect.aura_type,
                aura_types::SPELL_AURA_PROC_TRIGGER_SPELL
                    | aura_types::SPELL_AURA_PROC_TRIGGER_DAMAGE
            )
        {
            add_trigger_flag = true;
        }
    }

    if proc_spell_type_mask == 0 {
        return None;
    }

    let mut proc_entry = SpellProcEntryLikeCpp {
        school_mask: 0,
        spell_family_name: 0,
        spell_family_mask: [0, 0, 0, 0],
        proc_flags: spell_info.proc_flags,
        spell_type_mask: proc_spell_type_mask,
        spell_phase_mask: PROC_SPELL_PHASE_HIT_LIKE_CPP,
        hit_mask: 0,
        attributes_mask: 0,
        disable_effects_mask: non_proc_mask,
        procs_per_minute: 0.0,
        chance: spell_info.proc_chance,
        cooldown_ms: spell_info.proc_cooldown_ms,
        charges: spell_info.proc_charges,
    };

    for effect in &spell_info.effects {
        if effect.is_effect && implicit_proc_aura_info_like_cpp(effect.aura_type).is_some() {
            for (entry_mask, effect_mask) in proc_entry
                .spell_family_mask
                .iter_mut()
                .zip(effect.spell_class_mask.iter())
            {
                *entry_mask |= *effect_mask;
            }
        }
    }

    if proc_entry.spell_family_mask.iter().any(|mask| *mask != 0) {
        proc_entry.spell_family_name = spell_info.spell_family_name;
    }

    if proc_entry.proc_flags[0] & REQ_SPELL_PHASE_PROC_FLAG_MASK_LIKE_CPP == 0
        && proc_entry.proc_flags[1] & PROC_FLAG_2_CAST_SUCCESSFUL_LIKE_CPP != 0
    {
        proc_entry.spell_phase_mask = PROC_SPELL_PHASE_CAST_LIKE_CPP;
    }

    let mut triggers_spell = false;
    for effect in &spell_info.effects {
        if !effect.is_aura {
            continue;
        }

        match effect.aura_type {
            aura_types::SPELL_AURA_REFLECT_SPELLS
            | aura_types::SPELL_AURA_REFLECT_SPELLS_SCHOOL => {
                proc_entry.hit_mask = PROC_HIT_REFLECT_LIKE_CPP;
                break;
            }
            aura_types::SPELL_AURA_MOD_WEAPON_CRIT_PERCENT => {
                proc_entry.hit_mask = PROC_HIT_CRITICAL_LIKE_CPP;
                break;
            }
            aura_types::SPELL_AURA_MOD_BLOCK_PERCENT => {
                proc_entry.hit_mask = PROC_HIT_BLOCK_LIKE_CPP;
                break;
            }
            aura_types::SPELL_AURA_MOD_HIT_CHANCE => {
                if effect.calc_value <= -100 {
                    proc_entry.hit_mask = PROC_HIT_MISS_LIKE_CPP;
                }
                break;
            }
            aura_types::SPELL_AURA_PROC_TRIGGER_SPELL
            | aura_types::SPELL_AURA_PROC_TRIGGER_SPELL_WITH_VALUE => {
                triggers_spell = effect.trigger_spell != 0;
                break;
            }
            _ => {}
        }
    }

    if proc_entry.proc_flags[0] & PROC_FLAG_KILL_LIKE_CPP != 0 {
        proc_entry.attributes_mask |= PROC_ATTR_REQ_EXP_OR_HONOR_LIKE_CPP;
    }
    if add_trigger_flag {
        proc_entry.attributes_mask |= PROC_ATTR_TRIGGERED_CAN_PROC_LIKE_CPP;
    }

    if spell_info.attributes3 & attributes::SPELL_ATTR3_CAN_PROC_FROM_PROCS != 0
        && proc_entry.spell_family_mask.iter().all(|mask| *mask == 0)
        && proc_entry.chance >= 100.0
        && spell_info.proc_base_ppm <= 0.0
        && proc_entry.cooldown_ms == 0
        && proc_entry.charges == 0
        && proc_entry.proc_flags[0] & CAN_PROC_FROM_PROCS_UNRESTRICTED_DONE_FLAGS_LIKE_CPP != 0
        && triggers_spell
    {
        return None;
    }

    Some(proc_entry)
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellProcSourceSpellInfoLikeCpp {
    pub spell_id: u32,
    pub difficulty: u32,
    pub first_rank_spell_id: u32,
    pub next_rank_spell_id: Option<u32>,
    pub spell_family_name: u16,
    pub proc_flags: [u32; 2],
    pub proc_charges: u32,
    pub proc_chance: f32,
    pub proc_cooldown_ms: u32,
    pub proc_base_ppm: f32,
    pub attributes3: u32,
    pub effects: Vec<SpellEffectInfo>,
}

impl SpellProcSourceSpellInfoLikeCpp {
    pub fn from_loaded_spell_like_cpp(
        spell_id: u32,
        difficulty: u32,
        spells: &SpellStore,
        spell_chains: &SpellChainStoreLikeCpp,
        spell_aura_options: &crate::spell_db2::SpellAuraOptionsStore,
        spell_misc: &crate::spell_db2::SpellMiscStore,
        spell_class_options: &crate::spell_db2::SpellClassOptionsStore,
        spell_procs_per_minute: &crate::spell_db2::SpellProcsPerMinuteStore,
    ) -> Option<Self> {
        let spell = spells.get(i32::try_from(spell_id).ok()?)?;
        let difficulty_id = u8::try_from(difficulty).unwrap_or(0);
        let aura_options =
            spell_aura_options.entry_for_spell_difficulty_like_cpp(spell_id, difficulty_id);
        let spell_misc = spell_misc.entry_for_spell_difficulty_like_cpp(spell_id, difficulty_id);
        let spell_class_options = spell_class_options.entry_for_spell_like_cpp(spell_id);

        Some(Self {
            spell_id,
            difficulty,
            first_rank_spell_id: spell_chains.first_spell_in_chain_like_cpp(spell_id),
            next_rank_spell_id: match spell_chains.next_spell_in_chain_like_cpp(spell_id) {
                0 => None,
                next => Some(next),
            },
            spell_family_name: spell_class_options
                .map(|entry| u16::from(entry.spell_class_set))
                .unwrap_or(0),
            proc_flags: aura_options
                .map(|entry| {
                    [
                        entry.proc_type_mask[0] as u32,
                        entry.proc_type_mask[1] as u32,
                    ]
                })
                .unwrap_or([0, 0]),
            proc_charges: aura_options
                .map(|entry| entry.proc_charges as u32)
                .unwrap_or(0),
            proc_chance: aura_options
                .map(|entry| f32::from(entry.proc_chance))
                .unwrap_or(0.0),
            proc_cooldown_ms: aura_options
                .map(|entry| entry.proc_category_recovery as u32)
                .unwrap_or(0),
            proc_base_ppm: aura_options
                .and_then(|entry| {
                    spell_procs_per_minute.get(u32::from(entry.spell_procs_per_minute_id))
                })
                .map(|entry| entry.base_proc_rate)
                .unwrap_or(0.0),
            attributes3: spell_misc
                .map(|entry| entry.attributes[3] as u32)
                .unwrap_or(0),
            effects: spell.effects().to_vec(),
        })
    }

    pub fn is_ranked_like_cpp(&self) -> bool {
        self.first_rank_spell_id != self.spell_id || self.next_rank_spell_id.is_some()
    }

    pub fn implicit_proc_source_like_cpp(&self) -> ImplicitSpellProcSourceLikeCpp {
        ImplicitSpellProcSourceLikeCpp {
            spell_id: self.spell_id,
            difficulty: self.difficulty,
            spell_family_name: self.spell_family_name,
            proc_flags: self.proc_flags,
            proc_chance: self.proc_chance,
            proc_cooldown_ms: self.proc_cooldown_ms,
            proc_charges: self.proc_charges,
            proc_base_ppm: self.proc_base_ppm,
            attributes3: self.attributes3,
            effects: self
                .effects
                .iter()
                .map(|effect| ImplicitSpellProcEffectLikeCpp {
                    effect_index: effect.effect_index,
                    is_effect: effect.effect != 0,
                    is_aura: effect.is_aura_like_cpp(),
                    aura_type: effect.effect_aura,
                    spell_class_mask: effect.effect_spell_class_mask,
                    calc_value: effect.calc_value_no_caster_like_cpp(),
                    trigger_spell: u32::try_from(effect.effect_trigger_spell).unwrap_or(0),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SpellProcKeyLikeCpp {
    pub spell_id: u32,
    pub difficulty: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellProcLoadErrorKindLikeCpp {
    SpellMissing,
    AllRanksSpellNotRanked,
    AllRanksSpellNotFirstRank,
    DuplicateSpell,
    InvalidSchoolMask,
    NegativeChance,
    NegativeProcsPerMinute,
    MissingProcFlags,
    InvalidSpellTypeMask,
    SpellTypeMaskUnused,
    MissingSpellPhaseMask,
    InvalidSpellPhaseMask,
    SpellPhaseMaskUnused,
    InvalidHitMask,
    HitMaskUnused,
    DisabledEffectIsNotAura,
    ReqSpellmodWithoutSpellmodAura,
    InvalidAttributesMask,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellProcLoadErrorLikeCpp {
    pub spell_id: u32,
    pub difficulty: Option<u32>,
    pub effect_index: Option<u32>,
    pub kind: SpellProcLoadErrorKindLikeCpp,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpellProcStoreLikeCpp {
    pub proc_entries_by_spell_and_difficulty: BTreeMap<SpellProcKeyLikeCpp, SpellProcEntryLikeCpp>,
}

impl SpellProcStoreLikeCpp {
    pub async fn load_like_cpp(
        db: &WorldDatabase,
        spells: &SpellStore,
        spell_chains: &SpellChainStoreLikeCpp,
        spell_aura_options: &crate::spell_db2::SpellAuraOptionsStore,
        spell_misc: &crate::spell_db2::SpellMiscStore,
        spell_class_options: &crate::spell_db2::SpellClassOptionsStore,
        spell_procs_per_minute: &crate::spell_db2::SpellProcsPerMinuteStore,
    ) -> Result<SpellProcLoadOutcomeLikeCpp> {
        let stmt = db.prepare(WorldStatements::SEL_SPELL_PROC);
        let mut result = db.query(&stmt).await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(SpellProcRowLikeCpp {
                    spell_id: result.try_read::<i32>(0).unwrap_or(0),
                    school_mask: result.try_read::<u8>(1).unwrap_or(0),
                    spell_family_name: result.try_read::<u16>(2).unwrap_or(0),
                    spell_family_mask: [
                        result.try_read::<u32>(3).unwrap_or(0),
                        result.try_read::<u32>(4).unwrap_or(0),
                        result.try_read::<u32>(5).unwrap_or(0),
                        result.try_read::<u32>(6).unwrap_or(0),
                    ],
                    proc_flags: [
                        result.try_read::<u32>(7).unwrap_or(0),
                        result.try_read::<u32>(8).unwrap_or(0),
                    ],
                    spell_type_mask: result.try_read::<u32>(9).unwrap_or(0),
                    spell_phase_mask: result.try_read::<u32>(10).unwrap_or(0),
                    hit_mask: result.try_read::<u32>(11).unwrap_or(0),
                    attributes_mask: result.try_read::<u32>(12).unwrap_or(0),
                    disable_effects_mask: result.try_read::<u32>(13).unwrap_or(0),
                    procs_per_minute: result.try_read::<f32>(14).unwrap_or(0.0),
                    chance: result.try_read::<f32>(15).unwrap_or(0.0),
                    cooldown_ms: result.try_read::<u32>(16).unwrap_or(0),
                    charges: result.try_read::<u8>(17).unwrap_or(0),
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        let spell_infos = spells
            .iter()
            .filter_map(|spell| {
                let spell_id = u32::try_from(spell.spell_id).ok()?;
                SpellProcSourceSpellInfoLikeCpp::from_loaded_spell_like_cpp(
                    spell_id,
                    0,
                    spells,
                    spell_chains,
                    spell_aura_options,
                    spell_misc,
                    spell_class_options,
                    spell_procs_per_minute,
                )
            })
            .collect::<Vec<_>>();

        let spell_infos_by_id = spell_infos
            .iter()
            .cloned()
            .map(|spell_info| (spell_info.spell_id, spell_info))
            .collect::<BTreeMap<_, _>>();

        Ok(Self::from_rows_and_spell_infos_like_cpp(
            rows,
            |spell_id| spell_infos_by_id.get(&spell_id).cloned(),
            spell_infos,
        ))
    }

    pub fn from_rows_like_cpp<I, SpellInfoById>(
        rows: I,
        mut spell_info_by_id: SpellInfoById,
    ) -> SpellProcLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellProcRowLikeCpp>,
        SpellInfoById: FnMut(u32) -> Option<SpellProcSourceSpellInfoLikeCpp>,
    {
        let mut store = Self::default();
        let mut errors = Vec::new();
        let mut loaded_row_count = 0;

        for row in rows {
            let all_ranks = row.spell_id < 0;
            let spell_id = row.spell_id.unsigned_abs();
            let Some(mut spell_info) = spell_info_by_id(spell_id) else {
                errors.push(SpellProcLoadErrorLikeCpp {
                    spell_id,
                    difficulty: None,
                    effect_index: None,
                    kind: SpellProcLoadErrorKindLikeCpp::SpellMissing,
                });
                continue;
            };

            if all_ranks {
                if !spell_info.is_ranked_like_cpp() {
                    errors.push(SpellProcLoadErrorLikeCpp {
                        spell_id,
                        difficulty: Some(spell_info.difficulty),
                        effect_index: None,
                        kind: SpellProcLoadErrorKindLikeCpp::AllRanksSpellNotRanked,
                    });
                }

                if spell_info.first_rank_spell_id != spell_id {
                    errors.push(SpellProcLoadErrorLikeCpp {
                        spell_id,
                        difficulty: Some(spell_info.difficulty),
                        effect_index: None,
                        kind: SpellProcLoadErrorKindLikeCpp::AllRanksSpellNotFirstRank,
                    });
                    continue;
                }
            }

            loop {
                let key = SpellProcKeyLikeCpp {
                    spell_id: spell_info.spell_id,
                    difficulty: spell_info.difficulty,
                };

                if store
                    .proc_entries_by_spell_and_difficulty
                    .contains_key(&key)
                {
                    errors.push(SpellProcLoadErrorLikeCpp {
                        spell_id: spell_info.spell_id,
                        difficulty: Some(spell_info.difficulty),
                        effect_index: None,
                        kind: SpellProcLoadErrorKindLikeCpp::DuplicateSpell,
                    });
                    break;
                }

                let mut entry = SpellProcEntryLikeCpp::from_row_like_cpp(&row);
                apply_spell_proc_defaults_like_cpp(&mut entry, &spell_info);
                validate_spell_proc_entry_like_cpp(&mut entry, &spell_info, &mut errors);
                store
                    .proc_entries_by_spell_and_difficulty
                    .insert(key, entry);

                if !all_ranks {
                    break;
                }

                let Some(next_rank_spell_id) = spell_info.next_rank_spell_id else {
                    break;
                };
                let Some(next_spell_info) = spell_info_by_id(next_rank_spell_id) else {
                    break;
                };
                spell_info = next_spell_info;
            }

            loaded_row_count += 1;
        }

        SpellProcLoadOutcomeLikeCpp {
            store,
            loaded_row_count,
            generated_entry_count: 0,
            errors,
        }
    }

    pub fn from_rows_and_implicit_sources_like_cpp<I, SpellInfoById, ImplicitSources>(
        rows: I,
        spell_info_by_id: SpellInfoById,
        implicit_sources: ImplicitSources,
    ) -> SpellProcLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellProcRowLikeCpp>,
        SpellInfoById: FnMut(u32) -> Option<SpellProcSourceSpellInfoLikeCpp>,
        ImplicitSources: IntoIterator<Item = ImplicitSpellProcSourceLikeCpp>,
    {
        let mut outcome = Self::from_rows_like_cpp(rows, spell_info_by_id);

        for source in implicit_sources {
            let key = SpellProcKeyLikeCpp {
                spell_id: source.spell_id,
                difficulty: source.difficulty,
            };

            if outcome
                .store
                .proc_entries_by_spell_and_difficulty
                .contains_key(&key)
            {
                continue;
            }

            let Some(entry) = implicit_spell_proc_entry_like_cpp(&source) else {
                continue;
            };

            outcome
                .store
                .proc_entries_by_spell_and_difficulty
                .insert(key, entry);
            outcome.generated_entry_count += 1;
        }

        outcome
    }

    pub fn from_rows_and_spell_infos_like_cpp<I, SpellInfoById, SpellInfos>(
        rows: I,
        spell_info_by_id: SpellInfoById,
        spell_infos: SpellInfos,
    ) -> SpellProcLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellProcRowLikeCpp>,
        SpellInfoById: FnMut(u32) -> Option<SpellProcSourceSpellInfoLikeCpp>,
        SpellInfos: IntoIterator<Item = SpellProcSourceSpellInfoLikeCpp>,
    {
        Self::from_rows_and_implicit_sources_like_cpp(
            rows,
            spell_info_by_id,
            spell_infos
                .into_iter()
                .map(|spell_info| spell_info.implicit_proc_source_like_cpp()),
        )
    }

    pub fn spell_proc_entry_like_cpp(
        &self,
        spell_id: u32,
        difficulty: u32,
    ) -> Option<&SpellProcEntryLikeCpp> {
        self.proc_entries_by_spell_and_difficulty
            .get(&SpellProcKeyLikeCpp {
                spell_id,
                difficulty,
            })
    }

    pub fn spell_proc_entry_with_fallback_like_cpp<FallbackDifficulty>(
        &self,
        spell_id: u32,
        difficulty: u32,
        mut fallback_difficulty: FallbackDifficulty,
    ) -> Option<&SpellProcEntryLikeCpp>
    where
        FallbackDifficulty: FnMut(u32) -> Option<u32>,
    {
        if let Some(entry) = self.spell_proc_entry_like_cpp(spell_id, difficulty) {
            return Some(entry);
        }

        let mut current_difficulty = difficulty;
        while let Some(next_difficulty) = fallback_difficulty(current_difficulty) {
            if let Some(entry) = self.spell_proc_entry_like_cpp(spell_id, next_difficulty) {
                return Some(entry);
            }
            current_difficulty = next_difficulty;
        }

        None
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellProcLoadOutcomeLikeCpp {
    pub store: SpellProcStoreLikeCpp,
    pub loaded_row_count: usize,
    pub generated_entry_count: usize,
    pub errors: Vec<SpellProcLoadErrorLikeCpp>,
}

fn apply_spell_proc_defaults_like_cpp(
    entry: &mut SpellProcEntryLikeCpp,
    spell_info: &SpellProcSourceSpellInfoLikeCpp,
) {
    if !entry.proc_flags_any_like_cpp() {
        entry.proc_flags = spell_info.proc_flags;
    }
    if entry.charges == 0 {
        entry.charges = spell_info.proc_charges;
    }
    if entry.chance == 0.0 && entry.procs_per_minute == 0.0 {
        entry.chance = spell_info.proc_chance;
    }
    if entry.cooldown_ms == 0 {
        entry.cooldown_ms = spell_info.proc_cooldown_ms;
    }
}

fn validate_spell_proc_entry_like_cpp(
    entry: &mut SpellProcEntryLikeCpp,
    spell_info: &SpellProcSourceSpellInfoLikeCpp,
    errors: &mut Vec<SpellProcLoadErrorLikeCpp>,
) {
    let mut push_error = |kind, effect_index| {
        errors.push(SpellProcLoadErrorLikeCpp {
            spell_id: spell_info.spell_id,
            difficulty: Some(spell_info.difficulty),
            effect_index,
            kind,
        });
    };

    if entry.school_mask & !SPELL_SCHOOL_MASK_ALL_LIKE_CPP != 0 {
        push_error(SpellProcLoadErrorKindLikeCpp::InvalidSchoolMask, None);
    }
    if entry.chance < 0.0 {
        push_error(SpellProcLoadErrorKindLikeCpp::NegativeChance, None);
        entry.chance = 0.0;
    }
    if entry.procs_per_minute < 0.0 {
        push_error(SpellProcLoadErrorKindLikeCpp::NegativeProcsPerMinute, None);
        entry.procs_per_minute = 0.0;
    }
    if !entry.proc_flags_any_like_cpp() {
        push_error(SpellProcLoadErrorKindLikeCpp::MissingProcFlags, None);
    }
    if entry.spell_type_mask & !PROC_SPELL_TYPE_MASK_ALL_LIKE_CPP != 0 {
        push_error(SpellProcLoadErrorKindLikeCpp::InvalidSpellTypeMask, None);
    }
    if entry.spell_type_mask != 0 && entry.proc_flags[0] & SPELL_PROC_FLAG_MASK_LIKE_CPP == 0 {
        push_error(SpellProcLoadErrorKindLikeCpp::SpellTypeMaskUnused, None);
    }
    if entry.spell_phase_mask == 0
        && entry.proc_flags[0] & REQ_SPELL_PHASE_PROC_FLAG_MASK_LIKE_CPP != 0
    {
        push_error(SpellProcLoadErrorKindLikeCpp::MissingSpellPhaseMask, None);
    }
    if entry.spell_phase_mask & !PROC_SPELL_PHASE_MASK_ALL_LIKE_CPP != 0 {
        push_error(SpellProcLoadErrorKindLikeCpp::InvalidSpellPhaseMask, None);
    }
    if entry.spell_phase_mask != 0
        && entry.proc_flags[0] & REQ_SPELL_PHASE_PROC_FLAG_MASK_LIKE_CPP == 0
    {
        push_error(SpellProcLoadErrorKindLikeCpp::SpellPhaseMaskUnused, None);
    }
    if entry.spell_phase_mask == 0
        && entry.proc_flags[0] & REQ_SPELL_PHASE_PROC_FLAG_MASK_LIKE_CPP == 0
        && entry.proc_flags[1] & PROC_FLAG_2_CAST_SUCCESSFUL_LIKE_CPP != 0
    {
        entry.spell_phase_mask = PROC_SPELL_PHASE_CAST_LIKE_CPP;
    }
    if entry.hit_mask & !PROC_HIT_MASK_ALL_LIKE_CPP != 0 {
        push_error(SpellProcLoadErrorKindLikeCpp::InvalidHitMask, None);
    }
    if entry.hit_mask != 0
        && !(entry.proc_flags[0] & TAKEN_HIT_PROC_FLAG_MASK_LIKE_CPP != 0
            || (entry.proc_flags[0] & DONE_HIT_PROC_FLAG_MASK_LIKE_CPP != 0
                && (entry.spell_phase_mask == 0
                    || entry.spell_phase_mask
                        & (PROC_SPELL_PHASE_HIT_LIKE_CPP | PROC_SPELL_PHASE_FINISH_LIKE_CPP)
                        != 0)))
    {
        push_error(SpellProcLoadErrorKindLikeCpp::HitMaskUnused, None);
    }

    for effect in &spell_info.effects {
        if (entry.disable_effects_mask & (1u32 << effect.effect_index)) != 0
            && !effect.is_aura_like_cpp()
        {
            push_error(
                SpellProcLoadErrorKindLikeCpp::DisabledEffectIsNotAura,
                Some(effect.effect_index),
            );
        }
    }

    if entry.attributes_mask & PROC_ATTR_REQ_SPELLMOD_LIKE_CPP != 0
        && !spell_info.effects.iter().any(|effect| {
            effect.is_aura_like_cpp()
                && matches!(
                    effect.effect_aura,
                    aura_types::SPELL_AURA_ADD_PCT_MODIFIER
                        | aura_types::SPELL_AURA_ADD_FLAT_MODIFIER
                        | aura_types::SPELL_AURA_ADD_PCT_MODIFIER_BY_SPELL_LABEL
                        | aura_types::SPELL_AURA_IGNORE_SPELL_COOLDOWN
                )
        })
    {
        push_error(
            SpellProcLoadErrorKindLikeCpp::ReqSpellmodWithoutSpellmodAura,
            None,
        );
    }

    if entry.attributes_mask & !PROC_ATTR_ALL_ALLOWED_LIKE_CPP != 0 {
        push_error(SpellProcLoadErrorKindLikeCpp::InvalidAttributesMask, None);
        entry.attributes_mask &= PROC_ATTR_ALL_ALLOWED_LIKE_CPP;
    }
}

fn infer_same_effect_stack_aura_types_like_cpp<SpellInfoById>(
    spell_ids: &BTreeSet<u32>,
    spell_info_by_id: &mut SpellInfoById,
) -> BTreeSet<i32>
where
    SpellInfoById: FnMut(u32) -> Option<SpellInfo>,
{
    let mut frequency = BTreeMap::<i32, usize>::new();
    let mut aura_order = Vec::<i32>::new();

    for spell_id in spell_ids {
        if let Some(spell_info) = spell_info_by_id(*spell_id) {
            for effect in spell_info.effects() {
                if !effect.is_aura_like_cpp() {
                    continue;
                }

                let aura_type = normalize_same_effect_subgroup_aura_like_cpp(effect.effect_aura);
                if !frequency.contains_key(&aura_type) {
                    aura_order.push(aura_type);
                }
                *frequency.entry(aura_type).or_default() += 1;
            }
        }
    }

    let mut selected_aura_type = 0;
    let mut selected_count = 0;
    for aura_type in aura_order {
        let current_count = frequency.get(&aura_type).copied().unwrap_or(0);
        if current_count > selected_count {
            selected_aura_type = aura_type;
            selected_count = current_count;
        }
    }

    if selected_aura_type == aura_types::SPELL_AURA_MOD_MELEE_HASTE {
        BTreeSet::from([
            aura_types::SPELL_AURA_MOD_MELEE_HASTE,
            aura_types::SPELL_AURA_MOD_MELEE_RANGED_HASTE,
            aura_types::SPELL_AURA_MOD_RANGED_HASTE,
        ])
    } else {
        BTreeSet::from([selected_aura_type])
    }
}

fn normalize_same_effect_subgroup_aura_like_cpp(aura_type: i32) -> i32 {
    if matches!(
        aura_type,
        aura_types::SPELL_AURA_MOD_MELEE_HASTE
            | aura_types::SPELL_AURA_MOD_MELEE_RANGED_HASTE
            | aura_types::SPELL_AURA_MOD_RANGED_HASTE
    ) {
        aura_types::SPELL_AURA_MOD_MELEE_HASTE
    } else {
        aura_type
    }
}

fn spell_rank_chain_has_any_aura_like_cpp<SpellInfoById, NextRankSpell>(
    spell_id: u32,
    aura_types: &BTreeSet<i32>,
    spell_info_by_id: &mut SpellInfoById,
    next_rank_spell: &mut NextRankSpell,
) -> bool
where
    SpellInfoById: FnMut(u32) -> Option<SpellInfo>,
    NextRankSpell: FnMut(u32) -> Option<u32>,
{
    let mut current_spell_id = Some(spell_id);
    let mut seen = BTreeSet::new();

    while let Some(spell_id) = current_spell_id {
        if !seen.insert(spell_id) {
            break;
        }

        let Some(spell_info) = spell_info_by_id(spell_id) else {
            return false;
        };

        if aura_types
            .iter()
            .any(|aura_type| spell_info.has_aura_like_cpp(*aura_type))
        {
            return true;
        }

        current_spell_id = next_rank_spell(spell_id);
    }

    false
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellLearnSpellSqlRowLikeCpp {
    pub entry: u32,
    pub spell_id: u32,
    pub active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellLearnSpellNodeLikeCpp {
    pub spell: u32,
    pub overrides_spell: u32,
    pub active: bool,
    pub auto_learned: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellLearnSpellEffectLikeCpp {
    pub trigger_spell: u32,
    pub target_unit_pet: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellLearnSourceSpellInfoLikeCpp {
    pub spell_id: u32,
    pub difficulty_none: bool,
    pub is_talent: bool,
    pub is_passive: bool,
    pub has_skill_step_effect: bool,
    pub learn_spell_effects: Vec<SpellLearnSpellEffectLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellLearnSpellLoadErrorKindLikeCpp {
    SqlSourceSpellMissing,
    SqlLearnedSpellMissing,
    SqlSourceIsTalent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellLearnSpellLoadErrorLikeCpp {
    pub row: SpellLearnSpellSqlRowLikeCpp,
    pub kind: SpellLearnSpellLoadErrorKindLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellLearnSpellLoadWarningKindLikeCpp {
    RedundantSqlRowForSpellEffect {
        source_spell: u32,
        learned_spell: u32,
    },
    RedundantSqlRowForDb2 {
        source_spell: u32,
        learned_spell: i32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellLearnSpellLoadWarningLikeCpp {
    pub kind: SpellLearnSpellLoadWarningKindLikeCpp,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellLearnSpellStoreLikeCpp {
    pub learned_by_spell_id: BTreeMap<u32, Vec<SpellLearnSpellNodeLikeCpp>>,
}

impl SpellLearnSpellStoreLikeCpp {
    pub fn from_sources_like_cpp<SqlRows, SourceSpells, Db2Rows, SpellLookup, SpellExists>(
        sql_rows: SqlRows,
        source_spells: SourceSpells,
        db2_rows: Db2Rows,
        mut spell_lookup: SpellLookup,
        mut spell_exists: SpellExists,
    ) -> SpellLearnSpellLoadOutcomeLikeCpp
    where
        SqlRows: IntoIterator<Item = SpellLearnSpellSqlRowLikeCpp>,
        SourceSpells: IntoIterator<Item = SpellLearnSourceSpellInfoLikeCpp>,
        Db2Rows: IntoIterator<Item = crate::spell_db2::SpellLearnSpellEntry>,
        SpellLookup: FnMut(u32) -> Option<SpellLearnSourceSpellInfoLikeCpp>,
        SpellExists: FnMut(u32) -> bool,
    {
        let mut store = Self::default();
        let mut sql_loaded_row_count = 0;
        let mut dbc_loaded_row_count = 0;
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let sql_rows = sql_rows.into_iter().collect::<Vec<_>>();

        if sql_rows.is_empty() {
            return SpellLearnSpellLoadOutcomeLikeCpp {
                store,
                sql_loaded_row_count,
                dbc_loaded_row_count,
                sql_result_empty: true,
                errors,
                warnings,
            };
        }

        for row in sql_rows {
            let Some(source_spell) = spell_lookup(row.entry) else {
                errors.push(SpellLearnSpellLoadErrorLikeCpp {
                    row,
                    kind: SpellLearnSpellLoadErrorKindLikeCpp::SqlSourceSpellMissing,
                });
                continue;
            };

            if !spell_exists(row.spell_id) {
                errors.push(SpellLearnSpellLoadErrorLikeCpp {
                    row,
                    kind: SpellLearnSpellLoadErrorKindLikeCpp::SqlLearnedSpellMissing,
                });
                continue;
            }

            if source_spell.is_talent {
                errors.push(SpellLearnSpellLoadErrorLikeCpp {
                    row,
                    kind: SpellLearnSpellLoadErrorKindLikeCpp::SqlSourceIsTalent,
                });
                continue;
            }

            store
                .learned_by_spell_id
                .entry(row.entry)
                .or_default()
                .push(SpellLearnSpellNodeLikeCpp {
                    spell: row.spell_id,
                    overrides_spell: 0,
                    active: row.active,
                    auto_learned: false,
                });
            sql_loaded_row_count += 1;
        }

        let db_spell_learn_spells = store.learned_by_spell_id.clone();

        for source_spell in source_spells {
            if !source_spell.difficulty_none {
                continue;
            }

            for effect in source_spell.learn_spell_effects {
                let dbc_node = SpellLearnSpellNodeLikeCpp {
                    spell: effect.trigger_spell,
                    overrides_spell: 0,
                    active: true,
                    auto_learned: effect.target_unit_pet
                        || source_spell.is_talent
                        || source_spell.is_passive
                        || source_spell.has_skill_step_effect,
                };

                if !spell_exists(dbc_node.spell) {
                    continue;
                }

                if Self::contains_learn_pair_in_map(
                    &db_spell_learn_spells,
                    source_spell.spell_id,
                    dbc_node.spell,
                ) {
                    warnings.push(SpellLearnSpellLoadWarningLikeCpp {
                        kind:
                            SpellLearnSpellLoadWarningKindLikeCpp::RedundantSqlRowForSpellEffect {
                                source_spell: source_spell.spell_id,
                                learned_spell: dbc_node.spell,
                            },
                    });
                    continue;
                }

                store
                    .learned_by_spell_id
                    .entry(source_spell.spell_id)
                    .or_default()
                    .push(dbc_node);
                dbc_loaded_row_count += 1;
            }
        }

        for db2_row in db2_rows {
            let source_spell = db2_row.spell_id as u32;
            let learned_spell = db2_row.learn_spell_id as u32;

            if !spell_exists(source_spell) || !spell_exists(learned_spell) {
                continue;
            }

            if db_spell_learn_spells
                .get(&source_spell)
                .is_some_and(|nodes| {
                    nodes
                        .iter()
                        .any(|node| node.spell as i32 == db2_row.learn_spell_id)
                })
            {
                warnings.push(SpellLearnSpellLoadWarningLikeCpp {
                    kind: SpellLearnSpellLoadWarningKindLikeCpp::RedundantSqlRowForDb2 {
                        source_spell,
                        learned_spell: db2_row.learn_spell_id,
                    },
                });
                continue;
            }

            if Self::contains_learn_pair_in_map(
                &store.learned_by_spell_id,
                source_spell,
                learned_spell,
            ) {
                continue;
            }

            store
                .learned_by_spell_id
                .entry(source_spell)
                .or_default()
                .push(SpellLearnSpellNodeLikeCpp {
                    spell: learned_spell,
                    overrides_spell: db2_row.overrides_spell_id as u32,
                    active: true,
                    auto_learned: false,
                });
            dbc_loaded_row_count += 1;
        }

        SpellLearnSpellLoadOutcomeLikeCpp {
            store,
            sql_loaded_row_count,
            dbc_loaded_row_count,
            sql_result_empty: false,
            errors,
            warnings,
        }
    }

    fn contains_learn_pair_in_map(
        map: &BTreeMap<u32, Vec<SpellLearnSpellNodeLikeCpp>>,
        source_spell: u32,
        learned_spell: u32,
    ) -> bool {
        map.get(&source_spell)
            .is_some_and(|nodes| nodes.iter().any(|node| node.spell == learned_spell))
    }

    pub fn get_spell_learn_spell_map_bounds_like_cpp(
        &self,
        spell_id: u32,
    ) -> &[SpellLearnSpellNodeLikeCpp] {
        self.learned_by_spell_id
            .get(&spell_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn is_spell_learn_spell_like_cpp(&self, spell_id: u32) -> bool {
        self.learned_by_spell_id.contains_key(&spell_id)
    }

    pub fn is_spell_learn_to_spell_like_cpp(&self, spell_id1: u32, spell_id2: u32) -> bool {
        self.get_spell_learn_spell_map_bounds_like_cpp(spell_id1)
            .iter()
            .any(|node| node.spell == spell_id2)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellLearnSpellLoadOutcomeLikeCpp {
    pub store: SpellLearnSpellStoreLikeCpp,
    pub sql_loaded_row_count: usize,
    pub dbc_loaded_row_count: usize,
    pub sql_result_empty: bool,
    pub errors: Vec<SpellLearnSpellLoadErrorLikeCpp>,
    pub warnings: Vec<SpellLearnSpellLoadWarningLikeCpp>,
}

impl SpellInfo {
    /// Convenience: returns the effective cooldown (per-spell or global, whichever is larger).
    pub fn effective_cooldown_ms(&self) -> u32 {
        self.recovery_time_ms.max(self.cooldown_ms)
    }

    /// Returns true if this spell has a cast time (not instant).
    pub fn has_cast_time(&self) -> bool {
        self.cast_time_ms > 0
    }

    pub fn effects(&self) -> &[SpellEffectInfo] {
        &self.effects
    }

    pub fn has_aura_like_cpp(&self, aura_type: i32) -> bool {
        self.effects
            .iter()
            .any(|effect| effect.effect_aura == aura_type)
    }

    pub fn requires_spell_focus_like_cpp(&self) -> bool {
        self.requires_spell_focus != 0
    }

    pub fn normalized_implicit_target_effect_mask_like_cpp(&self, mut effect_mask: u32) -> u32 {
        let original_mask = effect_mask;
        for effect in &self.effects {
            let bit = 1u32.checked_shl(effect.effect_index).unwrap_or(0);
            if bit == 0 || (original_mask & bit) == 0 {
                continue;
            }

            if !effect.accepts_implicit_target_conditions_like_cpp() {
                effect_mask &= !bit;
            }
        }
        effect_mask
    }
}

impl SpellEffectInfo {
    pub fn is_aura_like_cpp(&self) -> bool {
        use spell_effect_types::*;
        matches!(
            self.effect,
            SPELL_EFFECT_APPLY_AURA
                | SPELL_EFFECT_APPLY_AREA_AURA_PARTY
                | SPELL_EFFECT_APPLY_AREA_AURA_RAID
                | SPELL_EFFECT_APPLY_AREA_AURA_FRIEND
                | SPELL_EFFECT_APPLY_AREA_AURA_ENEMY
                | SPELL_EFFECT_APPLY_AREA_AURA_PET
                | SPELL_EFFECT_APPLY_AREA_AURA_OWNER
                | SPELL_EFFECT_APPLY_AURA_ON_PET
                | SPELL_EFFECT_APPLY_AREA_AURA_SUMMONS
                | SPELL_EFFECT_APPLY_AREA_AURA_PARTY_NONRANDOM
        )
    }

    pub fn calc_value_no_caster_with_die_roll_like_cpp<F>(&self, mut roll_die: F) -> i32
    where
        F: FnMut(i32, i32) -> i32,
    {
        let mut value = self.effect_base_points;
        match self.effect_die_sides {
            0 => {}
            1 => value += 1,
            die_sides if die_sides > 1 => value += roll_die(1, die_sides),
            die_sides => value += roll_die(die_sides, 1),
        }
        value
    }

    pub fn calc_value_no_caster_like_cpp(&self) -> i32 {
        use rand::Rng;

        self.calc_value_no_caster_with_die_roll_like_cpp(|min, max| {
            rand::thread_rng().gen_range(min..=max)
        })
    }

    pub fn is_mounted_aura_like_cpp(&self) -> bool {
        self.effect == spell_effect_types::SPELL_EFFECT_APPLY_AURA
            && self.effect_aura == aura_types::SPELL_AURA_MOUNTED
    }

    pub fn is_provide_spell_focus_aura_like_cpp(&self) -> bool {
        self.effect == spell_effect_types::SPELL_EFFECT_APPLY_AURA
            && self.effect_aura == aura_types::SPELL_AURA_PROVIDE_SPELL_FOCUS
    }

    pub fn is_battle_pet_xp_pct_aura_like_cpp(&self) -> bool {
        self.effect == spell_effect_types::SPELL_EFFECT_APPLY_AURA
            && self.effect_aura == aura_types::SPELL_AURA_MOD_BATTLE_PET_XP_PCT
    }

    pub fn has_focus_destination_implicit_target_like_cpp(&self) -> bool {
        matches!(
            self.implicit_target_1,
            implicit_targets::TARGET_DEST_NEARBY_ENTRY
                | implicit_targets::TARGET_DEST_NEARBY_ENTRY_2
                | implicit_targets::TARGET_DEST_NEARBY_ENTRY_OR_DB
        ) || matches!(
            self.implicit_target_2,
            implicit_targets::TARGET_DEST_NEARBY_ENTRY
                | implicit_targets::TARGET_DEST_NEARBY_ENTRY_2
                | implicit_targets::TARGET_DEST_NEARBY_ENTRY_OR_DB
        )
    }

    pub fn accepts_implicit_target_conditions_like_cpp(&self) -> bool {
        self.chain_targets > 0
            || implicit_target_category_accepts_conditions_like_cpp(self.implicit_target_1)
            || implicit_target_category_accepts_conditions_like_cpp(self.implicit_target_2)
            || spell_effect_accepts_implicit_target_conditions_like_cpp(self.effect)
    }

    pub fn has_spell_target_position_target_like_cpp(&self) -> bool {
        matches!(
            self.implicit_target_1,
            implicit_targets::TARGET_DEST_DB | implicit_targets::TARGET_DEST_NEARBY_ENTRY_OR_DB
        ) || matches!(
            self.implicit_target_2,
            implicit_targets::TARGET_DEST_DB | implicit_targets::TARGET_DEST_NEARBY_ENTRY_OR_DB
        )
    }
}

impl SpellTargetPositionStoreLikeCpp {
    pub fn from_rows_like_cpp(
        rows: impl IntoIterator<Item = SpellTargetPositionRowLikeCpp>,
        spells: &SpellStore,
        mut map_exists: impl FnMut(u16) -> bool,
    ) -> Self {
        let mut store = Self::default();

        for row in rows {
            if !map_exists(row.target_map_id) {
                store.load_report.skipped_missing_map += 1;
                continue;
            }

            if row.x == 0.0 && row.y == 0.0 && row.z == 0.0 {
                store.load_report.skipped_zero_position += 1;
                continue;
            }

            let Some(spell) = spells.get(row.spell_id as i32) else {
                store.load_report.skipped_missing_spell += 1;
                continue;
            };
            let Some(effect) = spell
                .effects()
                .iter()
                .find(|effect| effect.effect_index == row.effect_index)
            else {
                store.load_report.skipped_missing_effect += 1;
                continue;
            };

            if !effect.has_spell_target_position_target_like_cpp() {
                store.load_report.skipped_unsupported_target += 1;
                continue;
            }

            let orientation = row.orientation.unwrap_or_else(|| {
                if effect.position_facing > TAU {
                    effect.position_facing * std::f32::consts::PI / 180.0
                } else {
                    effect.position_facing
                }
            });

            store.positions.insert(
                (row.spell_id, row.effect_index),
                SpellTargetPositionLikeCpp {
                    target_map_id: row.target_map_id,
                    position: wow_core::Position::new(row.x, row.y, row.z, orientation),
                },
            );
            store.load_report.loaded += 1;
        }

        store
    }

    pub async fn load_like_cpp(
        db: &WorldDatabase,
        spells: &SpellStore,
        map_exists: impl FnMut(u16) -> bool,
    ) -> Result<Self> {
        let mut result = db
            .direct_query(wow_database::WorldStatements::SEL_SPELL_TARGET_POSITION.sql())
            .await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(SpellTargetPositionRowLikeCpp {
                    spell_id: result.try_read::<u32>(0).unwrap_or(0),
                    effect_index: result.try_read::<u8>(1).unwrap_or(0) as u32,
                    target_map_id: result.try_read::<u16>(2).unwrap_or(0),
                    x: result.try_read::<f32>(3).unwrap_or(0.0),
                    y: result.try_read::<f32>(4).unwrap_or(0.0),
                    z: result.try_read::<f32>(5).unwrap_or(0.0),
                    orientation: result.try_read::<Option<f32>>(6).unwrap_or(None),
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_rows_like_cpp(rows, spells, map_exists))
    }

    pub fn get(&self, spell_id: u32, effect_index: u32) -> Option<&SpellTargetPositionLikeCpp> {
        self.positions.get(&(spell_id, effect_index))
    }

    pub fn len(&self) -> usize {
        self.positions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    pub fn load_report_like_cpp(&self) -> &SpellTargetPositionLoadReportLikeCpp {
        &self.load_report
    }
}

const fn spell_effect_accepts_implicit_target_conditions_like_cpp(effect: u32) -> bool {
    use spell_effect_types::*;
    matches!(
        effect,
        SPELL_EFFECT_PERSISTENT_AREA_AURA
            | SPELL_EFFECT_APPLY_AREA_AURA_PARTY
            | SPELL_EFFECT_APPLY_AREA_AURA_RAID
            | SPELL_EFFECT_APPLY_AREA_AURA_FRIEND
            | SPELL_EFFECT_APPLY_AREA_AURA_ENEMY
            | SPELL_EFFECT_APPLY_AREA_AURA_PET
            | SPELL_EFFECT_APPLY_AREA_AURA_OWNER
            | SPELL_EFFECT_APPLY_AURA_ON_PET
            | SPELL_EFFECT_APPLY_AREA_AURA_SUMMONS
            | SPELL_EFFECT_APPLY_AREA_AURA_PARTY_NONRANDOM
    )
}

const fn implicit_target_category_accepts_conditions_like_cpp(target: u32) -> bool {
    matches!(
        target,
        2 | 3
            | 4
            | 7
            | 8
            | 15
            | 16
            | 20
            | 24
            | 30
            | 31
            | 33
            | 34
            | 37
            | 38
            | 40
            | 46
            | 51
            | 52
            | 54
            | 56
            | 58
            | 59
            | 60
            | 61
            | 89
            | 93
            | 104
            | 105
            | 107
            | 108
            | 109
            | 110
            | 115
            | 116
            | 118
            | 119
            | 120
            | 122
            | 123
            | 128
            | 129
            | 130
            | 133
            | 134
            | 135
            | 136
            | 142
            | 151
    )
}

/// In-memory store of all spells loaded from DB2 or hotfixes database.
#[derive(Default)]
pub struct SpellStore {
    spells: HashMap<i32, SpellInfo>,
    implicit_target_conditions: HashMap<(i32, u32), ConditionsReference>,
}

impl SpellStore {
    /// Create a new empty spell store.
    pub fn new() -> Self {
        Self {
            spells: HashMap::new(),
            implicit_target_conditions: HashMap::new(),
        }
    }

    /// Load spell data from hotfixes database.
    ///
    /// Queries `hotfixes.spell_misc` (cast time, cooldowns) and
    /// `hotfixes.spell_effect` (effect type, damage/healing parameters).
    ///
    /// # Arguments
    ///
    /// * `db` - HotfixDatabase connection pool
    ///
    /// # Returns
    ///
    /// A populated SpellStore on success, or a database error on failure.
    pub async fn load(db: &HotfixDatabase) -> Result<Self> {
        let mut store = Self::new();

        // Query spell_misc and spell_effect from hotfixes database
        // NOTE: Phase 1 — cast_time_ms and cooldown_ms are hardcoded to 0 (instant).
        // Phase 2+ will load from SpellCastTimes.dbc and SpellDuration.dbc using
        // CastingTimeIndex and DurationIndex respectively.
        let sql = r#"
SELECT 
    CAST(sm.ID AS SIGNED) as spell_id,
    CAST(0 AS UNSIGNED) as cast_time_ms,
    CAST(0 AS UNSIGNED) as cooldown_ms,
    CAST(0 AS UNSIGNED) as recovery_time_ms,
    CAST(COALESCE(se.Effect, 0) AS UNSIGNED) as effect_type,
    CAST(COALESCE(se.EffectBasePoints, 0) AS SIGNED) as effect_base_points,
    CAST(COALESCE(se.EffectBonusCoefficient, 0.0) AS DECIMAL(10,2)) as effect_bonus_coeff,
    CAST(COALESCE(se.EffectAura, 0) AS SIGNED) as effect_aura,
    CAST(COALESCE(se.EffectMiscValue1, 0) AS SIGNED) as effect_misc_value_1,
    CAST(COALESCE(se.EffectMiscValue2, 0) AS SIGNED) as effect_misc_value_2,
    CAST(COALESCE(se.EffectTriggerSpell, 0) AS SIGNED) as effect_trigger_spell,
    CAST(COALESCE(se.EffectRadiusIndex1, 0) AS UNSIGNED) as effect_radius_index_1,
    CAST(COALESCE(se.EffectPosFacing, 0.0) AS DECIMAL(10,4)) as position_facing,
    CAST(COALESCE(se.EffectIndex, 0) AS UNSIGNED) as effect_index,
    CAST(COALESCE(se.EffectChainTargets, 0) AS SIGNED) as effect_chain_targets,
    CAST(COALESCE(se.ImplicitTarget1, 0) AS UNSIGNED) as implicit_target_1,
    CAST(COALESCE(se.ImplicitTarget2, 0) AS UNSIGNED) as implicit_target_2,
    CAST(COALESCE(scr.RequiresSpellFocus, 0) AS UNSIGNED) as requires_spell_focus,
    CAST(COALESCE(se.EffectSpellClassMask1, 0) AS UNSIGNED) as effect_spell_class_mask_1,
    CAST(COALESCE(se.EffectSpellClassMask2, 0) AS UNSIGNED) as effect_spell_class_mask_2,
    CAST(COALESCE(se.EffectSpellClassMask3, 0) AS UNSIGNED) as effect_spell_class_mask_3,
    CAST(COALESCE(se.EffectSpellClassMask4, 0) AS UNSIGNED) as effect_spell_class_mask_4,
    CAST(COALESCE(se.EffectDieSides, 0) AS SIGNED) as effect_die_sides
FROM hotfixes.spell_misc sm
LEFT JOIN hotfixes.spell_effect se 
    ON sm.ID = se.SpellID AND se.DifficultyID = 0
LEFT JOIN hotfixes.spell_casting_requirements scr
    ON sm.ID = scr.SpellID AND scr.DifficultyID = 0
ORDER BY sm.ID, se.EffectIndex
        "#;

        let mut result = db.direct_query(sql).await?;

        if !result.is_empty() {
            loop {
                let spell_id: i32 = result.read(0);
                let cast_time_ms: u32 = result.read(1);
                let cooldown_ms: u32 = result.read(2);
                let recovery_time_ms: u32 = result.read(3);
                let effect_type: u32 = result.try_read(4).unwrap_or(0);
                let effect_base_points: i32 = result.try_read(5).unwrap_or(0);
                let effect_bonus_coefficient: f32 = result.try_read(6).unwrap_or(0.0);
                let aura_type: Option<i32> = result.try_read(7);
                let effect_misc_value_1: i32 = result.try_read(8).unwrap_or(0);
                let effect_misc_value_2: i32 = result.try_read(9).unwrap_or(0);
                let effect_trigger_spell: i32 = result.try_read(10).unwrap_or(0);
                let effect_radius_index_1: u32 = result.try_read(11).unwrap_or(0);
                let position_facing: f32 = result.try_read(12).unwrap_or(0.0);
                let effect_index: u32 = result.try_read(13).unwrap_or(0);
                let effect_chain_targets: i32 = result.try_read(14).unwrap_or(0);
                let implicit_target_1: u32 = result.try_read(15).unwrap_or(0);
                let implicit_target_2: u32 = result.try_read(16).unwrap_or(0);
                let requires_spell_focus: u32 = result.try_read(17).unwrap_or(0);
                let effect_spell_class_mask = [
                    result.try_read(18).unwrap_or(0),
                    result.try_read(19).unwrap_or(0),
                    result.try_read(20).unwrap_or(0),
                    result.try_read(21).unwrap_or(0),
                ];
                let effect_die_sides: i32 = result.try_read(22).unwrap_or(0);

                let spell_info = store.spells.entry(spell_id).or_insert_with(|| SpellInfo {
                    spell_id,
                    cast_time_ms,
                    cooldown_ms,
                    recovery_time_ms,
                    effect_type,
                    effect_base_points,
                    effect_bonus_coefficient,
                    aura_type,
                    display_flags: 0,
                    requires_spell_focus,
                    effects: Vec::new(),
                });

                if effect_type != 0 {
                    spell_info.effects.push(SpellEffectInfo {
                        effect_index,
                        effect: effect_type,
                        effect_aura: aura_type.unwrap_or(0),
                        effect_base_points,
                        effect_die_sides,
                        effect_spell_class_mask,
                        effect_misc_value_1,
                        effect_misc_value_2,
                        effect_trigger_spell,
                        effect_radius_index_1,
                        position_facing,
                        chain_targets: effect_chain_targets,
                        implicit_target_1,
                        implicit_target_2,
                    });
                }

                if !result.next_row() {
                    break;
                }
            }
        }

        info!(
            "Loaded {} spells from hotfixes database",
            store.spells.len()
        );
        Ok(store)
    }

    /// Look up a spell by ID.
    pub fn get(&self, spell_id: i32) -> Option<&SpellInfo> {
        self.spells.get(&spell_id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &SpellInfo> {
        self.spells.values()
    }

    pub fn implicit_target_conditions_like_cpp(
        &self,
        spell_id: i32,
        effect_index: u32,
    ) -> Option<&ConditionsReference> {
        self.implicit_target_conditions
            .get(&(spell_id, effect_index))
    }

    pub fn attach_spell_implicit_target_conditions_like_cpp(
        &mut self,
        conditions: &ConditionEntriesByTypeStore,
    ) -> usize {
        let mut attached = 0;
        let Some(entries) = conditions.entries_for_source_type_like_cpp(
            wow_constants::ConditionSourceType::SpellImplicitTarget,
        ) else {
            return attached;
        };

        self.implicit_target_conditions.clear();
        for (id, bucket) in entries {
            let Some(spell) = self.spells.get(&id.source_entry) else {
                continue;
            };

            for effect in &spell.effects {
                let bit = 1_u32.checked_shl(effect.effect_index).unwrap_or(0);
                if bit == 0 || (id.source_group & bit) == 0 {
                    continue;
                }

                self.implicit_target_conditions.insert(
                    (id.source_entry, effect.effect_index),
                    ConditionsReference::new(bucket),
                );
                attached += bucket.len();
            }
        }

        attached
    }

    /// Insert a spell into the store (for testing or dynamic registration).
    #[allow(dead_code)]
    pub fn insert(&mut self, spell_id: i32, info: SpellInfo) {
        self.spells.insert(spell_id, info);
    }

    /// Get the total number of loaded spells.
    pub fn len(&self) -> usize {
        self.spells.len()
    }

    /// Check if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.spells.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Condition, ConditionEntriesByTypeStore};
    use wow_constants::{ConditionSourceType, ConditionType};

    #[test]
    fn test_spell_store_creation() {
        let store = SpellStore::new();
        assert!(store.is_empty(), "new store should be empty");
    }

    #[test]
    fn test_spell_info_effective_cooldown() {
        let spell = SpellInfo {
            spell_id: 100,
            cast_time_ms: 0,
            cooldown_ms: 1500,
            recovery_time_ms: 8000,
            effect_type: 2,
            effect_base_points: 50,
            effect_bonus_coefficient: 0.5,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            effects: Vec::new(),
        };

        // recovery_time_ms is larger
        assert_eq!(spell.effective_cooldown_ms(), 8000);

        let instant = SpellInfo {
            spell_id: 100,
            cast_time_ms: 0,
            cooldown_ms: 1500,
            recovery_time_ms: 0,
            effect_type: 2,
            effect_base_points: 50,
            effect_bonus_coefficient: 0.5,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            effects: Vec::new(),
        };

        // GCD is the limit
        assert_eq!(instant.effective_cooldown_ms(), 1500);
    }

    #[test]
    fn spell_info_requires_spell_focus_matches_cpp_field() {
        let mut spell = SpellInfo {
            spell_id: 100,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            effects: Vec::new(),
        };

        assert!(!spell.requires_spell_focus_like_cpp());
        spell.requires_spell_focus = 181;
        assert!(spell.requires_spell_focus_like_cpp());
    }

    #[test]
    fn spell_implicit_target_effect_mask_normalizes_like_cpp_conditionmgr() {
        let spell = SpellInfo {
            spell_id: 100,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            effects: vec![
                SpellEffectInfo {
                    effect_index: 0,
                    effect: 0,
                    chain_targets: 0,
                    implicit_target_1: 6,
                    implicit_target_2: 0,
                    ..Default::default()
                },
                SpellEffectInfo {
                    effect_index: 1,
                    effect: 0,
                    chain_targets: 0,
                    implicit_target_1: 7,
                    implicit_target_2: 0,
                    ..Default::default()
                },
                SpellEffectInfo {
                    effect_index: 2,
                    effect: spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_RAID,
                    chain_targets: 0,
                    implicit_target_1: 0,
                    implicit_target_2: 0,
                    ..Default::default()
                },
                SpellEffectInfo {
                    effect_index: 3,
                    effect: 0,
                    chain_targets: 2,
                    implicit_target_1: 0,
                    implicit_target_2: 0,
                    ..Default::default()
                },
            ],
        };

        assert_eq!(
            spell.normalized_implicit_target_effect_mask_like_cpp(0b1111),
            0b1110
        );
        assert_eq!(
            spell.normalized_implicit_target_effect_mask_like_cpp(0b0001),
            0
        );
    }

    #[test]
    fn spell_effect_detects_mounted_aura_like_cpp() {
        let mounted = SpellEffectInfo {
            effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_aura: aura_types::SPELL_AURA_MOUNTED,
            effect_base_points: 11,
            effect_misc_value_1: 22,
            effect_misc_value_2: 33,
            ..Default::default()
        };
        let other_aura = SpellEffectInfo {
            effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_aura: aura_types::SPELL_AURA_HASTE_SPELLS,
            ..Default::default()
        };

        assert!(mounted.is_mounted_aura_like_cpp());
        assert!(!other_aura.is_mounted_aura_like_cpp());
        assert_eq!(mounted.effect_base_points, 11);
        assert_eq!(mounted.effect_misc_value_1, 22);
        assert_eq!(mounted.effect_misc_value_2, 33);
    }

    #[test]
    fn spell_effect_calc_value_no_caster_rolls_die_sides_like_cpp() {
        let no_die = SpellEffectInfo {
            effect_base_points: 10,
            effect_die_sides: 0,
            ..Default::default()
        };
        assert_eq!(
            no_die.calc_value_no_caster_with_die_roll_like_cpp(|_, _| unreachable!()),
            10
        );

        let one_sided = SpellEffectInfo {
            effect_base_points: 10,
            effect_die_sides: 1,
            ..Default::default()
        };
        assert_eq!(
            one_sided.calc_value_no_caster_with_die_roll_like_cpp(|_, _| unreachable!()),
            11
        );

        let positive_range = SpellEffectInfo {
            effect_base_points: 10,
            effect_die_sides: 7,
            ..Default::default()
        };
        assert_eq!(
            positive_range.calc_value_no_caster_with_die_roll_like_cpp(|min, max| {
                assert_eq!((min, max), (1, 7));
                4
            }),
            14
        );

        let negative_range = SpellEffectInfo {
            effect_base_points: 10,
            effect_die_sides: -3,
            ..Default::default()
        };
        assert_eq!(
            negative_range.calc_value_no_caster_with_die_roll_like_cpp(|min, max| {
                assert_eq!((min, max), (-3, 1));
                -2
            }),
            8
        );
    }

    #[test]
    fn spell_effect_constants_match_cpp_shared_defines() {
        // C++ `SharedDefines.h`: `SpellEffects` enum.
        assert_eq!(spell_effect_types::SPELL_EFFECT_NONE, 0);
        assert_eq!(spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE, 2);
        assert_eq!(spell_effect_types::SPELL_EFFECT_PORTAL_TELEPORT, 4);
        assert_eq!(spell_effect_types::SPELL_EFFECT_APPLY_AURA, 6);
        assert_eq!(spell_effect_types::SPELL_EFFECT_ENVIRONMENTAL_DAMAGE, 7);
        assert_eq!(spell_effect_types::SPELL_EFFECT_POWER_DRAIN, 8);
        assert_eq!(spell_effect_types::SPELL_EFFECT_HEALTH_LEECH, 9);
        assert_eq!(spell_effect_types::SPELL_EFFECT_HEAL, 10);
        assert_eq!(spell_effect_types::SPELL_EFFECT_BIND, 11);
        assert_eq!(spell_effect_types::SPELL_EFFECT_PORTAL, 12);
        assert_eq!(spell_effect_types::SPELL_EFFECT_RITUAL_BASE, 13);
        assert_eq!(spell_effect_types::SPELL_EFFECT_RITUAL_SPECIALIZE, 14);
        assert_eq!(spell_effect_types::SPELL_EFFECT_RITUAL_ACTIVATE_PORTAL, 15);
        assert_eq!(spell_effect_types::SPELL_EFFECT_QUEST_COMPLETE, 16);
        assert_eq!(spell_effect_types::SPELL_EFFECT_ADD_EXTRA_ATTACKS, 19);
        assert_eq!(spell_effect_types::SPELL_EFFECT_DODGE, 20);
        assert_eq!(spell_effect_types::SPELL_EFFECT_EVADE, 21);
        assert_eq!(spell_effect_types::SPELL_EFFECT_PARRY, 22);
        assert_eq!(spell_effect_types::SPELL_EFFECT_BLOCK, 23);
        assert_eq!(spell_effect_types::SPELL_EFFECT_WEAPON, 25);
        assert_eq!(spell_effect_types::SPELL_EFFECT_DEFENSE, 26);
        assert_eq!(spell_effect_types::SPELL_EFFECT_ENERGIZE, 30);
        assert_eq!(spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_PARTY, 35);
        assert_eq!(spell_effect_types::SPELL_EFFECT_LEARN_SPELL, 36);
        assert_eq!(spell_effect_types::SPELL_EFFECT_SPELL_DEFENSE, 37);
        assert_eq!(spell_effect_types::SPELL_EFFECT_LANGUAGE, 39);
        assert_eq!(spell_effect_types::SPELL_EFFECT_DUAL_WIELD, 40);
        assert_eq!(spell_effect_types::SPELL_EFFECT_SKILL, 118);
        assert_eq!(spell_effect_types::SPELL_EFFECT_PLAY_MOVIE, 45);
        assert_eq!(spell_effect_types::SPELL_EFFECT_SPAWN, 46);
        assert_eq!(spell_effect_types::SPELL_EFFECT_TRADE_SKILL, 47);
        assert_eq!(spell_effect_types::SPELL_EFFECT_STEALTH, 48);
        assert_eq!(spell_effect_types::SPELL_EFFECT_DETECT, 49);
        assert_eq!(spell_effect_types::SPELL_EFFECT_FORCE_CRITICAL_HIT, 51);
        assert_eq!(spell_effect_types::SPELL_EFFECT_GUARANTEE_HIT, 52);
        assert_eq!(spell_effect_types::SPELL_EFFECT_POWER_BURN, 62);
        assert_eq!(spell_effect_types::SPELL_EFFECT_THREAT, 63);
        assert_eq!(spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_RAID, 65);
        assert_eq!(spell_effect_types::SPELL_EFFECT_HEAL_MAX_HEALTH, 67);
        assert_eq!(spell_effect_types::SPELL_EFFECT_DISTRACT, 69);
        assert_eq!(spell_effect_types::SPELL_EFFECT_PULL, 70);
        assert_eq!(spell_effect_types::SPELL_EFFECT_HEAL_MECHANICAL, 75);
        assert_eq!(spell_effect_types::SPELL_EFFECT_ATTACK, 78);
        assert_eq!(spell_effect_types::SPELL_EFFECT_SANCTUARY, 79);
        assert_eq!(spell_effect_types::SPELL_EFFECT_CREATE_HOUSE, 81);
        assert_eq!(spell_effect_types::SPELL_EFFECT_BIND_SIGHT, 82);
        assert_eq!(spell_effect_types::SPELL_EFFECT_DUEL, 83);
        assert_eq!(spell_effect_types::SPELL_EFFECT_KILL_CREDIT, 90);
        assert_eq!(spell_effect_types::SPELL_EFFECT_THREAT_ALL, 91);
        assert_eq!(spell_effect_types::SPELL_EFFECT_FORCE_DESELECT, 93);
        assert_eq!(spell_effect_types::SPELL_EFFECT_INEBRIATE, 100);
        assert_eq!(spell_effect_types::SPELL_EFFECT_DISMISS_PET, 102);
        assert_eq!(spell_effect_types::SPELL_EFFECT_REPUTATION, 103);
        assert_eq!(spell_effect_types::SPELL_EFFECT_SURVEY, 105);
        assert_eq!(spell_effect_types::SPELL_EFFECT_CHANGE_RAID_MARKER, 106);
        assert_eq!(spell_effect_types::SPELL_EFFECT_SHOW_CORPSE_LOOT, 107);
        assert_eq!(spell_effect_types::SPELL_EFFECT_112, 112);
        assert_eq!(spell_effect_types::SPELL_EFFECT_ATTACK_ME, 114);
        assert_eq!(spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_PET, 119);
        assert_eq!(spell_effect_types::SPELL_EFFECT_122, 122);
        assert_eq!(spell_effect_types::SPELL_EFFECT_MODIFY_THREAT_PERCENT, 125);
        assert_eq!(spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_FRIEND, 128);
        assert_eq!(spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_ENEMY, 129);
        assert_eq!(spell_effect_types::SPELL_EFFECT_KILL_CREDIT2, 134);
        assert_eq!(spell_effect_types::SPELL_EFFECT_CALL_PET, 135);
        assert_eq!(spell_effect_types::SPELL_EFFECT_HEAL_PCT, 136);
        assert_eq!(spell_effect_types::SPELL_EFFECT_ENERGIZE_PCT, 137);
        assert_eq!(spell_effect_types::SPELL_EFFECT_OBLITERATE_ITEM, 163);
        assert_eq!(spell_effect_types::SPELL_EFFECT_ALLOW_CONTROL_PET, 168);
        assert_eq!(spell_effect_types::SPELL_EFFECT_175, 175);
        assert_eq!(
            spell_effect_types::SPELL_EFFECT_DESPAWN_PERSISTENT_AREA_AURA,
            177
        );
        assert_eq!(spell_effect_types::SPELL_EFFECT_178, 178);
        assert_eq!(spell_effect_types::SPELL_EFFECT_UPDATE_AREATRIGGER, 180);
        assert_eq!(spell_effect_types::SPELL_EFFECT_DESPAWN_AREATRIGGER, 182);
        assert_eq!(spell_effect_types::SPELL_EFFECT_183, 183);
        assert_eq!(spell_effect_types::SPELL_EFFECT_REPUTATION_2, 184);
        assert_eq!(spell_effect_types::SPELL_EFFECT_185, 185);
        assert_eq!(spell_effect_types::SPELL_EFFECT_186, 186);
        assert_eq!(
            spell_effect_types::SPELL_EFFECT_RANDOMIZE_ARCHAEOLOGY_DIGSITES,
            187
        );
        assert_eq!(
            spell_effect_types::SPELL_EFFECT_SUMMON_STABLED_PET_AS_GUARDIAN,
            188
        );
        assert_eq!(spell_effect_types::SPELL_EFFECT_LOOT, 189);
        assert_eq!(spell_effect_types::SPELL_EFFECT_CHANGE_PARTY_MEMBERS, 190);
        assert_eq!(spell_effect_types::SPELL_EFFECT_TELEPORT_TO_DIGSITE, 191);
        assert_eq!(spell_effect_types::SPELL_EFFECT_UNCAGE_BATTLEPET, 192);
        assert_eq!(spell_effect_types::SPELL_EFFECT_START_PET_BATTLE, 193);
        assert_eq!(spell_effect_types::SPELL_EFFECT_194, 194);
        assert_eq!(spell_effect_types::SPELL_EFFECT_DESPAWN_SUMMON, 199);
        assert_eq!(
            spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_SUMMONS,
            202
        );
        assert_eq!(
            spell_effect_types::SPELL_EFFECT_CHANGE_BATTLEPET_QUALITY,
            204
        );
        assert_eq!(spell_effect_types::SPELL_EFFECT_ALTER_ITEM, 206);
        assert_eq!(spell_effect_types::SPELL_EFFECT_LAUNCH_QUEST_TASK, 207);
        assert_eq!(spell_effect_types::SPELL_EFFECT_SET_REPUTATION, 208);
        assert_eq!(spell_effect_types::SPELL_EFFECT_209, 209);
        assert_eq!(
            spell_effect_types::SPELL_EFFECT_LEARN_GARRISON_BUILDING,
            210
        );
        assert_eq!(
            spell_effect_types::SPELL_EFFECT_LEARN_GARRISON_SPECIALIZATION,
            211
        );
        assert_eq!(spell_effect_types::SPELL_EFFECT_CREATE_GARRISON, 214);
        assert_eq!(
            spell_effect_types::SPELL_EFFECT_UPGRADE_CHARACTER_SPELLS,
            215
        );
        assert_eq!(spell_effect_types::SPELL_EFFECT_CREATE_SHIPMENT, 216);
        assert_eq!(spell_effect_types::SPELL_EFFECT_UPGRADE_GARRISON, 217);
        assert_eq!(spell_effect_types::SPELL_EFFECT_218, 218);
        assert_eq!(spell_effect_types::SPELL_EFFECT_ADD_GARRISON_FOLLOWER, 220);
        assert_eq!(spell_effect_types::SPELL_EFFECT_ADD_GARRISON_MISSION, 221);
        assert_eq!(spell_effect_types::SPELL_EFFECT_CHANGE_ITEM_BONUSES, 223);
        assert_eq!(
            spell_effect_types::SPELL_EFFECT_ACTIVATE_GARRISON_BUILDING,
            224
        );
        assert_eq!(spell_effect_types::SPELL_EFFECT_GRANT_BATTLEPET_LEVEL, 225);
        assert_eq!(spell_effect_types::SPELL_EFFECT_TRIGGER_ACTION_SET, 226);
        assert_eq!(
            spell_effect_types::SPELL_EFFECT_TELEPORT_TO_LFG_DUNGEON,
            227
        );
        assert_eq!(spell_effect_types::SPELL_EFFECT_228, 228);
        assert_eq!(spell_effect_types::SPELL_EFFECT_SET_FOLLOWER_QUALITY, 229);
        assert_eq!(spell_effect_types::SPELL_EFFECT_230, 230);
        assert_eq!(
            spell_effect_types::SPELL_EFFECT_INCREASE_FOLLOWER_EXPERIENCE,
            231
        );
        assert_eq!(spell_effect_types::SPELL_EFFECT_REMOVE_PHASE, 232);
        assert_eq!(
            spell_effect_types::SPELL_EFFECT_RANDOMIZE_FOLLOWER_ABILITIES,
            233
        );
        assert_eq!(spell_effect_types::SPELL_EFFECT_234, 234);
        assert_eq!(spell_effect_types::SPELL_EFFECT_235, 235);
        assert_eq!(spell_effect_types::SPELL_EFFECT_INCREASE_SKILL, 238);
        assert_eq!(
            spell_effect_types::SPELL_EFFECT_END_GARRISON_BUILDING_CONSTRUCTION,
            239
        );
        assert_eq!(spell_effect_types::SPELL_EFFECT_GIVE_ARTIFACT_POWER, 240);
        assert_eq!(spell_effect_types::SPELL_EFFECT_241, 241);
        assert_eq!(
            spell_effect_types::SPELL_EFFECT_GIVE_ARTIFACT_POWER_NO_BONUS,
            242
        );
        assert_eq!(spell_effect_types::SPELL_EFFECT_LEARN_FOLLOWER_ABILITY, 244);
        assert_eq!(spell_effect_types::SPELL_EFFECT_UPGRADE_HEIRLOOM, 245);
        assert_eq!(
            spell_effect_types::SPELL_EFFECT_FINISH_GARRISON_MISSION,
            246
        );
        assert_eq!(
            spell_effect_types::SPELL_EFFECT_ADD_GARRISON_MISSION_SET,
            247
        );
        assert_eq!(spell_effect_types::SPELL_EFFECT_FINISH_SHIPMENT, 248);
        assert_eq!(spell_effect_types::SPELL_EFFECT_FORCE_EQUIP_ITEM, 249);
        assert_eq!(spell_effect_types::SPELL_EFFECT_TAKE_SCREENSHOT, 250);
        assert_eq!(
            spell_effect_types::SPELL_EFFECT_SET_GARRISON_CACHE_SIZE,
            251
        );
        assert_eq!(spell_effect_types::SPELL_EFFECT_TELEPORT_UNITS, 252);
        assert_eq!(spell_effect_types::SPELL_EFFECT_GIVE_HONOR, 253);
        assert_eq!(spell_effect_types::SPELL_EFFECT_JUMP_CHARGE, 254);
        assert_eq!(spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_SET, 255);
        assert_eq!(spell_effect_types::SPELL_EFFECT_256, 256);
        assert_eq!(spell_effect_types::SPELL_EFFECT_257, 257);
        assert_eq!(spell_effect_types::SPELL_EFFECT_MODIFY_KEYSTONE, 258);
        assert_eq!(
            spell_effect_types::SPELL_EFFECT_RESPEC_AZERITE_EMPOWERED_ITEM,
            259
        );
        assert_eq!(spell_effect_types::SPELL_EFFECT_SUMMON_STABLED_PET, 260);
        assert_eq!(spell_effect_types::SPELL_EFFECT_SCRAP_ITEM, 261);
        assert_eq!(spell_effect_types::SPELL_EFFECT_262, 262);
        assert_eq!(spell_effect_types::SPELL_EFFECT_REPAIR_ITEM, 263);
        assert_eq!(spell_effect_types::SPELL_EFFECT_REMOVE_GEM, 264);
        assert_eq!(
            spell_effect_types::SPELL_EFFECT_LEARN_AZERITE_ESSENCE_POWER,
            265
        );
        assert_eq!(
            spell_effect_types::SPELL_EFFECT_SET_ITEM_BONUS_LIST_GROUP_ENTRY,
            266
        );
        assert_eq!(spell_effect_types::SPELL_EFFECT_APPLY_MOUNT_EQUIPMENT, 268);
        assert_eq!(
            spell_effect_types::SPELL_EFFECT_INCREASE_ITEM_BONUS_LIST_GROUP_STEP,
            269
        );
        assert_eq!(spell_effect_types::SPELL_EFFECT_270, 270);
        assert_eq!(
            spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_PARTY_NONRANDOM,
            271
        );
        assert_eq!(spell_effect_types::SPELL_EFFECT_SET_COVENANT, 272);
        assert_eq!(
            spell_effect_types::SPELL_EFFECT_CRAFT_RUNEFORGE_LEGENDARY,
            273
        );
        assert_eq!(spell_effect_types::SPELL_EFFECT_274, 274);
        assert_eq!(spell_effect_types::SPELL_EFFECT_275, 275);
        assert_eq!(
            spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_ILLUSION,
            276
        );
        assert_eq!(spell_effect_types::SPELL_EFFECT_SET_CHROMIE_TIME, 277);
        assert_eq!(spell_effect_types::SPELL_EFFECT_278, 278);
        assert_eq!(spell_effect_types::SPELL_EFFECT_LEARN_GARR_TALENT, 279);
        assert_eq!(spell_effect_types::SPELL_EFFECT_280, 280);
        assert_eq!(spell_effect_types::SPELL_EFFECT_LEARN_SOULBIND_CONDUIT, 281);
        assert_eq!(
            spell_effect_types::SPELL_EFFECT_CONVERT_ITEMS_TO_CURRENCY,
            282
        );
        assert_eq!(spell_effect_types::SPELL_EFFECT_COMPLETE_CAMPAIGN, 283);
        assert_eq!(spell_effect_types::SPELL_EFFECT_MODIFY_KEYSTONE_2, 285);
        assert_eq!(
            spell_effect_types::SPELL_EFFECT_GRANT_BATTLEPET_EXPERIENCE,
            286
        );
        assert_eq!(
            spell_effect_types::SPELL_EFFECT_SET_GARRISON_FOLLOWER_LEVEL,
            287
        );
        assert_eq!(spell_effect_types::SPELL_EFFECT_CRAFT_ITEM, 288);
        assert_eq!(spell_effect_types::SPELL_EFFECT_MODIFY_AURA_STACKS, 289);
        assert_eq!(spell_effect_types::SPELL_EFFECT_MODIFY_COOLDOWN, 290);
        assert_eq!(spell_effect_types::SPELL_EFFECT_MODIFY_COOLDOWNS, 291);
        assert_eq!(
            spell_effect_types::SPELL_EFFECT_MODIFY_COOLDOWNS_BY_CATEGORY,
            292
        );
        assert_eq!(spell_effect_types::SPELL_EFFECT_MODIFY_CHARGES, 293);
        assert_eq!(spell_effect_types::SPELL_EFFECT_CRAFT_LOOT, 294);
        assert_eq!(spell_effect_types::SPELL_EFFECT_SALVAGE_ITEM, 295);
        assert_eq!(spell_effect_types::SPELL_EFFECT_CRAFT_SALVAGE_ITEM, 296);
        assert_eq!(spell_effect_types::SPELL_EFFECT_RECRAFT_ITEM, 297);
        assert_eq!(
            spell_effect_types::SPELL_EFFECT_CANCEL_ALL_PRIVATE_CONVERSATIONS,
            298
        );
        assert_eq!(spell_effect_types::SPELL_EFFECT_299, 299);
        assert_eq!(spell_effect_types::SPELL_EFFECT_300, 300);
        assert_eq!(spell_effect_types::SPELL_EFFECT_CRAFT_ENCHANT, 301);
        assert_eq!(spell_effect_types::SPELL_EFFECT_GATHERING, 302);
        assert_eq!(spell_effect_types::SPELL_EFFECT_305, 305);
        assert_eq!(spell_effect_types::SPELL_EFFECT_UPDATE_INTERACTIONS, 306);
        assert_eq!(spell_effect_types::SPELL_EFFECT_307, 307);
        assert_eq!(spell_effect_types::SPELL_EFFECT_CANCEL_PRELOAD_WORLD, 308);
        assert_eq!(spell_effect_types::SPELL_EFFECT_PRELOAD_WORLD, 309);
        assert_eq!(spell_effect_types::SPELL_EFFECT_310, 310);
        assert_eq!(spell_effect_types::SPELL_EFFECT_ENSURE_WORLD_LOADED, 311);
        assert_eq!(spell_effect_types::SPELL_EFFECT_312, 312);
        assert_eq!(spell_effect_types::SPELL_EFFECT_CHANGE_ITEM_BONUSES_2, 313);
        assert_eq!(spell_effect_types::SPELL_EFFECT_ADD_SOCKET_BONUS, 314);
        assert_eq!(
            spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_APPEARANCE_FROM_ITEM_MOD_APPEARANCE_GROUP,
            315
        );

        // C++ `SpellAuraDefines.h`: selected `AuraType` enum anchors.
        assert_eq!(aura_types::SPELL_AURA_MOD_DETECT_RANGE, 91);
        assert_eq!(aura_types::SPELL_AURA_MOD_DETECTED_RANGE, 152);
        assert_eq!(aura_types::SPELL_AURA_MOD_BATTLE_PET_XP_PCT, 420);
    }

    #[test]
    fn spell_effect_null_or_unused_classifier_matches_cpp_dispatch_subset() {
        for effect in [
            spell_effect_types::SPELL_EFFECT_NONE,
            spell_effect_types::SPELL_EFFECT_PORTAL_TELEPORT,
            spell_effect_types::SPELL_EFFECT_PORTAL,
            spell_effect_types::SPELL_EFFECT_RITUAL_BASE,
            spell_effect_types::SPELL_EFFECT_RITUAL_SPECIALIZE,
            spell_effect_types::SPELL_EFFECT_RITUAL_ACTIVATE_PORTAL,
            spell_effect_types::SPELL_EFFECT_DODGE,
            spell_effect_types::SPELL_EFFECT_EVADE,
            spell_effect_types::SPELL_EFFECT_WEAPON,
            spell_effect_types::SPELL_EFFECT_DEFENSE,
            spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_PARTY,
            spell_effect_types::SPELL_EFFECT_SPELL_DEFENSE,
            spell_effect_types::SPELL_EFFECT_LANGUAGE,
            spell_effect_types::SPELL_EFFECT_SPAWN,
            spell_effect_types::SPELL_EFFECT_STEALTH,
            spell_effect_types::SPELL_EFFECT_DETECT,
            spell_effect_types::SPELL_EFFECT_FORCE_CRITICAL_HIT,
            spell_effect_types::SPELL_EFFECT_GUARANTEE_HIT,
            spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_RAID,
            spell_effect_types::SPELL_EFFECT_ATTACK,
            spell_effect_types::SPELL_EFFECT_CREATE_HOUSE,
            spell_effect_types::SPELL_EFFECT_BIND_SIGHT,
            spell_effect_types::SPELL_EFFECT_THREAT_ALL,
            spell_effect_types::SPELL_EFFECT_SURVEY,
            spell_effect_types::SPELL_EFFECT_SHOW_CORPSE_LOOT,
            spell_effect_types::SPELL_EFFECT_112,
            spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_PET,
            spell_effect_types::SPELL_EFFECT_122,
            spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_FRIEND,
            spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_ENEMY,
            spell_effect_types::SPELL_EFFECT_CALL_PET,
            spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_OWNER,
            spell_effect_types::SPELL_EFFECT_OBLITERATE_ITEM,
            spell_effect_types::SPELL_EFFECT_ALLOW_CONTROL_PET,
            spell_effect_types::SPELL_EFFECT_175,
            spell_effect_types::SPELL_EFFECT_DESPAWN_PERSISTENT_AREA_AURA,
            spell_effect_types::SPELL_EFFECT_178,
            spell_effect_types::SPELL_EFFECT_UPDATE_AREATRIGGER,
            spell_effect_types::SPELL_EFFECT_DESPAWN_AREATRIGGER,
            spell_effect_types::SPELL_EFFECT_183,
            spell_effect_types::SPELL_EFFECT_REPUTATION_2,
            spell_effect_types::SPELL_EFFECT_185,
            spell_effect_types::SPELL_EFFECT_186,
            spell_effect_types::SPELL_EFFECT_RANDOMIZE_ARCHAEOLOGY_DIGSITES,
            spell_effect_types::SPELL_EFFECT_SUMMON_STABLED_PET_AS_GUARDIAN,
            spell_effect_types::SPELL_EFFECT_LOOT,
            spell_effect_types::SPELL_EFFECT_CHANGE_PARTY_MEMBERS,
            spell_effect_types::SPELL_EFFECT_TELEPORT_TO_DIGSITE,
            spell_effect_types::SPELL_EFFECT_START_PET_BATTLE,
            spell_effect_types::SPELL_EFFECT_194,
            spell_effect_types::SPELL_EFFECT_DESPAWN_SUMMON,
            spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_SUMMONS,
            spell_effect_types::SPELL_EFFECT_ALTER_ITEM,
            spell_effect_types::SPELL_EFFECT_LAUNCH_QUEST_TASK,
            spell_effect_types::SPELL_EFFECT_SET_REPUTATION,
            spell_effect_types::SPELL_EFFECT_209,
            spell_effect_types::SPELL_EFFECT_LEARN_GARRISON_BUILDING,
            spell_effect_types::SPELL_EFFECT_LEARN_GARRISON_SPECIALIZATION,
            spell_effect_types::SPELL_EFFECT_CREATE_GARRISON,
            spell_effect_types::SPELL_EFFECT_UPGRADE_CHARACTER_SPELLS,
            spell_effect_types::SPELL_EFFECT_CREATE_SHIPMENT,
            spell_effect_types::SPELL_EFFECT_UPGRADE_GARRISON,
            spell_effect_types::SPELL_EFFECT_218,
            spell_effect_types::SPELL_EFFECT_ADD_GARRISON_FOLLOWER,
            spell_effect_types::SPELL_EFFECT_ADD_GARRISON_MISSION,
            spell_effect_types::SPELL_EFFECT_CHANGE_ITEM_BONUSES,
            spell_effect_types::SPELL_EFFECT_ACTIVATE_GARRISON_BUILDING,
            spell_effect_types::SPELL_EFFECT_TRIGGER_ACTION_SET,
            spell_effect_types::SPELL_EFFECT_TELEPORT_TO_LFG_DUNGEON,
            spell_effect_types::SPELL_EFFECT_228,
            spell_effect_types::SPELL_EFFECT_SET_FOLLOWER_QUALITY,
            spell_effect_types::SPELL_EFFECT_230,
            spell_effect_types::SPELL_EFFECT_INCREASE_FOLLOWER_EXPERIENCE,
            spell_effect_types::SPELL_EFFECT_REMOVE_PHASE,
            spell_effect_types::SPELL_EFFECT_RANDOMIZE_FOLLOWER_ABILITIES,
            spell_effect_types::SPELL_EFFECT_234,
            spell_effect_types::SPELL_EFFECT_235,
            spell_effect_types::SPELL_EFFECT_INCREASE_SKILL,
            spell_effect_types::SPELL_EFFECT_END_GARRISON_BUILDING_CONSTRUCTION,
            spell_effect_types::SPELL_EFFECT_GIVE_ARTIFACT_POWER,
            spell_effect_types::SPELL_EFFECT_241,
            spell_effect_types::SPELL_EFFECT_GIVE_ARTIFACT_POWER_NO_BONUS,
            spell_effect_types::SPELL_EFFECT_LEARN_FOLLOWER_ABILITY,
            spell_effect_types::SPELL_EFFECT_FINISH_GARRISON_MISSION,
            spell_effect_types::SPELL_EFFECT_ADD_GARRISON_MISSION_SET,
            spell_effect_types::SPELL_EFFECT_FINISH_SHIPMENT,
            spell_effect_types::SPELL_EFFECT_FORCE_EQUIP_ITEM,
            spell_effect_types::SPELL_EFFECT_TAKE_SCREENSHOT,
            spell_effect_types::SPELL_EFFECT_SET_GARRISON_CACHE_SIZE,
            spell_effect_types::SPELL_EFFECT_256,
            spell_effect_types::SPELL_EFFECT_257,
            spell_effect_types::SPELL_EFFECT_MODIFY_KEYSTONE,
            spell_effect_types::SPELL_EFFECT_RESPEC_AZERITE_EMPOWERED_ITEM,
            spell_effect_types::SPELL_EFFECT_SUMMON_STABLED_PET,
            spell_effect_types::SPELL_EFFECT_SCRAP_ITEM,
            spell_effect_types::SPELL_EFFECT_262,
            spell_effect_types::SPELL_EFFECT_REPAIR_ITEM,
            spell_effect_types::SPELL_EFFECT_REMOVE_GEM,
            spell_effect_types::SPELL_EFFECT_LEARN_AZERITE_ESSENCE_POWER,
            spell_effect_types::SPELL_EFFECT_SET_ITEM_BONUS_LIST_GROUP_ENTRY,
            spell_effect_types::SPELL_EFFECT_APPLY_MOUNT_EQUIPMENT,
            spell_effect_types::SPELL_EFFECT_INCREASE_ITEM_BONUS_LIST_GROUP_STEP,
            spell_effect_types::SPELL_EFFECT_270,
            spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_PARTY_NONRANDOM,
            spell_effect_types::SPELL_EFFECT_SET_COVENANT,
            spell_effect_types::SPELL_EFFECT_CRAFT_RUNEFORGE_LEGENDARY,
            spell_effect_types::SPELL_EFFECT_274,
            spell_effect_types::SPELL_EFFECT_275,
            spell_effect_types::SPELL_EFFECT_SET_CHROMIE_TIME,
            spell_effect_types::SPELL_EFFECT_278,
            spell_effect_types::SPELL_EFFECT_LEARN_GARR_TALENT,
            spell_effect_types::SPELL_EFFECT_280,
            spell_effect_types::SPELL_EFFECT_LEARN_SOULBIND_CONDUIT,
            spell_effect_types::SPELL_EFFECT_CONVERT_ITEMS_TO_CURRENCY,
            spell_effect_types::SPELL_EFFECT_COMPLETE_CAMPAIGN,
            spell_effect_types::SPELL_EFFECT_MODIFY_KEYSTONE_2,
            spell_effect_types::SPELL_EFFECT_SET_GARRISON_FOLLOWER_LEVEL,
            spell_effect_types::SPELL_EFFECT_CRAFT_ITEM,
            spell_effect_types::SPELL_EFFECT_CRAFT_LOOT,
            spell_effect_types::SPELL_EFFECT_SALVAGE_ITEM,
            spell_effect_types::SPELL_EFFECT_CRAFT_SALVAGE_ITEM,
            spell_effect_types::SPELL_EFFECT_RECRAFT_ITEM,
            spell_effect_types::SPELL_EFFECT_CANCEL_ALL_PRIVATE_CONVERSATIONS,
            spell_effect_types::SPELL_EFFECT_299,
            spell_effect_types::SPELL_EFFECT_300,
            spell_effect_types::SPELL_EFFECT_CRAFT_ENCHANT,
            spell_effect_types::SPELL_EFFECT_GATHERING,
            spell_effect_types::SPELL_EFFECT_305,
            spell_effect_types::SPELL_EFFECT_UPDATE_INTERACTIONS,
            spell_effect_types::SPELL_EFFECT_307,
            spell_effect_types::SPELL_EFFECT_CANCEL_PRELOAD_WORLD,
            spell_effect_types::SPELL_EFFECT_PRELOAD_WORLD,
            spell_effect_types::SPELL_EFFECT_310,
            spell_effect_types::SPELL_EFFECT_ENSURE_WORLD_LOADED,
            spell_effect_types::SPELL_EFFECT_312,
            spell_effect_types::SPELL_EFFECT_CHANGE_ITEM_BONUSES_2,
            spell_effect_types::SPELL_EFFECT_ADD_SOCKET_BONUS,
            spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_APPEARANCE_FROM_ITEM_MOD_APPEARANCE_GROUP,
        ] {
            assert!(
                spell_effect_types::is_cpp_null_or_unused_noop(effect),
                "effect {effect} should mirror C++ EffectNULL/EffectUnused"
            );
        }

        assert!(
            !spell_effect_types::is_cpp_null_or_unused_noop(3),
            "C++ SPELL_EFFECT_DUMMY dispatches EffectDummy and remains script-driven"
        );
        assert!(!spell_effect_types::is_cpp_null_or_unused_noop(
            spell_effect_types::SPELL_EFFECT_QUEST_COMPLETE
        ));
        for real_handler_effect in [
            spell_effect_types::SPELL_EFFECT_CHANGE_BATTLEPET_QUALITY,
            spell_effect_types::SPELL_EFFECT_GRANT_BATTLEPET_LEVEL,
            243,
            spell_effect_types::SPELL_EFFECT_UPGRADE_HEIRLOOM,
            spell_effect_types::SPELL_EFFECT_TELEPORT_UNITS,
            spell_effect_types::SPELL_EFFECT_GIVE_HONOR,
            spell_effect_types::SPELL_EFFECT_JUMP_CHARGE,
            spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_SET,
            spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_ILLUSION,
            284,
            spell_effect_types::SPELL_EFFECT_GRANT_BATTLEPET_EXPERIENCE,
            289,
            290,
            291,
            292,
            293,
            303,
            304,
        ] {
            assert!(
                !spell_effect_types::is_cpp_null_or_unused_noop(real_handler_effect),
                "effect {real_handler_effect} has a real C++ dispatch handler in this range"
            );
        }
    }

    #[test]
    fn spell_effect_detects_provide_spell_focus_aura_like_cpp() {
        let focus = SpellEffectInfo {
            effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_aura: aura_types::SPELL_AURA_PROVIDE_SPELL_FOCUS,
            effect_misc_value_1: 181,
            ..Default::default()
        };
        let other_effect = SpellEffectInfo {
            effect: spell_effect_types::SPELL_EFFECT_HEAL,
            effect_aura: aura_types::SPELL_AURA_PROVIDE_SPELL_FOCUS,
            ..Default::default()
        };

        assert!(focus.is_provide_spell_focus_aura_like_cpp());
        assert!(!other_effect.is_provide_spell_focus_aura_like_cpp());
        assert_eq!(focus.effect_misc_value_1, 181);
    }

    #[test]
    fn spell_effect_detects_focus_destination_implicit_targets_like_cpp() {
        let mut effect = SpellEffectInfo {
            implicit_target_1: implicit_targets::TARGET_DEST_NEARBY_ENTRY,
            ..Default::default()
        };
        assert!(effect.has_focus_destination_implicit_target_like_cpp());

        effect.implicit_target_1 = 0;
        effect.implicit_target_2 = implicit_targets::TARGET_DEST_NEARBY_ENTRY_2;
        assert!(effect.has_focus_destination_implicit_target_like_cpp());

        effect.implicit_target_2 = implicit_targets::TARGET_DEST_NEARBY_ENTRY_OR_DB;
        assert!(effect.has_focus_destination_implicit_target_like_cpp());

        effect.implicit_target_2 = 40;
        assert!(!effect.has_focus_destination_implicit_target_like_cpp());
    }

    #[test]
    fn spell_target_position_store_loads_or_db_targets_like_cpp() {
        let mut spell_store = SpellStore::new();
        spell_store.insert(
            710,
            SpellInfo {
                spell_id: 710,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: 0,
                effect_base_points: 0,
                effect_bonus_coefficient: 0.0,
                aura_type: None,
                display_flags: 0,
                requires_spell_focus: 0,
                effects: vec![SpellEffectInfo {
                    effect_index: 1,
                    implicit_target_1: implicit_targets::TARGET_DEST_NEARBY_ENTRY_OR_DB,
                    ..Default::default()
                }],
            },
        );

        let store = SpellTargetPositionStoreLikeCpp::from_rows_like_cpp(
            [SpellTargetPositionRowLikeCpp {
                spell_id: 710,
                effect_index: 1,
                target_map_id: 571,
                x: 100.0,
                y: 200.0,
                z: 30.0,
                orientation: Some(1.25),
            }],
            &spell_store,
            |map_id| map_id == 571,
        );

        assert_eq!(store.load_report_like_cpp().loaded, 1);
        assert_eq!(
            store.get(710, 1).map(|target| target.position),
            Some(wow_core::Position::new(100.0, 200.0, 30.0, 1.25))
        );
    }

    #[test]
    fn spell_target_position_store_uses_effect_facing_when_orientation_is_null_like_cpp() {
        let mut spell_store = SpellStore::new();
        spell_store.insert(
            9268,
            SpellInfo {
                spell_id: 9268,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: 0,
                effect_base_points: 0,
                effect_bonus_coefficient: 0.0,
                aura_type: None,
                display_flags: 0,
                requires_spell_focus: 0,
                effects: vec![SpellEffectInfo {
                    effect_index: 0,
                    position_facing: 90.0,
                    implicit_target_1: implicit_targets::TARGET_DEST_DB,
                    ..Default::default()
                }],
            },
        );

        let store = SpellTargetPositionStoreLikeCpp::from_rows_like_cpp(
            [SpellTargetPositionRowLikeCpp {
                spell_id: 9268,
                effect_index: 0,
                target_map_id: 0,
                x: -10.0,
                y: 20.0,
                z: 5.0,
                orientation: None,
            }],
            &spell_store,
            |map_id| map_id == 0,
        );

        let position = store.get(9268, 0).expect("target position").position;
        assert!((position.orientation - std::f32::consts::FRAC_PI_2).abs() < 0.0001);
    }

    #[test]
    fn spell_target_position_store_rejects_wrong_effect_target_like_cpp() {
        let mut spell_store = SpellStore::new();
        spell_store.insert(
            711,
            SpellInfo {
                spell_id: 711,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: 0,
                effect_base_points: 0,
                effect_bonus_coefficient: 0.0,
                aura_type: None,
                display_flags: 0,
                requires_spell_focus: 0,
                effects: vec![SpellEffectInfo {
                    effect_index: 0,
                    implicit_target_1: implicit_targets::TARGET_DEST_NEARBY_ENTRY,
                    ..Default::default()
                }],
            },
        );

        let store = SpellTargetPositionStoreLikeCpp::from_rows_like_cpp(
            [SpellTargetPositionRowLikeCpp {
                spell_id: 711,
                effect_index: 0,
                target_map_id: 571,
                x: 1.0,
                y: 2.0,
                z: 3.0,
                orientation: Some(0.0),
            }],
            &spell_store,
            |_| true,
        );

        assert!(store.is_empty());
        assert_eq!(store.load_report_like_cpp().skipped_unsupported_target, 1);
    }

    #[test]
    fn spell_implicit_target_conditions_attach_to_effects_like_cpp() {
        let mut store = SpellStore::new();
        store.insert(
            100,
            SpellInfo {
                spell_id: 100,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: 0,
                effect_base_points: 0,
                effect_bonus_coefficient: 0.0,
                aura_type: None,
                display_flags: 0,
                requires_spell_focus: 0,
                effects: vec![
                    SpellEffectInfo {
                        effect_index: 0,
                        effect: 0,
                        chain_targets: 0,
                        implicit_target_1: 6,
                        implicit_target_2: 0,
                        ..Default::default()
                    },
                    SpellEffectInfo {
                        effect_index: 1,
                        effect: 0,
                        chain_targets: 0,
                        implicit_target_1: 7,
                        implicit_target_2: 0,
                        ..Default::default()
                    },
                ],
            },
        );
        let conditions = ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
            source_type: ConditionSourceType::SpellImplicitTarget,
            source_group: 0b11,
            source_entry: 100,
            condition_type: ConditionType::Aura,
            ..Condition::default()
        }]);

        assert_eq!(
            store.attach_spell_implicit_target_conditions_like_cpp(&conditions),
            2
        );
        assert!(
            store
                .implicit_target_conditions_like_cpp(100, 0)
                .and_then(|reference| reference.upgrade())
                .is_some_and(|conditions| conditions.len() == 1)
        );
        assert!(
            store
                .implicit_target_conditions_like_cpp(100, 1)
                .and_then(|reference| reference.upgrade())
                .is_some_and(|conditions| conditions.len() == 1)
        );
    }

    #[test]
    fn spell_pet_aura_store_loads_first_row_metadata_and_wildcard_like_cpp() {
        let outcome = SpellPetAuraStoreLikeCpp::load_spell_pet_auras_like_cpp(
            [
                SpellPetAuraRowLikeCpp {
                    spell_id: 10,
                    effect_index: 1,
                    pet_entry: 0,
                    aura_id: 100,
                },
                SpellPetAuraRowLikeCpp {
                    spell_id: 10,
                    effect_index: 1,
                    pet_entry: 700,
                    aura_id: 200,
                },
            ],
            |spell_id, effect_index| {
                assert_eq!((spell_id, effect_index), (10, 1));
                SpellPetAuraSourceLookupLikeCpp::Found(SpellPetAuraSourceEffectLikeCpp {
                    effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    apply_aura_name: SPELL_AURA_DUMMY_LIKE_CPP,
                    target_a: TARGET_UNIT_PET_LIKE_CPP,
                    calc_value: 35,
                })
            },
            |aura_id| matches!(aura_id, 100 | 200),
        );

        assert_eq!(outcome.loaded_row_count, 2);
        assert!(outcome.errors.is_empty());
        let pet_aura = outcome.store.get_pet_aura_like_cpp(10, 1).unwrap();
        assert!(pet_aura.remove_on_change_pet);
        assert_eq!(pet_aura.damage, 35);
        assert_eq!(pet_aura.aura_for_pet_entry_like_cpp(700), 200);
        assert_eq!(
            pet_aura.aura_for_pet_entry_like_cpp(701),
            100,
            "C++ PetAura::GetAura falls back to petEntry 0"
        );
        assert_eq!(
            outcome.store.get_pet_aura_like_cpp(10, 2),
            None,
            "C++ SpellMgr::GetPetAura keys by (spell << 8) + effect index"
        );
    }

    #[test]
    fn spell_pet_aura_store_rejects_invalid_first_rows_like_cpp() {
        let rows = [
            SpellPetAuraRowLikeCpp {
                spell_id: 1,
                effect_index: 0,
                pet_entry: 0,
                aura_id: 10,
            },
            SpellPetAuraRowLikeCpp {
                spell_id: 2,
                effect_index: 3,
                pet_entry: 0,
                aura_id: 20,
            },
            SpellPetAuraRowLikeCpp {
                spell_id: 3,
                effect_index: 0,
                pet_entry: 0,
                aura_id: 30,
            },
            SpellPetAuraRowLikeCpp {
                spell_id: 4,
                effect_index: 0,
                pet_entry: 0,
                aura_id: 40,
            },
        ];

        let outcome = SpellPetAuraStoreLikeCpp::load_spell_pet_auras_like_cpp(
            rows,
            |spell_id, _| match spell_id {
                1 => SpellPetAuraSourceLookupLikeCpp::SpellMissing,
                2 => SpellPetAuraSourceLookupLikeCpp::EffectIndexMissing,
                3 => SpellPetAuraSourceLookupLikeCpp::Found(SpellPetAuraSourceEffectLikeCpp {
                    effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    apply_aura_name: 73,
                    target_a: 0,
                    calc_value: 0,
                }),
                4 => SpellPetAuraSourceLookupLikeCpp::Found(SpellPetAuraSourceEffectLikeCpp {
                    effect: spell_effect_types::SPELL_EFFECT_DUMMY,
                    apply_aura_name: 0,
                    target_a: 0,
                    calc_value: 0,
                }),
                _ => unreachable!(),
            },
            |aura_id| aura_id != 40,
        );

        assert_eq!(outcome.loaded_row_count, 0);
        assert!(outcome.store.auras_by_spell_effect_key.is_empty());
        assert_eq!(
            outcome
                .errors
                .iter()
                .map(|error| error.kind)
                .collect::<Vec<_>>(),
            vec![
                SpellPetAuraLoadErrorKindLikeCpp::SpellMissing,
                SpellPetAuraLoadErrorKindLikeCpp::EffectIndexMissing,
                SpellPetAuraLoadErrorKindLikeCpp::SourceEffectNotDummy,
                SpellPetAuraLoadErrorKindLikeCpp::AuraSpellMissing,
            ]
        );
    }

    #[test]
    fn spell_pet_aura_store_duplicate_keys_add_aura_without_revalidation_like_cpp() {
        let mut source_lookups = 0;
        let mut aura_checks = 0;
        let outcome = SpellPetAuraStoreLikeCpp::load_spell_pet_auras_like_cpp(
            [
                SpellPetAuraRowLikeCpp {
                    spell_id: 77,
                    effect_index: 2,
                    pet_entry: 500,
                    aura_id: 900,
                },
                SpellPetAuraRowLikeCpp {
                    spell_id: 77,
                    effect_index: 2,
                    pet_entry: 501,
                    aura_id: 0,
                },
            ],
            |_, _| {
                source_lookups += 1;
                SpellPetAuraSourceLookupLikeCpp::Found(SpellPetAuraSourceEffectLikeCpp {
                    effect: spell_effect_types::SPELL_EFFECT_DUMMY,
                    apply_aura_name: 0,
                    target_a: 0,
                    calc_value: -15,
                })
            },
            |aura_id| {
                aura_checks += 1;
                aura_id == 900
            },
        );

        assert_eq!(
            source_lookups, 1,
            "C++ validates only before creating a new SpellPetAuraMap entry"
        );
        assert_eq!(aura_checks, 1);
        assert_eq!(outcome.loaded_row_count, 2);
        assert!(outcome.errors.is_empty());
        let pet_aura = outcome.store.get_pet_aura_like_cpp(77, 2).unwrap();
        assert!(!pet_aura.remove_on_change_pet);
        assert_eq!(pet_aura.damage, -15);
        assert_eq!(pet_aura.aura_for_pet_entry_like_cpp(500), 900);
        assert_eq!(pet_aura.aura_for_pet_entry_like_cpp(501), 0);
    }

    #[test]
    fn spell_threat_store_skips_missing_spells_like_cpp() {
        let outcome = SpellThreatStoreLikeCpp::from_rows_like_cpp(
            [
                SpellThreatRowLikeCpp {
                    spell_id: 100,
                    flat_mod: 7,
                    pct_mod: 1.25,
                    ap_pct_mod: 0.5,
                },
                SpellThreatRowLikeCpp {
                    spell_id: 200,
                    flat_mod: 9,
                    pct_mod: 2.0,
                    ap_pct_mod: 0.0,
                },
            ],
            |spell_id| spell_id == 100,
        );

        assert_eq!(outcome.loaded_row_count, 1);
        assert_eq!(outcome.errors.len(), 1);
        assert_eq!(outcome.errors[0].row.spell_id, 200);
        assert_eq!(
            outcome
                .store
                .get_spell_threat_entry_like_cpp(100, |_| unreachable!()),
            Some(&SpellThreatEntryLikeCpp {
                flat_mod: 7,
                pct_mod: 1.25,
                ap_pct_mod: 0.5,
            })
        );
    }

    #[test]
    fn spell_threat_store_duplicate_rows_last_wins_like_cpp() {
        let outcome = SpellThreatStoreLikeCpp::from_rows_like_cpp(
            [
                SpellThreatRowLikeCpp {
                    spell_id: 300,
                    flat_mod: 1,
                    pct_mod: 1.0,
                    ap_pct_mod: 0.0,
                },
                SpellThreatRowLikeCpp {
                    spell_id: 300,
                    flat_mod: -4,
                    pct_mod: 0.75,
                    ap_pct_mod: 0.25,
                },
            ],
            |_| true,
        );

        assert_eq!(
            outcome.loaded_row_count, 2,
            "C++ increments count for every valid row before unordered_map overwrite visibility"
        );
        assert!(outcome.errors.is_empty());
        assert_eq!(outcome.store.entries_by_spell_id.len(), 1);
        assert_eq!(
            outcome
                .store
                .get_spell_threat_entry_like_cpp(300, |_| unreachable!()),
            Some(&SpellThreatEntryLikeCpp {
                flat_mod: -4,
                pct_mod: 0.75,
                ap_pct_mod: 0.25,
            })
        );
    }

    #[test]
    fn spell_threat_store_falls_back_to_first_spell_in_chain_like_cpp() {
        let outcome = SpellThreatStoreLikeCpp::from_rows_like_cpp(
            [SpellThreatRowLikeCpp {
                spell_id: 11,
                flat_mod: 40,
                pct_mod: 1.5,
                ap_pct_mod: 0.0,
            }],
            |_| true,
        );

        assert_eq!(
            outcome
                .store
                .get_spell_threat_entry_like_cpp(42, |spell_id| {
                    assert_eq!(spell_id, 42);
                    11
                }),
            Some(&SpellThreatEntryLikeCpp {
                flat_mod: 40,
                pct_mod: 1.5,
                ap_pct_mod: 0.0,
            })
        );
        assert_eq!(
            outcome.store.get_spell_threat_entry_like_cpp(43, |_| 43),
            None
        );
    }

    #[test]
    fn spell_linked_store_skips_missing_trigger_and_effect_like_cpp() {
        let outcome = SpellLinkedStoreLikeCpp::from_rows_like_cpp(
            [
                SpellLinkedRowLikeCpp {
                    spell_trigger: 100,
                    spell_effect: 200,
                    link_type: 0,
                },
                SpellLinkedRowLikeCpp {
                    spell_trigger: 300,
                    spell_effect: 400,
                    link_type: 0,
                },
            ],
            |spell_id| match spell_id {
                100 => Some(SpellLinkedSpellInfoLikeCpp {
                    effect_calc_values_by_index: Vec::new(),
                }),
                _ => None,
            },
        );

        assert_eq!(outcome.loaded_row_count, 0);
        assert_eq!(
            outcome
                .errors
                .iter()
                .map(|error| error.kind)
                .collect::<Vec<_>>(),
            vec![
                SpellLinkedLoadErrorKindLikeCpp::EffectSpellMissing,
                SpellLinkedLoadErrorKindLikeCpp::TriggerSpellMissing,
            ]
        );
        assert!(outcome.store.effects_by_type_and_trigger.is_empty());
    }

    #[test]
    fn spell_linked_store_preserves_signed_effects_and_push_order_like_cpp() {
        let outcome = SpellLinkedStoreLikeCpp::from_rows_like_cpp(
            [
                SpellLinkedRowLikeCpp {
                    spell_trigger: 10,
                    spell_effect: 20,
                    link_type: 1,
                },
                SpellLinkedRowLikeCpp {
                    spell_trigger: 10,
                    spell_effect: -30,
                    link_type: 1,
                },
            ],
            |_| {
                Some(SpellLinkedSpellInfoLikeCpp {
                    effect_calc_values_by_index: Vec::new(),
                })
            },
        );

        assert_eq!(outcome.loaded_row_count, 2);
        assert!(outcome.errors.is_empty());
        assert_eq!(
            outcome
                .store
                .get_spell_linked_like_cpp(SpellLinkedTypeLikeCpp::Hit, 10),
            Some([20, -30].as_slice())
        );
    }

    #[test]
    fn spell_linked_store_negative_trigger_forces_remove_like_cpp() {
        let outcome = SpellLinkedStoreLikeCpp::from_rows_like_cpp(
            [SpellLinkedRowLikeCpp {
                spell_trigger: -50,
                spell_effect: 60,
                link_type: 1,
            }],
            |_| {
                Some(SpellLinkedSpellInfoLikeCpp {
                    effect_calc_values_by_index: Vec::new(),
                })
            },
        );

        assert_eq!(outcome.loaded_row_count, 1);
        assert!(outcome.errors.is_empty());
        assert_eq!(outcome.warnings.len(), 1);
        assert_eq!(
            outcome.warnings[0].kind,
            SpellLinkedLoadWarningKindLikeCpp::NegativeTriggerLinkTypeCoercedToRemove
        );
        assert_eq!(
            outcome
                .store
                .get_spell_linked_like_cpp(SpellLinkedTypeLikeCpp::Remove, 50),
            Some([60].as_slice())
        );
    }

    #[test]
    fn spell_linked_store_invalid_type_and_self_loop_match_cpp() {
        let outcome = SpellLinkedStoreLikeCpp::from_rows_like_cpp(
            [
                SpellLinkedRowLikeCpp {
                    spell_trigger: 10,
                    spell_effect: 10,
                    link_type: 0,
                },
                SpellLinkedRowLikeCpp {
                    spell_trigger: 20,
                    spell_effect: 20,
                    link_type: 2,
                },
                SpellLinkedRowLikeCpp {
                    spell_trigger: 30,
                    spell_effect: 40,
                    link_type: 9,
                },
            ],
            |_| {
                Some(SpellLinkedSpellInfoLikeCpp {
                    effect_calc_values_by_index: Vec::new(),
                })
            },
        );

        assert_eq!(outcome.loaded_row_count, 1);
        assert_eq!(
            outcome
                .errors
                .iter()
                .map(|error| error.kind)
                .collect::<Vec<_>>(),
            vec![
                SpellLinkedLoadErrorKindLikeCpp::SelfTriggerLoop,
                SpellLinkedLoadErrorKindLikeCpp::InvalidLinkType,
            ]
        );
        assert_eq!(
            outcome
                .store
                .get_spell_linked_like_cpp(SpellLinkedTypeLikeCpp::Aura, 20),
            Some([20].as_slice())
        );
    }

    #[test]
    fn spell_linked_store_same_base_point_warning_does_not_skip_like_cpp() {
        let outcome = SpellLinkedStoreLikeCpp::from_rows_like_cpp(
            [SpellLinkedRowLikeCpp {
                spell_trigger: 70,
                spell_effect: 12,
                link_type: 0,
            }],
            |spell_id| {
                if spell_id == 70 {
                    Some(SpellLinkedSpellInfoLikeCpp {
                        effect_calc_values_by_index: vec![(2, 12)],
                    })
                } else {
                    Some(SpellLinkedSpellInfoLikeCpp {
                        effect_calc_values_by_index: Vec::new(),
                    })
                }
            },
        );

        assert_eq!(outcome.loaded_row_count, 1);
        assert!(outcome.errors.is_empty());
        assert_eq!(
            outcome.warnings[0].kind,
            SpellLinkedLoadWarningKindLikeCpp::TriggerEffectSameBasePoint { effect_index: 2 }
        );
        assert_eq!(
            outcome
                .store
                .get_spell_linked_like_cpp(SpellLinkedTypeLikeCpp::Cast, 70),
            Some([12].as_slice())
        );
    }

    #[test]
    fn spell_totem_model_store_skips_missing_dependencies_like_cpp() {
        let outcome = SpellTotemModelStoreLikeCpp::from_rows_like_cpp(
            [
                SpellTotemModelRowLikeCpp {
                    spell_id: 10,
                    race_id: 2,
                    display_id: 100,
                },
                SpellTotemModelRowLikeCpp {
                    spell_id: 20,
                    race_id: 2,
                    display_id: 100,
                },
                SpellTotemModelRowLikeCpp {
                    spell_id: 10,
                    race_id: 3,
                    display_id: 100,
                },
                SpellTotemModelRowLikeCpp {
                    spell_id: 10,
                    race_id: 2,
                    display_id: 200,
                },
            ],
            |spell_id| spell_id == 10,
            |race_id| race_id == 2,
            |display_id| display_id == 100,
        );

        assert_eq!(outcome.loaded_row_count, 1);
        assert_eq!(
            outcome
                .errors
                .iter()
                .map(|error| error.kind)
                .collect::<Vec<_>>(),
            vec![
                SpellTotemModelLoadErrorKindLikeCpp::SpellMissing,
                SpellTotemModelLoadErrorKindLikeCpp::RaceMissing,
                SpellTotemModelLoadErrorKindLikeCpp::DisplayMissing,
            ]
        );
        assert_eq!(outcome.store.get_model_for_totem_like_cpp(10, 2), 100);
        assert_eq!(outcome.store.get_model_for_totem_like_cpp(10, 3), 0);
    }

    #[test]
    fn spell_totem_model_store_duplicate_rows_last_wins_like_cpp() {
        let outcome = SpellTotemModelStoreLikeCpp::from_rows_like_cpp(
            [
                SpellTotemModelRowLikeCpp {
                    spell_id: 50,
                    race_id: 8,
                    display_id: 1000,
                },
                SpellTotemModelRowLikeCpp {
                    spell_id: 50,
                    race_id: 8,
                    display_id: 2000,
                },
            ],
            |_| true,
            |_| true,
            |_| true,
        );

        assert_eq!(
            outcome.loaded_row_count, 2,
            "C++ increments count for every valid row before std::map overwrite visibility"
        );
        assert!(outcome.errors.is_empty());
        assert_eq!(outcome.store.display_id_by_spell_and_race.len(), 1);
        assert_eq!(outcome.store.get_model_for_totem_like_cpp(50, 8), 2000);
        assert_eq!(outcome.store.get_model_for_totem_like_cpp(50, 2), 0);
    }

    #[test]
    fn spell_required_store_skips_missing_and_same_chain_like_cpp() {
        let outcome = SpellRequiredStoreLikeCpp::from_rows_like_cpp(
            [
                SpellRequiredRowLikeCpp {
                    spell_id: 10,
                    req_spell: 20,
                },
                SpellRequiredRowLikeCpp {
                    spell_id: 30,
                    req_spell: 40,
                },
                SpellRequiredRowLikeCpp {
                    spell_id: 50,
                    req_spell: 60,
                },
            ],
            |spell_id| matches!(spell_id, 10 | 20 | 30 | 50 | 60),
            |spell_id, req_spell| spell_id == 50 && req_spell == 60,
        );

        assert_eq!(outcome.loaded_row_count, 1);
        assert_eq!(
            outcome
                .errors
                .iter()
                .map(|error| error.kind)
                .collect::<Vec<_>>(),
            vec![
                SpellRequiredLoadErrorKindLikeCpp::RequiredSpellMissing,
                SpellRequiredLoadErrorKindLikeCpp::SameRankChain,
            ]
        );
        assert_eq!(outcome.store.spells_required_for_spell_like_cpp(10), &[20]);
        assert_eq!(outcome.store.spells_requiring_spell_like_cpp(20), &[10]);
    }

    #[test]
    fn spell_required_store_skips_missing_spell_id_like_cpp() {
        let outcome = SpellRequiredStoreLikeCpp::from_rows_like_cpp(
            [SpellRequiredRowLikeCpp {
                spell_id: 70,
                req_spell: 80,
            }],
            |spell_id| spell_id == 80,
            |_, _| false,
        );

        assert_eq!(outcome.loaded_row_count, 0);
        assert_eq!(outcome.errors.len(), 1);
        assert_eq!(
            outcome.errors[0].kind,
            SpellRequiredLoadErrorKindLikeCpp::SpellMissing
        );
    }

    #[test]
    fn spell_required_store_skips_duplicate_exact_pair_like_cpp() {
        let outcome = SpellRequiredStoreLikeCpp::from_rows_like_cpp(
            [
                SpellRequiredRowLikeCpp {
                    spell_id: 90,
                    req_spell: 100,
                },
                SpellRequiredRowLikeCpp {
                    spell_id: 90,
                    req_spell: 100,
                },
                SpellRequiredRowLikeCpp {
                    spell_id: 91,
                    req_spell: 100,
                },
            ],
            |_| true,
            |_, _| false,
        );

        assert_eq!(outcome.loaded_row_count, 2);
        assert_eq!(outcome.errors.len(), 1);
        assert_eq!(
            outcome.errors[0].kind,
            SpellRequiredLoadErrorKindLikeCpp::Duplicate
        );
        assert!(outcome.store.is_spell_requiring_spell_like_cpp(90, 100));
        assert!(outcome.store.is_spell_requiring_spell_like_cpp(91, 100));
        assert_eq!(outcome.store.spells_required_for_spell_like_cpp(90), &[100]);
        assert_eq!(
            outcome.store.spells_requiring_spell_like_cpp(100),
            &[90, 91]
        );
    }

    fn learn_skill_source(
        spell_id: u32,
        difficulty_none: bool,
        effects: Vec<SpellLearnSkillEffectLikeCpp>,
    ) -> SpellLearnSkillSourceSpellInfoLikeCpp {
        SpellLearnSkillSourceSpellInfoLikeCpp {
            spell_id,
            difficulty_none,
            effects,
        }
    }

    #[test]
    fn spell_learn_skill_store_derives_skill_effect_like_cpp() {
        let outcome = SpellLearnSkillStoreLikeCpp::from_spell_infos_like_cpp([learn_skill_source(
            100,
            true,
            vec![SpellLearnSkillEffectLikeCpp {
                effect: spell_effect_types::SPELL_EFFECT_SKILL,
                misc_value: 755,
                calc_value: 4,
            }],
        )]);

        assert_eq!(outcome.dbc_loaded_row_count, 1);
        assert_eq!(
            outcome.store.get_spell_learn_skill_like_cpp(100),
            Some(&SpellLearnSkillNodeLikeCpp {
                skill: 755,
                step: 4,
                value: 0,
                maxvalue: 0,
            })
        );
    }

    #[test]
    fn spell_learn_skill_store_derives_dual_wield_like_cpp() {
        let outcome = SpellLearnSkillStoreLikeCpp::from_spell_infos_like_cpp([learn_skill_source(
            200,
            true,
            vec![SpellLearnSkillEffectLikeCpp {
                effect: spell_effect_types::SPELL_EFFECT_DUAL_WIELD,
                misc_value: 0,
                calc_value: 0,
            }],
        )]);

        assert_eq!(outcome.dbc_loaded_row_count, 1);
        assert_eq!(
            outcome.store.get_spell_learn_skill_like_cpp(200),
            Some(&SpellLearnSkillNodeLikeCpp {
                skill: SKILL_DUAL_WIELD_LIKE_CPP,
                step: 1,
                value: 1,
                maxvalue: 1,
            })
        );
    }

    #[test]
    fn spell_learn_skill_store_skips_non_base_difficulty_and_breaks_after_first_match_like_cpp() {
        let outcome = SpellLearnSkillStoreLikeCpp::from_spell_infos_like_cpp([
            learn_skill_source(
                300,
                false,
                vec![SpellLearnSkillEffectLikeCpp {
                    effect: spell_effect_types::SPELL_EFFECT_SKILL,
                    misc_value: 333,
                    calc_value: 3,
                }],
            ),
            learn_skill_source(
                301,
                true,
                vec![
                    SpellLearnSkillEffectLikeCpp {
                        effect: spell_effect_types::SPELL_EFFECT_NONE,
                        misc_value: 0,
                        calc_value: 0,
                    },
                    SpellLearnSkillEffectLikeCpp {
                        effect: spell_effect_types::SPELL_EFFECT_DUAL_WIELD,
                        misc_value: 0,
                        calc_value: 0,
                    },
                    SpellLearnSkillEffectLikeCpp {
                        effect: spell_effect_types::SPELL_EFFECT_SKILL,
                        misc_value: 755,
                        calc_value: 8,
                    },
                ],
            ),
        ]);

        assert_eq!(outcome.dbc_loaded_row_count, 1);
        assert!(outcome.store.get_spell_learn_skill_like_cpp(300).is_none());
        assert_eq!(
            outcome.store.get_spell_learn_skill_like_cpp(301),
            Some(&SpellLearnSkillNodeLikeCpp {
                skill: SKILL_DUAL_WIELD_LIKE_CPP,
                step: 1,
                value: 1,
                maxvalue: 1,
            })
        );
    }

    #[test]
    fn spell_chain_store_builds_rank_links_from_skill_line_supercedes_like_cpp() {
        let store = SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [
                SpellRankEdgeLikeCpp {
                    spell_id: 3,
                    supercedes_spell_id: 1,
                },
                SpellRankEdgeLikeCpp {
                    spell_id: 4,
                    supercedes_spell_id: 3,
                },
                SpellRankEdgeLikeCpp {
                    spell_id: 5,
                    supercedes_spell_id: 4,
                },
                SpellRankEdgeLikeCpp {
                    spell_id: 999,
                    supercedes_spell_id: 998,
                },
            ],
            |spell_id| matches!(spell_id, 1 | 3 | 4 | 5),
        );

        assert_eq!(store.chains_by_spell_id.len(), 4);
        assert_eq!(
            store.spell_chain_node_like_cpp(1),
            Some(&SpellChainNodeLikeCpp {
                prev_spell_id: None,
                next_spell_id: Some(3),
                first_spell_id: 1,
                last_spell_id: 5,
                rank: 1,
            })
        );
        assert_eq!(
            store.spell_chain_node_like_cpp(4),
            Some(&SpellChainNodeLikeCpp {
                prev_spell_id: Some(3),
                next_spell_id: Some(5),
                first_spell_id: 1,
                last_spell_id: 5,
                rank: 3,
            })
        );
        assert!(store.spell_chain_node_like_cpp(999).is_none());
    }

    #[test]
    fn spell_chain_store_accessors_match_cpp_fallbacks() {
        let store = SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [
                SpellRankEdgeLikeCpp {
                    spell_id: 20,
                    supercedes_spell_id: 10,
                },
                SpellRankEdgeLikeCpp {
                    spell_id: 30,
                    supercedes_spell_id: 20,
                },
            ],
            |spell_id| matches!(spell_id, 10 | 20 | 30),
        );

        assert_eq!(store.first_spell_in_chain_like_cpp(30), 10);
        assert_eq!(store.last_spell_in_chain_like_cpp(10), 30);
        assert_eq!(store.next_spell_in_chain_like_cpp(10), 20);
        assert_eq!(store.prev_spell_in_chain_like_cpp(30), 20);
        assert_eq!(store.spell_rank_like_cpp(20), 2);
        assert_eq!(store.first_spell_in_chain_like_cpp(99), 99);
        assert_eq!(store.last_spell_in_chain_like_cpp(99), 99);
        assert_eq!(store.next_spell_in_chain_like_cpp(99), 0);
        assert_eq!(store.prev_spell_in_chain_like_cpp(99), 0);
        assert_eq!(store.spell_rank_like_cpp(99), 0);
        assert_eq!(store.spell_with_rank_like_cpp(10, 3, true), 30);
        assert_eq!(store.spell_with_rank_like_cpp(30, 1, true), 10);
        assert_eq!(store.spell_with_rank_like_cpp(99, 2, true), 0);
        assert_eq!(store.spell_with_rank_like_cpp(99, 2, false), 99);
    }

    fn spell_area_row(spell_id: u32) -> SpellAreaRowLikeCpp {
        SpellAreaRowLikeCpp {
            spell_id,
            area_id: 0,
            quest_start: 0,
            quest_start_status: 0,
            quest_end_status: 0,
            quest_end: 0,
            aura_spell: 0,
            race_mask: 0,
            gender: GENDER_NONE_LIKE_CPP,
            flags: 0,
        }
    }

    #[test]
    fn spell_area_store_populates_primary_and_secondary_indices_like_cpp() {
        let mut row = spell_area_row(100);
        row.area_id = 10;
        row.quest_start = 20;
        row.quest_start_status = 1 << 3;
        row.quest_end = 30;
        row.quest_end_status = 1 << 6;
        row.aura_spell = -40;
        row.race_mask = 1;
        row.gender = GENDER_MALE_LIKE_CPP;
        row.flags = SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP;

        let outcome = SpellAreaStoreLikeCpp::from_rows_like_cpp(
            [row],
            |spell_id| matches!(spell_id, 40 | 100),
            |area_id| area_id == 10,
            |quest_id| matches!(quest_id, 20 | 30),
        );

        assert!(outcome.errors.is_empty());
        assert_eq!(outcome.loaded_row_count, 1);
        assert_eq!(outcome.store.spell_area_map_bounds_like_cpp(100).len(), 1);
        assert_eq!(
            outcome
                .store
                .spell_area_for_area_map_bounds_like_cpp(10)
                .len(),
            1
        );
        assert_eq!(
            outcome
                .store
                .spell_area_for_quest_map_bounds_like_cpp(20)
                .len(),
            1
        );
        assert_eq!(
            outcome
                .store
                .spell_area_for_quest_map_bounds_like_cpp(30)
                .len(),
            1
        );
        assert_eq!(
            outcome
                .store
                .spell_area_for_quest_end_map_bounds_like_cpp(30)
                .len(),
            1
        );
        assert_eq!(
            outcome
                .store
                .spell_area_for_aura_map_bounds_like_cpp(40)
                .len(),
            1
        );
        assert_eq!(
            outcome.store.areas_like_cpp()[0],
            SpellAreaLikeCpp {
                spell_id: 100,
                area_id: 10,
                quest_start: 20,
                quest_end: 30,
                aura_spell: -40,
                race_mask: 1,
                gender: GENDER_MALE_LIKE_CPP,
                quest_start_status: 1 << 3,
                quest_end_status: 1 << 6,
                flags: SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP,
            }
        );
    }

    #[test]
    fn spell_area_store_validates_rows_like_cpp() {
        let mut duplicate_first = spell_area_row(100);
        duplicate_first.area_id = 10;
        duplicate_first.quest_start = 20;
        duplicate_first.aura_spell = 40;
        duplicate_first.race_mask = 1;
        duplicate_first.gender = GENDER_FEMALE_LIKE_CPP;

        let duplicate_second = duplicate_first;
        let mut missing_area = spell_area_row(100);
        missing_area.area_id = 999;
        let mut missing_start_quest = spell_area_row(100);
        missing_start_quest.quest_start = 999;
        let mut missing_end_quest = spell_area_row(100);
        missing_end_quest.quest_end = 999;
        let mut missing_aura = spell_area_row(100);
        missing_aura.aura_spell = 999;
        let mut self_aura = spell_area_row(100);
        self_aura.aura_spell = 100;
        let mut invalid_race = spell_area_row(100);
        invalid_race.race_mask = 1_u64 << 62;
        let mut invalid_gender = spell_area_row(100);
        invalid_gender.gender = 3;

        let outcome = SpellAreaStoreLikeCpp::from_rows_like_cpp(
            [
                duplicate_first,
                duplicate_second,
                missing_area,
                missing_start_quest,
                missing_end_quest,
                missing_aura,
                self_aura,
                invalid_race,
                invalid_gender,
            ],
            |spell_id| matches!(spell_id, 40 | 100),
            |area_id| area_id == 10,
            |quest_id| matches!(quest_id, 20 | 30),
        );

        assert_eq!(outcome.loaded_row_count, 1);
        assert_eq!(
            outcome
                .errors
                .iter()
                .map(|error| error.kind)
                .collect::<Vec<_>>(),
            vec![
                SpellAreaLoadErrorKindLikeCpp::DuplicateSimilarRequirements,
                SpellAreaLoadErrorKindLikeCpp::AreaMissing,
                SpellAreaLoadErrorKindLikeCpp::QuestStartMissing,
                SpellAreaLoadErrorKindLikeCpp::QuestEndMissing,
                SpellAreaLoadErrorKindLikeCpp::AuraSpellMissing,
                SpellAreaLoadErrorKindLikeCpp::AuraSpellSelfRequirement,
                SpellAreaLoadErrorKindLikeCpp::InvalidRaceMask,
                SpellAreaLoadErrorKindLikeCpp::InvalidGender,
            ]
        );
    }

    #[test]
    fn spell_area_store_rejects_autocast_aura_chains_like_cpp() {
        let mut aura_to_spell = spell_area_row(200);
        aura_to_spell.aura_spell = 100;
        aura_to_spell.flags = SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP;

        let mut spell_to_aura = spell_area_row(100);
        spell_to_aura.aura_spell = 200;
        spell_to_aura.flags = SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP;

        let outcome = SpellAreaStoreLikeCpp::from_rows_like_cpp(
            [aura_to_spell, spell_to_aura],
            |spell_id| matches!(spell_id, 100 | 200),
            |_| true,
            |_| true,
        );

        assert_eq!(outcome.loaded_row_count, 1);
        assert_eq!(
            outcome.errors,
            vec![SpellAreaLoadErrorLikeCpp {
                row: spell_to_aura,
                kind: SpellAreaLoadErrorKindLikeCpp::AuraAutocastChain,
            }]
        );
    }

    fn custom_attr_source(
        spell_id: u32,
        difficulty: u32,
        effect_type: u32,
    ) -> SpellCustomAttributeSourceSpellInfoLikeCpp {
        SpellCustomAttributeSourceSpellInfoLikeCpp {
            spell_id,
            difficulty,
            effects: vec![SpellEffectInfo {
                effect_index: 0,
                effect: effect_type,
                ..Default::default()
            }],
        }
    }

    #[test]
    fn spell_custom_attribute_store_applies_sql_rows_per_difficulty_like_cpp() {
        let outcome = SpellCustomAttributeStoreLikeCpp::from_sql_rows_like_cpp(
            [
                SpellCustomAttributeRowLikeCpp {
                    spell_id: 100,
                    attributes: SPELL_ATTR0_CU_CAN_CRIT_LIKE_CPP,
                },
                SpellCustomAttributeRowLikeCpp {
                    spell_id: 100,
                    attributes: SPELL_ATTR0_CU_DIRECT_DAMAGE_LIKE_CPP,
                },
            ],
            |spell_id| {
                (spell_id == 100)
                    .then(|| {
                        vec![
                            custom_attr_source(
                                100,
                                0,
                                spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
                            ),
                            custom_attr_source(100, 1, spell_effect_types::SPELL_EFFECT_HEAL),
                        ]
                    })
                    .unwrap_or_default()
            },
        );

        assert!(outcome.errors.is_empty());
        assert_eq!(outcome.loaded_row_count, 2);
        assert_eq!(outcome.applied_variant_count, 4);
        assert_eq!(
            outcome
                .store
                .attributes_for_spell_difficulty_like_cpp(100, 0),
            SPELL_ATTR0_CU_CAN_CRIT_LIKE_CPP | SPELL_ATTR0_CU_DIRECT_DAMAGE_LIKE_CPP
        );
        assert_eq!(
            outcome
                .store
                .attributes_for_spell_difficulty_like_cpp(100, 1),
            SPELL_ATTR0_CU_CAN_CRIT_LIKE_CPP | SPELL_ATTR0_CU_DIRECT_DAMAGE_LIKE_CPP
        );
    }

    #[test]
    fn spell_custom_attribute_store_validates_missing_spell_like_cpp() {
        let outcome = SpellCustomAttributeStoreLikeCpp::from_sql_rows_like_cpp(
            [SpellCustomAttributeRowLikeCpp {
                spell_id: 999,
                attributes: SPELL_ATTR0_CU_CAN_CRIT_LIKE_CPP,
            }],
            |_| Vec::new(),
        );

        assert_eq!(outcome.loaded_row_count, 0);
        assert_eq!(outcome.applied_variant_count, 0);
        assert_eq!(
            outcome.errors,
            vec![SpellCustomAttributeLoadErrorLikeCpp {
                spell_id: 999,
                difficulty: None,
                kind: SpellCustomAttributeLoadErrorKindLikeCpp::SpellMissing,
            }]
        );
    }

    #[test]
    fn spell_custom_attribute_store_rejects_share_damage_without_school_damage_like_cpp() {
        let outcome = SpellCustomAttributeStoreLikeCpp::from_sql_rows_like_cpp(
            [SpellCustomAttributeRowLikeCpp {
                spell_id: 100,
                attributes: SPELL_ATTR0_CU_SHARE_DAMAGE_LIKE_CPP,
            }],
            |spell_id| {
                (spell_id == 100)
                    .then(|| {
                        vec![
                            custom_attr_source(
                                100,
                                0,
                                spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
                            ),
                            custom_attr_source(100, 1, spell_effect_types::SPELL_EFFECT_HEAL),
                        ]
                    })
                    .unwrap_or_default()
            },
        );

        assert_eq!(outcome.loaded_row_count, 1);
        assert_eq!(outcome.applied_variant_count, 1);
        assert_eq!(
            outcome
                .store
                .attributes_for_spell_difficulty_like_cpp(100, 0),
            SPELL_ATTR0_CU_SHARE_DAMAGE_LIKE_CPP
        );
        assert_eq!(
            outcome
                .store
                .attributes_for_spell_difficulty_like_cpp(100, 1),
            0
        );
        assert_eq!(
            outcome.errors,
            vec![SpellCustomAttributeLoadErrorLikeCpp {
                spell_id: 100,
                difficulty: Some(1),
                kind: SpellCustomAttributeLoadErrorKindLikeCpp::ShareDamageWithoutSchoolDamage,
            }]
        );
    }

    #[test]
    fn spell_group_store_validates_rows_like_cpp() {
        let outcome = SpellGroupStoreLikeCpp::from_rows_like_cpp(
            [
                SpellGroupRowLikeCpp {
                    group_id: 5,
                    spell_id: 10,
                },
                SpellGroupRowLikeCpp {
                    group_id: 1001,
                    spell_id: 11,
                },
                SpellGroupRowLikeCpp {
                    group_id: 1002,
                    spell_id: 12,
                },
                SpellGroupRowLikeCpp {
                    group_id: 1003,
                    spell_id: -1999,
                },
            ],
            |spell_id| matches!(spell_id, 12),
            |spell_id| {
                if spell_id == 12 { 2 } else { 1 }
            },
        );

        assert_eq!(outcome.loaded_row_count, 0);
        assert_eq!(
            outcome
                .errors
                .iter()
                .map(|error| error.kind)
                .collect::<Vec<_>>(),
            vec![
                SpellGroupLoadErrorKindLikeCpp::CoreRangeGroupMissing,
                SpellGroupLoadErrorKindLikeCpp::SpellMissing,
                SpellGroupLoadErrorKindLikeCpp::SpellNotFirstRank,
                SpellGroupLoadErrorKindLikeCpp::ReferencedGroupMissing,
            ]
        );
    }

    #[test]
    fn spell_group_store_expands_nested_groups_like_cpp() {
        let outcome = SpellGroupStoreLikeCpp::from_rows_like_cpp(
            [
                SpellGroupRowLikeCpp {
                    group_id: 1001,
                    spell_id: 10,
                },
                SpellGroupRowLikeCpp {
                    group_id: 1001,
                    spell_id: -1002,
                },
                SpellGroupRowLikeCpp {
                    group_id: 1002,
                    spell_id: 20,
                },
                SpellGroupRowLikeCpp {
                    group_id: 1002,
                    spell_id: 20,
                },
                SpellGroupRowLikeCpp {
                    group_id: 1002,
                    spell_id: -1001,
                },
            ],
            |spell_id| matches!(spell_id, 10 | 20),
            |_| 1,
        );

        assert!(outcome.errors.is_empty());
        assert_eq!(
            outcome.store.spell_group_spell_map_bounds_like_cpp(1001),
            &[10, -1002]
        );
        assert_eq!(
            outcome.store.set_of_spells_in_spell_group_like_cpp(1001),
            BTreeSet::from([10, 20])
        );
        assert_eq!(
            outcome.store.set_of_spells_in_spell_group_like_cpp(1002),
            BTreeSet::from([10, 20])
        );
        assert!(
            outcome
                .store
                .is_spell_member_of_spell_group_like_cpp(20, 1001, |spell_id| spell_id)
        );
        assert_eq!(
            outcome
                .store
                .spell_spell_group_map_bounds_like_cpp(25, |_| 20),
            &[1001, 1002],
            "C++ GetSpellSpellGroupMapBounds first normalizes to GetFirstSpellInChain"
        );
    }

    #[test]
    fn spell_group_stack_rule_store_validates_rows_like_cpp() {
        let spell_groups = SpellGroupStoreLikeCpp::from_rows_like_cpp(
            [SpellGroupRowLikeCpp {
                group_id: 1001,
                spell_id: 10,
            }],
            |spell_id| spell_id == 10,
            |_| 1,
        )
        .store;

        let outcome = SpellGroupStackRuleStoreLikeCpp::from_rows_like_cpp(
            [
                SpellGroupStackRuleRowLikeCpp {
                    group_id: 1001,
                    stack_rule: SpellGroupStackRuleLikeCpp::MAX_LIKE_CPP,
                },
                SpellGroupStackRuleRowLikeCpp {
                    group_id: 1999,
                    stack_rule: SpellGroupStackRuleLikeCpp::Exclusive as u8,
                },
                SpellGroupStackRuleRowLikeCpp {
                    group_id: 1001,
                    stack_rule: SpellGroupStackRuleLikeCpp::Exclusive as u8,
                },
            ],
            &spell_groups,
            |_| None,
            |_| None,
        );

        assert_eq!(outcome.loaded_row_count, 1);
        assert_eq!(
            outcome
                .errors
                .iter()
                .map(|error| error.kind)
                .collect::<Vec<_>>(),
            vec![
                SpellGroupStackRuleLoadErrorKindLikeCpp::StackRuleMissing,
                SpellGroupStackRuleLoadErrorKindLikeCpp::GroupMissing,
            ]
        );
        assert_eq!(
            outcome.store.spell_group_stack_rule_like_cpp(1001),
            SpellGroupStackRuleLikeCpp::Exclusive
        );
        assert_eq!(
            outcome.store.spell_group_stack_rule_like_cpp(1999),
            SpellGroupStackRuleLikeCpp::Default
        );
    }

    #[test]
    fn spell_group_stack_rule_store_infers_same_effect_aura_group_like_cpp() {
        let spell_groups = SpellGroupStoreLikeCpp::from_rows_like_cpp(
            [
                SpellGroupRowLikeCpp {
                    group_id: 1001,
                    spell_id: 10,
                },
                SpellGroupRowLikeCpp {
                    group_id: 1001,
                    spell_id: 20,
                },
                SpellGroupRowLikeCpp {
                    group_id: 1001,
                    spell_id: 30,
                },
            ],
            |spell_id| matches!(spell_id, 10 | 20 | 30),
            |_| 1,
        )
        .store;
        let spells = BTreeMap::from([
            (
                10,
                test_spell_info_with_aura(10, aura_types::SPELL_AURA_MOD_MELEE_HASTE),
            ),
            (
                20,
                test_spell_info_with_aura(20, aura_types::SPELL_AURA_MOD_MELEE_RANGED_HASTE),
            ),
            (30, test_spell_info_without_aura(30)),
            (
                31,
                test_spell_info_with_aura(31, aura_types::SPELL_AURA_MOD_RANGED_HASTE),
            ),
        ]);

        let outcome = SpellGroupStackRuleStoreLikeCpp::from_rows_like_cpp(
            [SpellGroupStackRuleRowLikeCpp {
                group_id: 1001,
                stack_rule: SpellGroupStackRuleLikeCpp::ExclusiveSameEffect as u8,
            }],
            &spell_groups,
            |spell_id| spells.get(&spell_id).cloned(),
            |spell_id| if spell_id == 30 { Some(31) } else { None },
        );

        assert!(outcome.errors.is_empty());
        assert_eq!(outcome.loaded_row_count, 1);
        assert_eq!(outcome.same_effect_parsed_count, 1);
        assert_eq!(
            outcome
                .store
                .same_effect_stack_rule_aura_types_like_cpp(1001),
            Some(&BTreeSet::from([
                aura_types::SPELL_AURA_MOD_MELEE_HASTE,
                aura_types::SPELL_AURA_MOD_MELEE_RANGED_HASTE,
                aura_types::SPELL_AURA_MOD_RANGED_HASTE,
            ])),
            "C++ collapses the melee/ranged haste subgroup to its first aura before expanding it back"
        );
    }

    #[test]
    fn spell_group_stack_rule_store_checks_common_group_rules_like_cpp() {
        let spell_groups = SpellGroupStoreLikeCpp::from_rows_like_cpp(
            [
                SpellGroupRowLikeCpp {
                    group_id: 1001,
                    spell_id: 10,
                },
                SpellGroupRowLikeCpp {
                    group_id: 1001,
                    spell_id: 20,
                },
                SpellGroupRowLikeCpp {
                    group_id: 1002,
                    spell_id: 30,
                },
            ],
            |spell_id| matches!(spell_id, 10 | 20 | 30),
            |_| 1,
        )
        .store;

        let outcome = SpellGroupStackRuleStoreLikeCpp::from_rows_like_cpp(
            [SpellGroupStackRuleRowLikeCpp {
                group_id: 1001,
                stack_rule: SpellGroupStackRuleLikeCpp::ExclusiveHighest as u8,
            }],
            &spell_groups,
            |_| None,
            |_| None,
        );

        assert_eq!(
            outcome
                .store
                .check_spell_group_stack_rules_like_cpp(&spell_groups, 10, 20),
            SpellGroupStackRuleLikeCpp::ExclusiveHighest
        );
        assert_eq!(
            outcome
                .store
                .check_spell_group_stack_rules_like_cpp(&spell_groups, 10, 30),
            SpellGroupStackRuleLikeCpp::Default
        );
    }

    #[test]
    fn spell_proc_store_expands_negative_spell_id_to_all_ranks_like_cpp() {
        let outcome = SpellProcStoreLikeCpp::from_rows_like_cpp(
            [SpellProcRowLikeCpp {
                spell_id: -100,
                proc_flags: [PROC_FLAG_DEAL_MELEE_SWING_LIKE_CPP, 0],
                chance: 25.0,
                ..test_spell_proc_row_like_cpp(100)
            }],
            |spell_id| {
                Some(match spell_id {
                    100 => test_spell_proc_source_like_cpp(100, 100, Some(101)),
                    101 => test_spell_proc_source_like_cpp(101, 100, None),
                    _ => return None,
                })
            },
        );

        assert!(outcome.errors.is_empty());
        assert_eq!(outcome.loaded_row_count, 1);
        assert_eq!(
            outcome
                .store
                .spell_proc_entry_like_cpp(100, 0)
                .map(|entry| entry.chance),
            Some(25.0)
        );
        assert_eq!(
            outcome
                .store
                .spell_proc_entry_like_cpp(101, 0)
                .map(|entry| entry.proc_flags),
            Some([PROC_FLAG_DEAL_MELEE_SWING_LIKE_CPP, 0])
        );
    }

    #[test]
    fn spell_proc_store_applies_spellinfo_defaults_like_cpp() {
        let outcome = SpellProcStoreLikeCpp::from_rows_like_cpp(
            [SpellProcRowLikeCpp {
                spell_id: 200,
                ..test_spell_proc_row_like_cpp(200)
            }],
            |spell_id| {
                let mut source = test_spell_proc_source_like_cpp(spell_id, spell_id, None);
                source.proc_flags = [PROC_FLAG_TAKE_MELEE_SWING_LIKE_CPP, 0];
                source.proc_charges = 3;
                source.proc_chance = 12.5;
                source.proc_cooldown_ms = 1500;
                Some(source)
            },
        );

        let entry = outcome.store.spell_proc_entry_like_cpp(200, 0).unwrap();
        assert_eq!(entry.proc_flags, [PROC_FLAG_TAKE_MELEE_SWING_LIKE_CPP, 0]);
        assert_eq!(entry.charges, 3);
        assert_eq!(entry.chance, 12.5);
        assert_eq!(entry.cooldown_ms, 1500);
    }

    #[test]
    fn spell_proc_store_validates_and_sanitizes_like_cpp() {
        let outcome = SpellProcStoreLikeCpp::from_rows_like_cpp(
            [SpellProcRowLikeCpp {
                spell_id: 300,
                school_mask: 0x80,
                proc_flags: [0, PROC_FLAG_2_CAST_SUCCESSFUL_LIKE_CPP],
                spell_type_mask: PROC_SPELL_TYPE_MASK_ALL_LIKE_CPP << 1,
                spell_phase_mask: PROC_SPELL_PHASE_MASK_ALL_LIKE_CPP << 1,
                hit_mask: PROC_HIT_MASK_ALL_LIKE_CPP << 1,
                attributes_mask: PROC_ATTR_ALL_ALLOWED_LIKE_CPP | 0x0000_0100,
                disable_effects_mask: 0x1,
                procs_per_minute: -1.0,
                chance: -1.0,
                ..test_spell_proc_row_like_cpp(300)
            }],
            |spell_id| {
                let mut source = test_spell_proc_source_like_cpp(spell_id, spell_id, None);
                source.effects = vec![SpellEffectInfo {
                    effect_index: 0,
                    effect: spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
                    effect_aura: 0,
                    ..SpellEffectInfo::default()
                }];
                Some(source)
            },
        );

        let entry = outcome.store.spell_proc_entry_like_cpp(300, 0).unwrap();
        assert_eq!(entry.chance, 0.0);
        assert_eq!(entry.procs_per_minute, 0.0);
        assert_eq!(entry.attributes_mask, PROC_ATTR_ALL_ALLOWED_LIKE_CPP);
        assert_eq!(
            outcome
                .errors
                .iter()
                .map(|error| error.kind)
                .collect::<Vec<_>>(),
            vec![
                SpellProcLoadErrorKindLikeCpp::InvalidSchoolMask,
                SpellProcLoadErrorKindLikeCpp::NegativeChance,
                SpellProcLoadErrorKindLikeCpp::NegativeProcsPerMinute,
                SpellProcLoadErrorKindLikeCpp::InvalidSpellTypeMask,
                SpellProcLoadErrorKindLikeCpp::SpellTypeMaskUnused,
                SpellProcLoadErrorKindLikeCpp::InvalidSpellPhaseMask,
                SpellProcLoadErrorKindLikeCpp::SpellPhaseMaskUnused,
                SpellProcLoadErrorKindLikeCpp::InvalidHitMask,
                SpellProcLoadErrorKindLikeCpp::HitMaskUnused,
                SpellProcLoadErrorKindLikeCpp::DisabledEffectIsNotAura,
                SpellProcLoadErrorKindLikeCpp::ReqSpellmodWithoutSpellmodAura,
                SpellProcLoadErrorKindLikeCpp::InvalidAttributesMask,
            ]
        );
    }

    #[test]
    fn spell_proc_store_lookup_uses_exact_difficulty_before_fallback_like_cpp() {
        let store = test_spell_proc_store_with_entries_like_cpp([
            (400, 1, [PROC_FLAG_DEATH_LIKE_CPP, 0]),
            (400, 2, [PROC_FLAG_KILL_LIKE_CPP, 0]),
        ]);

        let entry = store
            .spell_proc_entry_with_fallback_like_cpp(400, 2, |_| Some(1))
            .unwrap();

        assert_eq!(entry.proc_flags, [PROC_FLAG_KILL_LIKE_CPP, 0]);
    }

    #[test]
    fn spell_proc_store_lookup_walks_difficulty_fallback_chain_like_cpp() {
        let store =
            test_spell_proc_store_with_entries_like_cpp([(500, 1, [PROC_FLAG_DEATH_LIKE_CPP, 0])]);

        let entry = store
            .spell_proc_entry_with_fallback_like_cpp(500, 3, |difficulty| match difficulty {
                3 => Some(2),
                2 => Some(1),
                _ => None,
            })
            .unwrap();

        assert_eq!(entry.proc_flags, [PROC_FLAG_DEATH_LIKE_CPP, 0]);
        assert!(
            store
                .spell_proc_entry_with_fallback_like_cpp(500, 3, |_| None)
                .is_none(),
            "C++ stops when sDifficultyStore.LookupEntry returns null"
        );
    }

    #[test]
    fn spell_proc_store_generates_implicit_entries_after_sql_like_cpp() {
        let mut implicit = test_implicit_spell_proc_source_like_cpp();
        implicit.spell_id = 601;
        implicit.proc_flags = [PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP, 0];
        implicit.proc_chance = 35.0;
        implicit.effects = vec![test_implicit_proc_effect_like_cpp(
            0,
            aura_types::SPELL_AURA_PROC_TRIGGER_SPELL,
            [0, 0, 0, 0],
        )];

        let outcome = SpellProcStoreLikeCpp::from_rows_and_implicit_sources_like_cpp(
            [SpellProcRowLikeCpp {
                spell_id: 600,
                proc_flags: [PROC_FLAG_KILL_LIKE_CPP, 0],
                chance: 10.0,
                ..test_spell_proc_row_like_cpp(600)
            }],
            |spell_id| Some(test_spell_proc_source_like_cpp(spell_id, spell_id, None)),
            [implicit],
        );

        assert!(outcome.errors.is_empty());
        assert_eq!(outcome.loaded_row_count, 1);
        assert_eq!(outcome.generated_entry_count, 1);
        assert_eq!(
            outcome
                .store
                .spell_proc_entry_like_cpp(600, 0)
                .map(|entry| (entry.proc_flags, entry.chance)),
            Some(([PROC_FLAG_KILL_LIKE_CPP, 0], 10.0))
        );
        assert_eq!(
            outcome
                .store
                .spell_proc_entry_like_cpp(601, 0)
                .map(|entry| (entry.proc_flags, entry.chance)),
            Some(([PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP, 0], 35.0))
        );
    }

    #[test]
    fn spell_proc_store_explicit_sql_suppresses_same_key_implicit_like_cpp() {
        let mut duplicate_implicit = test_implicit_spell_proc_source_like_cpp();
        duplicate_implicit.spell_id = 700;
        duplicate_implicit.proc_flags = [PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP, 0];
        duplicate_implicit.proc_chance = 90.0;
        duplicate_implicit.effects = vec![test_implicit_proc_effect_like_cpp(
            0,
            aura_types::SPELL_AURA_PROC_TRIGGER_SPELL,
            [0, 0, 0, 0],
        )];

        let mut invalid_implicit = duplicate_implicit.clone();
        invalid_implicit.spell_id = 701;
        invalid_implicit.proc_flags = [0, 0];

        let outcome = SpellProcStoreLikeCpp::from_rows_and_implicit_sources_like_cpp(
            [SpellProcRowLikeCpp {
                spell_id: 700,
                proc_flags: [PROC_FLAG_KILL_LIKE_CPP, 0],
                chance: 11.0,
                ..test_spell_proc_row_like_cpp(700)
            }],
            |spell_id| Some(test_spell_proc_source_like_cpp(spell_id, spell_id, None)),
            [duplicate_implicit, invalid_implicit],
        );

        assert!(outcome.errors.is_empty());
        assert_eq!(outcome.loaded_row_count, 1);
        assert_eq!(outcome.generated_entry_count, 0);
        assert_eq!(
            outcome
                .store
                .spell_proc_entry_like_cpp(700, 0)
                .map(|entry| (entry.proc_flags, entry.chance)),
            Some(([PROC_FLAG_KILL_LIKE_CPP, 0], 11.0))
        );
        assert!(outcome.store.spell_proc_entry_like_cpp(701, 0).is_none());
    }

    #[test]
    fn spell_proc_source_builds_implicit_source_from_spell_effects_like_cpp() {
        let mut source = test_spell_proc_source_like_cpp(800, 800, None);
        source.spell_family_name = 42;
        source.proc_flags = [PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP, 0];
        source.proc_chance = 30.0;
        source.proc_cooldown_ms = 500;
        source.proc_charges = 2;
        source.proc_base_ppm = 1.5;
        source.attributes3 = attributes::SPELL_ATTR3_CAN_PROC_FROM_PROCS;
        source.effects = vec![SpellEffectInfo {
            effect_index: 1,
            effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_aura: aura_types::SPELL_AURA_PROC_TRIGGER_SPELL,
            effect_base_points: -100,
            effect_spell_class_mask: [1, 2, 3, 4],
            effect_trigger_spell: 900,
            ..SpellEffectInfo::default()
        }];

        let implicit = source.implicit_proc_source_like_cpp();

        assert_eq!(implicit.spell_id, 800);
        assert_eq!(implicit.difficulty, 0);
        assert_eq!(implicit.spell_family_name, 42);
        assert_eq!(implicit.proc_flags, source.proc_flags);
        assert_eq!(implicit.proc_chance, 30.0);
        assert_eq!(implicit.proc_cooldown_ms, 500);
        assert_eq!(implicit.proc_charges, 2);
        assert_eq!(implicit.proc_base_ppm, 1.5);
        assert_eq!(
            implicit.attributes3,
            attributes::SPELL_ATTR3_CAN_PROC_FROM_PROCS
        );
        assert_eq!(implicit.effects.len(), 1);
        assert_eq!(implicit.effects[0].effect_index, 1);
        assert!(implicit.effects[0].is_effect);
        assert!(implicit.effects[0].is_aura);
        assert_eq!(
            implicit.effects[0].aura_type,
            aura_types::SPELL_AURA_PROC_TRIGGER_SPELL
        );
        assert_eq!(implicit.effects[0].spell_class_mask, [1, 2, 3, 4]);
        assert_eq!(implicit.effects[0].calc_value, -100);
        assert_eq!(implicit.effects[0].trigger_spell, 900);
    }

    #[test]
    fn spell_proc_source_builds_from_loaded_spell_and_db2_stores_like_cpp() {
        let mut spells = SpellStore::new();
        spells.insert(
            100,
            SpellInfo {
                spell_id: 100,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: 0,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(aura_types::SPELL_AURA_PROC_TRIGGER_SPELL),
                display_flags: 0,
                requires_spell_focus: 0,
                effects: vec![SpellEffectInfo {
                    effect_index: 0,
                    effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: aura_types::SPELL_AURA_PROC_TRIGGER_SPELL,
                    effect_spell_class_mask: [10, 20, 30, 40],
                    ..Default::default()
                }],
            },
        );
        spells.insert(101, test_spell_info_without_aura(101));

        let chains = SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [SpellRankEdgeLikeCpp {
                spell_id: 101,
                supercedes_spell_id: 100,
            }],
            |spell_id| spells.get(spell_id as i32).is_some(),
        );
        let aura_options = crate::spell_db2::SpellAuraOptionsStore::from_entries([
            test_spell_aura_options_entry_like_cpp(1, 100, 0, [1, 0], 10, 2, 300, 9),
            test_spell_aura_options_entry_like_cpp(2, 100, 1, [-1, 7], 35, -2, -300, 42),
        ]);
        let misc = crate::spell_db2::SpellMiscStore::from_entries([
            test_spell_misc_entry_like_cpp(1, 100, 0, 0x0100),
            test_spell_misc_entry_like_cpp(2, 100, 1, attributes::SPELL_ATTR3_CAN_PROC_FROM_PROCS),
        ]);
        let class_options = crate::spell_db2::SpellClassOptionsStore::from_entries([
            crate::spell_db2::SpellClassOptionsEntry {
                id: 1,
                spell_id: 100,
                modal_next_spell: 0,
                spell_class_set: 8,
                spell_class_mask: [10, 20, 30, 40],
            },
        ]);
        let ppm = crate::spell_db2::SpellProcsPerMinuteStore::from_entries([
            crate::spell_db2::SpellProcsPerMinuteEntry {
                id: 42,
                base_proc_rate: 1.75,
                flags: 0,
            },
        ]);

        let source = SpellProcSourceSpellInfoLikeCpp::from_loaded_spell_like_cpp(
            100,
            1,
            &spells,
            &chains,
            &aura_options,
            &misc,
            &class_options,
            &ppm,
        )
        .unwrap();

        assert_eq!(source.spell_id, 100);
        assert_eq!(source.difficulty, 1);
        assert_eq!(source.first_rank_spell_id, 100);
        assert_eq!(source.next_rank_spell_id, Some(101));
        assert_eq!(source.spell_family_name, 8);
        assert_eq!(source.proc_flags, [u32::MAX, 7]);
        assert_eq!(source.proc_chance, 35.0);
        assert_eq!(source.proc_charges, u32::MAX - 1);
        assert_eq!(source.proc_cooldown_ms, (-300_i32) as u32);
        assert_eq!(source.proc_base_ppm, 1.75);
        assert_eq!(
            source.attributes3,
            attributes::SPELL_ATTR3_CAN_PROC_FROM_PROCS
        );
        assert_eq!(source.effects.len(), 1);
        assert_eq!(source.effects[0].effect_spell_class_mask, [10, 20, 30, 40]);

        let fallback_source = SpellProcSourceSpellInfoLikeCpp::from_loaded_spell_like_cpp(
            100,
            2,
            &spells,
            &chains,
            &aura_options,
            &misc,
            &class_options,
            &ppm,
        )
        .unwrap();
        assert_eq!(fallback_source.proc_flags, [1, 0]);
        assert_eq!(fallback_source.attributes3, 0x0100);
    }

    #[test]
    fn spell_proc_store_generates_from_spell_infos_after_sql_like_cpp() {
        let mut generated = test_spell_proc_source_like_cpp(901, 901, None);
        generated.proc_flags = [PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP, 0];
        generated.proc_chance = 45.0;
        generated.effects = vec![SpellEffectInfo {
            effect_index: 0,
            effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_aura: aura_types::SPELL_AURA_PROC_TRIGGER_SPELL,
            ..SpellEffectInfo::default()
        }];

        let mut explicit_duplicate = generated.clone();
        explicit_duplicate.spell_id = 900;
        explicit_duplicate.proc_chance = 95.0;

        let outcome = SpellProcStoreLikeCpp::from_rows_and_spell_infos_like_cpp(
            [SpellProcRowLikeCpp {
                spell_id: 900,
                proc_flags: [PROC_FLAG_KILL_LIKE_CPP, 0],
                chance: 12.0,
                ..test_spell_proc_row_like_cpp(900)
            }],
            |spell_id| Some(test_spell_proc_source_like_cpp(spell_id, spell_id, None)),
            [explicit_duplicate, generated],
        );

        assert!(outcome.errors.is_empty());
        assert_eq!(outcome.loaded_row_count, 1);
        assert_eq!(outcome.generated_entry_count, 1);
        assert_eq!(
            outcome
                .store
                .spell_proc_entry_like_cpp(900, 0)
                .map(|entry| (entry.proc_flags, entry.chance)),
            Some(([PROC_FLAG_KILL_LIKE_CPP, 0], 12.0))
        );
        assert_eq!(
            outcome
                .store
                .spell_proc_entry_like_cpp(901, 0)
                .map(|entry| (entry.proc_flags, entry.chance)),
            Some(([PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP, 0], 45.0))
        );
    }

    #[test]
    fn can_spell_trigger_proc_on_event_requires_proc_flag_overlap_like_cpp() {
        let mut entry = test_spell_proc_entry_like_cpp();
        entry.proc_flags = [0, PROC_FLAG_2_CAST_SUCCESSFUL_LIKE_CPP];
        let mut event = test_spell_proc_event_like_cpp(PROC_FLAG_DEAL_MELEE_SWING_LIKE_CPP);

        assert!(!can_spell_trigger_proc_on_event_like_cpp(&entry, &event));

        event.type_mask = [0, PROC_FLAG_2_CAST_SUCCESSFUL_LIKE_CPP];
        assert!(can_spell_trigger_proc_on_event_like_cpp(&entry, &event));
    }

    #[test]
    fn can_spell_trigger_proc_on_event_checks_xp_honor_and_power_attrs_like_cpp() {
        let mut entry = test_spell_proc_entry_like_cpp();
        entry.proc_flags = [PROC_FLAG_KILL_LIKE_CPP, 0];
        entry.attributes_mask = PROC_ATTR_REQ_EXP_OR_HONOR_LIKE_CPP;
        let mut event = test_spell_proc_event_like_cpp(PROC_FLAG_KILL_LIKE_CPP);
        event.actor_is_player = true;
        event.action_target_exists = true;
        event.action_target_is_honor_or_xp = false;

        assert!(!can_spell_trigger_proc_on_event_like_cpp(&entry, &event));

        event.action_target_is_honor_or_xp = true;
        assert!(can_spell_trigger_proc_on_event_like_cpp(&entry, &event));

        entry.attributes_mask = PROC_ATTR_REQ_POWER_COST_LIKE_CPP;
        event.proc_spell_has_positive_power_cost = None;
        assert!(!can_spell_trigger_proc_on_event_like_cpp(&entry, &event));

        event.proc_spell_has_positive_power_cost = Some(false);
        assert!(!can_spell_trigger_proc_on_event_like_cpp(&entry, &event));

        event.proc_spell_has_positive_power_cost = Some(true);
        assert!(can_spell_trigger_proc_on_event_like_cpp(&entry, &event));
    }

    #[test]
    fn can_spell_trigger_proc_on_event_heartbeat_bypasses_later_masks_like_cpp() {
        let mut entry = test_spell_proc_entry_like_cpp();
        entry.proc_flags = [PROC_FLAG_HEARTBEAT_LIKE_CPP, 0];
        entry.school_mask = 0x04;
        entry.spell_family_name = 7;
        entry.spell_family_mask = [0x10, 0, 0, 0];
        entry.spell_phase_mask = PROC_SPELL_PHASE_HIT_LIKE_CPP;
        entry.hit_mask = PROC_HIT_CRITICAL_LIKE_CPP;
        let mut event = test_spell_proc_event_like_cpp(PROC_FLAG_HEARTBEAT_LIKE_CPP);
        event.school_mask = 0x01;
        event.spell_info = Some(SpellProcEventSpellInfoLikeCpp {
            spell_family_name: 8,
            spell_family_mask: [0, 0, 0, 0],
        });
        event.spell_phase_mask = PROC_SPELL_PHASE_CAST_LIKE_CPP;
        event.hit_mask = PROC_HIT_NORMAL_LIKE_CPP;

        assert!(can_spell_trigger_proc_on_event_like_cpp(&entry, &event));
    }

    #[test]
    fn can_spell_trigger_proc_on_event_matches_school_family_and_type_like_cpp() {
        let mut entry = test_spell_proc_entry_like_cpp();
        entry.proc_flags = [PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP, 0];
        entry.school_mask = 0x04;
        entry.spell_family_name = 11;
        entry.spell_family_mask = [0x20, 0, 0, 0];
        entry.spell_type_mask = PROC_SPELL_TYPE_DAMAGE_LIKE_CPP;
        entry.spell_phase_mask = PROC_SPELL_PHASE_HIT_LIKE_CPP;
        let mut event = test_spell_proc_event_like_cpp(PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP);
        event.school_mask = 0x01;
        event.spell_info = Some(SpellProcEventSpellInfoLikeCpp {
            spell_family_name: 11,
            spell_family_mask: [0x20, 0, 0, 0],
        });
        event.spell_type_mask = PROC_SPELL_TYPE_DAMAGE_LIKE_CPP;
        event.spell_phase_mask = PROC_SPELL_PHASE_HIT_LIKE_CPP;
        event.hit_mask = PROC_HIT_NORMAL_LIKE_CPP;

        assert!(!can_spell_trigger_proc_on_event_like_cpp(&entry, &event));

        event.school_mask = 0x04;
        assert!(can_spell_trigger_proc_on_event_like_cpp(&entry, &event));

        event.spell_info = Some(SpellProcEventSpellInfoLikeCpp {
            spell_family_name: 12,
            spell_family_mask: [0x20, 0, 0, 0],
        });
        assert!(!can_spell_trigger_proc_on_event_like_cpp(&entry, &event));

        event.spell_info = None;
        assert!(
            can_spell_trigger_proc_on_event_like_cpp(&entry, &event),
            "C++ only checks SpellInfo::IsAffected when eventInfo.GetSpellInfo() exists"
        );

        event.spell_type_mask = PROC_SPELL_TYPE_HEAL_LIKE_CPP;
        assert!(!can_spell_trigger_proc_on_event_like_cpp(&entry, &event));
    }

    #[test]
    fn can_spell_trigger_proc_on_event_matches_phase_and_hit_defaults_like_cpp() {
        let mut entry = test_spell_proc_entry_like_cpp();
        entry.proc_flags = [PROC_FLAG_TAKE_MELEE_SWING_LIKE_CPP, 0];
        entry.spell_phase_mask = PROC_SPELL_PHASE_HIT_LIKE_CPP;
        entry.hit_mask = 0;
        let mut event = test_spell_proc_event_like_cpp(PROC_FLAG_TAKE_MELEE_SWING_LIKE_CPP);
        event.spell_phase_mask = 0;
        event.hit_mask = PROC_HIT_ABSORB_LIKE_CPP;

        assert!(!can_spell_trigger_proc_on_event_like_cpp(&entry, &event));

        event.hit_mask = PROC_HIT_CRITICAL_LIKE_CPP;
        assert!(can_spell_trigger_proc_on_event_like_cpp(&entry, &event));

        entry.proc_flags = [PROC_FLAG_DEAL_MELEE_SWING_LIKE_CPP, 0];
        event.type_mask = [PROC_FLAG_DEAL_MELEE_SWING_LIKE_CPP, 0];
        event.hit_mask = PROC_HIT_ABSORB_LIKE_CPP;
        assert!(can_spell_trigger_proc_on_event_like_cpp(&entry, &event));

        event.spell_phase_mask = PROC_SPELL_PHASE_CAST_LIKE_CPP;
        event.hit_mask = 0;
        assert!(
            can_spell_trigger_proc_on_event_like_cpp(&entry, &event),
            "C++ skips done-hit HitMask checks during PROC_SPELL_PHASE_CAST"
        );
    }

    #[test]
    fn spell_proc_event_spell_info_is_affected_matches_cpp_zero_family_name() {
        let event_spell = SpellProcEventSpellInfoLikeCpp {
            spell_family_name: 3,
            spell_family_mask: [0, 0, 0, 0],
        };

        assert!(event_spell.is_affected_like_cpp(0, [0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF]));
    }

    #[test]
    fn implicit_proc_aura_info_matches_cpp_trigger_table() {
        assert_eq!(
            implicit_proc_aura_info_like_cpp(aura_types::SPELL_AURA_DUMMY),
            Some(ImplicitProcAuraInfoLikeCpp {
                spell_type_mask: PROC_SPELL_TYPE_MASK_ALL_LIKE_CPP,
                triggered_can_proc: false,
            })
        );
        assert_eq!(
            implicit_proc_aura_info_like_cpp(aura_types::SPELL_AURA_SCHOOL_ABSORB),
            Some(ImplicitProcAuraInfoLikeCpp {
                spell_type_mask: PROC_SPELL_TYPE_MASK_ALL_LIKE_CPP,
                triggered_can_proc: true,
            })
        );
        assert_eq!(
            implicit_proc_aura_info_like_cpp(aura_types::SPELL_AURA_MOD_STEALTH),
            Some(ImplicitProcAuraInfoLikeCpp {
                spell_type_mask: PROC_SPELL_TYPE_DAMAGE_LIKE_CPP
                    | PROC_SPELL_TYPE_NO_DMG_HEAL_LIKE_CPP,
                triggered_can_proc: true,
            })
        );
        assert_eq!(
            implicit_proc_aura_info_like_cpp(aura_types::SPELL_AURA_MOD_CONFUSE),
            Some(ImplicitProcAuraInfoLikeCpp {
                spell_type_mask: PROC_SPELL_TYPE_DAMAGE_LIKE_CPP,
                triggered_can_proc: true,
            })
        );
        assert_eq!(
            implicit_proc_aura_info_like_cpp(aura_types::SPELL_AURA_MOUNTED),
            None
        );
    }

    #[test]
    fn implicit_spell_proc_entry_matches_cpp_default_generation() {
        let mut source = test_implicit_spell_proc_source_like_cpp();
        source.proc_flags = [
            PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP | PROC_FLAG_KILL_LIKE_CPP,
            0,
        ];
        source.spell_family_name = 42;
        source.proc_chance = 25.0;
        source.proc_cooldown_ms = 1500;
        source.proc_charges = 3;
        source.effects = vec![
            test_implicit_proc_effect_like_cpp(
                0,
                aura_types::SPELL_AURA_PROC_TRIGGER_SPELL,
                [0x10, 0, 0, 0],
            ),
            test_implicit_proc_effect_like_cpp(1, aura_types::SPELL_AURA_MOUNTED, [0, 0, 0, 0]),
        ];

        let entry = implicit_spell_proc_entry_like_cpp(&source).unwrap();

        assert_eq!(entry.proc_flags, source.proc_flags);
        assert_eq!(entry.spell_family_name, 42);
        assert_eq!(entry.spell_family_mask, [0x10, 0, 0, 0]);
        assert_eq!(entry.spell_type_mask, PROC_SPELL_TYPE_MASK_ALL_LIKE_CPP);
        assert_eq!(entry.spell_phase_mask, PROC_SPELL_PHASE_HIT_LIKE_CPP);
        assert_eq!(entry.disable_effects_mask, 1 << 1);
        assert_eq!(entry.attributes_mask, PROC_ATTR_REQ_EXP_OR_HONOR_LIKE_CPP);
        assert_eq!(entry.chance, 25.0);
        assert_eq!(entry.cooldown_ms, 1500);
        assert_eq!(entry.charges, 3);
    }

    #[test]
    fn implicit_spell_proc_entry_sets_special_phase_and_hit_masks_like_cpp() {
        let mut source = test_implicit_spell_proc_source_like_cpp();
        source.proc_flags = [
            PROC_FLAG_DEAL_MELEE_SWING_LIKE_CPP,
            PROC_FLAG_2_CAST_SUCCESSFUL_LIKE_CPP,
        ];
        source.effects = vec![test_implicit_proc_effect_like_cpp(
            0,
            aura_types::SPELL_AURA_MOD_BLOCK_PERCENT,
            [0, 0, 0, 0],
        )];

        let entry = implicit_spell_proc_entry_like_cpp(&source).unwrap();

        assert_eq!(entry.spell_phase_mask, PROC_SPELL_PHASE_CAST_LIKE_CPP);
        assert_eq!(entry.hit_mask, PROC_HIT_BLOCK_LIKE_CPP);

        source.effects = vec![test_implicit_proc_effect_like_cpp(
            0,
            aura_types::SPELL_AURA_REFLECT_SPELLS,
            [0, 0, 0, 0],
        )];
        assert_eq!(
            implicit_spell_proc_entry_like_cpp(&source)
                .unwrap()
                .hit_mask,
            PROC_HIT_REFLECT_LIKE_CPP
        );

        source.effects = vec![test_implicit_proc_effect_with_calc_like_cpp(
            0,
            aura_types::SPELL_AURA_MOD_HIT_CHANCE,
            -100,
        )];
        assert_eq!(
            implicit_spell_proc_entry_like_cpp(&source)
                .unwrap()
                .hit_mask,
            PROC_HIT_MISS_LIKE_CPP
        );
    }

    #[test]
    fn implicit_spell_proc_entry_applies_taken_trigger_attr_and_skips_invalid_like_cpp() {
        let mut source = test_implicit_spell_proc_source_like_cpp();
        source.proc_flags = [PROC_FLAG_TAKE_HARMFUL_SPELL_LIKE_CPP, 0];
        source.effects = vec![test_implicit_proc_effect_like_cpp(
            0,
            aura_types::SPELL_AURA_PROC_TRIGGER_DAMAGE,
            [0, 0, 0, 0],
        )];

        let entry = implicit_spell_proc_entry_like_cpp(&source).unwrap();
        assert_eq!(entry.attributes_mask, PROC_ATTR_TRIGGERED_CAN_PROC_LIKE_CPP);

        source.proc_flags = [0, 0];
        assert!(implicit_spell_proc_entry_like_cpp(&source).is_none());

        source.proc_flags = [PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP, 0];
        source.effects = vec![test_implicit_proc_effect_like_cpp(
            0,
            aura_types::SPELL_AURA_MOUNTED,
            [0, 0, 0, 0],
        )];
        assert!(implicit_spell_proc_entry_like_cpp(&source).is_none());
    }

    #[test]
    fn implicit_spell_proc_entry_rejects_can_proc_from_procs_loop_like_cpp() {
        let mut source = test_implicit_spell_proc_source_like_cpp();
        source.proc_flags = [PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP, 0];
        source.proc_chance = 100.0;
        source.attributes3 = attributes::SPELL_ATTR3_CAN_PROC_FROM_PROCS;
        let mut effect = test_implicit_proc_effect_like_cpp(
            0,
            aura_types::SPELL_AURA_PROC_TRIGGER_SPELL,
            [0, 0, 0, 0],
        );
        effect.trigger_spell = 123;
        source.effects = vec![effect];

        assert!(implicit_spell_proc_entry_like_cpp(&source).is_none());
    }

    fn learn_source(
        spell_id: u32,
        is_talent: bool,
        is_passive: bool,
        has_skill_step_effect: bool,
        learn_spell_effects: Vec<SpellLearnSpellEffectLikeCpp>,
    ) -> SpellLearnSourceSpellInfoLikeCpp {
        SpellLearnSourceSpellInfoLikeCpp {
            spell_id,
            difficulty_none: true,
            is_talent,
            is_passive,
            has_skill_step_effect,
            learn_spell_effects,
        }
    }

    fn test_spell_info_with_aura(spell_id: i32, aura_type: i32) -> SpellInfo {
        SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(aura_type),
            display_flags: 0,
            requires_spell_focus: 0,
            effects: vec![SpellEffectInfo {
                effect_index: 0,
                effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: aura_type,
                ..SpellEffectInfo::default()
            }],
        }
    }

    fn test_spell_info_without_aura(spell_id: i32) -> SpellInfo {
        SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: spell_effect_types::SPELL_EFFECT_NONE,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            effects: Vec::new(),
        }
    }

    fn test_spell_proc_entry_like_cpp() -> SpellProcEntryLikeCpp {
        SpellProcEntryLikeCpp {
            school_mask: 0,
            spell_family_name: 0,
            spell_family_mask: [0, 0, 0, 0],
            proc_flags: [PROC_FLAG_DEAL_MELEE_SWING_LIKE_CPP, 0],
            spell_type_mask: 0,
            spell_phase_mask: PROC_SPELL_PHASE_CAST_LIKE_CPP,
            hit_mask: 0,
            attributes_mask: 0,
            disable_effects_mask: 0,
            procs_per_minute: 0.0,
            chance: 0.0,
            cooldown_ms: 0,
            charges: 0,
        }
    }

    fn test_spell_proc_event_like_cpp(type_mask: u32) -> SpellProcEventInfoLikeCpp {
        SpellProcEventInfoLikeCpp {
            type_mask: [type_mask, 0],
            actor_is_player: false,
            action_target_exists: false,
            action_target_is_honor_or_xp: false,
            proc_spell_has_positive_power_cost: None,
            school_mask: SPELL_SCHOOL_MASK_ALL_LIKE_CPP,
            spell_info: None,
            spell_type_mask: PROC_SPELL_TYPE_MASK_ALL_LIKE_CPP,
            spell_phase_mask: PROC_SPELL_PHASE_CAST_LIKE_CPP,
            hit_mask: PROC_HIT_NORMAL_LIKE_CPP,
        }
    }

    fn test_spell_proc_store_with_entries_like_cpp(
        entries: impl IntoIterator<Item = (u32, u32, [u32; 2])>,
    ) -> SpellProcStoreLikeCpp {
        let mut store = SpellProcStoreLikeCpp::default();
        for (spell_id, difficulty, proc_flags) in entries {
            let mut entry = test_spell_proc_entry_like_cpp();
            entry.proc_flags = proc_flags;
            store.proc_entries_by_spell_and_difficulty.insert(
                SpellProcKeyLikeCpp {
                    spell_id,
                    difficulty,
                },
                entry,
            );
        }
        store
    }

    fn test_implicit_spell_proc_source_like_cpp() -> ImplicitSpellProcSourceLikeCpp {
        ImplicitSpellProcSourceLikeCpp {
            spell_id: 1000,
            difficulty: 0,
            spell_family_name: 0,
            proc_flags: [PROC_FLAG_DEAL_MELEE_SWING_LIKE_CPP, 0],
            proc_chance: 0.0,
            proc_cooldown_ms: 0,
            proc_charges: 0,
            proc_base_ppm: 0.0,
            attributes3: 0,
            effects: Vec::new(),
        }
    }

    fn test_implicit_proc_effect_like_cpp(
        effect_index: u32,
        aura_type: i32,
        spell_class_mask: [u32; 4],
    ) -> ImplicitSpellProcEffectLikeCpp {
        ImplicitSpellProcEffectLikeCpp {
            effect_index,
            is_effect: true,
            is_aura: true,
            aura_type,
            spell_class_mask,
            calc_value: 0,
            trigger_spell: 0,
        }
    }

    fn test_implicit_proc_effect_with_calc_like_cpp(
        effect_index: u32,
        aura_type: i32,
        calc_value: i32,
    ) -> ImplicitSpellProcEffectLikeCpp {
        let mut effect = test_implicit_proc_effect_like_cpp(effect_index, aura_type, [0, 0, 0, 0]);
        effect.calc_value = calc_value;
        effect
    }

    fn test_spell_aura_options_entry_like_cpp(
        id: u32,
        spell_id: u32,
        difficulty_id: u8,
        proc_type_mask: [i32; 2],
        proc_chance: u8,
        proc_charges: i32,
        proc_category_recovery: i32,
        spell_procs_per_minute_id: u16,
    ) -> crate::spell_db2::SpellAuraOptionsEntry {
        crate::spell_db2::SpellAuraOptionsEntry {
            id,
            difficulty_id,
            cumulative_aura: 0,
            proc_category_recovery,
            proc_chance,
            proc_charges,
            spell_procs_per_minute_id,
            proc_type_mask,
            spell_id,
        }
    }

    fn test_spell_misc_entry_like_cpp(
        id: u32,
        spell_id: u32,
        difficulty_id: u8,
        attributes3: u32,
    ) -> crate::spell_db2::SpellMiscEntry {
        let mut attributes = [0; 15];
        attributes[3] = attributes3 as i32;
        crate::spell_db2::SpellMiscEntry {
            id,
            attributes,
            difficulty_id,
            casting_time_index: 0,
            duration_index: 0,
            range_index: 0,
            school_mask: 0,
            speed: 0.0,
            launch_delay: 0.0,
            min_duration: 0.0,
            spell_icon_file_data_id: 0,
            active_icon_file_data_id: 0,
            content_tuning_id: 0,
            show_future_spell_player_condition_id: 0,
            spell_id,
        }
    }

    fn test_spell_proc_row_like_cpp(spell_id: i32) -> SpellProcRowLikeCpp {
        SpellProcRowLikeCpp {
            spell_id,
            school_mask: 0,
            spell_family_name: 0,
            spell_family_mask: [0; 4],
            proc_flags: [0; 2],
            spell_type_mask: 0,
            spell_phase_mask: 0,
            hit_mask: 0,
            attributes_mask: 0,
            disable_effects_mask: 0,
            procs_per_minute: 0.0,
            chance: 0.0,
            cooldown_ms: 0,
            charges: 0,
        }
    }

    fn test_spell_proc_source_like_cpp(
        spell_id: u32,
        first_rank_spell_id: u32,
        next_rank_spell_id: Option<u32>,
    ) -> SpellProcSourceSpellInfoLikeCpp {
        SpellProcSourceSpellInfoLikeCpp {
            spell_id,
            difficulty: 0,
            first_rank_spell_id,
            next_rank_spell_id,
            spell_family_name: 0,
            proc_flags: [0; 2],
            proc_charges: 0,
            proc_chance: 0.0,
            proc_cooldown_ms: 0,
            proc_base_ppm: 0.0,
            attributes3: 0,
            effects: Vec::new(),
        }
    }

    #[test]
    fn spell_learn_spell_store_validates_sql_rows_like_cpp() {
        let outcome = SpellLearnSpellStoreLikeCpp::from_sources_like_cpp(
            [
                SpellLearnSpellSqlRowLikeCpp {
                    entry: 10,
                    spell_id: 20,
                    active: false,
                },
                SpellLearnSpellSqlRowLikeCpp {
                    entry: 11,
                    spell_id: 21,
                    active: true,
                },
                SpellLearnSpellSqlRowLikeCpp {
                    entry: 12,
                    spell_id: 22,
                    active: true,
                },
                SpellLearnSpellSqlRowLikeCpp {
                    entry: 13,
                    spell_id: 23,
                    active: true,
                },
            ],
            [],
            [],
            |spell_id| match spell_id {
                10 => Some(learn_source(10, false, false, false, Vec::new())),
                12 => Some(learn_source(12, false, false, false, Vec::new())),
                13 => Some(learn_source(13, true, false, false, Vec::new())),
                _ => None,
            },
            |spell_id| matches!(spell_id, 20 | 23),
        );

        assert!(!outcome.sql_result_empty);
        assert_eq!(outcome.sql_loaded_row_count, 1);
        assert_eq!(outcome.dbc_loaded_row_count, 0);
        assert_eq!(
            outcome
                .errors
                .iter()
                .map(|error| error.kind)
                .collect::<Vec<_>>(),
            vec![
                SpellLearnSpellLoadErrorKindLikeCpp::SqlSourceSpellMissing,
                SpellLearnSpellLoadErrorKindLikeCpp::SqlLearnedSpellMissing,
                SpellLearnSpellLoadErrorKindLikeCpp::SqlSourceIsTalent,
            ]
        );
        assert_eq!(
            outcome.store.get_spell_learn_spell_map_bounds_like_cpp(10),
            &[SpellLearnSpellNodeLikeCpp {
                spell: 20,
                overrides_spell: 0,
                active: false,
                auto_learned: false,
            }]
        );
        assert!(outcome.store.is_spell_learn_spell_like_cpp(10));
        assert!(outcome.store.is_spell_learn_to_spell_like_cpp(10, 20));
        assert!(!outcome.store.is_spell_learn_to_spell_like_cpp(10, 21));
    }

    #[test]
    fn spell_learn_spell_store_preserves_empty_sql_early_return_like_cpp() {
        let outcome = SpellLearnSpellStoreLikeCpp::from_sources_like_cpp(
            [],
            [learn_source(
                100,
                false,
                false,
                false,
                vec![SpellLearnSpellEffectLikeCpp {
                    trigger_spell: 101,
                    target_unit_pet: false,
                }],
            )],
            [crate::spell_db2::SpellLearnSpellEntry {
                id: 1,
                spell_id: 200,
                learn_spell_id: 201,
                overrides_spell_id: 0,
            }],
            |_| None,
            |_| true,
        );

        assert!(outcome.sql_result_empty);
        assert_eq!(outcome.sql_loaded_row_count, 0);
        assert_eq!(outcome.dbc_loaded_row_count, 0);
        assert!(outcome.store.learned_by_spell_id.is_empty());
        assert!(outcome.errors.is_empty());
        assert!(outcome.warnings.is_empty());
    }

    #[test]
    fn spell_learn_spell_store_adds_spellinfo_effects_like_cpp() {
        let outcome = SpellLearnSpellStoreLikeCpp::from_sources_like_cpp(
            [SpellLearnSpellSqlRowLikeCpp {
                entry: 10,
                spell_id: 20,
                active: true,
            }],
            [
                learn_source(
                    10,
                    false,
                    false,
                    false,
                    vec![SpellLearnSpellEffectLikeCpp {
                        trigger_spell: 20,
                        target_unit_pet: false,
                    }],
                ),
                learn_source(
                    30,
                    false,
                    true,
                    false,
                    vec![SpellLearnSpellEffectLikeCpp {
                        trigger_spell: 31,
                        target_unit_pet: false,
                    }],
                ),
                SpellLearnSourceSpellInfoLikeCpp {
                    spell_id: 40,
                    difficulty_none: false,
                    is_talent: false,
                    is_passive: false,
                    has_skill_step_effect: false,
                    learn_spell_effects: vec![SpellLearnSpellEffectLikeCpp {
                        trigger_spell: 41,
                        target_unit_pet: true,
                    }],
                },
            ],
            [],
            |spell_id| match spell_id {
                10 => Some(learn_source(10, false, false, false, Vec::new())),
                _ => None,
            },
            |spell_id| matches!(spell_id, 20 | 31 | 41),
        );

        assert_eq!(outcome.sql_loaded_row_count, 1);
        assert_eq!(outcome.dbc_loaded_row_count, 1);
        assert_eq!(outcome.warnings.len(), 1);
        assert_eq!(
            outcome.warnings[0].kind,
            SpellLearnSpellLoadWarningKindLikeCpp::RedundantSqlRowForSpellEffect {
                source_spell: 10,
                learned_spell: 20,
            }
        );
        assert_eq!(
            outcome.store.get_spell_learn_spell_map_bounds_like_cpp(30),
            &[SpellLearnSpellNodeLikeCpp {
                spell: 31,
                overrides_spell: 0,
                active: true,
                auto_learned: true,
            }]
        );
        assert!(
            outcome
                .store
                .get_spell_learn_spell_map_bounds_like_cpp(40)
                .is_empty()
        );
    }

    #[test]
    fn spell_learn_spell_store_adds_db2_rows_after_sql_and_spell_effects_like_cpp() {
        let outcome = SpellLearnSpellStoreLikeCpp::from_sources_like_cpp(
            [SpellLearnSpellSqlRowLikeCpp {
                entry: 10,
                spell_id: 20,
                active: true,
            }],
            [learn_source(
                30,
                false,
                false,
                false,
                vec![SpellLearnSpellEffectLikeCpp {
                    trigger_spell: 31,
                    target_unit_pet: true,
                }],
            )],
            [
                crate::spell_db2::SpellLearnSpellEntry {
                    id: 1,
                    spell_id: 10,
                    learn_spell_id: 20,
                    overrides_spell_id: 0,
                },
                crate::spell_db2::SpellLearnSpellEntry {
                    id: 2,
                    spell_id: 30,
                    learn_spell_id: 31,
                    overrides_spell_id: 0,
                },
                crate::spell_db2::SpellLearnSpellEntry {
                    id: 3,
                    spell_id: 40,
                    learn_spell_id: 41,
                    overrides_spell_id: 42,
                },
                crate::spell_db2::SpellLearnSpellEntry {
                    id: 4,
                    spell_id: 50,
                    learn_spell_id: 51,
                    overrides_spell_id: 0,
                },
            ],
            |spell_id| match spell_id {
                10 => Some(learn_source(10, false, false, false, Vec::new())),
                _ => None,
            },
            |spell_id| matches!(spell_id, 10 | 20 | 30 | 31 | 40 | 41 | 51),
        );

        assert_eq!(outcome.sql_loaded_row_count, 1);
        assert_eq!(
            outcome.dbc_loaded_row_count, 2,
            "one SpellInfo effect plus one non-redundant SpellLearnSpell.db2 row"
        );
        assert_eq!(outcome.warnings.len(), 1);
        assert_eq!(
            outcome.warnings[0].kind,
            SpellLearnSpellLoadWarningKindLikeCpp::RedundantSqlRowForDb2 {
                source_spell: 10,
                learned_spell: 20,
            }
        );
        assert_eq!(
            outcome.store.get_spell_learn_spell_map_bounds_like_cpp(40),
            &[SpellLearnSpellNodeLikeCpp {
                spell: 41,
                overrides_spell: 42,
                active: true,
                auto_learned: false,
            }]
        );
        assert!(
            outcome
                .store
                .get_spell_learn_spell_map_bounds_like_cpp(50)
                .is_empty(),
            "C++ silently skips SpellLearnSpell.db2 rows whose source spell is missing"
        );
    }

    fn serverside_effect_row(spell_id: u32, effect_index: i32) -> ServersideSpellEffectRowLikeCpp {
        ServersideSpellEffectRowLikeCpp {
            spell_id,
            effect_index,
            difficulty_id: 0,
            effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA as i32,
            effect_aura: SPELL_AURA_DUMMY_LIKE_CPP,
            effect_amplitude: 0.0,
            effect_attributes: 0,
            effect_aura_period: 0,
            effect_bonus_coefficient: 0.0,
            effect_chain_amplitude: 0.0,
            effect_chain_targets: 0,
            effect_item_type: 0,
            effect_mechanic: 0,
            effect_points_per_resource: 0.0,
            effect_pos_facing: 0.0,
            effect_real_points_per_level: 0.0,
            effect_trigger_spell: 0,
            bonus_coefficient_from_ap: 0.0,
            pvp_multiplier: 0.0,
            coefficient: 0.0,
            variance: 0.0,
            resource_coefficient: 0.0,
            group_size_base_points_coefficient: 0.0,
            effect_base_points: 1.0,
            effect_misc_value_1: 0,
            effect_misc_value_2: 0,
            effect_radius_index_1: 0,
            effect_radius_index_2: 0,
            effect_spell_class_mask: [0, 0, 0, 0],
            implicit_target_1: 0,
            implicit_target_2: 0,
        }
    }

    #[test]
    fn serverside_spell_effect_store_groups_valid_effects_like_cpp() {
        let mut heroic = serverside_effect_row(100, 1);
        heroic.difficulty_id = 2;
        heroic.effect_radius_index_1 = 7;
        heroic.effect_radius_index_2 = 8;
        heroic.effect_spell_class_mask = [1, 2, 3, 4];
        heroic.implicit_target_1 = implicit_targets::TARGET_DEST_DB as i32;

        let outcome = ServersideSpellEffectStoreLikeCpp::from_rows_like_cpp(
            [heroic],
            |_| false,
            |difficulty| difficulty == 2,
            |radius| matches!(radius, 7 | 8),
        );

        assert_eq!(outcome.loaded_effect_count, 1);
        assert!(outcome.errors.is_empty());
        assert!(outcome.warnings.is_empty());
        let effects = outcome
            .store
            .effects_for_spell_difficulty_like_cpp(100, 2)
            .expect("valid serverside effect should be staged");
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0].effect_index, 1);
        assert_eq!(effects[0].effect_spell_class_mask, [1, 2, 3, 4]);
        assert_eq!(
            effects[0].implicit_target,
            [implicit_targets::TARGET_DEST_DB as i32, 0]
        );
    }

    #[test]
    fn serverside_spell_effect_store_skips_invalid_rows_like_cpp() {
        let mut regular_spell = serverside_effect_row(10, 0);
        let mut missing_difficulty = serverside_effect_row(20, 0);
        missing_difficulty.difficulty_id = 3;
        let effect_index = serverside_effect_row(30, MAX_SPELL_EFFECTS_LIKE_CPP);
        let mut effect_type = serverside_effect_row(40, 0);
        effect_type.effect = TOTAL_SPELL_EFFECTS_LIKE_CPP;
        let mut aura_type = serverside_effect_row(50, 0);
        aura_type.effect_aura = TOTAL_AURAS_LIKE_CPP;
        let mut target_a = serverside_effect_row(60, 0);
        target_a.implicit_target_1 = TOTAL_SPELL_TARGETS_LIKE_CPP;
        let mut target_b = serverside_effect_row(70, 0);
        target_b.implicit_target_2 = TOTAL_SPELL_TARGETS_LIKE_CPP;
        regular_spell.effect_base_points = 10.0;

        let outcome = ServersideSpellEffectStoreLikeCpp::from_rows_like_cpp(
            [
                regular_spell,
                missing_difficulty,
                effect_index,
                effect_type,
                aura_type,
                target_a,
                target_b,
            ],
            |spell_id| spell_id == 10,
            |_| false,
            |_| true,
        );

        assert_eq!(outcome.loaded_effect_count, 0);
        assert_eq!(
            outcome
                .errors
                .iter()
                .map(|error| error.kind)
                .collect::<Vec<_>>(),
            vec![
                ServersideSpellEffectLoadErrorKindLikeCpp::RegularSpellAlreadyLoaded,
                ServersideSpellEffectLoadErrorKindLikeCpp::DifficultyMissing,
                ServersideSpellEffectLoadErrorKindLikeCpp::EffectIndexOutOfRange,
                ServersideSpellEffectLoadErrorKindLikeCpp::EffectTypeOutOfRange,
                ServersideSpellEffectLoadErrorKindLikeCpp::AuraTypeOutOfRange,
                ServersideSpellEffectLoadErrorKindLikeCpp::ImplicitTarget1OutOfRange,
                ServersideSpellEffectLoadErrorKindLikeCpp::ImplicitTarget2OutOfRange,
            ]
        );
    }

    #[test]
    fn serverside_spell_effect_store_preserves_cpp_radius_warning_without_skip() {
        let mut row = serverside_effect_row(100, -1);
        row.effect_radius_index_1 = 77;
        row.effect_radius_index_2 = 88;

        let outcome = ServersideSpellEffectStoreLikeCpp::from_rows_like_cpp(
            [row],
            |_| false,
            |_| true,
            |_| false,
        );

        assert_eq!(outcome.loaded_effect_count, 1);
        assert!(outcome.errors.is_empty());
        assert_eq!(
            outcome
                .warnings
                .iter()
                .map(|warning| warning.kind)
                .collect::<Vec<_>>(),
            vec![
                ServersideSpellEffectLoadWarningKindLikeCpp::EffectRadius1Missing,
                ServersideSpellEffectLoadWarningKindLikeCpp::EffectRadius2Missing,
            ]
        );
        let effects = outcome
            .store
            .effects_for_spell_difficulty_like_cpp(100, 0)
            .expect("C++ still pushes effects with invalid radius rows");
        assert_eq!(effects[0].effect_index, -1);
        assert_eq!(effects[0].effect_radius_index, [77, 88]);
    }

    fn serverside_spell_row(spell_id: u32, difficulty_id: u32) -> ServersideSpellRowLikeCpp {
        ServersideSpellRowLikeCpp {
            spell_id,
            difficulty_id,
            category_id: 1,
            dispel: 2,
            mechanic: 3,
            attributes: 4,
            attributes_ex: [5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18],
            stances: 19,
            stances_not: 20,
            targets: 21,
            target_creature_type: 22,
            requires_spell_focus: 23,
            facing_caster_flags: 24,
            caster_aura_state: 25,
            target_aura_state: 26,
            exclude_caster_aura_state: 27,
            exclude_target_aura_state: 28,
            caster_aura_spell: 29,
            target_aura_spell: 30,
            exclude_caster_aura_spell: 31,
            exclude_target_aura_spell: 32,
            caster_aura_type: 33,
            target_aura_type: 34,
            exclude_caster_aura_type: 35,
            exclude_target_aura_type: 36,
            casting_time_index: 37,
            recovery_time: 38,
            category_recovery_time: 39,
            start_recovery_category: 40,
            start_recovery_time: 41,
            interrupt_flags: 42,
            aura_interrupt_flags: [43, 44],
            channel_interrupt_flags: [45, 46],
            proc_flags: [47, 48],
            proc_chance: 49,
            proc_charges: 50,
            proc_cooldown: 51,
            proc_base_ppm: 52.0,
            max_level: 53,
            base_level: 54,
            spell_level: 55,
            duration_index: 56,
            range_index: 57,
            speed: 58.0,
            launch_delay: 59.0,
            stack_amount: 60,
            equipped_item_class: -1,
            equipped_item_sub_class_mask: 62,
            equipped_item_inventory_type_mask: 63,
            content_tuning_id: 64,
            spell_name: format!("Serverside {spell_id}"),
            cone_angle: 65.0,
            cone_width: 66.0,
            max_target_level: 67,
            max_affected_targets: 68,
            spell_family_name: 69,
            spell_family_flags: [70, 71, 72, 73],
            dmg_class: 74,
            prevention_type: 75,
            area_group_id: 76,
            school_mask: 77,
            charge_category_id: 78,
        }
    }

    #[test]
    fn serverside_spell_store_composes_rows_with_staged_effects_like_cpp() {
        let effect_outcome = ServersideSpellEffectStoreLikeCpp::from_rows_like_cpp(
            [serverside_effect_row(100, 0)],
            |_| false,
            |_| true,
            |_| true,
        );
        let outcome = ServersideSpellStoreLikeCpp::from_rows_like_cpp(
            [serverside_spell_row(100, 0)],
            &effect_outcome.store,
            |_| false,
        );

        assert_eq!(outcome.loaded_spell_count, 1);
        assert!(outcome.errors.is_empty());
        assert_eq!(
            outcome.store.serverside_spell_names,
            vec![(100, "Serverside 100".to_string())]
        );
        let info = outcome
            .store
            .get_serverside_spell_like_cpp(100, 0)
            .expect("serverside spell should be represented");
        assert_eq!(info.row.attributes_ex[13], 18);
        assert_eq!(info.row.spell_family_flags, [70, 71, 72, 73]);
        assert_eq!(info.effects.len(), 1);
        assert_eq!(info.effects[0].effect_index, 0);
    }

    #[test]
    fn serverside_spell_store_rejects_regular_db2_spell_like_cpp() {
        let outcome = ServersideSpellStoreLikeCpp::from_rows_like_cpp(
            [serverside_spell_row(100, 0)],
            &ServersideSpellEffectStoreLikeCpp::default(),
            |spell_id| spell_id == 100,
        );

        assert_eq!(outcome.loaded_spell_count, 0);
        assert_eq!(outcome.errors.len(), 1);
        assert_eq!(
            outcome.errors[0].kind,
            ServersideSpellLoadErrorKindLikeCpp::RegularSpellAlreadyLoaded
        );
        assert!(outcome.store.serverside_spell_names.is_empty());
        assert!(outcome.store.spell_infos_by_spell_and_difficulty.is_empty());
    }

    #[test]
    fn serverside_spell_store_does_not_validate_main_row_difficulty_like_cpp() {
        let outcome = ServersideSpellStoreLikeCpp::from_rows_like_cpp(
            [serverside_spell_row(100, 999)],
            &ServersideSpellEffectStoreLikeCpp::default(),
            |_| false,
        );

        assert_eq!(outcome.loaded_spell_count, 1);
        assert!(outcome.errors.is_empty());
        assert!(
            outcome
                .store
                .get_serverside_spell_like_cpp(100, 999)
                .is_some(),
            "C++ LoadSpellInfoServerside validates DifficultyID for effect rows, not for the main serverside_spell row"
        );
    }
}
