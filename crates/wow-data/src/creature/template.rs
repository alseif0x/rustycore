use std::collections::HashMap;

use rand::Rng;
use wow_constants::{
    CreatureChaseMovementType, CreatureFlagsExtra, CreatureFlightMovementType,
    CreatureGroundMovementType, CreatureRandomMovementType, SheathState, UnitPvpFlags,
    UnitStandStateType,
};
use wow_entities::{
    CreatureAddonAuraApplicationLikeCpp, CreatureAddonLifecycleRecordLikeCpp,
    VisibilityDistanceTypeLikeCpp,
};

use crate::creature::model_info::CreatureModelInfoStoreLikeCpp;
use crate::{
    AnimKitStore, CreatureDisplayInfoStore, EmotesStore, SpellDurationStore, SpellMiscStore,
    SpellStore, spell::aura_types, spell_duration_ms_like_cpp,
};

mod state_1;
mod state_2;
#[allow(unused_imports)]
pub use state_1::*;
#[allow(unused_imports)]
pub use state_2::*;

#[cfg(test)]
#[path = "template/tests/mod.rs"]
pub(crate) mod tests;
