//! Represented quest log state at the Session boundary.
//!
//! Moved out of the Session root under #605. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn player_quest_gameplay_snapshot_like_cpp(
        &self,
    ) -> Option<PlayerQuestGameplayState> {
        let (state, hub) = crate::session::split_quest_state_ref(self);
        wow_world_quest::player_quest_gameplay_snapshot_like_cpp(hub, state)
    }

    /// The retained projection.
    ///
    /// #756 moved the quest state and its invariants to the canonical Player
    /// and replaced every single-transition caller with the named operations
    /// above. What remains are the catalog-driven walks over the whole quest
    /// log — objective progress and quest-slot compaction — which C++ performs
    /// in `Player::AdjustQuestObjectiveProgress` (`Player.cpp:15874`) and
    /// `SetQuestObjectiveData` (`:16426`) while holding the Player. Their rules
    /// need quest templates and objectives, which may not enter `wow-entities`,
    /// so they keep this borrow rather than gaining a differently named generic
    /// closure. **Exit condition:** they retire with the objective-progress
    /// operation contract of #41, which owns that complete operation.
    pub(crate) fn mutate_player_quest_gameplay_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut PlayerQuestGameplayState) -> R,
    ) -> Option<R> {
        let (state, hub) = crate::session::split_quest_state_mut(self);
        wow_world_quest::mutate_player_quest_gameplay_like_cpp(hub, state, mutate)
    }
    /// Set the quest store shared reference.
    pub fn set_quest_store(&mut self, store: Arc<wow_data::quest::QuestStore>) {
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.catalogs.quests.store = Some(store);
    }
    /// Set the QuestV2 store shared reference used for C++ quest unique-bit lookups.
    pub fn set_quest_v2_store(&mut self, store: Arc<QuestV2Store>) {
        self.catalogs.quests.v2_store = Some(store);
    }
    /// Set the QuestInfo store used by C++ Quest::GetQuestTag/IsImportant.
    pub fn set_quest_info_store(&mut self, store: Arc<QuestInfoStore>) {
        self.catalogs.quests.info_store = Some(store);
    }
    /// Set C++ `CONFIG_QUEST_LOW_LEVEL_HIDE_DIFF`.
    pub fn set_quest_low_level_hide_diff_like_cpp(&mut self, value: u32) {
        self.quest_state
            .set_quest_low_level_hide_diff_like_cpp(value);
    }
    /// Set C++ `CONFIG_QUEST_HIGH_LEVEL_HIDE_DIFF`.
    pub fn set_quest_high_level_hide_diff_like_cpp(&mut self, value: u32) {
        self.quest_state
            .set_quest_high_level_hide_diff_like_cpp(value);
    }
    /// Set the QuestXP store (loaded from QuestXP.db2).
    pub fn set_quest_xp_store(&mut self, store: Arc<wow_data::quest_xp::QuestXpStore>) {
        self.catalogs.quests.xp_store = Some(store);
    }
    pub fn set_min_quest_scaled_xp_ratio_like_cpp(&mut self, ratio: u32) {
        self.quest_state
            .set_min_quest_scaled_xp_ratio_like_cpp(ratio);
    }
    #[cfg(test)]
    pub(crate) fn represented_auction_replicate_requests_like_cpp(
        &self,
    ) -> &[RepresentedAuctionReplicateRequestLikeCpp] {
        self.inventory
            .represented_auction_replicate_requests_like_cpp()
    }
    pub(crate) fn request_represented_battleground_leave_like_cpp(&mut self) {
        #[cfg(test)]
        {
            self.fixtures
                .battleground
                .represented_battleground_leave_requests_like_cpp = self
                .fixtures
                .battleground
                .represented_battleground_leave_requests_like_cpp
                .saturating_add(1);
        }
    }
    #[cfg(test)]
    pub(crate) fn represented_resurrection_request_like_cpp(
        &self,
    ) -> Option<PlayerResurrectionRequestLikeCpp> {
        self.player_resurrection_state_snapshot_like_cpp()
            .and_then(|state| state.request)
    }
    #[cfg(test)]
    pub(crate) fn represented_duel_requests_like_cpp(&self) -> &[RepresentedDuelRequestedLikeCpp] {
        self.social.represented_duel_requests_for_test_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_vehicle_seat_change_requests_like_cpp(
        &self,
    ) -> &[RepresentedVehicleSeatChangeRequestLikeCpp] {
        &self
            .fixtures
            .vehicles
            .represented_vehicle_seat_change_requests_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_vehicle_seat_spell_click_requests_like_cpp(
        &self,
    ) -> &[RepresentedVehicleSeatSpellClickRequestLikeCpp] {
        &self
            .fixtures
            .vehicles
            .represented_vehicle_seat_spell_click_requests_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_vehicle_enter_requests_like_cpp(
        &self,
    ) -> &[RepresentedVehicleEnterRequestLikeCpp] {
        &self
            .fixtures
            .vehicles
            .represented_vehicle_enter_requests_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_activate_taxi_requests_like_cpp(
        &self,
    ) -> &[wow_world_core::session::RepresentedActivateTaxiLikeCpp] {
        &self
            .fixtures
            .vehicles
            .represented_activate_taxi_requests_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_confirm_respec_wipe_requests_like_cpp(
        &self,
    ) -> &[RepresentedConfirmRespecWipeLikeCpp] {
        &self
            .fixtures
            .progression
            .represented_confirm_respec_wipe_requests_like_cpp
    }
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_adventure_map_start_quest_like_cpp(
        &mut self,
        request: RepresentedAdventureMapStartQuestLikeCpp,
    ) {
        #[cfg(test)]
        self.instances
            .record_represented_adventure_map_start_quest_for_test_like_cpp(request);
    }
    #[cfg(test)]
    pub(crate) fn represented_adventure_map_start_quest_requests_like_cpp(
        &self,
    ) -> &[RepresentedAdventureMapStartQuestLikeCpp] {
        self.instances
            .represented_adventure_map_start_quest_requests_for_test_like_cpp()
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/quest/state/f3_shims.rs"]
mod f3_shims;
