//! Represented quest log state at the Session boundary.
//!
//! Moved out of the Session root under #605. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn represented_raid_difficulty_request_like_cpp(
        &self,
        difficulty_id: i32,
        legacy: bool,
    ) -> Option<u32> {
        let difficulty_id = u32::try_from(difficulty_id).ok()?;
        let entry = self
            .difficulty_store()
            .and_then(|store| store.get(difficulty_id))
            .copied()?;
        if entry.instance_type != MAP_RAID_LIKE_CPP {
            return None;
        }

        let flags = DifficultyFlags::from_bits_truncate(entry.flags);
        if !flags.contains(DifficultyFlags::CAN_SELECT) {
            return None;
        }

        (flags.contains(DifficultyFlags::LEGACY) == legacy).then_some(difficulty_id)
    }
    pub(crate) fn player_quest_gameplay_snapshot_like_cpp(
        &self,
    ) -> Option<PlayerQuestGameplayState> {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return Some(self.player_quest_gameplay_fixture_like_cpp());
        }
        self.with_owned_player_like_cpp(|player| player.gameplay_state().quests.clone())
    }
    pub(crate) fn mutate_player_quest_gameplay_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut PlayerQuestGameplayState) -> R,
    ) -> Option<R> {
        let mut mutate = Some(mutate);
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            let mut fixture = self.player_quest_gameplay_fixture_like_cpp();
            let result = mutate.take().expect("test quest mutation executes once")(&mut fixture);
            self.apply_player_quest_gameplay_fixture_like_cpp(fixture);
            return Some(result);
        }
        let canonical = self.mutate_canonical_player_like_cpp(|player| {
            mutate.take().expect("Player quest mutation executes once")(
                &mut player.gameplay_state_mut().quests,
            )
        });
        if canonical.is_some() {
            #[cfg(test)]
            if let Some(state) =
                self.with_owned_player_like_cpp(|player| player.gameplay_state().quests.clone())
            {
                self.apply_player_quest_core_compatibility_like_cpp(&state);
            }
            return canonical;
        }
        None
    }
    #[cfg(test)]
    fn player_quest_gameplay_fixture_like_cpp(&self) -> PlayerQuestGameplayState {
        let objective_counts_by_quest = self
            .player_quests
            .values()
            .map(|status| (status.quest_id, status.objective_counts.clone()))
            .collect();
        PlayerQuestGameplayState {
            statuses: self
                .player_quests
                .iter()
                .map(|(&quest_id, status)| (quest_id, status.clone()))
                .collect(),
            rewarded_quest_ids: self.rewarded_quests.iter().copied().collect(),
            daily_quest_ids: self
                .daily_quests_completed_like_cpp
                .iter()
                .copied()
                .collect(),
            weekly_quest_ids: self
                .weekly_quests_completed_like_cpp
                .iter()
                .copied()
                .collect(),
            monthly_quest_ids: self
                .monthly_quests_completed_like_cpp
                .iter()
                .copied()
                .collect(),
            seasonal_quests: self.seasonal_quests_like_cpp.clone(),
            df_quest_ids: self.df_quests_like_cpp.iter().copied().collect(),
            last_daily_quest_time_secs: self.last_daily_quest_time_like_cpp,
            seasonal_quest_changed: self.seasonal_quest_changed_like_cpp,
            status_authority_complete: self.player_quest_status_authority_complete_like_cpp,
            rewarded_quest_rows: self.represented_rewarded_quest_rows_like_cpp.clone(),
            objective_counts_by_quest,
            ..Default::default()
        }
    }
    #[cfg(test)]
    fn apply_player_quest_gameplay_fixture_like_cpp(&mut self, state: PlayerQuestGameplayState) {
        self.apply_player_quest_core_compatibility_like_cpp(&state);
        self.daily_quests_completed_like_cpp = state.daily_quest_ids.into_iter().collect();
        self.weekly_quests_completed_like_cpp = state.weekly_quest_ids.into_iter().collect();
        self.monthly_quests_completed_like_cpp = state.monthly_quest_ids.into_iter().collect();
        self.seasonal_quests_like_cpp = state.seasonal_quests;
        self.df_quests_like_cpp = state.df_quest_ids.into_iter().collect();
        self.last_daily_quest_time_like_cpp = state.last_daily_quest_time_secs;
        self.seasonal_quest_changed_like_cpp = state.seasonal_quest_changed;
    }
    #[cfg(test)]
    fn apply_player_quest_core_compatibility_like_cpp(&mut self, state: &PlayerQuestGameplayState) {
        self.player_quests = state
            .statuses
            .iter()
            .map(|(&quest_id, status)| (quest_id, status.clone()))
            .collect();
        self.rewarded_quests = state.rewarded_quest_ids.iter().copied().collect();
        self.player_quest_status_authority_complete_like_cpp = state.status_authority_complete;
        self.represented_rewarded_quest_rows_like_cpp = state.rewarded_quest_rows.clone();
    }
    pub(in crate::session) fn represented_quest_login_aura_sources_are_hit_inert_like_cpp(
        &self,
        difficulty_id: u8,
    ) -> bool {
        let Some(state) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return false;
        };
        if !state.status_authority_complete {
            return false;
        }
        let Some(quests) = self.quest_store.as_ref() else {
            return state.rewarded_quest_rows.is_empty() && state.statuses.is_empty();
        };

        // C++ `_LoadQuestStatusRewarded` calls `LearnQuestRewardedSpells`
        // before filtering special quests out of `m_RewardedQuests`. Keep the
        // narrow TESTBOT proof to rows whose static reward spell is absent.
        if state.rewarded_quest_rows.iter().any(|quest_id| {
            quests
                .get(*quest_id)
                .is_none_or(|quest| quest.reward_spell != 0)
        }) {
            return false;
        }

        state.statuses.values().all(|status| {
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
    /// C++ `Player::PushQuests` scans the complete global quest-template map
    /// during login and area updates. `AddQuest` can cast an AUTO_PUSH quest's
    /// SourceSpellID, so the narrow empty-source proof requires that the
    /// authoritative store contain no such template at all.
    pub(in crate::session) fn represented_auto_push_quest_aura_source_is_empty_like_cpp(
        &self,
    ) -> bool {
        const QUEST_FLAGS_EX_AUTO_PUSH_LIKE_CPP: u32 = 0x0400_0000;
        self.quest_store.as_ref().is_some_and(|quests| {
            quests
                .quests_like_cpp()
                .all(|quest| quest.flags_ex & QUEST_FLAGS_EX_AUTO_PUSH_LIKE_CPP == 0)
        })
    }
    pub(crate) fn spell_area_for_quest_map_bounds_like_cpp(
        &self,
        quest_id: u32,
    ) -> Vec<&SpellAreaLikeCpp> {
        self.spell_area_store
            .as_ref()
            .map(|store| store.spell_area_for_quest_map_bounds_like_cpp(quest_id))
            .unwrap_or_default()
    }
    pub(crate) fn spell_area_for_quest_end_map_bounds_like_cpp(
        &self,
        quest_id: u32,
    ) -> Vec<&SpellAreaLikeCpp> {
        self.spell_area_store
            .as_ref()
            .map(|store| store.spell_area_for_quest_end_map_bounds_like_cpp(quest_id))
            .unwrap_or_default()
    }
    /// Set the quest store shared reference.
    pub fn set_quest_store(&mut self, store: Arc<wow_data::quest::QuestStore>) {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.quest_store = Some(store);
    }
    /// Set the QuestV2 store shared reference used for C++ quest unique-bit lookups.
    pub fn set_quest_v2_store(&mut self, store: Arc<QuestV2Store>) {
        self.quest_v2_store = Some(store);
    }
    /// Set the QuestInfo store used by C++ Quest::GetQuestTag/IsImportant.
    pub fn set_quest_info_store(&mut self, store: Arc<QuestInfoStore>) {
        self.quest_info_store = Some(store);
    }
    /// Set C++ `CONFIG_QUEST_LOW_LEVEL_HIDE_DIFF`.
    pub fn set_quest_low_level_hide_diff_like_cpp(&mut self, value: u32) {
        self.quest_low_level_hide_diff_like_cpp = value;
    }
    /// Set C++ `CONFIG_QUEST_HIGH_LEVEL_HIDE_DIFF`.
    pub fn set_quest_high_level_hide_diff_like_cpp(&mut self, value: u32) {
        self.quest_high_level_hide_diff_like_cpp = value;
    }
    /// Set the QuestXP store (loaded from QuestXP.db2).
    pub fn set_quest_xp_store(&mut self, store: Arc<wow_data::quest_xp::QuestXpStore>) {
        self.quest_xp_store = Some(store);
    }
    pub fn set_min_quest_scaled_xp_ratio_like_cpp(&mut self, ratio: u32) {
        self.min_quest_scaled_xp_ratio_like_cpp = if ratio > 100 { 0 } else { ratio };
    }
    /// C++ `Player::GetQuestLevel`.
    pub(crate) fn player_quest_level_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> i32 {
        if quest.quest_level > 0 {
            quest.quest_level
        } else {
            i32::from(self.player_level_like_cpp()).min(quest.quest_max_scaling_level)
        }
    }
    /// Calculate XP reward for a quest.
    /// C++ `Quest::XPValue(player, questLevel, xpDifficulty, xpMultiplier)`.
    pub(crate) fn calculate_quest_xp(
        &self,
        difficulty: u32,
        quest_level: i32,
        xp_multiplier: f32,
    ) -> u32 {
        if let Some(store) = &self.quest_xp_store {
            store.calculate_xp(
                quest_level,
                self.player_level_like_cpp(),
                difficulty,
                xp_multiplier,
                self.min_quest_scaled_xp_ratio_like_cpp,
            )
        } else {
            const XP_TABLE: [u32; 10] = [0, 50, 100, 200, 400, 650, 1000, 1500, 2500, 4000];
            XP_TABLE[difficulty.min(9) as usize]
        }
    }
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_auction_replicate_request_like_cpp(
        &mut self,
        request: RepresentedAuctionReplicateRequestLikeCpp,
    ) {
        #[cfg(test)]
        self.represented_auction_replicate_requests_like_cpp
            .push(request);
    }
    #[cfg(test)]
    pub(crate) fn represented_auction_replicate_requests_like_cpp(
        &self,
    ) -> &[RepresentedAuctionReplicateRequestLikeCpp] {
        &self.represented_auction_replicate_requests_like_cpp
    }
    pub(crate) fn request_represented_battleground_leave_like_cpp(&mut self) {
        #[cfg(test)]
        {
            self.represented_battleground_leave_requests_like_cpp = self
                .represented_battleground_leave_requests_like_cpp
                .saturating_add(1);
        }
    }
    #[cfg(test)]
    pub(crate) fn represented_battleground_leave_requests_like_cpp(&self) -> u32 {
        self.represented_battleground_leave_requests_like_cpp
    }
    pub(crate) fn set_represented_resurrection_request_like_cpp(
        &mut self,
        request: PlayerResurrectionRequestLikeCpp,
    ) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_resurrection_request_like_cpp(request)
            })
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.represented_resurrection_request_like_cpp = Some(request);
        }
        canonical || cfg!(test) && self.player_handle_like_cpp.is_none()
    }
    pub(crate) fn clear_represented_resurrection_request_like_cpp(&mut self) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(Player::clear_resurrection_request_like_cpp)
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.represented_resurrection_request_like_cpp = None;
        }
        canonical || cfg!(test) && self.player_handle_like_cpp.is_none()
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
            .with_owned_player_mut_like_cpp(|player| {
                player.take_resurrection_request_if_requested_by_like_cpp(resurrecter)
            })
            .flatten();
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            if !self.represented_resurrection_requested_by_like_cpp(resurrecter) {
                return None;
            }
            return self.represented_resurrection_request_like_cpp.take();
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
    pub(crate) fn request_temporary_pet_unsummon_like_cpp(&mut self) {
        self.invalidate_represented_character_pet_empty_authority_like_cpp();
        #[cfg(test)]
        {
            self.temporary_pet_unsummon_requests_like_cpp = self
                .temporary_pet_unsummon_requests_like_cpp
                .saturating_add(1);
        }
    }
    #[cfg(test)]
    pub(crate) fn temporary_pet_unsummon_requests_like_cpp(&self) -> u32 {
        self.temporary_pet_unsummon_requests_like_cpp
    }
    pub(crate) fn request_jump_proc_like_cpp(&mut self) {
        #[cfg(test)]
        {
            self.movement_jump_proc_requests_like_cpp =
                self.movement_jump_proc_requests_like_cpp.saturating_add(1);
        }
    }
    #[cfg(test)]
    pub(crate) fn movement_jump_proc_requests_like_cpp(&self) -> u32 {
        self.movement_jump_proc_requests_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_duel_requests_like_cpp(&self) -> &[RepresentedDuelRequestedLikeCpp] {
        &self.represented_duel_requests_like_cpp
    }
    pub(crate) fn represented_request_vehicle_exit_like_cpp(&mut self) -> bool {
        let Some(seat_flags) = self
            .player_vehicle_seat_state_like_cpp()
            .and_then(|(flags, _)| flags)
        else {
            return false;
        };
        if !wow_data::vehicle_seat_flags_can_enter_or_exit_like_cpp(seat_flags) {
            return false;
        }

        if !self.set_player_vehicle_seat_state_like_cpp(None, None) {
            return false;
        }
        self.sync_player_registry_state_like_cpp();
        true
    }
    pub(crate) fn represented_request_adjacent_vehicle_seat_like_cpp(
        &mut self,
        next: bool,
    ) -> bool {
        match crate::handlers::vehicle::request_adjacent_vehicle_seat_action_like_cpp(
            self.player_vehicle_seat_state_like_cpp()
                .and_then(|(flags, _)| flags)
                .is_some(),
            self.represented_current_vehicle_seat_can_switch_from_like_cpp(),
            next,
        ) {
            crate::handlers::vehicle::VehicleHandlerAction::ChangeSeat { seat_id, next } => {
                #[cfg(test)]
                self.represented_vehicle_seat_change_requests_like_cpp
                    .push(RepresentedVehicleSeatChangeRequestLikeCpp { seat_id, next });
                #[cfg(not(test))]
                let _ = (seat_id, next);
                true
            }
            _ => false,
        }
    }
    #[cfg(test)]
    pub(crate) fn represented_vehicle_seat_change_requests_like_cpp(
        &self,
    ) -> &[RepresentedVehicleSeatChangeRequestLikeCpp] {
        &self.represented_vehicle_seat_change_requests_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_vehicle_seat_spell_click_requests_like_cpp(
        &self,
    ) -> &[RepresentedVehicleSeatSpellClickRequestLikeCpp] {
        &self.represented_vehicle_seat_spell_click_requests_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_vehicle_enter_requests_like_cpp(
        &self,
    ) -> &[RepresentedVehicleEnterRequestLikeCpp] {
        &self.represented_vehicle_enter_requests_like_cpp
    }
    pub(crate) fn represented_request_vehicle_switch_seat_like_cpp(
        &mut self,
        requested_vehicle: ObjectGuid,
        seat_index: u8,
    ) -> bool {
        let Some(vehicle_base_guid) = self.represented_vehicle_base_guid_for_switch_like_cpp()
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
            self.represented_current_vehicle_seat_can_switch_from_like_cpp(),
            vehicle_base_guid,
            requested_vehicle,
            seat_index,
            requested_vehicle_exists_with_empty_seat,
        );
        self.record_represented_vehicle_seat_action_like_cpp(action)
    }
    pub(in crate::session) async fn battle_pet_add_request_committed_like_cpp(
        &self,
        request_key: BattlePetAddRequestKeyLikeCpp,
    ) -> Result<bool, BattlePetAddFailureLikeCpp> {
        let Some(attachment) = &self.battle_pet_account_attachment_like_cpp else {
            return Ok(false);
        };
        attachment
            .owner_like_cpp()
            .add_request_committed_like_cpp(request_key)
            .await
    }
    #[cfg(test)]
    pub(crate) fn movement_visibility_refresh_requests_like_cpp(&self) -> u32 {
        self.movement_visibility_refresh_requests_like_cpp
    }
    pub(crate) fn consume_movement_visibility_refresh_request_like_cpp(&mut self) -> bool {
        if self.movement_visibility_refresh_requests_like_cpp == 0 {
            return false;
        }
        self.movement_visibility_refresh_requests_like_cpp = self
            .movement_visibility_refresh_requests_like_cpp
            .saturating_sub(1);
        true
    }
    #[cfg(test)]
    pub(crate) fn represented_activate_taxi_requests_like_cpp(
        &self,
    ) -> &[RepresentedActivateTaxiLikeCpp] {
        &self.represented_activate_taxi_requests_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_confirm_respec_wipe_requests_like_cpp(
        &self,
    ) -> &[RepresentedConfirmRespecWipeLikeCpp] {
        &self.represented_confirm_respec_wipe_requests_like_cpp
    }
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_adventure_map_start_quest_like_cpp(
        &mut self,
        request: RepresentedAdventureMapStartQuestLikeCpp,
    ) {
        #[cfg(test)]
        self.represented_adventure_map_start_quest_requests_like_cpp
            .push(request);
    }
    #[cfg(test)]
    pub(crate) fn represented_adventure_map_start_quest_requests_like_cpp(
        &self,
    ) -> &[RepresentedAdventureMapStartQuestLikeCpp] {
        &self.represented_adventure_map_start_quest_requests_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn temporary_pet_resummon_requests_like_cpp(&self) -> u32 {
        self.temporary_pet_resummon_requests_like_cpp
    }
    pub(crate) fn set_represented_pending_quest_sharing_like_cpp(
        &mut self,
        sender_guid: ObjectGuid,
        quest_id: u32,
    ) {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.represented_pending_quest_sharing_like_cpp =
                Some(RepresentedPendingQuestSharingLikeCpp {
                    sender_guid,
                    quest_id,
                });
            self.sync_player_registry_state_like_cpp();
            return;
        }
        let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.pending_share = Some((sender_guid, quest_id));
        });
        self.sync_player_registry_state_like_cpp();
    }
    pub(crate) fn clear_represented_pending_quest_sharing_like_cpp(&mut self) {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.represented_pending_quest_sharing_like_cpp = None;
            self.sync_player_registry_state_like_cpp();
            return;
        }
        let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.pending_share = None;
        });
        self.sync_player_registry_state_like_cpp();
    }
    pub(crate) fn represented_pending_quest_sharing_like_cpp(
        &self,
    ) -> Option<RepresentedPendingQuestSharingLikeCpp> {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return self.represented_pending_quest_sharing_like_cpp;
        }
        self.player_quest_gameplay_snapshot_like_cpp()
            .and_then(|state| state.pending_share)
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
                state.df_quest_ids.insert(quest_id);
            } else {
                state.df_quest_ids.remove(&quest_id);
            }
        });
        self.sync_player_registry_state_like_cpp();
    }
    #[cfg(test)]
    pub(crate) fn represented_timed_quest_removals_like_cpp(&self) -> &[u32] {
        &self.represented_timed_quest_removals_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_quest_push_result_responses_like_cpp(
        &self,
    ) -> &[RepresentedQuestPushResultResponseLikeCpp] {
        &self.represented_quest_push_result_responses_like_cpp
    }
    pub(crate) fn record_represented_quest_push_result_response_like_cpp(
        &mut self,
        response: RepresentedQuestPushResultResponseLikeCpp,
    ) {
        #[cfg(test)]
        self.represented_quest_push_result_responses_like_cpp
            .push(response);
        #[cfg(not(test))]
        let _ = response;
    }
    pub(crate) fn record_represented_quest_push_result_sender_mismatch_like_cpp(&mut self) {
        #[cfg(test)]
        {
            self.represented_quest_push_result_sender_mismatch_count_like_cpp = self
                .represented_quest_push_result_sender_mismatch_count_like_cpp
                .saturating_add(1);
        }
    }
    #[cfg(test)]
    pub(crate) fn represented_quest_confirm_accepts_like_cpp(
        &self,
    ) -> &[RepresentedQuestConfirmAcceptLikeCpp] {
        &self.represented_quest_confirm_accepts_like_cpp
    }
    pub(crate) fn record_represented_quest_confirm_accept_like_cpp(
        &mut self,
        evidence: RepresentedQuestConfirmAcceptLikeCpp,
    ) {
        #[cfg(test)]
        self.represented_quest_confirm_accepts_like_cpp
            .push(evidence);
        #[cfg(not(test))]
        let _ = evidence;
    }
    #[cfg(test)]
    pub(crate) fn represented_push_quest_to_party_outcomes_like_cpp(
        &self,
    ) -> &[RepresentedPushQuestToPartyOutcomeLikeCpp] {
        &self.represented_push_quest_to_party_outcomes_like_cpp
    }
    pub(crate) fn record_represented_push_quest_to_party_outcome_like_cpp(
        &mut self,
        outcome: RepresentedPushQuestToPartyOutcomeLikeCpp,
    ) {
        #[cfg(test)]
        self.represented_push_quest_to_party_outcomes_like_cpp
            .push(outcome);
        #[cfg(not(test))]
        let _ = outcome;
    }
}
