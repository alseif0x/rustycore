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
        wow_world_application::player_quest_gameplay_snapshot_like_cpp(hub, state)
    }

    pub(crate) fn clear_represented_resurrection_request_like_cpp(&mut self) -> bool {
        let mut hub = crate::session::hub_mut(self);
        let canonical = hub
            .core
            .with_owned_player_mut_like_cpp(Player::clear_resurrection_request_like_cpp)
            .is_some();
        #[cfg(test)]
        if canonical || hub.core.player_handle_like_cpp.is_none() {
            hub.fixtures
                .combat
                .represented_resurrection_request_like_cpp = None;
        }
        canonical || cfg!(test) && hub.core.player_handle_like_cpp.is_none()
    }
    /// C++ `Player::SetQuestStatus` (`Player.cpp:15557`) installing one record.
    pub(crate) fn insert_represented_quest_status_like_cpp(
        &mut self,
        quest_id: u32,
        status: wow_entities::PlayerQuestStatusRecord,
    ) -> bool {
        self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.insert_status_like_cpp(quest_id, status);
        })
        .is_some()
    }

    /// C++ `Player::RemoveActiveQuest` (`Player.cpp:15575`).
    pub(crate) fn remove_represented_quest_status_like_cpp(&mut self, quest_id: u32) -> bool {
        self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.remove_status_like_cpp(quest_id);
        })
        .is_some()
    }

    /// C++ `Player::m_RewardedQuests` gaining or losing one quest.
    pub(crate) fn set_represented_quest_rewarded_like_cpp(
        &mut self,
        quest_id: u32,
        rewarded: bool,
    ) -> bool {
        self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.set_rewarded_like_cpp(quest_id, rewarded);
        })
        .is_some()
    }

    /// Settle one rewarded quest: C++ `Player::RewardQuest` removes the active
    /// entry and records a non-repeatable quest as rewarded.
    pub(crate) fn settle_represented_rewarded_quest_like_cpp(
        &mut self,
        quest_id: u32,
        repeatable: bool,
    ) -> bool {
        self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.remove_status_like_cpp(quest_id);
            if !repeatable {
                state.set_rewarded_like_cpp(quest_id, true);
            }
        })
        .is_some()
    }

    /// C++ `Player::CompleteQuest` moving one incomplete quest to complete and
    /// reporting the status it replaced.
    pub(crate) fn complete_represented_quest_status_like_cpp(
        &mut self,
        quest_id: u32,
        incomplete_status: u8,
        complete_status: u8,
    ) -> Option<u8> {
        self.mutate_player_quest_gameplay_like_cpp(|state| {
            let status = state.status_mut_like_cpp(quest_id)?;
            (status.status == incomplete_status).then(|| {
                let old_status = status.status;
                status.status = complete_status;
                old_status
            })
        })
        .flatten()
    }

    /// Mark one quest explored, reporting whether the record existed and
    /// whether the client must be told.
    pub(crate) fn mark_represented_quest_explored_like_cpp(
        &mut self,
        quest_id: u32,
        failed_status: u8,
    ) -> Option<(bool, bool)> {
        self.mutate_player_quest_gameplay_like_cpp(|state| {
            let Some(status) = state.status_mut_like_cpp(quest_id) else {
                return (false, false);
            };
            let should_send = !status.explored && status.status != failed_status;
            if should_send {
                status.explored = true;
            }
            (true, should_send)
        })
    }

    /// Ensure one seasonal event exists, without changing its quests.
    pub(crate) fn ensure_represented_seasonal_event_like_cpp(&mut self, event_id: u16) -> bool {
        self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.ensure_seasonal_event_like_cpp(event_id);
        })
        .is_some()
    }

    /// Install the authoritative loaded quest statuses and rewarded sets
    /// (`Player::_LoadQuestStatus` and `_LoadQuestStatusRewarded`).
    pub(crate) fn install_represented_loaded_quest_statuses_like_cpp(
        &mut self,
        loaded: &wow_entities::PlayerQuestGameplayState,
    ) -> bool {
        self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.replace_statuses_like_cpp(
                loaded.statuses_snapshot_like_cpp(),
                loaded.status_authority_complete_like_cpp(),
            );
            state.replace_rewarded_quest_ids_like_cpp(loaded.rewarded_quest_ids_like_cpp().clone());
            state.replace_rewarded_quest_rows_like_cpp(
                loaded.rewarded_quest_rows_like_cpp().clone(),
            );
        })
        .is_some()
    }

    /// Install the loaded daily bucket and its timestamp.
    pub(crate) fn install_represented_loaded_daily_quests_like_cpp(
        &mut self,
        df_quest_ids: std::collections::BTreeSet<u32>,
        daily_quest_ids: std::collections::BTreeSet<u32>,
        last_daily_time_secs: i64,
    ) -> bool {
        self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.replace_df_quest_ids_like_cpp(df_quest_ids);
            state.replace_daily_quest_ids_like_cpp(daily_quest_ids);
            state.set_last_daily_quest_time_secs_like_cpp(last_daily_time_secs);
        })
        .is_some()
    }

    /// Install the loaded weekly bucket.
    pub(crate) fn install_represented_loaded_weekly_quests_like_cpp(
        &mut self,
        weekly_quest_ids: std::collections::BTreeSet<u32>,
    ) -> bool {
        self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.replace_weekly_quest_ids_like_cpp(weekly_quest_ids);
        })
        .is_some()
    }

    /// Install the loaded monthly bucket.
    pub(crate) fn install_represented_loaded_monthly_quests_like_cpp(
        &mut self,
        monthly_quest_ids: std::collections::BTreeSet<u32>,
    ) -> bool {
        self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.replace_monthly_quest_ids_like_cpp(monthly_quest_ids);
        })
        .is_some()
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
        wow_world_application::mutate_player_quest_gameplay_like_cpp(hub, state, mutate)
    }
    pub(in crate::session) fn represented_quest_login_aura_sources_are_hit_inert_like_cpp(
        &self,
        difficulty_id: u8,
    ) -> bool {
        let Some(state) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return false;
        };
        if !state.status_authority_complete_like_cpp() {
            return false;
        }
        let Some(quests) = self.catalogs.quests.store.as_ref() else {
            return state.rewarded_quest_rows_like_cpp().is_empty()
                && state.statuses_like_cpp().is_empty();
        };

        // C++ `_LoadQuestStatusRewarded` calls `LearnQuestRewardedSpells`
        // before filtering special quests out of `m_RewardedQuests`. Keep the
        // narrow TESTBOT proof to rows whose static reward spell is absent.
        if state.rewarded_quest_rows_like_cpp().iter().any(|quest_id| {
            quests
                .get(*quest_id)
                .is_none_or(|quest| quest.reward_spell != 0)
        }) {
            return false;
        }

        state.statuses_like_cpp().values().all(|status| {
            let Some(quest) = quests.get(status.quest_id) else {
                return false;
            };
            let recasts_accept_spell = quest.flags & QUEST_FLAGS_PLAYER_CAST_ACCEPT_LIKE_CPP != 0
                && quest.flags_ex & QUEST_FLAGS_EX_RECAST_ACCEPT_SPELL_ON_LOGIN_LIKE_CPP != 0
                && quest.source_spell_id != 0;
            !recasts_accept_spell
                || self
                    .player_target_spell_is_hit_inert_like_cpp(quest.source_spell_id, difficulty_id)
        })
    }
    pub(crate) fn spell_area_for_quest_map_bounds_like_cpp(
        &self,
        quest_id: u32,
    ) -> Vec<&SpellAreaLikeCpp> {
        self.catalogs
            .spell_catalogs
            .spell_area_store
            .as_ref()
            .map(|store| store.spell_area_for_quest_map_bounds_like_cpp(quest_id))
            .unwrap_or_default()
    }
    pub(crate) fn spell_area_for_quest_end_map_bounds_like_cpp(
        &self,
        quest_id: u32,
    ) -> Vec<&SpellAreaLikeCpp> {
        self.catalogs
            .spell_catalogs
            .spell_area_store
            .as_ref()
            .map(|store| store.spell_area_for_quest_end_map_bounds_like_cpp(quest_id))
            .unwrap_or_default()
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
    pub(crate) fn player_quest_level_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> i32 {
        let (state, hub) = crate::session::split_quest_state_ref(self);
        let player_level = if quest.quest_level > 0 {
            0
        } else {
            hub.player_level_like_cpp()
        };
        state.player_quest_level_like_cpp(player_level, quest)
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
    pub(crate) fn set_represented_resurrection_request_like_cpp(
        &mut self,
        request: PlayerResurrectionRequestLikeCpp,
    ) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_resurrection_request_like_cpp(request)
            })
            .is_some();
        #[cfg(test)]
        if canonical || self.core.player_handle_like_cpp.is_none() {
            self.fixtures
                .combat
                .represented_resurrection_request_like_cpp = Some(request);
        }
        canonical || cfg!(test) && self.core.player_handle_like_cpp.is_none()
    }
    pub(crate) fn represented_resurrection_requested_by_like_cpp(
        &self,
        resurrecter: ObjectGuid,
    ) -> bool {
        self.player_resurrection_state_snapshot_like_cpp()
            .and_then(|state| state.request)
            .is_some_and(|request| {
                request.resurrecter != ObjectGuid::EMPTY && request.resurrecter == resurrecter
            })
    }
    pub(crate) fn take_represented_resurrection_request_if_requested_by_like_cpp(
        &mut self,
        resurrecter: ObjectGuid,
    ) -> Option<PlayerResurrectionRequestLikeCpp> {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.take_resurrection_request_if_requested_by_like_cpp(resurrecter)
            })
            .flatten();
        #[cfg(test)]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            if !self.represented_resurrection_requested_by_like_cpp(resurrecter) {
                return None;
            }
            return self
                .fixtures
                .combat
                .represented_resurrection_request_like_cpp
                .take();
        }
        canonical
    }
    #[cfg(test)]
    pub(crate) fn represented_resurrection_request_like_cpp(
        &self,
    ) -> Option<PlayerResurrectionRequestLikeCpp> {
        self.player_resurrection_state_snapshot_like_cpp()
            .and_then(|state| state.request)
    }
    pub(crate) fn request_jump_proc_like_cpp(&mut self) {
        #[cfg(test)]
        {
            self.fixtures.movement.movement_jump_proc_requests_like_cpp = self
                .fixtures
                .movement
                .movement_jump_proc_requests_like_cpp
                .saturating_add(1);
        }
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
    pub(crate) fn represented_request_vehicle_switch_seat_like_cpp(
        &mut self,
        requested_vehicle: ObjectGuid,
        seat_index: u8,
    ) -> bool {
        let Some(vehicle_base_guid) =
            crate::session::hub_ref(self).represented_vehicle_base_guid_for_switch_like_cpp()
        else {
            return false;
        };
        let seat_id = seat_index as i8;
        let requested_vehicle_exists_with_empty_seat = vehicle_base_guid != requested_vehicle
            && self.represented_vehicle_seat_spell_click_plan_available_like_cpp(
                requested_vehicle,
                seat_id,
            );
        let action = crate::handlers::vehicle::request_vehicle_switch_seat_action_like_cpp(
            true,
            crate::session::hub_ref(self)
                .represented_current_vehicle_seat_can_switch_from_like_cpp(),
            vehicle_base_guid,
            requested_vehicle,
            seat_index,
            requested_vehicle_exists_with_empty_seat,
        );
        self.record_represented_vehicle_seat_action_like_cpp(action)
    }
    #[cfg(test)]
    pub(crate) fn represented_activate_taxi_requests_like_cpp(
        &self,
    ) -> &[RepresentedActivateTaxiLikeCpp] {
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
    pub(crate) fn set_represented_pending_quest_sharing_like_cpp(
        &mut self,
        sender_guid: ObjectGuid,
        quest_id: u32,
    ) {
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            self.quest_state
                .fixture_set_represented_pending_quest_sharing_like_cpp(Some(
                    RepresentedPendingQuestSharingLikeCpp {
                        sender_guid,
                        quest_id,
                    },
                ));
            self.sync_player_registry_state_like_cpp();
            return;
        }
        let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.set_pending_share_like_cpp(Some((sender_guid, quest_id)));
        });
        self.sync_player_registry_state_like_cpp();
    }
    pub(crate) fn clear_represented_pending_quest_sharing_like_cpp(&mut self) {
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            self.quest_state
                .fixture_set_represented_pending_quest_sharing_like_cpp(None);
            self.sync_player_registry_state_like_cpp();
            return;
        }
        let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.set_pending_share_like_cpp(None);
        });
        self.sync_player_registry_state_like_cpp();
    }
    pub(crate) fn represented_pending_quest_sharing_like_cpp(
        &self,
    ) -> Option<RepresentedPendingQuestSharingLikeCpp> {
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            return self
                .quest_state
                .fixture_represented_pending_quest_sharing_like_cpp();
        }
        self.player_quest_gameplay_snapshot_like_cpp()
            .and_then(|state| state.pending_share_like_cpp())
            .map(
                |(sender_guid, quest_id)| RepresentedPendingQuestSharingLikeCpp {
                    sender_guid,
                    quest_id,
                },
            )
    }
    pub(crate) fn set_represented_df_quest_like_cpp_for_test(
        &mut self,
        quest_id: u32,
        present: bool,
    ) {
        let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
            if present {
                state.set_df_quest_like_cpp(quest_id, true);
            } else {
                state.set_df_quest_like_cpp(quest_id, false);
            }
        });
        self.sync_player_registry_state_like_cpp();
    }
}

impl crate::session::QuestStateCxRef<'_> {
    pub(in crate::session) async fn battle_pet_add_request_committed_like_cpp(
        &self,
        request_key: BattlePetAddRequestKeyLikeCpp,
    ) -> Result<bool, BattlePetAddFailureLikeCpp> {
        let Some(attachment) = self.lifecycle.battle_pet_account_attachment_like_cpp() else {
            return Ok(false);
        };
        attachment
            .owner_like_cpp()
            .add_request_committed_like_cpp(request_key)
            .await
    }
}

impl crate::session::QuestStateCx<'_> {
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_auction_replicate_request_like_cpp(
        &mut self,
        request: RepresentedAuctionReplicateRequestLikeCpp,
    ) {
        #[cfg(test)]
        self.inventory
            .record_represented_auction_replicate_request_like_cpp(request);
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/quest/state/f3_shims.rs"]
mod f3_shims;
