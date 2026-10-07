// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the quest query handler context (#1263 F5).
//!
//! The application crate owns the handlers and their context; the session only
//! lends its hub and the read-only quest store.

use std::sync::Arc;

use wow_core::ObjectGuid;
use wow_data::quest::{QuestStore, QuestTemplate};
use wow_entities::PlayerQuestGameplayState;
use wow_world_application::{
    QuestGiverCompleteQuestHostLikeCpp, QuestQueryHandlerCxLikeCpp, QuestQueryHandlerHostLikeCpp,
};
use wow_world_core::session::HubRef;

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl QuestQueryHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn quest_query_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> QuestQueryHandlerCxLikeCpp<'a> {
        let quest_store = self.catalogs.quests.store.clone();
        let (quest_state, lifecycle, hub) = crate::session::split_quest_state_lifecycle_mut(self);
        QuestQueryHandlerCxLikeCpp::new(hub, quest_store, quest_state, lifecycle, cfg!(test))
    }

    fn sync_player_registry_state_after_quest_change_like_cpp(&mut self) {
        self.sync_player_registry_state_like_cpp();
    }
}

/// The shell-only capabilities of the moved quest-completion body. Every method
/// delegates to the existing World operation at the exact point the World body
/// invoked it, so no World body is duplicated and no step value is needed.
impl QuestGiverCompleteQuestHostLikeCpp for WorldSession {
    fn complete_quest_hub_ref_like_cpp(&self) -> HubRef<'_> {
        crate::session::hub_ref(self)
    }

    fn complete_quest_quest_store_like_cpp(&self) -> Option<Arc<QuestStore>> {
        self.catalogs.quests.store.clone()
    }

    fn complete_quest_player_quest_gameplay_snapshot_like_cpp(
        &self,
    ) -> Option<PlayerQuestGameplayState> {
        WorldSession::player_quest_gameplay_snapshot_like_cpp(self)
    }

    fn complete_quest_represented_involved_source_allows_like_cpp(
        &self,
        source_guid: ObjectGuid,
        quest_id: u32,
        quest_store: &QuestStore,
    ) -> bool {
        crate::session::cx_quest_state_ref(self)
            .represented_quest_giver_involved_source_allows_quest_like_cpp(
                source_guid,
                quest_id,
                quest_store,
            )
    }

    fn complete_quest_can_see_start_quest_represented_bounded_like_cpp(
        &self,
        quest: &QuestTemplate,
    ) -> bool {
        WorldSession::can_see_start_quest_represented_bounded_like_cpp(self, quest)
    }

    fn complete_quest_can_reward_quest_represented_bounded_like_cpp(
        &self,
        quest: &QuestTemplate,
    ) -> bool {
        WorldSession::can_reward_quest_represented_bounded_like_cpp(self, quest)
    }

    fn complete_quest_can_complete_repeatable_quest_represented_bounded_like_cpp(
        &self,
        quest: &QuestTemplate,
    ) -> bool {
        WorldSession::can_complete_repeatable_quest_represented_bounded_like_cpp(self, quest)
    }

    fn send_represented_quest_giver_request_items_with_completion_like_cpp(
        &mut self,
        source_guid: ObjectGuid,
        quest: &QuestTemplate,
        can_complete: bool,
        auto_launched: bool,
    ) {
        WorldSession::send_represented_quest_giver_request_items_with_completion_like_cpp(
            self,
            source_guid,
            quest,
            can_complete,
            auto_launched,
        )
    }
}
