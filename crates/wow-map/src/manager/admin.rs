//! MapManager instance and administrative operations.

use super::*;

impl MapManager {
    pub fn num_instances(&self) -> u32 {
        self.maps
            .values()
            .filter(|map| map.kind().is_dungeon())
            .count() as u32
    }

    pub fn num_players_in_instances(&self) -> u32 {
        self.maps
            .values()
            .filter(|map| map.kind().is_dungeon())
            .map(ManagedMap::player_count)
            .sum()
    }

    pub fn init_instance_ids(&mut self, max_existing_instance_id: u64) {
        self.instance_ids
            .init_instance_ids(max_existing_instance_id);
    }

    pub fn register_instance_id(&mut self, instance_id: u32) {
        self.instance_ids.register_instance_id(instance_id);
    }

    pub fn generate_instance_id(&mut self) -> Option<u32> {
        self.instance_ids.generate_instance_id()
    }

    pub fn free_instance_id(&mut self, instance_id: u32) {
        self.instance_ids.free_instance_id(instance_id);
    }

    pub fn next_instance_id(&self) -> u32 {
        self.instance_ids.next_instance_id()
    }

    pub fn map_updater(&self) -> &MapUpdater {
        &self.updater
    }

    pub fn map_updater_mut(&mut self) -> &mut MapUpdater {
        &mut self.updater
    }

    pub fn increase_scheduled_scripts_count(&mut self) {
        self.scheduled_scripts += 1;
    }

    pub fn decrease_scheduled_script_count(&mut self) {
        self.scheduled_scripts = self.scheduled_scripts.saturating_sub(1);
    }

    pub fn decrease_scheduled_script_count_by(&mut self, count: usize) {
        self.scheduled_scripts = self.scheduled_scripts.saturating_sub(count);
    }

}
