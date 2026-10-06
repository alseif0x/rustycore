use std::sync::Arc;

use wow_data::AreaTableStore;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_data::{AreaTriggerDb2Store, AreaTriggerScriptStoreLikeCpp, TavernAreaTriggerStoreLikeCpp};

impl crate::session::state::SessionCatalogs {
    /// Set the DB2-backed area trigger store for this session.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_area_trigger_db2_store(&mut self, store: Arc<AreaTriggerDb2Store>) {
        self.area_trigger_db2_store = Some(store);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_area_trigger_script_store(&mut self, store: Arc<AreaTriggerScriptStoreLikeCpp>) {
        self.area_trigger_script_store = Some(store);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_tavern_area_trigger_store(&mut self, store: Arc<TavernAreaTriggerStoreLikeCpp>) {
        self.tavern_area_trigger_store = Some(store);
    }
}

impl crate::session::HubRef<'_> {
    pub fn player_zone_area_like_cpp(&self) -> Option<(u32, u32)> {
        self.player_world_local_state_like_cpp()
            .map(|state| state.zone_area_like_cpp())
    }
}

impl crate::session::HubMut<'_> {
    pub fn set_player_zone_area_like_cpp(&mut self, zone_id: u32, area_id: u32) {
        let changed = self
            .shared()
            .player_zone_area_like_cpp()
            .is_some_and(|current| current != (zone_id, area_id));
        if changed {
            self.core
                .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        }
        let _ = self.set_player_world_local_zone_area_like_cpp(zone_id, area_id);
        let _ = self.core.with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .world_mut()
                .set_zone_and_area(zone_id, area_id);
        });
    }
}

impl crate::session::state::SessionCatalogs {
    pub fn area_table_store(&self) -> Option<&Arc<AreaTableStore>> {
        self.area_table_store.as_ref()
    }
}
