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

/// Aura types (from AuraType enum)

/// Selected `Targets` ids from C++ `SpellImplicitTargetInfo::_data`.
pub mod implicit_targets {
    pub const TARGET_DEST_HOME: u32 = 9;
    pub const TARGET_DEST_DB: u32 = 17;
    pub const TARGET_DEST_NEARBY_ENTRY: u32 = 46;
    pub const TARGET_DEST_NEARBY_ENTRY_2: u32 = 107;
    pub const TARGET_DEST_NEARBY_ENTRY_OR_DB: u32 = 142;
}

mod state_1;
mod state_2;
#[allow(unused_imports)]
pub use state_1::*;
#[allow(unused_imports)]
pub use state_2::*;

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

pub mod aura_types;
pub mod spell_effect_types;
#[cfg(test)]
#[path = "../spell_tests.rs"]
mod tests;
