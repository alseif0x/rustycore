// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The creature CREATE projection: the data the map/entity side owns before the
//! wire crate encodes it. Moved out of `wow-packet` so the map manager stops
//! depending on an `adapter-platform` crate.

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

/// The create-time class and power fields of one creature's CREATE projection.
///
/// #1263 F6-8D3a-3: the respawn entry carries these from the projection the
/// creature was built with, not from the live unit (whose power may have moved
/// since). The canonical runtime keeps them so the respawn entry can be built
/// without the legacy projection.
///
/// #1263 F6-8D3b-1: it also carries the template movement rates and scales C++
/// `Creature::InitEntry` re-applies on every respawn (`Creature.cpp:547-553`:
/// `SetSpeedRate(MOVE_WALK/RUN, cinfo->speed_walk/run)` and
/// `SetObjectScale(GetNativeObjectScale())`; the display scales come with the
/// model), so a respawn entry no longer hardcodes 1.0/1.14286 and 1.0.
#[derive(Debug, Clone, PartialEq)]
pub struct CreatureRespawnCreateProjectionLikeCpp {
    pub unit_class: u8,
    pub display_power: u8,
    pub power: [i32; 10],
    pub max_power: [i32; 10],
    pub base_mana: i32,
    pub speed_walk_rate: f32,
    pub speed_run_rate: f32,
    pub display_scale: f32,
    pub native_x_display_scale: f32,
    pub scale: f32,
}

impl CreatureRespawnCreateProjectionLikeCpp {
    #[must_use]
    pub const fn from_create_data_like_cpp(create_data: &CreatureCreateData) -> Self {
        Self {
            unit_class: create_data.unit_class,
            display_power: create_data.display_power,
            power: create_data.power,
            max_power: create_data.max_power,
            base_mana: create_data.base_mana,
            speed_walk_rate: create_data.speed_walk_rate,
            speed_run_rate: create_data.speed_run_rate,
            display_scale: create_data.display_scale,
            native_x_display_scale: create_data.native_x_display_scale,
            scale: create_data.scale,
        }
    }

    /// The same fields read from a unit, as
    /// `WorldCreature::create_data_from_canonical_like_cpp` projects them.
    #[must_use]
    pub fn from_unit_like_cpp(unit: &crate::Unit) -> Self {
        let data = unit.data();
        let speed_rate = unit.speed_rate();
        Self {
            unit_class: data.class_id,
            display_power: data.display_power,
            power: data.power,
            max_power: data.max_power,
            base_mana: data.base_mana,
            speed_walk_rate: speed_rate[wow_constants::UnitMoveType::Walk as usize],
            speed_run_rate: speed_rate[wow_constants::UnitMoveType::Run as usize],
            display_scale: data.display_scale,
            native_x_display_scale: data.native_display_scale,
            scale: unit.world().object().scale(),
        }
    }
}
