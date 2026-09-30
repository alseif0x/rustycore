use std::collections::HashMap;

use crate::creature::model_info::CreatureModelInfoStoreLikeCpp;
use crate::{
    AnimKitStore, CreatureDisplayInfoStore, EmotesStore, SpellDurationStore, SpellMiscStore,
    SpellStore, spell::aura_types, spell_duration_ms_like_cpp,
};
use rand::Rng;
use wow_constants::{
    CreatureChaseMovementType, CreatureFlagsExtra, CreatureFlightMovementType,
    CreatureGroundMovementType, CreatureRandomMovementType, SheathState, UnitPvpFlags,
    UnitStandStateType,
};
pub use wow_data_model::creature::{
    CreatureAddonAuraApplicationLikeCpp, CreatureAddonAuraEffectLikeCpp,
    CreatureAddonLifecycleRecordLikeCpp, DEFAULT_VISIBILITY_DISTANCE, MAX_VISIBILITY_DISTANCE,
    VISIBILITY_DISTANCE_GIGANTIC, VISIBILITY_DISTANCE_LARGE, VISIBILITY_DISTANCE_SMALL,
    VISIBILITY_DISTANCE_TINY, VisibilityDistanceTypeLikeCpp,
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
