// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Complete quest operations and their results.

pub(crate) mod reward_commit;
pub(crate) mod reward_plan;

pub(crate) use reward_plan::QuestRewardDurablePlanLikeCpp;
