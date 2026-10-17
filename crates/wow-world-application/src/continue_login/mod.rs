// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The continue-login coordinator (#1363 slice 13c).
//!
//! C++ `WorldSession::HandlePlayerLogin` after the instance socket connects:
//! `Player::LoadFromDB` in its load order, the login packet sequence, the
//! reset-at-login and first-login steps, the online mark and
//! `sScriptMgr->OnPlayerLogin`. Moved from the World
//! `handlers/character/world_entry/login.rs` verbatim except for the
//! mechanical `self` -> `host` rewrite: each [`ContinueLoginHostLikeCpp`]
//! method (and each supertrait method) is the same-named `WorldSession`
//! operation, called at the exact point the body called it before the move,
//! and a method returning a future returns the session's own future
//! unchanged. The loaders it calls are `wow_world_character`'s, the login
//! spell application `wow_world_spell`'s, the account appearance loads
//! `wow_world_inventory`'s and the explored-zones and difficulty loads
//! `wow_world_instances`'s.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::future::Future;
use std::sync::Arc;

use tracing::{info, warn};
use wow_constants::rest::REST_STATE_NORMAL_LIKE_CPP;
use wow_core::{ObjectGuid, Position};
use wow_entities::{INVENTORY_DEFAULT_SIZE, PlayerHomebindLikeCpp as RepresentedHomebindLikeCpp};
use wow_packet::packets::auth::ResumeComms;
use wow_packet::packets::update::PlayerCombatStats;
use wow_world_character::character_creation::{
    default_display_id, default_health_mana, restored_saved_health_like_cpp,
};
use wow_world_character::character_login_support::{
    battleground_login_fallback_location_like_cpp, usable_character_homebind_like_cpp,
};
use wow_world_character::{
    CharacterLoginHostLikeCpp, LoginAdmissionDataLikeCpp, LoginSpellRowsLikeCpp,
    abort_partial_login_sequence_like_cpp, apply_default_skills_for_login_like_cpp,
    delete_invalid_character_homebind_like_cpp, finalize_player_spell_map_for_login_like_cpp,
    load_action_buttons_for_login_like_cpp, load_character_auras_for_login_like_cpp,
    load_character_currencies_for_login_like_cpp,
    load_character_favorite_spells_for_login_like_cpp, load_character_glyphs_for_login_like_cpp,
    load_character_reputation_for_login_like_cpp, load_character_skill_rows_for_login_like_cpp,
    load_character_spell_history_packets_like_cpp, load_character_spell_rows_for_login_like_cpp,
    load_character_talents_for_login_like_cpp, load_cuf_profiles_for_login_like_cpp,
    load_group_membership_for_login_like_cpp, load_inventory_for_login_like_cpp,
    load_login_admission_data_like_cpp, player_login_combat_stats_like_cpp,
    repair_character_homebind_like_cpp, restore_persisted_transport_for_login_like_cpp,
    retry_login_at_homebind_like_cpp, seed_login_location_zone_area_like_cpp, send_login_sequence,
    skill_rewarded_spell_changes_for_login_like_cpp,
};
use wow_world_core::session::map_admission::PlayerGridLoadResolverLikeCpp;
use wow_world_core::session::{
    PlayerBootstrapCatalogsLikeCpp, PlayerRestRatePolicyLikeCpp, ProgressionCatalogsLikeCpp,
    SupportFeaturePolicyLikeCpp,
};
use wow_world_entities::CreatureSpawnCatalogsLikeCpp;
use wow_world_inventory::{BattlePetSessionHostLikeCpp, InitialLoadedItemModsOutcomeLikeCpp};
use wow_world_spell::login_spell_rules::{
    apply_skill_rewarded_spell_changes_to_login_like_cpp, favorite_known_spells_for_send_like_cpp,
    sync_loaded_fist_weapons_with_unarmed_like_cpp,
};
use wow_world_spell::{SpellExecutionHostLikeCpp, SpellbookHostLikeCpp};

use crate::SessionLifecycleHostLikeCpp;

mod hydration;
mod identity;
mod login;
mod passive_auras;
mod spell_reset;

pub use hydration::{
    PLAYER_FLAGS_GROUP_LEADER_LIKE_CPP, apply_represented_group_leader_flag_like_cpp,
    load_represented_character_titles_like_cpp,
    loaded_inventory_slot_count_with_legacy_rust_compat,
    login_known_spells_after_account_collections_like_cpp,
};
pub use identity::{gender_from_u8, set_loaded_player_identity_like_cpp};
pub use login::handle_continue_player_login_with_module_registry_like_cpp;
pub use passive_auras::{
    apply_loaded_known_spell_previous_rank_passive_auras_like_cpp,
    apply_login_passive_known_spell_auras_like_cpp,
};
pub use spell_reset::apply_represented_login_spell_reset_if_needed_like_cpp;

/// The World operations the moved continue-login coordinator still calls.
pub trait ContinueLoginHostLikeCpp:
    SessionLifecycleHostLikeCpp + SpellExecutionHostLikeCpp
{
    fn apply_initial_loaded_item_mods_like_cpp(
        &mut self,
        loaded_equipped_item_guids: &[ObjectGuid],
    ) -> InitialLoadedItemModsOutcomeLikeCpp;
    /// The World aura application (the aura-application context's
    /// `apply_aura_with_effect_mask_like_cpp`) the moved passive-aura replays call.
    fn apply_aura_with_effect_mask_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        duration_ms: u32,
        aura_flags: u32,
        effect_mask: u32,
    ) -> Result<(), &'static str>;
    fn apply_login_known_spell_proficiencies_like_cpp(&mut self, known_spells: &[i32]) -> usize;
    fn apply_offline_xp_rest_bonus_with_policy_like_cpp(
        &mut self,
        policy: &PlayerRestRatePolicyLikeCpp,
        logout_time_secs: u64,
        now_secs: u64,
        was_logout_resting: bool,
    ) -> f32;
    fn apply_represented_login_spell_reset_if_needed_like_cpp(&mut self) -> bool;
    fn load_instance_time_restrictions_like_cpp(&mut self) -> impl Future<Output = ()> + Send;
    fn load_player_quests(&mut self) -> impl Future<Output = ()> + Send;
    fn set_represented_guild_id_like_cpp(&mut self, guild_id: u64) -> bool;
    fn sync_player_spell_hit_aura_authority_to_canonical_like_cpp(&mut self) -> Option<bool>;
}
