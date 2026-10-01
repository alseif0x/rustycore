// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub fn set_adventure_map_poi_store(&mut self, store: Arc<AdventureMapPoiStore>) {
        self.catalogs.set_adventure_map_poi_store(store)
    }
    #[cfg(test)]
    pub fn adventure_map_poi_store(&self) -> Option<&Arc<AdventureMapPoiStore>> {
        self.catalogs.adventure_map_poi_store()
    }
    #[cfg(test)]
    pub(crate) fn represented_reveal_world_map_overlay_criteria_like_cpp(&self) -> &[u32] {
        self.instances
            .represented_reveal_world_map_overlay_criteria_like_cpp()
    }
}
