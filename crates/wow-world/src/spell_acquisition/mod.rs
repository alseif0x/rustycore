// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World adapters and application of player spell/skill acquisition plans.
//!
//! `wow-spell-acquisition` owns deterministic planning, models and evidence
//! authorities. This facade keeps existing callers stable; Session snapshot
//! resolution, persistence, runtime installation and publication stay here.

use std::collections::{BTreeMap, BTreeSet};

use wow_data::{
    SpellAcquisitionCatalogLikeCpp, SpellAcquisitionEffectLikeCpp, SpellAcquisitionMiscLikeCpp,
    SpellAcquisitionResolvedEffectsLookupLikeCpp, SpellAcquisitionResolvedMetadataLookupLikeCpp,
    SpellChainLookupLikeCpp, SpellChainStoreLikeCpp, SpellLearnSkillNodeLikeCpp,
    SpellLinkedTypeLikeCpp, SpellRequiredStoreLikeCpp,
};

use wow_data::skill::{SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP, SKILL_RIDING_LIKE_CPP};
use wow_data::spell::spell_effect_types::{
    SPELL_EFFECT_DUAL_WIELD, SPELL_EFFECT_LEARN_SPELL, SPELL_EFFECT_SKILL, SPELL_EFFECT_SKILL_STEP,
};
use wow_data::trait_tree::TraitDefinitionStore;

mod adapter;
mod application;
mod effect_learning;
mod runtime_adapter;
mod trainer_purchase;

pub(crate) use application::*;
pub(crate) use effect_learning::*;
pub(crate) use trainer_purchase::*;
pub(crate) use wow_spell_acquisition::*;

#[cfg(test)]
mod tests;
