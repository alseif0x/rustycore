// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Cross-domain instance-lock and difficulty packet operations.

mod context;
mod difficulty;
mod lock_operations;
mod raid_info;
mod registration;

pub use context::{
    InstanceDifficultyHandlerCxLikeCpp, InstanceLockOperationsHandlerCxLikeCpp,
    InstanceRaidInfoHandlerCxLikeCpp, InstancesHandlerHostLikeCpp,
};
pub use difficulty::{
    handle_set_difficulty_id_like_cpp, handle_set_dungeon_difficulty_like_cpp,
    handle_set_raid_difficulty_like_cpp, handle_toggle_difficulty_like_cpp,
};
pub use lock_operations::{
    InstanceLockResponseOutcomeLikeCpp, InstanceResetMethodLikeCpp,
    handle_instance_lock_response_like_cpp, handle_reset_instances_like_cpp,
    handle_set_saved_instance_extend_like_cpp, reset_represented_instances_like_cpp,
};
pub use raid_info::handle_request_raid_info_like_cpp;
pub use registration::register_instance_handlers_like_cpp;
