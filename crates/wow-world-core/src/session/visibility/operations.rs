use crate::session::state::SessionCore;
use wow_core::ObjectGuid;

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
