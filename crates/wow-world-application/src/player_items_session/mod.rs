// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player-item operations moved from the World `session/player_items/**` and
//! `handlers/character/items/host.rs` (#1363 slice 12b1b).
//!
//! C++ anchors stay on each body. The context builders and the operations that
//! run them take the World session's disjoint fields through
//! [`crate::TrainerSessionHostLikeCpp::split_trainer_mut`] (the general field
//! split slice 12a introduced; the same borrows the World bodies took), and
//! the former World `cfg!(test)` argument is
//! [`crate::TrainerSessionHostLikeCpp::world_test_consumer_like_cpp`]. The
//! planning functions read the session's condition projection through
//! [`wow_world_quest::QuestSessionHostLikeCpp::player_condition_projection_cx_like_cpp`],
//! the same World construction the bodies called before the move.

mod builders;
mod committed;
mod planning;

pub use builders::{
    apply_inventory_item_remove_side_effects_like_cpp,
    apply_inventory_item_store_side_effects_like_cpp,
    execute_inventory_equip_to_empty_raw_like_cpp, publish_inventory_position_changes_like_cpp,
};
pub use committed::{
    apply_committed_inventory_item_relocation_like_cpp,
    apply_committed_inventory_item_removal_like_cpp, apply_committed_inventory_item_swap_like_cpp,
};
pub use planning::{
    plan_bank_existing_inventory_item_at_like_cpp, plan_equip_existing_inventory_item_like_cpp,
    plan_store_existing_inventory_item_at_like_cpp, plan_store_new_direct_inventory_item,
    plan_store_new_direct_inventory_item_at,
    plan_store_new_direct_inventory_item_with_overlays_like_cpp,
    validate_inventory_swap_target_like_cpp,
};
