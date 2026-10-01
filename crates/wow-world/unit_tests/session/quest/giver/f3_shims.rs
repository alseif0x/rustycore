// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn represented_quest_giver_accept_source_allows_quest_like_cpp(
        &self,
        source_guid: ObjectGuid,
        quest_id: u32,
        quest_store: &wow_data::quest::QuestStore,
    ) -> bool {
        crate::session::cx_quest_state_ref(self)
            .represented_quest_giver_accept_source_allows_quest_like_cpp(
                source_guid,
                quest_id,
                quest_store,
            )
    }
    pub(crate) fn represented_quest_giver_involved_source_allows_quest_like_cpp(
        &self,
        source_guid: ObjectGuid,
        quest_id: u32,
        quest_store: &wow_data::quest::QuestStore,
    ) -> bool {
        crate::session::cx_quest_state_ref(self)
            .represented_quest_giver_involved_source_allows_quest_like_cpp(
                source_guid,
                quest_id,
                quest_store,
            )
    }
}
