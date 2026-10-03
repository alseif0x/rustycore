use crate::InstanceState;
use wow_constants::object::TypeId;
use wow_data::{
    DisableWorldObjectRefLikeCpp, DISABLE_TYPE_MAP, MAP_ARENA_LIKE_CPP,
    MAP_BATTLEGROUND_LIKE_CPP,
};
use wow_world_core::session::HubRef;

impl InstanceState {
    pub fn create_map_db2_entries_like_cpp(
        &self,
        hub: HubRef<'_>,
        map_id: u32,
        difficulty_id: wow_map::Difficulty,
    ) -> Option<wow_instances::MapDb2Entries> {
        wow_instances::MapDb2Entries::from_downscaled_stores_like_cpp(
            hub.catalogs.map_store()?.as_ref(),
            hub.catalogs.map_difficulty_store()?.as_ref(),
            hub.catalogs.difficulty_store()?.as_ref(),
            map_id,
            difficulty_id,
        )
    }

    /// Resolve the existing downscaled map/difficulty row from the selected
    /// catalog references supplied by the application handler.
    pub fn create_map_db2_entries_from_stores_like_cpp(
        &self,
        map_store: &wow_data::MapStore,
        map_difficulty_store: &wow_data::MapDifficultyStore,
        difficulty_store: &wow_data::DifficultyStore,
        map_id: u32,
        difficulty_id: wow_map::Difficulty,
    ) -> Option<wow_instances::MapDb2Entries> {
        wow_instances::MapDb2Entries::from_downscaled_stores_like_cpp(
            map_store,
            map_difficulty_store,
            difficulty_store,
            map_id,
            difficulty_id,
        )
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_reveal_world_map_overlay_criteria_like_cpp(&self) -> &[u32] {
        &self.represented_reveal_world_map_overlay_criteria_like_cpp
    }

    pub fn is_disabled_map_type_for_player_like_cpp(
        &self,
        hub: HubRef<'_>,
        disable_type: u32,
        map_id: u32,
    ) -> bool {
        let Some(disable_mgr) = hub.catalogs.disable_mgr() else {
            return false;
        };
        let Some(map_store) = hub.catalogs.map_store() else {
            return false;
        };

        let current_map_id = u32::from(hub.core.player_map_id_like_cpp());
        let Some((_, area_id)) = hub.player_zone_area_like_cpp() else {
            return true;
        };
        let current_map_instance_type = map_store
            .get(current_map_id)
            .map(|entry| entry.instance_type);

        disable_mgr.is_disabled_for_like_cpp(
            disable_type,
            map_id,
            Some(DisableWorldObjectRefLikeCpp {
                type_id: TypeId::Player,
                map_id: current_map_id,
                area_id,
                is_pet: false,
                is_battle_arena: current_map_instance_type == Some(MAP_ARENA_LIKE_CPP),
                is_battleground: current_map_instance_type == Some(MAP_BATTLEGROUND_LIKE_CPP),
                player_map_difficulty: None,
            }),
            0,
            Some(map_store.as_ref()),
        )
    }

    pub fn is_map_disabled_for_player_like_cpp(
        &self,
        hub: HubRef<'_>,
        map_id: u32,
    ) -> bool {
        self.is_disabled_map_type_for_player_like_cpp(hub, DISABLE_TYPE_MAP, map_id)
    }
}
