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

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::f32::consts::TAU;

use anyhow::Result;
use tracing::info;
use wow_constants::{PowerType, SpellCastResult};
use wow_database::{
    HotfixDatabase, HotfixStatements, StatementDef, WorldDatabase, WorldStatements,
};
use wow_entities::PetAuraLikeCpp;

use crate::{
    ConditionEntriesByTypeStore, ConditionsReference,
    conditions::RACEMASK_ALL_PLAYABLE_LIKE_CPP,
    skill::SkillLineAbilityRankRowLikeCpp,
    spell_acquisition::{
        AcquisitionValueDomainLikeCpp, SpellAcquisitionIndeterminateReasonLikeCpp,
    },
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
    /// C++ `SPELL_EFFECT_SKILL_STEP`; used by `SpellMgr::LoadSpellLearnSpells`
    /// to mark dependent learn-spell rows as auto-learned.
    pub const SPELL_EFFECT_SKILL_STEP: u32 = 44;
    pub const SPELL_EFFECT_PLAY_MOVIE: u32 = 45;
    pub const SPELL_EFFECT_SPAWN: u32 = 46;
    pub const SPELL_EFFECT_TRADE_SKILL: u32 = 47;
    pub const SPELL_EFFECT_STEALTH: u32 = 48;
    pub const SPELL_EFFECT_DETECT: u32 = 49;
    pub const SPELL_EFFECT_FORCE_CRITICAL_HIT: u32 = 51;
    pub const SPELL_EFFECT_GUARANTEE_HIT: u32 = 52;
    pub const SPELL_EFFECT_PROFICIENCY: u32 = 60;
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
    /// C++ `SPELL_EFFECT_TITAN_GRIP`; see `Spell::EffectTitanGrip`
    /// (`SpellEffects.cpp:4910-4919`).
    pub const SPELL_EFFECT_TITAN_GRIP: u32 = 155;
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
    /// C++ `AuraType::SPELL_AURA_SCHOOL_ABSORB`.
    pub const SPELL_AURA_SCHOOL_ABSORB: i32 = 69;
    pub const SPELL_AURA_SCHOOL_IMMUNITY: i32 = 39;
    pub const SPELL_AURA_DUMMY_ABSORB: i32 = 3;
    pub const SPELL_AURA_PERIODIC_DAMAGE: i32 = 3;
    pub const SPELL_AURA_MOD_CONFUSE: i32 = 5;
    pub const SPELL_AURA_MOD_FEAR: i32 = 7;
    pub const SPELL_AURA_PERIODIC_HEAL: i32 = 8;
    pub const SPELL_AURA_MOD_THREAT: i32 = 10;
    pub const SPELL_AURA_MOD_TAUNT: i32 = 11;
    pub const SPELL_AURA_MOD_STUN: i32 = 12;
    pub const SPELL_AURA_MOD_DAMAGE_DONE: i32 = 13;
    pub const SPELL_AURA_MOD_DAMAGE_TAKEN: i32 = 14;
    pub const SPELL_AURA_MOD_STEALTH: i32 = 16;
    pub const SPELL_AURA_MOD_STEALTH_DETECT: i32 = 17;
    pub const SPELL_AURA_MOD_INVISIBILITY: i32 = 18;
    pub const SPELL_AURA_MOD_RESISTANCE: i32 = 22;
    pub const SPELL_AURA_MOD_ROOT: i32 = 26;
    pub const SPELL_AURA_MOD_SILENCE: i32 = 27;
    pub const SPELL_AURA_MOD_STAT: i32 = 29;
    pub const SPELL_AURA_REFLECT_SPELLS: i32 = 28;
    pub const SPELL_AURA_MOD_INCREASE_SPEED: i32 = 31;
    pub const SPELL_AURA_MODIFY_DAMAGE_PERCENT_TAKEN: i32 = 31;
    pub const SPELL_AURA_MOD_INCREASE_MOUNTED_SPEED: i32 = 32;
    pub const SPELL_AURA_MOD_DECREASE_SPEED: i32 = 33;
    pub const SPELL_AURA_MOD_INCREASE_HEALTH: i32 = 34;
    pub const SPELL_AURA_MOD_SHAPESHIFT: i32 = 36;
    pub const SPELL_AURA_DAMAGE_IMMUNITY: i32 = 40;
    pub const SPELL_AURA_PROC_TRIGGER_SPELL: i32 = 42;
    pub const SPELL_AURA_PROC_TRIGGER_DAMAGE: i32 = 43;
    pub const SPELL_AURA_MOD_BLOCK_PERCENT: i32 = 51;
    pub const SPELL_AURA_MOD_WEAPON_CRIT_PERCENT: i32 = 52;
    pub const SPELL_AURA_MOD_HIT_CHANCE: i32 = 54;
    pub const SPELL_AURA_TRANSFORM: i32 = 56;
    pub const SPELL_AURA_MOD_SPELL_CRIT_CHANCE: i32 = 57;
    pub const SPELL_AURA_MOD_INCREASE_SWIM_SPEED: i32 = 58;
    pub const SPELL_AURA_MOD_SCALE: i32 = 61;
    pub const SPELL_AURA_MOD_CASTING_SPEED_NOT_STACK: i32 = 65;
    pub const SPELL_AURA_MOD_POWER_COST_SCHOOL_PCT: i32 = 72;
    pub const SPELL_AURA_HASTE_SPELLS: i32 = 73;
    pub const SPELL_AURA_MOD_POWER_COST_SCHOOL: i32 = 73;
    pub const SPELL_AURA_REFLECT_SPELLS_SCHOOL: i32 = 74;
    pub const SPELL_AURA_MECHANIC_IMMUNITY: i32 = 77;
    pub const SPELL_AURA_MOUNTED: i32 = 78;
    pub const SPELL_AURA_MOD_DAMAGE_PERCENT_DONE: i32 = 79;
    pub const SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN: i32 = 87;
    pub const SPELL_AURA_PERIODIC_DAMAGE_PERCENT: i32 = 89;
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
    pub const SPELL_AURA_MOD_SPEED_ALWAYS: i32 = 129;
    pub const SPELL_AURA_MOD_INCREASE_HEALTH_PERCENT: i32 = 133;
    pub const SPELL_AURA_MOD_MOUNTED_SPEED_ALWAYS: i32 = 130;
    pub const SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE: i32 = 137;
    pub const SPELL_AURA_MOD_MELEE_HASTE: i32 = 138;
    pub const SPELL_AURA_FORCE_REACTION: i32 = 139;
    pub const SPELL_AURA_MOD_RANGED_HASTE: i32 = 140;
    pub const SPELL_AURA_MOD_DETECTED_RANGE: i32 = 152;
    pub const SPELL_AURA_MOD_REPUTATION_GAIN: i32 = 156;
    pub const SPELL_AURA_MOD_ATTACK_POWER_PCT: i32 = 166;
    pub const SPELL_AURA_MOD_SPEED_NOT_STACK: i32 = 171;
    pub const SPELL_AURA_MOD_MOUNTED_SPEED_NOT_STACK: i32 = 172;
    pub const SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE: i32 = 184;
    pub const SPELL_AURA_USE_NORMAL_MOVEMENT_SPEED: i32 = 191;
    pub const SPELL_AURA_MOD_MELEE_RANGED_HASTE: i32 = 192;
    /// C++ `AuraType::SPELL_AURA_MOD_XP_PCT`.
    pub const SPELL_AURA_MOD_XP_PCT: i32 = 200;
    pub const SPELL_AURA_FLY: i32 = 201;
    pub const SPELL_AURA_MOD_INCREASE_VEHICLE_FLIGHT_SPEED: i32 = 206;
    pub const SPELL_AURA_MOD_INCREASE_MOUNTED_FLIGHT_SPEED: i32 = 207;
    pub const SPELL_AURA_MOD_INCREASE_FLIGHT_SPEED: i32 = 208;
    pub const SPELL_AURA_MOD_MOUNTED_FLIGHT_SPEED_ALWAYS: i32 = 209;
    pub const SPELL_AURA_MOD_FLIGHT_SPEED_NOT_STACK: i32 = 211;
    pub const SPELL_AURA_ADD_PCT_MODIFIER_BY_SPELL_LABEL: i32 = 218;
    pub const SPELL_AURA_MOD_DETAUNT: i32 = 221;
    pub const SPELL_AURA_PERIODIC_DUMMY: i32 = 226;
    pub const SPELL_AURA_PROC_TRIGGER_SPELL_WITH_VALUE: i32 = 231;
    pub const SPELL_AURA_MOD_EXPERTISE: i32 = 240;
    pub const SPELL_AURA_ABILITY_IGNORE_AURASTATE: i32 = 262;
    pub const SPELL_AURA_MOD_SCHOOL_MASK_DAMAGE_FROM_CASTER: i32 = 270;
    pub const SPELL_AURA_MOD_SPELL_DAMAGE_FROM_CASTER: i32 = 271;
    pub const SPELL_AURA_PROVIDE_SPELL_FOCUS: i32 = 281;
    pub const SPELL_AURA_MOD_MINIMUM_SPEED: i32 = 305;
    pub const SPELL_AURA_MOD_MELEE_HASTE_3: i32 = 319;
    pub const SPELL_AURA_MOD_SPEED_NO_CONTROL: i32 = 373;
    pub const SPELL_AURA_SCHOOL_HEAL_ABSORB: i32 = 301;
    pub const SPELL_AURA_IGNORE_SPELL_COOLDOWN: i32 = 383;
    pub const SPELL_AURA_MOD_BATTLE_PET_XP_PCT: i32 = 420;
    pub const SPELL_AURA_MOD_MINIMUM_SPEED_RATE: i32 = 437;
    pub const SPELL_AURA_MOD_ROOT_2: i32 = 455;
    pub const SPELL_AURA_MOD_RESTED_XP_CONSUMPTION: i32 = 499;
}

/// Selected `Targets` ids from C++ `SpellImplicitTargetInfo::_data`.
pub mod implicit_targets {
    pub const TARGET_DEST_HOME: u32 = 9;
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
    /// C++ `SPELL_ATTR0_IS_ABILITY` (`SharedDefines.h`).
    pub const SPELL_ATTR0_IS_ABILITY: u32 = 0x0000_0010;
    /// C++ `SPELL_ATTR0_PASSIVE` (`SharedDefines.h`).
    pub const SPELL_ATTR0_PASSIVE: u32 = 0x0000_0040;
    /// C++ `SPELL_ATTR0_DO_NOT_DISPLAY_SPELLBOOK_AURA_ICON_COMBAT_LOG` (`SharedDefines.h`).
    pub const SPELL_ATTR0_DO_NOT_DISPLAY_SPELLBOOK_AURA_ICON_COMBAT_LOG: u32 = 0x0000_0080;
    /// C++ `SPELL_ATTR0_NOT_SHAPESHIFTED` (`SharedDefines.h`).
    pub const SPELL_ATTR0_NOT_SHAPESHIFTED: u32 = 0x0001_0000;
    /// C++ `SPELL_ATTR0_ONLY_INDOORS` (`SharedDefines.h`).
    pub const SPELL_ATTR0_ONLY_INDOORS: u32 = 0x0000_4000;
    /// C++ `SPELL_ATTR0_ONLY_OUTDOORS` (`SharedDefines.h`).
    pub const SPELL_ATTR0_ONLY_OUTDOORS: u32 = 0x0000_8000;
    /// C++ `SPELL_ATTR0_ALLOW_WHILE_MOUNTED` (`SharedDefines.h`).
    pub const SPELL_ATTR0_ALLOW_WHILE_MOUNTED: u32 = 0x0100_0000;
    /// C++ `SPELL_ATTR0_NOT_IN_COMBAT_ONLY_PEACEFUL` (`SharedDefines.h`).
    pub const SPELL_ATTR0_NOT_IN_COMBAT_ONLY_PEACEFUL: u32 = 0x1000_0000;
    /// C++ `SPELL_ATTR0_NO_AURA_CANCEL` (`SharedDefines.h`).
    pub const SPELL_ATTR0_NO_AURA_CANCEL: u32 = 0x8000_0000;

    /// C++ `SPELL_ATTR1_IS_CHANNELLED` (`SharedDefines.h`).
    pub const SPELL_ATTR1_IS_CHANNELLED: u32 = 0x0000_0004;
    /// C++ `SPELL_ATTR1_NO_THREAT` (`SharedDefines.h`).
    pub const SPELL_ATTR1_NO_THREAT: u32 = 0x0000_0400;

    /// C++ `SPELL_ATTR1_IS_SELF_CHANNELLED` (`SharedDefines.h`).
    pub const SPELL_ATTR1_IS_SELF_CHANNELLED: u32 = 0x0000_0040;
    /// C++ `SPELL_ATTR1_NO_AUTOCAST_AI` (`SharedDefines.h`).
    pub const SPELL_ATTR1_NO_AUTOCAST_AI: u32 = 0x0002_0000;
    /// C++ `SPELL_ATTR1_NO_AURA_ICON` (`SharedDefines.h`).
    pub const SPELL_ATTR1_NO_AURA_ICON: u32 = 0x1000_0000;
    /// C++ `SPELL_ATTR2_IGNORE_LINE_OF_SIGHT` (`SharedDefines.h`).
    pub const SPELL_ATTR2_IGNORE_LINE_OF_SIGHT: u32 = 0x0000_0004;
    /// C++ `SPELL_ATTR2_ALLOW_WHILE_NOT_SHAPESHIFTED_CASTER_FORM` (`SharedDefines.h`).
    pub const SPELL_ATTR2_ALLOW_WHILE_NOT_SHAPESHIFTED_CASTER_FORM: u32 = 0x0008_0000;
    /// C++ `SPELL_ATTR2_NO_INITIAL_THREAT` (`SharedDefines.h`).
    pub const SPELL_ATTR2_NO_INITIAL_THREAT: u32 = 0x0040_0000;
    /// C++ `SPELL_ATTR3_CAN_PROC_FROM_PROCS` (`SharedDefines.h`).
    pub const SPELL_ATTR3_CAN_PROC_FROM_PROCS: u32 = 0x0400_0000;
    /// C++ `SPELL_ATTR4_AURA_EXPIRES_OFFLINE` (`SharedDefines.h`).
    pub const SPELL_ATTR4_AURA_EXPIRES_OFFLINE: u32 = 0x0000_0004;
    /// C++ `SPELL_ATTR4_NO_HELPFUL_THREAT` (`SharedDefines.h`).
    pub const SPELL_ATTR4_NO_HELPFUL_THREAT: u32 = 0x0000_0008;
    /// C++ `SPELL_ATTR4_NO_HARMFUL_THREAT` (`SharedDefines.h`).
    pub const SPELL_ATTR4_NO_HARMFUL_THREAT: u32 = 0x0000_0010;
    pub const SPELL_ATTR4_USE_FACING_FROM_SPELL: u32 = 0x8000_0000;
}

pub mod shapeshift_form_flags {
    /// C++ `SpellShapeshiftFormFlags::Stance` (`DBCEnums.h`).
    pub const STANCE: i32 = 0x0000_0001;
    /// C++ `SpellShapeshiftFormFlags::CanOnlyCastShapeshiftSpells` (`DBCEnums.h`).
    pub const CAN_ONLY_CAST_SHAPESHIFT_SPELLS: i32 = 0x0000_0400;
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
    /// C++ `SpellInfo::PowerCosts`, hydrated from `SpellPower.db2`.
    pub power_costs: Vec<SpellPowerCostInfoLikeCpp>,
    /// Spell effects keyed by C++ `SpellEffectInfo::EffectIndex`.
    pub effects: Vec<SpellEffectInfo>,
}

/// Difficulty-aware spell metadata used by C++ spell-hit resolution.
///
/// Each effect mechanic is keyed by `SpellEffectEntry::EffectIndex`. A zero
/// value is retained when the effect row exists so that an exact-difficulty
/// row still suppresses the same effect slot from a fallback difficulty.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellHitMetadataLikeCpp {
    /// C++ `SpellInfo::CategoryId`, resolved from `SpellCategories`.
    pub category_id: u32,
    /// C++ `SpellInfo::ChargeCategoryId`, resolved from `SpellCategories`.
    pub charge_category_id: u32,
    pub defense_type: i8,
    pub spell_mechanic: i8,
    pub school_mask: u8,
    pub effect_mechanics: BTreeMap<u32, i32>,
}

/// Missing or malformed data that prevents safe C++ primary-profession
/// classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryProfessionSpellClassificationErrorLikeCpp {
    InvalidSpellId {
        spell_id: i32,
    },
    InvalidSkillId {
        spell_id: i32,
        effect_index: u32,
        skill_id: i32,
    },
    MissingSkillLinePayload {
        spell_id: i32,
        skill_id: u32,
    },
    RankChainIndeterminate {
        spell_id: u32,
    },
}

/// Represented subset of C++ `SpellPowerEntry` stored on `SpellInfo::PowerCosts`.
#[derive(Debug, Clone, PartialEq)]
pub struct SpellPowerCostInfoLikeCpp {
    pub order_index: u8,
    pub power_type: i8,
    pub mana_cost: i32,
    pub mana_cost_per_level: i32,
    pub mana_per_second: i32,
    pub power_cost_pct: f32,
    pub power_cost_max_pct: f32,
    pub power_pct_per_second: f32,
    pub required_aura_spell_id: i32,
    pub optional_cost: u32,
}

