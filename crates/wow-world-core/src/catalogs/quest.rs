// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest template and quest-rule catalogs a session reads.
//!
//! Separated from `WorldSession` under #674: the session holds exactly one field
//! of this type, so no slot is mirrored. The owner depends on `wow-data` stores
//! only and performs no async work.

use std::sync::Arc;

#[derive(Default)]
pub struct QuestCatalogsLikeCpp {
    pub faction_reward_store: Option<Arc<wow_data::progression_rewards::QuestFactionRewardStore>>,
    pub info_store: Option<Arc<wow_data::progression_rewards::QuestInfoStore>>,
    pub money_reward_store: Option<Arc<wow_data::progression_rewards::QuestMoneyRewardStore>>,
    pub package_item_store: Option<Arc<wow_data::progression_rewards::QuestPackageItemStore>>,
    pub pool_store: Option<Arc<wow_data::quest::QuestPoolStoreLikeCpp>>,
    pub store: Option<Arc<wow_data::quest::QuestStore>>,
    pub v2_store: Option<Arc<wow_data::progression_rewards::QuestV2Store>>,
    pub xp_store: Option<Arc<wow_data::quest_xp::QuestXpStore>>,
}
