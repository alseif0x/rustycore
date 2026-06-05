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

use std::collections::HashMap;
use std::f32::consts::TAU;

use anyhow::Result;
use tracing::info;
use wow_database::{HotfixDatabase, WorldDatabase};

use crate::{ConditionEntriesByTypeStore, ConditionsReference};

/// Spell effect types (from SpellEffectType enum)
pub mod spell_effect_types {
    pub const SPELL_EFFECT_NONE: u32 = 0;
    pub const SPELL_EFFECT_INSTAKILL: u32 = 1;
    pub const SPELL_EFFECT_SCHOOL_DAMAGE: u32 = 2;
    pub const SPELL_EFFECT_PORTAL_TELEPORT: u32 = 4;
    pub const SPELL_EFFECT_ENVIRONMENTAL_DAMAGE: u32 = 7;
    pub const SPELL_EFFECT_APPLY_AURA: u32 = 6;
    pub const SPELL_EFFECT_HEALTH_LEECH: u32 = 9;
    pub const SPELL_EFFECT_HEAL: u32 = 10;
    pub const SPELL_EFFECT_BIND: u32 = 11;
    pub const SPELL_EFFECT_PORTAL: u32 = 12;
    pub const SPELL_EFFECT_RITUAL_BASE: u32 = 13;
    pub const SPELL_EFFECT_RITUAL_SPECIALIZE: u32 = 14;
    pub const SPELL_EFFECT_RITUAL_ACTIVATE_PORTAL: u32 = 15;
    pub const SPELL_EFFECT_QUEST_COMPLETE: u32 = 16;
    pub const SPELL_EFFECT_DODGE: u32 = 20;
    pub const SPELL_EFFECT_EVADE: u32 = 21;
    pub const SPELL_EFFECT_PARRY: u32 = 22;
    pub const SPELL_EFFECT_BLOCK: u32 = 23;
    pub const SPELL_EFFECT_WEAPON: u32 = 25;
    pub const SPELL_EFFECT_DEFENSE: u32 = 26;
    pub const SPELL_EFFECT_PERSISTENT_AREA_AURA: u32 = 27;
    pub const SPELL_EFFECT_APPLY_AREA_AURA_PARTY: u32 = 35;
    pub const SPELL_EFFECT_SPELL_DEFENSE: u32 = 37;
    pub const SPELL_EFFECT_LANGUAGE: u32 = 39;
    pub const SPELL_EFFECT_DUAL_WIELD: u32 = 40;
    pub const SPELL_EFFECT_SPAWN: u32 = 46;
    pub const SPELL_EFFECT_STEALTH: u32 = 48;
    pub const SPELL_EFFECT_DETECT: u32 = 49;
    pub const SPELL_EFFECT_FORCE_CRITICAL_HIT: u32 = 51;
    pub const SPELL_EFFECT_GUARANTEE_HIT: u32 = 52;
    pub const SPELL_EFFECT_APPLY_AREA_AURA_RAID: u32 = 65;
    pub const SPELL_EFFECT_HEAL_MAX_HEALTH: u32 = 67;
    pub const SPELL_EFFECT_ADD_FARSIGHT: u32 = 72;
    pub const SPELL_EFFECT_HEAL_MECHANICAL: u32 = 75;
    /// C++ `SPELL_EFFECT_SUMMON_OBJECT_WILD`; see
    /// `Spell::EffectSummonObjectWild` (`SpellEffects.cpp:2937-2986`).
    pub const SPELL_EFFECT_SUMMON_OBJECT_WILD: u32 = 76;
    pub const SPELL_EFFECT_ATTACK: u32 = 78;
    pub const SPELL_EFFECT_CREATE_HOUSE: u32 = 81;
    pub const SPELL_EFFECT_BIND_SIGHT: u32 = 82;
    pub const SPELL_EFFECT_KILL_CREDIT: u32 = 90;
    pub const SPELL_EFFECT_THREAT_ALL: u32 = 91;
    /// First C++ `SPELL_EFFECT_SUMMON_OBJECT_SLOT*` value; see
    /// `Spell::EffectSummonObject` (`SpellEffects.cpp:3541-3597`).
    pub const SPELL_EFFECT_SUMMON_OBJECT_SLOT1: u32 = 104;
    pub const SPELL_EFFECT_SURVEY: u32 = 105;
    pub const SPELL_EFFECT_SHOW_CORPSE_LOOT: u32 = 107;
    pub const SPELL_EFFECT_112: u32 = 112;
    pub const SPELL_EFFECT_APPLY_AREA_AURA_PET: u32 = 119;
    pub const SPELL_EFFECT_122: u32 = 122;
    pub const SPELL_EFFECT_APPLY_AREA_AURA_FRIEND: u32 = 128;
    pub const SPELL_EFFECT_APPLY_AREA_AURA_ENEMY: u32 = 129;
    pub const SPELL_EFFECT_KILL_CREDIT2: u32 = 134;
    pub const SPELL_EFFECT_CALL_PET: u32 = 135;
    pub const SPELL_EFFECT_HEAL_PCT: u32 = 136;
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
    pub const SPELL_EFFECT_START_PET_BATTLE: u32 = 193;
    pub const SPELL_EFFECT_194: u32 = 194;
    pub const SPELL_EFFECT_DESPAWN_SUMMON: u32 = 199;
    pub const SPELL_EFFECT_APPLY_AREA_AURA_SUMMONS: u32 = 202;
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
    pub const SPELL_EFFECT_SET_GARRISON_FOLLOWER_LEVEL: u32 = 287;
    pub const SPELL_EFFECT_CRAFT_ITEM: u32 = 288;
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

