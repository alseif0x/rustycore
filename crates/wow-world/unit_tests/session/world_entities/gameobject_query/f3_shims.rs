// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn record_represented_gameobject_faction_template_like_cpp(
        &mut self,
        guid: ObjectGuid,
        faction_template: u32,
    ) {
        self.world_entities
            .record_represented_gameobject_faction_template_like_cpp(guid, faction_template)
    }
    pub(crate) fn gameobject_template_lifecycle_store(
        &self,
    ) -> Option<&Arc<GameObjectTemplateLifecycleStoreLikeCpp>> {
        self.catalogs.gameobject_template_lifecycle_store()
    }
}
