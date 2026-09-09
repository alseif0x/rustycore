// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! Runtime side of C++ `ConditionMgr` evaluation context.

use std::sync::{Arc, OnceLock};

use num_traits::FromPrimitive;
use parking_lot::RwLock;
use wow_constants::MAX_CONDITION_TARGETS;
use wow_constants::{
    ComparisonType, ConditionInstanceInfo, ConditionSourceType, ConditionType, RelationType,
    TypeId, TypeMask, UnitStandStateType,
};
use wow_data::{
    Condition, ConditionEntriesByTypeStore, ConditionId, NpcSpellClickStoreLikeCpp,
    PlayerConditionContextLikeCpp, PlayerConditionStore, SPELL_CLICK_USER_FRIEND_LIKE_CPP,
    SPELL_CLICK_USER_PARTY_LIKE_CPP, SPELL_CLICK_USER_RAID_LIKE_CPP, SpellClickInfoLikeCpp,
    UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP, is_player_meeting_condition_like_cpp,
};
use wow_entities::WorldObject;
use wow_loot::{LootStoreItemContext, condition_source_type_for_loot_store_kind_like_cpp};

mod state_1;
mod state_2;
mod state_3;
#[allow(unused_imports)]
pub use state_1::*;
#[allow(unused_imports)]
pub use state_2::*;
#[allow(unused_imports)]
pub use state_3::*;

#[cfg(test)]
#[path = "conditions/tests/mod.rs"]
mod tests;
