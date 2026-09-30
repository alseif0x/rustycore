// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest reward orchestration, secondary effects, and required-item removal.

use super::*;
use crate::session::RepresentedQuestRecurrenceLikeCpp;

use crate::quest::application::QuestRewardDurablePlanLikeCpp;

mod currencies;
mod items;
mod validation;

mod required_items;
mod publication;
mod reputation;
mod lockouts;
mod settlement;

mod value;
pub(super) use value::{player_quest_level_like_cpp, reputation_rank_from_standing_like_cpp};
