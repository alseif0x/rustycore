//! Feature-only Character identity operation forwards.

use super::*;

impl WorldSession {
    pub const CHARACTER_GLOBAL_CACHE_MASK_FOR_TEST: u32 = GLOBAL_CACHE_MASK_LIKE_CPP;
    pub const CHARACTER_PER_CHARACTER_CACHE_MASK_FOR_TEST: u32 = PER_CHARACTER_CACHE_MASK_LIKE_CPP;
    pub const CHARACTER_ALL_ACCOUNT_DATA_CACHE_MASK_FOR_TEST: u32 = ALL_ACCOUNT_DATA_CACHE_MASK_LIKE_CPP;
    pub const CHARACTER_REST_FLAG_IN_TAVERN_FOR_TEST: u32 = REST_FLAG_IN_TAVERN_LIKE_CPP;
    pub const CHARACTER_REST_FLAG_IN_CITY_FOR_TEST: u32 = REST_FLAG_IN_CITY_LIKE_CPP;
    pub const CHARACTER_REST_FLAG_IN_FACTION_AREA_FOR_TEST: u32 = REST_FLAG_IN_FACTION_AREA_LIKE_CPP;
    pub const CHARACTER_PLAYER_FLAGS_RESTING_FOR_TEST: u32 = PLAYER_FLAGS_RESTING_LIKE_CPP;
    pub const CHARACTER_PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_FOR_TEST: u32 = PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP;

    pub fn character_account_data_times_for_test(
        &self,
        player_guid: ObjectGuid,
        mask: u32,
    ) -> wow_packet::packets::misc::AccountDataTimes {
        self.account_data_times_like_cpp(player_guid, mask)
    }

    pub fn character_complete_represented_trait_config_authority_load_for_test(
        &mut self,
        configs: impl IntoIterator<Item = (i32, i32, i32, i32)>,
        entries_empty: bool,
    ) -> bool {
        self.complete_represented_trait_config_authority_load_like_cpp(configs, entries_empty)
    }

    pub fn character_ensure_login_player_controller_for_test(
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
        self.ensure_login_player_controller_like_cpp(guid, name, position, map_id, race, class, level, gender)
    }

    pub fn character_fall_information_for_test(
        &self,
    ) -> (u32, f32) {
        self.resolved_fall_information_like_cpp().expect("test Player fall-information owner must resolve")
    }

    pub fn character_id_generators_for_test_for_test(
        &self,
    ) -> SessionIdGeneratorsLikeCpp {
        self.id_generators_for_test_like_cpp()
    }

    pub fn character_known_spells_for_test(
        &self,
    ) -> Vec<i32> {
        self.known_spells_like_cpp()
    }

    pub fn character_player_class_for_test(
        &self,
    ) -> u8 {
        self.player_class_like_cpp()
    }

    pub fn character_player_gender_for_test(
        &self,
    ) -> u8 {
        self.player_gender_like_cpp()
    }

    pub fn character_player_gold_for_test(
        &self,
    ) -> u64 {
        self.player_gold_like_cpp()
    }

    pub fn character_player_level_for_test(
        &self,
    ) -> u8 {
        self.player_level_like_cpp()
    }

    pub fn character_player_map_id_for_test(
        &self,
    ) -> u16 {
        self.player_map_id_like_cpp()
    }

    pub fn character_player_name_for_test(
        &self,
    ) -> Option<String> {
        self.player_name_like_cpp()
    }

    pub fn character_player_position_for_test(
        &self,
    ) -> Option<wow_core::Position> {
        self.player_position_like_cpp()
    }

    pub fn character_player_race_for_test(
        &self,
    ) -> u8 {
        self.player_race_like_cpp()
    }

    pub fn character_set_account_data_for_test(
        &mut self,
        data_type: u8,
        time: i64,
        data: String,
    ) -> bool {
        self.set_account_data_like_cpp(data_type, time, data)
    }

