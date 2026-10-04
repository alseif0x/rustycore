//! Loot state and value contracts owned by the world-session application boundary.

mod contracts;
mod random_properties;
mod state;
pub mod storage_plans;

pub use contracts::{
    LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP, LOOT_SLOT_TYPE_LOCKED_LIKE_CPP,
    LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP, LootStoreRandomProperties,
    RepresentedCreatureLootStateLikeCpp, RepresentedLootRollState,
    loot_roll_broadcast_item_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use contracts::RepresentedLootRollCriteriaEvent;
pub use random_properties::loot_store_data_can_stack_with_item;
pub use state::LootState;
