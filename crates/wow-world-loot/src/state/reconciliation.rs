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
        hub.core.loot_release_access_like_cpp().loot_reconciliation_map_key_still_valid_like_cpp(map_key, canonical_player_was_present)
    }
}
