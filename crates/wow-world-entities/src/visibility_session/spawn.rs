// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature spawn-row materialization for the visibility paths (#1363 slice
//! 8a; moved from the World `handlers/character/{visibility,items,mod}.rs`).
//!
//! C++ `ObjectMgr::LoadCreatures` builds `CreatureData` once and
//! `Creature::LoadFromDB` consumes it (`Creature::Create`,
//! `Creature::UpdateEntry`, `Creature::LoadCreaturesAddon`,
//! `Creature::LoadEquipment`). The Rust SQL fallback reads one row at a time,
//! so the login spawn and the movement refresh share this single reader.

use tracing::warn;
use wow_constants::CreatureFlagsExtra;
use wow_constants::UnitStandStateType;
use wow_constants::movement::MovementFlag;
use wow_constants::unit::{
    SheathState, UNIT_FLAGS_ALLOWED_LIKE_CPP, UNIT_FLAGS2_ALLOWED_LIKE_CPP,
    UNIT_FLAGS3_ALLOWED_LIKE_CPP, UnitFlags,
};
use wow_core::{ObjectGuid, Position};
use wow_entities::{
    CreatureAddonLifecycleRecordLikeCpp, MovementGeneratorType,
    normalize_creature_chase_movement_type_like_cpp,
    normalize_creature_random_movement_type_like_cpp,
};
use wow_packet::packets::update::CreatureCreateData;

use super::VisibilitySessionHostLikeCpp;
use crate::{CreatureSpawnCatalogsLikeCpp, MaterializedCreatureSpawnLikeCpp};

pub const WAYPOINT_MOTION_TYPE_LIKE_CPP: u8 = 2;
// C++ ObjectDefines.h: DEFAULT_VISIBILITY_DISTANCE = VISIBILITY_DISTANCE_NORMAL = 100 yards.
// Wider values here make the SQL fallback load whole areas and can crash the 3.4.3 client.
pub(super) const DEFAULT_VISIBILITY_DISTANCE_LIKE_CPP: f32 =
    wow_entities::VISIBILITY_RADIUS_LIKE_CPP;

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

pub(super) fn is_within_2d_visibility_range_like_cpp(
    viewer: &Position,
    object_x: f32,
    object_y: f32,
    range: f32,
) -> bool {
    let dx = viewer.x - object_x;
    let dy = viewer.y - object_y;
    dx * dx + dy * dy <= range * range
}

