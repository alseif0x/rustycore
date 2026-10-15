// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The continue-login coordinator's World side. Its body moved from
//! `world_entry/login.rs` (deleted with its `world_entry.rs` mount) to
//! `wow_world_application` (#1363 slice 13c); the session driver calls the
//! free function. This file keeps the method form the `wow-world/tests`
//! integration tests call, and the World adapter of its host: each method
//! forwards to the inherent `WorldSession` operation of the same name, and a
//! method returning a future returns that operation's own future unchanged.

use std::future::Future;

use wow_world_application::ContinueLoginHostLikeCpp;
use wow_world_inventory::InitialLoadedItemModsOutcomeLikeCpp;

use super::*;
use crate::session::PlayerRestRatePolicyLikeCpp;

impl WorldSession {
    /// Continue the player login after the instance socket is connected.
    pub async fn handle_continue_player_login_with_module_registry_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        modules: &wow_module_api::ModuleRegistry,
        creature_spawn_catalogs: &crate::session::CreatureSpawnCatalogsLikeCpp,
        player_bootstrap: &PlayerBootstrapCatalogsLikeCpp,
        player_rest_rates: &PlayerRestRatePolicyLikeCpp,
        progression: &crate::session::ProgressionCatalogsLikeCpp,
        feature_policy: &SupportFeaturePolicyLikeCpp,
        player_grid_loader: &crate::session::PlayerGridLoadResolverLikeCpp,
    ) {
        wow_world_application::handle_continue_player_login_with_module_registry_like_cpp(
            self,
            item_guid_generator,
            modules,
            creature_spawn_catalogs,
            player_bootstrap,
            player_rest_rates,
            progression,
            feature_policy,
            player_grid_loader,
        )
        .await
    }
}

impl ContinueLoginHostLikeCpp for WorldSession {
    fn apply_initial_loaded_item_mods_like_cpp(
        &mut self,
        loaded_equipped_item_guids: &[ObjectGuid],
    ) -> InitialLoadedItemModsOutcomeLikeCpp {
        WorldSession::apply_initial_loaded_item_mods_like_cpp(self, loaded_equipped_item_guids)
    }
    fn apply_loaded_known_spell_previous_rank_passive_auras_like_cpp(
        &mut self,
        known_spells: &[i32],
    ) -> usize {
        WorldSession::apply_loaded_known_spell_previous_rank_passive_auras_like_cpp(
            self,
            known_spells,
        )
    }
    fn apply_login_known_spell_proficiencies_like_cpp(&mut self, known_spells: &[i32]) -> usize {
        WorldSession::apply_login_known_spell_proficiencies_like_cpp(self, known_spells)
    }
    fn apply_login_passive_known_spell_auras_like_cpp(&mut self) -> usize {
        WorldSession::apply_login_passive_known_spell_auras_like_cpp(self)
    }
    fn apply_offline_xp_rest_bonus_with_policy_like_cpp(
        &mut self,
        policy: &PlayerRestRatePolicyLikeCpp,
        logout_time_secs: u64,
        now_secs: u64,
        was_logout_resting: bool,
    ) -> f32 {
        WorldSession::apply_offline_xp_rest_bonus_with_policy_like_cpp(
            self,
            policy,
            logout_time_secs,
            now_secs,
            was_logout_resting,
        )
    }
    fn apply_represented_login_spell_reset_if_needed_like_cpp(&mut self) -> bool {
        WorldSession::apply_represented_login_spell_reset_if_needed_like_cpp(self)
    }
    fn load_instance_time_restrictions_like_cpp(&mut self) -> impl Future<Output = ()> + Send {
        WorldSession::load_instance_time_restrictions_like_cpp(self)
    }
    fn load_player_quests(&mut self) -> impl Future<Output = ()> + Send {
        wow_world_quest::load_player_quests(self)
    }
    fn load_represented_xp_rest_bonus_like_cpp(&mut self, rest_state: u8, rest_bonus: f32) {
        WorldSession::load_represented_xp_rest_bonus_like_cpp(self, rest_state, rest_bonus)
    }
    fn set_loaded_player_identity_like_cpp(
        &mut self,
        map_id: u16,
        race: u8,
        class: u8,
        level: u8,
        gender: u8,
    ) {
        WorldSession::set_loaded_player_identity_like_cpp(self, map_id, race, class, level, gender)
    }
    fn set_represented_at_login_flags_like_cpp(&mut self, flags: u16) -> bool {
        WorldSession::set_represented_at_login_flags_like_cpp(self, flags)
    }
    fn set_represented_guild_id_like_cpp(&mut self, guild_id: u64) -> bool {
        WorldSession::set_represented_guild_id_like_cpp(self, guild_id)
    }
    fn sync_player_spell_hit_aura_authority_to_canonical_like_cpp(&mut self) -> Option<bool> {
        WorldSession::sync_player_spell_hit_aura_authority_to_canonical_like_cpp(self)
    }
}
