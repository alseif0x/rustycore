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
use wow_world_loot_ops::LootSessionHostLikeCpp;
use wow_world_quest::QuestSessionHostLikeCpp;
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

/// The player-flags bit the war-mode zone-aura source reads.
pub const PLAYER_FLAGS_WAR_MODE_DESIRED_LIKE_CPP: u32 = 0x0000_0800;

/// The World operations the moved continue-login coordinator still calls.
pub trait ContinueLoginHostLikeCpp:
    SessionLifecycleHostLikeCpp + SpellExecutionHostLikeCpp + QuestSessionHostLikeCpp
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
    /// The identity-completeness predicate of the spell-hit authority proof
    /// (`WorldSession::player_spell_hit_source_identity_complete_like_cpp`).
    fn player_spell_hit_source_identity_complete_like_cpp(&self) -> bool;
    /// `WorldSession::resolved_represented_guild_id_like_cpp`.
    fn resolved_represented_guild_id_like_cpp(&self) -> Option<u64>;
    // The spell-hit authority proof (moved from the World
    // `session/spell_state/aura/spell_hit_authority.rs` and
    // `session/spell_state/aura_publication.rs`, #1363 final cleanup).
    fn can_authorize_empty_player_spell_hit_aura_source_for_difficulty_like_cpp(
        &self,
        difficulty_id: u8,
    ) -> bool
    where
        Self: Sized,
    {
        let Some(aura_subsystem) = self.hub_ref().player_aura_subsystem_snapshot_like_cpp() else {
            return false;
        };
        if !self.player_spell_hit_source_identity_complete_like_cpp()
            || aura_subsystem.spell_hit_aura_authority_tombstoned_like_cpp()
            || !aura_subsystem.persisted_player_aura_authority_complete_like_cpp()
            || !LootSessionHostLikeCpp::split_inventory_ref(self)
                .0
                .player_equipment_inventory_authority_complete_like_cpp(self.hub_ref())
            || self
                .resolved_represented_guild_id_like_cpp()
                .is_none_or(|guild_id| guild_id != 0)
            || self
                .hub_ref()
                .complete_player_skill_records_like_cpp()
                .is_none()
            || wow_world_spell::complete_represented_player_spell_rows_like_cpp(self).is_none()
            || wow_world_spell::complete_represented_spell_trait_definition_ids_like_cpp(self)
                .is_none_or(|traits| !traits.is_empty())
            || !wow_world_spell::represented_trait_config_aura_source_is_empty_like_cpp(self)
            || !wow_world_spell::represented_active_glyph_aura_source_is_empty_like_cpp(self)
            || !CharacterLoginHostLikeCpp::split_lifecycle_ref(self)
                .0
                .represented_battle_pet_login_spell_source_is_empty_like_cpp(self.hub_ref())
            || !wow_world_spell::represented_add_player_to_map_aura_source_is_empty_like_cpp(self)
            || !self
                .hub_ref()
                .catalogs
                .represented_auto_push_quest_aura_source_is_empty_like_cpp()
            || !self
                .hub_ref()
                .represented_character_pet_aura_source_is_empty_like_cpp()
            || !wow_world_quest::represented_quest_login_aura_sources_are_hit_inert_like_cpp(
                self,
                difficulty_id,
            )
            || !wow_world_spell::represented_spell_area_autocast_source_is_empty_like_cpp(self)
            || !self.represented_war_mode_update_zone_aura_source_is_empty_like_cpp()
            || !wow_world_spell::represented_update_area_pvp_rule_aura_source_is_empty_like_cpp(
                self,
            )
            || !wow_world_spell::represented_update_zone_script_aura_source_is_hit_inert_like_cpp(
                self,
            )
            || !aura_subsystem
                .runtime_applications_like_cpp()
                .values()
                .all(|aura| {
                    let (s, h) = SpellbookHostLikeCpp::split_spell_state_ref(self);
                    s.player_visible_aura_is_spell_hit_inert_like_cpp(h, aura)
                })
            || LootSessionHostLikeCpp::split_inventory_ref(self)
                .0
                .resolved_inventory_items_like_cpp(self.hub_ref())
                .is_none_or(|items| !items.is_empty())
            || LootSessionHostLikeCpp::split_inventory_ref(self)
                .0
                .resolved_buyback_items_like_cpp(self.hub_ref())
                .is_none_or(|items| !items.is_empty())
            || LootSessionHostLikeCpp::split_inventory_ref(self)
                .0
                .resolved_inventory_item_objects_like_cpp(self.hub_ref())
                .is_none_or(|items| !items.is_empty())
        {
            return false;
        }

        let known_spells = wow_world_spell::known_spells_like_cpp(self);
        if known_spells.is_empty() {
            return true;
        }
        let Some(spell_store) = self.hub_ref().catalogs.spell_catalogs.spell_store.as_ref() else {
            return false;
        };
        known_spells.iter().copied().all(|spell_id| {
            spell_id > 0
                && spell_store
                    .misc_attributes_for_difficulty_like_cpp(
                        spell_id,
                        difficulty_id,
                        self.hub_ref().catalogs.difficulty_store.as_deref(),
                    )
                    .is_some_and(|attributes| {
                        attributes[0] & wow_data::spell::attributes::SPELL_ATTR0_PASSIVE == 0
                            || u32::try_from(spell_id).ok().is_some_and(|spell_id| {
                                wow_world_spell::player_target_spell_is_hit_inert_like_cpp(
                                    self,
                                    spell_id,
                                    difficulty_id,
                                )
                            })
                    })
        })
    }
    fn can_authorize_empty_player_spell_hit_aura_source_like_cpp(&self) -> bool
    where
        Self: Sized,
    {
        self.can_authorize_empty_player_spell_hit_aura_source_for_difficulty_like_cpp(
            self.core().current_map_difficulty_id_like_cpp(),
        )
    }
    /// Publish the combined session-source proof to the canonical Player.
    /// Positive publication is reserved for explicit login/snapshot boundaries;
    /// individual source mutations call the invalidation helper instead.
    fn sync_player_spell_hit_aura_authority_to_canonical_like_cpp(&mut self) -> Option<bool>
    where
        Self: Sized,
    {
        let complete = self.can_authorize_empty_player_spell_hit_aura_source_like_cpp();
        self.core().mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .subsystems_mut()
                .auras
                .set_spell_hit_aura_authority_inert_like_cpp(complete);
            complete
        })
    }
    fn represented_war_mode_update_zone_aura_source_is_empty_like_cpp(&self) -> bool
    where
        Self: Sized,
    {
        wow_world_entities::represented_player_flags_value_like_cpp(self).is_some()
            && !wow_world_entities::represented_player_has_flag_like_cpp(
                self,
                PLAYER_FLAGS_WAR_MODE_DESIRED_LIKE_CPP,
            )
    }
}
