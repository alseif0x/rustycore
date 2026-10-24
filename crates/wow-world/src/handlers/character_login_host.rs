// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The World adapter of the character login host (#1363 slice 8d; the login
//! data and gossip operations since slice 8e). Every
//! method names the inherent `WorldSession` operation by path, so none
//! resolves to its own trait method; a future is the session's own future,
//! returned unchanged. Moved out of `handlers/character` in #1363 slice 12d
//! (one adapter file per destination crate, outside the hotspot trees), where
//! the retired split forwards' bodies now sit.

use std::collections::{HashMap, HashSet};

use wow_core::ObjectGuid;
use wow_data::TalentTabStore;
use wow_entities::{PhaseShift, PlayerCurrency, PlayerEnchantTimeUpdate, PlayerItemTimeUpdate};
use wow_world_core::session::{HubMut, HubRef, SessionState};
use wow_world_interaction::InteractionState;

use wow_world_lifecycle::{
    FinalizationMode, FinalizationOutcome, FinalizationReport, SessionLifecycleState,
};
use wow_world_loot::LootState;
use wow_world_spell::{
    CharacterAuraEffectRowLikeCpp, CharacterAuraRowLikeCpp, RepresentedPlayerSpellLikeCpp,
    SessionSpellState,
};
use wow_world_visibility::VisibilityState;

use wow_packet::packets::gossip::{ClientGossipText, Hello};
use wow_packet::packets::misc::DungeonDifficultySet;
use wow_packet::packets::spell::{SpellCastVisual, SpellTargetData};
use wow_progression::mgr::CharacterReputationRowLikeCpp;

use crate::session::{
    CreatureSpawnCatalogsLikeCpp, RepresentedHomebindLikeCpp, SpellCastMetadata, WorldSession,
};

impl wow_world_character::CharacterLoginHostLikeCpp for WorldSession {
    fn lifecycle_mut(&mut self) -> &mut SessionLifecycleState {
        &mut self.lifecycle
    }
    fn split_lifecycle_ref(&self) -> (&SessionLifecycleState, HubRef<'_>) {
        crate::session::split_lifecycle_ref(self)
    }
    fn split_spell_state_ref(&self) -> (&SessionSpellState, HubRef<'_>) {
        crate::session::split_spell_state_ref(self)
    }
    fn split_spell_state_mut(&mut self) -> (&mut SessionSpellState, HubMut<'_>) {
        crate::session::split_spell_state_mut(self)
    }
    fn visibility_mut(&mut self) -> &mut VisibilityState {
        &mut self.visibility
    }
    fn split_visibility_mut(&mut self) -> (&mut VisibilityState, HubMut<'_>) {
        crate::session::split_visibility_mut(self)
    }
    fn split_interaction(&mut self) -> (&mut InteractionState, HubRef<'_>) {
        crate::session::split_interaction(self)
    }
    #[cfg(test)]
    fn interaction_for_test_like_cpp(&self) -> Option<&InteractionState> {
        Some(&self.interaction)
    }
    fn loot(&self) -> &LootState {
        &self.loot
    }
    fn loot_mut(&mut self) -> &mut LootState {
        &mut self.loot
    }

