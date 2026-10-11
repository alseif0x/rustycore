// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World adapters for the App-owned quest reward operation.

use super::*;
use crate::quest::application::QuestRewardDurablePlanLikeCpp;

mod controller;
mod currencies;
mod items;
mod validation;

impl WorldSession {
    #[cfg(test)]
    pub(super) async fn reward_represented_quest_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
        quest_giver_guid: ObjectGuid,
        choice: QuestChoiceItemLikeCpp,
    ) -> bool {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return false;
        };
        self.reward_represented_quest_with_generator_like_cpp(
            generator.as_ref(),
            quest,
            quest_giver_guid,
            choice,
        )
        .await
    }
}
