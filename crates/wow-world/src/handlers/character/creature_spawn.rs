//! Creature spawn materialisation: template normalisation, flags, addon and equipment
//! create fields, and the create-time movement facts.
//!
//! Split out of `character/mod.rs` under #584 (B5); items are unchanged.

use super::*;

pub fn creature_movement_generator_type_from_db_like_cpp(
    db_movement_type: u8,
    wander_distance: f32,
) -> MovementGeneratorType {
    const RANDOM_MOTION_TYPE_LIKE_CPP: u8 = 1;
    match db_movement_type {
        WAYPOINT_MOTION_TYPE_LIKE_CPP => MovementGeneratorType::Waypoint,
        RANDOM_MOTION_TYPE_LIKE_CPP if wander_distance > 0.0 => MovementGeneratorType::Random,
        _ => MovementGeneratorType::Idle,
    }
}

pub fn normalized_creature_wander_distance_like_cpp(
    default_movement_type: MovementGeneratorType,
    wander_distance: f32,
) -> f32 {
    let wander_distance = wander_distance.max(0.0);
    if default_movement_type == MovementGeneratorType::Idle {
        0.0
    } else {
        wander_distance
    }
}

pub fn normalize_creature_template_speed_walk_like_cpp(speed_walk: f32) -> f32 {
    if speed_walk == 0.0 { 1.0 } else { speed_walk }
}

pub fn normalize_creature_template_speed_run_like_cpp(speed_run: f32) -> f32 {
    if speed_run == 0.0 { 1.14286 } else { speed_run }
}

pub fn spawn_difficulties_contains_spawn_mode_like_cpp(
    spawn_difficulties: &str,
    spawn_mode: u8,
) -> bool {
    // C++ ObjectMgr::ParseSpawnDifficulties parses comma-separated Difficulty
    // values and maps invalid tokens to DIFFICULTY_NONE before the map/grid
    // code filters by Map::GetSpawnMode().
    spawn_difficulties
        .split(',')
        .filter(|token| !token.is_empty())
        .map(|token| token.parse::<u8>().unwrap_or(0))
        .any(|difficulty| difficulty == spawn_mode)
}

pub fn choose_creature_flags_like_cpp(
    template_npc_flags: u64,
    template_unit_flags: u32,
    template_unit_flags2: u32,
    template_unit_flags3: u32,
    spawn_npc_flags: Option<u64>,
    spawn_unit_flags: Option<u32>,
    spawn_unit_flags2: Option<u32>,
    spawn_unit_flags3: Option<u32>,
    flags_extra: u32,
) -> (u64, u32, u32, u32) {
    // C++ ObjectMgr::ChooseCreatureFlags: spawn overrides are optional;
    // missing values fall back to creature_template.
    let npc_flags = spawn_npc_flags.unwrap_or(template_npc_flags);
    let mut unit_flags =
        spawn_unit_flags.unwrap_or(template_unit_flags) & UNIT_FLAGS_ALLOWED_LIKE_CPP;
    let unit_flags2 =
        spawn_unit_flags2.unwrap_or(template_unit_flags2) & UNIT_FLAGS2_ALLOWED_LIKE_CPP;
    let unit_flags3 =
        spawn_unit_flags3.unwrap_or(template_unit_flags3) & UNIT_FLAGS3_ALLOWED_LIKE_CPP;

    // C++ Creature::UpdateEntry clears template combat state on create and
    // only restores it when the creature is already in combat.
    unit_flags &= !UnitFlags::IN_COMBAT.bits();

    // C++ Creature::UpdateEntry calls SetUninteractible(true) for triggers
    // after selecting DB flags.
    if CreatureFlagsExtra::from_bits_truncate(flags_extra).contains(CreatureFlagsExtra::TRIGGER) {
        unit_flags |= UnitFlags::UNINTERACTIBLE.bits();
    }

    (npc_flags, unit_flags, unit_flags2, unit_flags3)
}

