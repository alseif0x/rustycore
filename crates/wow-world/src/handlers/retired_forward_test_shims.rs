// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the World split forwards and test-only
//! bodies retired in #1363 slice 12d. Each is the retired body verbatim (the
//! production host impls now name the split, the field or the free function),
//! so no scenario call site changes.
#![cfg(test)]

use std::collections::HashMap;

use wow_core::{ObjectGuid, Position};
use wow_data::is_player_meeting_condition_like_cpp;
use wow_entities::{Item, PlayerCurrency, PlayerInventoryItem as InventoryItem};
use wow_packet::packets::misc::DungeonDifficultySet;

use crate::session::{PlayerCurrencyDelta, WorldSession};

impl WorldSession {
    // Currencies (from `session/currency_adapter.rs`).
    pub(crate) fn set_player_currencies_like_cpp(
        &mut self,
        currencies: HashMap<u32, PlayerCurrency>,
    ) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.set_player_currencies_like_cpp(&mut hub, currencies)
    }
    pub(crate) fn player_currencies_like_cpp(&self) -> Option<HashMap<u32, PlayerCurrency>> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.player_currencies_like_cpp(hub)
    }
    // Instances (from `session/instances/{binding,difficulty}.rs`).
    pub(crate) fn player_is_locked_to_dungeon_encounter_like_cpp(
        &self,
        player_guid: ObjectGuid,
        dungeon_encounter_id: u32,
    ) -> Option<bool> {
        let (state, hub) = crate::session::split_instances_ref(self);
        state.player_is_locked_to_dungeon_encounter_like_cpp(hub, player_guid, dungeon_encounter_id)
    }
    pub(crate) fn replace_player_difficulty_preferences_like_cpp(
        &mut self,
        dungeon: u32,
        raid: u32,
        legacy_raid: u32,
    ) -> bool {
        let (state, mut hub) = crate::session::split_instances_mut(self);
        state.replace_player_difficulty_preferences_like_cpp(&mut hub, dungeon, raid, legacy_raid)
    }
    pub(crate) fn represented_dungeon_difficulty_packet_like_cpp(
        &self,
    ) -> Option<DungeonDifficultySet> {
        let (state, hub) = crate::session::split_instances_ref(self);
        state.represented_dungeon_difficulty_packet_like_cpp(hub)
    }
    pub(crate) fn represented_failed_map_difficulty_x_condition_like_cpp(
        &self,
        map_difficulty_id: u32,
    ) -> Option<u32> {
        let store = self.catalogs.maps.difficulty_x_condition_store.as_ref()?;
        let player_conditions = self.catalogs.player_condition_store.as_ref()?;
        let context = self.represented_player_condition_context_like_cpp()?;
        store.failed_condition_like_cpp(map_difficulty_id, player_conditions, |condition| {
            context
                .as_context(self)
                .is_some_and(|context| is_player_meeting_condition_like_cpp(condition, &context))
        })
    }
    pub(crate) fn set_player_zone_area_authority_complete_like_cpp(&mut self, complete: bool) {
        let (state, mut hub) = crate::session::split_instances_mut(self);
        state.set_player_zone_area_authority_complete_like_cpp(&mut hub, complete)
    }
    // Lifecycle (from `session/lifecycle/transfer_destination.rs`,
    // `session/persistence/{load,plans}.rs`).
    pub(crate) fn set_pending_teleport_like_cpp(
        &mut self,
        destination: Option<(u32, Position)>,
    ) -> bool {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.set_pending_teleport_like_cpp(&mut hub, destination)
    }
    pub(crate) fn load_character_reputation_rows_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = wow_progression::mgr::CharacterReputationRowLikeCpp>,
    ) -> bool {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.load_character_reputation_rows_like_cpp(&mut hub, rows)
    }
    pub(crate) fn mark_represented_talents_loaded_like_cpp(&mut self) {
        let _ = crate::session::hub_mut(self).mark_talents_loaded_like_cpp();
        wow_world_quest::refresh_represented_talent_points_like_cpp(self);
    }
    pub(crate) fn plan_add_currency_vendor_like_cpp(
        &self,
        currencies: &mut HashMap<u32, PlayerCurrency>,
        currency_id: u32,
        amount: u32,
    ) -> Result<Option<PlayerCurrencyDelta>, ()> {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.plan_add_currency_vendor_like_cpp(hub, currencies, currency_id, amount)
    }
    // Loot (from `session/loot/operations.rs`).
    pub(crate) fn resolved_pass_on_group_loot_like_cpp(&self) -> Option<bool> {
        let (state, hub) = crate::session::split_loot_ref(self);
        state.resolved_pass_on_group_loot_like_cpp(hub)
    }
    // Interaction (from `session/npc_interaction.rs`).
    pub(crate) fn player_interaction_source_guid_like_cpp(&self) -> Option<ObjectGuid> {
        let (state, hub) = crate::session::split_interaction_ref(self);
        state.player_interaction_source_guid_like_cpp(hub)
    }
    // Player items (from `session/player_binding.rs`, `session/player_items/*`).
    pub(crate) fn set_player_gold_like_cpp(&mut self, gold: u64) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.set_player_gold_like_cpp(&mut hub, gold)
    }
    pub(crate) fn represented_non_bank_item_count_like_cpp(&self, entry_id: u32) -> Option<u32> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_non_bank_item_count_like_cpp(hub, entry_id)
    }
    pub(crate) fn load_represented_equipment_set_row_like_cpp(
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
    pub(crate) fn apply_committed_new_inventory_item_at_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        inventory_item: InventoryItem,
        item_object: Item,
    ) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.apply_committed_new_inventory_item_at_like_cpp(
            &mut hub,
            bag,
            slot,
            inventory_item,
            item_object,
        )
    }
    pub(crate) fn register_loaded_inventory_item_duration_refs_like_cpp(
        &mut self,
        loaded_item_guids: &[ObjectGuid],
        loaded_equipped_item_guids: &[ObjectGuid],
    ) -> (
        Vec<wow_entities::PlayerItemTimeUpdate>,
        Vec<wow_entities::PlayerEnchantTimeUpdate>,
    ) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.register_loaded_inventory_item_duration_refs_like_cpp(
            &mut hub,
            loaded_item_guids,
            loaded_equipped_item_guids,
        )
    }
    pub(crate) fn insert_inventory_item_object(&mut self, item: Item) -> Option<Item> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.insert_inventory_item_object(&mut hub, item)
    }
    pub(crate) fn insert_inventory_item_like_cpp(
        &mut self,
        slot: u8,
        item: InventoryItem,
    ) -> Option<InventoryItem> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.insert_inventory_item_like_cpp(&mut hub, slot, item)
    }
    pub(crate) fn represented_inventory_item_counts_like_cpp(&self) -> Option<HashMap<u32, u32>> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_inventory_item_counts_like_cpp(hub)
    }
    pub(crate) fn resolved_inventory_items_like_cpp(&self) -> Option<HashMap<u8, InventoryItem>> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.resolved_inventory_items_like_cpp(hub)
    }
    pub(crate) fn resolved_inventory_item_like_cpp(&self, slot: u8) -> Option<InventoryItem> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.resolved_inventory_item_like_cpp(hub, slot)
    }
    pub(crate) fn resolved_inventory_item_object_like_cpp(&self, guid: ObjectGuid) -> Option<Item> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.resolved_inventory_item_object_like_cpp(hub, guid)
    }
    pub(crate) fn get_inventory_item_by_pos(&self, bag: u8, slot: u8) -> Option<InventoryItem> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.get_inventory_item_by_pos(hub, bag, slot)
    }
    // World entities (from `session/world_entities/gameobject.rs`).
    pub(crate) fn mutate_canonical_gameobject_by_guid_like_cpp<R>(
        &mut self,
        guid: ObjectGuid,
        f: impl FnOnce(&mut wow_entities::GameObject) -> R,
    ) -> Option<R> {
        let (state, mut hub) = crate::session::split_world_entities_mut(self);
        state.mutate_canonical_gameobject_by_guid_like_cpp(&mut hub, guid, f)
    }
    // Taxi (the scenario-only edge distance, from `session/taxi/operations.rs`).
    pub(crate) fn represented_taxi_edge_distance_like_cpp(
        &self,
        destination_has_required_team_flag: bool,
        destination_condition_id: u32,
        distance: u32,
    ) -> u32 {
        if !destination_has_required_team_flag {
            return u16::MAX as u32;
        }

        if !self.represented_meets_player_condition_id_like_cpp(destination_condition_id) {
            return u16::MAX as u32;
        }

        distance
    }
}
