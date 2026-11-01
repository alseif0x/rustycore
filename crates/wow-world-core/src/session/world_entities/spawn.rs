use crate::phasing::{init_db_phase_shift_like_cpp, init_db_visible_map_id_like_cpp};
use wow_entities::PhaseShift;

impl crate::session::state::SessionCatalogs {
    pub fn db_spawn_phase_shift_like_cpp(
        &self,
        map_id: u16,
        phase_use_flags: u8,
        phase_id: u16,
        phase_group_id: u32,
        terrain_swap_map: i32,
    ) -> (PhaseShift, i32) {
        let mut phase_shift = PhaseShift::default();
        if let (Some(phase_store), Some(phase_group_store)) =
            (&self.phase_store, &self.phase_group_store)
        {
            init_db_phase_shift_like_cpp(
                &mut phase_shift,
                phase_store,
                phase_group_store,
                phase_use_flags,
                phase_id,
                phase_group_id,
            );
        }

        let mut validated_terrain_swap_map = -1;
        if let (Some(map_store), Some(terrain_swap_store)) =
            (&self.maps.store, &self.terrain_swap_store)
            && let Some(terrain_swap_map) = terrain_swap_store.validate_spawn_terrain_swap_like_cpp(
                map_store,
                u32::from(map_id),
                terrain_swap_map,
            )
        {
            init_db_visible_map_id_like_cpp(
                &mut phase_shift,
                terrain_swap_store,
                i32::try_from(terrain_swap_map).unwrap_or(-1),
            );
            validated_terrain_swap_map = i32::try_from(terrain_swap_map).unwrap_or(-1);
        }

        (phase_shift, validated_terrain_swap_map)
    }
}
