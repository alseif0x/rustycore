use wow_core::ObjectGuid;
use wow_entities::game_time_secs_like_cpp;

use crate::{InteractionState, VendorItemCount};

impl InteractionState {
    pub fn vendor_item_current_count(
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
        let now = game_time_secs_like_cpp().max(0) as u64;
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

    pub fn update_vendor_item_current_count(
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
            VendorItemCount {
                count: new_count,
                last_increment_time: game_time_secs_like_cpp().max(0) as u64,
            },
        );
        new_count
    }
}

fn vendor_buy_stock_refill_count(
    current_count: u32,
    elapsed_secs: u64,
    incr_time: u32,
    buy_count: u32,
    max_count: u32,
) -> (u32, bool) {
    if max_count == 0 || current_count >= max_count || incr_time == 0 {
        // C++ assumes nonzero incrtime for finite stock; keep invalid DB rows from dividing by zero.
        return (current_count.min(max_count), current_count >= max_count);
    }

    let increments = elapsed_secs / u64::from(incr_time);
    if increments == 0 {
        return (current_count, false);
    }

    let restored = increments.saturating_mul(u64::from(buy_count.max(1)));
    let new_count = u64::from(current_count).saturating_add(restored);
    if new_count >= u64::from(max_count) {
        (max_count, true)
    } else {
        (new_count as u32, false)
    }
}

#[cfg(test)]
mod tests {
    use super::vendor_buy_stock_refill_count;

    #[test]
    fn vendor_buy_stock_refill_matches_cpp_increment_and_full_reset() {
        assert_eq!(vendor_buy_stock_refill_count(2, 20, 10, 5, 20), (12, false));
        assert_eq!(vendor_buy_stock_refill_count(18, 10, 10, 5, 20), (20, true));
        assert_eq!(vendor_buy_stock_refill_count(2, 9, 10, 5, 20), (2, false));
    }
}
