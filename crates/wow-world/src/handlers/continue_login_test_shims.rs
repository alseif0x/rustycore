// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the continue-login hydration steps
//! moved with the coordinator (#1363 slice 13c): one-call method forwards to
//! the moved free functions the scenarios call, with this session as the
//! host, so no call site changes; and the one-line `WorldSession` split and
//! hub forwards the coordinator inlined that only the scenarios still call,
//! moved verbatim from `handlers/character/account/collections.rs`,
//! `session/instances/map_resolution.rs`,
//! `session/spell_state/cast.rs` (the first-login cast test entry), the
//! `session/persistence/load` F3 shim (the difficulty load forward),
//! `session/persistence/{load,load_authority}.rs`,
//! `session/player_items/{appearance,bank,enchantment,storage_slots}.rs`,
//! `session/progression/skills.rs` and
//! `session/spell_state/aura/spell_hit_authority.rs`.

#![cfg(test)]

use std::collections::{HashMap, HashSet};

use wow_core::ObjectGuid;
use wow_entities::MAX_POWERS_PER_CLASS;
use wow_packet::packets::misc::AccountMount;
use wow_world_core::session::PlayerBootstrapCatalogsLikeCpp;
use wow_world_core::session::RepresentedPlayerSkillLikeCpp;
use wow_world_entities::CreatureSpawnCatalogsLikeCpp;
use wow_world_inventory::LoadedEquippedItemEnchantmentsOutcomeLikeCpp;

use crate::session::WorldSession;

