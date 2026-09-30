//! Legacy per-map entry to the single memory-only Creature respawn prefix.
use super::MapManager;
use crate::map::LegacyCreatureRespawnPrefix;
use std::time::Instant;

impl MapManager {
    pub fn prepare_creature_respawns(
        &mut self,
        map_id: u16,
        instance_id: u32,
        now: Instant,
        conversion_now: Instant,
        conversion_now_secs: i64,
        persistent_world_map: bool,
    ) -> LegacyCreatureRespawnPrefix {
        crate::map::prepare_legacy_creature_respawns(
            self, (map_id, instance_id), now, conversion_now, conversion_now_secs,
            persistent_world_map,
        )
    }
}
