// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest availability: status classification and the `SatisfyQuest*` gates.

use super::*;
use wow_data::quest::{QuestDayCooldownBlock, QuestStatusBlock};

mod catalog;
mod prerequisites;
mod admission;

pub(crate) use prerequisites::represented_satisfy_quest_dependent_previous_quests_failed_like_cpp;
pub(super) use prerequisites::{
    player_race_or_class_mask_like_cpp,
    represented_satisfy_quest_dependent_previous_quests_failed_with_rules,
    represented_satisfy_quest_dependent_breadcrumb_quests_failed_like_cpp,
    represented_can_take_quest_after_expansion_like_cpp,
};
