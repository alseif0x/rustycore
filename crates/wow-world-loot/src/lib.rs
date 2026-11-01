//! Loot state and value contracts owned by the world-session application boundary.

mod contracts;
mod random_properties;
mod response;
mod roll_values;
mod state;
pub mod storage_plans;

#[cfg(any(test, feature = "test-fixtures"))]
pub use contracts::RepresentedLootRollCriteriaEvent;
pub use contracts::{
    LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP, LOOT_SLOT_TYPE_LOCKED_LIKE_CPP,
    LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP, LootStoreRandomProperties,
    RepresentedCreatureLootStateLikeCpp, RepresentedLootRollState,
    loot_roll_broadcast_item_like_cpp,
};
pub use random_properties::loot_store_data_can_stack_with_item;
pub use response::{loot_type_for_client_like_cpp, represented_loot_response_items_like_cpp};
pub use roll_values::{
    LOOT_ROLL_TIMEOUT_MS_LIKE_CPP, connected_roll_looters_like_cpp, loot_item_context,
    represented_max_enchanting_skill_like_cpp, start_loot_roll_packet_like_cpp,
};
pub use state::LootState;
