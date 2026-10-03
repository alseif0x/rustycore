#![cfg(any(test, feature = "test-fixtures"))]

use std::time::Instant;

use super::SessionLifecycleState;

impl SessionLifecycleState {
    pub fn set_tutorials_loaded_coherently_for_test_like_cpp(&mut self, loaded: bool) {
        self.tutorials_loaded_coherently_like_cpp = loaded;
    }

    pub fn set_account_data_time_for_test_like_cpp(&mut self, index: usize, time: i64) {
        self.account_data_like_cpp[index].time = time;
    }

    pub fn clear_session_account_state_port_for_test_like_cpp(&mut self) {
        self.persistence_ports_like_cpp
            .admission
            .session_account_state = None;
    }

    pub fn logout_time_for_test_like_cpp(&self) -> Option<Instant> {
        self.logout_time
    }

    pub fn set_logout_time_for_test_like_cpp(&mut self, logout_time: Option<Instant>) {
        self.logout_time = logout_time;
    }

    pub fn set_represented_loaded_player_flags_for_test_like_cpp(
        &mut self,
        player_flags: Option<u32>,
    ) {
        self.player_flags_test_fixture_like_cpp
            .represented_loaded_player_flags_like_cpp = player_flags;
    }

    pub fn represented_loaded_player_flags_for_test_like_cpp(&self) -> Option<u32> {
        self.player_flags_test_fixture_like_cpp
            .represented_loaded_player_flags_like_cpp
    }

    pub fn represented_loaded_player_flags_ex_for_test_like_cpp(&self) -> Option<u32> {
        self.player_flags_test_fixture_like_cpp
            .represented_loaded_player_flags_ex_like_cpp
    }

    pub fn set_represented_loaded_player_flags_applied_for_test_like_cpp(
        &mut self,
        applied: bool,
    ) {
        self.player_flags_test_fixture_like_cpp
            .represented_loaded_player_flags_applied_like_cpp = applied;
    }

    pub fn reset_loaded_player_flags_fixture_for_test_like_cpp(&mut self) {
        self.player_flags_test_fixture_like_cpp
            .represented_loaded_player_flags_like_cpp = None;
        self.player_flags_test_fixture_like_cpp
            .represented_loaded_player_flags_ex_like_cpp = None;
        self.player_flags_test_fixture_like_cpp
            .represented_loaded_player_flags_applied_like_cpp = false;
    }

    pub fn represented_at_login_flags_for_test_like_cpp(&self) -> u16 {
        self.represented_at_login_flags_like_cpp
    }

    pub fn set_represented_at_login_flags_for_test_like_cpp(&mut self, flags: u16) {
        self.represented_at_login_flags_like_cpp = flags;
    }

    pub fn record_at_login_flag_removal_for_test_like_cpp(
        &mut self,
        removal: crate::RepresentedAtLoginFlagRemovalLikeCpp,
    ) {
        self.represented_at_login_flag_removals_like_cpp
            .push(removal);
    }

    pub fn loaded_player_customizations_for_test_like_cpp(
        &self,
    ) -> &[wow_packet::packets::update::ChrCustomizationChoiceValuesUpdate] {
        self.loaded_player_customizations_like_cpp.as_slice()
    }

    pub fn pet_load_spells_are_empty_for_test_like_cpp(&self) -> bool {
        self.pet_load_query_holder_rows_like_cpp
            .spells_are_empty_like_cpp()
    }

    pub fn pet_load_spell_cooldowns_are_empty_for_test_like_cpp(&self) -> bool {
        self.pet_load_query_holder_rows_like_cpp
            .spell_cooldowns_are_empty_like_cpp()
    }

    pub fn pet_load_spell_charges_are_empty_for_test_like_cpp(&self) -> bool {
        self.pet_load_query_holder_rows_like_cpp
            .spell_charges_are_empty_like_cpp()
    }

    pub fn pet_load_auras_are_empty_for_test_like_cpp(&self) -> bool {
        self.pet_load_query_holder_rows_like_cpp
            .auras_are_empty_like_cpp()
    }

    pub fn pet_load_aura_effects_are_empty_for_test_like_cpp(&self) -> bool {
        self.pet_load_query_holder_rows_like_cpp
            .aura_effects_are_empty_like_cpp()
    }

    pub fn pet_load_declined_names_are_empty_for_test_like_cpp(&self) -> bool {
        self.pet_load_query_holder_rows_like_cpp
            .declined_names_are_empty_like_cpp()
    }
}
