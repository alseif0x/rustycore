//! Feature-only Character progression operation forwards.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterAreaZoneCriterionForTest {
    EnterArea(u32),
    LeaveArea(u32),
    EnterTopLevelArea(u32),
    LeaveTopLevelArea(u32),
}

impl WorldSession {
    pub fn character_apply_offline_xp_rest_bonus_for_test(
        &mut self,
        logout_time_secs: u64,
        now_secs: u64,
        was_logout_resting: bool,
    ) -> f32 {
        {
            let policy = self.player_rest_rate_policy_for_test_like_cpp();
            self.apply_offline_xp_rest_bonus_with_policy_like_cpp(
                &policy, logout_time_secs, now_secs, was_logout_resting,
            )
        }
    }

    pub fn character_apply_represented_first_login_explored_zones_for_test(
        &mut self,
    ) -> usize {
        let player_bootstrap = self.player_bootstrap_catalogs_for_test_like_cpp();
        self.apply_represented_first_login_explored_zones_with_catalogs_like_cpp(&player_bootstrap)
    }

    pub fn character_canonical_player_pvp_flags_for_test(
        &self,
        guid: ObjectGuid,
    ) -> Option<UnitPvpFlags> {
        if self.player_guid() == Some(guid)
            && let Some(flags) =
                self.with_owned_player_like_cpp(|player| player.unit().pvp_flags_like_cpp())
        {
            return Some(flags);
        }
        if !self.character_lifecycle_handleless_fixture() {
            return None;
        }
        let map_id = u32::from(self.player_map_id_like_cpp());
        let manager = Arc::clone(self.canonical_map_manager.as_ref()?);
        let manager = manager.lock().ok()?;
        let mut result = None;
        manager.do_for_all_maps_with_map_id(map_id, |managed| {
            if result.is_none() {
                result = managed
                    .map()
                    .get_typed_player(guid)
                    .map(|player| player.unit().pvp_flags_like_cpp());
            }
        });
        result
    }

    pub fn character_load_represented_xp_rest_bonus_for_test(
        &mut self,
        rest_state: u8,
        rest_bonus: f32,
    ) {
        self.load_represented_xp_rest_bonus_like_cpp(rest_state, rest_bonus)
    }

    pub fn character_player_health_for_test(
        &self,
    ) -> u32 {
        self.resolved_player_vitals_like_cpp().unwrap().0
    }

    pub fn character_player_rest_state_snapshot_for_test(
        &self,
    ) -> Option<wow_entities::PlayerRestState> {
        self.player_rest_state_snapshot_like_cpp()
    }

    pub fn character_player_xp_for_test(
        &self,
    ) -> u32 {
        self.resolved_player_xp_like_cpp().expect("test Player progression owner must resolve")
    }

    pub fn character_player_zone_area_for_test(
        &self,
    ) -> Option<(u32, u32)> {
        self.player_zone_area_like_cpp()
    }

    pub fn character_represented_area_zone_criteria_for_test(
        &self,
    ) -> Vec<CharacterAreaZoneCriterionForTest> {
        self.represented_area_zone_criteria_like_cpp()
            .iter()
            .map(|criterion| match *criterion {
                RepresentedAreaZoneCriteriaLikeCpp::EnterArea(id) =>
                    CharacterAreaZoneCriterionForTest::EnterArea(id),
                RepresentedAreaZoneCriteriaLikeCpp::LeaveArea(id) =>
                    CharacterAreaZoneCriterionForTest::LeaveArea(id),
                RepresentedAreaZoneCriteriaLikeCpp::EnterTopLevelArea(id) =>
                    CharacterAreaZoneCriterionForTest::EnterTopLevelArea(id),
                RepresentedAreaZoneCriteriaLikeCpp::LeaveTopLevelArea(id) =>
                    CharacterAreaZoneCriterionForTest::LeaveTopLevelArea(id),
            })
            .collect()
    }

