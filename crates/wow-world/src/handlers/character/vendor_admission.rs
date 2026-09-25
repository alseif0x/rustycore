// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Vendor stock and C++-slot catalog admission.

use super::vendor::rules::vendor_buy_stock_refill_count;
use super::*;

impl WorldSession {
    pub(super) fn vendor_item_current_count(
        &mut self,
        vendor_guid: ObjectGuid,
        item_id: u32,
        max_count: u32,
        incr_time: u32,
        buy_count: u32,
    ) -> u32 {
        if max_count == 0 {
            return 0;
        }
        let key = (vendor_guid, item_id);
        let now = wow_entities::game_time_secs_like_cpp().max(0) as u64;
        let Some(count) = self.vendor_item_counts.get(&key).copied() else {
            return max_count;
        };
        let elapsed = now.saturating_sub(count.last_increment_time);
        let (new_count, full) =
            vendor_buy_stock_refill_count(count.count, elapsed, incr_time, buy_count, max_count);
        if full {
            self.vendor_item_counts.remove(&key);
            max_count
        } else if let Some(count) = self.vendor_item_counts.get_mut(&key) {
            count.count = new_count;
            if incr_time > 0 && elapsed >= u64::from(incr_time) {
                count.last_increment_time = now;
            }
            count.count
        } else {
            new_count
        }
    }

    pub(super) fn update_vendor_item_current_count(
        &mut self,
        vendor_guid: ObjectGuid,
        item_id: u32,
        max_count: u32,
        incr_time: u32,
        buy_count: u32,
        used_count: u32,
    ) -> u32 {
        if max_count == 0 {
            return 0;
        }
        let current =
            self.vendor_item_current_count(vendor_guid, item_id, max_count, incr_time, buy_count);
        let new_count = current.saturating_sub(used_count);
        self.vendor_item_counts.insert(
            (vendor_guid, item_id),
            crate::session::VendorItemCount {
                count: new_count,
                last_increment_time: wow_entities::game_time_secs_like_cpp().max(0) as u64,
            },
        );
        new_count
    }
}
