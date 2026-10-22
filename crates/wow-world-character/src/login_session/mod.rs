// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The character login load (#1363 slice 8d): world entry, the logout
//! request, the ordered login packet bursts before and after map insertion,
//! the login loaders, the inventory login load, the login recovery and
//! transport restore, the character lifecycle helpers and the post-add entry
//! zone; with #1363 slice 8e, the login data loaders (skill-rewarded spell
//! changes, spell history, trait configs). The gossip bodies extend the same
//! host (`crate::gossip_session`).
//!
//! C++ `WorldSession::HandlePlayerLogin`, `Player::LoadFromDB`,
//! `Player::SendInitialPacketsBeforeAddToMap`, `Map::AddPlayerToMap` and
//! `Player::SendInitialPacketsAfterAddToMap` run on the Player. Their Rust
//! bodies moved here from `wow-world` verbatim except for the mechanical
//! `self` -> `host` rewrite: each [`CharacterLoginHostLikeCpp`] method is the
//! same-named `WorldSession` operation, called at the exact point the body
//! called it before the move, and a method returning a future returns the
//! session's own future unchanged. The continue-login coordinator
//! (`WorldSession::handle_continue_player_login_with_catalogs_like_cpp`)
//! stays in the World and calls these functions.

use std::collections::{HashMap, HashSet};
use std::future::Future;
use wow_core::ObjectGuid;
use wow_data::TalentTabStore;
use wow_entities::PlayerHomebindLikeCpp as RepresentedHomebindLikeCpp;
use wow_entities::{
    PhaseShift, PlayerCurrency, PlayerEnchantTimeUpdate, PlayerItemTimeUpdate, SpellCastMetadata,
};
use wow_packet::packets::gossip::{ClientGossipText, Hello};
use wow_packet::packets::misc::DungeonDifficultySet;
use wow_packet::packets::spell::{SpellCastVisual, SpellTargetData};
use wow_progression::mgr::CharacterReputationRowLikeCpp;
use wow_world_core::session::SessionState;
use wow_world_entities::CreatureSpawnCatalogsLikeCpp;
use wow_world_interaction::InteractionState;
use wow_world_inventory::VendorSessionHostLikeCpp;
use wow_world_lifecycle::{
    FinalizationMode, FinalizationOutcome, FinalizationReport, SessionLifecycleState,
};
use wow_world_loot::LootState;
use wow_world_spell::{
    CharacterAuraEffectRowLikeCpp, CharacterAuraRowLikeCpp, RepresentedPlayerSpellLikeCpp,
    SessionSpellState,
};
use wow_world_visibility::VisibilityState;

use wow_world_core::session::{HubMut, HubRef};
use wow_world_entities::{CorpseHostLikeCpp, WorldEntitiesHostLikeCpp};

mod account_mounts;
mod action_buttons;
mod admission;
mod aura_loading;
mod cuf_profiles;
mod currency_loading;
mod default_skills;
mod entry_zone;
mod glyph_loading;
mod group_loading;
mod initial_packets;
mod inventory_load;
mod lifecycle;
mod login_data;
mod login_recovery;
mod login_sequence;
mod represented_load;
mod reputation_loading;
mod skill_loading;
mod spell_loading;
mod spell_map_finalization;
mod talent_loading;
mod transport_restore;
mod world_entry;

