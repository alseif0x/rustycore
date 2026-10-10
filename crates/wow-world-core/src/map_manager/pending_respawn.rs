//! Pending respawn items of mod.
//!
//! Separated from mod.rs under #711; every item keeps its name,
//! signature and body.

use super::*;

// #1263 F6-8D3a-3: the respawn entry types and the queue moved to the map
// crate, so the canonical map owns the same queue the legacy `MapInstance`
// owns; every legacy path keeps naming them here.
pub use wow_map::creature_respawn_queue::{
    CreatureRespawnQueueLikeCpp, LegacyRespawnTimeAddOutcomeLikeCpp, PendingRespawn,
    PersistedRespawnRowLikeCpp,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LegacyRespawnQueueReloadReportLikeCpp {
    pub rows: usize,
    pub timers_loaded: usize,
    pub creature_queued: usize,
    pub gameobject_loaded: usize,
    pub rejected_zero_spawn_id: usize,
    pub rejected_unsupported_type: usize,
    pub rejected_existing_later: usize,
    pub missing_creature_runtime: usize,
}

pub(super) fn spawn_object_type_raw_like_cpp(object_type: SpawnObjectType) -> u16 {
    u16::from(object_type as u8)
}

pub fn respawn_time_from_instant_like_cpp(respawn_at: Instant, now: Instant, now_secs: i64) -> i64 {
    let delay_secs = respawn_at
        .checked_duration_since(now)
        .map(|delay| i64::try_from(delay.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or(0);
    now_secs.saturating_add(delay_secs)
}

pub fn instant_from_respawn_time_like_cpp(
    respawn_time: i64,
    now: Instant,
    now_secs: i64,
) -> Instant {
    let delay_secs = respawn_time.saturating_sub(now_secs);
    if delay_secs <= 0 {
        return now;
    }

    let requested_delay_secs = u64::try_from(delay_secs).unwrap_or(u64::MAX);
    if let Some(deadline) = now.checked_add(Duration::from_secs(requested_delay_secs)) {
        return deadline;
    }

    // C++ uses `time_t::max()` as a never-respawn sentinel for some bosses.
    // `Instant` has a platform-specific upper bound; saturate to its farthest
    // representable future point instead of turning overflow into "ready now".
    let mut low = 0_u64;
    let mut high = requested_delay_secs;
    while low < high {
        let span = high - low;
        let midpoint = low + span / 2 + span % 2;
        if now.checked_add(Duration::from_secs(midpoint)).is_some() {
            low = midpoint;
        } else {
            high = midpoint - 1;
        }
    }
    now.checked_add(Duration::from_secs(low)).unwrap_or(now)
}

/// Persist one pending respawn's time in a map's respawn queue and return the
/// DB statement, like C++ `Map::SaveRespawnInfoDB` after `SaveRespawnTime`.
pub fn save_pending_respawn_time_on_queue_like_cpp(
    queue: &mut CreatureRespawnQueueLikeCpp,
    map_id: u16,
    instance_id: u32,
    respawn: &PendingRespawn,
    now: Instant,
    now_secs: i64,
) -> Option<RespawnPersistenceMutationLikeCpp> {
    let row = PersistedRespawnRowLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: respawn.spawn_id,
        respawn_time: respawn_time_from_instant_like_cpp(respawn.respawn_at, now, now_secs),
        map_id,
        instance_id,
    };
    match queue.add_persisted_respawn_time_like_cpp(row) {
        LegacyRespawnTimeAddOutcomeLikeCpp::Inserted
        | LegacyRespawnTimeAddOutcomeLikeCpp::ReplacedExisting => {
            Some(RespawnPersistenceMutationLikeCpp::Save {
                key: RespawnPersistenceKeyLikeCpp {
                    object_type_raw: spawn_object_type_raw_like_cpp(row.object_type),
                    spawn_id: row.spawn_id,
                    map_id: row.map_id,
                    instance_id: row.instance_id,
                },
                respawn_time: row.respawn_time,
            })
        }
        LegacyRespawnTimeAddOutcomeLikeCpp::RejectedZeroSpawnId
        | LegacyRespawnTimeAddOutcomeLikeCpp::RejectedUnsupportedType
        | LegacyRespawnTimeAddOutcomeLikeCpp::RejectedExistingSoonerOrEqual => None,
    }
}

pub fn respawn_delete_mutation_like_cpp(
    object_type: SpawnObjectType,
    spawn_id: u64,
    map_id: u16,
    instance_id: u32,
) -> RespawnPersistenceMutationLikeCpp {
    RespawnPersistenceMutationLikeCpp::Delete {
        key: RespawnPersistenceKeyLikeCpp {
            object_type_raw: spawn_object_type_raw_like_cpp(object_type),
            spawn_id,
            map_id,
            instance_id,
        },
    }
}

/// Ground-snap a freshly respawned creature, like C++ `Creature::Respawn`'s
/// `UpdateAllowedPositionZ` + `SetHomePosition` (`Creature.cpp:461`).
///
/// No-op (Z untouched) when terrain has no tile/ground under the position, so the
/// no-terrain runtime path is byte-identical to before this wiring. Flyers keep
/// their altitude (raise-only); grounded creatures sit on `ground + hover`.
pub fn snap_respawn_creature_to_ground_like_cpp(
    creature: &mut Creature,
    map_id: u16,
    terrain: &LiveTerrainHeights,
) {
    let pos = creature.unit().world().position();
    // C++ `Unit::GetHoverOffset()`: hover height only while the HOVER flag is set.
    let hover_offset = if creature
        .movement_flags_like_cpp()
        .contains(MovementFlag::HOVER)
    {
        creature.unit().data().hover_height
    } else {
        0.0
    };
    let caps = AllowedPositionZCaps {
        // Transport passengers recompute position from the transport elsewhere;
        // the legacy respawn path does not model on-transport spawns.
        on_transport: false,
        can_fly: creature.can_fly_like_cpp(),
        can_swim: creature.can_swim_like_cpp(),
        hover_offset,
    };

    // C++ `GetMapHeight` offsets the probe by `Z_OFFSET_FIND_HEIGHT` before the
    // terrain lookup.
    let probe_z = pos.z + Z_OFFSET_FIND_HEIGHT;
    let ground = terrain.static_height_like_cpp(u32::from(map_id), pos.x, pos.y, probe_z);
    let new_z = allowed_position_z_from_ground_like_cpp(true, ground, pos.z, caps);
    if new_z != pos.z {
        let snapped = Position::new(pos.x, pos.y, new_z, pos.orientation);
        creature.unit_mut().world_mut().relocate(snapped);
        creature.set_ai_home_position(snapped);
    }
}

/// Build a map-owned respawn entry from the represented runtime creature.
///
/// Mirrors the data captured by the session-local corpse despawn path; this
/// helper exists so the future global lifecycle driver and the legacy session
/// path do not drift.
pub fn pending_respawn_from_world_creature_like_cpp(
    creature: &WorldCreature,
    respawn_at: Instant,
    map_id: u16,
) -> PendingRespawn {
    pending_respawn_from_creature_like_cpp(&creature.creature, respawn_at, map_id)
}

/// The class/power projection a respawn entry carries: the one this
/// incarnation was built with, or — for an incarnation built without one — the
/// unit's own fields, exactly as `WorldCreature::create_data_from_canonical_like_cpp`
/// projects them.
fn respawn_create_projection_like_cpp(
    creature: &Creature,
) -> wow_entities::creature_create::CreatureRespawnCreateProjectionLikeCpp {
    creature
        .runtime_like_cpp()
        .respawn_create_projection_like_cpp()
        .cloned()
        .unwrap_or_else(|| {
            wow_entities::creature_create::CreatureRespawnCreateProjectionLikeCpp::from_unit_like_cpp(
                creature.unit(),
            )
        })
}

/// Build a map-owned respawn entry from one creature incarnation, on either
/// store (#1263 F6-8D3a-3: the one body; it reads only the canonical
/// creature and its runtime state).
pub fn pending_respawn_from_creature_like_cpp(
    creature: &Creature,
    respawn_at: Instant,
    map_id: u16,
) -> PendingRespawn {
    let (spell_hit_aura_source_authority, spell_cast_log_aura_source_authority) = creature
        .runtime_like_cpp()
        .respawn_aura_source_authority_like_cpp();
    let projection = respawn_create_projection_like_cpp(creature);
    let persistent_spawn = creature.spawn_id() != 0;
    let spawn_id = match creature.spawn_id() {
        0 => creature.guid().low_value().max(0) as u64,
        spawn_id => spawn_id,
    };
    PendingRespawn {
        respawn_at,
        spawn_id,
        persistent_spawn,
        home_pos: creature.home_position(),
        create_data: CreatureCreateData {
            guid: creature.guid(),
            entry: creature.entry(),
            display_id: creature.display_id(),
            native_display_id: creature.display_id(),
            display_scale: projection.display_scale,
            native_x_display_scale: projection.native_x_display_scale,
            bounding_radius: creature.unit().data().bounding_radius,
            combat_reach: creature.unit().data().combat_reach,
            health: creature.max_hp() as i64,
            max_health: creature.max_hp() as i64,
            level: creature.level(),
            faction_template: creature.faction() as i32,
            npc_flags: creature.npc_flags_mask_like_cpp(),
            unit_flags: creature.unit_flags(),
            unit_flags2: 0,
            unit_flags3: 0,
            aura_state: WorldCreature::health_aura_state_like_cpp(
                creature.max_hp() as u64,
                creature.max_hp() as u64,
                creature.max_hp() > 0,
            ),
            damage_school: creature.melee_damage_school_like_cpp(),
            scale: projection.scale,
            unit_class: projection.unit_class,
            display_power: projection.display_power,
            power: projection.power,
            max_power: projection.max_power,
            base_mana: projection.base_mana,
            virtual_items: [
                (
                    creature.unit().data().virtual_items[0].item_id,
                    creature.unit().data().virtual_items[0].item_appearance_mod_id,
                    creature.unit().data().virtual_items[0].item_visual,
                ),
                (
                    creature.unit().data().virtual_items[1].item_id,
                    creature.unit().data().virtual_items[1].item_appearance_mod_id,
                    creature.unit().data().virtual_items[1].item_visual,
                ),
                (
                    creature.unit().data().virtual_items[2].item_id,
                    creature.unit().data().virtual_items[2].item_appearance_mod_id,
                    creature.unit().data().virtual_items[2].item_visual,
                ),
            ],
            base_attack_time: 2000,
            ranged_attack_time: 0,
            movement_flags: creature.movement_flags_like_cpp().bits(),
            vehicle_id: creature.lifecycle_metadata().vehicle_id.unwrap_or(0),
            play_hover_anim: false,
            hover_height: creature.unit().data().hover_height,
            mount_display_id: creature.unit().data().mount_display_id,
            stand_state: creature.unit().data().stand_state,
            vis_flags: creature.unit().data().vis_flags,
            anim_tier: creature.unit().data().anim_tier,
            emote_state: creature.unit().emote_state_like_cpp() as i32,
            sheathe_state: creature.unit().data().sheathe_state,
            pvp_flags: creature.unit().data().pvp_flags,
            current_area_id: 0,
            speed_walk_rate: projection.speed_walk_rate,
            speed_run_rate: projection.speed_run_rate,
            ai_anim_kit_id: creature.unit().ai_anim_kit_id_like_cpp(),
            movement_anim_kit_id: creature.unit().movement_anim_kit_id_like_cpp(),
            melee_anim_kit_id: creature.unit().melee_anim_kit_id_like_cpp(),
        },
        max_hp: creature.max_hp(),
        level: creature.level(),
        min_dmg: creature.min_dmg(),
        max_dmg: creature.max_dmg(),
        combat_log_stats: creature.combat_log_stats_like_cpp(),
        spell_hit_aura_source_authority_like_cpp: spell_hit_aura_source_authority,
        spell_cast_log_aura_source_authority_like_cpp: spell_cast_log_aura_source_authority,
        aggro_radius: creature.ai_ownership().aggro_radius,
        wander_distance: creature.ai_ownership().wander_radius.max(0.0),
        flags_extra: creature.lifecycle_metadata().flags_extra,
        static_flags: creature.lifecycle_metadata().static_flags,
        ai_name: creature.lifecycle_metadata().ai_name.clone(),
        script_name: creature.lifecycle_metadata().script_name.clone(),
        string_id: creature.lifecycle_metadata().string_id.clone(),
        addon: creature.lifecycle_metadata().addon.clone(),
        ground_movement_type: creature.ground_movement_type_like_cpp(),
        swim_allowed: creature.swim_allowed_like_cpp(),
        flight_movement_type: creature.flight_movement_type_like_cpp(),
        rooted: creature.is_template_rooted_like_cpp(),
        chase_movement_type: creature.chase_movement_type_like_cpp(),
        random_movement_type: creature.random_movement_type_like_cpp(),
        interaction_pause_timer_ms: creature.interaction_pause_timer_ms_like_cpp(),
        default_movement_type: creature.default_movement_type(),
        waypoint_path_id: creature.waypoint_path_id_like_cpp(),
        npc_flags: creature.npc_flags(),
        unit_flags: creature.unit_flags(),
        map_id,
        loot_id: creature.ai_ownership().loot_id,
        skin_loot_id: creature.ai_ownership().skin_loot_id,
        gold_min: creature.ai_ownership().gold_min,
        gold_max: creature.ai_ownership().gold_max,
        respawn_delay_secs: creature
            .ai_ownership()
            .respawn_time_secs
            .min(u64::from(u32::MAX)) as u32,
        selected_equipment_id: creature.equipment_id(),
        original_equipment_id: creature.original_equipment_id(),
        boss_id: creature.ai_ownership().boss_id,
        dungeon_encounter_id: creature.ai_ownership().dungeon_encounter_id,
        phase_use_flags: creature.ai_ownership().phase_use_flags,
        phase_id: creature.ai_ownership().phase_id,
        phase_group_id: creature.ai_ownership().phase_group_id,
        terrain_swap_map: creature.ai_ownership().terrain_swap_map,
        phase_shift: creature.phase_shift().clone(),
    }
}

pub fn pending_respawn_create_position_like_cpp(respawn: &PendingRespawn) -> Position {
    let mut position = respawn.home_pos;
    let create_flags =
        wow_constants::movement::MovementFlag::from_bits_retain(respawn.create_data.movement_flags);
    let addon_sets_hover = respawn.addon.is_some()
        && respawn.ground_movement_type == wow_constants::CreatureGroundMovementType::Hover as u8;
    if create_flags.contains(wow_constants::movement::MovementFlag::HOVER) || addon_sets_hover {
        position.z += respawn.create_data.hover_height;
    }
    position
}

/// Recreate a represented world creature from a map-owned respawn entry.
///
/// This is the session-free equivalent of `WorldSession::register_world_creature`
/// for the global lifecycle driver. It intentionally uses the already-resolved
/// phase shift stored in [`PendingRespawn`].
pub fn world_creature_from_pending_respawn_like_cpp(
    respawn: &PendingRespawn,
    instance_id: u32,
) -> WorldCreature {
    WorldCreature::from_installed_canonical_like_cpp(
        creature_from_pending_respawn_like_cpp(respawn, instance_id),
        respawn.create_data.clone(),
    )
}

/// Rebuild the creature incarnation of one map-owned respawn entry.
///
/// #1263 F6-8D3a-3: the one rebuild body. The legacy bridge wraps the result
/// in its packet projection; the admitted canonical executor installs it on
/// the canonical map as is.
pub fn creature_from_pending_respawn_like_cpp(
    respawn: &PendingRespawn,
    instance_id: u32,
) -> Creature {
    let create_data = &respawn.create_data;
    let position = pending_respawn_create_position_like_cpp(respawn);
    let guid = create_data.guid;
    let entry = create_data.entry;
    let hp = create_data.health.max(1) as u32;
    let level = create_data.level;
    let display_id = create_data.display_id;
    let faction = create_data.faction_template.max(0) as u32;
    let npc_flags = create_data.npc_flags as u32;
    let npc_flags2 = (create_data.npc_flags >> 32) as u32;
    let unit_flags = create_data.unit_flags;
    let unit_flags2 = create_data.unit_flags2;
    let unit_flags3 = create_data.unit_flags3;
    let damage_school = create_data.damage_school;

    let mut creature = Creature::new(false);
    creature.set_spawn_id(if respawn.persistent_spawn {
        respawn.spawn_id
    } else {
        0
    });
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(entry);
    let _ = creature
        .unit_mut()
        .world_mut()
        .set_map(u32::from(respawn.map_id), instance_id);
    creature.unit_mut().world_mut().relocate(position);
    *creature.unit_mut().world_mut().phase_shift_mut() = respawn.phase_shift.clone();
    creature.unit_mut().set_level(level);
    creature.unit_mut().set_max_health(u64::from(hp));
    creature.unit_mut().set_health(u64::from(hp));
    creature.unit_mut().set_class(create_data.unit_class);
    let power_type = power_type_from_u8_like_cpp(create_data.display_power);
    creature.set_power_type(power_type);
    creature
        .unit_mut()
        .set_create_mana_like_cpp(create_data.base_mana);
    creature
        .unit_mut()
        .replace_create_power_arrays_like_cpp(create_data.power, create_data.max_power);
    creature.set_combat_log_stats_like_cpp(respawn.combat_log_stats);
    creature.set_ai_identity_runtime(display_id, faction, npc_flags, unit_flags);
    creature.set_npc_flags2_runtime_like_cpp(npc_flags2);
    creature.set_unit_flags2_runtime_like_cpp(unit_flags2);
    creature.set_unit_flags3_runtime_like_cpp(unit_flags3);
    creature.set_melee_damage_school_like_cpp(damage_school);
    creature
        .unit_mut()
        .set_native_display_id_like_cpp(create_data.native_display_id);
    creature.unit_mut().set_display_scales_like_cpp(
        create_data.display_scale,
        create_data.native_x_display_scale,
    );
    creature
        .unit_mut()
        .set_bounding_radius(create_data.bounding_radius);
    creature
        .unit_mut()
        .set_combat_reach(create_data.combat_reach);
    creature
        .unit_mut()
        .set_hover_height_like_cpp(create_data.hover_height);
    creature.set_flags_extra_runtime_like_cpp(respawn.flags_extra);
    creature.set_static_flags_runtime_like_cpp(respawn.static_flags);
    creature.set_ai_identity_names_runtime_like_cpp(
        respawn.ai_name.clone(),
        respawn.script_name.clone(),
    );
    creature.set_spawn_string_id_runtime_like_cpp(respawn.string_id.clone());
    creature.set_ground_movement_type_runtime_like_cpp(respawn.ground_movement_type);
    creature.set_swim_allowed_runtime_like_cpp(respawn.swim_allowed);
    creature.set_flight_movement_type_runtime_like_cpp(respawn.flight_movement_type);
    creature.set_template_rooted_like_cpp(respawn.rooted);
    creature.set_chase_movement_type_runtime_like_cpp(respawn.chase_movement_type);
    creature.set_random_movement_type_runtime_like_cpp(respawn.random_movement_type);
    creature.set_interaction_pause_timer_ms_runtime_like_cpp(respawn.interaction_pause_timer_ms);
    creature.set_default_movement_type_runtime_like_cpp(respawn.default_movement_type);
    creature.set_equipment_id_like_cpp(respawn.selected_equipment_id);
    creature.set_original_equipment_id_like_cpp(respawn.original_equipment_id);
    if respawn.waypoint_path_id != 0 {
        creature.load_path_like_cpp(respawn.waypoint_path_id);
    }
    creature.apply_creatures_addon_lifecycle_like_cpp(respawn.addon.as_ref());
    let effective_waypoint_path_id = creature.waypoint_path_id_like_cpp();
    if effective_waypoint_path_id != 0 {
        creature.load_path_like_cpp(effective_waypoint_path_id);
    }
    creature.configure_ai_runtime(
        respawn.home_pos,
        respawn.aggro_radius,
        respawn.wander_distance.max(0.0),
        30,
    );
    creature.ai_ownership_mut().respawn_time_secs = u64::from(respawn.respawn_delay_secs);
    creature.set_respawn_delay(respawn.respawn_delay_secs);
    creature.ai_ownership_mut().min_damage = respawn.min_dmg;
    creature.ai_ownership_mut().max_damage = respawn.max_dmg;
    creature.ai_ownership_mut().loot_id = respawn.loot_id;
    creature.ai_ownership_mut().skin_loot_id = respawn.skin_loot_id;
    creature.ai_ownership_mut().gold_min = respawn.gold_min;
    creature.ai_ownership_mut().gold_max = respawn.gold_max;
    creature.ai_ownership_mut().boss_id = respawn.boss_id;
    creature.ai_ownership_mut().dungeon_encounter_id = respawn.dungeon_encounter_id;
    creature.ai_ownership_mut().phase_use_flags = respawn.phase_use_flags;
    creature.ai_ownership_mut().phase_id = respawn.phase_id;
    creature.ai_ownership_mut().phase_group_id = respawn.phase_group_id;
    creature.ai_ownership_mut().terrain_swap_map = respawn.terrain_swap_map;
    // C++ `Creature::UpdateEntry` (`Creature.cpp:547-550`), reached from
    // `Map::DoRespawn` → `Creature::LoadFromDB` → `Create` → `InitEntry`: the
    // template walk/run rates, swim and flight at 1.0. #1263 F6-8D3a-3: the
    // rebuilt unit carries the rates its CREATE projection caches, so
    // `Unit::m_speed_rate` and `CreatureCreateData::speed_*_rate` agree.
    let unit = creature.unit_mut();
    unit.set_speed_rate_like_cpp(UnitMoveType::Walk, create_data.speed_walk_rate);
    unit.set_speed_rate_like_cpp(UnitMoveType::Run, create_data.speed_run_rate);
    unit.set_speed_rate_like_cpp(UnitMoveType::Swim, 1.0);
    unit.set_speed_rate_like_cpp(UnitMoveType::Flight, 1.0);
    // `SetObjectScale(GetNativeObjectScale())` (`Creature.cpp:553`), the
    // template scale the entry carries (#1263 F6-8D3b-1).
    unit.world_mut().object_mut().set_scale(create_data.scale);
    creature.clear_data_changes();

    WorldCreature::install_bridge_runtime_like_cpp(&mut creature, &respawn.create_data);
    super::respawn::restore_respawn_aura_source_authority_like_cpp(
        &mut creature,
        respawn.spell_hit_aura_source_authority_like_cpp,
        respawn.spell_cast_log_aura_source_authority_like_cpp,
    );
    creature
}
