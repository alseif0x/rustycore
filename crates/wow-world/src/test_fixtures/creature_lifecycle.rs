//! Feature-only forwarding to the existing lifecycle APP operations.
use crate::session::WorldSession;
use crate::map_manager::{PendingRespawn, RuntimeOutput, RuntimeTickOwner, SharedMapManager};
use std::time::Instant;

impl WorldSession {
    pub fn fixture_lifecycle_set_current_map(&mut self, map_id: u16) {
        self.current_map_id = map_id;
    }
    pub fn fixture_lifecycle_map_id(&self) -> u16 {
        self.player_map_id_like_cpp()
    }
    pub fn fixture_lifecycle_manager(&self) -> &Option<SharedMapManager> {
        &self.map_manager
    }
    pub fn fixture_lifecycle_tick_owner(&self) -> RuntimeTickOwner {
        self.runtime_tick_owner_like_cpp()
    }
    pub fn fixture_lifecycle_tick(&mut self) -> RuntimeOutput {
        self.run_creatures_tick_with_fixture_phase(true)
    }
    pub fn fixture_lifecycle_normal_tick(&mut self) -> RuntimeOutput {
        self.run_creatures_tick()
    }
    pub fn fixture_lifecycle_tick_sync(&mut self) {
        let output = self.fixture_lifecycle_tick();
        self.flush_runtime_output(output);
    }
    pub fn fixture_lifecycle_flush(&self, output: RuntimeOutput) {
        self.flush_runtime_output(output);
    }
    pub fn fixture_lifecycle_push_respawn(&mut self, map_id: u16, instance_id: u32, row: PendingRespawn) {
        self.push_map_respawn_like_cpp(map_id, instance_id, row);
    }
    pub fn fixture_lifecycle_drain_respawns(&mut self, map_id: u16, instance_id: u32, now: Instant) -> Vec<PendingRespawn> {
        self.drain_ready_map_respawns_like_cpp(map_id, instance_id, now)
    }
}
