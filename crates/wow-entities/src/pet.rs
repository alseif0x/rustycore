use std::collections::{BTreeMap, BTreeSet};

use wow_constants::{Class, CreatureType, DeathState, PowerType, UnitFlags};
use wow_core::ObjectGuid;

use crate::{
    ACT_DISABLED_LIKE_CPP, ACT_ENABLED_LIKE_CPP, ACT_PASSIVE_LIKE_CPP, CharmInfoState, Creature,
    CreatureRuntimePlan, MAX_UNIT_ACTION_BAR_INDEX, ReactState, UNIT_MASK_CONTROLABLE_GUARDIAN,
    UNIT_MASK_GUARDIAN, UNIT_MASK_HUNTER_PET, UNIT_MASK_MINION, UNIT_MASK_PET, UNIT_MASK_SUMMON,
    UnitAddToWorldOutcomeLikeCpp, UnitRemoveFromWorldOutcomeLikeCpp,
    make_unit_action_button_like_cpp, unit_action_button_action_like_cpp,
    unit_action_button_type_like_cpp,
};

mod ops_1;
mod ops_2;
mod state;
#[allow(unused_imports)]
pub use ops_1::*;
#[allow(unused_imports)]
pub use ops_2::*;
#[allow(unused_imports)]
pub use state::*;

#[cfg(test)]
#[path = "pet/tests/mod.rs"]
mod tests;
