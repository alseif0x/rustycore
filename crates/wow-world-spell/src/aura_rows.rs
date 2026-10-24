// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The persisted character aura rows C++ `Player::_LoadAuras` reads, moved
//! down from `wow-world` (`session/pet_loading.rs`) so the login aura loader
//! in `wow-world-character` can build them (#1363 slice 8d).

use wow_core::ObjectGuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterAuraRowLikeCpp {
    pub caster_guid: ObjectGuid,
    pub item_guid: ObjectGuid,
    pub spell_id: u32,
    pub effect_mask: u32,
    pub recalculate_mask: u32,
    pub difficulty: u8,
    pub stack_count: u8,
    pub max_duration_ms: i32,
    pub remain_time_ms: i32,
    pub remain_charges: u8,
    pub cast_item_id: u32,
    pub cast_item_level: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterAuraEffectRowLikeCpp {
    pub caster_guid: ObjectGuid,
    pub item_guid: ObjectGuid,
    pub spell_id: u32,
    pub effect_mask: u32,
    pub effect_index: u8,
    pub amount: i32,
    pub base_amount: i32,
}
