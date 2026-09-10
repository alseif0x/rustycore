// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Unit and creature update blocks.

use super::*;

mod channels_and_pets;
mod create;
mod values_delta;
mod values_update;

pub use channels_and_pets::*;
pub use create::*;
pub use values_delta::*;

pub(super) use channels_and_pets::write_artifact_power_values_update;

pub(super) use create::{
    debug_creature_create_values_len_like_cpp, write_creature_create_block,
    write_stationary_world_object_create_prefix_like_cpp,
};

pub(super) use values_update::{
    VALUES_TYPE_UNIT, health_aura_state_like_cpp, power_type_for_class,
    write_creature_health_update_block, write_full_unit_values_update_block,
    write_unit_data_values_update, write_unit_data_values_update_section,
};

use channels_and_pets::write_unit_channel_values_update;

use values_delta::unit_mask_has;