pub(super) fn represented_go_state_from_i8_like_cpp(state: i8) -> Option<wow_entities::GoState> {
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
    has_addon: bool,
    mount_display_id: i32,
    stand_state: u8,
    vis_flags: u8,
    anim_tier: u8,
    sheathe_state: u8,
    pvp_flags: u8,
    emote_state: i32,
    ai_anim_kit_id: u16,
    movement_anim_kit_id: u16,
    melee_anim_kit_id: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CreatureEquipmentCreateFieldsLikeCpp {
    selected_equipment_id: u8,
    original_equipment_id: i8,
    virtual_items: [(i32, u16, u16); 3],
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

fn creature_addon_create_fields_like_cpp(
    addon: Option<&CreatureAddonLifecycleRecordLikeCpp>,
) -> CreatureAddonCreateFieldsLikeCpp {
    let Some(addon) = addon else {
        // C++ Creature::UpdateEntry calls SetSheath(SHEATH_STATE_MELEE)
        // when no addon row exists; addon rows then own the exact value.
        return CreatureAddonCreateFieldsLikeCpp {
            stand_state: UnitStandStateType::Stand as u8,
            sheathe_state: SheathState::Melee as u8,
            ..CreatureAddonCreateFieldsLikeCpp::default()
        };
    };

    CreatureAddonCreateFieldsLikeCpp {
        has_addon: true,
        mount_display_id: addon.mount_display_id as i32,
        stand_state: addon.stand_state as u8,
        vis_flags: addon.vis_flags,
        anim_tier: addon.anim_tier,
        sheathe_state: addon.sheath_state as u8,
        pvp_flags: addon.pvp_flags.bits(),
        emote_state: addon.emote as i32,
        ai_anim_kit_id: addon.ai_anim_kit_id,
        movement_anim_kit_id: addon.movement_anim_kit_id,
        melee_anim_kit_id: addon.melee_anim_kit_id,
    }
}

pub(super) fn materialize_creature_spawn_row_with_catalogs_like_cpp<
    H: VisibilitySessionHostLikeCpp,
>(
    host: &mut H,
    catalogs: &CreatureSpawnCatalogsLikeCpp,
    map_id: u16,
    row: &wow_persistence::CreatureVisibilityPersistenceRowLikeCpp,
    viewer_position: &Position,
    visibility_range: f32,
) -> Option<MaterializedCreatureSpawnLikeCpp> {
    // C++ ObjectMgr::LoadCreatures materializes CreatureData once, then
    // Creature::LoadFromDB consumes that data for create/update fields.
    // Rust still queries lazily by visibility; keep a single row reader so
    // login and movement refresh cannot drift from each other.
    let spawn_guid = row.spawn_guid;
    let entry = row.entry;
    let difficulty_id = host.core().current_map_difficulty_id_like_cpp();
    let [pos_x, pos_y, pos_z, orientation] = row.position;
    if !is_within_2d_visibility_range_like_cpp(viewer_position, pos_x, pos_y, visibility_range) {
        return None;
    }
    let spawn_difficulties = &row.spawn_difficulties;
    if !spawn_difficulties_contains_spawn_mode_like_cpp(spawn_difficulties, difficulty_id) {
        return None;
    }
    let type_flags = catalogs
        .difficulty
        .get_like_cpp(entry, difficulty_id)
        .type_flags;

    let cur_health = row.current_health;
    let cur_mana = row.current_mana;
    let model_id = row.model_id;
    let min_level = row.min_level;
    let faction = row.faction;
    let template_npc_flags = row.template_npc_flags;
    let [
        template_unit_flags,
        template_unit_flags2,
        template_unit_flags3,
    ] = row.template_unit_flags;
    let speed_walk = normalize_creature_template_speed_walk_like_cpp(row.speed_walk);
    let speed_run = normalize_creature_template_speed_run_like_cpp(row.speed_run);
    let scale = row.scale;
    let unit_class = row.unit_class;
    let flags_extra = row.flags_extra;
    let (npc_flags, unit_flags, unit_flags2, unit_flags3) = choose_creature_flags_like_cpp(
        template_npc_flags,
        template_unit_flags,
        template_unit_flags2,
        template_unit_flags3,
        row.spawn_npc_flags_override,
        row.spawn_unit_flags_override[0],
        row.spawn_unit_flags_override[1],
        row.spawn_unit_flags_override[2],
        flags_extra,
    );
    let classification = row.classification;
    let regen_health = row.regen_health;
    let [base_attack_time, ranged_attack_time] = row.attack_time;
    let template_display_id = row.template_display_id;
    let template_display_scale = row.template_display_scale;
    let loot_id = row.loot_id;
    let skin_loot_id = row.skin_loot_id;
    let [gold_min, gold_max] = row.gold;
    let respawn_delay_secs = row.respawn_delay_secs;
    let script_name = row.script_name.clone();
    let string_id = row.string_id.clone();
    let vehicle_id = row.vehicle_id;
    let phase_use_flags = row.phase_use_flags;
    let phase_id = row.phase_id;
    let phase_group_id = row.phase_group_id;
    let terrain_swap_map = row.terrain_swap_map;
    let ground_movement_type = row.ground_movement_type;
    let swim_allowed = row.swim_allowed;
    let flight_movement_type = row.flight_movement_type;
    let rooted = row.rooted;
    let chase_movement_type =
        normalize_creature_chase_movement_type_like_cpp(row.chase_movement_type);
    let random_movement_type =
        normalize_creature_random_movement_type_like_cpp(row.random_movement_type);
    let interaction_pause_timer_ms = row.interaction_pause_timer_ms;
    let wander_distance = row.wander_distance;
    let default_movement_type = creature_movement_generator_type_from_db_like_cpp(
        row.effective_movement_type,
        wander_distance,
    );
    let wander_distance =
        normalized_creature_wander_distance_like_cpp(default_movement_type, wander_distance);
    let waypoint_path_id = row.waypoint_path_id;

    let Some(display_selection) = host.hub_ref().catalogs.choose_creature_display_like_cpp(
        entry,
        model_id,
        flags_extra,
        template_display_id,
        template_display_scale,
    ) else {
        return None;
    };
    let display_id = display_selection.display_id;
    let Some(model_scalars) = host
        .hub_ref()
        .catalogs
        .creature_create_model_scalars_like_cpp(display_id, scale, display_selection.display_scale)
    else {
        warn!(
            "Skipping creature entry={} spawn={} display={} because creature_model_info is missing, matching C++ CreateFromProto failure",
            entry, spawn_guid, display_id
        );
        return None;
    };

    let (target_phase_shift, _) = host.hub_ref().catalogs.db_spawn_phase_shift_like_cpp(
        map_id,
        phase_use_flags,
        phase_id,
        phase_group_id,
        terrain_swap_map,
    );
    if !host.can_see_phase_shift_like_cpp(&target_phase_shift) {
        return None;
    }

    let creature_stats = {
        let (state, hub) = host.split_world_entities_ref();
        state.creature_create_stats_with_catalogs_like_cpp(
            hub,
            catalogs,
            entry,
            min_level,
            unit_class,
            classification,
            regen_health,
            cur_health,
            cur_mana,
        )
    };
    let addon = catalogs.addons.get_for_creature_like_cpp(spawn_guid, entry);
    let addon_fields = creature_addon_create_fields_like_cpp(addon.as_ref());
    let equipment_fields = creature_virtual_items_from_row_with_catalogs_like_cpp(
        host,
        catalogs,
        entry,
        row.equipment_id,
    );

    let guid = if vehicle_id != 0 {
        ObjectGuid::create_vehicle_like_cpp(
            host.core().realm_id(),
            map_id,
            entry,
            spawn_guid as i64,
        )
    } else {
        ObjectGuid::create_creature_like_cpp(
            host.core().realm_id(),
            map_id,
            entry,
            spawn_guid as i64,
        )
    };
    let movement_flags = creature_create_movement_flags_like_cpp(ground_movement_type, rooted);
    let position = creature_create_position_after_hover_offset_like_cpp(
        Position::new(pos_x, pos_y, pos_z, orientation),
        movement_flags,
        model_scalars.hover_height,
    );
    let create_data = CreatureCreateData {
        guid,
        entry,
        display_id,
        native_display_id: display_id,
        display_scale: model_scalars.display_scale,
        native_x_display_scale: model_scalars.native_x_display_scale,
        bounding_radius: model_scalars.bounding_radius,
        combat_reach: model_scalars.combat_reach,
        health: creature_stats.health,
        max_health: creature_stats.max_health,
        level: min_level,
        faction_template: faction,
        npc_flags,
        unit_flags,
        unit_flags2,
        unit_flags3,
        aura_state: wow_entities::Creature::health_aura_state_like_cpp(
            creature_stats.health.max(0) as u64,
            creature_stats.max_health.max(0) as u64,
            creature_stats.health > 0,
        ),
        damage_school: wow_constants::spell::SpellSchools::Normal as u8,
        scale,
        unit_class,
        display_power: creature_stats.power_type as u8,
        power: {
            let mut power = [0; 10];
            power[0] = creature_stats.power;
            power
        },
        max_power: {
            let mut max_power = [0; 10];
            max_power[0] = creature_stats.max_power;
            max_power
        },
        base_mana: creature_stats.base_mana,
        virtual_items: equipment_fields.virtual_items,
        base_attack_time,
        ranged_attack_time,
        movement_flags,
        vehicle_id,
        play_hover_anim: false,
        hover_height: model_scalars.hover_height,
        mount_display_id: addon_fields.mount_display_id,
        stand_state: addon_fields.stand_state,
        vis_flags: addon_fields.vis_flags,
        anim_tier: addon_fields.anim_tier,
        emote_state: addon_fields.emote_state,
        sheathe_state: addon_fields.sheathe_state,
        pvp_flags: addon_fields.pvp_flags,
        current_area_id: 0,
        speed_walk_rate: speed_walk,
        speed_run_rate: speed_run,
        ai_anim_kit_id: addon_fields.ai_anim_kit_id,
        movement_anim_kit_id: addon_fields.movement_anim_kit_id,
        melee_anim_kit_id: addon_fields.melee_anim_kit_id,
    };

    let aggro_radius = {
        let (state, hub) = host.split_world_entities_ref();
        state.creature_aggro_radius_for_faction_template_like_cpp(hub, faction.max(0) as u32, 15.0)
    };
    let min_damage = (min_level as u32).saturating_sub(1) * 3 + 5;
    let max_damage = min_damage + min_damage / 2;

    Some(MaterializedCreatureSpawnLikeCpp {
        guid,
        position,
        create_data,
        min_damage,
        max_damage,
        aggro_radius,
        loot_id,
        skin_loot_id,
        gold_min,
        gold_max,
        respawn_delay_secs,
        selected_equipment_id: equipment_fields.selected_equipment_id,
        original_equipment_id: equipment_fields.original_equipment_id,
        script_name,
        string_id,
        addon,
        phase_use_flags,
        phase_id,
        phase_group_id,
        terrain_swap_map,
        flags_extra,
        type_flags,
        ground_movement_type,
        swim_allowed,
        flight_movement_type,
        rooted,
        chase_movement_type,
        random_movement_type,
        interaction_pause_timer_ms,
        wander_distance,
        default_movement_type,
        waypoint_path_id,
    })
}

#[cfg(any(test, feature = "test-fixtures"))]
pub fn materialize_creature_spawn_row_for_test<H: VisibilitySessionHostLikeCpp>(
    host: &mut H,
    catalogs: &CreatureSpawnCatalogsLikeCpp,
    map_id: u16,
    row: &wow_persistence::CreatureVisibilityPersistenceRowLikeCpp,
    viewer_position: &Position,
    visibility_range: f32,
) -> Option<MaterializedCreatureSpawnLikeCpp> {
    materialize_creature_spawn_row_with_catalogs_like_cpp(
        host,
        catalogs,
        map_id,
        row,
        viewer_position,
        visibility_range,
    )
}

fn creature_virtual_items_from_row_with_catalogs_like_cpp<H: VisibilitySessionHostLikeCpp>(
    host: &mut H,
    catalogs: &CreatureSpawnCatalogsLikeCpp,
    entry: u32,
    persisted_equipment_id: i16,
) -> CreatureEquipmentCreateFieldsLikeCpp {
    let mut equipment_id = persisted_equipment_id;
    let original_equipment_id = i8::try_from(equipment_id).unwrap_or(0);
    if equipment_id == 0 {
        return CreatureEquipmentCreateFieldsLikeCpp {
            selected_equipment_id: 0,
            original_equipment_id: 0,
            virtual_items: [(0, 0, 0); 3],
        };
    }

    {
        let store = &catalogs.equipment;
        let equipment = if equipment_id == -1 {
            let count = store.len_for_entry(entry);
            if count == 0 {
                None
            } else {
                let index = host
                    .core_mut()
                    .represented_urand_u32_like_cpp(0, (count - 1) as u32)
                    as usize;
                store.nth_for_entry(entry, index).map(|(id, info)| {
                    equipment_id = i16::from(id);
                    info
                })
            }
        } else {
            u8::try_from(equipment_id)
                .ok()
                .and_then(|id| store.get(entry, id))
        };

        if let Some(equipment) = equipment {
            let selected_equipment_id = u8::try_from(equipment_id).unwrap_or(0);
            return CreatureEquipmentCreateFieldsLikeCpp {
                selected_equipment_id,
                original_equipment_id,
                virtual_items: equipment.items.map(|item| {
                    (
                        i32::try_from(item.item_id).unwrap_or(0),
                        item.appearance_mod_id,
                        item.item_visual,
                    )
                }),
            };
        }
    }

    CreatureEquipmentCreateFieldsLikeCpp {
        selected_equipment_id: 0,
        original_equipment_id: 0,
        virtual_items: [(0, 0, 0); 3],
    }
}