    /// C++ dispatch entries that intentionally run `EffectNULL` or
    /// `EffectUnused` in `SpellEffects.cpp` for the represented early effect
    /// range. This deliberately excludes `SPELL_EFFECT_DUMMY`, whose behavior
    /// is script-driven through `ScriptMgr::OnSpellEffectDummy`.
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
    pub const SPELL_AURA_MOD_TAUNT: i32 = 11;
    pub const SPELL_AURA_MODIFY_DAMAGE_PERCENT_TAKEN: i32 = 31;
    pub const SPELL_AURA_HASTE_SPELLS: i32 = 73;
    pub const SPELL_AURA_MOUNTED: i32 = 78;
    pub const SPELL_AURA_PROVIDE_SPELL_FOCUS: i32 = 281;
}

/// Selected `Targets` ids from C++ `SpellImplicitTargetInfo::_data`.
pub mod implicit_targets {
    pub const TARGET_DEST_DB: u32 = 17;
    pub const TARGET_DEST_NEARBY_ENTRY: u32 = 46;
    pub const TARGET_DEST_NEARBY_ENTRY_2: u32 = 107;
    pub const TARGET_DEST_NEARBY_ENTRY_OR_DB: u32 = 142;
}

pub mod attributes {
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
    pub effect_misc_value_1: i32,
    pub effect_misc_value_2: i32,
    /// C++ `SpellEffectEntry::EffectRadiusIndex[0]` / TargetA radius index.
    pub effect_radius_index_1: u32,
    pub position_facing: f32,
    pub chain_targets: i32,
    pub implicit_target_1: u32,
    pub implicit_target_2: u32,
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
    pub fn is_mounted_aura_like_cpp(&self) -> bool {
        self.effect == spell_effect_types::SPELL_EFFECT_APPLY_AURA
            && self.effect_aura == aura_types::SPELL_AURA_MOUNTED
    }

