use crate::session::RepresentedGameObjectAccessLikeCpp;
use crate::session::state::SessionCore;
use wow_core::ObjectGuid;

impl SessionCore {
    pub fn canonical_gameobject_access_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<RepresentedGameObjectAccessLikeCpp> {
        if guid.is_empty() || !guid.is_game_object() {
            return None;
        }
        let map_key = self
            .canonical_object_lookup_map_key_like_cpp(u32::from(self.player_map_id_like_cpp()))?;
        let manager = self.canonical_map_manager.as_ref()?;
        let Ok(manager) = manager.lock() else {
            return None;
        };
        let map = manager.find_map(map_key.map_id, map_key.instance_id)?;
        let game_object = map.map().get_typed_game_object(guid)?;
        Some(RepresentedGameObjectAccessLikeCpp {
            entry: game_object.world().object().entry(),
            position: game_object.world().position(),
        })
    }
}
