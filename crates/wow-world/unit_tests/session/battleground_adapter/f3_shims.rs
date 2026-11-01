// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn set_player_battleground_type_id_like_cpp(&mut self, bg_type_id: u32) -> bool {
        crate::session::hub_mut(self).set_player_battleground_type_id_like_cpp(bg_type_id)
    }
    #[cfg(test)]
    pub(crate) fn set_player_battleground_context_like_cpp(
        &mut self,
        bg_type_id: u32,
        bg_map_id: u32,
    ) -> bool {
        crate::session::hub_mut(self)
            .set_player_battleground_context_like_cpp(bg_type_id, bg_map_id)
    }
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn set_represented_battleground_status_like_cpp(&mut self, status: Option<u8>) {
        crate::session::hub_mut(self).set_represented_battleground_status_like_cpp(status)
    }
    #[cfg(test)]
    pub(crate) fn add_represented_battleground_queue_slot_like_cpp(
        &mut self,
        slot: u32,
        queue_type_id: RepresentedBattlegroundQueueTypeIdLikeCpp,
        invited_instance_guid: u32,
    ) {
        crate::session::hub_mut(self).add_represented_battleground_queue_slot_like_cpp(
            slot,
            queue_type_id,
            invited_instance_guid,
        )
    }
    #[cfg(test)]
    pub(crate) fn represented_battlemaster_hellos_like_cpp(
        &self,
    ) -> &[RepresentedBattlemasterHelloLikeCpp] {
        self.fixtures
            .battleground
            .represented_battlemaster_hellos_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_battlefield_lists_like_cpp(
        &self,
    ) -> &[RepresentedBattlefieldListLikeCpp] {
        self.fixtures
            .battleground
            .represented_battlefield_lists_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_battlemaster_joins_like_cpp(
        &self,
    ) -> &[RepresentedBattlemasterJoinLikeCpp] {
        self.fixtures
            .battleground
            .represented_battlemaster_joins_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_battlemaster_join_arenas_like_cpp(
        &self,
    ) -> &[RepresentedBattlemasterJoinArenaLikeCpp] {
        self.fixtures
            .battleground
            .represented_battlemaster_join_arenas_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_battlemaster_join_skirmishes_like_cpp(
        &self,
    ) -> &[RepresentedBattlemasterJoinSkirmishLikeCpp] {
        self.fixtures
            .battleground
            .represented_battlemaster_join_skirmishes_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_battlefield_ports_like_cpp(
        &self,
    ) -> &[RepresentedBattlefieldPortLikeCpp] {
        self.fixtures
            .battleground
            .represented_battlefield_ports_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_wargame_invite_acceptances_like_cpp(
        &self,
    ) -> &[RepresentedWargameInviteAcceptanceLikeCpp] {
        self.fixtures
            .battleground
            .represented_wargame_invite_acceptances_like_cpp()
    }
    pub(crate) fn represented_battleground_status_is_wait_leave_like_cpp(&self) -> bool {
        crate::session::hub_ref(self).represented_battleground_status_is_wait_leave_like_cpp()
    }
}
