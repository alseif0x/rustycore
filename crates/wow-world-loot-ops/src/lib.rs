// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot operations shared by World adapters (#1363 A7: moved from
//! `wow-world-application`).
//!
//! The loot handlers and the loot-release operations sit above
//! `wow-world-quest` (loot release reads quest eligibility, conditions and
//! the registry sync); `wow-world-loot` stays the loot state owner.
//!
//! `session_loot` holds the World-session loot bodies moved out of
//! `wow-world` behind [`LootSessionHostLikeCpp`] (#1363 slice 5a).

mod loot_handlers;
mod loot_release;
mod session_loot;
pub use loot_handlers::{
    LootHandlerCxLikeCpp, LootHandlerHostLikeCpp, register_loot_handlers_like_cpp,
};
pub use loot_release::{
    LootReleaseCxLikeCpp, direct_item_count_after_loot_release_like_cpp,
    durable_loot_item_fanout_viewers_like_cpp,
    queue_chest_gameobject_state_refresh_for_same_map_like_cpp,
    represented_gameobject_can_autostore_loot_item_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use session_loot::{
    DisenchantLootTemplateFrame, LOOT_MODE_DEFAULT_LIKE_CPP, LOOT_MODE_JUNK_FISH_LIKE_CPP,
    disenchant_loot_template_frame_like_cpp, generated_creature_loot_item_to_entry_like_cpp,
    generated_shared_gameobject_loot_item_to_entry_like_cpp,
    master_loot_error_for_inventory_result_like_cpp,
    represented_disenchant_loot_plain_row_can_roll_like_cpp,
    represented_disenchant_loot_reference_row_can_roll_like_cpp,
    represented_loot_object_guid_like_cpp,
};
pub use session_loot::{
    LootSessionHostLikeCpp, RepresentedGameObjectLootInstallObservationLikeCpp,
    creature_loot_release_values_for_viewer_like_cpp,
    ensure_represented_creature_kill_loot_like_cpp,
    evaluate_creature_loot_condition_for_player_like_cpp_representable,
    generate_loot_store_random_properties_with_rng_like_cpp,
    generate_represented_creature_loot_items_for_player_like_cpp,
    generate_represented_creature_loot_like_cpp,
    generate_represented_disenchant_loot_template_entries_like_cpp,
    generate_represented_gameobject_chest_loot_with_template_money_like_cpp,
    handle_sync_chest_gameobject_state_and_refresh_like_cpp,
    handle_sync_gathering_node_gameobject_state_and_refresh_like_cpp,
    handle_sync_goober_gameobject_state_and_refresh_like_cpp,
    has_incomplete_quest_objective_for_item_like_cpp,
    install_represented_creature_kill_loot_if_current_like_cpp,
    item_loot_quest_status_allows_for_player_like_cpp, item_loot_quest_status_allows_like_cpp,
    load_creature_item_template_addon_loot_metadata_like_cpp,
    mark_represented_master_loot_item_removed_like_cpp, next_canonical_loot_object_guid_like_cpp,
    next_represented_loot_object_guid_like_cpp,
    open_represented_fishing_hole_with_catalogs_like_cpp,
    open_represented_fishing_node_loot_with_catalogs_like_cpp,
    open_represented_gameobject_chest_with_template_money_like_cpp,
    open_represented_gathering_node_with_catalogs_like_cpp,
    prepare_owned_loot_authority_for_active_request_outcome_like_cpp,
    queue_goober_gameobject_state_refresh_for_same_map_like_cpp,
    reconcile_represented_loot_cache_like_cpp,
    record_represented_gameobject_chest_release_metadata_like_cpp,
    refresh_owned_loot_summary_like_cpp, refresh_represented_loot_owner_canonical_summary_like_cpp,
    represented_ae_loot_creature_targets_like_cpp, represented_creature_loot_state_like_cpp,
    represented_creature_position_for_loot_like_cpp,
    represented_gameobject_chest_group_state_like_cpp,
    represented_gameobject_loot_install_observation_like_cpp,
    represented_group_looters_at_reward_distance_like_cpp,
    represented_loot_authority_pools_like_cpp, represented_master_loot_can_store_error_like_cpp,
    represented_master_loot_candidate_list_like_cpp,
    represented_owned_loot_authority_outcome_like_cpp,
    store_claimed_direct_loot_item_from_owner_with_generator_like_cpp,
    sync_represented_creature_loot_to_canonical_like_cpp,
    upsert_represented_personal_gameobject_loot_authority_if_observed_like_cpp,
    upsert_represented_personal_gameobject_loot_authority_if_observed_with_empty_policy_like_cpp,
};