pub use account_mounts::account_mount_login_partial_rows_like_cpp;
pub use action_buttons::load_action_buttons_for_login_like_cpp;
pub use admission::{LoginAdmissionDataLikeCpp, load_login_admission_data_like_cpp};
pub use aura_loading::load_character_auras_for_login_like_cpp;
pub use cuf_profiles::load_cuf_profiles_for_login_like_cpp;
pub use currency_loading::load_character_currencies_for_login_like_cpp;
pub use default_skills::apply_default_skills_for_login_like_cpp;
pub use entry_zone::apply_post_add_zone_from_terrain_like_cpp;
pub use glyph_loading::load_character_glyphs_for_login_like_cpp;
pub use group_loading::load_group_membership_for_login_like_cpp;
pub use initial_packets::send_initial_packets_after_add_to_map_with_catalogs_like_cpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use initial_packets::{
    load_initial_world_states_for_login_like_cpp, send_initial_packets_before_add_to_map,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use inventory_load::void_storage_login_context_like_cpp;
pub use inventory_load::{LoginInventorySnapshotLikeCpp, load_inventory_for_login_like_cpp};
pub use lifecycle::{
    enqueue_character_rename_like_cpp, handle_hearth_and_resurrect, update_realm_characters,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use lifecycle::{
    load_default_graveyard_homebind_like_cpp, load_map_corpse_data_like_cpp,
    load_player_customizations_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use login_data::{
    load_active_player_trait_configs_like_cpp, skill_rewarded_quest_fallback_allowed_like_cpp,
};
pub use login_data::{
    load_character_spell_history_packets_like_cpp, skill_rewarded_spell_changes_for_login_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use login_recovery::continue_login_after_grid_load_like_cpp;
pub use login_recovery::{
    abort_partial_login_sequence_like_cpp, delete_invalid_character_homebind_like_cpp,
    repair_character_homebind_like_cpp, retry_login_at_homebind_like_cpp,
    seed_login_location_zone_area_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use login_sequence::resolve_persisted_transport_login_like_cpp;
pub use login_sequence::send_login_sequence;
pub use represented_load::{
    begin_represented_trait_config_authority_load_like_cpp,
    complete_represented_trait_config_authority_load_like_cpp,
    load_represented_group_by_db_store_id_like_cpp, load_represented_group_difficulties_like_cpp,
    load_represented_void_storage_row_like_cpp, mark_represented_void_storage_loaded_like_cpp,
    prepare_represented_void_storage_login_load_like_cpp,
};
pub use reputation_loading::load_character_reputation_for_login_like_cpp;
pub use skill_loading::load_character_skill_rows_for_login_like_cpp;
pub use spell_loading::{
    LoginSpellRowsLikeCpp, load_character_favorite_spells_for_login_like_cpp,
    load_character_spell_rows_for_login_like_cpp,
};
pub use spell_map_finalization::finalize_player_spell_map_for_login_like_cpp;
pub use talent_loading::load_character_talents_for_login_like_cpp;
pub use transport_restore::{
    restore_persisted_transport_for_login_like_cpp, set_player_transport_position_like_cpp,
};
pub use world_entry::{
    LogoutDenialLikeCpp, fallback_direct_login, handle_logout_request_with_generator_like_cpp,
    logout_admission_like_cpp, player_login_combat_stats_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use world_entry::{motd_lines_like_cpp, send_handle_player_login_packets_like_cpp};

pub trait CharacterLoginHostLikeCpp:
    VendorSessionHostLikeCpp + CorpseHostLikeCpp + WorldEntitiesHostLikeCpp
{
    fn lifecycle_mut(&mut self) -> &mut SessionLifecycleState;
    fn split_lifecycle_ref(&self) -> (&SessionLifecycleState, HubRef<'_>);
    fn split_spell_state_ref(&self) -> (&SessionSpellState, HubRef<'_>);
    fn split_spell_state_mut(&mut self) -> (&mut SessionSpellState, HubMut<'_>);
    fn visibility_mut(&mut self) -> &mut VisibilityState;
    fn split_visibility_mut(&mut self) -> (&mut VisibilityState, HubMut<'_>);
    fn split_interaction(&mut self) -> (&mut InteractionState, HubRef<'_>);
    /// The World `interaction` field inside the World crate's own `cfg(test)`
    /// build, `None` everywhere else: the gossip-option read's fixture step
    /// runs exactly where its former `#[cfg(test)]` block ran (#1363 slice 12b5).
    #[cfg(any(test, feature = "test-fixtures"))]
    fn interaction_for_test_like_cpp(&self) -> Option<&InteractionState> {
        None
    }
    fn loot(&self) -> &LootState;
    fn loot_mut(&mut self) -> &mut LootState;

    fn apply_loaded_inventory_item_collection_hooks_like_cpp(&mut self, item: &wow_entities::Item);
    fn cleanup_shared_runtime_state(&mut self) -> FinalizationOutcome;
    fn begin_represented_trait_config_authority_load_like_cpp(&mut self);
    fn clear_player_currencies_like_cpp(&mut self) -> bool;
    fn complete_represented_trait_config_authority_load_like_cpp(
        &mut self,
        configs: impl IntoIterator<Item = (i32, i32, i32, i32)>,
        entries_empty: bool,
    ) -> bool;
    #[allow(clippy::too_many_arguments)]
    fn ensure_login_player_controller_like_cpp(
        &mut self,
        guid: ObjectGuid,
        name: String,
        position: wow_core::Position,
        map_id: u16,
        race: u8,
        class: u8,
        level: u8,
        gender: u8,
    ) -> bool;
    #[allow(clippy::too_many_arguments)]
    fn execute_spell_with_visual_and_target_data_with_metadata_and_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
        spell_id: i32,
        target_guid: ObjectGuid,
        cast_id: ObjectGuid,
        spell_visual: SpellCastVisual,
        target_data: SpellTargetData,
        metadata: SpellCastMetadata,
    ) -> impl Future<Output = Result<(), &'static str>> + Send;
    fn finalize_session_with_generator_like_cpp(
        &mut self,
        mode: FinalizationMode,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
    ) -> impl Future<Output = FinalizationReport> + Send;
    /// The `CanFreeMove` block of C++ `HandleLogoutRequestOpcode`
    /// (`MiscHandler.cpp:276-284`): sit, root, `UNIT_FLAG_STUNNED`. The
    /// stand-state owner lives above this crate.
    fn begin_logout_wait_movement_like_cpp(&mut self);
    fn force_update_visibility_with_catalogs_like_cpp(
        &mut self,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    ) -> impl Future<Output = ()> + Send;
    fn handle_trainer_list(&mut self, hello: Hello) -> impl Future<Output = ()> + Send;
    fn handle_trainer_list_for_gossip_option_like_cpp(
        &mut self,
        hello: Hello,
        gossip_menu_id: u32,
        gossip_option_id: u32,
    ) -> impl Future<Output = ()> + Send;
    fn known_spells_like_cpp(&self) -> Vec<i32>;
    fn load_character_reputation_rows_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = CharacterReputationRowLikeCpp>,
    ) -> bool;
    fn load_represented_character_auras_like_cpp(
        &mut self,
        aura_rows: impl IntoIterator<Item = CharacterAuraRowLikeCpp>,
        effect_rows: impl IntoIterator<Item = CharacterAuraEffectRowLikeCpp>,
        timediff_secs: u32,
    ) -> usize;
    #[allow(clippy::too_many_arguments)]
    fn load_represented_equipment_set_row_like_cpp(
        &mut self,
        guid: u64,
        set_id: u32,
        set_name: String,
        set_icon: String,
        ignore_mask: u32,
        assigned_spec_index: i32,
        pieces: [ObjectGuid; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
    ) -> bool;
    fn load_represented_talent_row_with_spell_side_effects_like_cpp(
        &mut self,
        talent_tabs: &TalentTabStore,
        talent_id: u32,
        rank: u8,
        talent_group: u8,
        known_spells: &mut Vec<i32>,
        dependent_spells: &mut HashSet<i32>,
    ) -> bool;
    fn mark_represented_spell_acquisition_snapshot_complete_like_cpp(&mut self);
    fn load_represented_group_by_db_store_id_like_cpp(&mut self, db_store_id: u32) -> bool;
    fn mark_represented_talents_loaded_like_cpp(&mut self);
    fn next_represented_spell_cast_guid_like_cpp(&self, spell_id: i32) -> Option<ObjectGuid>;
    fn notify_other_players_visibility_changed_like_cpp(&self);
    fn player_currencies_like_cpp(&self) -> Option<HashMap<u32, PlayerCurrency>>;
    fn player_current_map_instanceable_like_cpp(&self) -> bool;
    fn player_interaction_source_guid_like_cpp(&self) -> Option<ObjectGuid>;
    /// The World `replace_player_difficulty_preferences_like_cpp` (the
    /// instances split), for the group-difficulty restore (#1363 slice 12b4).
    fn replace_player_difficulty_preferences_like_cpp(
        &mut self,
        dungeon: u32,
        raid: u32,
        legacy_raid: u32,
    ) -> bool;
    fn quest_log_create_entries_like_cpp(&self) -> Vec<(u32, u32, i64, [u16; 24])>;
    fn register_in_player_registry(&self);
    fn register_loaded_inventory_item_duration_refs_like_cpp(
        &mut self,
        loaded_item_guids: &[ObjectGuid],
        loaded_equipped_item_guids: &[ObjectGuid],
    ) -> (Vec<PlayerItemTimeUpdate>, Vec<PlayerEnchantTimeUpdate>);
    fn represented_creature_gossip_text_like_cpp(
        &self,
        creature_entry: u32,
    ) -> Vec<ClientGossipText>;
    fn represented_dependent_known_spells_like_cpp(&self) -> HashSet<i32>;
    fn represented_dungeon_difficulty_packet_like_cpp(&self) -> Option<DungeonDifficultySet>;
    fn represented_homebind_like_cpp(&self) -> Option<RepresentedHomebindLikeCpp>;
    fn reset_group_update_sequence_if_needed_like_cpp(&mut self) -> bool;
    fn reset_represented_talents_like_cpp(&mut self);
    fn resolved_player_flags_for_create_like_cpp(&self) -> Option<(u32, u32)>;
    fn send_battle_pet_journal_lock_status_like_cpp(&mut self) -> impl Future<Output = ()> + Send;
    fn send_contact_list_like_cpp(&mut self, flags: u32) -> impl Future<Output = ()> + Send;
    fn send_login_stat_update_with_represented_item_bonuses_like_cpp(&mut self);
    fn set_complete_represented_player_spell_rows_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = RepresentedPlayerSpellLikeCpp>,
    ) -> bool;
    fn set_owned_player_group_like_cpp(&mut self, membership: Option<(u64, u8)>) -> bool;
    /// The World `void_storage_is_unlocked_like_cpp` (the canonical player
    /// flag read), for the void-storage login load (#1363 slice 12b4).
    fn void_storage_is_unlocked_like_cpp(&self) -> bool;
    fn set_player_currencies_like_cpp(&mut self, currencies: HashMap<u32, PlayerCurrency>) -> bool;
    fn set_player_aura_authority_complete_like_cpp(&mut self, complete: bool) -> bool;
    fn set_complete_represented_spell_trait_definition_ids_like_cpp(
        &mut self,
        traits: impl IntoIterator<Item = (i32, i32)>,
    ) -> bool;
    fn set_player_guid(&mut self, guid: Option<ObjectGuid>);
    fn set_player_moved_unit_guid_like_cpp(&mut self, guid: ObjectGuid);
    fn set_player_transport_guid_like_cpp(&mut self, guid: Option<ObjectGuid>);
    fn set_player_zone_area_authority_complete_like_cpp(&mut self, complete: bool);
    fn set_state(&mut self, state: SessionState);
    fn should_send_init_transport_like_cpp(
        &self,
        transport_guid: ObjectGuid,
        transport_phase_shift: &PhaseShift,
    ) -> bool;
    fn stats_application_cx_like_cpp(
        &mut self,
    ) -> wow_world_inventory::CharacterStatsApplicationCxLikeCpp<'_>;
    fn sync_current_player_session_visibility_detection_like_cpp(&mut self);
    fn take_deferred_rest_flag_update_dirty_like_cpp(&mut self) -> bool;
    fn update_represented_item_level_area_based_scaling_with_publication_like_cpp(
        &mut self,
        publish: bool,
    ) -> Option<bool>;
    fn use_represented_creature_questgiver_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        creature_entry: u32,
    ) -> bool;
    fn update_zone_represented_without_rest_update_packet_like_cpp(
        &mut self,
        new_zone: u32,
        new_area: u32,
    ) -> bool;
}

#[cfg(test)]
#[path = "../../unit_tests/login_session_scans.rs"]
mod tests;