impl WorldSession {
    pub(crate) fn set_known_spells_like_cpp(&mut self, spells: Vec<i32>) {
        wow_world_spell::set_known_spells_like_cpp(self, spells)
    }
    pub(crate) fn set_represented_favorite_known_spells_like_cpp(
        &mut self,
        favorite_spells: HashSet<i32>,
    ) {
        wow_world_spell::set_represented_favorite_known_spells_like_cpp(self, favorite_spells)
    }
    pub(crate) fn apply_loaded_known_spell_dependencies_like_cpp(
        &mut self,
        known_spells: &mut Vec<i32>,
    ) -> usize {
        wow_world_spell::apply_loaded_known_spell_dependencies_like_cpp(self, known_spells)
    }
    pub(crate) fn apply_represented_start_all_spells_with_catalogs_like_cpp(
        &mut self,
        player_bootstrap: &PlayerBootstrapCatalogsLikeCpp,
        known_spells: &mut Vec<i32>,
    ) -> usize {
        wow_world_spell::apply_represented_start_all_spells_with_catalogs_like_cpp(
            self,
            player_bootstrap,
            known_spells,
        )
    }
    pub(crate) fn apply_loaded_spell_dependency_skills_like_cpp(
        &mut self,
        known_spells: &mut Vec<i32>,
        loaded_spell_side_effect_spells: &mut Vec<i32>,
    ) -> (usize, bool) {
        wow_world_spell::apply_loaded_spell_dependency_skills_like_cpp(
            self,
            known_spells,
            loaded_spell_side_effect_spells,
        )
    }
    pub(crate) fn apply_loaded_spell_dependencies_from_roots_like_cpp(
        &mut self,
        roots: &[i32],
        known_spells: &mut Vec<i32>,
    ) -> usize {
        wow_world_spell::apply_loaded_spell_dependencies_from_roots_like_cpp(
            self,
            roots,
            known_spells,
        )
    }
    pub(crate) async fn apply_represented_first_login_cast_spells_with_catalogs_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
        player_bootstrap: &PlayerBootstrapCatalogsLikeCpp,
    ) -> usize {
        wow_world_spell::apply_represented_first_login_cast_spells_with_catalogs_like_cpp(
            self,
            item_guid_generator,
            creature_spawn_catalogs,
            player_bootstrap,
        )
        .await
    }
    pub(crate) fn promote_loaded_character_mount_spells_like_cpp(
        &mut self,
        spells: &[i32],
    ) -> usize {
        wow_world_spell::promote_loaded_character_mount_spells_like_cpp(self, spells)
    }
    pub(crate) fn set_account_mounts_like_cpp(&mut self, mounts: Vec<AccountMount>) {
        wow_world_spell::set_account_mounts_like_cpp(self, mounts)
    }
    pub(crate) fn load_represented_explored_zones_like_cpp(&mut self, input: &str) -> usize {
        wow_world_instances::load_represented_explored_zones_like_cpp(self, input)
    }
    pub(crate) fn load_represented_character_titles_like_cpp(
        &mut self,
        known_titles: &str,
        chosen_title: u32,
    ) {
        wow_world_application::load_represented_character_titles_like_cpp(
            self,
            known_titles,
            chosen_title,
        )
    }
    pub(crate) fn login_known_spells_after_account_collections_like_cpp(&self) -> Vec<i32> {
        wow_world_application::login_known_spells_after_account_collections_like_cpp(self)
    }

    pub(crate) async fn load_account_toys_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.load_account_toys_like_cpp(&mut hub).await
    }

    pub(crate) async fn load_account_heirlooms_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.load_account_heirlooms_like_cpp(&mut hub).await
    }
    /// The login and battleground-fallback map entry and the cross-crate
    /// fixture call it as a session method; the body moved to
    /// `wow_world_instances` (#1363 slice 13a).
    pub(crate) fn ensure_canonical_world_map_for_current_player_like_cpp(
        &mut self,
    ) -> Option<wow_map::CreateMapDecision> {
        wow_world_instances::ensure_canonical_world_map_for_current_player_like_cpp(self)
    }
    pub(crate) fn clamp_loaded_player_xp_to_next_level_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.clamp_loaded_player_xp_to_next_level_like_cpp(&mut hub)
    }
    pub(crate) async fn load_player_account_data_like_cpp(&mut self, guid: ObjectGuid) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state
            .load_player_account_data_like_cpp(&mut hub, guid)
            .await
    }
    pub(crate) fn set_loaded_player_flags_like_cpp(&mut self, player_flags: u32) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.set_loaded_player_flags_like_cpp(&mut hub, player_flags)
    }
    pub(crate) fn set_loaded_player_flags_ex_like_cpp(&mut self, player_flags_ex: u32) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.set_loaded_player_flags_ex_like_cpp(&mut hub, player_flags_ex)
    }
    pub(crate) fn set_loaded_player_powers_like_cpp(
        &mut self,
        powers: [i32; MAX_POWERS_PER_CLASS],
    ) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.set_loaded_player_powers_like_cpp(&mut hub, powers)
    }
    pub(crate) async fn load_completed_achievements_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.load_completed_achievements_like_cpp(&mut hub).await
    }
    pub(crate) fn load_represented_account_item_appearances_like_cpp(
        &mut self,
        known_appearance_blocks: impl IntoIterator<Item = (u32, u32)>,
        favorite_appearances: impl IntoIterator<Item = u32>,
    ) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.load_represented_account_item_appearances_like_cpp(
            &mut hub,
            known_appearance_blocks,
            favorite_appearances,
        )
    }
    pub(crate) fn load_represented_account_transmog_illusions_like_cpp(
        &mut self,
        known_illusion_blocks: impl IntoIterator<Item = (u32, u32)>,
    ) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.load_represented_account_transmog_illusions_like_cpp(&mut hub, known_illusion_blocks)
    }
    pub(crate) fn set_player_bank_bag_slot_count_like_cpp(&mut self, count: u8) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.set_player_bank_bag_slot_count_like_cpp(&mut hub, count)
    }
    pub(crate) fn send_loaded_equipped_item_enchantment_updates_like_cpp(
        &self,
        outcome: &LoadedEquippedItemEnchantmentsOutcomeLikeCpp,
    ) {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.send_loaded_equipped_item_enchantment_updates_like_cpp(hub, outcome)
    }
    pub(crate) fn set_player_inventory_slot_count_like_cpp(&mut self, count: u8) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.set_player_inventory_slot_count_like_cpp(&mut hub, count)
    }
    pub(crate) fn replace_player_skill_records_like_cpp(
        &mut self,
        skill_records: HashMap<u16, RepresentedPlayerSkillLikeCpp>,
        loaded: bool,
        complete: bool,
    ) -> bool {
        crate::session::hub_mut(self).replace_player_skill_records_like_cpp(
            skill_records,
            loaded,
            complete,
        )
    }
    pub(crate) fn tombstone_player_spell_hit_aura_authority_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_spell_state_mut(self);
        state.tombstone_player_spell_hit_aura_authority_like_cpp(&mut hub)
    }
    #[cfg(test)]
    pub(crate) async fn apply_represented_first_login_cast_spells_like_cpp(&mut self) -> usize {
        let generators = self.id_generators_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        let player_bootstrap = self.player_bootstrap_catalogs_for_test_like_cpp();
        self.apply_represented_first_login_cast_spells_with_catalogs_like_cpp(
            generators.item.as_ref(),
            &creature_spawn_catalogs,
            &player_bootstrap,
        )
        .await
    }
}
