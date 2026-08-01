// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Pure, deterministic projection of player spell and skill acquisition.
//! Focused fixtures live beside the implementation under `tests/`.
//!
//! The planner mirrors the causal parts of C++ `Player::LearnSpell`,
//! `Player::AddSpell`, `Player::SetSkill` and trainer-wrapper casts without
//! mutating a session, writing a database row or publishing a packet.  It
//! deliberately fails closed when the immutable metadata cannot prove every
//! acquisition-relevant edge.

use std::collections::{BTreeMap, BTreeSet};

use wow_data::{
    MountStore, SKILL_FLAG_ALWAYS_MAX_VALUE_LIKE_CPP, SKILL_RUNEFORGING_LIKE_CPP,
    SkillLineAbilityCoverageLikeCpp, SkillLineAcquisitionFieldsLikeCpp,
    SkillLineAcquisitionPayloadLikeCpp, SkillLineStore, SkillRaceClassInfoMatchCoverageLikeCpp,
    SkillRangeTypeLikeCpp, SkillStore, SkillStoreLoadDiagnosticLikeCpp, SkillTiersStoreLikeCpp,
    SpellAcquisitionCatalogLikeCpp, SpellAcquisitionDependenciesLookupLikeCpp,
    SpellAcquisitionEffectLikeCpp, SpellAcquisitionEffectsLookupLikeCpp,
    SpellAcquisitionIndeterminateReasonLikeCpp, SpellAcquisitionMetadataLookupLikeCpp,
    SpellAcquisitionMiscLikeCpp, SpellAcquisitionTableLikeCpp, SpellAcquisitionTalentLookupLikeCpp,
    SpellChainLoadDiagnosticLikeCpp, SpellChainLookupLikeCpp, SpellChainNodeLikeCpp,
    SpellChainStoreLikeCpp, SpellCustomAttributeStoreLikeCpp,
    SpellLearnSkillIndeterminateReasonLikeCpp, SpellLearnSkillLookupLikeCpp,
    SpellLearnSkillNodeLikeCpp, SpellLearnSkillStoreLikeCpp, SpellLearnSpellNodeLikeCpp,
    SpellLearnSpellStoreLikeCpp, SpellRequiredStoreLikeCpp,
};

use wow_data::skill::{
    CLASS_DEATH_KNIGHT_LIKE_CPP, SKILL_CATEGORY_PROFESSION_LIKE_CPP,
    SKILL_LINE_ABILITY_CAN_FALLBACK_TO_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
    SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
    SKILL_LINE_ABILITY_LEARNED_ON_SKILL_VALUE_LIKE_CPP,
    SKILL_LINE_ABILITY_REWARDED_FROM_QUEST_LIKE_CPP, SKILL_RIDING_LIKE_CPP,
    race_mask_for_race_like_cpp,
};
use wow_data::spell::spell_effect_types::{
    SPELL_EFFECT_DUAL_WIELD, SPELL_EFFECT_LEARN_SPELL, SPELL_EFFECT_SKILL, SPELL_EFFECT_SKILL_STEP,
};
use wow_data::trait_tree::TraitDefinitionStore;

const DIFFICULTY_NONE_LIKE_CPP: u32 = 0;
const SPELL_EFFECT_DUMMY_LIKE_CPP: u32 = 3;
const SPELL_EFFECT_CREATE_ITEM_LIKE_CPP: u32 = 24;
const SPELL_EFFECT_SUMMON_LIKE_CPP: u32 = 28;
const SPELL_EFFECT_TRIGGER_MISSILE_LIKE_CPP: u32 = 32;
const SPELL_EFFECT_SUMMON_CHANGE_ITEM_LIKE_CPP: u32 = 34;
const SPELL_EFFECT_SUMMON_PET_LIKE_CPP: u32 = 56;
const SPELL_EFFECT_LEARN_PET_SPELL_LIKE_CPP: u32 = 57;
const SPELL_EFFECT_TRIGGER_SPELL_LIKE_CPP: u32 = 64;
const SPELL_EFFECT_SCRIPT_EFFECT_LIKE_CPP: u32 = 77;
const SPELL_EFFECT_UNLEARN_SPECIALIZATION_LIKE_CPP: u32 = 133;
const SPELL_EFFECT_TRIGGER_SPELL_WITH_VALUE_LIKE_CPP: u32 = 142;
const SPELL_EFFECT_TRIGGER_MISSILE_SPELL_WITH_VALUE_LIKE_CPP: u32 = 148;
const SPELL_EFFECT_TRIGGER_SPELL_2_LIKE_CPP: u32 = 151;
const SPELL_EFFECT_CREATE_LOOT_LIKE_CPP: u32 = 157;
const SPELL_EFFECT_UPGRADE_CHARACTER_SPELLS_LIKE_CPP: u32 = 215;
const SPELL_EFFECT_TRIGGER_ACTION_SET_LIKE_CPP: u32 = 226;
const TARGET_UNIT_PET_LIKE_CPP: i64 = 5;
const MAX_PLAYER_SKILLS_LIKE_CPP: usize = 256;
const CLASS_WARRIOR_LIKE_CPP: u8 = 1;
/// C++ `MAX_CLASSES` is max player-class ID + 1.
const MAX_CLASSES_LIKE_CPP: u8 = 15;
const DEFAULT_ACQUISITION_WORK_LIMIT: usize = 16_384;
const SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP: u32 = wow_data::SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP;

mod adapter;
mod authority;
mod model;
mod planner;

pub(crate) use authority::*;
pub(crate) use model::*;
#[allow(unused_imports)] // Private prerequisite seam consumed by trainer issue #157.
pub(crate) use planner::*;

#[cfg(test)]
mod tests;
