// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_packet::ServerPacket;

impl crate::session::HubMut<'_> {
    pub fn clear_player_emote_state_on_player_movement_like_cpp(&mut self) {
        if let Some(update) = self.clear_player_emote_state_on_movement_like_cpp() {
            self.core.send_packet(&update);
            self.shared()
                .broadcast_to_movement_set_like_cpp(update.to_bytes(), false);
        }
    }
}
