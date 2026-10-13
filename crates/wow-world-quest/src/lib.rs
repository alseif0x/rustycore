// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest operations, player-condition projection and inventory valuation
//! shared by World adapters (#1363 A6: moved from `wow-world-application`).
//!
//! `PlayerConditionProjectionCxLikeCpp` is extended by inherent impls in
//! `quest/visibility.rs` and `inventory_valuation`, so the cluster lives in
//! one crate.

mod trainer_admission;
pub use trainer_admission::{
    TrainerAdmissionProofLikeCpp, trainer_condition_admission_proof_like_cpp,
};
mod inventory_valuation;
mod player_conditions;
mod quest;
mod quest_handlers;
mod quest_query_handlers;
mod registry_sync;
pub use inventory_valuation::{
    can_equip_inventory_item_like_cpp, can_equip_unique_item_like_cpp,
    can_use_inventory_item_represented_with_loading_like_cpp,
};
pub use player_conditions::{
    PlayerConditionProjectionCxLikeCpp, PlayerConditionProjectionInputsLikeCpp,
    RepresentedPlayerConditionContextLikeCpp, meets_player_condition_id_like_cpp,
    player_condition_projection_cx_like_cpp,
};
pub use quest::QuestEligibilityCx;
pub use quest::QuestRewardDurablePlanLikeCpp;
pub use quest::add_currency_quest_reward_like_cpp;
pub use quest::represented_gameobject_loot_ids_have_quest_loot_for_player_like_cpp;
pub use quest::{
    MAX_QUEST_LOG_SIZE_LIKE_CPP, QuestObjectiveProgressCx, QuestRewardCommitCx, QuestRewardCx,
    RepresentedPendingQuestSharingLikeCpp, RepresentedPushQuestToPartyOutcomeLikeCpp,
    RepresentedPushQuestToPartyOutcomeReasonLikeCpp, RepresentedQuestCompleteStatusUpdateLikeCpp,
    RepresentedQuestConfirmAcceptLikeCpp, RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp,
    RepresentedQuestObjectiveProgressEventLikeCpp, RepresentedQuestPushResultResponseLikeCpp,
    RepresentedQuestRewardReputationSourceLikeCpp, SessionQuestState,
    begin_exclusive_player_money_persistence_like_cpp, find_quest_slot_like_cpp,
    get_quest_slot_quest_id_like_cpp, invalidate_player_quest_status_authority_like_cpp,
    mutate_and_persist_player_gold_exclusive_like_cpp, plan_quest_status_save_like_cpp,
    quest_log_create_entries_like_cpp, reconcile_durable_loot_money_before_save_like_cpp,
    represented_can_see_spell_click_on_like_cpp,
    represented_viewer_dependent_creature_npc_flags_like_cpp, save_changed_quest_statuses_like_cpp,
    save_quest_to_db_like_cpp, send_represented_quest_log_slot_update_like_cpp,
};
pub use quest::{QuestDialogClassificationLikeCpp, RepresentedQuestGiverStatusSourceLikeCpp};
#[cfg(any(test, feature = "test-fixtures"))]
pub use quest::{
    QuestObjectiveRegistryFixtureRefsLikeCpp, QuestRewardItemPlanningFixtureRefsLikeCpp,
    QuestRewardReputationFixtureRefsLikeCpp, QuestXpGainFixtureRefsLikeCpp,
    RepresentedQuestRewardMailLikeCpp, RepresentedQuestRewardReputationLikeCpp,
    RepresentedQuestRewardSpellCastLikeCpp, RepresentedQuestRewardSpellKindLikeCpp,
    RepresentedQuestRewardTalentPointsLikeCpp, RepresentedQuestRewardTitleLikeCpp,
};
pub use quest::{
    RepresentedQuestCompleteDialogLikeCpp, represented_quest_complete_dialog_like_cpp,
    represented_quest_has_item_objective_like_cpp, represented_quest_rewards_block_like_cpp,
};
pub use quest::{
    VisibilityRefreshCxLikeCpp, VisibilityRefreshHostLikeCpp,
    represented_gameobject_activate_to_quest_like_cpp,
    represented_gameobject_dynamic_flags_for_player_like_cpp,
    represented_gameobject_dynamic_flags_update_like_cpp,
    represented_gameobject_go_state_for_viewer_like_cpp,
    represented_gameobject_is_for_quests_like_cpp, represented_has_quest_for_gameobject_like_cpp,
    represented_meets_player_condition_id_like_cpp, update_visible_gameobjects_like_cpp,
    update_visible_gameobjects_or_spell_clicks_like_cpp, update_visible_spell_clicks_like_cpp,
};
pub use quest::{
    clear_represented_pending_quest_sharing_like_cpp, mutate_player_quest_gameplay_like_cpp,
    player_quest_gameplay_snapshot_like_cpp, represented_pending_quest_sharing_like_cpp,
};
pub use quest_query_handlers::QuestPushResultTailLikeCpp;
pub use quest_query_handlers::represented_quest_completion_npc_response_like_cpp;
pub use quest_query_handlers::{
    QuestQueryHandlerCxLikeCpp, QuestQueryHandlerHostLikeCpp,
    register_quest_query_handlers_like_cpp,
};
// The exact `ApplicationQuest` owner facade the handler-contract checker
// requires: the two canonical exports as one use item (#1263 F5 remaining
// families).
pub use quest_handlers::{QuestHandlerHostLikeCpp, register_quest_handlers_like_cpp};
// The moved quest-completion body and the two quest helpers it owns are exported
// as a separate item so the owner's exact three-name facade above stays unchanged.
pub use quest_query_handlers::{
    QuestGiverCompleteQuestHostLikeCpp, handle_quest_giver_complete_quest_like_cpp,
    quest_giver_creature_id_from_source_like_cpp, quest_is_disabled_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use registry_sync::PlayerRegistryHydrationContext;
pub use registry_sync::PlayerRegistrySyncContext;
pub use registry_sync::sync_player_registry_state_like_cpp;
