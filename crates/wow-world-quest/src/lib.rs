// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest operations shared by World adapters (#1363 A6: moved from
//! `wow-world-application`). The player-condition projection, inventory
//! valuation and trainer admission proof moved down to
//! `wow-world-player-conditions` (#1363 slice Q).

mod module_hooks;
pub use module_hooks::{
    PlayerLevelChangedHookEventLikeCpp, PlayerMoneyChangedHookEventLikeCpp,
    QuestAcceptedHookEventLikeCpp, QuestModuleHookPortLikeCpp, QuestRewardedHookEventLikeCpp,
};
mod quest;
mod quest_handlers;
mod quest_query_handlers;
mod quest_session;
pub use quest::QuestEligibilityCx;
pub use quest::QuestRewardDurablePlanLikeCpp;
pub use quest::add_currency_quest_reward_like_cpp;
pub use quest::represented_gameobject_loot_ids_have_quest_loot_for_player_like_cpp;
pub use quest::{
    MAX_QUEST_LOG_SIZE_LIKE_CPP, QuestObjectiveProgressCx, QuestRewardCommitCx, QuestRewardCx,
    QuestSourceItemBoundPersistencePlanLikeCpp, QuestSourceItemBoundPreflightLikeCpp,
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
// The represented quest state, persistence, dialog and publication bodies
// moved from the World session with their host trait (#1363 slice 6a), and the
// item-objective, source-item, reward-inventory and eligibility bodies with the
// item-storage host (slice 6b).
pub use quest_session::unit_update_publication;
pub use quest_session::{
    ItemTransferQuestPersistencePlanLikeCpp, LoadSeasonalQuestStatusOutcomeLikeCpp,
    QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP, QUEST_FLAGS_EX_RECAST_ACCEPT_SPELL_ON_LOGIN_LIKE_CPP,
    QUEST_FLAGS_PLAYER_CAST_ACCEPT_LIKE_CPP, QUEST_FLAGS_SHARABLE_LIKE_CPP,
    QUEST_PUSH_REASON_ALREADY_DONE_LIKE_CPP, QUEST_PUSH_REASON_ALREADY_DONE_TO_RECIPIENT_LIKE_CPP,
    QUEST_PUSH_REASON_BUSY_LIKE_CPP, QUEST_PUSH_REASON_CLASS_LIKE_CPP,
    QUEST_PUSH_REASON_CLASS_TO_RECIPIENT_LIKE_CPP, QUEST_PUSH_REASON_DEAD_LIKE_CPP,
    QUEST_PUSH_REASON_DEAD_TO_RECIPIENT_LIKE_CPP, QUEST_PUSH_REASON_EXPANSION_LIKE_CPP,
    QUEST_PUSH_REASON_EXPANSION_TO_RECIPIENT_LIKE_CPP, QUEST_PUSH_REASON_HIGH_LEVEL_LIKE_CPP,
    QUEST_PUSH_REASON_HIGH_LEVEL_TO_RECIPIENT_LIKE_CPP, QUEST_PUSH_REASON_INVALID_LIKE_CPP,
    QUEST_PUSH_REASON_INVALID_TO_RECIPIENT_LIKE_CPP, QUEST_PUSH_REASON_LOG_FULL_LIKE_CPP,
    QUEST_PUSH_REASON_LOG_FULL_TO_RECIPIENT_LIKE_CPP, QUEST_PUSH_REASON_LOW_FACTION_LIKE_CPP,
    QUEST_PUSH_REASON_LOW_FACTION_TO_RECIPIENT_LIKE_CPP, QUEST_PUSH_REASON_LOW_LEVEL_LIKE_CPP,
    QUEST_PUSH_REASON_LOW_LEVEL_TO_RECIPIENT_LIKE_CPP, QUEST_PUSH_REASON_ON_QUEST_LIKE_CPP,
    QUEST_PUSH_REASON_ON_QUEST_TO_RECIPIENT_LIKE_CPP, QUEST_PUSH_REASON_PREREQUISITE_LIKE_CPP,
    QUEST_PUSH_REASON_PREREQUISITE_TO_RECIPIENT_LIKE_CPP, QUEST_PUSH_REASON_RACE_LIKE_CPP,
    QUEST_PUSH_REASON_RACE_TO_RECIPIENT_LIKE_CPP, QUEST_PUSH_REASON_SUCCESS_LIKE_CPP,
    QuestChoiceItemLikeCpp, QuestHandlerSessionHostLikeCpp, QuestItemStorageHostLikeCpp,
    QuestSessionHostLikeCpp, QuestSourceItemStoreOutcomeLikeCpp,
    RepresentedPreparedQuestMenuItemLikeCpp, ResetSeasonalQuestStatusOutcomeLikeCpp,
    ResetSeasonalQuestStatusReasonLikeCpp, SeasonalQuestStatusDbRowLikeCpp,
    add_quest_confirm_accept_local_state_like_cpp, apply_quest_complete_effect_like_cpp,
    apply_quest_item_added_bound_state_like_cpp, apply_quest_item_added_non_bound_state_like_cpp,
    apply_quest_item_added_objective_progress_with_generator_like_cpp,
    apply_quest_item_removed_like_cpp,
    apply_quest_source_item_added_non_bound_objective_progress_with_generator_like_cpp,
    apply_quest_source_item_bound_objective_preflight_with_generator_like_cpp,
    apply_quest_source_item_bound_objective_progress_for_object_with_generator_like_cpp,
    apply_represented_quest_rewarded_spells_like_cpp,
    begin_item_transfer_quest_persistence_like_cpp,
    begin_player_quest_status_authority_load_like_cpp, build_quest_poi_store_like_cpp,
    can_complete_repeatable_quest_represented_bounded_like_cpp,
    can_reward_quest_represented_bounded_like_cpp,
    can_see_start_quest_represented_bounded_like_cpp, can_take_quest,
    clear_loaded_quest_completed_bit_like_cpp,
    complete_represented_quest_after_add_with_generator_like_cpp,
    finish_item_transfer_quest_persistence_like_cpp, first_free_quest_slot_like_cpp,
    get_represented_quest_giver_status_with_catalog_like_cpp,
    handle_adventure_map_start_quest_with_catalog_like_cpp, handle_push_quest_to_party,
    handle_quest_confirm_accept_with_generator_like_cpp,
    handle_quest_giver_accept_quest_with_generator_like_cpp,
    handle_quest_giver_choose_reward_with_generator_like_cpp, handle_quest_giver_hello,
    handle_quest_giver_query_quest, handle_quest_giver_request_reward_with_generator_like_cpp,
    handle_quest_giver_status_query_with_catalog_like_cpp, handle_quest_poi_query,
    handle_send_repeatable_turn_in_request_items_command_like_cpp,
    handle_set_quest_sharing_info_and_send_details_command_like_cpp,
    insert_represented_quest_status_like_cpp, is_quest_disabled_like_cpp, load_player_quests,
    load_seasonal_quest_status_like_cpp, mark_represented_quest_explored_like_cpp,
    plan_bank_item_quest_persistence_like_cpp, plan_item_transfer_quest_persistence_like_cpp,
    plan_item_transfer_withdrawal_quest_persistence_like_cpp,
    plan_quest_source_item_bound_objective_persistence_like_cpp,
    publish_quest_item_added_status_changes_like_cpp,
    quest_bound_item_objective_quest_order_like_cpp, quest_money_reward_like_cpp,
    quest_poi_store_like_cpp, quest_source_item_quest_log_item_id_like_cpp,
    quest_xp_reward_like_cpp, read_quest_choice_item_like_cpp,
    read_quest_giver_accept_quest_like_cpp, read_quest_giver_query_quest_like_cpp,
    refresh_represented_talent_points_like_cpp,
    remove_represented_active_rewarded_duplicates_like_cpp,
    represented_can_reward_quest_inventory_like_cpp, represented_creature_gossip_text_like_cpp,
    represented_creature_quest_menu_items_like_cpp, represented_player_quest_status_like_cpp,
    represented_quest_giver_accept_source_allows_quest_like_cpp,
    represented_quest_giver_involved_source_allows_quest_like_cpp,
    represented_quest_giver_status_query_source_like_cpp, represented_quest_is_important_like_cpp,
    represented_quest_login_aura_sources_are_hit_inert_like_cpp,
    represented_quest_package_choice_matches_like_cpp,
    represented_reward_choice_matches_loaded_type_like_cpp,
    represented_reward_choice_template_exists_like_cpp,
    represented_spell_area_quest_status_like_cpp, reset_seasonal_quest_status_like_cpp,
    save_changed_represented_quest_statuses_like_cpp, save_quest_to_db,
    save_represented_quest_status_like_cpp,
    save_represented_quest_statuses_completed_after_like_cpp,
    send_represented_quest_giver_offer_reward_like_cpp,
    send_represented_quest_giver_query_quest_like_cpp,
    send_represented_quest_giver_quest_details_like_cpp,
    send_represented_quest_giver_request_items_with_completion_like_cpp,
    set_loaded_quest_completed_bit_like_cpp, set_represented_pending_quest_sharing_like_cpp,
    set_represented_quest_rewarded_like_cpp, store_quest_source_item_with_generator_like_cpp,
    use_represented_creature_questgiver_like_cpp, use_represented_gameobject_questgiver_like_cpp,
};
