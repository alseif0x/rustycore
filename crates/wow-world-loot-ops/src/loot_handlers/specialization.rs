// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot handler family: loot specialization selection
//! (`CMSG_SET_LOOT_SPECIALIZATION`, C++ `HandleSetLootSpecializationOpcode`).

use super::*;

impl<'a> LootHandlerCxLikeCpp<'a> {
    /// CMSG_SET_LOOT_SPECIALIZATION — select or clear the loot specialization.
    ///
    /// C++ accepts non-zero values only when `sChrSpecializationStore` has the
    /// row and its `ClassID` matches the player's class; `SpecID == 0` clears.
    pub async fn handle_set_loot_specialization(&mut self, packet: SetLootSpecialization) {
        if self.hub.shared().core.player_guid().is_none() {
            return;
        }

        if packet.spec_id == 0 {
            self.loot
                .set_loot_specialization_id_like_cpp(&mut self.hub, 0);
            return;
        }

        let Some(store) = self.hub.catalogs.chr_specialization_store() else {
            return;
        };
        let Some(spec) = store.get(packet.spec_id) else {
            return;
        };
        if spec.class_id != self.hub.shared().player_class_like_cpp() {
            return;
        }

        self.loot
            .set_loot_specialization_id_like_cpp(&mut self.hub, packet.spec_id);
    }
}