    pub fn character_set_equipment_set_guid_generator_for_test(
        &mut self,
        generator: Arc<EquipmentSetGuidGeneratorLikeCpp>,
    ) {
        self.set_equipment_set_guid_generator_like_cpp(generator)
    }

    pub fn character_set_fall_information_for_test(
        &mut self,
        time: u32,
        z: f32,
    ) -> bool {
        self.set_fall_information_like_cpp(time, z)
    }

    pub fn character_set_known_spells_for_test(
        &mut self,
        spells: Vec<i32>,
    ) {
        self.set_known_spells_like_cpp(spells)
    }

    pub fn character_set_loaded_player_identity_for_test(
        &mut self,
        map_id: u16,
        race: u8,
        class: u8,
        level: u8,
        gender: u8,
    ) {
        self.set_loaded_player_identity_like_cpp(map_id, race, class, level, gender)
    }

    pub fn character_set_player_game_master_for_test(
        &mut self,
        is_game_master: bool,
    ) {
        self.set_player_game_master_like_cpp(is_game_master)
    }

    pub fn character_set_player_gold_for_test(
        &mut self,
        gold: u64,
    ) -> bool {
        self.set_player_gold_like_cpp(gold)
    }

    pub fn character_set_represented_homebind_for_test(
        &mut self,
        homebind: wow_entities::PlayerHomebindLikeCpp,
    ) -> bool {
        self.set_represented_homebind_like_cpp(homebind)
    }

    pub fn character_set_taxi_flight_state_for_test(
        &mut self,
        current_node: wow_entities::PlayerTaxiFlightNodeLikeCpp,
        node_after_teleport: Option<wow_entities::PlayerTaxiFlightNodeLikeCpp>,
    ) {
        let _ = self.mutate_player_taxi_state_like_cpp(|taxi| {
            taxi.begin_taxi_flight_like_cpp(current_node, node_after_teleport);
        });
    }

    pub fn character_support_feature_policy_for_test_for_test(
        &self,
    ) -> SupportFeaturePolicyLikeCpp {
        self.support_feature_policy_for_test_like_cpp()
    }

    pub fn character_attach_player_controller_for_test(
        &mut self,
        guid: ObjectGuid,
        name: String,
        position: Position,
        map_id: u16,
        race: u8,
        class: u8,
        level: u8,
        gender: u8,
    ) {
        if !self.character_lifecycle_handleless_fixture() {
            return;
        }

        self.attach_player_controller_for_fixture(SessionPlayerController::new(
            guid, name, position, map_id, race, class, level, gender,
        ));
    }

    pub fn character_set_active_player_local_flags_for_test(&mut self, flags: u32) {
        let _ = self.mutate_active_player_update_state_like_cpp(|state| {
            state.active_local_flags = flags;
        });
        self.sync_current_player_session_visibility_detection_like_cpp();;
    }

    pub fn character_set_saved_identity_projection_for_test(&mut self, map_id: u16, level: u8) {
        self.current_map_id = map_id;
        self.player_level = level;
    }

    pub fn character_player_handle_for_test(&self) -> Option<wow_map::PlayerHandle> {
        self.player_handle_like_cpp
    }

    pub fn character_inn_trigger_for_test(&self) -> u32 {
        self.rest_mgr_test_fixture_like_cpp.represented_inn_area_trigger_id_like_cpp
    }

    pub fn character_fixture_progression_inputs_for_test(&self) -> (u32, u32, (u32, f32)) {
        (self.player_xp, self.player_next_level_xp,
            (self.last_fall_time_like_cpp, self.last_fall_z_like_cpp))
    }

    pub fn character_lifecycle_fixture_is_enabled_for_test(&self) -> bool {
        self.character_lifecycle_fixture_mode()
    }

    pub fn character_set_player_map_position_for_test(&mut self, map_id: u16, position: Position) {
        self.set_player_map_position_like_cpp(map_id, position);
    }
}
