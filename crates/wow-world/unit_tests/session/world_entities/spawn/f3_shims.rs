// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn creature_spawn_catalogs_for_test_like_cpp(&self) -> CreatureSpawnCatalogsLikeCpp {
        let (state, hub) = crate::session::split_world_entities_ref(self);
        state.creature_spawn_catalogs_for_test_like_cpp(hub)
    }
    #[cfg(test)]
    pub(in crate::session) fn player_mount_vehicle_despawn_delay_ms_like_cpp(&self) -> i32 {
        let (state, hub) = crate::session::split_world_entities_ref(self);
        state.player_mount_vehicle_despawn_delay_ms_like_cpp(hub)
    }
    pub(crate) fn db_spawn_phase_shift_like_cpp(
        &self,
        map_id: u16,
        phase_use_flags: u8,
        phase_id: u16,
        phase_group_id: u32,
        terrain_swap_map: i32,
    ) -> (PhaseShift, i32) {
        self.catalogs.db_spawn_phase_shift_like_cpp(
            map_id,
            phase_use_flags,
            phase_id,
            phase_group_id,
            terrain_swap_map,
        )
    }
}
