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
use wow_data::PlayerCreateInfoLikeCpp;
use wow_packet::packets::misc::{AccountMount, LogoutRequest, SpellChargeEntry, SpellHistoryEntry};
use wow_packet::packets::update::{ChrCustomizationChoiceValuesUpdate, PlayerCombatStats};
use wow_world_character::character_login_support::CharacterLoginLocationLikeCpp;
use wow_world_core::corpse_load::MapCorpseLoadOutcomeLikeCpp;
use wow_world_lifecycle::login_transport::PersistedTransportLoginLikeCpp;

use crate::session::{
    CreatureSpawnCatalogsLikeCpp, InventoryItem, SupportFeaturePolicyLikeCpp, WorldSession,
};

impl WorldSession {
    pub(crate) async fn handle_logout_request(&mut self, req: LogoutRequest) {
        let generators = self.id_generators_for_test_like_cpp();
        wow_world_character::handle_logout_request_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
            req,
        )
        .await;
    }

    pub(crate) async fn test_load_initial_world_states_for_login_like_cpp(
        &self,
        map_id: i32,
        player_area_id: u32,
    ) -> Vec<(i32, i32)> {
        wow_world_character::load_initial_world_states_for_login_like_cpp(
            self,
            map_id,
            player_area_id,
        )
        .await
    }

    pub(crate) async fn send_initial_packets_after_add_to_map(
        &mut self,
        guid: ObjectGuid,
        position: &Position,
        map_id: i32,
        updateobject_trace_enabled: bool,
    ) {
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        wow_world_character::send_initial_packets_after_add_to_map_with_catalogs_like_cpp(
            self,
            &catalogs,
            guid,
            position,
            map_id,
            updateobject_trace_enabled,
        )
        .await;
    }

    pub(crate) fn abort_partial_login_sequence_like_cpp(&mut self) {
        wow_world_character::abort_partial_login_sequence_like_cpp(self)
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

    pub(crate) async fn delete_invalid_character_homebind_like_cpp(&self, guid: ObjectGuid) {
        wow_world_character::delete_invalid_character_homebind_like_cpp(self, guid).await
    }

    pub(crate) async fn handle_hearth_and_resurrect(&mut self, pkt: wow_packet::WorldPacket) {
        wow_world_character::handle_hearth_and_resurrect(self, pkt).await
    }

    pub(crate) fn load_default_graveyard_homebind_like_cpp(
        &self,
        race: u8,
    ) -> Option<CharacterLoginLocationLikeCpp> {
        wow_world_character::load_default_graveyard_homebind_like_cpp(self, race)
    }

    pub(crate) async fn load_map_corpse_data_like_cpp(
        &self,
        map_id: u16,
        instance_id: u32,
    ) -> MapCorpseLoadOutcomeLikeCpp {
        wow_world_character::load_map_corpse_data_like_cpp(self, map_id, instance_id).await
    }

    pub(crate) async fn load_player_customizations_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Vec<ChrCustomizationChoiceValuesUpdate> {
        wow_world_character::load_player_customizations_like_cpp(self, guid).await
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

    pub(crate) async fn repair_character_homebind_like_cpp(
        &self,
        guid: ObjectGuid,
        race: u8,
        player_create_info: PlayerCreateInfoLikeCpp,
        create_mode: u8,
        first_login: bool,
    ) -> Option<CharacterLoginLocationLikeCpp> {
        wow_world_character::repair_character_homebind_like_cpp(
            self,
            guid,
            race,
            player_create_info,
            create_mode,
            first_login,
        )
        .await
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

    pub(crate) async fn send_handle_player_login_packets_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        feature_policy: &SupportFeaturePolicyLikeCpp,
        guid: ObjectGuid,
        position: &Position,
        map_id: i32,
        account_mounts: &[AccountMount],
        motd: &str,
    ) -> bool {
        wow_world_character::send_handle_player_login_packets_like_cpp(
            self,
            item_guid_generator,
            feature_policy,
            guid,
            position,
            map_id,
            account_mounts,
            motd,
        )
        .await
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

    pub(crate) async fn send_login_sequence(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        trait_node_entries: &wow_data::trait_tree::TraitNodeEntryStore,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
        feature_policy: &SupportFeaturePolicyLikeCpp,
        player_grid_loader: &crate::session::PlayerGridLoadResolverLikeCpp,
        guid: ObjectGuid,
        race: u8,
        class: u8,
        sex: u8,
        level: u8,
        display_id: u32,
        position: &Position,
        map_id: i32,
        zone_id: i32,
        homebind: CharacterLoginLocationLikeCpp,
        persisted_transport_login: Option<PersistedTransportLoginLikeCpp>,
        visible_items: [(i32, u16, u16); 19],
        inv_slots: [ObjectGuid; 141],
        item_creates: Vec<wow_packet::packets::update::ItemCreateData>,
        combat: PlayerCombatStats,
        current_power0: i32,
        base_mana: i32,
        known_spells: Vec<i32>,
        favorite_spells: Vec<i32>,
        spell_history_entries: Vec<SpellHistoryEntry>,
        spell_charge_entries: Vec<SpellChargeEntry>,
        action_buttons: [i64; 180],
        skill_info: Vec<(u16, u16, u16, u16, u16, i16, u16)>,
        account_mounts: Vec<AccountMount>,
    ) -> bool {
        wow_world_character::send_login_sequence(
            self,
            item_guid_generator,
            trait_node_entries,
            creature_spawn_catalogs,
            feature_policy,
            player_grid_loader,
            guid,
            race,
            class,
            sex,
            level,
            display_id,
            position,
            map_id,
            zone_id,
            homebind,
            persisted_transport_login,
            visible_items,
            inv_slots,
            item_creates,
            combat,
            current_power0,
            base_mana,
            known_spells,
            favorite_spells,
            spell_history_entries,
            spell_charge_entries,
            action_buttons,
            skill_info,
            account_mounts,
        )
        .await
    }

    pub(crate) async fn update_realm_characters(&self) {
        wow_world_character::update_realm_characters(self).await
    }

    pub(crate) fn apply_post_add_zone_from_terrain_like_cpp(
        &mut self,
        map_id: i32,
        position: &Position,
    ) -> bool {
        wow_world_character::apply_post_add_zone_from_terrain_like_cpp(self, map_id, position)
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
