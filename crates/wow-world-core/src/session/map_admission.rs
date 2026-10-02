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
