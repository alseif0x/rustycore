// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub fn set_area_trigger_db2_store(&mut self, store: Arc<AreaTriggerDb2Store>) {
        self.catalogs.set_area_trigger_db2_store(store)
    }
    #[cfg(test)]
    pub fn set_area_trigger_script_store(&mut self, store: Arc<AreaTriggerScriptStoreLikeCpp>) {
        self.catalogs.set_area_trigger_script_store(store)
    }
    #[cfg(test)]
    pub fn set_area_trigger_script_dispatcher_like_cpp(
        &mut self,
        dispatcher: AreaTriggerScriptDispatcherLikeCpp,
    ) {
        self.view
            .set_area_trigger_script_dispatcher_like_cpp(dispatcher)
    }
    #[cfg(test)]
    pub fn set_tavern_area_trigger_store(&mut self, store: Arc<TavernAreaTriggerStoreLikeCpp>) {
        self.catalogs.set_tavern_area_trigger_store(store)
    }
    #[cfg(test)]
    pub(crate) fn represented_area_zone_criteria_like_cpp(
        &self,
    ) -> &[RepresentedAreaZoneCriteriaLikeCpp] {
        self.instances.represented_area_zone_criteria_like_cpp()
    }
}