    pub fn is_provide_spell_focus_aura_like_cpp(&self) -> bool {
        self.effect == spell_effect_types::SPELL_EFFECT_APPLY_AURA
            && self.effect_aura == aura_types::SPELL_AURA_PROVIDE_SPELL_FOCUS
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
            .direct_query(
                "SELECT ID, EffectIndex, MapID, PositionX, PositionY, PositionZ, Orientation FROM spell_target_position",
            )
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
    CAST(COALESCE(se.EffectRadiusIndex1, 0) AS UNSIGNED) as effect_radius_index_1,
    CAST(COALESCE(se.EffectPosFacing, 0.0) AS DECIMAL(10,4)) as position_facing,
    CAST(COALESCE(se.EffectIndex, 0) AS UNSIGNED) as effect_index,
    CAST(COALESCE(se.EffectChainTargets, 0) AS SIGNED) as effect_chain_targets,
    CAST(COALESCE(se.ImplicitTarget1, 0) AS UNSIGNED) as implicit_target_1,
    CAST(COALESCE(se.ImplicitTarget2, 0) AS UNSIGNED) as implicit_target_2,
    CAST(COALESCE(scr.RequiresSpellFocus, 0) AS UNSIGNED) as requires_spell_focus
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
                let effect_radius_index_1: u32 = result.try_read(10).unwrap_or(0);
                let position_facing: f32 = result.try_read(11).unwrap_or(0.0);
                let effect_index: u32 = result.try_read(12).unwrap_or(0);
                let effect_chain_targets: i32 = result.try_read(13).unwrap_or(0);
                let implicit_target_1: u32 = result.try_read(14).unwrap_or(0);
                let implicit_target_2: u32 = result.try_read(15).unwrap_or(0);
                let requires_spell_focus: u32 = result.try_read(16).unwrap_or(0);

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
                        effect_misc_value_1,
                        effect_misc_value_2,
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
    fn spell_effect_constants_match_cpp_shared_defines() {
        // C++ `SharedDefines.h`: `SpellEffects` enum.
        assert_eq!(spell_effect_types::SPELL_EFFECT_NONE, 0);
        assert_eq!(spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE, 2);
        assert_eq!(spell_effect_types::SPELL_EFFECT_PORTAL_TELEPORT, 4);
        assert_eq!(spell_effect_types::SPELL_EFFECT_APPLY_AURA, 6);
        assert_eq!(spell_effect_types::SPELL_EFFECT_ENVIRONMENTAL_DAMAGE, 7);
        assert_eq!(spell_effect_types::SPELL_EFFECT_HEALTH_LEECH, 9);
        assert_eq!(spell_effect_types::SPELL_EFFECT_HEAL, 10);
        assert_eq!(spell_effect_types::SPELL_EFFECT_BIND, 11);
        assert_eq!(spell_effect_types::SPELL_EFFECT_PORTAL, 12);
        assert_eq!(spell_effect_types::SPELL_EFFECT_RITUAL_BASE, 13);
        assert_eq!(spell_effect_types::SPELL_EFFECT_RITUAL_SPECIALIZE, 14);
        assert_eq!(spell_effect_types::SPELL_EFFECT_RITUAL_ACTIVATE_PORTAL, 15);
        assert_eq!(spell_effect_types::SPELL_EFFECT_QUEST_COMPLETE, 16);
        assert_eq!(spell_effect_types::SPELL_EFFECT_DODGE, 20);
        assert_eq!(spell_effect_types::SPELL_EFFECT_EVADE, 21);
        assert_eq!(spell_effect_types::SPELL_EFFECT_PARRY, 22);
        assert_eq!(spell_effect_types::SPELL_EFFECT_BLOCK, 23);
        assert_eq!(spell_effect_types::SPELL_EFFECT_WEAPON, 25);
        assert_eq!(spell_effect_types::SPELL_EFFECT_DEFENSE, 26);
        assert_eq!(spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_PARTY, 35);
        assert_eq!(spell_effect_types::SPELL_EFFECT_SPELL_DEFENSE, 37);
        assert_eq!(spell_effect_types::SPELL_EFFECT_LANGUAGE, 39);
        assert_eq!(spell_effect_types::SPELL_EFFECT_DUAL_WIELD, 40);
        assert_eq!(spell_effect_types::SPELL_EFFECT_SPAWN, 46);
        assert_eq!(spell_effect_types::SPELL_EFFECT_STEALTH, 48);
        assert_eq!(spell_effect_types::SPELL_EFFECT_DETECT, 49);
        assert_eq!(spell_effect_types::SPELL_EFFECT_FORCE_CRITICAL_HIT, 51);
        assert_eq!(spell_effect_types::SPELL_EFFECT_GUARANTEE_HIT, 52);
        assert_eq!(spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_RAID, 65);
        assert_eq!(spell_effect_types::SPELL_EFFECT_HEAL_MAX_HEALTH, 67);
        assert_eq!(spell_effect_types::SPELL_EFFECT_HEAL_MECHANICAL, 75);
        assert_eq!(spell_effect_types::SPELL_EFFECT_ATTACK, 78);
        assert_eq!(spell_effect_types::SPELL_EFFECT_CREATE_HOUSE, 81);
        assert_eq!(spell_effect_types::SPELL_EFFECT_BIND_SIGHT, 82);
        assert_eq!(spell_effect_types::SPELL_EFFECT_KILL_CREDIT, 90);
        assert_eq!(spell_effect_types::SPELL_EFFECT_THREAT_ALL, 91);
        assert_eq!(spell_effect_types::SPELL_EFFECT_SURVEY, 105);
        assert_eq!(spell_effect_types::SPELL_EFFECT_SHOW_CORPSE_LOOT, 107);
        assert_eq!(spell_effect_types::SPELL_EFFECT_112, 112);
        assert_eq!(spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_PET, 119);
        assert_eq!(spell_effect_types::SPELL_EFFECT_122, 122);
        assert_eq!(spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_FRIEND, 128);
        assert_eq!(spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_ENEMY, 129);
        assert_eq!(spell_effect_types::SPELL_EFFECT_KILL_CREDIT2, 134);
        assert_eq!(spell_effect_types::SPELL_EFFECT_CALL_PET, 135);
        assert_eq!(spell_effect_types::SPELL_EFFECT_HEAL_PCT, 136);
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
        assert_eq!(spell_effect_types::SPELL_EFFECT_START_PET_BATTLE, 193);
        assert_eq!(spell_effect_types::SPELL_EFFECT_194, 194);
        assert_eq!(spell_effect_types::SPELL_EFFECT_DESPAWN_SUMMON, 199);
        assert_eq!(
            spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_SUMMONS,
            202
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
            spell_effect_types::SPELL_EFFECT_SET_GARRISON_FOLLOWER_LEVEL,
            287
        );
        assert_eq!(spell_effect_types::SPELL_EFFECT_CRAFT_ITEM, 288);
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
            243,
            245,
            spell_effect_types::SPELL_EFFECT_TELEPORT_UNITS,
            spell_effect_types::SPELL_EFFECT_GIVE_HONOR,
            spell_effect_types::SPELL_EFFECT_JUMP_CHARGE,
            spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_SET,
            spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_ILLUSION,
            284,
            286,
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
}
