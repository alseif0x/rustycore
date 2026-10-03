// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Vendor stock and C++-slot catalog admission.

use super::*;

impl WorldSession {
    pub(super) fn update_vendor_item_current_count(
        &mut self,
        vendor_guid: ObjectGuid,
        item_id: u32,
        max_count: u32,
        incr_time: u32,
        buy_count: u32,
        used_count: u32,
    ) -> u32 {
        self.interaction.update_vendor_item_current_count(
            vendor_guid,
            item_id,
            max_count,
            incr_time,
            buy_count,
            used_count,
        )
    }
}
