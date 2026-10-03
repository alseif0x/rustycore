use super::LootState;
use wow_map::MapKey;
use wow_world_core::session::HubRef;

impl LootState {
    /// Revalidates the exact map ownership captured before a multi-lock loot
    /// authority reconciliation. If a canonical Player existed at capture
    /// time, fallback lookup is forbidden: disappearing or moving during the
    /// attempt must fail closed rather than mutate the old map.
    pub fn loot_reconciliation_map_key_still_valid_like_cpp(
        &self,
        hub: HubRef<'_>,
        map_key: MapKey,
        canonical_player_was_present: bool,
    ) -> bool {
        if canonical_player_was_present {
            return hub.core.current_canonical_player_map_key_like_cpp() == Some(map_key);
        }
        if hub.core.canonical_map_manager.is_some() {
            return hub
                .core
                .canonical_object_lookup_map_key_like_cpp(map_key.map_id)
                == Some(map_key);
        }
        let (map_id, instance_id) = hub.core.current_legacy_runtime_map_key_like_cpp();
        u32::from(map_id) == map_key.map_id && instance_id == map_key.instance_id
    }
}
