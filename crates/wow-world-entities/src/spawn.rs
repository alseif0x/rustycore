use std::sync::Arc;

use wow_data::character_progression::PowerTypeStore;
use wow_data::{
    CreatureAddonStoreLikeCpp, CreatureBaseStatsStoreLikeCpp, CreatureDifficultyStoreLikeCpp,
    CreatureEquipmentStoreLikeCpp,
};
use wow_world_core::session::{HubMut, HubRef};

use crate::{CreatureSpawnCatalogsLikeCpp, WorldEntitiesState};

impl WorldEntitiesState {
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
            localized_names: hub
                .catalogs
                .object_mgr_catalogs_like_cpp
                .as_ref()
                .map(|catalogs| Arc::clone(&catalogs.creature))
                .unwrap_or_else(|| Arc::new(wow_data::CreatureQueryCatalogLikeCpp::default())),
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