/// Calculated spell power cost, mirroring C++ `Spell::m_powerCost`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellPowerCostLikeCpp {
    pub power_type: i8,
    pub amount: i32,
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
    pub async fn load_like_cpp<RegularSpellExists, DifficultyExists, RadiusExists>(
        db: &WorldDatabase,
        regular_spell_exists: RegularSpellExists,
        difficulty_exists: DifficultyExists,
        radius_exists: RadiusExists,
    ) -> Result<ServersideSpellEffectLoadOutcomeLikeCpp>
    where
        RegularSpellExists: FnMut(u32) -> bool,
        DifficultyExists: FnMut(u32) -> bool,
        RadiusExists: FnMut(u32) -> bool,
    {
        let mut result = db
            .direct_query(WorldStatements::SEL_SERVERSIDE_SPELL_EFFECT.sql())
            .await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(ServersideSpellEffectRowLikeCpp {
                    spell_id: result.try_read::<u32>(0).unwrap_or(0),
                    effect_index: result.try_read::<i32>(1).unwrap_or(0),
                    difficulty_id: result.try_read::<u32>(2).unwrap_or(0),
                    effect: result.try_read::<i32>(3).unwrap_or(0),
                    effect_aura: result.try_read::<i32>(4).unwrap_or(0),
                    effect_amplitude: result.try_read::<f32>(5).unwrap_or(0.0),
                    effect_attributes: result.try_read::<i32>(6).unwrap_or(0),
                    effect_aura_period: result.try_read::<i32>(7).unwrap_or(0),
                    effect_bonus_coefficient: result.try_read::<f32>(8).unwrap_or(0.0),
                    effect_chain_amplitude: result.try_read::<f32>(9).unwrap_or(0.0),
                    effect_chain_targets: result.try_read::<i32>(10).unwrap_or(0),
                    effect_item_type: result.try_read::<i32>(11).unwrap_or(0),
                    effect_mechanic: result.try_read::<i32>(12).unwrap_or(0),
                    effect_points_per_resource: result.try_read::<f32>(13).unwrap_or(0.0),
                    effect_pos_facing: result.try_read::<f32>(14).unwrap_or(0.0),
                    effect_real_points_per_level: result.try_read::<f32>(15).unwrap_or(0.0),
                    effect_trigger_spell: result.try_read::<i32>(16).unwrap_or(0),
                    bonus_coefficient_from_ap: result.try_read::<f32>(17).unwrap_or(0.0),
                    pvp_multiplier: result.try_read::<f32>(18).unwrap_or(0.0),
                    coefficient: result.try_read::<f32>(19).unwrap_or(0.0),
                    variance: result.try_read::<f32>(20).unwrap_or(0.0),
                    resource_coefficient: result.try_read::<f32>(21).unwrap_or(0.0),
                    group_size_base_points_coefficient: result.try_read::<f32>(22).unwrap_or(0.0),
                    effect_base_points: result.try_read::<f32>(23).unwrap_or(0.0),
                    effect_misc_value_1: result.try_read::<i32>(24).unwrap_or(0),
                    effect_misc_value_2: result.try_read::<i32>(25).unwrap_or(0),
                    effect_radius_index_1: result.try_read::<u32>(26).unwrap_or(0),
                    effect_radius_index_2: result.try_read::<u32>(27).unwrap_or(0),
                    effect_spell_class_mask: [
                        result.try_read::<i32>(28).unwrap_or(0),
                        result.try_read::<i32>(29).unwrap_or(0),
                        result.try_read::<i32>(30).unwrap_or(0),
                        result.try_read::<i32>(31).unwrap_or(0),
                    ],
                    implicit_target_1: result.try_read::<i32>(32).unwrap_or(0),
                    implicit_target_2: result.try_read::<i32>(33).unwrap_or(0),
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_rows_like_cpp(
            rows,
            regular_spell_exists,
            difficulty_exists,
            radius_exists,
        ))
    }

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

impl ServersideSpellInfoLikeCpp {
    /// Port of C++ `SpellInfo::CheckShapeshift` (`SpellInfo.cpp`).
    pub fn check_shapeshift_like_cpp<'a, F>(&self, form: u32, mut lookup_form: F) -> SpellCastResult
    where
        F: FnMut(u32) -> Option<&'a crate::spell_db2::SpellShapeshiftFormEntry>,
    {
        let stance_mask = form
            .checked_sub(1)
            .and_then(|shift| 1u64.checked_shl(shift))
            .unwrap_or(0);

        if stance_mask & self.row.stances_not != 0 {
            return SpellCastResult::NotShapeshift;
        }

        if stance_mask & self.row.stances != 0 {
            return SpellCastResult::Success;
        }

        let mut act_as_shifted = false;
        let mut form_flags = 0;
        if form > 0 {
            let Some(shape_info) = lookup_form(form) else {
                return SpellCastResult::Success;
            };
            form_flags = shape_info.flags;
            act_as_shifted = form_flags & shapeshift_form_flags::STANCE == 0;
        }

        if act_as_shifted {
            if self.row.attributes & attributes::SPELL_ATTR0_NOT_SHAPESHIFTED != 0
                || form_flags & shapeshift_form_flags::CAN_ONLY_CAST_SHAPESHIFT_SPELLS != 0
            {
                return SpellCastResult::NotShapeshift;
            }

            if self.row.stances != 0 {
                return SpellCastResult::OnlyShapeshift;
            }
        } else if self.row.attributes_ex[1]
            & attributes::SPELL_ATTR2_ALLOW_WHILE_NOT_SHAPESHIFTED_CASTER_FORM
            == 0
            && self.row.stances != 0
        {
            return SpellCastResult::OnlyShapeshift;
        }

        SpellCastResult::Success
    }
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
    pub async fn load_like_cpp<RegularSpellExists>(
        db: &WorldDatabase,
        effects: &ServersideSpellEffectStoreLikeCpp,
        regular_spell_exists: RegularSpellExists,
    ) -> Result<ServersideSpellLoadOutcomeLikeCpp>
    where
        RegularSpellExists: FnMut(u32) -> bool,
    {
        let mut result = db
            .direct_query(WorldStatements::SEL_SERVERSIDE_SPELL.sql())
            .await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(ServersideSpellRowLikeCpp {
                    spell_id: result.try_read::<u32>(0).unwrap_or(0),
                    difficulty_id: result.try_read::<u32>(1).unwrap_or(0),
                    category_id: result.try_read::<u32>(2).unwrap_or(0),
                    dispel: result.try_read::<u32>(3).unwrap_or(0),
                    mechanic: result.try_read::<u32>(4).unwrap_or(0),
                    attributes: result.try_read::<u32>(5).unwrap_or(0),
                    attributes_ex: [
                        result.try_read::<u32>(6).unwrap_or(0),
                        result.try_read::<u32>(7).unwrap_or(0),
                        result.try_read::<u32>(8).unwrap_or(0),
                        result.try_read::<u32>(9).unwrap_or(0),
                        result.try_read::<u32>(10).unwrap_or(0),
                        result.try_read::<u32>(11).unwrap_or(0),
                        result.try_read::<u32>(12).unwrap_or(0),
                        result.try_read::<u32>(13).unwrap_or(0),
                        result.try_read::<u32>(14).unwrap_or(0),
                        result.try_read::<u32>(15).unwrap_or(0),
                        result.try_read::<u32>(16).unwrap_or(0),
                        result.try_read::<u32>(17).unwrap_or(0),
                        result.try_read::<u32>(18).unwrap_or(0),
                        result.try_read::<u32>(19).unwrap_or(0),
                    ],
                    stances: result.try_read::<u64>(20).unwrap_or(0),
                    stances_not: result.try_read::<u64>(21).unwrap_or(0),
                    targets: result.try_read::<u32>(22).unwrap_or(0),
                    target_creature_type: result.try_read::<u32>(23).unwrap_or(0),
                    requires_spell_focus: result.try_read::<u32>(24).unwrap_or(0),
                    facing_caster_flags: result.try_read::<u32>(25).unwrap_or(0),
                    caster_aura_state: result.try_read::<u32>(26).unwrap_or(0),
                    target_aura_state: result.try_read::<u32>(27).unwrap_or(0),
                    exclude_caster_aura_state: result.try_read::<u32>(28).unwrap_or(0),
                    exclude_target_aura_state: result.try_read::<u32>(29).unwrap_or(0),
                    caster_aura_spell: result.try_read::<u32>(30).unwrap_or(0),
                    target_aura_spell: result.try_read::<u32>(31).unwrap_or(0),
                    exclude_caster_aura_spell: result.try_read::<u32>(32).unwrap_or(0),
                    exclude_target_aura_spell: result.try_read::<u32>(33).unwrap_or(0),
                    caster_aura_type: result.try_read::<i32>(34).unwrap_or(0),
                    target_aura_type: result.try_read::<i32>(35).unwrap_or(0),
                    exclude_caster_aura_type: result.try_read::<i32>(36).unwrap_or(0),
                    exclude_target_aura_type: result.try_read::<i32>(37).unwrap_or(0),
                    casting_time_index: result.try_read::<u32>(38).unwrap_or(0),
                    recovery_time: result.try_read::<u32>(39).unwrap_or(0),
                    category_recovery_time: result.try_read::<u32>(40).unwrap_or(0),
                    start_recovery_category: result.try_read::<u32>(41).unwrap_or(0),
                    start_recovery_time: result.try_read::<u32>(42).unwrap_or(0),
                    interrupt_flags: result.try_read::<u32>(43).unwrap_or(0),
                    aura_interrupt_flags: [
                        result.try_read::<u32>(44).unwrap_or(0),
                        result.try_read::<u32>(45).unwrap_or(0),
                    ],
                    channel_interrupt_flags: [
                        result.try_read::<u32>(46).unwrap_or(0),
                        result.try_read::<u32>(47).unwrap_or(0),
                    ],
                    proc_flags: [
                        result.try_read::<u32>(48).unwrap_or(0),
                        result.try_read::<u32>(49).unwrap_or(0),
                    ],
                    proc_chance: result.try_read::<u32>(50).unwrap_or(0),
                    proc_charges: result.try_read::<u32>(51).unwrap_or(0),
                    proc_cooldown: result.try_read::<u32>(52).unwrap_or(0),
                    proc_base_ppm: result.try_read::<f32>(53).unwrap_or(0.0),
                    max_level: result.try_read::<u32>(54).unwrap_or(0),
                    base_level: result.try_read::<u32>(55).unwrap_or(0),
                    spell_level: result.try_read::<u32>(56).unwrap_or(0),
                    duration_index: result.try_read::<u32>(57).unwrap_or(0),
                    range_index: result.try_read::<u32>(58).unwrap_or(0),
                    speed: result.try_read::<f32>(59).unwrap_or(0.0),
                    launch_delay: result.try_read::<f32>(60).unwrap_or(0.0),
                    stack_amount: result.try_read::<u32>(61).unwrap_or(0),
                    equipped_item_class: result.try_read::<i32>(62).unwrap_or(0),
                    equipped_item_sub_class_mask: result.try_read::<i32>(63).unwrap_or(0),
                    equipped_item_inventory_type_mask: result.try_read::<i32>(64).unwrap_or(0),
                    content_tuning_id: result.try_read::<u32>(65).unwrap_or(0),
                    spell_name: result.try_read::<String>(66).unwrap_or_default(),
                    cone_angle: result.try_read::<f32>(67).unwrap_or(0.0),
                    cone_width: result.try_read::<f32>(68).unwrap_or(0.0),
                    max_target_level: result.try_read::<u32>(69).unwrap_or(0),
                    max_affected_targets: result.try_read::<u32>(70).unwrap_or(0),
                    spell_family_name: result.try_read::<u32>(71).unwrap_or(0),
                    spell_family_flags: [
                        result.try_read::<u32>(72).unwrap_or(0),
                        result.try_read::<u32>(73).unwrap_or(0),
                        result.try_read::<u32>(74).unwrap_or(0),
                        result.try_read::<u32>(75).unwrap_or(0),
                    ],
                    dmg_class: result.try_read::<u32>(76).unwrap_or(0),
                    prevention_type: result.try_read::<u32>(77).unwrap_or(0),
                    area_group_id: result.try_read::<i32>(78).unwrap_or(0),
                    school_mask: result.try_read::<u32>(79).unwrap_or(0),
                    charge_category_id: result.try_read::<u32>(80).unwrap_or(0),
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_rows_like_cpp(
            rows,
            effects,
            regular_spell_exists,
        ))
    }

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

    pub async fn load_like_cpp(
        db: &WorldDatabase,
        spells: &SpellStore,
    ) -> Result<SpellPetAuraLoadOutcomeLikeCpp> {
        let stmt = db.prepare(WorldStatements::SEL_SPELL_PET_AURAS);
        let mut result = db.query(&stmt).await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(SpellPetAuraRowLikeCpp {
                    spell_id: result.try_read::<u32>(0).unwrap_or(0),
                    effect_index: result.try_read::<u8>(1).unwrap_or(0),
                    pet_entry: result.try_read::<u32>(2).unwrap_or(0),
                    aura_id: result.try_read::<u32>(3).unwrap_or(0),
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::load_spell_pet_auras_like_cpp(
            rows,
            |spell_id, effect_index| {
                let Some(spell) = spells.get(spell_id as i32) else {
                    return SpellPetAuraSourceLookupLikeCpp::SpellMissing;
                };
                let Some(effect) = spell
                    .effects()
                    .iter()
                    .find(|effect| effect.effect_index == u32::from(effect_index))
                else {
                    return SpellPetAuraSourceLookupLikeCpp::EffectIndexMissing;
                };
                SpellPetAuraSourceLookupLikeCpp::Found(SpellPetAuraSourceEffectLikeCpp {
                    effect: effect.effect,
                    apply_aura_name: effect.effect_aura,
                    target_a: effect.implicit_target_1,
                    calc_value: effect.calc_value_no_caster_like_cpp(),
                })
            },
            |aura_id| spells.get(aura_id as i32).is_some(),
        ))
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
    pub async fn load_like_cpp(
        db: &WorldDatabase,
        spells: &SpellStore,
    ) -> Result<SpellLinkedLoadOutcomeLikeCpp> {
        let stmt = db.prepare(WorldStatements::SEL_SPELL_LINKED);
        let mut result = db.query(&stmt).await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(SpellLinkedRowLikeCpp {
                    spell_trigger: result.try_read::<i32>(0).unwrap_or(0),
                    spell_effect: result.try_read::<i32>(1).unwrap_or(0),
                    link_type: result.try_read::<u8>(2).unwrap_or(0),
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_rows_like_cpp(rows, |spell_id| {
            spells
                .get(spell_id as i32)
                .map(SpellLinkedSpellInfoLikeCpp::from_represented_spell_info_base_points)
        }))
    }

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
    pub async fn load_like_cpp<SpellExists, RaceExists, DisplayExists>(
        db: &WorldDatabase,
        spell_exists: SpellExists,
        race_exists: RaceExists,
        display_exists: DisplayExists,
    ) -> Result<SpellTotemModelLoadOutcomeLikeCpp>
    where
        SpellExists: FnMut(u32) -> bool,
        RaceExists: FnMut(u8) -> bool,
        DisplayExists: FnMut(u32) -> bool,
    {
        let stmt = db.prepare(WorldStatements::SEL_SPELL_TOTEM_MODEL);
        let mut result = db.query(&stmt).await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(SpellTotemModelRowLikeCpp {
                    spell_id: result.try_read::<u32>(0).unwrap_or(0),
                    race_id: result.try_read::<u8>(1).unwrap_or(0),
                    display_id: result.try_read::<u32>(2).unwrap_or(0),
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_rows_like_cpp(
            rows,
            spell_exists,
            race_exists,
            display_exists,
        ))
    }

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
    /// Deterministic C++ `SpellEffectInfo::CalcValue()` result for
    /// `SPELL_EFFECT_SKILL`. Ranged results are retained separately as a
    /// typed indeterminate source and never enter this compatibility shape.
    pub calc_value: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellLearnSkillSourceSpellInfoLikeCpp {
    pub spell_id: u32,
    pub difficulty_none: bool,
    pub effects: Vec<SpellLearnSkillEffectLikeCpp>,
}

/// Why the deterministic Rust acquisition authority cannot publish C++'s
/// compatibility `SpellLearnSkillNode`.
///
/// C++ samples `SpellEffectInfo::CalcValue()` once during startup.  A ranged
/// result can therefore select a different persisted skill tier after a
/// restart.  The official 3.4.3 `SKILL` data is entirely singleton-valued;
/// Rust keeps custom/future ranged metadata explicit so the pure acquisition
/// planner can fail closed instead of confusing it with a covered spell that
/// has no learn-skill effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellLearnSkillIndeterminateReasonLikeCpp {
    MissingEffectiveCoverage {
        difficulty_id: u32,
    },
    EffectiveMetadata(Vec<SpellAcquisitionIndeterminateReasonLikeCpp>),
    InvalidEffectiveValue {
        record_id: u32,
        field: &'static str,
        raw: i64,
    },
    RngDependentCalcValue {
        record_id: u32,
        domain: AcquisitionValueDomainLikeCpp,
    },
    SkillOutOfRange {
        value: i32,
    },
    StepOutOfRange {
        value: i32,
    },
    DuplicateSourceSpell,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellLearnSkillLookupLikeCpp<'a> {
    Present(&'a SpellLearnSkillNodeLikeCpp),
    CoveredWithoutNode,
    Indeterminate(&'a SpellLearnSkillIndeterminateReasonLikeCpp),
    MissingCoverage,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellLearnSkillStoreLikeCpp {
    pub skill_by_spell_id: BTreeMap<u32, SpellLearnSkillNodeLikeCpp>,
    pub covered_spell_ids: BTreeSet<u32>,
    pub indeterminate_by_spell_id: BTreeMap<u32, SpellLearnSkillIndeterminateReasonLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellLearnSkillLoadErrorKindLikeCpp {
    SkillOutOfRange { value: i32 },
    StepOutOfRange { value: i32 },
    DuplicateSourceSpell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellLearnSkillLoadErrorLikeCpp {
    pub spell_id: u32,
    pub kind: SpellLearnSkillLoadErrorKindLikeCpp,
}

impl SpellLearnSkillStoreLikeCpp {
    /// Build C++'s first matching `SKILL` / `DUAL_WIELD` node.
    ///
    /// The selected effect order remains faithful, while Rust deliberately
    /// rejects values that C++ would narrow into `uint16`: the complete
    /// acquisition catalog retains the source value so authorization can
    /// classify the spell as indeterminate instead of accepting a wrapped ID.
    pub fn from_spell_infos_like_cpp<I>(source_spells: I) -> SpellLearnSkillLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellLearnSkillSourceSpellInfoLikeCpp>,
    {
        let mut store = Self::default();
        let mut dbc_loaded_row_count = 0;
        let mut errors = Vec::new();

        for source_spell in source_spells {
            if !source_spell.difficulty_none {
                continue;
            }

            if !store.covered_spell_ids.insert(source_spell.spell_id) {
                if store
                    .skill_by_spell_id
                    .remove(&source_spell.spell_id)
                    .is_some()
                {
                    dbc_loaded_row_count -= 1;
                }
                store.indeterminate_by_spell_id.insert(
                    source_spell.spell_id,
                    SpellLearnSkillIndeterminateReasonLikeCpp::DuplicateSourceSpell,
                );
                errors.push(SpellLearnSkillLoadErrorLikeCpp {
                    spell_id: source_spell.spell_id,
                    kind: SpellLearnSkillLoadErrorKindLikeCpp::DuplicateSourceSpell,
                });
                continue;
            }
            for effect in source_spell.effects {
                let node = match effect.effect {
                    spell_effect_types::SPELL_EFFECT_SKILL => {
                        let Ok(skill) = u16::try_from(effect.misc_value) else {
                            store.indeterminate_by_spell_id.insert(
                                source_spell.spell_id,
                                SpellLearnSkillIndeterminateReasonLikeCpp::SkillOutOfRange {
                                    value: effect.misc_value,
                                },
                            );
                            errors.push(SpellLearnSkillLoadErrorLikeCpp {
                                spell_id: source_spell.spell_id,
                                kind: SpellLearnSkillLoadErrorKindLikeCpp::SkillOutOfRange {
                                    value: effect.misc_value,
                                },
                            });
                            break;
                        };
                        let Ok(step) = u16::try_from(effect.calc_value) else {
                            store.indeterminate_by_spell_id.insert(
                                source_spell.spell_id,
                                SpellLearnSkillIndeterminateReasonLikeCpp::StepOutOfRange {
                                    value: effect.calc_value,
                                },
                            );
                            errors.push(SpellLearnSkillLoadErrorLikeCpp {
                                spell_id: source_spell.spell_id,
                                kind: SpellLearnSkillLoadErrorKindLikeCpp::StepOutOfRange {
                                    value: effect.calc_value,
                                },
                            });
                            break;
                        };
                        SpellLearnSkillNodeLikeCpp {
                            skill,
                            step,
                            value: 0,
                            maxvalue: 0,
                        }
                    }
                    spell_effect_types::SPELL_EFFECT_DUAL_WIELD => SpellLearnSkillNodeLikeCpp {
                        skill: SKILL_DUAL_WIELD_LIKE_CPP,
                        step: 1,
                        value: 1,
                        maxvalue: 1,
                    },
                    _ => continue,
                };

                store
                    .indeterminate_by_spell_id
                    .remove(&source_spell.spell_id);
                store.skill_by_spell_id.insert(source_spell.spell_id, node);
                dbc_loaded_row_count += 1;
                break;
            }
        }

        SpellLearnSkillLoadOutcomeLikeCpp {
            store,
            dbc_loaded_row_count,
            errors,
        }
    }

    pub fn get_spell_learn_skill_like_cpp(
        &self,
        spell_id: u32,
    ) -> Option<&SpellLearnSkillNodeLikeCpp> {
        self.skill_by_spell_id.get(&spell_id)
    }

    pub fn mark_spell_learn_skill_indeterminate_like_cpp(
        &mut self,
        spell_id: u32,
        reason: SpellLearnSkillIndeterminateReasonLikeCpp,
    ) {
        self.skill_by_spell_id.remove(&spell_id);
        self.indeterminate_by_spell_id.insert(spell_id, reason);
    }

    pub fn spell_learn_skill_lookup_like_cpp(
        &self,
        spell_id: u32,
    ) -> SpellLearnSkillLookupLikeCpp<'_> {
        if let Some(reason) = self.indeterminate_by_spell_id.get(&spell_id) {
            return SpellLearnSkillLookupLikeCpp::Indeterminate(reason);
        }
        if let Some(node) = self.skill_by_spell_id.get(&spell_id) {
            return SpellLearnSkillLookupLikeCpp::Present(node);
        }
        if self.covered_spell_ids.contains(&spell_id) {
            SpellLearnSkillLookupLikeCpp::CoveredWithoutNode
        } else {
            SpellLearnSkillLookupLikeCpp::MissingCoverage
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellLearnSkillLoadOutcomeLikeCpp {
    pub store: SpellLearnSkillStoreLikeCpp,
    pub dbc_loaded_row_count: usize,
    pub errors: Vec<SpellLearnSkillLoadErrorLikeCpp>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellChainLoadDiagnosticLikeCpp {
    SelfLoop {
        spell_id: u32,
    },
    MultiplePredecessors {
        spell_id: u32,
        predecessor_spell_ids: Vec<u32>,
    },
    Cycle {
        spell_ids: Vec<u32>,
    },
    RankOutOfRange {
        first_spell_id: u32,
        spell_id: u32,
        rank: usize,
    },
    InvalidEffectiveSkillLineAbilityRankEndpoints {
        record_id: u32,
        spell_raw: i128,
        supercedes_spell_raw: i128,
        affected_spell_ids: Vec<u32>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellChainLookupLikeCpp<'a> {
    Unranked,
    Node(&'a SpellChainNodeLikeCpp),
    Indeterminate(&'a [SpellChainLoadDiagnosticLikeCpp]),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellChainStoreLikeCpp {
    pub chains_by_spell_id: BTreeMap<u32, SpellChainNodeLikeCpp>,
    indeterminate_by_spell_id_like_cpp:
        BTreeMap<u32, std::sync::Arc<[SpellChainLoadDiagnosticLikeCpp]>>,
    global_indeterminate_like_cpp: Option<std::sync::Arc<[SpellChainLoadDiagnosticLikeCpp]>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellChainLoadOutcomeLikeCpp {
    pub store: SpellChainStoreLikeCpp,
    pub diagnostics_in_order_like_cpp: Vec<SpellChainLoadDiagnosticLikeCpp>,
}

impl SpellChainLoadOutcomeLikeCpp {
    /// Fail every known weak component touched by one invalid effective rank
    /// row closed. If neither raw endpoint fits C++'s signed `int32` source
    /// domain, the complete rank projection becomes indeterminate.
    fn mark_invalid_skill_line_ability_rank_row_like_cpp(
        &mut self,
        record_id: u32,
        spell_raw: i128,
        supercedes_spell_raw: i128,
        affected_spell_ids: &[u32],
    ) {
        let diagnostic =
            SpellChainLoadDiagnosticLikeCpp::InvalidEffectiveSkillLineAbilityRankEndpoints {
                record_id,
                spell_raw,
                supercedes_spell_raw,
                affected_spell_ids: affected_spell_ids.to_vec(),
            };

        if affected_spell_ids.is_empty() || self.store.global_indeterminate_like_cpp.is_some() {
            let mut global_diagnostics = self
                .store
                .global_indeterminate_like_cpp
                .as_ref()
                .map(|diagnostics| diagnostics.to_vec())
                .unwrap_or_default();
            if self.store.global_indeterminate_like_cpp.is_none() {
                for local_diagnostics in self.store.indeterminate_by_spell_id_like_cpp.values() {
                    for local_diagnostic in local_diagnostics.iter() {
                        if !global_diagnostics.contains(local_diagnostic) {
                            global_diagnostics.push(local_diagnostic.clone());
                        }
                    }
                }
            }
            if !global_diagnostics.contains(&diagnostic) {
                global_diagnostics.push(diagnostic.clone());
            }
            self.store.global_indeterminate_like_cpp = Some(global_diagnostics.into());
            self.store.chains_by_spell_id.clear();
            self.store.indeterminate_by_spell_id_like_cpp.clear();
            if !self.diagnostics_in_order_like_cpp.contains(&diagnostic) {
                self.diagnostics_in_order_like_cpp.push(diagnostic);
            }
            return;
        }

        let mut complete_affected_spell_ids = BTreeSet::new();
        let mut combined_diagnostics = Vec::new();
        for spell_id in affected_spell_ids {
            if let Some(node) = self.store.chains_by_spell_id.get(spell_id) {
                let first_spell_id = node.first_spell_id;
                complete_affected_spell_ids.extend(
                    self.store.chains_by_spell_id.iter().filter_map(
                        |(candidate_spell_id, candidate)| {
                            (candidate.first_spell_id == first_spell_id)
                                .then_some(*candidate_spell_id)
                        },
                    ),
                );
                continue;
            }

            if let Some(existing) = self
                .store
                .indeterminate_by_spell_id_like_cpp
                .get(spell_id)
                .cloned()
            {
                combined_diagnostics.extend(existing.iter().cloned());
                complete_affected_spell_ids.extend(
                    self.store
                        .indeterminate_by_spell_id_like_cpp
                        .iter()
                        .filter_map(|(candidate_spell_id, candidate)| {
                            std::sync::Arc::ptr_eq(candidate, &existing)
                                .then_some(*candidate_spell_id)
                        }),
                );
                continue;
            }

            complete_affected_spell_ids.insert(*spell_id);
        }

        // Diagnostics are inserted in deterministic effective RecordID order;
        // deduplication preserves the first occurrence.
        let mut deduplicated_diagnostics = Vec::new();
        for existing in combined_diagnostics {
            if !deduplicated_diagnostics.contains(&existing) {
                deduplicated_diagnostics.push(existing);
            }
        }
        if !deduplicated_diagnostics.contains(&diagnostic) {
            deduplicated_diagnostics.push(diagnostic.clone());
        }
        let shared_diagnostics: std::sync::Arc<[SpellChainLoadDiagnosticLikeCpp]> =
            deduplicated_diagnostics.into();

        for affected_spell_id in complete_affected_spell_ids {
            self.store.chains_by_spell_id.remove(&affected_spell_id);
            self.store
                .indeterminate_by_spell_id_like_cpp
                .insert(affected_spell_id, shared_diagnostics.clone());
        }
        if !self.diagnostics_in_order_like_cpp.contains(&diagnostic) {
            self.diagnostics_in_order_like_cpp.push(diagnostic);
        }
    }
}

fn spell_rank_endpoint_id_from_raw_like_cpp(raw: i128) -> Option<u32> {
    i32::try_from(raw).ok().map(|value| value as u32)
}

impl SpellChainStoreLikeCpp {
    pub fn from_skill_line_ability_supercedes_like_cpp<I, SpellExists>(
        rows: I,
        spell_exists: SpellExists,
    ) -> Self
    where
        I: IntoIterator<Item = SpellRankEdgeLikeCpp>,
        SpellExists: FnMut(u32) -> bool,
    {
        Self::from_skill_line_ability_supercedes_with_diagnostics_like_cpp(rows, spell_exists).store
    }

    /// Build ranks from the final rank-specific raw authority. Valid
    /// endpoints remain usable even when unrelated `SkillLineAbility` fields
    /// failed hydration; invalid endpoints become explicit component/global
    /// indeterminacy.
    pub fn from_skill_line_ability_rank_rows_with_diagnostics_like_cpp<I, SpellExists>(
        rows: I,
        mut spell_exists: SpellExists,
    ) -> SpellChainLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SkillLineAbilityRankRowLikeCpp>,
        SpellExists: FnMut(u32) -> bool,
    {
        struct PendingIndeterminateRankRowLikeCpp {
            source_order: usize,
            record_id: u32,
            spell_raw: i128,
            supercedes_spell_raw: i128,
            affected_spell_ids: Vec<u32>,
        }

        enum EffectiveRankCandidateLikeCpp {
            Edge(SpellRankEdgeLikeCpp),
            Indeterminate(PendingIndeterminateRankRowLikeCpp),
        }

        let mut existence_by_spell_id = BTreeMap::new();
        let mut candidate_by_predecessor = BTreeMap::new();
        let mut unkeyed_indeterminate_rows = Vec::new();
        let mut malformed_source_diagnostics = Vec::new();

        for (source_order, row) in rows.into_iter().enumerate() {
            match row {
                SkillLineAbilityRankRowLikeCpp::Edge {
                    spell_id,
                    supercedes_spell_id,
                    ..
                } => {
                    if supercedes_spell_id == 0 {
                        continue;
                    }
                    let has_spell = *existence_by_spell_id
                        .entry(spell_id)
                        .or_insert_with(|| spell_exists(spell_id));
                    let has_supercedes = *existence_by_spell_id
                        .entry(supercedes_spell_id)
                        .or_insert_with(|| spell_exists(supercedes_spell_id));
                    if has_spell && has_supercedes {
                        candidate_by_predecessor.insert(
                            supercedes_spell_id,
                            EffectiveRankCandidateLikeCpp::Edge(SpellRankEdgeLikeCpp {
                                spell_id,
                                supercedes_spell_id,
                            }),
                        );
                    }
                }
                SkillLineAbilityRankRowLikeCpp::Indeterminate {
                    record_id,
                    spell_raw,
                    supercedes_spell_raw,
                } => {
                    let spell_id = spell_rank_endpoint_id_from_raw_like_cpp(spell_raw);
                    let supercedes_spell_id =
                        spell_rank_endpoint_id_from_raw_like_cpp(supercedes_spell_raw);
                    if supercedes_spell_id == Some(0) {
                        continue;
                    }

                    let mut affected_spell_ids = [spell_id, supercedes_spell_id]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>();
                    affected_spell_ids.sort_unstable();
                    affected_spell_ids.dedup();

                    // C++ skips a row unless both endpoint lookups succeed. If
                    // any representable endpoint is proven absent, an
                    // unrepresentable endpoint cannot make the row relevant.
                    let mut every_representable_endpoint_exists = true;
                    for affected_spell_id in &affected_spell_ids {
                        let exists = *existence_by_spell_id
                            .entry(*affected_spell_id)
                            .or_insert_with(|| spell_exists(*affected_spell_id));
                        every_representable_endpoint_exists &= exists;
                    }
                    if !affected_spell_ids.is_empty() && !every_representable_endpoint_exists {
                        continue;
                    }

                    // Normalize a manually constructed but fully
                    // representable variant instead of letting it bypass the
                    // same predecessor authority as `Edge`.
                    if let (Some(spell_id), Some(supercedes_spell_id)) =
                        (spell_id, supercedes_spell_id)
                    {
                        candidate_by_predecessor.insert(
                            supercedes_spell_id,
                            EffectiveRankCandidateLikeCpp::Edge(SpellRankEdgeLikeCpp {
                                spell_id,
                                supercedes_spell_id,
                            }),
                        );
                        continue;
                    }

                    let diagnostic =
                        SpellChainLoadDiagnosticLikeCpp::InvalidEffectiveSkillLineAbilityRankEndpoints {
                            record_id,
                            spell_raw,
                            supercedes_spell_raw,
                            affected_spell_ids: affected_spell_ids.clone(),
                        };
                    if !malformed_source_diagnostics.contains(&diagnostic) {
                        malformed_source_diagnostics.push(diagnostic);
                    }
                    let pending = PendingIndeterminateRankRowLikeCpp {
                        source_order,
                        record_id,
                        spell_raw,
                        supercedes_spell_raw,
                        affected_spell_ids,
                    };

                    if let Some(supercedes_spell_id) = supercedes_spell_id {
                        // This candidate participates in the exact same
                        // last-wins predecessor authority as a valid edge. A
                        // later valid row can repair it; a later ambiguous row
                        // can eclipse an earlier valid edge.
                        candidate_by_predecessor.insert(
                            supercedes_spell_id,
                            EffectiveRankCandidateLikeCpp::Indeterminate(pending),
                        );
                    } else {
                        unkeyed_indeterminate_rows.push(pending);
                    }
                }
            }
        }

        let mut filtered_edges = Vec::new();
        let mut indeterminate_rows = unkeyed_indeterminate_rows;
        for candidate in candidate_by_predecessor.into_values() {
            match candidate {
                EffectiveRankCandidateLikeCpp::Edge(edge) => filtered_edges.push(edge),
                EffectiveRankCandidateLikeCpp::Indeterminate(row) => {
                    indeterminate_rows.push(row);
                }
            }
        }
        indeterminate_rows.sort_by_key(|row| row.source_order);

        let mut outcome = Self::from_skill_line_ability_supercedes_with_diagnostics_like_cpp(
            filtered_edges,
            |_| true,
        );
        let graph_diagnostics = std::mem::take(&mut outcome.diagnostics_in_order_like_cpp);
        outcome.diagnostics_in_order_like_cpp = malformed_source_diagnostics;
        for diagnostic in graph_diagnostics {
            if !outcome.diagnostics_in_order_like_cpp.contains(&diagnostic) {
                outcome.diagnostics_in_order_like_cpp.push(diagnostic);
            }
        }

        for row in indeterminate_rows {
            outcome.mark_invalid_skill_line_ability_rank_row_like_cpp(
                row.record_id,
                row.spell_raw,
                row.supercedes_spell_raw,
                &row.affected_spell_ids,
            );
        }
        outcome
    }

    /// Builds the effective `SpellMgr::LoadSpellRanks` projection and retains
    /// malformed custom/hotfix graph evidence instead of inheriting C++'s
    /// startup hang or silently treating an ambiguous rank as unranked.
    ///
    /// Input order is significant for the C++ `std::map::operator[]`
    /// last-wins rule when multiple records name the same predecessor. The
    /// production caller supplies final `SkillLineAbility` rows in ascending
    /// RecordID order.
    pub fn from_skill_line_ability_supercedes_with_diagnostics_like_cpp<I, SpellExists>(
        rows: I,
        mut spell_exists: SpellExists,
    ) -> SpellChainLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellRankEdgeLikeCpp>,
        SpellExists: FnMut(u32) -> bool,
    {
        let mut chain_next_by_spell_id = BTreeMap::new();

        for row in rows {
            if row.supercedes_spell_id == 0 {
                continue;
            }

            if !spell_exists(row.supercedes_spell_id) || !spell_exists(row.spell_id) {
                continue;
            }

            chain_next_by_spell_id.insert(row.supercedes_spell_id, row.spell_id);
        }

        let mut store = Self::default();
        let mut diagnostics_in_order_like_cpp = Vec::new();
        let mut parents_by_spell_id = BTreeMap::<u32, BTreeSet<u32>>::new();
        let mut adjacent_by_spell_id = BTreeMap::<u32, BTreeSet<u32>>::new();
        for (&spell_id, &next_spell_id) in &chain_next_by_spell_id {
            parents_by_spell_id
                .entry(next_spell_id)
                .or_default()
                .insert(spell_id);
            adjacent_by_spell_id
                .entry(spell_id)
                .or_default()
                .insert(next_spell_id);
            adjacent_by_spell_id
                .entry(next_spell_id)
                .or_default()
                .insert(spell_id);
        }

        let mut unvisited = adjacent_by_spell_id
            .keys()
            .copied()
            .collect::<BTreeSet<_>>();
        while let Some(component_start) = unvisited.first().copied() {
            let mut pending = vec![component_start];
            let mut component = BTreeSet::new();
            while let Some(spell_id) = pending.pop() {
                if !component.insert(spell_id) {
                    continue;
                }
                unvisited.remove(&spell_id);
                if let Some(adjacent) = adjacent_by_spell_id.get(&spell_id) {
                    pending.extend(
                        adjacent
                            .iter()
                            .rev()
                            .filter(|adjacent_spell_id| !component.contains(adjacent_spell_id))
                            .copied(),
                    );
                }
            }

            let component_spell_ids = component.iter().copied().collect::<Vec<_>>();
            let mut component_diagnostics = Vec::new();
            for &spell_id in &component_spell_ids {
                if chain_next_by_spell_id.get(&spell_id) == Some(&spell_id) {
                    component_diagnostics
                        .push(SpellChainLoadDiagnosticLikeCpp::SelfLoop { spell_id });
                }

                if let Some(predecessors) = parents_by_spell_id.get(&spell_id)
                    && predecessors.len() > 1
                {
                    component_diagnostics.push(
                        SpellChainLoadDiagnosticLikeCpp::MultiplePredecessors {
                            spell_id,
                            predecessor_spell_ids: predecessors.iter().copied().collect(),
                        },
                    );
                }
            }

            for spell_ids in
                spell_chain_cycles_like_cpp(&component_spell_ids, &chain_next_by_spell_id)
            {
                if spell_ids.len() > 1 {
                    component_diagnostics
                        .push(SpellChainLoadDiagnosticLikeCpp::Cycle { spell_ids });
                }
            }

            let roots = component_spell_ids
                .iter()
                .copied()
                .filter(|spell_id| !parents_by_spell_id.contains_key(spell_id))
                .collect::<Vec<_>>();
            let mut ordered_chain = Vec::new();
            if component_diagnostics.is_empty() && roots.len() == 1 {
                let mut current_spell_id = Some(roots[0]);
                let mut seen = BTreeSet::new();
                while let Some(spell_id) = current_spell_id {
                    if !seen.insert(spell_id) {
                        break;
                    }
                    ordered_chain.push(spell_id);
                    current_spell_id = chain_next_by_spell_id.get(&spell_id).copied();
                }

                if let Some(&spell_id) = ordered_chain.get(usize::from(u8::MAX)) {
                    component_diagnostics.push(SpellChainLoadDiagnosticLikeCpp::RankOutOfRange {
                        first_spell_id: roots[0],
                        spell_id,
                        rank: usize::from(u8::MAX) + 1,
                    });
                }
            }

            if !component_diagnostics.is_empty() {
                let shared_diagnostics: std::sync::Arc<[SpellChainLoadDiagnosticLikeCpp]> =
                    component_diagnostics.clone().into();
                for spell_id in component_spell_ids {
                    store
                        .indeterminate_by_spell_id_like_cpp
                        .insert(spell_id, shared_diagnostics.clone());
                }
                diagnostics_in_order_like_cpp.extend(component_diagnostics);
                continue;
            }

            // A weakly connected functional component with no cycle and no
            // merge has exactly one root and one path covering every node.
            // Keep a defensive fail-closed guard in case that invariant is
            // changed by a future graph representation.
            if roots.len() != 1 || ordered_chain.len() != component_spell_ids.len() {
                let diagnostic = SpellChainLoadDiagnosticLikeCpp::Cycle {
                    spell_ids: component_spell_ids.clone(),
                };
                let shared_diagnostics: std::sync::Arc<[SpellChainLoadDiagnosticLikeCpp]> =
                    vec![diagnostic.clone()].into();
                for spell_id in component_spell_ids {
                    store
                        .indeterminate_by_spell_id_like_cpp
                        .insert(spell_id, shared_diagnostics.clone());
                }
                diagnostics_in_order_like_cpp.push(diagnostic);
                continue;
            }

            let first_spell_id = ordered_chain[0];
            let last_spell_id = *ordered_chain.last().expect("non-empty rank chain");
            for (index, &spell_id) in ordered_chain.iter().enumerate() {
                let rank = u8::try_from(index + 1).expect("rank overflow diagnosed above");
                store.chains_by_spell_id.insert(
                    spell_id,
                    SpellChainNodeLikeCpp {
                        prev_spell_id: index.checked_sub(1).map(|previous| ordered_chain[previous]),
                        next_spell_id: ordered_chain.get(index + 1).copied(),
                        first_spell_id,
                        last_spell_id,
                        rank,
                    },
                );
            }
        }

        SpellChainLoadOutcomeLikeCpp {
            store,
            diagnostics_in_order_like_cpp,
        }
    }

    pub fn spell_chain_lookup_like_cpp(&self, spell_id: u32) -> SpellChainLookupLikeCpp<'_> {
        if let Some(diagnostics) = &self.global_indeterminate_like_cpp {
            return SpellChainLookupLikeCpp::Indeterminate(diagnostics);
        }
        if let Some(diagnostics) = self.indeterminate_by_spell_id_like_cpp.get(&spell_id) {
            return SpellChainLookupLikeCpp::Indeterminate(diagnostics);
        }

        self.chains_by_spell_id
            .get(&spell_id)
            .map(SpellChainLookupLikeCpp::Node)
            .unwrap_or(SpellChainLookupLikeCpp::Unranked)
    }

    pub fn indeterminate_diagnostics_for_spell_like_cpp(
        &self,
        spell_id: u32,
    ) -> Option<&[SpellChainLoadDiagnosticLikeCpp]> {
        if let Some(diagnostics) = &self.global_indeterminate_like_cpp {
            return Some(diagnostics);
        }
        self.indeterminate_by_spell_id_like_cpp
            .get(&spell_id)
            .map(AsRef::as_ref)
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

fn spell_chain_cycles_like_cpp(
    component_spell_ids: &[u32],
    chain_next_by_spell_id: &BTreeMap<u32, u32>,
) -> Vec<Vec<u32>> {
    let mut completed = BTreeSet::new();
    let mut cycles = Vec::new();

    for &start_spell_id in component_spell_ids {
        if completed.contains(&start_spell_id) {
            continue;
        }

        let mut path = Vec::new();
        let mut path_index_by_spell_id = BTreeMap::new();
        let mut current_spell_id = Some(start_spell_id);
        while let Some(spell_id) = current_spell_id {
            if completed.contains(&spell_id) {
                break;
            }
            if let Some(&cycle_start) = path_index_by_spell_id.get(&spell_id) {
                let mut cycle = path[cycle_start..].to_vec();
                if let Some((minimum_index, _)) = cycle
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, spell_id)| *spell_id)
                {
                    cycle.rotate_left(minimum_index);
                }
                cycles.push(cycle);
                break;
            }

            path_index_by_spell_id.insert(spell_id, path.len());
            path.push(spell_id);
            current_spell_id = chain_next_by_spell_id.get(&spell_id).copied();
        }
        completed.extend(path);
    }

    cycles.sort();
    cycles
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
    pub async fn load_like_cpp(
        db: &WorldDatabase,
        spell_exists: impl FnMut(u32) -> bool,
        area_exists: impl FnMut(u32) -> bool,
        quest_exists: impl FnMut(u32) -> bool,
    ) -> Result<SpellAreaLoadOutcomeLikeCpp> {
        let mut result = db
            .direct_query(WorldStatements::SEL_SPELL_AREA.sql())
            .await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(SpellAreaRowLikeCpp {
                    spell_id: result.try_read::<u32>(0).unwrap_or(0),
                    area_id: result.try_read::<u32>(1).unwrap_or(0),
                    quest_start: result.try_read::<u32>(2).unwrap_or(0),
                    quest_start_status: result.try_read::<u32>(3).unwrap_or(0),
                    quest_end_status: result.try_read::<u32>(4).unwrap_or(0),
                    quest_end: result.try_read::<u32>(5).unwrap_or(0),
                    aura_spell: result.try_read::<i32>(6).unwrap_or(0),
                    race_mask: result.try_read::<u64>(7).unwrap_or(0),
                    gender: result.try_read::<u8>(8).unwrap_or(GENDER_NONE_LIKE_CPP),
                    flags: result.try_read::<u8>(9).unwrap_or(0),
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_rows_like_cpp(
            rows,
            spell_exists,
            area_exists,
            quest_exists,
        ))
    }

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
    fn into_source_variant_like_cpp(self) -> SpellCustomAttributeSourceVariantLikeCpp {
        SpellCustomAttributeSourceVariantLikeCpp {
            spell_id: self.spell_id,
            difficulty: self.difficulty,
            effect_types: Some(
                self.effects
                    .into_iter()
                    .map(|effect| effect.effect)
                    .collect(),
            ),
        }
    }
}

/// Exact spell variant used while composing SQL custom attributes.
///
/// `effect_types == None` means that the variant exists but its effect payload is not represented
/// by the current source. Attributes that do not depend on an effect type can still be composed;
/// effect-dependent validation must fail closed instead of treating missing coverage as an empty
/// effect list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellCustomAttributeSourceVariantLikeCpp {
    pub spell_id: u32,
    pub difficulty: u32,
    pub effect_types: Option<Vec<u32>>,
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
    ShareDamageEffectCoverageUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellCustomAttributeLoadErrorLikeCpp {
    pub spell_id: u32,
    pub difficulty: Option<u32>,
    /// Raw SQL bits from the rejected row. Consumers can keep uncertainty
    /// scoped to the attributes that were actually requested.
    pub attributes: u32,
    pub kind: SpellCustomAttributeLoadErrorKindLikeCpp,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellCustomAttributeStoreLikeCpp {
    pub attributes_by_spell_and_difficulty: BTreeMap<SpellCustomAttributeKeyLikeCpp, u32>,
}

impl SpellCustomAttributeStoreLikeCpp {
    pub async fn load_like_cpp<SpellInfosById>(
        db: &WorldDatabase,
        spell_infos_by_id: SpellInfosById,
    ) -> Result<SpellCustomAttributeLoadOutcomeLikeCpp>
    where
        SpellInfosById: FnMut(u32) -> Vec<SpellCustomAttributeSourceSpellInfoLikeCpp>,
    {
        let mut spell_infos_by_id = spell_infos_by_id;
        Self::load_for_variants_like_cpp(db, move |spell_id| {
            spell_infos_by_id(spell_id)
                .into_iter()
                .map(SpellCustomAttributeSourceSpellInfoLikeCpp::into_source_variant_like_cpp)
                .collect()
        })
        .await
    }

    /// Loads SQL custom attributes over exact spell variants without requiring hydrated
    /// `SpellEffectInfo` values.
    pub async fn load_for_variants_like_cpp<VariantsById>(
        db: &WorldDatabase,
        variants_by_id: VariantsById,
    ) -> Result<SpellCustomAttributeLoadOutcomeLikeCpp>
    where
        VariantsById: FnMut(u32) -> Vec<SpellCustomAttributeSourceVariantLikeCpp>,
    {
        let mut result = db
            .direct_query(WorldStatements::SEL_SPELL_CUSTOM_ATTR.sql())
            .await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(SpellCustomAttributeRowLikeCpp {
                    spell_id: result.try_read::<u32>(0).unwrap_or(0),
                    attributes: result.try_read::<u32>(1).unwrap_or(0),
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_sql_rows_for_variants_like_cpp(
            rows,
            variants_by_id,
        ))
    }

    pub fn from_sql_rows_like_cpp<I, SpellInfosById>(
        rows: I,
        mut spell_infos_by_id: SpellInfosById,
    ) -> SpellCustomAttributeLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellCustomAttributeRowLikeCpp>,
        SpellInfosById: FnMut(u32) -> Vec<SpellCustomAttributeSourceSpellInfoLikeCpp>,
    {
        Self::from_sql_rows_for_variants_like_cpp(rows, move |spell_id| {
            spell_infos_by_id(spell_id)
                .into_iter()
                .map(SpellCustomAttributeSourceSpellInfoLikeCpp::into_source_variant_like_cpp)
                .collect()
        })
    }

    pub fn from_sql_rows_for_variants_like_cpp<I, VariantsById>(
        rows: I,
        mut variants_by_id: VariantsById,
    ) -> SpellCustomAttributeLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellCustomAttributeRowLikeCpp>,
        VariantsById: FnMut(u32) -> Vec<SpellCustomAttributeSourceVariantLikeCpp>,
    {
        let mut store = Self::default();
        let mut loaded_row_count = 0;
        let mut applied_variant_count = 0;
        let mut errors = Vec::new();

        for row in rows {
            let variants = variants_by_id(row.spell_id);
            if variants.is_empty() {
                errors.push(SpellCustomAttributeLoadErrorLikeCpp {
                    spell_id: row.spell_id,
                    difficulty: None,
                    attributes: row.attributes,
                    kind: SpellCustomAttributeLoadErrorKindLikeCpp::SpellMissing,
                });
                continue;
            }

            for variant in variants {
                if row.attributes & SPELL_ATTR0_CU_SHARE_DAMAGE_LIKE_CPP != 0 {
                    match variant.effect_types.as_ref() {
                        None => {
                            errors.push(SpellCustomAttributeLoadErrorLikeCpp {
                                spell_id: row.spell_id,
                                difficulty: Some(variant.difficulty),
                                attributes: row.attributes,
                                kind: SpellCustomAttributeLoadErrorKindLikeCpp::ShareDamageEffectCoverageUnavailable,
                            });
                            continue;
                        }
                        Some(effect_types)
                            if !effect_types
                                .contains(&spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE) =>
                        {
                            errors.push(SpellCustomAttributeLoadErrorLikeCpp {
                                spell_id: row.spell_id,
                                difficulty: Some(variant.difficulty),
                                attributes: row.attributes,
                                kind: SpellCustomAttributeLoadErrorKindLikeCpp::ShareDamageWithoutSchoolDamage,
                            });
                            continue;
                        }
                        Some(_) => {}
                    }
                }

                let key = SpellCustomAttributeKeyLikeCpp {
                    spell_id: variant.spell_id,
                    difficulty: variant.difficulty,
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

    pub fn attributes_for_spell_any_difficulty_like_cpp(&self, spell_id: u32) -> u32 {
        self.attributes_by_spell_and_difficulty
            .range(
                SpellCustomAttributeKeyLikeCpp {
                    spell_id,
                    difficulty: 0,
                }..=SpellCustomAttributeKeyLikeCpp {
                    spell_id,
                    difficulty: u32::MAX,
                },
            )
            .fold(0, |attributes, (_, variant_attributes)| {
                attributes | variant_attributes
            })
    }

    pub fn has_attribute_any_difficulty_like_cpp(&self, spell_id: u32, attribute: u32) -> bool {
        self.attributes_for_spell_any_difficulty_like_cpp(spell_id) & attribute != 0
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
    pub async fn load_like_cpp(
        db: &WorldDatabase,
        spells: &SpellStore,
        spell_chains: &SpellChainStoreLikeCpp,
    ) -> Result<SpellGroupLoadOutcomeLikeCpp> {
        let stmt = db.prepare(WorldStatements::SEL_SPELL_GROUP);
        let mut result = db.query(&stmt).await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(SpellGroupRowLikeCpp {
                    group_id: result.try_read::<u32>(0).unwrap_or(0),
                    spell_id: result.try_read::<i32>(1).unwrap_or(0),
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_rows_like_cpp(
            rows,
            |spell_id| spells.get(spell_id as i32).is_some(),
            |spell_id| u32::from(spell_chains.spell_rank_like_cpp(spell_id)),
        ))
    }

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
    pub async fn load_like_cpp(
        db: &WorldDatabase,
        spell_groups: &SpellGroupStoreLikeCpp,
        spells: &SpellStore,
        spell_chains: &SpellChainStoreLikeCpp,
    ) -> Result<SpellGroupStackRuleLoadOutcomeLikeCpp> {
        let stmt = db.prepare(WorldStatements::SEL_SPELL_GROUP_STACK_RULES);
        let mut result = db.query(&stmt).await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(SpellGroupStackRuleRowLikeCpp {
                    group_id: result.try_read::<u32>(0).unwrap_or(0),
                    stack_rule: result.try_read::<u8>(1).unwrap_or(0),
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_rows_like_cpp(
            rows,
            spell_groups,
            |spell_id| spells.get(spell_id as i32).cloned(),
            |spell_id| {
                let next_spell_id = spell_chains.next_spell_in_chain_like_cpp(spell_id);
                (next_spell_id != 0).then_some(next_spell_id)
            },
        ))
    }

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
    pub async fn load_like_cpp<SourceSpells, Db2Rows, SpellLookup, SpellExists>(
        db: &WorldDatabase,
        source_spells: SourceSpells,
        db2_rows: Db2Rows,
        mut spell_lookup: SpellLookup,
        spell_exists: SpellExists,
    ) -> Result<SpellLearnSpellLoadOutcomeLikeCpp>
    where
        SourceSpells: IntoIterator<Item = SpellLearnSourceSpellInfoLikeCpp>,
        Db2Rows: IntoIterator<Item = crate::spell_db2::SpellLearnSpellEntry>,
        SpellLookup: FnMut(u32) -> Option<SpellLearnSourceSpellInfoLikeCpp>,
        SpellExists: FnMut(u32) -> bool,
    {
        let mut result = db
            .direct_query(WorldStatements::SEL_SPELL_LEARN_SPELL.sql())
            .await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(SpellLearnSpellSqlRowLikeCpp {
                    entry: result.try_read::<u32>(0).unwrap_or(0),
                    spell_id: result.try_read::<u32>(1).unwrap_or(0),
                    active: result.try_read::<u8>(2).unwrap_or(0) != 0,
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_sources_like_cpp(
            rows,
            source_spells,
            db2_rows,
            &mut spell_lookup,
            spell_exists,
        ))
    }

    /// Compose the represented C++ learning graph.
    ///
    /// This intentionally repairs the legacy `SpellMgr::LoadSpellLearnSpells`
    /// empty-query early return: an empty world table contributes zero custom
    /// rows but does not suppress canonical `SpellEffect` or
    /// `SpellLearnSpell.db2` edges.
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
        let sql_result_empty = sql_rows.is_empty();

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
            sql_result_empty,
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

fn calculate_pct_i32_like_cpp(base: i32, pct: f32) -> i32 {
    ((base as f32) * pct / 100.0) as i32
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

    pub fn has_effect_like_cpp(&self, effect_type: u32) -> bool {
        self.effects
            .iter()
            .any(|effect| effect.effect == effect_type)
    }

    /// Returns every distinct primary-profession skill line referenced by a
    /// C++ `SPELL_EFFECT_SKILL` effect.
    ///
    /// C++ treats a missing `SkillLine` as non-primary. Rust's effective store
    /// can know an SQL-only record identity without hydrating category/parent
    /// payload, so an authorization caller must distinguish that case and fail
    /// closed rather than silently granting capacity.
    ///
    /// This returns ordered effect metadata only. It is not the set of skills
    /// learned by `Player::AddSpell`, whose C++ `SpellLearnSkillNode` selects
    /// a narrower outcome; the caller must resolve the actual learn path.
    ///
    /// The caller must first resolve this `SpellInfo` through the effective
    /// spell-key authority. This payload-only predicate cannot prove that a
    /// formerly hydrated spell remains effective after SQL/hotfix removal.
    pub fn primary_profession_skill_effect_ids_like_cpp(
        &self,
        skill_lines: &crate::skill_talent::SkillLineStore,
    ) -> Result<Vec<u32>, PrimaryProfessionSpellClassificationErrorLikeCpp> {
        let mut skill_effects: Vec<_> = self
            .effects
            .iter()
            .filter(|effect| effect.effect == spell_effect_types::SPELL_EFFECT_SKILL)
            .collect();
        skill_effects.sort_by_key(|effect| effect.effect_index);

        let mut seen_primary_skills = BTreeSet::new();
        let mut primary_skills = Vec::new();
        for effect in skill_effects {
            let skill_id = u32::try_from(effect.effect_misc_value_1).map_err(|_| {
                PrimaryProfessionSpellClassificationErrorLikeCpp::InvalidSkillId {
                    spell_id: self.spell_id,
                    effect_index: effect.effect_index,
                    skill_id: effect.effect_misc_value_1,
                }
            })?;
            let Some(is_primary) = skill_lines.is_primary_profession_skill_like_cpp(skill_id)
            else {
                return Err(
                    PrimaryProfessionSpellClassificationErrorLikeCpp::MissingSkillLinePayload {
                        spell_id: self.spell_id,
                        skill_id,
                    },
                );
            };
            if is_primary && seen_primary_skills.insert(skill_id) {
                primary_skills.push(skill_id);
            }
        }

        Ok(primary_skills)
    }

    /// C++ `SpellInfo::IsPrimaryProfession`.
    ///
    /// This is a boolean property of the spell's effects, not a description
    /// of which skills `Player::AddSpell` will learn. If partial metadata
    /// makes one effect undecidable, a later hydrated primary effect still
    /// proves the boolean result; otherwise the missing payload fails closed.
    pub fn is_primary_profession_like_cpp(
        &self,
        skill_lines: &crate::skill_talent::SkillLineStore,
    ) -> Result<bool, PrimaryProfessionSpellClassificationErrorLikeCpp> {
        let mut skill_effects: Vec<_> = self
            .effects
            .iter()
            .filter(|effect| effect.effect == spell_effect_types::SPELL_EFFECT_SKILL)
            .collect();
        skill_effects.sort_by_key(|effect| effect.effect_index);

        let mut undecidable = None;
        for effect in skill_effects {
            let skill_id = match u32::try_from(effect.effect_misc_value_1) {
                Ok(skill_id) => skill_id,
                Err(_) => {
                    undecidable.get_or_insert(
                        PrimaryProfessionSpellClassificationErrorLikeCpp::InvalidSkillId {
                            spell_id: self.spell_id,
                            effect_index: effect.effect_index,
                            skill_id: effect.effect_misc_value_1,
                        },
                    );
                    continue;
                }
            };
            match skill_lines.is_primary_profession_skill_like_cpp(skill_id) {
                Some(true) => return Ok(true),
                Some(false) => {}
                None => {
                    undecidable.get_or_insert(
                        PrimaryProfessionSpellClassificationErrorLikeCpp::MissingSkillLinePayload {
                            spell_id: self.spell_id,
                            skill_id,
                        },
                    );
                }
            }
        }

        undecidable.map_or(Ok(false), Err)
    }

    /// C++ `SpellInfo::IsPrimaryProfessionFirstRank`.
    ///
    /// `SpellInfo::GetRank()` returns one for an unranked spell. That differs
    /// intentionally from Rust's existing `SpellMgr::GetSpellRank`-shaped
    /// accessor, which returns zero when no chain node exists.
    pub fn is_primary_profession_first_rank_like_cpp(
        &self,
        skill_lines: &crate::skill_talent::SkillLineStore,
        spell_chains: &SpellChainStoreLikeCpp,
    ) -> Result<bool, PrimaryProfessionSpellClassificationErrorLikeCpp> {
        let spell_id = u32::try_from(self.spell_id).map_err(|_| {
            PrimaryProfessionSpellClassificationErrorLikeCpp::InvalidSpellId {
                spell_id: self.spell_id,
            }
        })?;
        let rank = match spell_chains.spell_chain_lookup_like_cpp(spell_id) {
            SpellChainLookupLikeCpp::Node(node) => node.rank,
            SpellChainLookupLikeCpp::Unranked => 1,
            SpellChainLookupLikeCpp::Indeterminate(_) => {
                // Preserve the other safe short-circuit in C++'s
                // `IsPrimaryProfession() && GetRank() == 1`: a spell proven
                // non-primary is false regardless of an ambiguous rank.
                return match self.is_primary_profession_like_cpp(skill_lines) {
                    Ok(false) => Ok(false),
                    Ok(true) | Err(_) => Err(
                        PrimaryProfessionSpellClassificationErrorLikeCpp::RankChainIndeterminate {
                            spell_id,
                        },
                    ),
                };
            }
        };
        // With complete C++ data this is equivalent to the original
        // `IsPrimaryProfession() && GetRank() == 1`. Resolving rank first also
        // avoids requiring partial SkillLine payload when rank already proves
        // the result false.
        if rank != 1 {
            return Ok(false);
        }

        self.is_primary_profession_like_cpp(skill_lines)
    }

    pub fn requires_spell_focus_like_cpp(&self) -> bool {
        self.requires_spell_focus != 0
    }

    /// Represented subset of C++ `SpellInfo::CalcPowerCost` (`SpellInfo.cpp:3984`).
    ///
    /// This covers the DB2 `ManaCost` flat amount and mana percentage costs used
    /// by early live casts. Aura/spellmod/NPC scaling and non-mana max-power DB2
    /// lookups are intentionally deferred.
    pub fn calc_power_costs_like_cpp(&self, caster_create_mana: i32) -> Vec<SpellPowerCostLikeCpp> {
        let mut costs = Vec::new();

        for power in &self.power_costs {
            // C++ skips this power entry unless the caster has the required aura.
            // The represented cast path has no full aura query yet, so fail-open
            // by ignoring gated costs rather than charging the wrong row.
            if power.required_aura_spell_id != 0 {
                continue;
            }

            let mut amount = power.mana_cost;
            if power.power_cost_pct != 0.0 {
                if power.power_type == PowerType::Mana as i8 {
                    amount +=
                        calculate_pct_i32_like_cpp(caster_create_mana.max(0), power.power_cost_pct);
                } else {
                    continue;
                }
            }

            Self::push_power_cost_like_cpp(&mut costs, power.power_type, amount);
        }

        costs
    }

    fn push_power_cost_like_cpp(
        costs: &mut Vec<SpellPowerCostLikeCpp>,
        power_type: i8,
        amount: i32,
    ) {
        if amount == 0 {
            return;
        }

        if let Some(existing) = costs.iter_mut().find(|cost| cost.power_type == power_type) {
            existing.amount = existing.amount.saturating_add(amount);
        } else {
            costs.push(SpellPowerCostLikeCpp { power_type, amount });
        }
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
        let mut value = f64::from(self.effect_base_points);
        match self.effect_die_sides {
            0 => {}
            1 => value += 1.0,
            die_sides if die_sides > 1 => value += f64::from(roll_die(1, die_sides)),
            die_sides => value += f64::from(roll_die(die_sides, 1)),
        }
        value.round() as i32
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

    pub fn is_mod_shapeshift_aura_like_cpp(&self) -> bool {
        self.effect == spell_effect_types::SPELL_EFFECT_APPLY_AURA
            && self.effect_aura == aura_types::SPELL_AURA_MOD_SHAPESHIFT
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SpellInterruptRowLikeCpp {
    key: (i32, u8),
    flags: ([u32; 2], [u32; 2]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SpellHitCategoriesRowLikeCpp {
    record_id: u32,
    category_id: u32,
    charge_category_id: u32,
    defense_type: i8,
    spell_mechanic: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SpellHitMiscRowLikeCpp {
    record_id: u32,
    school_mask: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SpellHitEffectMechanicRowLikeCpp {
    record_id: u32,
    mechanic: i32,
}

#[derive(Default)]
pub struct SpellStore {
    spells: HashMap<i32, SpellInfo>,
    spell_info_keys_like_cpp: crate::spell_info_keys::SpellInfoKeyStoreLikeCpp,
    spell_effects_by_difficulty: HashMap<(i32, u8), Vec<SpellEffectInfo>>,
    spell_misc_attributes: HashMap<i32, [u32; 15]>,
    spell_misc_attributes_by_difficulty: HashMap<(i32, u8), [u32; 15]>,
    spell_interrupt_flags: HashMap<(i32, u8), ([u32; 2], [u32; 2])>,
    spell_interrupt_rows_by_id: BTreeMap<u32, SpellInterruptRowLikeCpp>,
    spell_hit_categories_by_difficulty: HashMap<(i32, u8), SpellHitCategoriesRowLikeCpp>,
    spell_hit_misc_by_difficulty: HashMap<(i32, u8), SpellHitMiscRowLikeCpp>,
    spell_hit_effect_mechanics_by_difficulty:
        HashMap<(i32, u8), BTreeMap<u32, SpellHitEffectMechanicRowLikeCpp>>,
    spell_shapeshift_masks: HashMap<i32, (u64, u64)>,
    implicit_target_conditions: HashMap<(i32, u32), ConditionsReference>,
}

impl SpellStore {
    /// Create a new empty spell store.
    pub fn new() -> Self {
        Self {
            spells: HashMap::new(),
            spell_info_keys_like_cpp: crate::spell_info_keys::SpellInfoKeyStoreLikeCpp::default(),
            spell_effects_by_difficulty: HashMap::new(),
            spell_misc_attributes: HashMap::new(),
            spell_misc_attributes_by_difficulty: HashMap::new(),
            spell_interrupt_flags: HashMap::new(),
            spell_interrupt_rows_by_id: BTreeMap::new(),
            spell_hit_categories_by_difficulty: HashMap::new(),
            spell_hit_misc_by_difficulty: HashMap::new(),
            spell_hit_effect_mechanics_by_difficulty: HashMap::new(),
            spell_shapeshift_masks: HashMap::new(),
            implicit_target_conditions: HashMap::new(),
        }
    }

    fn make_pair64_like_cpp(low: i32, high: i32) -> u64 {
        u64::from(low as u32) | (u64::from(high as u32) << 32)
    }

    pub fn effects_for_difficulty_like_cpp(
        &self,
        spell_id: i32,
        requested_difficulty_id: u8,
        difficulty_store: Option<&crate::difficulty::DifficultyStore>,
    ) -> Option<&[SpellEffectInfo]> {
        let mut difficulty_id = requested_difficulty_id;
        let mut visited = HashSet::new();
        loop {
            if let Some(effects) = self
                .spell_effects_by_difficulty
                .get(&(spell_id, difficulty_id))
            {
                return Some(effects);
            }
            if difficulty_id == 0 || !visited.insert(difficulty_id) {
                break;
            }
            difficulty_id = difficulty_store
                .and_then(|store| store.get(u32::from(difficulty_id)))
                .map_or(0, |difficulty| difficulty.fallback_difficulty_id);
        }
        self.spells.get(&spell_id).map(|spell| spell.effects())
    }

    /// Resolve the fields consumed by C++ spell-hit logic through the
    /// requested difficulty and its `FallbackDifficultyID` chain.
    ///
    /// `SpellCategories`, `SpellMisc`, and every `SpellEffect` slot fall back
    /// independently. This matters when a difficulty overrides only one of
    /// those contributors.
    pub fn hit_metadata_for_difficulty_like_cpp(
        &self,
        spell_id: i32,
        requested_difficulty_id: u8,
        difficulty_store: Option<&crate::difficulty::DifficultyStore>,
    ) -> Option<SpellHitMetadataLikeCpp> {
        let mut metadata = SpellHitMetadataLikeCpp::default();
        let mut has_metadata = false;
        let mut categories_resolved = false;
        let mut misc_resolved = false;
        let mut difficulty_id = requested_difficulty_id;
        let mut visited = [false; 256];

        loop {
            let visited_slot = &mut visited[usize::from(difficulty_id)];
            if *visited_slot {
                break;
            }
            *visited_slot = true;

            if !categories_resolved
                && let Some(categories) = self
                    .spell_hit_categories_by_difficulty
                    .get(&(spell_id, difficulty_id))
            {
                metadata.category_id = categories.category_id;
                metadata.charge_category_id = categories.charge_category_id;
                metadata.defense_type = categories.defense_type;
                metadata.spell_mechanic = categories.spell_mechanic;
                categories_resolved = true;
                has_metadata = true;
            }
            if !misc_resolved
                && let Some(misc) = self
                    .spell_hit_misc_by_difficulty
                    .get(&(spell_id, difficulty_id))
            {
                metadata.school_mask = misc.school_mask;
                misc_resolved = true;
                has_metadata = true;
            }
            if let Some(effect_mechanics) = self
                .spell_hit_effect_mechanics_by_difficulty
                .get(&(spell_id, difficulty_id))
            {
                has_metadata = true;
                for (&effect_index, effect) in effect_mechanics {
                    metadata
                        .effect_mechanics
                        .entry(effect_index)
                        .or_insert(effect.mechanic);
                }
            }

            if difficulty_id == 0 {
                break;
            }
            difficulty_id = difficulty_store
                .and_then(|store| store.get(u32::from(difficulty_id)))
                .map_or(0, |difficulty| difficulty.fallback_difficulty_id);
        }

        has_metadata.then_some(metadata)
    }

    fn empty_spell_info_like_cpp(spell_id: i32) -> SpellInfo {
        SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        }
    }

    fn spell_effect_from_db2_like_cpp(
        effect: &crate::spell_db2::SpellEffectDb2Entry,
    ) -> SpellEffectInfo {
        SpellEffectInfo {
            effect_index: u32::try_from(effect.effect_index).unwrap_or(0),
            effect: effect.effect,
            effect_aura: i32::from(effect.effect_aura),
            effect_base_points: effect.effect_base_points,
            effect_die_sides: effect.effect_die_sides,
            effect_spell_class_mask: effect.effect_spell_class_mask,
            effect_misc_value_1: effect.effect_misc_value[0],
            effect_misc_value_2: effect.effect_misc_value[1],
            effect_trigger_spell: effect.effect_trigger_spell,
            effect_radius_index_1: effect.effect_radius_index[0],
            position_facing: effect.effect_pos_facing,
            chain_targets: effect.effect_chain_targets,
            implicit_target_1: u32::try_from(effect.implicit_target[0]).unwrap_or(0),
            implicit_target_2: u32::try_from(effect.implicit_target[1]).unwrap_or(0),
        }
    }

    fn hydrate_primary_effect_like_cpp(info: &mut SpellInfo) {
        info.effects.sort_by_key(|effect| effect.effect_index);
        if let Some(primary) = info.effects.iter().find(|effect| effect.effect != 0) {
            info.effect_type = primary.effect;
            info.effect_base_points = primary.effect_base_points;
            info.effect_bonus_coefficient = 0.0;
            info.aura_type = Some(primary.effect_aura);
        }
    }

    fn merge_spell_info_like_cpp(&mut self, mut incoming: SpellInfo) {
        Self::hydrate_primary_effect_like_cpp(&mut incoming);
        let entry = self
            .spells
            .entry(incoming.spell_id)
            .or_insert_with(|| Self::empty_spell_info_like_cpp(incoming.spell_id));
        entry.cast_time_ms = incoming.cast_time_ms;
        entry.cooldown_ms = incoming.cooldown_ms;
        entry.recovery_time_ms = incoming.recovery_time_ms;
        entry.requires_spell_focus = incoming.requires_spell_focus;
        entry.display_flags = incoming.display_flags;
        if !incoming.power_costs.is_empty() {
            entry.power_costs = incoming.power_costs;
        }

        let overlays_effects = !incoming.effects.is_empty();
        if overlays_effects {
            for hotfix_effect in incoming.effects {
                entry
                    .effects
                    .retain(|effect| effect.effect_index != hotfix_effect.effect_index);
                entry.effects.push(hotfix_effect);
            }
        }

        Self::hydrate_primary_effect_like_cpp(entry);
        if overlays_effects {
            self.spell_effects_by_difficulty
                .insert((entry.spell_id, 0), entry.effects.clone());
        }
    }

    fn merge_spell_misc_attributes_like_cpp(&mut self, incoming: HashMap<i32, [u32; 15]>) {
        for (spell_id, attributes) in incoming {
            self.spell_misc_attributes.insert(spell_id, attributes);
            self.spell_misc_attributes_by_difficulty
                .insert((spell_id, 0), attributes);
        }
    }

    /// Load base spell data from DB2 and overlay SQL hotfix rows.
    ///
    /// C++ builds `SpellInfo` primarily from `sSpellEffectStore` and
    /// `sSpellMiscStore` (`SpellMgr::LoadSpellInfoStore`) and then applies
    /// hotfix data through the DB2 hotfix pipeline. Mount spells commonly
    /// exist only in `SpellEffect.db2`/`SpellMisc.db2`, so a hotfix-only
    /// loader makes account mounts fail as unknown or effectless spells.
    pub async fn load_with_db2_and_hotfixes(
        data_dir: &str,
        locale: &str,
        hotfix_db: &HotfixDatabase,
        spell_name_store: &crate::spell_db2::SpellNameStore,
        hotfix_removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<Self> {
        let spell_info_keys_like_cpp =
            crate::spell_info_keys::SpellInfoKeyStoreLikeCpp::load_like_cpp(
                data_dir,
                locale,
                hotfix_db,
                spell_name_store,
                hotfix_removals,
            )
            .await?;
        let spell_categories_store =
            crate::spell_db2::SpellCategoriesStore::load_effective_like_cpp(
                data_dir,
                locale,
                hotfix_db,
                hotfix_removals,
            )
            .await?;
        let spell_misc_store = crate::spell_db2::SpellMiscStore::load_effective_like_cpp(
            data_dir,
            locale,
            hotfix_db,
            hotfix_removals,
        )
        .await?;
        let spell_effect_store = crate::spell_db2::SpellEffectDb2Store::load_effective_like_cpp(
            data_dir,
            locale,
            hotfix_db,
            hotfix_removals,
        )
        .await?;
        let spell_shapeshift_store =
            crate::spell_db2::SpellShapeshiftStore::load(data_dir, locale)?;
        let spell_interrupts_store =
            crate::spell_db2::SpellInterruptsStore::load(data_dir, locale)?;
        let mut store = Self::from_spell_db2_stores_like_cpp(
            &spell_categories_store,
            &spell_misc_store,
            &spell_effect_store,
            &spell_shapeshift_store,
        );
        store.spell_info_keys_like_cpp = spell_info_keys_like_cpp;
        store.apply_db2_interrupts_like_cpp(&spell_interrupts_store);
        let hotfix_interrupt_rows = store.apply_hotfix_interrupts_like_cpp(hotfix_db).await?;
        if hotfix_interrupt_rows != 0 {
            info!("Loaded {hotfix_interrupt_rows} SpellInterrupts hotfix rows");
        }

        let hotfix_store = Self::load(hotfix_db).await?;
        for spell in hotfix_store.spells.into_values() {
            store.merge_spell_info_like_cpp(spell);
        }
        store.merge_spell_misc_attributes_like_cpp(hotfix_store.spell_misc_attributes);

        // [M0.1/#14] Join DB2 SpellCastTimes via SpellMisc.CastingTimeIndex AFTER the
        // hotfix merge (the merge overwrites cast_time_ms). C++ sSpellCastTimesStore.
        let spell_cast_times_store = crate::spell_db2::SpellCastTimesStore::load(data_dir, locale)?;
        store.apply_db2_cast_times_like_cpp(&spell_misc_store, &spell_cast_times_store);

        // [M0.1/#14] Join DB2 SpellCooldowns (per-spell cooldown), also after the merge.
        let spell_cooldowns_store = crate::spell_db2::SpellCooldownsStore::load_effective_like_cpp(
            data_dir,
            locale,
            hotfix_db,
            hotfix_removals,
        )
        .await?;
        store.apply_db2_cooldowns_like_cpp(&spell_cooldowns_store);

        // [M0.1/#72] C++ SpellMgr loads SpellInfo::PowerCosts from
        // `sSpellPowerStore` after base SpellInfo construction.
        let spell_power_store = crate::spell_db2::SpellPowerStore::load(data_dir, locale)?;
        let spell_power_difficulty_store =
            crate::spell_db2::SpellPowerDifficultyStore::load(data_dir, locale)?;
        store.apply_db2_power_costs_like_cpp(&spell_power_store, &spell_power_difficulty_store);
        store.apply_interrupt_flag_corrections_like_cpp();

        info!(
            "Loaded {} spells from SpellMisc/SpellEffect DB2 with hotfix overlay",
            store.spells.len()
        );
        Ok(store)
    }

    /// Whether C++ `SpellMgr::GetSpellInfo` has an exact regular-spell key.
    ///
    /// This is deliberately separate from [`Self::get`]. `get` exposes the
    /// subset of `SpellInfo` payload fields Rust currently hydrates, whereas
    /// C++ creates existence keys from twenty DB2 contributors.
    pub fn contains_spell_info_exact_like_cpp(&self, spell_id: u32, difficulty_id: u8) -> bool {
        self.spell_info_keys_like_cpp
            .contains_exact_like_cpp(spell_id, difficulty_id)
    }

    /// Exact regular `SpellInfo` keys in deterministic `(SpellID, Difficulty)` order.
    pub fn spell_info_keys_in_order_like_cpp(&self) -> Vec<(u32, u8)> {
        self.spell_info_keys_like_cpp.exact_keys_in_order_like_cpp()
    }

    /// Whether C++ `GetSpellInfo(id, DIFFICULTY_NONE)` would find a regular
    /// or server-side spell.
    ///
    /// The shipped DB2 has no difficulty-zero row, but C++ permits SQL
    /// overlays to add one and then follows its `FallbackDifficultyID` chain.
    /// Keep that behavior for loader foreign-key checks while stopping an
    /// invalid custom cycle instead of hanging startup forever.
    pub fn contains_spell_info_difficulty_none_like_cpp(
        &self,
        serverside_spells: &ServersideSpellStoreLikeCpp,
        difficulty_store: &crate::difficulty::DifficultyStore,
        spell_id: u32,
    ) -> bool {
        let mut difficulty_id = 0u8;
        let mut visited = HashSet::new();
        loop {
            if !visited.insert(difficulty_id) {
                return false;
            }
            if self.contains_spell_info_exact_like_cpp(spell_id, difficulty_id)
                || serverside_spells
                    .get_serverside_spell_like_cpp(spell_id, u32::from(difficulty_id))
                    .is_some()
            {
                return true;
            }
            let Some(difficulty) = difficulty_store.get(u32::from(difficulty_id)) else {
                return false;
            };
            difficulty_id = difficulty.fallback_difficulty_id;
        }
    }

    /// Whether C++ `_GetSpellInfo(id)` would find any regular difficulty.
    pub fn contains_spell_info_any_difficulty_like_cpp(&self, spell_id: u32) -> bool {
        self.spell_info_keys_like_cpp
            .contains_any_difficulty_like_cpp(spell_id)
    }

    pub fn spell_info_key_count_like_cpp(&self) -> usize {
        self.spell_info_keys_like_cpp.len()
    }

    fn apply_db2_hit_metadata_like_cpp(
        &mut self,
        spell_categories_store: &crate::spell_db2::SpellCategoriesStore,
        spell_misc_store: &crate::spell_db2::SpellMiscStore,
        spell_effect_store: &crate::spell_db2::SpellEffectDb2Store,
    ) {
        for categories in spell_categories_store.entries_like_cpp() {
            let Ok(spell_id) = i32::try_from(categories.spell_id) else {
                continue;
            };
            let row = SpellHitCategoriesRowLikeCpp {
                record_id: categories.id,
                // C++ assigns the signed DB2 fields directly into the
                // corresponding uint32 SpellInfo members.
                category_id: categories.category as u32,
                charge_category_id: categories.charge_category as u32,
                defense_type: categories.defense_type,
                spell_mechanic: categories.mechanic,
            };
            self.spell_hit_categories_by_difficulty
                .entry((spell_id, categories.difficulty_id))
                .and_modify(|current| {
                    if row.record_id > current.record_id {
                        *current = row;
                    }
                })
                .or_insert(row);
        }

        for misc in spell_misc_store.entries_like_cpp() {
            let Ok(spell_id) = i32::try_from(misc.spell_id) else {
                continue;
            };
            let row = SpellHitMiscRowLikeCpp {
                record_id: misc.id,
                school_mask: misc.school_mask,
            };
            self.spell_hit_misc_by_difficulty
                .entry((spell_id, misc.difficulty_id))
                .and_modify(|current| {
                    if row.record_id > current.record_id {
                        *current = row;
                    }
                })
                .or_insert(row);
        }

        for effect in spell_effect_store.entries_like_cpp() {
            let Ok(spell_id) = i32::try_from(effect.spell_id) else {
                continue;
            };
            let Ok(difficulty_id) = u8::try_from(effect.difficulty_id) else {
                continue;
            };
            let Ok(effect_index) = u32::try_from(effect.effect_index) else {
                continue;
            };
            if effect_index >= MAX_SPELL_EFFECTS_LIKE_CPP as u32 {
                continue;
            }
            let row = SpellHitEffectMechanicRowLikeCpp {
                record_id: effect.id,
                mechanic: effect.effect_mechanic,
            };
            self.spell_hit_effect_mechanics_by_difficulty
                .entry((spell_id, difficulty_id))
                .or_default()
                .entry(effect_index)
                .and_modify(|current| {
                    if row.record_id > current.record_id {
                        *current = row;
                    }
                })
                .or_insert(row);
        }
    }

    fn from_spell_db2_stores_like_cpp(
        spell_categories_store: &crate::spell_db2::SpellCategoriesStore,
        spell_misc_store: &crate::spell_db2::SpellMiscStore,
        spell_effect_store: &crate::spell_db2::SpellEffectDb2Store,
        spell_shapeshift_store: &crate::spell_db2::SpellShapeshiftStore,
    ) -> Self {
        let mut store = Self::new();

        store.apply_db2_hit_metadata_like_cpp(
            spell_categories_store,
            spell_misc_store,
            spell_effect_store,
        );

        for misc in spell_misc_store.entries_like_cpp() {
            let Ok(spell_id) = i32::try_from(misc.spell_id) else {
                continue;
            };
            let difficulty_id = misc.difficulty_id;
            let attributes = misc.attributes.map(|attribute| attribute as u32);
            store
                .spell_misc_attributes_by_difficulty
                .insert((spell_id, difficulty_id), attributes);
            if difficulty_id != 0 {
                continue;
            }
            store
                .spells
                .entry(spell_id)
                .or_insert_with(|| Self::empty_spell_info_like_cpp(spell_id));
            store.spell_misc_attributes.insert(spell_id, attributes);
        }

        for effect in spell_effect_store.entries_like_cpp() {
            if effect.effect == 0 {
                continue;
            }
            let Ok(spell_id) = i32::try_from(effect.spell_id) else {
                continue;
            };
            let Ok(difficulty_id) = u8::try_from(effect.difficulty_id) else {
                continue;
            };
            let converted = Self::spell_effect_from_db2_like_cpp(effect);
            store
                .spell_effects_by_difficulty
                .entry((spell_id, difficulty_id))
                .or_default()
                .push(converted.clone());
            if difficulty_id != 0 {
                continue;
            }
            let spell = store
                .spells
                .entry(spell_id)
                .or_insert_with(|| Self::empty_spell_info_like_cpp(spell_id));
            spell.effects.push(converted);
        }

        for shapeshift in spell_shapeshift_store.entries_like_cpp() {
            if shapeshift.spell_id <= 0 {
                continue;
            }
            store.spell_shapeshift_masks.insert(
                shapeshift.spell_id,
                (
                    Self::make_pair64_like_cpp(
                        shapeshift.shapeshift_mask[0],
                        shapeshift.shapeshift_mask[1],
                    ),
                    Self::make_pair64_like_cpp(
                        shapeshift.shapeshift_exclude[0],
                        shapeshift.shapeshift_exclude[1],
                    ),
                ),
            );
        }

        for spell in store.spells.values_mut() {
            Self::hydrate_primary_effect_like_cpp(spell);
        }

        store
    }

    /// C++ `SpellMgr::LoadSpellInfoStore` copies the difficulty-specific
    /// `SpellInterrupts` row into `SpellInfo`. The current Rust `SpellInfo`
    /// keeps related DB2 joins in `SpellStore`, so retain both interrupt masks
    /// here without widening every dynamically constructed test SpellInfo.
    fn apply_db2_interrupts_like_cpp(
        &mut self,
        spell_interrupts_store: &crate::spell_db2::SpellInterruptsStore,
    ) {
        for interrupts in spell_interrupts_store.entries_like_cpp() {
            self.store_signed_interrupt_row_by_id_like_cpp(
                interrupts.id,
                interrupts.spell_id,
                interrupts.difficulty_id,
                interrupts.aura_interrupt_flags,
                interrupts.channel_interrupt_flags,
            );
        }
        self.rebuild_interrupt_flags_from_rows_like_cpp();
    }

    /// Apply one file/hotfix `SpellInterrupts` row. DB2 stores the bit fields
    /// as signed integers, while C++ preserves their complete `uint32` bit
    /// pattern in `SpellInfo`.
    fn store_signed_interrupt_row_by_id_like_cpp(
        &mut self,
        row_id: u32,
        spell_id: u32,
        difficulty_id: u8,
        aura_interrupt_flags: [i32; 2],
        channel_interrupt_flags: [i32; 2],
    ) -> bool {
        let Ok(spell_id) = i32::try_from(spell_id) else {
            return false;
        };
        self.spell_interrupt_rows_by_id.insert(
            row_id,
            SpellInterruptRowLikeCpp {
                key: (spell_id, difficulty_id),
                flags: (
                    aura_interrupt_flags.map(|flag| flag as u32),
                    channel_interrupt_flags.map(|flag| flag as u32),
                ),
            },
        );
        true
    }

    /// Rebuild the relational lookup once per load phase. C++ DB2 storage is
    /// indexed and iterated by ascending record ID, so later IDs win if two
    /// records resolve to the same spell/difficulty key.
    fn rebuild_interrupt_flags_from_rows_like_cpp(&mut self) {
        self.spell_interrupt_flags.clear();
        for row in self.spell_interrupt_rows_by_id.values() {
            self.spell_interrupt_flags.insert(row.key, row.flags);
        }
    }

    /// Overlay the typed hotfix mirror of `SpellInterrupts.db2` after the
    /// client-file rows. C++ `DB2StorageBase::LoadFromDB` loads official rows
    /// first and custom rows second; a present SQL row replaces its exact DB2
    /// record ID before the relational spell/difficulty lookup is rebuilt.
    async fn apply_hotfix_interrupts_like_cpp(&mut self, db: &HotfixDatabase) -> Result<usize> {
        let mut count = 0usize;
        for official in [true, false] {
            let mut stmt = db.prepare(HotfixStatements::SEL_SPELL_INTERRUPTS);
            stmt.set_bool(0, official);
            let mut result = db.query(&stmt).await?;
            if result.is_empty() {
                continue;
            }

            loop {
                let difficulty_id = result.try_read::<u8>(1).unwrap_or(0);
                if let (Some(row_id), Some(spell_id)) =
                    (result.try_read::<u32>(0), result.try_read::<u32>(7))
                {
                    count += usize::from(self.store_signed_interrupt_row_by_id_like_cpp(
                        row_id,
                        spell_id,
                        difficulty_id,
                        [
                            result.try_read::<i32>(3).unwrap_or(0),
                            result.try_read::<i32>(4).unwrap_or(0),
                        ],
                        [
                            result.try_read::<i32>(5).unwrap_or(0),
                            result.try_read::<i32>(6).unwrap_or(0),
                        ],
                    ));
                }

                if !result.next_row() {
                    break;
                }
            }
        }
        self.rebuild_interrupt_flags_from_rows_like_cpp();
        Ok(count)
    }

    /// Import world-DB `serverside_spell` interrupt masks into the same
    /// effective lookup used by live aura/channel decisions. C++ inserts these
    /// SpellInfo rows before applying corrections; effective file plus SQL
    /// `SpellName` IDs were already rejected while the server-side store was
    /// built.
    pub fn apply_serverside_spell_interrupts_like_cpp(
        &mut self,
        serverside_spells: &ServersideSpellStoreLikeCpp,
    ) {
        for info in serverside_spells
            .spell_infos_by_spell_and_difficulty
            .values()
        {
            let Ok(spell_id) = i32::try_from(info.row.spell_id) else {
                continue;
            };
            self.insert_spell_interrupt_flags_for_difficulty_like_cpp(
                spell_id,
                info.row.difficulty_id as u8,
                info.row.aura_interrupt_flags,
                info.row.channel_interrupt_flags,
            );
        }
        self.apply_interrupt_flag_corrections_like_cpp();
    }

    /// Interrupt-mask subset of C++ `SpellMgr::LoadSpellInfoCorrections`.
    /// `ApplySpellFix` mutates every difficulty variant, so update every stored
    /// key for each affected spell after DB2/hotfix/server-side composition.
    fn apply_interrupt_flag_corrections_like_cpp(&mut self) {
        const HOSTILE_ACTION_RECEIVED: u32 = 0x0000_0001;
        const DAMAGE: u32 = 0x0000_0002;
        const ACTION: u32 = 0x0000_0004;
        const MOVING: u32 = 0x0000_0008;
        const ANIM: u32 = 0x0000_0020;
        const LEAVE_WORLD: u32 = 0x0008_0000;

        for spell_id in [61_719, 29_726, 63_414, 24_314, 99_252] {
            if self.spells.contains_key(&spell_id)
                && !self
                    .spell_interrupt_flags
                    .keys()
                    .any(|(known_spell_id, _)| *known_spell_id == spell_id)
            {
                self.spell_interrupt_flags
                    .insert((spell_id, 0), ([0; 2], [0; 2]));
            }
        }

        for ((spell_id, _), (aura, channel)) in &mut self.spell_interrupt_flags {
            match *spell_id {
                // Easter Lay Noblegarden Egg Aura.
                61_719 => aura[0] = HOSTILE_ACTION_RECEIVED | DAMAGE,
                // Test Ribbon Pole Channel.
                29_726 => channel[0] &= !ACTION,
                // Spinning Up (Mimiron).
                63_414 => *channel = [0; 2],
                // Threatening Gaze.
                24_314 => aura[0] |= ACTION | MOVING | ANIM,
                // Blaze of Glory.
                99_252 => aura[0] |= LEAVE_WORLD,
                _ => {}
            }
        }
    }

    /// [M0.1/#14] Apply DB2 cast times onto already-built SpellInfo rows.
    ///
    /// Mirrors the C++ SpellInfo ctor `CastTimeEntry =
    /// sSpellCastTimesStore.LookupEntry(_misc->CastingTimeIndex)` (SpellInfo.cpp:1185)
    /// + `CalcCastTime`: cast time = `max(Base, Minimum)`, clamped to ≥ 0
    /// (SpellInfo.cpp:3922). Must run AFTER the hotfix merge, which overwrites
    /// `cast_time_ms` (and would clobber this back to 0).
    fn apply_db2_cast_times_like_cpp(
        &mut self,
        spell_misc_store: &crate::spell_db2::SpellMiscStore,
        spell_cast_times_store: &crate::spell_db2::SpellCastTimesStore,
    ) {
        for misc in spell_misc_store.entries_like_cpp() {
            if misc.difficulty_id != 0 || misc.casting_time_index == 0 {
                continue;
            }
            let Ok(spell_id) = i32::try_from(misc.spell_id) else {
                continue;
            };
            let Some(entry) = spell_cast_times_store.get(u32::from(misc.casting_time_index)) else {
                continue;
            };
            if let Some(spell) = self.spells.get_mut(&spell_id) {
                spell.cast_time_ms = entry.base.max(entry.minimum).max(0) as u32;
            }
        }
    }

    /// [M0.1/#14] Apply DB2 per-spell cooldowns onto already-built SpellInfo rows.
    ///
    /// Mirrors C++ SpellInfo `RecoveryTime/CategoryRecoveryTime` from
    /// `sSpellCooldownsStore` (SpellInfo.cpp:1263) and `GetRecoveryTime() =
    /// max(RecoveryTime, CategoryRecoveryTime)` (SpellInfo.cpp:3981) — the per-spell
    /// cooldown the cast gate checks (`recovery_time_ms`). `StartRecoveryTime` (the
    /// GCD) is a separate mechanic and is intentionally left to the GCD path.
    /// Must run AFTER the hotfix merge (which overwrites `recovery_time_ms`).
    fn apply_db2_cooldowns_like_cpp(
        &mut self,
        spell_cooldowns_store: &crate::spell_db2::SpellCooldownsStore,
    ) {
        for entry in spell_cooldowns_store.entries_like_cpp() {
            if entry.difficulty_id != 0 {
                continue;
            }
            let Ok(spell_id) = i32::try_from(entry.spell_id) else {
                continue;
            };
            if let Some(spell) = self.spells.get_mut(&spell_id) {
                spell.recovery_time_ms =
                    entry.recovery_time.max(entry.category_recovery_time).max(0) as u32;
            }
        }
    }

    /// [M0.1/#72] Apply DB2 power costs onto already-built SpellInfo rows.
    ///
    /// Mirrors C++ `SpellMgr::LoadSpellInfoStore`, which stores
    /// `SpellPowerEntry` rows in `SpellInfo::PowerCosts` keyed by
    /// `SpellID`/difficulty/order (`SpellMgr.cpp:2550`, `DB2Stores.cpp:301`).
    fn apply_db2_power_costs_like_cpp(
        &mut self,
        spell_power_store: &crate::spell_db2::SpellPowerStore,
        spell_power_difficulty_store: &crate::spell_db2::SpellPowerDifficultyStore,
    ) {
        for spell in self.spells.values_mut() {
            spell.power_costs.clear();
        }

        for power in spell_power_store.entries_like_cpp() {
            if power.spell_id == 0 {
                continue;
            }
            let Ok(spell_id) = i32::try_from(power.spell_id) else {
                continue;
            };

            let (difficulty_id, order_index) = spell_power_difficulty_store
                .get(power.id)
                .map(|difficulty| (difficulty.difficulty_id, difficulty.order_index))
                .unwrap_or((0, power.order_index));
            if difficulty_id != 0 {
                continue;
            }

            let Some(spell) = self.spells.get_mut(&spell_id) else {
                continue;
            };
            let power_cost = SpellPowerCostInfoLikeCpp {
                order_index,
                power_type: power.power_type,
                mana_cost: power.mana_cost,
                mana_cost_per_level: power.mana_cost_per_level,
                mana_per_second: power.mana_per_second,
                power_cost_pct: power.power_cost_pct,
                power_cost_max_pct: power.power_cost_max_pct,
                power_pct_per_second: power.power_pct_per_second,
                required_aura_spell_id: power.required_aura_spell_id,
                optional_cost: power.optional_cost,
            };

            if let Some(existing) = spell
                .power_costs
                .iter_mut()
                .find(|existing| existing.order_index == order_index)
            {
                *existing = power_cost;
            } else {
                spell.power_costs.push(power_cost);
            }
            spell.power_costs.sort_by_key(|entry| entry.order_index);
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
    ON sm.ID = scr.SpellID
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
                    power_costs: Vec::new(),
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

    /// Resolve the `SpellMisc` attributes owned by the same difficulty-specific
    /// C++ `SpellInfo` selected by `SpellMgr::GetSpellInfo`.
    pub fn misc_attributes_for_difficulty_like_cpp(
        &self,
        spell_id: i32,
        requested_difficulty_id: u8,
        difficulty_store: Option<&crate::difficulty::DifficultyStore>,
    ) -> Option<[u32; 15]> {
        let mut difficulty_id = requested_difficulty_id;
        let mut visited = HashSet::new();
        loop {
            if let Some(attributes) = self
                .spell_misc_attributes_by_difficulty
                .get(&(spell_id, difficulty_id))
                .copied()
            {
                return Some(attributes);
            }
            if difficulty_id == 0 || !visited.insert(difficulty_id) {
                break;
            }
            difficulty_id = difficulty_store
                .and_then(|store| store.get(u32::from(difficulty_id)))
                .map_or(0, |difficulty| difficulty.fallback_difficulty_id);
        }
        self.spell_misc_attributes.get(&spell_id).copied()
    }

    pub fn has_attribute_for_difficulty_like_cpp(
        &self,
        spell_id: i32,
        requested_difficulty_id: u8,
        difficulty_store: Option<&crate::difficulty::DifficultyStore>,
        attribute_word: usize,
        attribute: u32,
    ) -> bool {
        self.misc_attributes_for_difficulty_like_cpp(
            spell_id,
            requested_difficulty_id,
            difficulty_store,
        )
        .and_then(|attributes| attributes.get(attribute_word).copied())
        .is_some_and(|attributes| attributes & attribute != 0)
    }

    /// C++ `SpellInfo::HasAttribute` for attributes hydrated from `SpellMisc.db2`.
    pub fn has_attribute0_like_cpp(&self, spell_id: i32, attribute: u32) -> bool {
        self.spell_misc_attributes
            .get(&spell_id)
            .is_some_and(|attributes| attributes[0] & attribute != 0)
    }

    /// C++ `SpellInfo::HasAttribute(SpellAttr1)` for attributes hydrated from `SpellMisc.db2`.
    pub fn has_attribute1_like_cpp(&self, spell_id: i32, attribute: u32) -> bool {
        self.spell_misc_attributes
            .get(&spell_id)
            .is_some_and(|attributes| attributes[1] & attribute != 0)
    }

    /// C++ `SpellInfo::HasAttribute(SpellAttr2)` for attributes hydrated from `SpellMisc.db2`.
    pub fn has_attribute2_like_cpp(&self, spell_id: i32, attribute: u32) -> bool {
        self.spell_misc_attributes
            .get(&spell_id)
            .is_some_and(|attributes| attributes[2] & attribute != 0)
    }

    /// C++ `SpellInfo::HasAttribute(SpellAttr4)` for attributes hydrated from `SpellMisc.db2`.
    pub fn has_attribute4_like_cpp(&self, spell_id: i32, attribute: u32) -> bool {
        self.spell_misc_attributes
            .get(&spell_id)
            .is_some_and(|attributes| attributes[4] & attribute != 0)
    }

    /// C++ `SpellInfo::HasAttribute(SpellAttr8)` for attributes hydrated from `SpellMisc.db2`.
    pub fn has_attribute8_like_cpp(&self, spell_id: i32, attribute: u32) -> bool {
        self.spell_misc_attributes
            .get(&spell_id)
            .is_some_and(|attributes| attributes[8] & attribute != 0)
    }

    /// C++ `SpellInfo::Stances` / `StancesNot` for login passive-cast gates.
    pub fn shapeshift_masks_like_cpp(&self, spell_id: i32) -> (u64, u64) {
        self.spell_shapeshift_masks
            .get(&spell_id)
            .copied()
            .unwrap_or((0, 0))
    }

    /// C++ `SpellInfo::IsPassive`, for the represented paths that currently
    /// only need the `SPELL_ATTR0_PASSIVE` gate.
    pub fn is_passive_like_cpp(&self, spell_id: i32) -> bool {
        self.has_attribute0_like_cpp(spell_id, attributes::SPELL_ATTR0_PASSIVE)
    }

    /// C++ `SpellInfo::IsChanneled`.
    pub fn is_channeled_like_cpp(&self, spell_id: i32) -> bool {
        self.has_attribute1_like_cpp(
            spell_id,
            attributes::SPELL_ATTR1_IS_CHANNELLED | attributes::SPELL_ATTR1_IS_SELF_CHANNELLED,
        )
    }

    /// Resolve the C++ `SpellInterrupts` row for one spell/difficulty.
    ///
    /// `SpellMgr::GetSpellInfo` tries the exact map difficulty before walking
    /// `DifficultyEntry::FallbackDifficultyID`. Keep both aura and channel
    /// words coupled to the same selected row rather than merging metadata
    /// across difficulties.
    pub fn interrupt_flags_for_difficulty_like_cpp(
        &self,
        spell_id: i32,
        requested_difficulty_id: u8,
        difficulty_store: Option<&crate::difficulty::DifficultyStore>,
    ) -> Option<([u32; 2], [u32; 2])> {
        let mut difficulty_id = requested_difficulty_id;
        let mut visited = [false; 256];
        loop {
            if let Some(flags) = self
                .spell_interrupt_flags
                .get(&(spell_id, difficulty_id))
                .copied()
            {
                return Some(flags);
            }

            let visited_entry = &mut visited[usize::from(difficulty_id)];
            if *visited_entry {
                return None;
            }
            *visited_entry = true;

            difficulty_id = difficulty_store?.fallback_difficulty_id_like_cpp(difficulty_id)?;
        }
    }

    /// C++ `SpellInfo::HasAuraInterruptFlag` for the two
    /// `SpellAuraInterruptFlags` words loaded from difficulty zero.
    ///
    /// Transitional callers without map context retain the original base-row
    /// behavior; live paths should call the difficulty-aware variant.
    pub fn aura_interrupt_flags_like_cpp(&self, spell_id: i32) -> Option<[u32; 2]> {
        self.aura_interrupt_flags_for_difficulty_like_cpp(spell_id, 0, None)
    }

    pub fn aura_interrupt_flags_for_difficulty_like_cpp(
        &self,
        spell_id: i32,
        requested_difficulty_id: u8,
        difficulty_store: Option<&crate::difficulty::DifficultyStore>,
    ) -> Option<[u32; 2]> {
        self.interrupt_flags_for_difficulty_like_cpp(
            spell_id,
            requested_difficulty_id,
            difficulty_store,
        )
        .map(|(aura, _)| aura)
    }

    pub fn has_aura_interrupt_flag_like_cpp(&self, spell_id: i32, flags: u32, flags2: u32) -> bool {
        self.aura_interrupt_flags_like_cpp(spell_id)
            .is_some_and(|known| {
                (flags != 0 && known[0] & flags != 0) || (flags2 != 0 && known[1] & flags2 != 0)
            })
    }

    /// C++ `SpellInfo::HasChannelInterruptFlag` for the two
    /// `SpellAuraInterruptFlags` words loaded from difficulty zero.
    pub fn channel_interrupt_flags_like_cpp(&self, spell_id: i32) -> Option<[u32; 2]> {
        self.channel_interrupt_flags_for_difficulty_like_cpp(spell_id, 0, None)
    }

    pub fn channel_interrupt_flags_for_difficulty_like_cpp(
        &self,
        spell_id: i32,
        requested_difficulty_id: u8,
        difficulty_store: Option<&crate::difficulty::DifficultyStore>,
    ) -> Option<[u32; 2]> {
        self.interrupt_flags_for_difficulty_like_cpp(
            spell_id,
            requested_difficulty_id,
            difficulty_store,
        )
        .map(|(_, channel)| channel)
    }

    pub fn has_channel_interrupt_flag_like_cpp(
        &self,
        spell_id: i32,
        flags: u32,
        flags2: u32,
    ) -> bool {
        self.channel_interrupt_flags_like_cpp(spell_id)
            .is_some_and(|known| {
                (flags != 0 && known[0] & flags != 0) || (flags2 != 0 && known[1] & flags2 != 0)
            })
    }

    /// Port of C++ `SpellInfo::CheckShapeshift` for regular `SpellInfo`
    /// entries composed by `SpellMgr::LoadSpellInfoStore`.
    pub fn check_shapeshift_like_cpp<'a, F>(
        &self,
        spell_id: i32,
        form: u32,
        mut lookup_form: F,
    ) -> Option<SpellCastResult>
    where
        F: FnMut(u32) -> Option<&'a crate::spell_db2::SpellShapeshiftFormEntry>,
    {
        self.spells.get(&spell_id)?;

        let (stances, stances_not) = self
            .spell_shapeshift_masks
            .get(&spell_id)
            .copied()
            .unwrap_or((0, 0));
        let attributes = self
            .spell_misc_attributes
            .get(&spell_id)
            .copied()
            .unwrap_or([0; 15]);
        let stance_mask = form
            .checked_sub(1)
            .and_then(|shift| 1u64.checked_shl(shift))
            .unwrap_or(0);

        if stance_mask & stances_not != 0 {
            return Some(SpellCastResult::NotShapeshift);
        }

        if stance_mask & stances != 0 {
            return Some(SpellCastResult::Success);
        }

        let mut act_as_shifted = false;
        let mut form_flags = 0;
        if form > 0 {
            let Some(shape_info) = lookup_form(form) else {
                return Some(SpellCastResult::Success);
            };
            form_flags = shape_info.flags;
            act_as_shifted = form_flags & shapeshift_form_flags::STANCE == 0;
        }

        if act_as_shifted {
            if attributes[0] & attributes::SPELL_ATTR0_NOT_SHAPESHIFTED != 0
                || form_flags & shapeshift_form_flags::CAN_ONLY_CAST_SHAPESHIFT_SPELLS != 0
            {
                return Some(SpellCastResult::NotShapeshift);
            }

            if stances != 0 {
                return Some(SpellCastResult::OnlyShapeshift);
            }
        } else if attributes[2] & attributes::SPELL_ATTR2_ALLOW_WHILE_NOT_SHAPESHIFTED_CASTER_FORM
            == 0
            && stances != 0
        {
            return Some(SpellCastResult::OnlyShapeshift);
        }

        Some(SpellCastResult::Success)
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

    #[allow(dead_code)]
    pub fn insert_spell_misc_attributes_like_cpp(&mut self, spell_id: i32, attributes: [u32; 15]) {
        self.spell_misc_attributes.insert(spell_id, attributes);
        self.spell_misc_attributes_by_difficulty
            .insert((spell_id, 0), attributes);
    }

    #[allow(dead_code)]
    pub fn insert_spell_misc_attributes_for_difficulty_like_cpp(
        &mut self,
        spell_id: i32,
        difficulty_id: u8,
        attributes: [u32; 15],
    ) {
        self.spell_misc_attributes_by_difficulty
            .insert((spell_id, difficulty_id), attributes);
        if difficulty_id == 0 {
            self.spell_misc_attributes.insert(spell_id, attributes);
        }
    }

    /// Insert one synthetic hit-metadata projection for focused tests or
    /// dynamic registration without widening `SpellInfo`/`SpellEffectInfo`.
    #[allow(dead_code)]
    pub fn insert_spell_hit_metadata_for_difficulty_like_cpp(
        &mut self,
        spell_id: i32,
        difficulty_id: u8,
        metadata: SpellHitMetadataLikeCpp,
    ) {
        let SpellHitMetadataLikeCpp {
            category_id,
            charge_category_id,
            defense_type,
            spell_mechanic,
            school_mask,
            effect_mechanics,
        } = metadata;
        self.spell_hit_categories_by_difficulty.insert(
            (spell_id, difficulty_id),
            SpellHitCategoriesRowLikeCpp {
                record_id: u32::MAX,
                category_id,
                charge_category_id,
                defense_type,
                spell_mechanic,
            },
        );
        self.spell_hit_misc_by_difficulty.insert(
            (spell_id, difficulty_id),
            SpellHitMiscRowLikeCpp {
                record_id: u32::MAX,
                school_mask,
            },
        );
        self.spell_hit_effect_mechanics_by_difficulty.insert(
            (spell_id, difficulty_id),
            effect_mechanics
                .into_iter()
                .filter(|(effect_index, _)| *effect_index < MAX_SPELL_EFFECTS_LIKE_CPP as u32)
                .map(|(effect_index, mechanic)| {
                    (
                        effect_index,
                        SpellHitEffectMechanicRowLikeCpp {
                            record_id: u32::MAX,
                            mechanic,
                        },
                    )
                })
                .collect(),
        );
    }

    #[allow(dead_code)]
    pub fn insert_spell_interrupt_flags_like_cpp(
        &mut self,
        spell_id: i32,
        aura_interrupt_flags: [u32; 2],
        channel_interrupt_flags: [u32; 2],
    ) {
        self.insert_spell_interrupt_flags_for_difficulty_like_cpp(
            spell_id,
            0,
            aura_interrupt_flags,
            channel_interrupt_flags,
        );
    }

    #[allow(dead_code)]
    pub fn insert_spell_interrupt_flags_for_difficulty_like_cpp(
        &mut self,
        spell_id: i32,
        difficulty_id: u8,
        aura_interrupt_flags: [u32; 2],
        channel_interrupt_flags: [u32; 2],
    ) {
        self.spell_interrupt_flags.insert(
            (spell_id, difficulty_id),
            (aura_interrupt_flags, channel_interrupt_flags),
        );
    }

    #[allow(dead_code)]
    pub fn insert_spell_shapeshift_masks_like_cpp(
        &mut self,
        spell_id: i32,
        stances: u64,
        stances_not: u64,
    ) {
        self.spell_shapeshift_masks
            .insert(spell_id, (stances, stances_not));
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

    fn test_skill_line_like_cpp(
        id: u32,
        category_id: i8,
        parent_skill_line_id: u32,
    ) -> crate::skill_talent::SkillLineEntry {
        crate::skill_talent::SkillLineEntry {
            id,
            display_name: String::new(),
            alternate_verb: String::new(),
            description: String::new(),
            horde_display_name: String::new(),
            override_source_info_display_name: String::new(),
            category_id,
            spell_icon_file_id: 0,
            can_link: 0,
            parent_skill_line_id,
            parent_tier_index: 0,
            flags: 0,
            spell_book_spell_id: 0,
        }
    }

    fn test_skill_effect_like_cpp(effect_index: u32, skill_id: i32) -> SpellEffectInfo {
        SpellEffectInfo {
            effect_index,
            effect: spell_effect_types::SPELL_EFFECT_SKILL,
            effect_misc_value_1: skill_id,
            ..Default::default()
        }
    }

    fn test_effect_like_cpp(effect_index: u32, effect_aura: i32) -> SpellEffectInfo {
        SpellEffectInfo {
            effect_index,
            effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_aura,
            effect_base_points: 0,
            effect_die_sides: 0,
            effect_spell_class_mask: [0; 4],
            effect_misc_value_1: 0,
            effect_misc_value_2: 0,
            effect_trigger_spell: 0,
            effect_radius_index_1: 0,
            position_facing: 0.0,
            chain_targets: 0,
            implicit_target_1: 0,
            implicit_target_2: 0,
        }
    }

    #[test]
    fn primary_profession_spell_classifier_matches_cpp_root_and_rank_rules() {
        let skill_lines = crate::skill_talent::SkillLineStore::from_entries([
            test_skill_line_like_cpp(100, 11, 0),
            test_skill_line_like_cpp(101, 11, 100),
            test_skill_line_like_cpp(200, 9, 0),
            test_skill_line_like_cpp(300, 11, 0),
        ]);
        let mut spell = SpellStore::empty_spell_info_like_cpp(1_000);
        spell.effects = vec![
            test_skill_effect_like_cpp(2, 100),
            test_skill_effect_like_cpp(1, 300),
            test_skill_effect_like_cpp(2, 101),
            test_skill_effect_like_cpp(3, 200),
            test_skill_effect_like_cpp(4, 300),
        ];

        assert_eq!(
            spell
                .primary_profession_skill_effect_ids_like_cpp(&skill_lines)
                .unwrap(),
            vec![300, 100],
            "primary lines follow C++ effect-index order and deduplicate at first appearance"
        );
        assert!(
            spell
                .is_primary_profession_first_rank_like_cpp(
                    &skill_lines,
                    &SpellChainStoreLikeCpp::default(),
                )
                .unwrap(),
            "C++ SpellInfo::GetRank returns one without a ChainEntry"
        );

        let rank_two = SpellChainStoreLikeCpp {
            chains_by_spell_id: BTreeMap::from([(
                1_000,
                SpellChainNodeLikeCpp {
                    prev_spell_id: Some(999),
                    next_spell_id: None,
                    first_spell_id: 999,
                    last_spell_id: 1_000,
                    rank: 2,
                },
            )]),
            ..SpellChainStoreLikeCpp::default()
        };
        assert!(
            !spell
                .is_primary_profession_first_rank_like_cpp(&skill_lines, &rank_two)
                .unwrap()
        );

        let mut unhydrated_rank_two = SpellStore::empty_spell_info_like_cpp(1_000);
        unhydrated_rank_two.effects = vec![test_skill_effect_like_cpp(0, 999)];
        let partial_skill_lines =
            crate::skill_talent::SkillLineStore::from_hydrated_entries_and_effective_ids_like_cpp(
                [test_skill_line_like_cpp(100, 11, 0)],
                [100, 999],
            );
        assert_eq!(
            unhydrated_rank_two
                .is_primary_profession_first_rank_like_cpp(&partial_skill_lines, &rank_two,),
            Ok(false),
            "rank two is decidably false without requiring unrelated partial payload"
        );

        let mut partly_hydrated_rank_one = SpellStore::empty_spell_info_like_cpp(1_001);
        partly_hydrated_rank_one.effects = vec![
            test_skill_effect_like_cpp(0, 999),
            test_skill_effect_like_cpp(1, 100),
        ];
        assert_eq!(
            partly_hydrated_rank_one.is_primary_profession_first_rank_like_cpp(
                &partial_skill_lines,
                &SpellChainStoreLikeCpp::default(),
            ),
            Ok(true),
            "one hydrated primary effect proves C++'s boolean result"
        );

        let mut only_unhydrated_rank_one = SpellStore::empty_spell_info_like_cpp(1_002);
        only_unhydrated_rank_one.effects = vec![test_skill_effect_like_cpp(0, 999)];
        assert_eq!(
            only_unhydrated_rank_one.is_primary_profession_first_rank_like_cpp(
                &partial_skill_lines,
                &SpellChainStoreLikeCpp::default(),
            ),
            Err(
                PrimaryProfessionSpellClassificationErrorLikeCpp::MissingSkillLinePayload {
                    spell_id: 1_002,
                    skill_id: 999,
                }
            )
        );
    }

    #[test]
    fn primary_profession_first_rank_preserves_safe_indeterminate_short_circuits() {
        let skill_lines = crate::skill_talent::SkillLineStore::from_entries([
            test_skill_line_like_cpp(100, 11, 0),
            test_skill_line_like_cpp(200, 9, 0),
        ]);
        let mut primary_spell = SpellStore::empty_spell_info_like_cpp(1_000);
        primary_spell.effects = vec![test_skill_effect_like_cpp(0, 100)];
        let mut non_primary_spell = SpellStore::empty_spell_info_like_cpp(1_001);
        non_primary_spell.effects = vec![test_skill_effect_like_cpp(0, 200)];

        let local_indeterminate =
            SpellChainStoreLikeCpp::from_skill_line_ability_rank_rows_with_diagnostics_like_cpp(
                [SkillLineAbilityRankRowLikeCpp::Indeterminate {
                    record_id: 90,
                    spell_raw: 1_000,
                    supercedes_spell_raw: i128::from(i32::MAX) + 1,
                }],
                |spell_id| spell_id == 1_000,
            )
            .store;
        assert_eq!(
            primary_spell
                .is_primary_profession_first_rank_like_cpp(&skill_lines, &local_indeterminate,),
            Err(
                PrimaryProfessionSpellClassificationErrorLikeCpp::RankChainIndeterminate {
                    spell_id: 1_000,
                }
            )
        );

        let global_indeterminate =
            SpellChainStoreLikeCpp::from_skill_line_ability_rank_rows_with_diagnostics_like_cpp(
                [SkillLineAbilityRankRowLikeCpp::Indeterminate {
                    record_id: 91,
                    spell_raw: i128::from(i32::MAX) + 1,
                    supercedes_spell_raw: i128::from(i32::MAX) + 2,
                }],
                |_| false,
            )
            .store;
        assert_eq!(
            primary_spell
                .is_primary_profession_first_rank_like_cpp(&skill_lines, &global_indeterminate,),
            Err(
                PrimaryProfessionSpellClassificationErrorLikeCpp::RankChainIndeterminate {
                    spell_id: 1_000,
                }
            )
        );
        assert_eq!(
            non_primary_spell
                .is_primary_profession_first_rank_like_cpp(&skill_lines, &global_indeterminate,),
            Ok(false),
            "C++'s false primary-profession operand decides the conjunction without rank"
        );
    }

    #[test]
    fn primary_profession_spell_classifier_distinguishes_absent_unhydrated_and_invalid_skill() {
        let skill_lines =
            crate::skill_talent::SkillLineStore::from_hydrated_entries_and_effective_ids_like_cpp(
                [test_skill_line_like_cpp(100, 11, 0)],
                [100, 999],
            );
        let mut unhydrated = SpellStore::empty_spell_info_like_cpp(1_000);
        unhydrated.effects = vec![test_skill_effect_like_cpp(0, 999)];
        assert_eq!(
            unhydrated.primary_profession_skill_effect_ids_like_cpp(&skill_lines),
            Err(
                PrimaryProfessionSpellClassificationErrorLikeCpp::MissingSkillLinePayload {
                    spell_id: 1_000,
                    skill_id: 999,
                }
            )
        );

        let mut absent = SpellStore::empty_spell_info_like_cpp(1_001);
        absent.effects = vec![test_skill_effect_like_cpp(0, 998)];
        assert_eq!(
            absent.primary_profession_skill_effect_ids_like_cpp(&skill_lines),
            Ok(Vec::new()),
            "a failed C++ LookupEntry is non-primary"
        );

        let mut invalid = SpellStore::empty_spell_info_like_cpp(1_002);
        invalid.effects = vec![test_skill_effect_like_cpp(2, -1)];
        assert_eq!(
            invalid.primary_profession_skill_effect_ids_like_cpp(&skill_lines),
            Err(
                PrimaryProfessionSpellClassificationErrorLikeCpp::InvalidSkillId {
                    spell_id: 1_002,
                    effect_index: 2,
                    skill_id: -1,
                }
            )
        );

        let mut invalid_spell = SpellStore::empty_spell_info_like_cpp(1_003);
        invalid_spell.spell_id = -1;
        invalid_spell.effects = vec![test_skill_effect_like_cpp(0, 100)];
        assert_eq!(
            invalid_spell.is_primary_profession_first_rank_like_cpp(
                &skill_lines,
                &SpellChainStoreLikeCpp::default(),
            ),
            Err(PrimaryProfessionSpellClassificationErrorLikeCpp::InvalidSpellId { spell_id: -1 })
        );
    }

    #[test]
    fn hotfix_base_effect_replaces_difficulty_zero_lookup_like_cpp() {
        let mut store = SpellStore::new();
        let mut base = SpellStore::empty_spell_info_like_cpp(100);
        base.effects
            .push(test_effect_like_cpp(0, aura_types::SPELL_AURA_MOD_THREAT));
        store.merge_spell_info_like_cpp(base);

        let mut hotfix = SpellStore::empty_spell_info_like_cpp(100);
        hotfix
            .effects
            .push(test_effect_like_cpp(0, aura_types::SPELL_AURA_MOD_TAUNT));
        store.merge_spell_info_like_cpp(hotfix);

        let effects = store
            .effects_for_difficulty_like_cpp(100, 0, None)
            .expect("hotfixed base effect");
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0].effect_aura, aura_types::SPELL_AURA_MOD_TAUNT);
    }

    #[test]
    fn misc_attributes_resolve_exact_difficulty_then_base_like_cpp() {
        let mut store = SpellStore::new();
        let mut base = [0; 15];
        base[1] = attributes::SPELL_ATTR1_NO_THREAT;
        store.insert_spell_misc_attributes_like_cpp(100, base);
        let mut heroic = [0; 15];
        heroic[4] = attributes::SPELL_ATTR4_NO_HARMFUL_THREAT;
        store.insert_spell_misc_attributes_for_difficulty_like_cpp(100, 2, heroic);

        assert!(store.has_attribute_for_difficulty_like_cpp(
            100,
            2,
            None,
            4,
            attributes::SPELL_ATTR4_NO_HARMFUL_THREAT,
        ));
        assert!(!store.has_attribute_for_difficulty_like_cpp(
            100,
            2,
            None,
            1,
            attributes::SPELL_ATTR1_NO_THREAT,
        ));
        assert!(store.has_attribute_for_difficulty_like_cpp(
            100,
            3,
            None,
            1,
            attributes::SPELL_ATTR1_NO_THREAT,
        ));
    }

    #[test]
    fn hit_metadata_composes_each_db2_contributor_and_effect_slot_like_cpp() {
        let spell_id = 90_001;
        let categories = |id, difficulty_id, category, charge_category, defense_type, mechanic| {
            crate::spell_db2::SpellCategoriesEntry {
                id,
                difficulty_id,
                category,
                defense_type,
                dispel_type: 0,
                mechanic,
                prevention_type: 0,
                start_recovery_category: 0,
                charge_category,
                spell_id,
            }
        };
        let category_store = crate::spell_db2::SpellCategoriesStore::from_entries([
            categories(10, 0, 7, 8, 1, 2),
            categories(19, 2, 50, 60, 5, 6),
            categories(20, 2, 30, 40, 3, 4),
        ]);

        let mut base_misc = test_spell_misc_entry_like_cpp(10, spell_id, 0, 0);
        base_misc.school_mask = 1;
        let mut lower_duplicate_misc = test_spell_misc_entry_like_cpp(9, spell_id, 0, 0);
        lower_duplicate_misc.school_mask = 2;
        let misc_store =
            crate::spell_db2::SpellMiscStore::from_entries([base_misc, lower_duplicate_misc]);

        let effect_store = crate::spell_db2::SpellEffectDb2Store::from_entries([
            test_spell_effect_db2_entry_like_cpp(10, spell_id, 0, 0, 2, 7),
            // The row itself exists even though Effect=NONE, so it suppresses
            // the base slot's mechanic during per-effect fallback.
            test_spell_effect_db2_entry_like_cpp(20, spell_id, 2, 0, 0, 0),
            test_spell_effect_db2_entry_like_cpp(11, spell_id, 0, 1, 2, 8),
            test_spell_effect_db2_entry_like_cpp(21, spell_id, 1, 1, 2, 9),
            test_spell_effect_db2_entry_like_cpp(18, spell_id, 2, 2, 2, 5),
            test_spell_effect_db2_entry_like_cpp(22, spell_id, 2, 2, 2, 11),
            test_spell_effect_db2_entry_like_cpp(
                30,
                spell_id,
                2,
                MAX_SPELL_EFFECTS_LIKE_CPP,
                2,
                99,
            ),
        ]);
        let store = SpellStore::from_spell_db2_stores_like_cpp(
            &category_store,
            &misc_store,
            &effect_store,
            &crate::spell_db2::SpellShapeshiftStore::from_entries([]),
        );
        let difficulties = crate::DifficultyStore::from_entries([
            crate::DifficultyEntry {
                id: 2,
                instance_type: 0,
                flags: 0,
                fallback_difficulty_id: 1,
                toggle_difficulty_id: 0,
            },
            crate::DifficultyEntry {
                id: 1,
                instance_type: 0,
                flags: 0,
                fallback_difficulty_id: 0,
                toggle_difficulty_id: 0,
            },
        ]);

        assert_eq!(
            store.hit_metadata_for_difficulty_like_cpp(spell_id as i32, 2, Some(&difficulties)),
            Some(SpellHitMetadataLikeCpp {
                category_id: 30,
                charge_category_id: 40,
                defense_type: 3,
                spell_mechanic: 4,
                school_mask: 1,
                effect_mechanics: BTreeMap::from([(0, 0), (1, 9), (2, 11)]),
            })
        );
        assert_eq!(
            store.hit_metadata_for_difficulty_like_cpp(spell_id as i32, 3, None),
            Some(SpellHitMetadataLikeCpp {
                category_id: 7,
                charge_category_id: 8,
                defense_type: 1,
                spell_mechanic: 2,
                school_mask: 1,
                effect_mechanics: BTreeMap::from([(0, 7), (1, 8)]),
            })
        );
        assert!(
            store
                .hit_metadata_for_difficulty_like_cpp(99_999, 2, Some(&difficulties))
                .is_none()
        );
    }

    #[test]
    fn synthetic_hit_metadata_insertion_supports_focused_consumers() {
        let mut store = SpellStore::new();
        let metadata = SpellHitMetadataLikeCpp {
            category_id: 13,
            charge_category_id: 17,
            defense_type: 2,
            spell_mechanic: 7,
            school_mask: 4,
            effect_mechanics: BTreeMap::from([(0, 0), (2, 12)]),
        };
        store.insert_spell_hit_metadata_for_difficulty_like_cpp(90_002, 2, metadata.clone());

        assert_eq!(
            store.hit_metadata_for_difficulty_like_cpp(90_002, 2, None),
            Some(metadata)
        );
    }

    #[test]
    fn real_spell_15691_hit_metadata_matches_db2_when_data_exists() {
        let data_dir = std::env::var("RUSTYCORE_REAL_DATA_DIR")
            .unwrap_or_else(|_| "/home/server/woltk-server-core/Data".to_string());
        let locale = std::env::var("RUSTYCORE_REAL_LOCALE").unwrap_or_else(|_| "enUS".to_string());
        let dbc_dir = std::path::Path::new(&data_dir).join("dbc").join(&locale);
        if ["SpellCategories.db2", "SpellMisc.db2", "SpellEffect.db2"]
            .into_iter()
            .any(|file| !dbc_dir.join(file).is_file())
        {
            eprintln!(
                "Skipping real spell hit-metadata fixture: DB2 files not found at {}",
                dbc_dir.display()
            );
            return;
        }

        let category_store = crate::spell_db2::SpellCategoriesStore::load(&data_dir, &locale)
            .expect("load real SpellCategories.db2");
        let misc_store = crate::spell_db2::SpellMiscStore::load(&data_dir, &locale)
            .expect("load real SpellMisc.db2");
        let effect_store = crate::spell_db2::SpellEffectDb2Store::load(&data_dir, &locale)
            .expect("load real SpellEffect.db2");
        let store = SpellStore::from_spell_db2_stores_like_cpp(
            &category_store,
            &misc_store,
            &effect_store,
            &crate::spell_db2::SpellShapeshiftStore::from_entries([]),
        );

        assert_eq!(
            store.hit_metadata_for_difficulty_like_cpp(15_691, 0, None),
            Some(SpellHitMetadataLikeCpp {
                category_id: 0,
                charge_category_id: 0,
                defense_type: 2,
                spell_mechanic: 0,
                school_mask: 1,
                effect_mechanics: BTreeMap::from([(0, 0)]),
            })
        );
    }

    use crate::{Condition, ConditionEntriesByTypeStore};
    use wow_constants::{ConditionSourceType, ConditionType};

    #[test]
    fn test_spell_store_creation() {
        let store = SpellStore::new();
        assert!(store.is_empty(), "new store should be empty");
    }

    #[test]
    fn exact_spell_info_key_does_not_fabricate_hydrated_payload() {
        let mut store = SpellStore::new();
        store.spell_info_keys_like_cpp =
            crate::spell_info_keys::SpellInfoKeyStoreLikeCpp::from_candidate_keys_like_cpp(
                [(200, 2), (100, 0), (200, 1)],
                &HashSet::from([100, 200]),
            );

        assert!(store.contains_spell_info_exact_like_cpp(100, 0));
        assert!(store.get(100).is_none());
        assert_eq!(
            store.spell_info_keys_in_order_like_cpp(),
            [(100, 0), (200, 1), (200, 2)]
        );
    }

    #[test]
    fn difficulty_none_existence_composes_exact_regular_and_serverside_keys_like_cpp() {
        let mut regular = SpellStore::new();
        regular.spell_info_keys_like_cpp =
            crate::spell_info_keys::SpellInfoKeyStoreLikeCpp::from_candidate_keys_like_cpp(
                [(100, 0), (101, 2), (300, 2)],
                &HashSet::from([100, 101, 300]),
            );
        let serverside = ServersideSpellStoreLikeCpp::from_rows_like_cpp(
            [
                serverside_spell_row(200, 0),
                serverside_spell_row(201, 2),
                serverside_spell_row(400, 3),
            ],
            &ServersideSpellEffectStoreLikeCpp::default(),
            |_| false,
        );
        assert!(serverside.errors.is_empty());
        let no_fallback = crate::DifficultyStore::from_entries([]);

        assert!(
            regular.contains_spell_info_difficulty_none_like_cpp(
                &serverside.store,
                &no_fallback,
                100
            ),
            "an exact regular difficulty-zero key is visible even without hydrated payload"
        );
        assert!(
            !regular.contains_spell_info_difficulty_none_like_cpp(
                &serverside.store,
                &no_fallback,
                101
            ),
            "a regular key that exists only at another difficulty is not a trainer spell"
        );
        assert!(
            regular.contains_spell_info_difficulty_none_like_cpp(
                &serverside.store,
                &no_fallback,
                200
            ),
            "an exact server-side difficulty-zero key shares C++ GetSpellInfo authority"
        );
        assert!(
            !regular.contains_spell_info_difficulty_none_like_cpp(
                &serverside.store,
                &no_fallback,
                201
            ),
            "a server-side key that exists only at another difficulty is not a trainer spell"
        );

        let trainer = crate::trainer::TrainerStoreLikeCpp::from_rows_like_cpp(
            [crate::trainer::TrainerRowLikeCpp {
                id: 10,
                trainer_type: crate::trainer::TRAINER_TYPE_TRADESKILL_LIKE_CPP,
                greeting: String::new(),
            }],
            [100, 101, 200, 201].map(|spell_id| crate::trainer::TrainerSpellRowLikeCpp {
                trainer_id: 10,
                spell: crate::trainer::TrainerSpellLikeCpp {
                    spell_id,
                    money_cost: 0,
                    req_skill_line: 0,
                    req_skill_rank: 0,
                    req_ability: [0; 3],
                    req_level: 0,
                },
            }),
            [],
            [],
            |spell_id| {
                regular.contains_spell_info_difficulty_none_like_cpp(
                    &serverside.store,
                    &no_fallback,
                    spell_id,
                )
            },
            |_| true,
            |_| true,
            |_, _| true,
        );
        let loaded = trainer.store.get_trainer_like_cpp(10).unwrap();
        assert!(loaded.get_spell_like_cpp(100).is_some());
        assert!(loaded.get_spell_like_cpp(200).is_some());
        assert!(loaded.get_spell_like_cpp(101).is_none());
        assert!(loaded.get_spell_like_cpp(201).is_none());
        assert_eq!(
            trainer.report.skipped_spells_missing_spell,
            vec![(10, 101), (10, 201)]
        );

        let difficulty_fallbacks = crate::DifficultyStore::from_entries([
            crate::DifficultyEntry {
                id: 0,
                instance_type: 0,
                flags: 0,
                fallback_difficulty_id: 2,
                toggle_difficulty_id: 0,
            },
            crate::DifficultyEntry {
                id: 2,
                instance_type: 0,
                flags: 0,
                fallback_difficulty_id: 3,
                toggle_difficulty_id: 0,
            },
            crate::DifficultyEntry {
                id: 3,
                instance_type: 0,
                flags: 0,
                fallback_difficulty_id: 0,
                toggle_difficulty_id: 0,
            },
        ]);
        assert!(
            regular.contains_spell_info_difficulty_none_like_cpp(
                &serverside.store,
                &difficulty_fallbacks,
                300
            ),
            "a custom Difficulty(0) fallback reaches a regular spell like C++"
        );
        assert!(
            regular.contains_spell_info_difficulty_none_like_cpp(
                &serverside.store,
                &difficulty_fallbacks,
                400
            ),
            "the fallback chain reaches a server-side spell like C++"
        );
        assert!(
            !regular.contains_spell_info_difficulty_none_like_cpp(
                &serverside.store,
                &difficulty_fallbacks,
                999
            ),
            "invalid custom fallback cycles terminate instead of hanging startup"
        );
    }

    #[test]
    fn spell_store_db2_loader_keeps_mount_aura_spells_like_cpp() {
        let spell_id = 32_243;
        let mut misc = test_spell_misc_entry_like_cpp(1, spell_id, 0, 0);
        misc.attributes[0] = attributes::SPELL_ATTR0_NO_AURA_CANCEL as i32;
        let misc_store = crate::spell_db2::SpellMiscStore::from_entries([misc]);
        let effect_store = crate::spell_db2::SpellEffectDb2Store::from_entries([
            crate::spell_db2::SpellEffectDb2Entry {
                id: 1,
                difficulty_id: 0,
                effect_index: 0,
                effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_amplitude: 0.0,
                effect_attributes: 0,
                effect_aura: aura_types::SPELL_AURA_MOUNTED as i16,
                effect_aura_period: 0,
                effect_base_points: 77,
                effect_bonus_coefficient: 0.0,
                effect_chain_amplitude: 0.0,
                effect_chain_targets: 0,
                effect_die_sides: 0,
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
                effect_misc_value: [23966, 0],
                effect_radius_index: [0, 0],
                effect_spell_class_mask: [0, 0, 0, 0],
                implicit_target: [0, 0],
                spell_id,
            },
        ]);

        let shapeshift_store = crate::spell_db2::SpellShapeshiftStore::from_entries([]);
        let store = SpellStore::from_spell_db2_stores_like_cpp(
            &crate::spell_db2::SpellCategoriesStore::from_entries([]),
            &misc_store,
            &effect_store,
            &shapeshift_store,
        );
        let spell = store.get(spell_id as i32).expect("mount spell loaded");

        assert_eq!(
            spell.effect_type,
            spell_effect_types::SPELL_EFFECT_APPLY_AURA
        );
        assert_eq!(spell.aura_type, Some(aura_types::SPELL_AURA_MOUNTED));
        assert!(
            spell
                .effects
                .iter()
                .any(SpellEffectInfo::is_mounted_aura_like_cpp)
        );
        assert!(
            store.has_attribute0_like_cpp(spell_id as i32, attributes::SPELL_ATTR0_NO_AURA_CANCEL)
        );
    }

    #[test]
    fn spell_store_db2_loader_keeps_channeled_spell_attr1_like_cpp() {
        let spell_id = 51_588;
        let mut misc = test_spell_misc_entry_like_cpp(1, spell_id, 0, 0);
        misc.attributes[1] = attributes::SPELL_ATTR1_IS_CHANNELLED as i32;
        let misc_store = crate::spell_db2::SpellMiscStore::from_entries([misc]);
        let effect_store = crate::spell_db2::SpellEffectDb2Store::from_entries([]);

        let shapeshift_store = crate::spell_db2::SpellShapeshiftStore::from_entries([]);
        let store = SpellStore::from_spell_db2_stores_like_cpp(
            &crate::spell_db2::SpellCategoriesStore::from_entries([]),
            &misc_store,
            &effect_store,
            &shapeshift_store,
        );

        assert!(
            store.has_attribute1_like_cpp(spell_id as i32, attributes::SPELL_ATTR1_IS_CHANNELLED)
        );
        assert!(store.is_channeled_like_cpp(spell_id as i32));
        assert!(!store.is_channeled_like_cpp(99_999));
    }

    #[test]
    fn spell_store_resolves_interrupt_masks_by_difficulty_and_fallback_like_cpp() {
        let spell_id = 70_101;
        let exact_without_difficulty_entry_spell_id = 70_102;
        let interrupts = crate::spell_db2::SpellInterruptsStore::from_entries([
            crate::spell_db2::SpellInterruptsEntry {
                id: 1,
                difficulty_id: 0,
                interrupt_flags: 0,
                aura_interrupt_flags: [0x0004_0000, 0],
                channel_interrupt_flags: [0, 0],
                spell_id,
            },
            crate::spell_db2::SpellInterruptsEntry {
                id: 2,
                difficulty_id: 2,
                interrupt_flags: 0,
                aura_interrupt_flags: [0, 0],
                channel_interrupt_flags: [0x0004_0000, 0],
                spell_id,
            },
            crate::spell_db2::SpellInterruptsEntry {
                id: 3,
                difficulty_id: 9,
                interrupt_flags: 0,
                aura_interrupt_flags: [0, 0x40],
                channel_interrupt_flags: [0, 0x80],
                spell_id: exact_without_difficulty_entry_spell_id,
            },
        ]);
        let mut store = SpellStore::new();
        let difficulties = crate::difficulty::DifficultyStore::from_entries([
            crate::difficulty::DifficultyEntry {
                id: 1,
                instance_type: 0,
                flags: 0,
                fallback_difficulty_id: 0,
                toggle_difficulty_id: 0,
            },
            crate::difficulty::DifficultyEntry {
                id: 2,
                instance_type: 0,
                flags: 0,
                fallback_difficulty_id: 1,
                toggle_difficulty_id: 0,
            },
            crate::difficulty::DifficultyEntry {
                id: 3,
                instance_type: 0,
                flags: 0,
                fallback_difficulty_id: 1,
                toggle_difficulty_id: 0,
            },
        ]);

        store.apply_db2_interrupts_like_cpp(&interrupts);

        assert_eq!(
            store.interrupt_flags_for_difficulty_like_cpp(spell_id as i32, 2, Some(&difficulties),),
            Some(([0, 0], [0x0004_0000, 0])),
            "the exact row overrides its base row without merging words"
        );
        assert_eq!(
            store.interrupt_flags_for_difficulty_like_cpp(spell_id as i32, 3, Some(&difficulties),),
            Some(([0x0004_0000, 0], [0, 0])),
            "difficulty 3 walks 3 -> 1 -> 0"
        );
        assert_eq!(
            store.interrupt_flags_for_difficulty_like_cpp(
                exact_without_difficulty_entry_spell_id as i32,
                9,
                Some(&difficulties),
            ),
            Some(([0, 0x40], [0, 0x80])),
            "an exact SpellInterrupts row wins before Difficulty lookup"
        );
        assert_eq!(
            store.interrupt_flags_for_difficulty_like_cpp(99_999, 3, Some(&difficulties)),
            None,
            "a fully missing fallback chain stays unknown"
        );
        assert!(store.has_aura_interrupt_flag_like_cpp(spell_id as i32, 0x0004_0000, 0));
        assert!(!store.has_channel_interrupt_flag_like_cpp(spell_id as i32, 0x0004_0000, 0));
    }

    #[test]
    fn spell_store_effective_interrupt_masks_follow_cpp_load_order() {
        let regular_spell_id = 24_314;
        let serverside_spell_id = 70_001;
        let interrupts = crate::spell_db2::SpellInterruptsStore::from_entries([
            crate::spell_db2::SpellInterruptsEntry {
                id: 1,
                difficulty_id: 2,
                interrupt_flags: 0,
                aura_interrupt_flags: [0x100, 0x200],
                channel_interrupt_flags: [0x300, 0x400],
                spell_id: regular_spell_id,
            },
        ]);
        let mut store = SpellStore::new();
        store.apply_db2_interrupts_like_cpp(&interrupts);

        assert_eq!(
            store.interrupt_flags_for_difficulty_like_cpp(regular_spell_id as i32, 2, None),
            Some(([0x100, 0x200], [0x300, 0x400]))
        );

        assert!(store.store_signed_interrupt_row_by_id_like_cpp(
            1,
            regular_spell_id,
            2,
            [0x10, -1],
            [i32::MIN, 0x40],
        ));
        store.rebuild_interrupt_flags_from_rows_like_cpp();
        assert_eq!(
            store.interrupt_flags_for_difficulty_like_cpp(regular_spell_id as i32, 2, None),
            Some(([0x10, u32::MAX], [0x8000_0000, 0x40])),
            "the later row for the same DB2 record ID replaces its masks and preserves signed bit patterns"
        );

        let serverside = ServersideSpellStoreLikeCpp::from_rows_like_cpp(
            [serverside_spell_row(serverside_spell_id, 2)],
            &ServersideSpellEffectStoreLikeCpp::default(),
            |_| false,
        );
        assert!(serverside.errors.is_empty());
        store.apply_serverside_spell_interrupts_like_cpp(&serverside.store);

        assert_eq!(
            store.interrupt_flags_for_difficulty_like_cpp(regular_spell_id as i32, 2, None),
            Some(([0x3c, u32::MAX], [0x8000_0000, 0x40])),
            "the interrupt correction runs after the file/hotfix composition"
        );
        assert_eq!(
            store.interrupt_flags_for_difficulty_like_cpp(serverside_spell_id as i32, 2, None),
            Some(([43, 44], [45, 46])),
            "server-side masks enter the same effective table before corrections"
        );
    }

    #[test]
    fn spell_store_hotfix_overlay_rekeys_by_db2_record_id_like_cpp() {
        let original_spell_id = 70_201;
        let rekeyed_spell_id = 70_202;
        let interrupts = crate::spell_db2::SpellInterruptsStore::from_entries([
            crate::spell_db2::SpellInterruptsEntry {
                id: 10,
                difficulty_id: 2,
                interrupt_flags: 0,
                aura_interrupt_flags: [0x10, 0],
                channel_interrupt_flags: [0x20, 0],
                spell_id: original_spell_id,
            },
            crate::spell_db2::SpellInterruptsEntry {
                id: 20,
                difficulty_id: 2,
                interrupt_flags: 0,
                aura_interrupt_flags: [0x30, 0],
                channel_interrupt_flags: [0x40, 0],
                spell_id: original_spell_id,
            },
        ]);
        let mut store = SpellStore::new();
        store.apply_db2_interrupts_like_cpp(&interrupts);

        assert_eq!(
            store.interrupt_flags_for_difficulty_like_cpp(original_spell_id as i32, 2, None),
            Some(([0x30, 0], [0x40, 0])),
            "the highest DB2 record ID wins when two rows have the same relational key"
        );

        assert!(store.store_signed_interrupt_row_by_id_like_cpp(
            20,
            rekeyed_spell_id,
            3,
            [0x50, 0],
            [0x60, 0],
        ));
        store.rebuild_interrupt_flags_from_rows_like_cpp();
        assert_eq!(
            store.interrupt_flags_for_difficulty_like_cpp(original_spell_id as i32, 2, None),
            Some(([0x10, 0], [0x20, 0])),
            "replacing record ID 20 uncovers record ID 10 at its former key"
        );
        assert_eq!(
            store.interrupt_flags_for_difficulty_like_cpp(rekeyed_spell_id as i32, 3, None),
            Some(([0x50, 0], [0x60, 0])),
            "the replacement row is indexed by its new spell/difficulty relationship"
        );
    }

    #[test]
    fn spell_store_interrupt_corrections_cover_every_stored_difficulty() {
        let mut store = SpellStore::new();
        for difficulty_id in [0, 2] {
            store.insert_spell_interrupt_flags_for_difficulty_like_cpp(
                29_726,
                difficulty_id,
                [0, 0],
                [0xffff_ffff, 0x20],
            );
            store.insert_spell_interrupt_flags_for_difficulty_like_cpp(
                24_314,
                difficulty_id,
                [0x10, 0x40],
                [0x80, 0x100],
            );
            store.insert_spell_interrupt_flags_for_difficulty_like_cpp(
                99_252,
                difficulty_id,
                [0x200, 0x400],
                [0x800, 0x1000],
            );
        }
        store.insert_spell_interrupt_flags_like_cpp(
            63_414,
            [0x10, 0x20],
            [0xffff_ffff, 0xffff_ffff],
        );
        store
            .spells
            .insert(61_719, SpellStore::empty_spell_info_like_cpp(61_719));

        store.apply_interrupt_flag_corrections_like_cpp();

        for difficulty_id in [0, 2] {
            assert_eq!(
                store.interrupt_flags_for_difficulty_like_cpp(29_726, difficulty_id, None),
                Some(([0, 0], [0xffff_fffb, 0x20]))
            );
            assert_eq!(
                store.interrupt_flags_for_difficulty_like_cpp(24_314, difficulty_id, None),
                Some(([0x3c, 0x40], [0x80, 0x100]))
            );
            assert_eq!(
                store.interrupt_flags_for_difficulty_like_cpp(99_252, difficulty_id, None),
                Some(([0x8_0200, 0x400], [0x800, 0x1000]))
            );
        }
        assert_eq!(
            store.interrupt_flags_for_difficulty_like_cpp(63_414, 0, None),
            Some(([0x10, 0x20], [0, 0]))
        );
        assert_eq!(
            store.interrupt_flags_for_difficulty_like_cpp(61_719, 0, None),
            Some(([0x3, 0], [0, 0])),
            "a corrected regular spell without a SpellInterrupts row receives a base mask"
        );
    }

    #[test]
    fn db2_cast_times_set_max_base_minimum_like_cpp() {
        use crate::spell_db2::{
            SpellCastTimesEntry, SpellCastTimesStore, SpellMiscEntry, SpellMiscStore,
        };
        let mut store = SpellStore::new();
        store
            .spells
            .insert(100, SpellStore::empty_spell_info_like_cpp(100));
        store
            .spells
            .insert(200, SpellStore::empty_spell_info_like_cpp(200));

        let misc = SpellMiscStore::from_entries([
            SpellMiscEntry {
                id: 1,
                spell_id: 100,
                casting_time_index: 5,
                difficulty_id: 0,
                ..Default::default()
            },
            // casting_time_index 0 → no cast-time row, stays instant.
            SpellMiscEntry {
                id: 2,
                spell_id: 200,
                casting_time_index: 0,
                difficulty_id: 0,
                ..Default::default()
            },
        ]);
        let cast_times = SpellCastTimesStore::from_entries([SpellCastTimesEntry {
            id: 5,
            base: 1500,
            minimum: 1000,
            ..Default::default()
        }]);

        store.apply_db2_cast_times_like_cpp(&misc, &cast_times);

        // C++ CalcCastTime = max(Base, Minimum) = max(1500, 1000) = 1500.
        assert_eq!(store.spells.get(&100).unwrap().cast_time_ms, 1500);
        assert!(store.spells.get(&100).unwrap().has_cast_time());
        // No CastingTimeIndex → untouched (instant).
        assert_eq!(store.spells.get(&200).unwrap().cast_time_ms, 0);
        assert!(!store.spells.get(&200).unwrap().has_cast_time());
    }

    #[test]
    fn db2_cooldowns_set_recovery_max_like_cpp() {
        use crate::spell_db2::{SpellCooldownsEntry, SpellCooldownsStore};
        let mut store = SpellStore::new();
        store
            .spells
            .insert(300, SpellStore::empty_spell_info_like_cpp(300));

        let cooldowns = SpellCooldownsStore::from_entries([SpellCooldownsEntry {
            id: 1,
            difficulty_id: 0,
            recovery_time: 3000,
            category_recovery_time: 5000,
            start_recovery_time: 1500,
            spell_id: 300,
        }]);
        store.apply_db2_cooldowns_like_cpp(&cooldowns);

        // C++ GetRecoveryTime = max(RecoveryTime 3000, CategoryRecoveryTime 5000) = 5000.
        assert_eq!(store.spells.get(&300).unwrap().recovery_time_ms, 5000);
        // GCD (cooldown_ms) is a separate mechanic — untouched by this slice.
        assert_eq!(store.spells.get(&300).unwrap().cooldown_ms, 0);
    }

    #[test]
    fn db2_spell_power_sets_power_costs_like_cpp() {
        use crate::spell_db2::{
            SpellPowerDifficultyEntry, SpellPowerDifficultyStore, SpellPowerEntry, SpellPowerStore,
        };

        let mut store = SpellStore::new();
        store
            .spells
            .insert(400, SpellStore::empty_spell_info_like_cpp(400));

        let spell_power = SpellPowerStore::from_entries([
            SpellPowerEntry {
                id: 10,
                order_index: 1,
                mana_cost: 40,
                mana_cost_per_level: 4,
                mana_per_second: 5,
                power_display_id: 0,
                alt_power_bar_id: 0,
                power_cost_pct: 10.0,
                power_cost_max_pct: 0.0,
                power_pct_per_second: 6.5,
                power_type: PowerType::Mana as i8,
                required_aura_spell_id: 0,
                optional_cost: 0,
                spell_id: 400,
            },
            SpellPowerEntry {
                id: 11,
                order_index: 2,
                mana_cost: 999,
                mana_cost_per_level: 0,
                mana_per_second: 0,
                power_display_id: 0,
                alt_power_bar_id: 0,
                power_cost_pct: 0.0,
                power_cost_max_pct: 0.0,
                power_pct_per_second: 0.0,
                power_type: PowerType::Mana as i8,
                required_aura_spell_id: 0,
                optional_cost: 0,
                spell_id: 400,
            },
        ]);
        let spell_power_difficulty =
            SpellPowerDifficultyStore::from_entries([SpellPowerDifficultyEntry {
                id: 11,
                difficulty_id: 1,
                order_index: 2,
            }]);

        store.apply_db2_power_costs_like_cpp(&spell_power, &spell_power_difficulty);

        let costs = &store.spells.get(&400).unwrap().power_costs;
        assert_eq!(costs.len(), 1, "non-default difficulty rows are skipped");
        assert_eq!(costs[0].order_index, 1);
        assert_eq!(costs[0].mana_cost, 40);
        assert_eq!(costs[0].mana_cost_per_level, 4);
        assert_eq!(costs[0].mana_per_second, 5);
        assert_eq!(costs[0].power_cost_pct, 10.0);
        assert_eq!(costs[0].power_pct_per_second, 6.5);
        assert_eq!(costs[0].power_type, PowerType::Mana as i8);
    }

    #[test]
    fn spell_info_calc_power_costs_flat_plus_mana_pct_like_cpp() {
        let mut spell = SpellStore::empty_spell_info_like_cpp(500);
        spell.power_costs.push(SpellPowerCostInfoLikeCpp {
            order_index: 0,
            power_type: PowerType::Mana as i8,
            mana_cost: 50,
            mana_cost_per_level: 0,
            mana_per_second: 0,
            power_cost_pct: 12.5,
            power_cost_max_pct: 0.0,
            power_pct_per_second: 0.0,
            required_aura_spell_id: 0,
            optional_cost: 0,
        });

        let costs = spell.calc_power_costs_like_cpp(1000);

        assert_eq!(
            costs,
            vec![SpellPowerCostLikeCpp {
                power_type: PowerType::Mana as i8,
                amount: 175,
            }]
        );
    }

    #[test]
    fn spell_info_calc_power_costs_ignores_mana_max_pct_like_cpp() {
        let mut spell = SpellStore::empty_spell_info_like_cpp(501);
        spell.power_costs.push(SpellPowerCostInfoLikeCpp {
            order_index: 0,
            power_type: PowerType::Mana as i8,
            mana_cost: 0,
            mana_cost_per_level: 0,
            mana_per_second: 0,
            power_cost_pct: 0.0,
            power_cost_max_pct: 18.0,
            power_pct_per_second: 0.0,
            required_aura_spell_id: 0,
            optional_cost: 0,
        });

        let costs = spell.calc_power_costs_like_cpp(1000);

        assert!(costs.is_empty());
    }

    #[test]
    fn spell_store_db2_loader_composes_shapeshift_masks_like_cpp() {
        let spell_id = 70_001;
        let misc_store =
            crate::spell_db2::SpellMiscStore::from_entries([test_spell_misc_entry_like_cpp(
                1, spell_id, 0, 0,
            )]);
        let effect_store = crate::spell_db2::SpellEffectDb2Store::from_entries([]);
        let shapeshift_store = crate::spell_db2::SpellShapeshiftStore::from_entries([
            crate::spell_db2::SpellShapeshiftEntry {
                id: 1,
                spell_id: spell_id as i32,
                stance_bar_order: 0,
                shapeshift_exclude: [1 << 2, 0],
                shapeshift_mask: [1 << 4, 0],
            },
        ]);
        let form = shapeshift_form(shapeshift_form_flags::STANCE);
        let store = SpellStore::from_spell_db2_stores_like_cpp(
            &crate::spell_db2::SpellCategoriesStore::from_entries([]),
            &misc_store,
            &effect_store,
            &shapeshift_store,
        );

        assert_eq!(
            store.check_shapeshift_like_cpp(spell_id as i32, 3, |_| Some(&form)),
            Some(SpellCastResult::NotShapeshift)
        );
        assert_eq!(
            store.check_shapeshift_like_cpp(spell_id as i32, 5, |_| Some(&form)),
            Some(SpellCastResult::Success)
        );
        assert_eq!(
            store.check_shapeshift_like_cpp(spell_id as i32, 0, |_| None),
            Some(SpellCastResult::OnlyShapeshift)
        );
    }

    #[test]
    fn spell_store_check_shapeshift_uses_spell_misc_attr2_like_cpp() {
        let spell_id = 70_002;
        let mut misc = test_spell_misc_entry_like_cpp(1, spell_id, 0, 0);
        misc.attributes[2] =
            attributes::SPELL_ATTR2_ALLOW_WHILE_NOT_SHAPESHIFTED_CASTER_FORM as i32;
        let misc_store = crate::spell_db2::SpellMiscStore::from_entries([misc]);
        let effect_store = crate::spell_db2::SpellEffectDb2Store::from_entries([]);
        let shapeshift_store = crate::spell_db2::SpellShapeshiftStore::from_entries([
            crate::spell_db2::SpellShapeshiftEntry {
                id: 2,
                spell_id: spell_id as i32,
                stance_bar_order: 0,
                shapeshift_exclude: [0, 0],
                shapeshift_mask: [1 << 4, 0],
            },
        ]);
        let store = SpellStore::from_spell_db2_stores_like_cpp(
            &crate::spell_db2::SpellCategoriesStore::from_entries([]),
            &misc_store,
            &effect_store,
            &shapeshift_store,
        );

        assert_eq!(
            store.check_shapeshift_like_cpp(spell_id as i32, 0, |_| None),
            Some(SpellCastResult::Success)
        );
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
            power_costs: Vec::new(),
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
            power_costs: Vec::new(),
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
            power_costs: Vec::new(),
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
            power_costs: Vec::new(),
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
    fn spell_effect_calc_value_no_caster_uses_cpp_double_accumulator() {
        let overflowing_int_add = SpellEffectInfo {
            effect_base_points: i32::MAX,
            effect_die_sides: 1,
            ..Default::default()
        };
        assert_eq!(
            overflowing_int_add.calc_value_no_caster_with_die_roll_like_cpp(|_, _| unreachable!()),
            i32::MAX
        );

        let underflowing_int_add = SpellEffectInfo {
            effect_base_points: i32::MIN,
            effect_die_sides: -1,
            ..Default::default()
        };
        assert_eq!(
            underflowing_int_add.calc_value_no_caster_with_die_roll_like_cpp(|min, max| {
                assert_eq!((min, max), (-1, 1));
                -1
            }),
            i32::MIN
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
        assert_eq!(aura_types::SPELL_AURA_MOD_INCREASE_SPEED, 31);
        assert_eq!(aura_types::SPELL_AURA_MOD_INCREASE_MOUNTED_SPEED, 32);
        assert_eq!(aura_types::SPELL_AURA_MOD_DECREASE_SPEED, 33);
        assert_eq!(aura_types::SPELL_AURA_MOD_SHAPESHIFT, 36);
        assert_eq!(aura_types::SPELL_AURA_TRANSFORM, 56);
        assert_eq!(aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED, 58);
        assert_eq!(aura_types::SPELL_AURA_MOD_SCALE, 61);
        assert_eq!(aura_types::SPELL_AURA_MOUNTED, 78);
        assert_eq!(aura_types::SPELL_AURA_MOD_DETECT_RANGE, 91);
        assert_eq!(aura_types::SPELL_AURA_MOD_SPEED_ALWAYS, 129);
        assert_eq!(aura_types::SPELL_AURA_MOD_MOUNTED_SPEED_ALWAYS, 130);
        assert_eq!(aura_types::SPELL_AURA_MOD_DETECTED_RANGE, 152);
        assert_eq!(aura_types::SPELL_AURA_MOD_SPEED_NOT_STACK, 171);
        assert_eq!(aura_types::SPELL_AURA_MOD_MOUNTED_SPEED_NOT_STACK, 172);
        assert_eq!(aura_types::SPELL_AURA_FLY, 201);
        assert_eq!(
            aura_types::SPELL_AURA_MOD_INCREASE_MOUNTED_FLIGHT_SPEED,
            207
        );
        assert_eq!(aura_types::SPELL_AURA_USE_NORMAL_MOVEMENT_SPEED, 191);
        assert_eq!(
            aura_types::SPELL_AURA_MOD_INCREASE_VEHICLE_FLIGHT_SPEED,
            206
        );
        assert_eq!(aura_types::SPELL_AURA_MOD_INCREASE_FLIGHT_SPEED, 208);
        assert_eq!(aura_types::SPELL_AURA_MOD_MOUNTED_FLIGHT_SPEED_ALWAYS, 209);
        assert_eq!(aura_types::SPELL_AURA_MOD_FLIGHT_SPEED_NOT_STACK, 211);
        assert_eq!(aura_types::SPELL_AURA_MOD_MINIMUM_SPEED, 305);
        assert_eq!(aura_types::SPELL_AURA_MOD_SPEED_NO_CONTROL, 373);
        assert_eq!(aura_types::SPELL_AURA_MOD_BATTLE_PET_XP_PCT, 420);
        assert_eq!(aura_types::SPELL_AURA_MOD_MINIMUM_SPEED_RATE, 437);
        assert_eq!(aura_types::SPELL_AURA_MOD_RESTED_XP_CONSUMPTION, 499);

        // C++ `SharedDefines.h`: selected SpellAttr0 anchors.
        assert_eq!(attributes::SPELL_ATTR0_ONLY_INDOORS, 0x0000_4000);
        assert_eq!(attributes::SPELL_ATTR0_ONLY_OUTDOORS, 0x0000_8000);
        assert_eq!(attributes::SPELL_ATTR0_ALLOW_WHILE_MOUNTED, 0x0100_0000);
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
                power_costs: Vec::new(),
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
                power_costs: Vec::new(),
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
                power_costs: Vec::new(),
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
                power_costs: Vec::new(),
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
        assert!(outcome.errors.is_empty());
        assert_eq!(
            outcome.store.get_spell_learn_skill_like_cpp(100),
            Some(&SpellLearnSkillNodeLikeCpp {
                skill: 755,
                step: 4,
                value: 0,
                maxvalue: 0,
            })
        );
        assert_eq!(
            outcome.store.spell_learn_skill_lookup_like_cpp(100),
            SpellLearnSkillLookupLikeCpp::Present(&SpellLearnSkillNodeLikeCpp {
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
        assert!(outcome.errors.is_empty());
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
        assert!(outcome.errors.is_empty());
        assert!(outcome.store.get_spell_learn_skill_like_cpp(300).is_none());
        assert_eq!(
            outcome.store.spell_learn_skill_lookup_like_cpp(300),
            SpellLearnSkillLookupLikeCpp::MissingCoverage
        );
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
    fn spell_learn_skill_lookup_distinguishes_covered_absence_and_indeterminate() {
        let mut outcome =
            SpellLearnSkillStoreLikeCpp::from_spell_infos_like_cpp([learn_skill_source(
                500,
                true,
                Vec::new(),
            )]);
        outcome.store.mark_spell_learn_skill_indeterminate_like_cpp(
            501,
            SpellLearnSkillIndeterminateReasonLikeCpp::RngDependentCalcValue {
                record_id: 9,
                domain: AcquisitionValueDomainLikeCpp {
                    minimum: 2,
                    maximum: 4,
                },
            },
        );

        assert_eq!(
            outcome.store.spell_learn_skill_lookup_like_cpp(500),
            SpellLearnSkillLookupLikeCpp::CoveredWithoutNode
        );
        assert!(matches!(
            outcome.store.spell_learn_skill_lookup_like_cpp(501),
            SpellLearnSkillLookupLikeCpp::Indeterminate(
                SpellLearnSkillIndeterminateReasonLikeCpp::RngDependentCalcValue {
                    record_id: 9,
                    domain: AcquisitionValueDomainLikeCpp {
                        minimum: 2,
                        maximum: 4,
                    },
                }
            )
        ));
        assert_eq!(
            outcome.store.spell_learn_skill_lookup_like_cpp(502),
            SpellLearnSkillLookupLikeCpp::MissingCoverage
        );
    }

    #[test]
    fn spell_learn_skill_store_rejects_duplicate_source_ids_in_every_order() {
        let valid = || {
            learn_skill_source(
                600,
                true,
                vec![SpellLearnSkillEffectLikeCpp {
                    effect: spell_effect_types::SPELL_EFFECT_SKILL,
                    misc_value: 755,
                    calc_value: 4,
                }],
            )
        };
        let invalid = || {
            learn_skill_source(
                600,
                true,
                vec![SpellLearnSkillEffectLikeCpp {
                    effect: spell_effect_types::SPELL_EFFECT_SKILL,
                    misc_value: -1,
                    calc_value: 4,
                }],
            )
        };

        for sources in [[valid(), invalid()], [invalid(), valid()]] {
            let outcome = SpellLearnSkillStoreLikeCpp::from_spell_infos_like_cpp(sources);

            assert_eq!(outcome.dbc_loaded_row_count, 0);
            assert!(
                outcome.store.get_spell_learn_skill_like_cpp(600).is_none(),
                "the legacy getter must not leak a node from either duplicate ordering"
            );
            assert_eq!(
                outcome.store.spell_learn_skill_lookup_like_cpp(600),
                SpellLearnSkillLookupLikeCpp::Indeterminate(
                    &SpellLearnSkillIndeterminateReasonLikeCpp::DuplicateSourceSpell
                )
            );
            assert!(outcome.errors.iter().any(|error| {
                error.spell_id == 600
                    && error.kind == SpellLearnSkillLoadErrorKindLikeCpp::DuplicateSourceSpell
            }));
        }
    }

    #[test]
    fn spell_learn_skill_store_rejects_out_of_range_skill_without_wrapping() {
        for misc_value in [-1, i32::from(u16::MAX) + 1] {
            let outcome =
                SpellLearnSkillStoreLikeCpp::from_spell_infos_like_cpp([learn_skill_source(
                    400,
                    true,
                    vec![
                        SpellLearnSkillEffectLikeCpp {
                            effect: spell_effect_types::SPELL_EFFECT_SKILL,
                            misc_value,
                            calc_value: 1,
                        },
                        SpellLearnSkillEffectLikeCpp {
                            effect: spell_effect_types::SPELL_EFFECT_DUAL_WIELD,
                            misc_value: 0,
                            calc_value: 0,
                        },
                    ],
                )]);

            assert_eq!(outcome.dbc_loaded_row_count, 0);
            assert!(outcome.store.get_spell_learn_skill_like_cpp(400).is_none());
            assert!(matches!(
                outcome.store.spell_learn_skill_lookup_like_cpp(400),
                SpellLearnSkillLookupLikeCpp::Indeterminate(
                    SpellLearnSkillIndeterminateReasonLikeCpp::SkillOutOfRange { value }
                ) if *value == misc_value
            ));
            assert_eq!(
                outcome.errors,
                vec![SpellLearnSkillLoadErrorLikeCpp {
                    spell_id: 400,
                    kind: SpellLearnSkillLoadErrorKindLikeCpp::SkillOutOfRange {
                        value: misc_value,
                    },
                }]
            );
        }
    }

    #[test]
    fn spell_learn_skill_store_rejects_out_of_range_step_without_wrapping() {
        for calc_value in [-1, i32::from(u16::MAX) + 1] {
            let outcome =
                SpellLearnSkillStoreLikeCpp::from_spell_infos_like_cpp([learn_skill_source(
                    401,
                    true,
                    vec![
                        SpellLearnSkillEffectLikeCpp {
                            effect: spell_effect_types::SPELL_EFFECT_SKILL,
                            misc_value: 755,
                            calc_value,
                        },
                        SpellLearnSkillEffectLikeCpp {
                            effect: spell_effect_types::SPELL_EFFECT_DUAL_WIELD,
                            misc_value: 0,
                            calc_value: 0,
                        },
                    ],
                )]);

            assert_eq!(outcome.dbc_loaded_row_count, 0);
            assert!(outcome.store.get_spell_learn_skill_like_cpp(401).is_none());
            assert!(matches!(
                outcome.store.spell_learn_skill_lookup_like_cpp(401),
                SpellLearnSkillLookupLikeCpp::Indeterminate(
                    SpellLearnSkillIndeterminateReasonLikeCpp::StepOutOfRange { value }
                ) if *value == calc_value
            ));
            assert_eq!(
                outcome.errors,
                vec![SpellLearnSkillLoadErrorLikeCpp {
                    spell_id: 401,
                    kind: SpellLearnSkillLoadErrorKindLikeCpp::StepOutOfRange { value: calc_value },
                }]
            );
        }
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
    fn spell_chain_store_derives_predecessors_after_cpp_last_wins_resolution() {
        let outcome =
            SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_with_diagnostics_like_cpp(
                [
                    SpellRankEdgeLikeCpp {
                        spell_id: 2,
                        supercedes_spell_id: 1,
                    },
                    SpellRankEdgeLikeCpp {
                        spell_id: 3,
                        supercedes_spell_id: 1,
                    },
                    SpellRankEdgeLikeCpp {
                        spell_id: 4,
                        supercedes_spell_id: 2,
                    },
                ],
                |_| true,
            );

        assert!(outcome.diagnostics_in_order_like_cpp.is_empty());
        assert_eq!(
            outcome.store.spell_chain_node_like_cpp(1),
            Some(&SpellChainNodeLikeCpp {
                prev_spell_id: None,
                next_spell_id: Some(3),
                first_spell_id: 1,
                last_spell_id: 3,
                rank: 1,
            })
        );
        assert_eq!(
            outcome.store.spell_chain_node_like_cpp(2),
            Some(&SpellChainNodeLikeCpp {
                prev_spell_id: None,
                next_spell_id: Some(4),
                first_spell_id: 2,
                last_spell_id: 4,
                rank: 1,
            }),
            "the child of an eclipsed edge must remain a root in the final graph"
        );
    }

    #[test]
    fn spell_chain_store_rejects_self_loops_and_pure_or_reachable_cycles() {
        let self_loop =
            SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_with_diagnostics_like_cpp(
                [SpellRankEdgeLikeCpp {
                    spell_id: 30,
                    supercedes_spell_id: 30,
                }],
                |_| true,
            );
        assert_eq!(
            self_loop.diagnostics_in_order_like_cpp,
            vec![SpellChainLoadDiagnosticLikeCpp::SelfLoop { spell_id: 30 }]
        );
        assert!(matches!(
            self_loop.store.spell_chain_lookup_like_cpp(30),
            SpellChainLookupLikeCpp::Indeterminate(diagnostics)
                if diagnostics == [SpellChainLoadDiagnosticLikeCpp::SelfLoop { spell_id: 30 }]
        ));

        let pure_cycle =
            SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_with_diagnostics_like_cpp(
                [
                    SpellRankEdgeLikeCpp {
                        spell_id: 20,
                        supercedes_spell_id: 10,
                    },
                    SpellRankEdgeLikeCpp {
                        spell_id: 10,
                        supercedes_spell_id: 20,
                    },
                ],
                |_| true,
            );
        assert_eq!(
            pure_cycle.diagnostics_in_order_like_cpp,
            vec![SpellChainLoadDiagnosticLikeCpp::Cycle {
                spell_ids: vec![10, 20],
            }]
        );
        assert!(matches!(
            pure_cycle.store.spell_chain_lookup_like_cpp(10),
            SpellChainLookupLikeCpp::Indeterminate(_)
        ));
        assert!(matches!(
            pure_cycle.store.spell_chain_lookup_like_cpp(20),
            SpellChainLookupLikeCpp::Indeterminate(_)
        ));

        let reachable_cycle =
            SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_with_diagnostics_like_cpp(
                [
                    SpellRankEdgeLikeCpp {
                        spell_id: 2,
                        supercedes_spell_id: 1,
                    },
                    SpellRankEdgeLikeCpp {
                        spell_id: 3,
                        supercedes_spell_id: 2,
                    },
                    SpellRankEdgeLikeCpp {
                        spell_id: 2,
                        supercedes_spell_id: 3,
                    },
                ],
                |_| true,
            );
        assert_eq!(
            reachable_cycle.diagnostics_in_order_like_cpp,
            vec![
                SpellChainLoadDiagnosticLikeCpp::MultiplePredecessors {
                    spell_id: 2,
                    predecessor_spell_ids: vec![1, 3],
                },
                SpellChainLoadDiagnosticLikeCpp::Cycle {
                    spell_ids: vec![2, 3],
                },
            ]
        );
        for spell_id in 1..=3 {
            assert!(matches!(
                reachable_cycle.store.spell_chain_lookup_like_cpp(spell_id),
                SpellChainLookupLikeCpp::Indeterminate(_)
            ));
        }
    }

    #[test]
    fn spell_chain_store_rejects_merge_components_without_partial_links() {
        let outcome =
            SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_with_diagnostics_like_cpp(
                [
                    SpellRankEdgeLikeCpp {
                        spell_id: 3,
                        supercedes_spell_id: 1,
                    },
                    SpellRankEdgeLikeCpp {
                        spell_id: 3,
                        supercedes_spell_id: 2,
                    },
                    SpellRankEdgeLikeCpp {
                        spell_id: 4,
                        supercedes_spell_id: 3,
                    },
                ],
                |_| true,
            );

        assert_eq!(
            outcome.diagnostics_in_order_like_cpp,
            vec![SpellChainLoadDiagnosticLikeCpp::MultiplePredecessors {
                spell_id: 3,
                predecessor_spell_ids: vec![1, 2],
            }]
        );
        assert!(outcome.store.chains_by_spell_id.is_empty());
        for spell_id in 1..=4 {
            assert!(matches!(
                outcome.store.spell_chain_lookup_like_cpp(spell_id),
                SpellChainLookupLikeCpp::Indeterminate(_)
            ));
        }
        assert_eq!(
            outcome.store.spell_chain_lookup_like_cpp(99),
            SpellChainLookupLikeCpp::Unranked
        );
    }

    #[test]
    fn spell_chain_store_propagates_invalid_effective_rows_to_the_whole_component() {
        let outcome =
            SpellChainStoreLikeCpp::from_skill_line_ability_rank_rows_with_diagnostics_like_cpp(
                [
                    SkillLineAbilityRankRowLikeCpp::Edge {
                        record_id: 1,
                        spell_id: 2,
                        supercedes_spell_id: 1,
                    },
                    SkillLineAbilityRankRowLikeCpp::Edge {
                        record_id: 2,
                        spell_id: 3,
                        supercedes_spell_id: 2,
                    },
                    SkillLineAbilityRankRowLikeCpp::Indeterminate {
                        record_id: 90,
                        spell_raw: 2,
                        supercedes_spell_raw: i128::from(i32::MAX) + 1,
                    },
                ],
                |spell_id| matches!(spell_id, 1 | 2 | 3),
            );

        assert!(outcome.store.chains_by_spell_id.is_empty());
        for spell_id in 1..=3 {
            assert!(matches!(
                outcome.store.spell_chain_lookup_like_cpp(spell_id),
                SpellChainLookupLikeCpp::Indeterminate(diagnostics)
                    if diagnostics == [SpellChainLoadDiagnosticLikeCpp::InvalidEffectiveSkillLineAbilityRankEndpoints {
                        record_id: 90,
                        spell_raw: 2,
                        supercedes_spell_raw: i128::from(i32::MAX) + 1,
                        affected_spell_ids: vec![2],
                    }]
            ));
        }
        assert_eq!(
            outcome.store.spell_chain_lookup_like_cpp(10),
            SpellChainLookupLikeCpp::Unranked
        );
    }

    #[test]
    fn spell_chain_store_propagates_invalid_spell_endpoint_from_the_predecessor() {
        let outcome =
            SpellChainStoreLikeCpp::from_skill_line_ability_rank_rows_with_diagnostics_like_cpp(
                [
                    SkillLineAbilityRankRowLikeCpp::Edge {
                        record_id: 1,
                        spell_id: 2,
                        supercedes_spell_id: 1,
                    },
                    SkillLineAbilityRankRowLikeCpp::Edge {
                        record_id: 2,
                        spell_id: 3,
                        supercedes_spell_id: 2,
                    },
                    SkillLineAbilityRankRowLikeCpp::Indeterminate {
                        record_id: 91,
                        spell_raw: i128::from(i32::MAX) + 1,
                        supercedes_spell_raw: 2,
                    },
                ],
                |spell_id| matches!(spell_id, 1 | 2 | 3),
            );

        assert!(outcome.store.chains_by_spell_id.is_empty());
        for spell_id in 1..=2 {
            assert!(matches!(
                outcome.store.spell_chain_lookup_like_cpp(spell_id),
                SpellChainLookupLikeCpp::Indeterminate(_)
            ));
        }
        assert_eq!(
            outcome.store.spell_chain_lookup_like_cpp(3),
            SpellChainLookupLikeCpp::Unranked,
            "the invalid final candidate eclipses the former 2→3 edge before components form"
        );
    }

    #[test]
    fn spell_chain_store_skips_invalid_row_with_a_proven_absent_endpoint() {
        let outcome =
            SpellChainStoreLikeCpp::from_skill_line_ability_rank_rows_with_diagnostics_like_cpp(
                [
                    SkillLineAbilityRankRowLikeCpp::Edge {
                        record_id: 1,
                        spell_id: 2,
                        supercedes_spell_id: 1,
                    },
                    SkillLineAbilityRankRowLikeCpp::Indeterminate {
                        record_id: 92,
                        spell_raw: 999_999,
                        supercedes_spell_raw: i128::from(i32::MAX) + 1,
                    },
                ],
                |spell_id| matches!(spell_id, 1 | 2),
            );

        assert!(outcome.diagnostics_in_order_like_cpp.is_empty());
        assert_eq!(outcome.store.chains_by_spell_id.len(), 2);
        assert!(matches!(
            outcome.store.spell_chain_lookup_like_cpp(1),
            SpellChainLookupLikeCpp::Node(node) if node.rank == 1
        ));
        assert!(matches!(
            outcome.store.spell_chain_lookup_like_cpp(2),
            SpellChainLookupLikeCpp::Node(node) if node.rank == 2
        ));
    }

    #[test]
    fn spell_chain_rank_authority_is_last_wins_across_valid_and_invalid_rows() {
        let repaired =
            SpellChainStoreLikeCpp::from_skill_line_ability_rank_rows_with_diagnostics_like_cpp(
                [
                    SkillLineAbilityRankRowLikeCpp::Indeterminate {
                        record_id: 10,
                        spell_raw: i128::from(i32::MAX) + 1,
                        supercedes_spell_raw: 1,
                    },
                    SkillLineAbilityRankRowLikeCpp::Edge {
                        record_id: 20,
                        spell_id: 2,
                        supercedes_spell_id: 1,
                    },
                ],
                |spell_id| matches!(spell_id, 1 | 2),
            );

        assert!(matches!(
            repaired.store.spell_chain_lookup_like_cpp(1),
            SpellChainLookupLikeCpp::Node(node)
                if node.rank == 1 && node.next_spell_id == Some(2)
        ));
        assert!(matches!(
            repaired.store.spell_chain_lookup_like_cpp(2),
            SpellChainLookupLikeCpp::Node(node)
                if node.rank == 2 && node.prev_spell_id == Some(1)
        ));
        assert_eq!(
            repaired
                .diagnostics_in_order_like_cpp
                .iter()
                .filter(|diagnostic| matches!(
                    diagnostic,
                    SpellChainLoadDiagnosticLikeCpp::InvalidEffectiveSkillLineAbilityRankEndpoints {
                        record_id: 10,
                        ..
                    }
                ))
                .count(),
            1,
            "an eclipsed malformed source remains observable without poisoning final authority"
        );

        let eclipsed =
            SpellChainStoreLikeCpp::from_skill_line_ability_rank_rows_with_diagnostics_like_cpp(
                [
                    SkillLineAbilityRankRowLikeCpp::Edge {
                        record_id: 10,
                        spell_id: 2,
                        supercedes_spell_id: 1,
                    },
                    SkillLineAbilityRankRowLikeCpp::Indeterminate {
                        record_id: 20,
                        spell_raw: i128::from(i32::MAX) + 1,
                        supercedes_spell_raw: 1,
                    },
                ],
                |spell_id| matches!(spell_id, 1 | 2),
            );

        assert!(matches!(
            eclipsed.store.spell_chain_lookup_like_cpp(1),
            SpellChainLookupLikeCpp::Indeterminate(diagnostics)
                if diagnostics == [SpellChainLoadDiagnosticLikeCpp::InvalidEffectiveSkillLineAbilityRankEndpoints {
                    record_id: 20,
                    spell_raw: i128::from(i32::MAX) + 1,
                    supercedes_spell_raw: 1,
                    affected_spell_ids: vec![1],
                }]
        ));
        assert_eq!(
            eclipsed.store.spell_chain_lookup_like_cpp(2),
            SpellChainLookupLikeCpp::Unranked,
            "the destination of an eclipsed edge must not remain in the ambiguous component"
        );
    }

    #[test]
    fn invalid_rank_seed_unites_every_touched_valid_component() {
        let mut outcome =
            SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_with_diagnostics_like_cpp(
                [
                    SpellRankEdgeLikeCpp {
                        spell_id: 2,
                        supercedes_spell_id: 1,
                    },
                    SpellRankEdgeLikeCpp {
                        spell_id: 11,
                        supercedes_spell_id: 10,
                    },
                ],
                |_| true,
            );

        outcome.mark_invalid_skill_line_ability_rank_row_like_cpp(
            93,
            i128::from(i32::MAX) + 1,
            i128::from(i32::MAX) + 2,
            &[2, 11],
        );

        assert!(outcome.store.chains_by_spell_id.is_empty());
        for spell_id in [1, 2, 10, 11] {
            assert!(matches!(
                outcome.store.spell_chain_lookup_like_cpp(spell_id),
                SpellChainLookupLikeCpp::Indeterminate(diagnostics)
                    if diagnostics == [SpellChainLoadDiagnosticLikeCpp::InvalidEffectiveSkillLineAbilityRankEndpoints {
                        record_id: 93,
                        spell_raw: i128::from(i32::MAX) + 1,
                        supercedes_spell_raw: i128::from(i32::MAX) + 2,
                        affected_spell_ids: vec![2, 11],
                    }]
            ));
        }
    }

    #[test]
    fn invalid_rank_seed_preserves_existing_component_diagnostics() {
        let mut outcome =
            SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_with_diagnostics_like_cpp(
                [
                    SpellRankEdgeLikeCpp {
                        spell_id: 2,
                        supercedes_spell_id: 1,
                    },
                    SpellRankEdgeLikeCpp {
                        spell_id: 1,
                        supercedes_spell_id: 2,
                    },
                ],
                |_| true,
            );

        outcome.mark_invalid_skill_line_ability_rank_row_like_cpp(
            94,
            1,
            i128::from(i32::MAX) + 1,
            &[1],
        );

        for spell_id in [1, 2] {
            assert!(matches!(
                outcome.store.spell_chain_lookup_like_cpp(spell_id),
                SpellChainLookupLikeCpp::Indeterminate(diagnostics)
                    if diagnostics == [
                        SpellChainLoadDiagnosticLikeCpp::Cycle {
                            spell_ids: vec![1, 2],
                        },
                        SpellChainLoadDiagnosticLikeCpp::InvalidEffectiveSkillLineAbilityRankEndpoints {
                            record_id: 94,
                            spell_raw: 1,
                            supercedes_spell_raw: i128::from(i32::MAX) + 1,
                            affected_spell_ids: vec![1],
                        },
                    ]
            ));
        }
    }

    #[test]
    fn global_rank_seed_preserves_existing_component_diagnostics() {
        let mut outcome =
            SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_with_diagnostics_like_cpp(
                [
                    SpellRankEdgeLikeCpp {
                        spell_id: 2,
                        supercedes_spell_id: 1,
                    },
                    SpellRankEdgeLikeCpp {
                        spell_id: 1,
                        supercedes_spell_id: 2,
                    },
                ],
                |_| true,
            );

        outcome.mark_invalid_skill_line_ability_rank_row_like_cpp(
            95,
            i128::from(i32::MAX) + 1,
            i128::from(i32::MAX) + 2,
            &[],
        );

        for spell_id in [1, 2, 999] {
            assert!(matches!(
                outcome.store.spell_chain_lookup_like_cpp(spell_id),
                SpellChainLookupLikeCpp::Indeterminate(diagnostics)
                    if diagnostics == [
                        SpellChainLoadDiagnosticLikeCpp::Cycle {
                            spell_ids: vec![1, 2],
                        },
                        SpellChainLoadDiagnosticLikeCpp::InvalidEffectiveSkillLineAbilityRankEndpoints {
                            record_id: 95,
                            spell_raw: i128::from(i32::MAX) + 1,
                            supercedes_spell_raw: i128::from(i32::MAX) + 2,
                            affected_spell_ids: Vec::new(),
                        },
                    ]
            ));
        }
        assert!(outcome.store.indeterminate_by_spell_id_like_cpp.is_empty());
    }

    #[test]
    fn spell_chain_store_global_seed_fails_every_lookup_closed() {
        let outcome =
            SpellChainStoreLikeCpp::from_skill_line_ability_rank_rows_with_diagnostics_like_cpp(
                [
                    SkillLineAbilityRankRowLikeCpp::Edge {
                        record_id: 1,
                        spell_id: 2,
                        supercedes_spell_id: 1,
                    },
                    SkillLineAbilityRankRowLikeCpp::Indeterminate {
                        record_id: 91,
                        spell_raw: i128::from(i32::MAX) + 1,
                        supercedes_spell_raw: i128::from(i32::MAX) + 2,
                    },
                ],
                |spell_id| matches!(spell_id, 1 | 2),
            );

        assert!(outcome.store.chains_by_spell_id.is_empty());
        for spell_id in [1, 2, 999] {
            assert!(matches!(
                outcome.store.spell_chain_lookup_like_cpp(spell_id),
                SpellChainLookupLikeCpp::Indeterminate(_)
            ));
        }
    }

    #[test]
    fn spell_chain_store_rejects_ranks_wider_than_cpp_uint8() {
        let outcome =
            SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_with_diagnostics_like_cpp(
                (1..=u32::from(u8::MAX)).map(|spell_id| SpellRankEdgeLikeCpp {
                    spell_id: spell_id + 1,
                    supercedes_spell_id: spell_id,
                }),
                |_| true,
            );

        assert_eq!(
            outcome.diagnostics_in_order_like_cpp,
            vec![SpellChainLoadDiagnosticLikeCpp::RankOutOfRange {
                first_spell_id: 1,
                spell_id: 256,
                rank: 256,
            }]
        );
        assert!(outcome.store.chains_by_spell_id.is_empty());
        assert!(matches!(
            outcome.store.spell_chain_lookup_like_cpp(1),
            SpellChainLookupLikeCpp::Indeterminate(_)
        ));
        assert!(matches!(
            outcome.store.spell_chain_lookup_like_cpp(256),
            SpellChainLookupLikeCpp::Indeterminate(_)
        ));
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
                attributes: SPELL_ATTR0_CU_CAN_CRIT_LIKE_CPP,
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
                attributes: SPELL_ATTR0_CU_SHARE_DAMAGE_LIKE_CPP,
                kind: SpellCustomAttributeLoadErrorKindLikeCpp::ShareDamageWithoutSchoolDamage,
            }]
        );
    }

    #[test]
    fn spell_custom_attribute_store_applies_non_effect_attribute_with_unknown_effect_coverage() {
        let outcome = SpellCustomAttributeStoreLikeCpp::from_sql_rows_for_variants_like_cpp(
            [SpellCustomAttributeRowLikeCpp {
                spell_id: 100,
                attributes: SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP,
            }],
            |spell_id| {
                (spell_id == 100)
                    .then_some(vec![SpellCustomAttributeSourceVariantLikeCpp {
                        spell_id,
                        difficulty: 2,
                        effect_types: None,
                    }])
                    .unwrap_or_default()
            },
        );

        assert!(outcome.errors.is_empty());
        assert_eq!(outcome.loaded_row_count, 1);
        assert_eq!(outcome.applied_variant_count, 1);
        assert_eq!(
            outcome
                .store
                .attributes_for_spell_difficulty_like_cpp(100, 2),
            SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP
        );
    }

    #[test]
    fn spell_custom_attribute_store_rejects_share_damage_with_unknown_effect_coverage() {
        let outcome = SpellCustomAttributeStoreLikeCpp::from_sql_rows_for_variants_like_cpp(
            [SpellCustomAttributeRowLikeCpp {
                spell_id: 100,
                attributes: SPELL_ATTR0_CU_SHARE_DAMAGE_LIKE_CPP,
            }],
            |spell_id| {
                (spell_id == 100)
                    .then_some(vec![SpellCustomAttributeSourceVariantLikeCpp {
                        spell_id,
                        difficulty: 2,
                        effect_types: None,
                    }])
                    .unwrap_or_default()
            },
        );

        assert_eq!(outcome.loaded_row_count, 1);
        assert_eq!(outcome.applied_variant_count, 0);
        assert_eq!(
            outcome
                .store
                .attributes_for_spell_difficulty_like_cpp(100, 2),
            0
        );
        assert_eq!(
            outcome.errors,
            vec![SpellCustomAttributeLoadErrorLikeCpp {
                spell_id: 100,
                difficulty: Some(2),
                attributes: SPELL_ATTR0_CU_SHARE_DAMAGE_LIKE_CPP,
                kind:
                    SpellCustomAttributeLoadErrorKindLikeCpp::ShareDamageEffectCoverageUnavailable,
            }]
        );
    }

    #[test]
    fn spell_custom_attribute_store_queries_attributes_across_exact_difficulties() {
        let store = SpellCustomAttributeStoreLikeCpp {
            attributes_by_spell_and_difficulty: BTreeMap::from([
                (
                    SpellCustomAttributeKeyLikeCpp {
                        spell_id: 100,
                        difficulty: 0,
                    },
                    SPELL_ATTR0_CU_CAN_CRIT_LIKE_CPP,
                ),
                (
                    SpellCustomAttributeKeyLikeCpp {
                        spell_id: 100,
                        difficulty: 2,
                    },
                    SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP,
                ),
                (
                    SpellCustomAttributeKeyLikeCpp {
                        spell_id: 101,
                        difficulty: 0,
                    },
                    SPELL_ATTR0_CU_DIRECT_DAMAGE_LIKE_CPP,
                ),
            ]),
        };

        assert_eq!(
            store.attributes_for_spell_any_difficulty_like_cpp(100),
            SPELL_ATTR0_CU_CAN_CRIT_LIKE_CPP | SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP
        );
        assert!(
            store.has_attribute_any_difficulty_like_cpp(100, SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP)
        );
        assert!(
            !store
                .has_attribute_any_difficulty_like_cpp(100, SPELL_ATTR0_CU_DIRECT_DAMAGE_LIKE_CPP)
        );
        assert_eq!(store.attributes_for_spell_any_difficulty_like_cpp(999), 0);
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
                power_costs: Vec::new(),
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
            power_costs: Vec::new(),
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
            power_costs: Vec::new(),
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

    fn test_spell_effect_db2_entry_like_cpp(
        id: u32,
        spell_id: u32,
        difficulty_id: i32,
        effect_index: i32,
        effect: u32,
        effect_mechanic: i32,
    ) -> crate::spell_db2::SpellEffectDb2Entry {
        crate::spell_db2::SpellEffectDb2Entry {
            id,
            difficulty_id,
            effect_index,
            effect,
            effect_amplitude: 0.0,
            effect_attributes: 0,
            effect_aura: 0,
            effect_aura_period: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            effect_chain_amplitude: 0.0,
            effect_chain_targets: 0,
            effect_die_sides: 0,
            effect_item_type: 0,
            effect_mechanic,
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
            effect_misc_value: [0; 2],
            effect_radius_index: [0; 2],
            effect_spell_class_mask: [0; 4],
            implicit_target: [0; 2],
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
    fn spell_learn_spell_store_keeps_effect_and_db2_edges_when_world_sql_is_empty() {
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
        assert_eq!(outcome.dbc_loaded_row_count, 2);
        assert_eq!(
            outcome.store.get_spell_learn_spell_map_bounds_like_cpp(100),
            &[SpellLearnSpellNodeLikeCpp {
                spell: 101,
                overrides_spell: 0,
                active: true,
                auto_learned: false,
            }]
        );
        assert_eq!(
            outcome.store.get_spell_learn_spell_map_bounds_like_cpp(200),
            &[SpellLearnSpellNodeLikeCpp {
                spell: 201,
                overrides_spell: 0,
                active: true,
                auto_learned: false,
            }]
        );
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

    fn serverside_spell_info_for_shapeshift(
        stances: u64,
        stances_not: u64,
        attributes: u32,
        attributes_ex2: u32,
    ) -> ServersideSpellInfoLikeCpp {
        let mut row = serverside_spell_row(7000, 0);
        row.attributes = attributes;
        row.attributes_ex = [0; 14];
        row.attributes_ex[1] = attributes_ex2;
        row.stances = stances;
        row.stances_not = stances_not;
        ServersideSpellInfoLikeCpp {
            row,
            effects: Vec::new(),
        }
    }

    fn shapeshift_form(flags: i32) -> crate::spell_db2::SpellShapeshiftFormEntry {
        crate::spell_db2::SpellShapeshiftFormEntry {
            id: 1,
            name: "Test Form".to_string(),
            creature_type: 0,
            flags,
            attack_icon_file_id: 0,
            bonus_action_bar: 0,
            combat_round_time: 0,
            damage_variance: 0.0,
            mount_type_id: 0,
            creature_display_id: [0; 4],
            preset_spell_id: [0; crate::spell_db2::MAX_SHAPESHIFT_SPELLS],
        }
    }

    #[test]
    fn serverside_spell_check_shapeshift_rejects_excluded_form_like_cpp() {
        let spell = serverside_spell_info_for_shapeshift(0, 1 << 2, 0, 0);
        let form = shapeshift_form(shapeshift_form_flags::STANCE);

        assert_eq!(
            spell.check_shapeshift_like_cpp(3, |_| Some(&form)),
            SpellCastResult::NotShapeshift
        );
    }

    #[test]
    fn serverside_spell_check_shapeshift_allows_explicit_form_like_cpp() {
        let spell = serverside_spell_info_for_shapeshift(1 << 4, 0, 0, 0);
        let form = shapeshift_form(shapeshift_form_flags::STANCE);

        assert_eq!(
            spell.check_shapeshift_like_cpp(5, |_| Some(&form)),
            SpellCastResult::Success
        );
    }

    #[test]
    fn serverside_spell_check_shapeshift_missing_form_allows_like_cpp() {
        let spell =
            serverside_spell_info_for_shapeshift(0, 0, attributes::SPELL_ATTR0_NOT_SHAPESHIFTED, 0);

        assert_eq!(
            spell.check_shapeshift_like_cpp(7, |_| None),
            SpellCastResult::Success
        );
    }

    #[test]
    fn serverside_spell_check_shapeshift_rejects_not_shapeshifted_attr_like_cpp() {
        let spell =
            serverside_spell_info_for_shapeshift(0, 0, attributes::SPELL_ATTR0_NOT_SHAPESHIFTED, 0);
        let form = shapeshift_form(0);

        assert_eq!(
            spell.check_shapeshift_like_cpp(1, |_| Some(&form)),
            SpellCastResult::NotShapeshift
        );
    }

    #[test]
    fn serverside_spell_check_shapeshift_rejects_can_only_cast_shapeshift_spells_like_cpp() {
        let spell = serverside_spell_info_for_shapeshift(0, 0, 0, 0);
        let form = shapeshift_form(shapeshift_form_flags::CAN_ONLY_CAST_SHAPESHIFT_SPELLS);

        assert_eq!(
            spell.check_shapeshift_like_cpp(1, |_| Some(&form)),
            SpellCastResult::NotShapeshift
        );
    }

    #[test]
    fn serverside_spell_check_shapeshift_requires_other_shifted_form_like_cpp() {
        let spell = serverside_spell_info_for_shapeshift(1 << 4, 0, 0, 0);
        let form = shapeshift_form(0);

        assert_eq!(
            spell.check_shapeshift_like_cpp(2, |_| Some(&form)),
            SpellCastResult::OnlyShapeshift
        );
    }

    #[test]
    fn serverside_spell_check_shapeshift_requires_form_when_unshifted_like_cpp() {
        let spell = serverside_spell_info_for_shapeshift(1 << 4, 0, 0, 0);

        assert_eq!(
            spell.check_shapeshift_like_cpp(0, |_| None),
            SpellCastResult::OnlyShapeshift
        );
    }

    #[test]
    fn serverside_spell_check_shapeshift_allows_unshifted_with_attr2_like_cpp() {
        let spell = serverside_spell_info_for_shapeshift(
            1 << 4,
            0,
            0,
            attributes::SPELL_ATTR2_ALLOW_WHILE_NOT_SHAPESHIFTED_CASTER_FORM,
        );

        assert_eq!(
            spell.check_shapeshift_like_cpp(0, |_| None),
            SpellCastResult::Success
        );
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
