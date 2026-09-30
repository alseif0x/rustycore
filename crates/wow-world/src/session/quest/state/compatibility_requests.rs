//! compatibility requests operations at the existing Quest application boundary.

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
        &self
            .duel_test_fixture_like_cpp
            .represented_duel_requests_like_cpp
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
        let Some(attachment) = &self.lifecycle.battle_pet_account_attachment_like_cpp else {
            return Ok(false);
        };
        attachment
            .owner_like_cpp()
            .add_request_committed_like_cpp(request_key)
            .await
    }
    #[cfg(test)]
    pub(crate) fn movement_visibility_refresh_requests_like_cpp(&self) -> u32 {
        self.quest_state
            .movement_visibility_refresh_requests_like_cpp
    }
    pub(crate) fn consume_movement_visibility_refresh_request_like_cpp(&mut self) -> bool {
        if self
            .quest_state
            .movement_visibility_refresh_requests_like_cpp
            == 0
        {
            return false;
        }
        self.quest_state
            .movement_visibility_refresh_requests_like_cpp = self
            .quest_state
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
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn temporary_pet_resummon_requests_like_cpp(&self) -> u32 {
        self.temporary_pet_resummon_requests_like_cpp
    }
}