    fn apply_loaded_inventory_item_collection_hooks_like_cpp(&mut self, item: &wow_entities::Item) {
        let _ = crate::session::cx_lifecycle(self)
            .check_account_heirloom_upgrades_like_cpp(item.object().entry());
        let _ = wow_world_inventory::add_item_appearance_for_runtime_item_like_cpp(self, item);
    }
    fn cleanup_shared_runtime_state(&mut self) -> FinalizationOutcome {
        WorldSession::cleanup_shared_runtime_state(self)
    }
    fn begin_represented_trait_config_authority_load_like_cpp(&mut self) {
        wow_world_character::begin_represented_trait_config_authority_load_like_cpp(self)
    }
    fn clear_player_currencies_like_cpp(&mut self) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.set_player_currencies_like_cpp(&mut hub, HashMap::new())
    }
    fn complete_represented_trait_config_authority_load_like_cpp(
        &mut self,
        configs: impl IntoIterator<Item = (i32, i32, i32, i32)>,
        entries_empty: bool,
    ) -> bool {
        wow_world_character::complete_represented_trait_config_authority_load_like_cpp(
            self,
            configs,
            entries_empty,
        )
    }
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
    ) -> bool {
        WorldSession::ensure_login_player_controller_like_cpp(
            self, guid, name, position, map_id, race, class, level, gender,
        )
    }
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
    ) -> impl Future<Output = Result<(), &'static str>> + Send {
        wow_world_spell::execute_spell_with_visual_and_target_data_with_metadata_and_generator_like_cpp(
            self,
            item_guid_generator,
            creature_spawn_catalogs,
            spell_id,
            target_guid,
            cast_id,
            spell_visual,
            target_data,
            metadata,
        )
    }
    fn finalize_session_with_resurrection_catalogs(
        &mut self,
        mode: FinalizationMode,
        catalogs: wow_world_entities::ResurrectionOperationCatalogs<'_>,
    ) -> impl Future<Output = FinalizationReport> + Send {
        wow_world_application::finalize_session_with_resurrection_catalogs(
            self,
            mode,
            catalogs.item_guid_generator,
            catalogs,
        )
    }
    fn begin_logout_wait_movement_like_cpp(&mut self) {
        wow_world_application::begin_logout_wait_movement_like_cpp(self)
    }
    fn force_update_visibility_with_catalogs_like_cpp(
        &mut self,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    ) -> impl Future<Output = ()> + Send {
        wow_world_entities::force_update_visibility_with_catalogs_like_cpp(
            self,
            creature_spawn_catalogs,
        )
    }
    fn handle_trainer_list(&mut self, hello: Hello) -> impl Future<Output = ()> + Send {
        wow_world_application::handle_trainer_list(self, hello)
    }
    fn handle_trainer_list_for_gossip_option_like_cpp(
        &mut self,
        hello: Hello,
        gossip_menu_id: u32,
        gossip_option_id: u32,
    ) -> impl Future<Output = ()> + Send {
        wow_world_application::handle_trainer_list_for_gossip_option_like_cpp(
            self,
            hello,
            gossip_menu_id,
            gossip_option_id,
        )
    }
    fn known_spells_like_cpp(&self) -> Vec<i32> {
        wow_world_spell::known_spells_like_cpp(self)
    }
    fn load_character_reputation_rows_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = CharacterReputationRowLikeCpp>,
    ) -> bool {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.load_character_reputation_rows_like_cpp(&mut hub, rows)
    }
    fn load_represented_character_auras_like_cpp(
        &mut self,
        aura_rows: impl IntoIterator<Item = CharacterAuraRowLikeCpp>,
        effect_rows: impl IntoIterator<Item = CharacterAuraEffectRowLikeCpp>,
        timediff_secs: u32,
    ) -> usize {
        wow_world_spell::load_represented_character_auras_like_cpp(
            self,
            aura_rows,
            effect_rows,
            timediff_secs,
        )
    }
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
    ) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.load_represented_equipment_set_row_like_cpp(
            &mut hub,
            guid,
            set_id,
            set_name,
            set_icon,
            ignore_mask,
            assigned_spec_index,
            pieces,
        )
    }
    fn load_represented_group_by_db_store_id_like_cpp(&mut self, db_store_id: u32) -> bool {
        wow_world_character::load_represented_group_by_db_store_id_like_cpp(self, db_store_id)
    }
    fn load_represented_talent_row_with_spell_side_effects_like_cpp(
        &mut self,
        talent_tabs: &TalentTabStore,
        talent_id: u32,
        rank: u8,
        talent_group: u8,
        known_spells: &mut Vec<i32>,
        dependent_spells: &mut HashSet<i32>,
    ) -> bool {
        wow_world_spell::load_represented_talent_row_with_spell_side_effects_like_cpp(
            self,
            talent_tabs,
            talent_id,
            rank,
            talent_group,
            known_spells,
            dependent_spells,
        )
    }
    fn mark_represented_spell_acquisition_snapshot_complete_like_cpp(&mut self) {
        wow_world_spell::mark_represented_spell_acquisition_snapshot_complete_like_cpp(self)
    }
    fn mark_represented_talents_loaded_like_cpp(&mut self) {
        let _ = crate::session::hub_mut(self).mark_talents_loaded_like_cpp();
        wow_world_quest::refresh_represented_talent_points_like_cpp(self);
    }
    fn next_represented_spell_cast_guid_like_cpp(&self, spell_id: i32) -> Option<ObjectGuid> {
        wow_world_spell::next_represented_spell_cast_guid_like_cpp(self, spell_id)
    }
    fn notify_other_players_visibility_changed_like_cpp(&self) {
        wow_world_spell::notify_other_players_visibility_changed_like_cpp(self)
    }
    fn player_currencies_like_cpp(&self) -> Option<HashMap<u32, PlayerCurrency>> {
        self.inventory
            .player_currencies_like_cpp(crate::session::hub_ref(self))
    }
    fn player_current_map_instanceable_like_cpp(&self) -> bool {
        wow_world_spell::player_current_map_instanceable_like_cpp(self)
    }
    fn player_interaction_source_guid_like_cpp(&self) -> Option<ObjectGuid> {
        let (state, hub) = crate::session::split_interaction_ref(self);
        state.player_interaction_source_guid_like_cpp(hub)
    }
    fn replace_player_difficulty_preferences_like_cpp(&mut self, d: u32, r: u32, l: u32) -> bool {
        let (state, mut hub) = crate::session::split_instances_mut(self);
        state.replace_player_difficulty_preferences_like_cpp(&mut hub, d, r, l)
    }
    fn quest_log_create_entries_like_cpp(&self) -> Vec<(u32, u32, i64, [u16; 24])> {
        WorldSession::quest_log_create_entries_like_cpp(self)
    }
    fn register_in_player_registry(&self) {
        WorldSession::register_in_player_registry(self)
    }
    fn register_loaded_inventory_item_duration_refs_like_cpp(
        &mut self,
        loaded_item_guids: &[ObjectGuid],
        loaded_equipped_item_guids: &[ObjectGuid],
    ) -> (Vec<PlayerItemTimeUpdate>, Vec<PlayerEnchantTimeUpdate>) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.register_loaded_inventory_item_duration_refs_like_cpp(
            &mut hub,
            loaded_item_guids,
            loaded_equipped_item_guids,
        )
    }
    fn represented_creature_gossip_text_like_cpp(
        &self,
        creature_entry: u32,
    ) -> Vec<ClientGossipText> {
        wow_world_quest::represented_creature_gossip_text_like_cpp(self, creature_entry)
    }
    fn represented_dependent_known_spells_like_cpp(&self) -> HashSet<i32> {
        wow_world_spell::represented_dependent_known_spells_like_cpp(self)
    }
    fn represented_dungeon_difficulty_packet_like_cpp(&self) -> Option<DungeonDifficultySet> {
        self.instances
            .represented_dungeon_difficulty_packet_like_cpp(crate::session::hub_ref(self))
    }
    fn represented_homebind_like_cpp(&self) -> Option<RepresentedHomebindLikeCpp> {
        wow_world_spell::represented_homebind_like_cpp(self)
    }
    fn reset_group_update_sequence_if_needed_like_cpp(&mut self) -> bool {
        wow_world_instances::reset_group_update_sequence_if_needed_like_cpp(self)
    }
    fn reset_represented_talents_like_cpp(&mut self) {
        wow_world_spell::reset_represented_talents_like_cpp(self)
    }
    fn resolved_player_flags_for_create_like_cpp(&self) -> Option<(u32, u32)> {
        WorldSession::resolved_player_flags_for_create_like_cpp(self)
    }
    fn send_battle_pet_journal_lock_status_like_cpp(&mut self) -> impl Future<Output = ()> + Send {
        WorldSession::send_battle_pet_journal_lock_status_like_cpp(self)
    }
    fn send_contact_list_like_cpp(&mut self, flags: u32) -> impl Future<Output = ()> + Send {
        WorldSession::send_contact_list_like_cpp(self, flags)
    }
    fn send_login_stat_update_with_represented_item_bonuses_like_cpp(&mut self) {
        WorldSession::stats_application_cx_like_cpp(self)
            .send_login_stat_update_with_represented_item_bonuses_like_cpp();
    }
    fn set_complete_represented_player_spell_rows_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = RepresentedPlayerSpellLikeCpp>,
    ) -> bool {
        wow_world_spell::set_complete_represented_player_spell_rows_like_cpp(self, rows)
    }
    fn set_owned_player_group_like_cpp(&mut self, membership: Option<(u64, u8)>) -> bool {
        let (state, mut hub) = crate::session::split_social_mut(self);
        state.set_owned_player_group_like_cpp(&mut hub, membership)
    }
    fn void_storage_is_unlocked_like_cpp(&self) -> bool {
        WorldSession::void_storage_is_unlocked_like_cpp(self)
    }
    fn set_player_currencies_like_cpp(&mut self, currencies: HashMap<u32, PlayerCurrency>) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.set_player_currencies_like_cpp(&mut hub, currencies)
    }
    fn set_player_aura_authority_complete_like_cpp(&mut self, complete: bool) -> bool {
        wow_world_spell::set_player_aura_authority_complete_like_cpp(self, complete)
    }
    fn set_complete_represented_spell_trait_definition_ids_like_cpp(
        &mut self,
        traits: impl IntoIterator<Item = (i32, i32)>,
    ) -> bool {
        wow_world_spell::set_complete_represented_spell_trait_definition_ids_like_cpp(self, traits)
    }
    fn set_player_guid(&mut self, guid: Option<ObjectGuid>) {
        WorldSession::set_player_guid(self, guid)
    }
    fn set_player_moved_unit_guid_like_cpp(&mut self, guid: ObjectGuid) {
        WorldSession::set_player_moved_unit_guid_like_cpp(self, guid)
    }
    fn set_player_transport_guid_like_cpp(&mut self, guid: Option<ObjectGuid>) {
        WorldSession::set_player_transport_guid_like_cpp(self, guid)
    }
    fn set_player_zone_area_authority_complete_like_cpp(&mut self, complete: bool) {
        let (state, mut hub) = crate::session::split_instances_mut(self);
        state.set_player_zone_area_authority_complete_like_cpp(&mut hub, complete)
    }
    fn set_state(&mut self, state: SessionState) {
        WorldSession::set_state(self, state)
    }
    fn should_send_init_transport_like_cpp(
        &self,
        transport_guid: ObjectGuid,
        transport_phase_shift: &PhaseShift,
    ) -> bool {
        WorldSession::should_send_init_transport_like_cpp(
            self,
            transport_guid,
            transport_phase_shift,
        )
    }
    fn stats_application_cx_like_cpp(
        &mut self,
    ) -> wow_world_inventory::CharacterStatsApplicationCxLikeCpp<'_> {
        WorldSession::stats_application_cx_like_cpp(self)
    }
    fn sync_current_player_session_visibility_detection_like_cpp(&mut self) {
        wow_world_entities::sync_current_player_session_visibility_detection_like_cpp(self)
    }
    fn take_deferred_rest_flag_update_dirty_like_cpp(&mut self) -> bool {
        wow_world_instances::take_player_deferred_rest_flag_update_dirty_like_cpp(self)
    }
    fn update_represented_item_level_area_based_scaling_with_publication_like_cpp(
        &mut self,
        publish: bool,
    ) -> Option<bool> {
        wow_world_inventory::update_represented_item_level_area_based_scaling_with_publication_like_cpp(
            self, publish,
        )
    }
    fn use_represented_creature_questgiver_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        creature_entry: u32,
    ) -> bool {
        wow_world_quest::use_represented_creature_questgiver_like_cpp(
            self,
            creature_guid,
            creature_entry,
        )
    }
    fn update_zone_represented_without_rest_update_packet_like_cpp(
        &mut self,
        new_zone: u32,
        new_area: u32,
    ) -> bool {
        wow_world_instances::update_zone_represented_without_rest_update_packet_like_cpp(
            self, new_zone, new_area,
        )
    }
}
