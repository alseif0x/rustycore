// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical attacker removal shared with Session combat adapters.

use crate::session::movement_protocol::MovementAckEventLikeCpp;
use wow_core::ObjectGuid;

impl crate::session::HubMut<'_> {
    pub fn remove_canonical_attacker_like_cpp(&mut self, victim: ObjectGuid, attacker: ObjectGuid) {
        if self
            .core
            .mutate_canonical_player_by_guid_like_cpp(victim, |victim| {
                victim.unit_mut().remove_attacker_like_cpp(attacker)
            })
            .is_some()
        {
            return;
        }
        let _ = self
            .core
            .mutate_canonical_creature_by_guid_like_cpp(victim, |victim| {
                victim.unit_mut().remove_attacker_like_cpp(attacker)
            });
    }
}

#[cfg(any(test, feature = "test-fixtures"))]
impl crate::session::state::MovementState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn movement_ack_events_like_cpp(&self) -> &[MovementAckEventLikeCpp] {
        &self.movement_ack_events_like_cpp
    }
}

impl crate::session::HubRef<'_> {
    pub fn validate_movement_ack_status_like_cpp(
        &self,
        status: &wow_packet::packets::movement::MovementInfo,
    ) -> bool {
        let Some(player_guid) = self.core.player_guid() else {
            return false;
        };

        status.guid == player_guid && status.position.is_valid_map_coord_like_cpp()
    }
}

impl crate::session::HubMut<'_> {
    pub fn record_movement_ack_event_like_cpp(&mut self, event: MovementAckEventLikeCpp) {
        #[cfg(any(test, feature = "test-fixtures"))]
        self.fixtures
            .movement
            .movement_ack_events_like_cpp
            .push(event);
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = event;
    }
}
