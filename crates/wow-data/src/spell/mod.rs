// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spell static data.
//!
//! Issue #227 split the former 7,559-line `spell.rs` by real catalog/store
//! responsibility. Every public type and data contract is unchanged.

mod acquisition;
mod catalog;
mod corrections;
mod stores;

pub use acquisition::*;
pub use catalog::*;
pub use corrections::*;
pub use stores::*;

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use std::f32::consts::TAU;

use anyhow::Result;

use tracing::info;

use wow_constants::{PowerType, SpellCastResult};

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

/// Calculated spell power cost, mirroring C++ `Spell::m_powerCost`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellPowerCostLikeCpp {
    pub power_type: i8,
    pub amount: i32,
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
pub struct SpellThreatLoadErrorLikeCpp {
    pub row: SpellThreatRowLikeCpp,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellRequiredLoadOutcomeLikeCpp {
    pub store: SpellRequiredStoreLikeCpp,
    pub loaded_row_count: usize,
    pub errors: Vec<SpellRequiredLoadErrorLikeCpp>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellAreaLoadOutcomeLikeCpp {
    pub store: SpellAreaStoreLikeCpp,
    pub loaded_row_count: usize,
    pub errors: Vec<SpellAreaLoadErrorLikeCpp>,
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

fn calculate_pct_i32_like_cpp(base: i32, pct: f32) -> i32 {
    ((base as f32) * pct / 100.0) as i32
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

#[cfg(test)]
#[path = "../spell_tests.rs"]
mod tests;
