use std::sync::Arc;

use wow_data::character_progression::PowerTypeStore;
use wow_data::{
    CreatureAddonStoreLikeCpp, CreatureBaseStatsStoreLikeCpp,
    CreatureDifficultyStoreLikeCpp, CreatureEquipmentStoreLikeCpp,
};
use wow_world_core::map_manager::PendingRespawn;
use wow_world_core::session::{HubMut, HubRef};

use crate::{CreatureSpawnCatalogsLikeCpp, WorldEntitiesState};

impl WorldEntitiesState {
    /// Push a `PendingRespawn` into the shared map's respawn queue.
    ///
    /// The registration bootstrap still defaults to instance `0`; live removal/mutation
    /// follows the canonical Player instance. The canonical respawn store
    /// (`wow_map::RespawnStoreLikeCpp`) is a separate step.
    /// Lock is acquired, respawn is pushed, then lock is released before returning.
    /// No `.await` is performed under lock.
    pub fn push_map_respawn_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        map_id: u16,
        instance_id: u32,
        r: PendingRespawn,
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
    pub fn drain_ready_map_respawns_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        map_id: u16,
        instance_id: u32,
        now: std::time::Instant,
    ) -> Vec<PendingRespawn> {
        if let Some(manager) = &hub.core.map_manager {
            return manager
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .drain_ready_respawns(map_id, instance_id, now);
        }
        Vec::new()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn creature_spawn_catalogs_for_test_like_cpp(
        &self,
        hub: HubRef<'_>,
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

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_mount_vehicle_despawn_delay_ms_like_cpp(&self, hub: HubRef<'_>) -> i32 {
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
