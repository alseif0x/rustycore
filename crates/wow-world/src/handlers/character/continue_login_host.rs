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

use wow_world_application::{
    ContinueLoginHostLikeCpp, PlayerLoginHookEventLikeCpp, apply_initial_loaded_item_mods_like_cpp,
};
use wow_world_inventory::InitialLoadedItemModsOutcomeLikeCpp;

use super::*;
use crate::session::PlayerRestRatePolicyLikeCpp;

impl WorldSession {
    /// Continue the player login after the instance socket is connected.
    pub async fn handle_continue_player_login_with_catalogs_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &crate::session::CreatureSpawnCatalogsLikeCpp,
        player_bootstrap: &PlayerBootstrapCatalogsLikeCpp,
        player_rest_rates: &PlayerRestRatePolicyLikeCpp,
        progression: &crate::session::ProgressionCatalogsLikeCpp,
        feature_policy: &SupportFeaturePolicyLikeCpp,
        player_grid_loader: &crate::session::PlayerGridLoadResolverLikeCpp,
    ) {
        wow_world_application::handle_continue_player_login_with_catalogs_like_cpp(
            self,
            item_guid_generator,
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
    /// The `player.login` hook port, dispatched by the World module adapter.
    fn player_login_hook_like_cpp(&mut self, event: &PlayerLoginHookEventLikeCpp) {
        self.module_hooks
            .player_login_like_cpp(self.core.packet_publication_access_like_cpp(), event);
    }
    fn player_login_hook_wanted_like_cpp(&self) -> bool {
        self.module_hooks
            .wants_like_cpp(wow_module_api::HookFamily::PlayerLogin)
    }
    fn apply_initial_loaded_item_mods_like_cpp(
        &mut self,
        loaded_equipped_item_guids: &[ObjectGuid],
    ) -> InitialLoadedItemModsOutcomeLikeCpp {
        apply_initial_loaded_item_mods_like_cpp(self, loaded_equipped_item_guids)
    }
    fn apply_aura_with_effect_mask_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        duration_ms: u32,
        aura_flags: u32,
        effect_mask: u32,
    ) -> Result<(), &'static str> {
        self.player_aura_application_cx_like_cpp()
            .apply_aura_with_effect_mask_like_cpp(
                spell_id,
                caster_guid,
                duration_ms,
                aura_flags,
                effect_mask,
            )
    }
    fn apply_login_known_spell_proficiencies_like_cpp(&mut self, known_spells: &[i32]) -> usize {
        wow_world_spell::apply_login_known_spell_proficiencies_like_cpp(self, known_spells)
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
        wow_world_application::apply_represented_login_spell_reset_if_needed_like_cpp(self)
    }
    fn load_instance_time_restrictions_like_cpp(&mut self) -> impl Future<Output = ()> + Send {
        WorldSession::load_instance_time_restrictions_like_cpp(self)
    }
    fn load_player_quests(&mut self) -> impl Future<Output = ()> + Send {
        wow_world_quest::load_player_quests(self)
    }
    fn set_represented_guild_id_like_cpp(&mut self, guild_id: u64) -> bool {
        wow_world_social::set_represented_guild_id_like_cpp(self, guild_id)
    }
    fn player_spell_hit_source_identity_complete_like_cpp(&self) -> bool {
        WorldSession::player_spell_hit_source_identity_complete_like_cpp(self)
    }
    fn resolved_represented_guild_id_like_cpp(&self) -> Option<u64> {
        WorldSession::resolved_represented_guild_id_like_cpp(self)
    }
}
