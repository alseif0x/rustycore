// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Trade adapter: private Session responsibility.
//! Relocated under #1233; under #1263 F5 the trade item transitions moved to
//! `wow-world-application::trade_handlers`, so only the cfg(test) reads that
//! unit tests still use remain here.

use super::WorldSession;
use super::{ObjectGuid, TRADE_SLOT_COUNT_LIKE_CPP};

impl WorldSession {
    #[cfg(test)]
    pub(crate) fn represented_trade_item_like_cpp(&self, slot: u8) -> Option<ObjectGuid> {
        (slot < TRADE_SLOT_COUNT_LIKE_CPP)
            .then(|| {
                self.player_trade_state_snapshot_like_cpp()
                    .flatten()
                    .and_then(|state| state.items[slot as usize])
            })
            .flatten()
    }
}
