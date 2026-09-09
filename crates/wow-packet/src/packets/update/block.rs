// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Shared update-block framing, masks and value writers.

use super::*;

mod ops_1;
mod ops_2;
mod state_1;
mod state_2;
#[allow(unused_imports)]
pub use ops_1::*;
#[allow(unused_imports)]
pub use ops_2::*;
#[allow(unused_imports)]
pub use state_1::*;
pub(in crate::packets::update) use state_1::{
    VALUES_TYPE_OBJECT, dynamic_mask_has_index, field_blocks_have, field_mask_has,
    write_arena_cooldown_values_update, write_changed_i32_dynamic_values,
    write_chr_customization_choice_values_update, write_dungeon_score_summary_values_update,
    write_dynamic_field_update_mask, write_dynamic_field_update_mask_bits,
    write_object_data_values_update_section, write_object_values_update_block,
    write_passive_spell_history_values_update, write_scale_curve_values_create,
    write_scale_curve_values_update, write_update_field_blocks_mask,
    write_update_field_blocks_mask_u32, write_visual_anim_values_create,
    write_visual_anim_values_update,
};
#[allow(unused_imports)]
pub use state_2::*;

// ── MovementBlock ───────────────────────────────────────────────────

// ── UpdateObject (SMSG_UPDATE_OBJECT) ───────────────────────────────