    pub fn character_represented_explored_zones_db_string_for_test(
        &self,
    ) -> Option<String> {
        self.represented_explored_zones_db_string_like_cpp()
    }

    pub fn character_represented_is_resting_for_test(
        &self,
    ) -> bool {
        self.resolved_is_resting_like_cpp().expect("test Player rest owner must resolve")
    }

    pub fn character_represented_player_power_values_for_test(
        &self,
    ) -> Option<[i32; MAX_POWERS_PER_CLASS]> {
        self.represented_player_power_values_like_cpp()
    }

    pub fn character_represented_using_pvp_item_levels_for_test(
        &self,
    ) -> bool {
        self.resolved_using_pvp_item_levels_like_cpp().expect("test Player PvP item-level owner must resolve")
    }

    pub fn character_represented_xp_rest_bonus_for_test(
        &self,
    ) -> f32 {
        self.resolved_xp_rest_bonus_like_cpp().expect("test Player rest owner must resolve")
    }

    pub fn character_represented_xp_rest_state_for_test(
        &self,
    ) -> u8 {
        self.resolved_xp_rest_state_like_cpp().expect("test Player rest owner must resolve")
    }

    pub fn character_send_represented_resting_player_flag_update_for_test(
        &self,
    ) -> bool {
        self.send_represented_resting_player_flag_update_like_cpp()
    }

    pub fn character_set_loaded_player_powers_for_test(
        &mut self,
        powers: [i32; MAX_POWERS_PER_CLASS],
    ) {
        self.set_loaded_player_powers_like_cpp(powers)
    }

    pub fn character_set_player_health_for_test(
        &mut self,
        health: u32,
        max_health: u32,
    ) {
        self.set_player_health_like_cpp(health, max_health)
    }

    pub fn character_set_player_next_level_xp_for_test(
        &mut self,
        xp: u32,
    ) -> bool {
        self.set_player_next_level_xp_like_cpp(xp)
    }

    pub fn character_set_player_xp_for_test(
        &mut self,
        xp: u32,
    ) -> bool {
        self.set_player_xp_like_cpp(xp)
    }

    pub fn character_set_player_zone_area_for_test(
        &mut self,
        zone_id: u32,
        area_id: u32,
    ) {
        self.set_player_zone_area_like_cpp(zone_id, area_id)
    }

    pub fn character_set_represented_rest_flag_for_test(
        &mut self,
        rest_flag: u32,
        trigger_id: u32,
    ) -> bool {
        self.set_represented_rest_flag_like_cpp(rest_flag, trigger_id)
    }

    pub fn character_take_deferred_rest_flag_update_dirty_for_test(
        &mut self,
    ) -> bool {
        self.take_deferred_rest_flag_update_dirty_like_cpp()
    }

    pub fn character_update_represented_online_xp_rest_bonus_for_test(
        &mut self,
        now_secs: u64,
    ) -> (f32, u8) {
        {
            let policy = self.player_rest_rate_policy_for_test_like_cpp();
            self.update_represented_online_xp_rest_bonus_with_policy_like_cpp(&policy, now_secs)
        }
    }

    pub fn character_update_zone_represented_for_test(
        &mut self,
        new_zone: u32,
        new_area: u32,
    ) -> bool {
        self.update_zone_represented_like_cpp(new_zone, new_area)
    }

    pub fn character_update_zone_represented_without_rest_update_packet_for_test(
        &mut self,
        new_zone: u32,
        new_area: u32,
    ) -> bool {
        self.update_zone_represented_without_rest_update_packet_like_cpp(new_zone, new_area)
    }

    pub fn character_rest_time_for_test(&self) -> u64 {
        self.rest_mgr_test_fixture_like_cpp.represented_rest_time_secs_like_cpp
    }

    pub fn character_rest_flags_for_test(&self) -> u32 {
        self.rest_mgr_test_fixture_like_cpp.represented_rest_flag_mask_like_cpp
    }
}
