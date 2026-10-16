//! Map key resolution used by the represented Session.
//!
//! Moved out of the Session root under #613. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub fn set_vmap_indoor_check_like_cpp(&mut self, enabled: bool) {
        self.config.vmap_indoor_check_like_cpp = enabled;
    }
    pub fn set_mmap_runtime_config_like_cpp(&mut self, config: MMapRuntimeConfigLikeCpp) {
        self.config.mmap_runtime_config_like_cpp = config;
    }
    pub fn set_map_store(&mut self, store: Arc<MapStore>) {
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.catalogs.maps.store = Some(store);
    }
    /// Set the immutable C++ `sDungeonEncounterStore` catalog used by
    /// `Player::IsLockedToDungeonEncounter`.
    pub fn set_dungeon_encounter_store(&mut self, store: Arc<DungeonEncounterStore>) {
        self.catalogs.dungeon_encounter_store = Some(store);
    }
    #[cfg(test)]
    pub(crate) fn under_map_damage_events_like_cpp(&self) -> &[MovementUnderMapDamageEvent] {
        &self.fixtures.movement.under_map_damage_events_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn set_taxi_node_map_id_like_cpp(&mut self, node_id: u32, map_id: u16) {
        self.view.taxi_node_map_ids_like_cpp.insert(node_id, map_id);
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/instances/map_key/f3_shims.rs"]
mod f3_shims;
