// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::collections::HashSet;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MMapRuntimeConfigLikeCpp {
    pub data_dir: String,
    pub enabled: bool,
    pub disabled_map_ids: HashSet<u32>,
}

pub type WaypointPathResolverLikeCpp =
    Arc<dyn Fn(u32) -> Option<wow_movement::WaypointPath> + Send + Sync>;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlayerGridLoadOutcomeLikeCpp {
    pub map_unavailable: bool,
    pub map_created: bool,
    pub grid_loaded_now: bool,
    pub metadata_entries: usize,
    pub skipped_already_loaded: usize,
    pub skipped_should_not_spawn: usize,
    pub skipped_difficulty_mismatch: usize,
    pub stale_index_entries: usize,
    pub creature_records_added: usize,
    pub gameobject_records_added: usize,
    pub area_trigger_records_added: usize,
    pub pre_add_records_added: usize,
    pub add_to_map_errors: usize,
    pub load_record_missing: usize,
    pub creature_load_record_missing: usize,
    pub gameobject_load_record_missing: usize,
    pub area_trigger_load_record_missing: usize,
}

pub type PlayerGridLoadResolverLikeCpp =
    Arc<dyn Fn(u16, Option<u32>, wow_core::Position) -> PlayerGridLoadOutcomeLikeCpp + Send + Sync>;

impl Default for MMapRuntimeConfigLikeCpp {
    fn default() -> Self {
        Self {
            data_dir: "./Data".to_string(),
            enabled: true,
            disabled_map_ids: HashSet::new(),
        }
    }
}

impl MMapRuntimeConfigLikeCpp {
    pub fn pathfinding_enabled_for_map_like_cpp(&self, map_id: u32) -> bool {
        self.enabled && !self.disabled_map_ids.contains(&map_id)
    }

    pub fn should_try_pathfinding_like_cpp(
        &self,
        map_id: u32,
        owner_ignores_pathfinding: bool,
    ) -> bool {
        self.pathfinding_enabled_for_map_like_cpp(map_id) && !owner_ignores_pathfinding
    }
}