pub fn is_within_2d_visibility_range_like_cpp(
    viewer: &Position,
    object_x: f32,
    object_y: f32,
    range: f32,
) -> bool {
    let dx = viewer.x - object_x;
    let dy = viewer.y - object_y;
    dx * dx + dy * dy <= range * range
}

pub fn represented_go_state_from_i8_like_cpp(state: i8) -> Option<wow_entities::GoState> {
    match state {
        0 => Some(wow_entities::GoState::Active),
        1 => Some(wow_entities::GoState::Ready),
        2 => Some(wow_entities::GoState::Destroyed),
        24 => Some(wow_entities::GoState::TransportActive),
        25 => Some(wow_entities::GoState::TransportStopped),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct CreatureAddonCreateFieldsLikeCpp {
    pub has_addon: bool,
    pub mount_display_id: i32,
    pub stand_state: u8,
    pub vis_flags: u8,
    pub anim_tier: u8,
    pub sheathe_state: u8,
    pub pvp_flags: u8,
    pub emote_state: i32,
    pub ai_anim_kit_id: u16,
    pub movement_anim_kit_id: u16,
    pub melee_anim_kit_id: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureEquipmentCreateFieldsLikeCpp {
    pub selected_equipment_id: u8,
    pub original_equipment_id: i8,
    pub virtual_items: [(i32, u16, u16); 3],
}

#[derive(Debug, Clone)]
pub struct MaterializedCreatureSpawnLikeCpp {
    pub guid: ObjectGuid,
    pub position: Position,
    pub create_data: CreatureCreateData,
    pub min_damage: u32,
    pub max_damage: u32,
    pub aggro_radius: f32,
    pub loot_id: u32,
    pub skin_loot_id: u32,
    pub gold_min: u32,
    pub gold_max: u32,
    pub respawn_delay_secs: u32,
    pub selected_equipment_id: u8,
    pub original_equipment_id: i8,
    pub script_name: String,
    pub string_id: Option<String>,
    pub addon: Option<CreatureAddonLifecycleRecordLikeCpp>,
    pub phase_use_flags: u8,
    pub phase_id: u16,
    pub phase_group_id: u32,
    pub terrain_swap_map: i32,
    pub flags_extra: u32,
    pub ground_movement_type: u8,
    pub swim_allowed: bool,
    pub flight_movement_type: u8,
    pub rooted: bool,
    pub chase_movement_type: u8,
    pub random_movement_type: u8,
    pub interaction_pause_timer_ms: u32,
    pub wander_distance: f32,
    pub default_movement_type: MovementGeneratorType,
    pub waypoint_path_id: u32,
}

pub fn creature_create_movement_flags_like_cpp(ground_movement_type: u8, rooted: bool) -> u32 {
    let mut flags = MovementFlag::empty();
    if ground_movement_type == wow_constants::CreatureGroundMovementType::Hover as u8 {
        // C++ Creature::LoadCreaturesAddon calls AddUnitMovementFlag(MOVEMENTFLAG_HOVER)
        // when CanHover(), and CanHover() is true for ground movement type Hover.
        flags.insert(MovementFlag::HOVER);
    }
    if rooted {
        // C++ Creature::LoadTemplateRoot -> SetTemplateRooted -> SetControlled(... ROOT)
        // ends in Unit::SetRooted, removing moving flags before adding MOVEMENTFLAG_ROOT.
        flags.remove(MovementFlag::MASK_MOVING);
        flags.insert(MovementFlag::ROOT);
    }
    flags.bits()
}

pub fn creature_create_position_after_hover_offset_like_cpp(
    mut position: Position,
    movement_flags: u32,
    hover_height: f32,
) -> Position {
    // C++ `Creature::Create` calls `LoadCreaturesAddon()` and then
    // `m_positionZ += GetHoverOffset()`. `GetHoverOffset()` is
    // MOVEMENTFLAG_HOVER ? UnitData::HoverHeight : 0.
    if MovementFlag::from_bits_retain(movement_flags).contains(MovementFlag::HOVER) {
        position.z += hover_height;
    }
    position
}
