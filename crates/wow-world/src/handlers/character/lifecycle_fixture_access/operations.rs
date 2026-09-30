//! Feature-gated forwards to the real Character operation; no replacement logic.

use super::*;

impl WorldSession {
    pub fn character_abort_partial_login_sequence_for_test(
        &mut self,
    )  {
        self.abort_partial_login_sequence_like_cpp()
    }

    pub fn character_continue_login_after_grid_load_for_test(
        &mut self,
        guid: ObjectGuid,
        map_id: i32,
        instance_id: u32,
        outcome: Option<crate::session::PlayerGridLoadOutcomeLikeCpp>,
    ) -> bool {
        self.continue_login_after_grid_load_like_cpp(guid, map_id, instance_id, outcome)
    }

    pub async fn character_delete_invalid_character_homebind_for_test(
        &self,
        guid: ObjectGuid,
    )  {
        self.delete_invalid_character_homebind_like_cpp(guid).await
    }

    pub fn character_load_default_graveyard_homebind_for_test(
        &self,
        race: u8,
    ) -> Option<CharacterLoginLocationForTest> {
        self.load_default_graveyard_homebind_like_cpp(race).map(Into::into)
    }

    pub async fn character_load_map_corpse_data_for_test(
        &self,
        map_id: u16,
        instance_id: u32,
    ) -> MapCorpseLoadOutcomeForTest {
        self.load_map_corpse_data_like_cpp(map_id, instance_id).await.into()
    }

    pub async fn character_repair_character_homebind_for_test(
        &self,
        guid: ObjectGuid,
        race: u8,
        player_create_info: PlayerCreateInfoLikeCpp,
        create_mode: u8,
        first_login: bool,
    ) -> Option<CharacterLoginLocationForTest> {
        self.repair_character_homebind_like_cpp(guid, race, player_create_info, create_mode, first_login).await.map(Into::into)
    }

    pub async fn character_resolve_persisted_transport_login_for_test(
        &self,
        guid_low: u64,
        saved_map_id: u16,
        offset: Position,
    ) -> Option<PersistedTransportLoginForTest> {
        self.resolve_persisted_transport_login_like_cpp(guid_low, saved_map_id, offset).await.map(Into::into)
    }

    pub fn character_retry_login_at_homebind_for_test(
        &mut self,
        map_id: &mut i32,
        zone_id: &mut i32,
        position: &mut Position,
        homebind: CharacterLoginLocationForTest,
    ) -> bool {
        self.retry_login_at_homebind_like_cpp(map_id, zone_id, position, homebind.into())
    }

    pub async fn character_send_handle_player_login_packets_for_test(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        feature_policy: &SupportFeaturePolicyLikeCpp,
        guid: ObjectGuid,
        position: &Position,
        map_id: i32,
        account_mounts: &[AccountMount],
        motd: &str,
    ) -> bool {
        self.send_handle_player_login_packets_like_cpp(item_guid_generator, feature_policy, guid, position, map_id, account_mounts, motd).await
    }

    pub async fn character_send_initial_packets_before_add_to_map_for_test(
        &mut self,
        guid: ObjectGuid,
        _position: &Position,
        _map_id: i32,
        _zone_id: i32,
        homebind: CharacterLoginLocationForTest,
        known_spells: Vec<i32>,
        favorite_spells: Vec<i32>,
        spell_history_entries: Vec<SpellHistoryEntry>,
        spell_charge_entries: Vec<SpellChargeEntry>,
        action_buttons: [i64; 180],
        account_mounts: Vec<AccountMount>,
        updateobject_trace_enabled: bool,
    ) -> bool {
        self.send_initial_packets_before_add_to_map(guid, _position, _map_id, _zone_id, homebind.into(), known_spells, favorite_spells, spell_history_entries, spell_charge_entries, action_buttons, account_mounts, updateobject_trace_enabled).await
    }

    pub async fn character_send_login_sequence_for_test(
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
        homebind: CharacterLoginLocationForTest,
        persisted_transport_login: Option<PersistedTransportLoginForTest>,
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
        self.send_login_sequence(item_guid_generator, trait_node_entries, creature_spawn_catalogs, feature_policy, player_grid_loader, guid, race, class, sex, level, display_id, position, map_id, zone_id, homebind.into(), persisted_transport_login.map(Into::into), visible_items, inv_slots, item_creates, combat, current_power0, base_mana, known_spells, favorite_spells, spell_history_entries, spell_charge_entries, action_buttons, skill_info, account_mounts).await
    }

    pub async fn character_update_realm_characters_for_test(
        &self,
    )  {
        self.update_realm_characters().await
    }

    pub async fn character_enumerate_for_test(&mut self) {
        let policy = self.support_feature_policy_for_test_like_cpp();
        self.handle_enum_characters_with_policy_like_cpp(&policy).await;
    }

    pub async fn character_initial_packets_after_add_for_test(
        &mut self,
        guid: ObjectGuid,
        position: &Position,
        map_id: i32,
        updateobject_trace_enabled: bool,
    ) {
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.send_initial_packets_after_add_to_map_with_catalogs_like_cpp(
            &catalogs, guid, position, map_id, updateobject_trace_enabled,
        ).await;
    }

    pub async fn character_initial_world_states_for_test(
        &self,
        map_id: i32,
        area_id: u32,
    ) -> Vec<(i32, i32)> {
        self.test_load_initial_world_states_for_login_like_cpp(map_id, area_id).await
    }
}
