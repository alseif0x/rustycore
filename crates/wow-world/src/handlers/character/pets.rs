// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Stable-master and character-pet handlers.

use super::*;

impl WorldSession {
    /// CMSG_REQUEST_STABLED_PETS — player opens stable master UI.
    /// C++ ref: `WorldSession::HandleRequestStabledPets`.
    pub async fn handle_request_stabled_pets(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match RequestStabledPets::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "RequestStabledPets parse failed: {error}"
                );
                return;
            }
        };

        // C++ returns before sending anything when CheckStableMaster fails.
        // The live stable-master validation and Player::SetStableMaster update
        // fields are not ported here yet, so preserve that observable branch.
        debug!(
            account = self.account_id,
            stable_master = ?request.stable_master,
            "RequestStabledPets ignored without represented stable-master runtime"
        );
    }
}
