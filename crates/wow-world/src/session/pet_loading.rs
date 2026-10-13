// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Pet loading row types. The pet mode state and the `PetsCx` row loaders moved
//! to `wow_world_lifecycle::pet_session` (#1363 slice 7a).

use super::{Item, ObjectGuid};

pub(crate) use wow_world_lifecycle::adjusted_represented_pet_aura_remain_time_like_cpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CharacterAuraRowLikeCpp {
    pub caster_guid: ObjectGuid,
    pub spell_id: u32,
    pub effect_mask: u32,
    pub recalculate_mask: u32,
    pub difficulty: u8,
    pub stack_count: u8,
    pub max_duration_ms: i32,
    pub remain_time_ms: i32,
    pub remain_charges: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CharacterAuraEffectRowLikeCpp {
    pub caster_guid: ObjectGuid,
    pub spell_id: u32,
    pub effect_mask: u32,
    pub effect_index: u8,
    pub amount: i32,
    pub base_amount: i32,
}
