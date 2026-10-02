//! Represented spawn handling for observed world entities.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn drain_ready_map_respawns_like_cpp(
        &mut self,
        map_id: u16,
        instance_id: u32,
        now: std::time::Instant,
    ) -> Vec<crate::map_manager::PendingRespawn> {
        let (state, mut hub) = crate::session::split_world_entities_mut(self);
        state.drain_ready_map_respawns_like_cpp(&mut hub, map_id, instance_id, now)
    }
    pub(in crate::session) fn despawn_represented_linked_trap_by_guid_like_cpp(
        &mut self,
        trap_guid: ObjectGuid,
    ) {
        if trap_guid.is_empty() || !trap_guid.is_game_object() {
            return;
        }
        if let Some(state) = self
            .world_entities
            .represented_gameobject_use_states
            .get_mut(&trap_guid)
        {
            state.loot_state = Some(wow_entities::LootState::NotReady);
            state.loot_state_unit_guid = ObjectGuid::EMPTY;
            if state.go_type.map(u32::from) != Some(wow_entities::GAMEOBJECT_TYPE_TRANSPORT) {
                state.go_state = Some(wow_entities::GoState::Ready);
            }
        }
        self.core.client_visible_guids_like_cpp.remove(&trap_guid);
        self.loot.loot_table.remove(&trap_guid);
        self.send_represented_gameobject_delete_packets_like_cpp(trap_guid);
    }
}

impl crate::session::state::WorldEntitiesState {
    /// Push a `PendingRespawn` into the shared map's respawn queue.
    ///
    /// The registration bootstrap still defaults to instance `0`; live removal/mutation
    /// follows the canonical Player instance. The canonical respawn store
    /// (`wow_map::RespawnStoreLikeCpp`) is a separate step.
    /// Lock is acquired, respawn is pushed, then lock is released before returning.
    /// No `.await` is performed under lock.
    pub(crate) fn push_map_respawn_like_cpp(
        &mut self,
        hub: &mut crate::session::HubMut<'_>,
        map_id: u16,
        instance_id: u32,
        r: crate::map_manager::PendingRespawn,
    ) {
        if let Some(manager) = &hub.core.map_manager {
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
        hub: &mut crate::session::HubMut<'_>,
        map_id: u16,
        instance_id: u32,
        now: std::time::Instant,
    ) -> Vec<crate::map_manager::PendingRespawn> {
        if let Some(manager) = &hub.core.map_manager {
            return manager
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .drain_ready_respawns(map_id, instance_id, now);
        }
        Vec::new()
    }

    #[cfg(test)]
    pub(crate) fn creature_spawn_catalogs_for_test_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
    ) -> CreatureSpawnCatalogsLikeCpp {
        CreatureSpawnCatalogsLikeCpp {
            difficulty: hub
                .catalogs
                .creature_difficulty_store_like_cpp
                .clone()
                .unwrap_or_else(|| Arc::new(CreatureDifficultyStoreLikeCpp::default())),
            base_stats: hub
                .catalogs
                .creature_base_stats_store_like_cpp
                .clone()
                .unwrap_or_else(|| Arc::new(CreatureBaseStatsStoreLikeCpp::default())),
            health_rates: hub.config.creature_health_rates_like_cpp,
            addons: hub
                .catalogs
                .creature_addon_store_like_cpp
                .clone()
                .unwrap_or_else(|| Arc::new(CreatureAddonStoreLikeCpp::default())),
            equipment: hub
                .catalogs
                .creature_equipment_store_like_cpp
                .clone()
                .unwrap_or_else(|| Arc::new(CreatureEquipmentStoreLikeCpp::default())),
            power_types: hub
                .catalogs
                .power_type_store
                .clone()
                .unwrap_or_else(|| Arc::new(PowerTypeStore::from_entries([]))),
        }
    }

    #[cfg(test)]
    pub(in crate::session) fn player_mount_vehicle_despawn_delay_ms_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
    ) -> i32 {
        let Some(Some(vehicle_kit)) = hub.player_mount_vehicle_kit_snapshot_like_cpp() else {
            return 1;
        };

        hub.catalogs
            .vehicle_template_store
            .as_ref()
            .map(|store| store.despawn_delay_ms_like_cpp(vehicle_kit.creature_entry()))
            .unwrap_or(1)
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/world_entities/spawn/f3_shims.rs"]
mod f3_shims;
