use std::sync::Arc;

use crate::session::state::SessionCore;
use wow_core::ObjectGuid;

impl SessionCore {
    pub fn mutate_canonical_creature_by_guid_like_cpp<R>(
        &mut self,
        guid: ObjectGuid,
        f: impl FnOnce(&mut wow_entities::Creature) -> R,
    ) -> Option<R> {
        let map_key = self
            .canonical_object_lookup_map_key_like_cpp(u32::from(self.player_map_id_like_cpp()))?;
        let manager = Arc::clone(self.canonical_map_manager.as_ref()?);
        let mut manager = manager.lock().ok()?;
        let managed = manager.find_map_mut(map_key.map_id, map_key.instance_id)?;
        let creature = managed.map_mut().get_typed_creature_mut(guid)?;
        Some(f(creature))
    }

    pub fn world_creature_guids(&self) -> Vec<ObjectGuid> {
        let (map_id, instance_id) = self.current_legacy_runtime_map_key_like_cpp();
        if let Some(manager) = &self.map_manager {
            return manager
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .creature_guids(map_id, instance_id);
        }

        Vec::new()
    }
}
