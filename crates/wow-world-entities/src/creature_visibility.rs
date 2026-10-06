use wow_constants::NPCFlags1;
use wow_core::ObjectGuid;
use wow_world_core::session::{HubMut, HubRef, RepresentedCreatureAccessLikeCpp};

use crate::{MaterializedCreatureSpawnLikeCpp, WorldEntitiesState};

const MAX_AREA_SPIRIT_HEALER_RANGE_LIKE_CPP: f32 = 20.0;

impl WorldEntitiesState {
    pub fn register_materialized_creature_spawn_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        map_id: u16,
        spawn: &MaterializedCreatureSpawnLikeCpp,
    ) {
        self.register_world_creature_with_flags_extra_movement_and_default_motion_like_cpp(
            hub,
            map_id,
            spawn.position,
            spawn.create_data.clone(),
            spawn.min_damage,
            spawn.max_damage,
            spawn.aggro_radius,
            spawn.loot_id,
            spawn.skin_loot_id,
            spawn.gold_min,
            spawn.gold_max,
            spawn.respawn_delay_secs,
            spawn.selected_equipment_id,
            spawn.original_equipment_id,
            spawn.script_name.clone(),
            spawn.string_id.clone(),
            spawn.addon.clone(),
            None,
            0,
            spawn.phase_use_flags,
            spawn.phase_id,
            spawn.phase_group_id,
            spawn.terrain_swap_map,
            spawn.flags_extra,
            spawn.ground_movement_type,
            spawn.swim_allowed,
            spawn.flight_movement_type,
            spawn.rooted,
            spawn.chase_movement_type,
            spawn.random_movement_type,
            spawn.interaction_pause_timer_ms,
            spawn.wander_distance,
            spawn.default_movement_type,
            spawn.waypoint_path_id,
        );
    }
}

impl WorldEntitiesState {
    /// Shared C++ area-spirit-healer checks: creature exists, has the area
    /// spirit-healer flag, and is within MAX_AREA_SPIRIT_HEALER_RANGE.
    pub fn represented_area_spirit_healer_access_like_cpp(
        &self,
        hub: HubRef<'_>,
        healer_guid: ObjectGuid,
    ) -> Option<RepresentedCreatureAccessLikeCpp> {
        let access = self.canonical_creature_access_like_cpp(hub, healer_guid)?;
        if (access.npc_flags & NPCFlags1::AREA_SPIRIT_HEALER.bits()) == 0 {
            return None;
        }

        let player_position = hub.player_position_like_cpp()?;
        access
            .position
            .is_within_dist(&player_position, MAX_AREA_SPIRIT_HEALER_RANGE_LIKE_CPP)
            .then_some(access)
    }
}
