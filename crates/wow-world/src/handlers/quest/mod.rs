// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest handlers, organised by feature.
//!
//! Issue #224 split the former 8,274-line `handlers/quest.rs` into private
//! feature modules. The logical owner, every registration, opcode and
//! dispatcher arm are unchanged; this module keeps the shared constants,
//! helper types and free functions the features build on.

mod quest_handler_host;
mod quest_query_host;
mod rewards;
mod state;

// The quest item-storage adapter (`handlers/quest_item_storage_host.rs`, moved
// out of this tree in #1363 slice 12d) and the scenarios read it from here.
pub(super) use wow_constants::InventoryResult;
// Test-only: its production readers were the eleven quest-giver registrations
// that moved to the quest crate's `quest_handlers` area registrar under
// #1263 F5; the quest test shims and scenarios still reach it through
// `super::*`. The item names are test-only since #1363 slice 6b: their
// production readers moved with the item-objective and source-item bodies.
#[cfg(test)]
use wow_constants::{ClientOpcodes, ItemContext, ItemFieldFlags, item::ItemFlags3};
use wow_core::ObjectGuid;
pub use wow_entities::PlayerQuestStatusRecord as PlayerQuestStatus;
// Test-only: the production readers of these two were the eleven quest-giver
// registrations that moved to the quest crate's `quest_handlers` area
// registrar under #1263 F5; the quest scenarios still reach them through
// `super::*`.
#[cfg(test)]
use wow_handler::{PacketProcessing, SessionStatus};
// Test-only: the quest handler test fixtures read the dialog status
// discriminators through this module's glob import.
#[cfg(test)]
use wow_packet::packets::quest::quest_giver_status;

use crate::session::{RepresentedAdventureMapStartQuestLikeCpp, WorldSession};
#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::{
    RepresentedQuestRewardMailLikeCpp, RepresentedQuestRewardReputationLikeCpp,
    RepresentedQuestRewardSpellCastLikeCpp, RepresentedQuestRewardSpellKindLikeCpp,
    RepresentedQuestRewardTalentPointsLikeCpp, RepresentedQuestRewardTitleLikeCpp,
};
#[cfg(test)]
use wow_conditions::{QUEST_STATUS_FAILED_LIKE_CPP, QUEST_STATUS_REWARDED_LIKE_CPP};

#[cfg(test)]
pub(crate) use wow_constants::quest::QUEST_FLAGS_PLAYER_CAST_COMPLETE_LIKE_CPP;
pub(crate) use wow_constants::quest::{
    QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP, QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
};
#[cfg(test)]
const CURRENCY_DESTROY_REASON_QUEST_TURNIN_LIKE_CPP: i32 = 3;

// The reward choice moved with the reward-inventory admission (#1363 slice 6b).
pub(crate) use wow_world_quest::QuestChoiceItemLikeCpp;

// The source-item bound-objective records are read by the loot item storage
// moved to `wow-world-loot-ops` (#1363 slice 5b), so they live in the quest crate.
pub(crate) use wow_world_quest::{
    QuestSourceItemBoundPersistencePlanLikeCpp, QuestSourceItemBoundPreflightLikeCpp,
};

pub(crate) use wow_world_quest::RepresentedQuestGiverStatusSourceLikeCpp;

#[cfg(test)]
#[path = "../../../unit_tests/handlers/quest_tests.rs"]
mod tests;
#[cfg(test)]
pub(crate) use super::quest_handler_test_shims::quest_handler_test_reexports::*;

#[cfg(test)]
use wow_constants::quest::{
    QUEST_OBJECTIVE_CURRENCY_LIKE_CPP as QUEST_OBJECTIVE_CURRENCY_LIKE_CPP_LOCAL,
    QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP as QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL,
    QUEST_OBJECTIVE_FLAG_OPTIONAL_LIKE_CPP as QUEST_OBJECTIVE_FLAG_OPTIONAL_LIKE_CPP_LOCAL,
    QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP as QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP_LOCAL,
    QUEST_OBJECTIVE_FLAG_SEQUENCED_LIKE_CPP as QUEST_OBJECTIVE_FLAG_SEQUENCED_LIKE_CPP_LOCAL,
    QUEST_OBJECTIVE_ITEM_LIKE_CPP as QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
    QUEST_OBJECTIVE_MONEY_LIKE_CPP as QUEST_OBJECTIVE_MONEY_LIKE_CPP_LOCAL,
    QUEST_OBJECTIVE_MONSTER_LIKE_CPP as QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL,
    QUEST_OBJECTIVE_PROGRESS_BAR_LIKE_CPP as QUEST_OBJECTIVE_PROGRESS_BAR_LIKE_CPP_LOCAL,
};

// ── PlayerQuestStatus ────────────────────────────────────────────────────────
