//! Represented spawn handling for observed world entities.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// Push a `PendingRespawn` into the shared map's respawn queue.
    ///
    /// The registration bootstrap still defaults to instance `0`; live removal/mutation
    /// follows the canonical Player instance. The canonical respawn store
    /// (`wow_map::RespawnStoreLikeCpp`) is a separate step.
    /// Lock is acquired, respawn is pushed, then lock is released before returning.
    /// No `.await` is performed under lock.
    pub(crate) fn push_map_respawn_like_cpp(
        &mut self,
        map_id: u16,
        instance_id: u32,
        r: crate::map_manager::PendingRespawn,
    ) {
        if let Some(manager) = &self.map_manager {
            manager
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push_respawn(map_id, instance_id, r);
        }
    }
    /// Drain ready respawns from the shared map's respawn queue for this (map_id, instance_id).
    ///
    /// Lock is acquired, ready entries are drained, then lock is released before returning.
    /// Packet building and `register_world_creature` calls MUST happen after this returns
    /// (never under lock). No `.await` is performed under lock.
    pub(crate) fn drain_ready_map_respawns_like_cpp(
        &mut self,
        map_id: u16,
        instance_id: u32,
        now: std::time::Instant,
    ) -> Vec<crate::map_manager::PendingRespawn> {
        if let Some(manager) = &self.map_manager {
            return manager
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .drain_ready_respawns(map_id, instance_id, now);
        }
        Vec::new()
    }
    #[cfg(test)]
    pub(crate) fn creature_spawn_catalogs_for_test_like_cpp(&self) -> CreatureSpawnCatalogsLikeCpp {
        CreatureSpawnCatalogsLikeCpp {
            difficulty: self
                .creature_difficulty_store_like_cpp
                .clone()
                .unwrap_or_else(|| Arc::new(CreatureDifficultyStoreLikeCpp::default())),
            base_stats: self
                .creature_base_stats_store_like_cpp
                .clone()
                .unwrap_or_else(|| Arc::new(CreatureBaseStatsStoreLikeCpp::default())),
            health_rates: self.creature_health_rates_like_cpp,
            addons: self
                .creature_addon_store_like_cpp
                .clone()
                .unwrap_or_else(|| Arc::new(CreatureAddonStoreLikeCpp::default())),
            equipment: self
                .creature_equipment_store_like_cpp
                .clone()
                .unwrap_or_else(|| Arc::new(CreatureEquipmentStoreLikeCpp::default())),
            power_types: self
                .power_type_store
                .clone()
                .unwrap_or_else(|| Arc::new(PowerTypeStore::from_entries([]))),
        }
    }
    pub(crate) fn db_spawn_phase_shift_like_cpp(
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
            (&self.map_store, &self.terrain_swap_store)
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
    #[cfg(test)]
    pub(in crate::session) fn player_mount_vehicle_despawn_delay_ms_like_cpp(&self) -> i32 {
        let Some(Some(vehicle_kit)) = self.player_mount_vehicle_kit_snapshot_like_cpp() else {
            return 1;
        };

        self.vehicle_template_store
            .as_ref()
            .map(|store| store.despawn_delay_ms_like_cpp(vehicle_kit.creature_entry()))
            .unwrap_or(1)
    }
    pub(in crate::session) fn despawn_represented_linked_trap_by_guid_like_cpp(
        &mut self,
        trap_guid: ObjectGuid,
    ) {
        if trap_guid.is_empty() || !trap_guid.is_game_object() {
            return;
        }
        if let Some(state) = self.represented_gameobject_use_states.get_mut(&trap_guid) {
            state.loot_state = Some(wow_entities::LootState::NotReady);
            state.loot_state_unit_guid = ObjectGuid::EMPTY;
            if state.go_type.map(u32::from) != Some(wow_entities::GAMEOBJECT_TYPE_TRANSPORT) {
                state.go_state = Some(wow_entities::GoState::Ready);
            }
        }
        self.client_visible_guids_like_cpp.remove(&trap_guid);
        self.loot_table.remove(&trap_guid);
        self.send_represented_gameobject_delete_packets_like_cpp(trap_guid);
    }
}
