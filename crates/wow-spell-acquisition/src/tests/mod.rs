use std::collections::{BTreeMap, BTreeSet};

use super::*;
use wow_data::skill::{SKILL_CATEGORY_ARMOR_LIKE_CPP, SKILL_CATEGORY_LANGUAGES_LIKE_CPP};
use wow_data::{
    EffectiveSpellAcquisitionRowsLikeCpp, MountEntry, SkillLineAbilityRecord, SkillTiersRowLikeCpp,
    SpellAcquisitionCoverageSeedLikeCpp, SpellAcquisitionTableHashesLikeCpp,
};

use crate::test_fixtures::*;

mod authority;
mod cast;
mod publication_boundary;
mod skill_parent;
mod skill_rewards;
mod skill_state;
mod spell;
