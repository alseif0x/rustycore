// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character handler operations (C++ `CharacterHandler.cpp` and the login
//! support around it), moved out of `wow-world-application` (#1363 A5).
//!
//! This crate sits high: only `wow-world-application` and `wow-world` may
//! depend on it (`tools/architecture/wow_world_closure.py`).

pub mod character_creation;
pub mod character_enumeration;
mod character_handler_families;
mod character_handlers;
pub mod character_login_support;
mod character_query_handlers;
mod gossip_options;
mod gossip_session;
mod login_session;
mod session_init;
pub use session_init::send_session_init_packets_with_policy_like_cpp;
mod worldport_session;

// The exact `ApplicationCharacter` owner facade the handler-contract checker
// requires: the three canonical exports as one use item. The remaining
// application-level re-exports of the same owner module follow separately so
// the facade item stays exact.
pub use character_handlers::{
    CharDeleteStepLikeCpp, CharRenameStepLikeCpp, ConnectToFailedStepLikeCpp,
    CreateCharacterStepLikeCpp, PlayerLoginStepLikeCpp, initial_character_rest_state_like_cpp,
    parse_equipment_cache, send_represented_cinematic_start_like_cpp,
};
pub use character_handlers::{
    CharacterHandlerCxLikeCpp, CharacterHandlerHostLikeCpp, register_character_handlers_like_cpp,
};
pub use character_query_handlers::realm_query_response_like_cpp;
pub use character_query_handlers::{
    CharacterQueryHandlerCxLikeCpp, CharacterQueryHandlerHostLikeCpp,
    register_character_query_handlers_like_cpp,
};
// The character login load (#1363 slice 8d).
pub use gossip_options::{
    clear_player_gossip_options_like_cpp, player_gossip_option_like_cpp,
    replace_player_gossip_options_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use gossip_session::{
    GOSSIP_OPTION_ID_AUTO_TRAINER_LIKE_CPP, GOSSIP_OPTION_NPC_TRAINER_LIKE_CPP,
};
pub use gossip_session::{
    build_gossip_menu, handle_binder_activate_with_generator_like_cpp, handle_gossip_hello,
    handle_gossip_select_option, send_represented_creature_trainer_gossip_menu_like_cpp,
};
pub use login_session::{
    CharacterLoginHostLikeCpp, LoginAdmissionDataLikeCpp, LoginInventorySnapshotLikeCpp,
    LoginSpellRowsLikeCpp, abort_partial_login_sequence_like_cpp,
    apply_default_skills_for_login_like_cpp, apply_post_add_zone_from_terrain_like_cpp,
    begin_represented_trait_config_authority_load_like_cpp,
    complete_represented_trait_config_authority_load_like_cpp,
    delete_invalid_character_homebind_like_cpp, enqueue_character_rename_like_cpp,
    fallback_direct_login, finalize_player_spell_map_for_login_like_cpp,
    handle_hearth_and_resurrect, handle_logout_request_with_generator_like_cpp,
    load_action_buttons_for_login_like_cpp, load_character_auras_for_login_like_cpp,
    load_character_currencies_for_login_like_cpp,
    load_character_favorite_spells_for_login_like_cpp, load_character_glyphs_for_login_like_cpp,
    load_character_reputation_for_login_like_cpp, load_character_skill_rows_for_login_like_cpp,
    load_character_spell_history_packets_like_cpp, load_character_spell_rows_for_login_like_cpp,
    load_character_talents_for_login_like_cpp, load_cuf_profiles_for_login_like_cpp,
    load_group_membership_for_login_like_cpp, load_inventory_for_login_like_cpp,
    load_login_admission_data_like_cpp, load_represented_group_by_db_store_id_like_cpp,
    load_represented_group_difficulties_like_cpp, load_represented_void_storage_row_like_cpp,
    mark_represented_void_storage_loaded_like_cpp, player_login_combat_stats_like_cpp,
    prepare_represented_void_storage_login_load_like_cpp, repair_character_homebind_like_cpp,
    restore_persisted_transport_for_login_like_cpp, retry_login_at_homebind_like_cpp,
    seed_login_location_zone_area_like_cpp,
    send_initial_packets_after_add_to_map_with_catalogs_like_cpp, send_login_sequence,
    set_player_transport_position_like_cpp, skill_rewarded_spell_changes_for_login_like_cpp,
    update_realm_characters,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use login_session::{
    continue_login_after_grid_load_like_cpp, load_active_player_trait_configs_like_cpp,
    load_default_graveyard_homebind_like_cpp, load_initial_world_states_for_login_like_cpp,
    load_map_corpse_data_like_cpp, load_player_customizations_like_cpp, motd_lines_like_cpp,
    resolve_persisted_transport_login_like_cpp, send_handle_player_login_packets_like_cpp,
    send_initial_packets_before_add_to_map, skill_rewarded_quest_fallback_allowed_like_cpp,
    void_storage_login_context_like_cpp,
};
// The far-transfer completion (#1363 slice 13a).
pub use worldport_session::{
    WorldportHostLikeCpp, finish_worldport_native_before_disconnect_like_cpp,
    handle_world_port_response_with_catalogs_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use worldport_session::{
    begin_worldport_post_add_like_cpp, prepare_player_self_create_for_teleport_like_cpp,
    recover_rejected_worldport_like_cpp, send_player_self_create_for_teleport_like_cpp,
};
