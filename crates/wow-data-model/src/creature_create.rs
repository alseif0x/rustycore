// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Shared creature CREATE projection schema. Entity/World callers construct it;
//! wow-packet owns encoding, and the canonical Creature retains gameplay authority.
//!
//! Historical field comments retain earlier client-crash reports. This relocation
//! adds no new capture or parity evidence for those reports.

use wow_core::ObjectGuid;

/// Data needed to build a creature create packet for the client.
#[derive(Debug, Clone)]
pub struct CreatureCreateData {
    pub guid: ObjectGuid,
    pub entry: u32,
    pub display_id: u32,
    pub native_display_id: u32,
    pub display_scale: f32,
    pub native_x_display_scale: f32,
    pub bounding_radius: f32,
    pub combat_reach: f32,
    pub health: i64,
    pub max_health: i64,
    pub level: u8,
    pub faction_template: i32,
    pub npc_flags: u64,
    pub unit_flags: u32,
    pub unit_flags2: u32,
    pub unit_flags3: u32,
    /// C++ `UNIT_FIELD_AURASTATE`. Derived from health in `Unit::Update` ->
    /// `ModifyAuraState` (Unit.cpp:469-476). A full-HP alive creature carries
    /// `0x00D00000` (WOUND_HEALTH_20_80 | HEALTHY_75_PERCENT | WOUND_HEALTH_35_80).
    /// The 3.4.3 client tests bit 0x100000 of this field on a per-frame unit tick;
    /// shipping 0 where the bit should be set crashes the client (ERROR #132).
    pub aura_state: u32,
    pub damage_school: u8,
    pub scale: f32,
    pub unit_class: u8,
    pub display_power: u8,
    pub power: [i32; 10],
    pub max_power: [i32; 10],
    pub base_mana: i32,
    pub virtual_items: [(i32, u16, u16); 3],
    pub base_attack_time: u32,
    pub ranged_attack_time: u32,
    pub movement_flags: u32,
    pub vehicle_id: u32,
    pub play_hover_anim: bool,
    pub hover_height: f32,
    pub mount_display_id: i32,
    pub stand_state: u8,
    pub vis_flags: u8,
    pub anim_tier: u8,
    pub emote_state: i32,
    pub sheathe_state: u8,
    pub pvp_flags: u8,
    pub current_area_id: u32,
    /// Speed rate from creature_template.speed_walk (1.0 = default).
    pub speed_walk_rate: f32,
    /// Speed rate from creature_template.speed_run (1.14286 = default).
    pub speed_run_rate: f32,
    pub ai_anim_kit_id: u16,
    pub movement_anim_kit_id: u16,
    pub melee_anim_kit_id: u16,
}
