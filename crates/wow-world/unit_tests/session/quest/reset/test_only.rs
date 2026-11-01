// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1363 slice 1: production-dead `reset` entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl WorldSession {
    pub(crate) fn set_quest_pool_store(
        &mut self,
        store: Arc<wow_data::quest::QuestPoolStoreLikeCpp>,
    ) {
        self.catalogs.set_quest_pool_store(store)
    }
}
