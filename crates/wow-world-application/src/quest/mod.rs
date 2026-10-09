// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest application operations shared by selected-owner World adapters.

mod complete;
mod completion;
mod currencies;
mod dialog_status;
mod loot_requirements;
mod money_persistence;
mod objective_progress;
mod objectives;
mod quest_log;
mod reward;
mod reward_commit;
mod reward_plan;
mod session_state;
mod visibility;

pub use self::complete::{
    RepresentedQuestCompleteDialogLikeCpp, represented_quest_complete_dialog_like_cpp,
    represented_quest_has_item_objective_like_cpp, represented_quest_rewards_block_like_cpp,
};
pub use self::currencies::add_currency_quest_reward_like_cpp;
pub use self::dialog_status::{
    QuestDialogClassificationLikeCpp, RepresentedQuestGiverStatusSourceLikeCpp,
};
pub use self::loot_requirements::represented_gameobject_loot_ids_have_quest_loot_for_player_like_cpp;
pub use self::money_persistence::{
    begin_exclusive_player_money_persistence_like_cpp,
    mutate_and_persist_player_gold_exclusive_like_cpp,
    reconcile_durable_loot_money_before_save_like_cpp,
};
pub use self::objective_progress::{
    MAX_QUEST_LOG_SIZE_LIKE_CPP, find_quest_slot_like_cpp,
    invalidate_player_quest_status_authority_like_cpp, plan_quest_status_save_like_cpp,
    save_changed_quest_statuses_like_cpp, save_quest_to_db_like_cpp,
};
pub use self::objectives::QuestObjectiveProgressCx;
#[cfg(any(test, feature = "test-fixtures"))]
pub use self::objectives::QuestObjectiveRegistryFixtureRefsLikeCpp;
pub use self::quest_log::{
    get_quest_slot_quest_id_like_cpp, quest_log_create_entries_like_cpp,
    send_represented_quest_log_slot_update_like_cpp,
};
pub use self::reward::QuestRewardCx;
#[cfg(any(test, feature = "test-fixtures"))]
pub use self::reward::QuestRewardItemPlanningFixtureRefsLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use self::reward::QuestRewardReputationFixtureRefsLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use self::reward::QuestXpGainFixtureRefsLikeCpp;
pub use self::reward_commit::QuestRewardCommitCx;
pub use self::reward_plan::QuestRewardDurablePlanLikeCpp;
pub use self::session_state::contracts::{
    RepresentedPendingQuestSharingLikeCpp, RepresentedPushQuestToPartyOutcomeLikeCpp,
    RepresentedPushQuestToPartyOutcomeReasonLikeCpp, RepresentedQuestCompleteStatusUpdateLikeCpp,
    RepresentedQuestConfirmAcceptLikeCpp, RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp,
    RepresentedQuestObjectiveProgressEventLikeCpp, RepresentedQuestPushResultResponseLikeCpp,
    RepresentedQuestRewardReputationSourceLikeCpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use self::session_state::contracts::{
    RepresentedQuestRewardMailLikeCpp, RepresentedQuestRewardReputationLikeCpp,
    RepresentedQuestRewardSpellCastLikeCpp, RepresentedQuestRewardSpellKindLikeCpp,
    RepresentedQuestRewardTalentPointsLikeCpp, RepresentedQuestRewardTitleLikeCpp,
};
pub use self::session_state::{
    SessionQuestState, clear_represented_pending_quest_sharing_like_cpp,
    mutate_player_quest_gameplay_like_cpp, player_quest_gameplay_snapshot_like_cpp,
    represented_pending_quest_sharing_like_cpp,
};
pub use self::visibility::QuestEligibilityCx;
pub use self::visibility::{
    RepresentedCanSeeSpellClickOutcomeLikeCpp, VisibilityRefreshCxLikeCpp,
    VisibilityRefreshHostLikeCpp, represented_can_see_spell_click_on_like_cpp,
    represented_gameobject_activate_to_quest_like_cpp,
    represented_gameobject_dynamic_flags_for_player_like_cpp,
    represented_gameobject_is_for_quests_like_cpp, represented_has_quest_for_gameobject_like_cpp,
    represented_meets_player_condition_id_like_cpp,
    represented_viewer_dependent_creature_npc_flags_like_cpp, update_visible_gameobjects_like_cpp,
    update_visible_gameobjects_or_spell_clicks_like_cpp, update_visible_spell_clicks_like_cpp,
};
