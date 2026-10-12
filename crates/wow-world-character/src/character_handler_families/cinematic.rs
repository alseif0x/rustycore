// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character handler family: opening cinematic (`HandleOpeningCinematic`).

use wow_packet::WorldPacket;

use crate::character_handlers::{
    CharacterHandlerCxLikeCpp, send_represented_cinematic_start_like_cpp,
};

impl CharacterHandlerCxLikeCpp<'_> {
    /// Handle CMSG_OPENING_CINEMATIC.
    pub async fn handle_opening_cinematic(&mut self, _pkt: WorldPacket) {
        let _ = self.opening_cinematic_like_cpp();
    }

    /// C++ `HandleOpeningCinematic`: a character that has never gained
    /// experience plays its class cinematic, or its race cinematic when the
    /// class has none.
    fn opening_cinematic_like_cpp(&mut self) -> Option<u32> {
        if self.hub.shared().resolved_player_xp_like_cpp()? != 0 {
            return None;
        }

        let class_store = self.hub.catalogs.chr.classes_store.as_ref()?;
        let class_entry = class_store.get(u32::from(self.hub.shared().player_class_like_cpp()))?;
        let cinematic_id = if class_entry.cinematic_sequence_id != 0 {
            u32::from(class_entry.cinematic_sequence_id)
        } else {
            let race_store = self.hub.catalogs.chr.races_store.as_ref()?;
            race_store
                .get(u32::from(self.hub.shared().player_race_like_cpp()))
                .map(|race_entry| race_entry.cinematic_sequence_id as u32)?
        };

        send_represented_cinematic_start_like_cpp(&mut self.hub, cinematic_id);
        Some(cinematic_id)
    }
}
