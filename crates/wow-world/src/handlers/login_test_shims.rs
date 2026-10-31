// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the character login-load bodies moved to
//! `wow_world_character::login_session` (#1363 slice 8d). Each forwards to the
//! moved body with this session as its host, so the scenarios keep their method
//! syntax and the character hotspot's test lines do not grow. The three entry
//! points first came verbatim from the `cfg(test)` methods of
//! `handlers/character/{world_entry,world_entry/initial_packets,session_state}.rs`
//! (they now name the moved free functions); the forwards after them name the
//! moved free functions the scenarios call.
//!
//! The one-statement wrappers at the end came verbatim from
//! `session/{lifecycle_ops/operations,persistence/load,player_items/appearance,
//! player_items/equipment_sets,player_items/items,player_items/persistence_load}.rs`:
//! their only production callers were the moved bodies, which now make the same
//! split and state call through the host.

#![cfg(test)]

use wow_core::{ObjectGuid, Position};
use wow_packet::packets::misc::{AccountMount, LogoutRequest, SpellChargeEntry, SpellHistoryEntry};
use wow_packet::packets::update::PlayerCombatStats;
use wow_world_character::character_login_support::CharacterLoginLocationLikeCpp;
use wow_world_lifecycle::login_transport::PersistedTransportLoginLikeCpp;

use crate::session::{InventoryItem, WorldSession};

impl WorldSession {
    pub(crate) async fn handle_logout_request(&mut self, req: LogoutRequest) {
        let generators = self.id_generators_for_test_like_cpp();
        let creature_spawns = self.creature_spawn_catalogs_for_test_like_cpp();
        wow_world_character::handle_logout_request_with_catalogs(
            self,
            wow_world_entities::ResurrectionOperationCatalogs {
                item_guid_generator: generators.item.as_ref(),
                creature_spawns: &creature_spawns,
                graveyards: None,
            },
            req,
        )
        .await
    }

    pub(crate) async fn send_initial_packets_after_add_to_map(
        &mut self,
        guid: ObjectGuid,
        position: &Position,
        map_id: i32,
        updateobject_trace_enabled: bool,
    ) -> bool {
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        wow_world_character::send_initial_packets_after_add_to_map_with_catalogs_like_cpp(
            self,
            &catalogs,
            guid,
            position,
            map_id,
            updateobject_trace_enabled,
        )
        .await
    }

    pub(crate) fn continue_login_after_grid_load_like_cpp(
        &mut self,
        guid: ObjectGuid,
        map_id: i32,
        instance_id: u32,
        outcome: Option<crate::session::PlayerGridLoadOutcomeLikeCpp>,
    ) -> bool {
        wow_world_character::continue_login_after_grid_load_like_cpp(
            self,
            guid,
            map_id,
            instance_id,
            outcome,
        )
    }

    pub(crate) fn player_login_combat_stats_like_cpp(
        &mut self,
        race: u8,
        class: u8,
        level: u8,
        saved_health: Option<u32>,
        saved_power0: i32,
    ) -> Option<(PlayerCombatStats, i32, i32)> {
        wow_world_character::player_login_combat_stats_like_cpp(
            self,
            race,
            class,
            level,
            saved_health,
            saved_power0,
        )
    }

    pub(crate) async fn resolve_persisted_transport_login_like_cpp(
        &self,
        guid_low: u64,
        saved_map_id: u16,
        offset: Position,
    ) -> Option<PersistedTransportLoginLikeCpp> {
        wow_world_character::resolve_persisted_transport_login_like_cpp(
            self,
            guid_low,
            saved_map_id,
            offset,
        )
        .await
    }

    pub(crate) fn retry_login_at_homebind_like_cpp(
        &mut self,
        map_id: &mut i32,
        zone_id: &mut i32,
        position: &mut Position,
        homebind: CharacterLoginLocationLikeCpp,
    ) -> bool {
        wow_world_character::retry_login_at_homebind_like_cpp(
            self, map_id, zone_id, position, homebind,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn send_initial_packets_before_add_to_map(
        &mut self,
        guid: ObjectGuid,
        _position: &Position,
        _map_id: i32,
        _zone_id: i32,
        homebind: CharacterLoginLocationLikeCpp,
        known_spells: Vec<i32>,
        favorite_spells: Vec<i32>,
        spell_history_entries: Vec<SpellHistoryEntry>,
        spell_charge_entries: Vec<SpellChargeEntry>,
        action_buttons: [i64; 180],
        account_mounts: Vec<AccountMount>,
        updateobject_trace_enabled: bool,
    ) -> bool {
        wow_world_character::send_initial_packets_before_add_to_map(
            self,
            guid,
            _position,
            _map_id,
            _zone_id,
            homebind,
            known_spells,
            favorite_spells,
            spell_history_entries,
            spell_charge_entries,
            action_buttons,
            account_mounts,
            updateobject_trace_enabled,
        )
        .await
    }

    pub(crate) fn send_account_heirlooms_like_cpp(&self) {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.send_account_heirlooms_like_cpp(hub)
    }

    pub(crate) fn load_represented_cuf_profile_like_cpp(
        &mut self,
        id: u8,
        profile: wow_packet::packets::misc::CufProfile,
    ) -> bool {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.load_represented_cuf_profile_like_cpp(&mut hub, id, profile)
    }

    pub(crate) fn represented_load_cuf_profiles_packet_like_cpp(
        &self,
    ) -> Option<wow_packet::packets::misc::LoadCufProfiles> {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.represented_load_cuf_profiles_packet_like_cpp(hub)
    }

    pub(crate) fn set_loaded_player_customizations_like_cpp(
        &mut self,
        customizations: Vec<wow_packet::packets::update::ChrCustomizationChoiceValuesUpdate>,
    ) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.set_loaded_player_customizations_like_cpp(&mut hub, customizations)
    }

    pub(crate) fn send_favorite_appearances_like_cpp(&self) {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.send_favorite_appearances_like_cpp(hub)
    }

    pub(crate) fn mark_represented_equipment_sets_loaded_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.mark_represented_equipment_sets_loaded_like_cpp(&mut hub)
    }

    pub(crate) fn represented_load_equipment_set_packet_like_cpp(
        &self,
    ) -> Option<wow_packet::packets::misc::LoadEquipmentSet> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_load_equipment_set_packet_like_cpp(hub)
    }

    pub(crate) fn insert_buyback_item_like_cpp(
        &mut self,
        slot: u8,
        item: InventoryItem,
    ) -> Option<InventoryItem> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.insert_buyback_item_like_cpp(&mut hub, slot, item)
    }

    pub(crate) fn complete_player_equipment_inventory_authority_load_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.complete_player_equipment_inventory_authority_load_like_cpp(&mut hub)
    }
}
