// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the World forwards and bodies retired or
//! moved in #1363 slice 12c: one-call forwards (or the retired split forward
//! verbatim) for the names the scenarios call, so no scenario call site changes.
#![cfg(test)]

use crate::session::WorldSession;

impl WorldSession {
    // Rest flags (moved to `wow_world_instances::instance_session::rest_flags`).
    pub(crate) fn with_owned_player_mut_for_rest_like_cpp<R>(
        &self,
        f: impl FnOnce(&mut wow_entities::Player) -> R,
    ) -> Option<R> {
        wow_world_instances::with_owned_player_mut_for_rest_like_cpp(self, f)
    }
    pub(crate) fn set_represented_rest_flag_like_cpp(
        &mut self,
        rest_flag: u32,
        trigger_id: u32,
    ) -> bool {
        wow_world_instances::set_player_rest_flag_like_cpp(self, rest_flag, trigger_id)
    }
    pub(crate) fn remove_represented_rest_flag_like_cpp(&mut self, rest_flag: u32) -> bool {
        wow_world_instances::remove_player_rest_flag_like_cpp(self, rest_flag)
    }
    pub(crate) fn take_deferred_rest_flag_update_dirty_like_cpp(&mut self) -> bool {
        wow_world_instances::take_player_deferred_rest_flag_update_dirty_like_cpp(self)
    }
    pub(crate) fn send_represented_resting_player_flag_update_like_cpp(&self) -> bool {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.send_represented_resting_player_flag_update_like_cpp(hub)
    }
    // Player items (the fully-looted removal moved to `wow_world_loot_ops`).
    pub(crate) fn remove_fully_looted_runtime_item(
        &mut self,
        bag: u8,
        slot: u8,
        item_guid: wow_core::ObjectGuid,
    ) {
        wow_world_loot_ops::remove_fully_looted_runtime_item(self, bag, slot, item_guid)
    }
    pub(crate) fn remove_inventory_item_object(
        &mut self,
        item_guid: wow_core::ObjectGuid,
    ) -> Option<wow_entities::Item> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.remove_inventory_item_object(&mut hub, item_guid)
    }
    pub(crate) fn remove_inventory_item_like_cpp(
        &mut self,
        slot: u8,
    ) -> Option<wow_entities::PlayerInventoryItem> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.remove_inventory_item_like_cpp(&mut hub, slot)
    }
    // Cast state (split forwards retired from `session/player_cast/state.rs`).
    pub(crate) fn mutate_cast_execution_like_cpp<R>(
        &mut self,
        f: impl FnOnce(&mut wow_entities::CastExecutionStateLikeCpp) -> R,
    ) -> Option<R> {
        let (state, mut hub) = crate::session::split_spell_state_mut(self);
        wow_world_application::mutate_cast_execution_like_cpp(&mut hub, state, f)
    }
    pub(crate) fn spell_last_cast_time_like_cpp(
        &self,
        spell_id: i32,
    ) -> Option<Option<std::time::Instant>> {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.spell_last_cast_time_like_cpp(hub, spell_id)
    }
    // Money (split forward retired from `session/money/operations.rs`).
    pub(crate) fn resolved_player_money_like_cpp(&self) -> Option<u64> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.resolved_player_money_like_cpp(hub)
    }
    // Quest state (the split seam retired from `session/quest/state.rs`).
    pub(crate) fn player_quest_gameplay_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerQuestGameplayState> {
        let (state, hub) = crate::session::split_quest_state_ref(self);
        wow_world_quest::player_quest_gameplay_snapshot_like_cpp(hub, state)
    }
    pub(crate) fn mutate_player_quest_gameplay_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut wow_entities::PlayerQuestGameplayState) -> R,
    ) -> Option<R> {
        let (state, hub) = crate::session::split_quest_state_mut(self);
        wow_world_quest::mutate_player_quest_gameplay_like_cpp(hub, state, mutate)
    }
    // World entities (split forward retired from `session/world_entities/creature_query.rs`).
    pub(crate) fn canonical_creature_access_like_cpp(
        &self,
        guid: wow_core::ObjectGuid,
    ) -> Option<crate::session::RepresentedCreatureAccessLikeCpp> {
        let (state, hub) = crate::session::split_world_entities_ref(self);
        state.canonical_creature_access_like_cpp(hub, guid)
    }
    // Player-item catalog reads (field forwards retired from `session/player_items`).
    pub(crate) fn item_storage_template(
        &self,
        item_id: u32,
    ) -> Option<wow_entities::ItemStorageTemplate> {
        self.catalogs.item_storage_template(item_id)
    }
    pub(crate) fn item_template_inventory_type(&self, item_id: u32) -> Option<u8> {
        crate::session::hub_ref(self).item_template_inventory_type(item_id)
    }
    // Retired one-call forwards (`session/player_presentation.rs`, `session/publication/operations.rs`).
    pub(crate) fn represented_mount_x_display_usable_like_cpp(
        &self,
        player_condition_id: u32,
    ) -> bool {
        self.represented_meets_player_condition_id_like_cpp(player_condition_id)
    }
    pub(crate) fn send_represented_cinematic_start_like_cpp(&mut self, cinematic_id: u32) {
        wow_world_character::send_represented_cinematic_start_like_cpp(
            &mut crate::session::hub_mut(self),
            cinematic_id,
        );
    }
    // Retired split forwards with scenario callers (`session/persistence/load.rs`, `session/lifecycle_ops`, `session/player_items`, `session/instances/map_key.rs`).
    pub(crate) fn represented_talents_loaded_like_cpp(&self) -> bool {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.represented_talents_loaded_like_cpp(hub)
    }
    pub(crate) fn upgrade_account_heirloom_like_cpp(
        &mut self,
        item_id: u32,
        cast_item: i32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.upgrade_account_heirloom_like_cpp(&mut hub, item_id, cast_item)
    }
    pub(crate) fn account_mount_rows_like_cpp(
        &self,
    ) -> Vec<wow_packet::packets::misc::AccountMount> {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.account_mount_rows_like_cpp(hub)
    }
    pub(crate) fn add_transmog_set_like_cpp(
        &mut self,
        transmog_set_id: u32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.add_transmog_set_like_cpp(&mut hub, transmog_set_id)
    }
    pub(crate) fn begin_player_equipment_inventory_authority_load_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.begin_player_equipment_inventory_authority_load_like_cpp(&mut hub)
    }
    pub(crate) fn create_map_db2_entries_like_cpp(
        &self,
        map_id: u32,
        difficulty_id: wow_map::Difficulty,
    ) -> Option<wow_instances::MapDb2Entries> {
        let (state, hub) = crate::session::split_instances_ref(self);
        state.create_map_db2_entries_like_cpp(hub, map_id, difficulty_id)
    }
    pub(crate) fn get_inventory_item_by_guid_like_cpp(
        &self,
        item_guid: wow_core::ObjectGuid,
    ) -> Option<(u8, u8, wow_entities::PlayerInventoryItem)> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.get_inventory_item_by_guid_like_cpp(hub, item_guid)
    }
    pub(crate) fn represented_empty_inventory_positions_like_cpp(&self) -> Option<Vec<(u8, u8)>> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_empty_inventory_positions_like_cpp(hub)
    }
    pub(crate) fn inventory_remove_enchantment_persistence_like_cpp(
        &self,
        item_guid: wow_core::ObjectGuid,
        clear_mainhand_only: bool,
    ) -> Option<(String, Vec<wow_constants::EnchantmentSlot>)> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.inventory_remove_enchantment_persistence_like_cpp(hub, item_guid, clear_mainhand_only)
    }
    pub(crate) fn refresh_inventory_item_enchantment_duration_refs_like_cpp(
        &mut self,
        item_guid: wow_core::ObjectGuid,
    ) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.refresh_inventory_item_enchantment_duration_refs_like_cpp(&mut hub, item_guid)
    }
    // Session-tree `_ref` split forwards retired in favour of `self.<field>.<op>(hub_ref(self), ..)` at the callers.
    pub(crate) fn pending_teleport_like_cpp(&self) -> Option<(u32, wow_core::Position)> {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.pending_teleport_like_cpp(hub)
    }
    pub(crate) fn player_equipment_inventory_authority_complete_like_cpp(&self) -> bool {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.player_equipment_inventory_authority_complete_like_cpp(hub)
    }
    pub(crate) fn resolved_inventory_item_objects_like_cpp(
        &self,
    ) -> Option<std::collections::HashMap<wow_core::ObjectGuid, wow_entities::Item>> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.resolved_inventory_item_objects_like_cpp(hub)
    }
    // Retired one-call forwards whose host impls name the target (`session/persistence/load.rs`, `session/pets/battle_pet.rs`, `session/spell_state/aura_application.rs`, `session/player_items/enchantment.rs`).
    pub(crate) fn player_loading(&self) -> Option<wow_core::ObjectGuid> {
        self.lifecycle.player_loading()
    }
    /// C++ `BattlePetMgr::HasJournalLock`.
    pub(crate) fn has_represented_battle_pet_journal_lock_like_cpp(&self) -> bool {
        wow_world_inventory::has_represented_battle_pet_journal_lock_like_cpp(
            crate::session::hub_ref(self),
            &self.lifecycle,
            cfg!(test),
        )
    }
    pub(crate) fn represented_battle_pet_like_cpp(
        &self,
        pet_guid: wow_core::ObjectGuid,
    ) -> Option<wow_world_core::session::RepresentedBattlePetDataLikeCpp> {
        wow_world_inventory::represented_battle_pet_like_cpp(
            crate::session::hub_ref(self),
            &self.lifecycle,
            cfg!(test),
            pet_guid,
        )
    }
    /// Remove an aura by slot and send SMSG_AURA_UPDATE.
    pub(crate) fn remove_aura(&mut self, slot: u8) -> Result<(), &'static str> {
        self.player_aura_application_cx_like_cpp()
            .remove_aura_like_cpp(slot)
    }
    pub(crate) fn apply_current_player_item_enchantment_plan_like_cpp(
        &mut self,
        item_guid: wow_core::ObjectGuid,
        slot: wow_constants::EnchantmentSlot,
        args: wow_entities::ApplyEnchantmentArgs,
    ) -> Option<wow_entities::ApplyEnchantmentPlan> {
        self.build_item_enchantment_handler_cx_like_cpp()
            .apply_current_player_item_enchantment_plan_like_cpp(item_guid, slot, args)
    }
}
