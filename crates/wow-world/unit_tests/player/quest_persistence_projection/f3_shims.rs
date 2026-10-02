// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;
use crate::handlers::quest::PlayerQuestStatus;

impl crate::session::WorldSession {
    pub(crate) fn represented_quest_status_persistence_like_cpp(
        &self,
        status: &PlayerQuestStatus,
    ) -> wow_persistence::QuestStatusPersistenceLikeCpp {
        self.catalogs
            .represented_quest_status_persistence_like_cpp(status)
    }
}
