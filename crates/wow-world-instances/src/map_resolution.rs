use crate::InstanceState;
use wow_core::ObjectGuid;
use wow_world_core::session::HubRef;

impl InstanceState {
    pub fn canonical_map_has_seer_like_object_like_cpp(
        &self,
        hub: HubRef<'_>,
        target: ObjectGuid,
    ) -> bool {
        if target.is_empty() {
            return false;
        }
        let Some(key) = hub.core.current_canonical_player_map_key_like_cpp() else {
            return false;
        };
        let Some(manager) = hub.core.canonical_map_manager.as_ref() else {
            return false;
        };
        let Ok(manager) = manager.lock() else {
            return false;
        };
        manager
            .find_map(key.map_id, key.instance_id)
            .and_then(|managed| {
                managed.map().with_world_object_by_kinds_like_cpp(
                    target,
                    wow_entities::represented_seer_kinds_like_cpp(),
                    |_| (),
                )
            })
            .is_some()
    }
}
