//! Map key resolution used by the represented Session.
//!
//! Moved out of the Session root under #613. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use std::sync::Arc;

use crate::session::MMapRuntimeConfigLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_data::AdventureMapPoiStore;
use wow_data::{DungeonEncounterStore, MapStore};

impl crate::session::state::SessionCatalogs {
    /// Set the C++ AdventureMapPOI.db2 store for this session.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_adventure_map_poi_store(&mut self, store: Arc<AdventureMapPoiStore>) {
        self.adventure_map_poi_store = Some(store);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn adventure_map_poi_store(&self) -> Option<&Arc<AdventureMapPoiStore>> {
        self.adventure_map_poi_store.as_ref()
    }

    pub fn dungeon_encounter_store(&self) -> Option<&Arc<DungeonEncounterStore>> {
        self.dungeon_encounter_store.as_ref()
    }

    pub fn map_store(&self) -> Option<&Arc<MapStore>> {
        self.maps.store.as_ref()
    }
}

impl crate::session::state::SessionWorldConfig {
    pub fn player_map_visibility_range_like_cpp(&self, map_id: u16) -> f32 {
        self.legacy_creature_aggro_config_like_cpp
            .map_visibility_range_like_cpp(map_id)
    }

    pub fn mmap_runtime_config_like_cpp(&self) -> &MMapRuntimeConfigLikeCpp {
        &self.mmap_runtime_config_like_cpp
    }
}

impl crate::session::state::SessionCore {
    pub fn player_map_id_like_cpp(&self) -> u16 {
        self.current_map_id
    }

    /// The legacy map facade must follow the same map instance that owns the
    /// canonical Player. Instance `0` remains only the bootstrap fallback for
    /// tests/runtime phases where no canonical Player has been materialized.
    pub fn current_legacy_runtime_map_key_like_cpp(&self) -> (u16, u32) {
        let fallback_map_id = self.player_map_id_like_cpp();
        let Some(map_key) = self.current_canonical_player_map_key_like_cpp() else {
            return (fallback_map_id, 0);
        };
        let Ok(map_id) = u16::try_from(map_key.map_id) else {
            return (fallback_map_id, 0);
        };
        (map_id, map_key.instance_id)
    }
}
