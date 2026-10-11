// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1363 slice 1: production-dead `persistence` entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl WorldSession {
    pub(crate) fn set_quest_poi_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::QuestPoiPersistencePortLikeCpp>,
    ) {
        crate::session::cx_quest_state(self).set_quest_poi_persistence_port_like_cpp(port)
    }
}

impl crate::session::QuestStateCx<'_> {
    pub(crate) fn set_quest_poi_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::QuestPoiPersistencePortLikeCpp>,
    ) {
        self.lifecycle.set_quest_poi_persistence_port_like_cpp(port);
    }
}
