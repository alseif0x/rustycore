// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn account_heirloom_rows_like_cpp(&self) -> Vec<(u32, u32)> {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.account_heirloom_rows_like_cpp(hub)
    }
    #[cfg(test)]
    pub(crate) fn account_heirloom_bonus_like_cpp(&self, item_id: u32) -> u32 {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.account_heirloom_bonus_like_cpp(hub, item_id)
    }
    pub(crate) fn account_heirloom_packet_rows_like_cpp(&self) -> Vec<AccountHeirloom> {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.account_heirloom_packet_rows_like_cpp(hub)
    }
    pub(crate) fn account_toy_rows_like_cpp(&self) -> Vec<(u32, bool, bool)> {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.account_toy_rows_like_cpp(hub)
    }
    pub(crate) fn account_toy_packet_rows_like_cpp(&self) -> Vec<AccountToy> {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.account_toy_packet_rows_like_cpp(hub)
    }
    pub(crate) fn set_account_data_like_cpp(
        &mut self,
        data_type: u8,
        time: i64,
        data: String,
    ) -> bool {
        self.lifecycle
            .set_account_data_like_cpp(data_type, time, data)
    }
    #[cfg(test)]
    pub(crate) fn represented_at_login_flag_removals_like_cpp(
        &self,
    ) -> &[RepresentedAtLoginFlagRemovalLikeCpp] {
        self.lifecycle.represented_at_login_flag_removals_like_cpp()
    }
    pub(crate) fn check_account_heirloom_upgrades_like_cpp(
        &mut self,
        item_id: u32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        crate::session::cx_lifecycle(self).check_account_heirloom_upgrades_like_cpp(item_id)
    }
    pub(crate) fn account_data_times_like_cpp(
        &self,
        player_guid: ObjectGuid,
        mask: u32,
    ) -> wow_packet::packets::misc::AccountDataTimes {
        self.lifecycle
            .account_data_times_like_cpp(player_guid, mask)
    }
}
