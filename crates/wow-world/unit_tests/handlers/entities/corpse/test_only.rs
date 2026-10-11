// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1363 slice 1: production-dead `corpse` entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    /// CMSG_CLIENT_PORT_GRAVEYARD — manually teleport ghost to graveyard.
    /// C++ ref: `WorldSession::HandlePortGraveyard`.
    pub(crate) async fn try_handle_client_port_graveyard_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
    ) -> bool {
        if PortGraveyard::read(&mut pkt).is_err() {
            return false;
        }

        if crate::session::hub_ref(self).resolved_player_is_alive_like_cpp() != Some(false)
            || !crate::session::hub_ref(self).player_has_ghost_flag_like_cpp()
        {
            return true;
        }

        // C++ calls `Player::RepopAtGraveyard()`. Rust still represents the
        // graveyard selection/teleport runtime as a counter seam shared with
        // release and instance-lock decline paths.
        #[cfg(test)]
        {
            self.fixtures.combat.represented_repop_at_graveyard_count = self
                .fixtures
                .combat
                .represented_repop_at_graveyard_count
                .saturating_add(1);
        }
        true
    }
}
