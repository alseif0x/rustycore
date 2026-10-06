use std::sync::Arc;

use crate::session::state::SessionCore;
use wow_data::{DifficultyStore, MapDifficultyStore};

impl crate::session::state::SessionCatalogs {
    pub fn map_difficulty_store(&self) -> Option<&Arc<MapDifficultyStore>> {
        self.maps.difficulty_store.as_ref()
    }

    pub fn difficulty_store(&self) -> Option<&Arc<DifficultyStore>> {
        self.difficulty_store.as_ref()
    }
}

impl SessionCore {
    pub fn current_map_difficulty_id_like_cpp(&self) -> u8 {
        if let Some(difficulty_id) = self.current_canonical_player_map_difficulty_id_like_cpp() {
            return difficulty_id;
        }
        let map_id = u32::from(self.player_map_id_like_cpp());
        self.canonical_map_manager
            .as_ref()
            .and_then(|manager| manager.lock().ok())
            .and_then(|manager| {
                manager
                    .find_map(map_id, 0)
                    .map(|managed| managed.map().spawn_mode())
            })
            .unwrap_or(0)
    }

    /// C++ `Map::GetDifficultyID` for the map that actually owns this Player.
    ///
    /// The same map id can have multiple live instances. Do not infer spell
    /// metadata from the instance-zero map when the canonical player belongs
    /// to a difficulty-specific `ManagedMap`.
    pub fn current_canonical_player_map_difficulty_id_like_cpp(&self) -> Option<u8> {
        let player_guid = self.player_guid()?;
        let map_id = u32::from(self.player_map_id_like_cpp());
        let manager = self.canonical_map_manager.as_ref()?.lock().ok()?;
        let mut difficulty_id = None;
        manager.do_for_all_maps_with_map_id(map_id, |managed| {
            if difficulty_id.is_none() && managed.map().get_typed_player(player_guid).is_some() {
                difficulty_id = Some(managed.difficulty());
            }
        });
        difficulty_id
    }
}
