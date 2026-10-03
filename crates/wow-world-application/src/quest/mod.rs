// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Pure quest operation planning shared by World adapters.

mod reward_plan;
mod reward_commit;
mod money_persistence;
mod session_state;

pub use self::reward_plan::QuestRewardDurablePlanLikeCpp;
pub use self::reward_commit::QuestRewardCommitCx;
pub use self::money_persistence::{
    begin_exclusive_player_money_persistence_like_cpp,
    reconcile_durable_loot_money_before_save_like_cpp,
};
pub use self::session_state::SessionQuestState;
pub use self::session_state::contracts::{
    RepresentedQuestCompleteStatusUpdateLikeCpp, RepresentedQuestObjectiveProgressEventLikeCpp,
    RepresentedQuestPushResultResponseLikeCpp, RepresentedPushQuestToPartyOutcomeLikeCpp,
    RepresentedPushQuestToPartyOutcomeReasonLikeCpp, RepresentedPendingQuestSharingLikeCpp,
    RepresentedQuestConfirmAcceptLikeCpp, RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp,
    RepresentedQuestRewardReputationSourceLikeCpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use self::session_state::contracts::{
    RepresentedQuestRewardMailLikeCpp, RepresentedQuestRewardReputationLikeCpp,
    RepresentedQuestRewardSpellCastLikeCpp, RepresentedQuestRewardSpellKindLikeCpp,
    RepresentedQuestRewardTalentPointsLikeCpp, RepresentedQuestRewardTitleLikeCpp,
};
