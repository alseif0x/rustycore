// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Bank interaction kept in the World: the show-bank publication the gossip
//! menu also uses. The banker activation and the bank slot purchase moved to
//! `wow_world_inventory::vendor_session` (#1363 slice 8b), the autobank and
//! autostore-bank moves to `wow_world_inventory::item_session` (#1363 slice 8c).

use super::*;

impl WorldSession {
    pub(super) fn send_show_bank_like_cpp(&mut self, banker_guid: ObjectGuid) {
        let (state, hub) = crate::session::split_interaction(self);
        state.send_show_bank_like_cpp(hub, banker_guid)
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/handlers/character/bank/test_only.rs"]
mod test_only;
