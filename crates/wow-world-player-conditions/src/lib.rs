// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player-condition projection, inventory valuation and the trainer admission
//! proof shared by the quest, loot, trainer, aura and inventory operations
//! (#1363 slice Q: moved down from `wow-world-quest`, which keeps only quest).
//!
//! `PlayerConditionProjectionCxLikeCpp` is extended by inherent impls in
//! `inventory_valuation`, so the projection and the valuation live in one
//! crate. The crate sits below `wow-world-quest` and above the domain crates
//! it reads (inventory, social, instances, spell).

mod trainer_admission;
pub use trainer_admission::{
    TrainerAdmissionProofLikeCpp, trainer_condition_admission_proof_like_cpp,
};
mod inventory_valuation;
mod player_conditions;
pub use inventory_valuation::{
    can_equip_inventory_item_like_cpp, can_equip_unique_item_like_cpp,
    can_use_inventory_item_represented_with_loading_like_cpp,
};
pub use player_conditions::{
    PlayerConditionProjectionCxLikeCpp, PlayerConditionProjectionInputsLikeCpp,
    RepresentedPlayerConditionContextLikeCpp, meets_player_condition_id_like_cpp,
    player_condition_projection_cx_like_cpp,
};
