use std::sync::Arc;

use crate::session::state::SessionCore;
use wow_core::ObjectGuid;
use wow_entities::WorldObject;

impl crate::session::state::SessionCatalogs {
    pub fn object_id_visibility_conditions_met_like_cpp(
        &self,
        target: &WorldObject,
        seer: &WorldObject,
    ) -> bool {
        let Some(condition_store) = self.condition_store.as_ref() else {
            return true;
        };

        let area_table_store = self.area_table_store.as_ref().map(Arc::clone);
        wow_conditions::is_object_meeting_visibility_by_object_id_conditions_like_cpp(
            condition_store,
            target.object().type_id() as u32,
            target.object().entry(),
            Some(seer),
            |condition, source_info| {
                wow_conditions::condition_meets_basic_like_cpp(
                    condition,
                    source_info,
                    |area_id, required_area_id| {
                        area_table_store.as_ref().is_some_and(|store| {
                            store.is_in_area_like_cpp(area_id, required_area_id)
                        })
                    },
                )
                .value()
                .unwrap_or(false)
            },
        )
    }
}

impl SessionCore {
    pub fn current_canonical_player_farsight_object_value_like_cpp(&self) -> Option<ObjectGuid> {
        let guid = self.player_guid()?;
        let key = self.current_canonical_player_map_key_like_cpp()?;
        let manager = self.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        Some(
            manager
                .find_map(key.map_id, key.instance_id)?
                .map()
                .get_typed_player(guid)?
                .active_data()
                .farsight_object,
        )
    }
}
