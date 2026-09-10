// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! GameObject, area-trigger, conversation and corpse update blocks.

use super::*;

mod area_trigger;
mod conversation;
mod corpse;
mod dynamic_object;
mod game_object;
mod scene_object;

pub use area_trigger::*;
pub use conversation::*;
pub use corpse::*;
pub use dynamic_object::*;
pub use game_object::*;
pub use scene_object::*;

pub(super) use area_trigger::{
    debug_area_trigger_create_block_len_like_cpp, write_area_trigger_create_block,
    write_area_trigger_values_update_block,
};

#[cfg(test)]
pub(super) use area_trigger::VALUES_TYPE_AREA_TRIGGER;

pub(super) use conversation::{
    write_conversation_create_block, write_conversation_values_update_block,
};

#[cfg(test)]
pub(super) use conversation::VALUES_TYPE_CONVERSATION;

pub(super) use corpse::{write_corpse_create_block, write_corpse_values_update_block};

#[cfg(test)]
pub(super) use corpse::VALUES_TYPE_CORPSE;

pub(super) use dynamic_object::{
    write_dynamic_object_create_block, write_dynamic_object_values_update_block,
};

#[cfg(test)]
pub(super) use dynamic_object::VALUES_TYPE_DYNAMIC_OBJECT;

pub(super) use game_object::{
    debug_gameobject_create_values_len_like_cpp, write_game_object_values_update_block,
    write_gameobject_create_block, write_transport_create_block,
};

#[cfg(test)]
pub(super) use game_object::VALUES_TYPE_GAME_OBJECT;

pub(super) use scene_object::{
    write_scene_object_create_block, write_scene_object_values_update_block,
};

#[cfg(test)]
pub(super) use scene_object::VALUES_TYPE_SCENE_OBJECT;
