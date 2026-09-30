//! Charge queue transitions on the canonical SpellHistory.

use super::*;

impl SpellHistory {
    pub fn set_charges(
        &mut self,
        charge_category_id: u32,
        charges: u8,
        started_at_ms: u64,
        recharge_ms: u32,
    ) {
        let queue = self.charges.entry(charge_category_id).or_default();
        queue.clear();
        let mut start = started_at_ms;
        for _ in 0..charges {
            let end = start + u64::from(recharge_ms);
            queue.push_back(SpellChargeState {
                recharge_start_ms: start,
                recharge_end_ms: end,
            });
            start = end;
        }
    }

    pub fn charges(&self, charge_category_id: u32) -> Option<&VecDeque<SpellChargeState>> {
        self.charges.get(&charge_category_id)
    }

    pub fn add_charge_state_like_cpp(
        &mut self,
        charge_category_id: u32,
        recharge_start_ms: u64,
        recharge_end_ms: u64,
    ) -> bool {
        if charge_category_id == 0 {
            return false;
        }
        self.charges
            .entry(charge_category_id)
            .or_default()
            .push_back(SpellChargeState {
                recharge_start_ms,
                recharge_end_ms,
            });
        true
    }

    pub fn consumed_charges(&self, charge_category_id: u32) -> u8 {
        self.charges
            .get(&charge_category_id)
            .map_or(0, |charges| charges.len().min(u8::MAX as usize) as u8)
    }

    pub fn has_charge(&self, charge_category_id: u32, max_charges: i32) -> bool {
        charge_category_id == 0
            || max_charges <= 0
            || self
                .charges
                .get(&charge_category_id)
                .is_none_or(|charges| charges.len() < max_charges as usize)
    }

    pub fn consume_charge(
        &mut self,
        charge_category_id: u32,
        now_ms: u64,
        recovery_ms: u32,
        max_charges: i32,
    ) -> bool {
        if charge_category_id == 0 || recovery_ms == 0 || max_charges <= 0 {
            return false;
        }

        let queue = self.charges.entry(charge_category_id).or_default();
        let recharge_start_ms = queue.back().map_or(now_ms, |charge| charge.recharge_end_ms);
        queue.push_back(SpellChargeState {
            recharge_start_ms,
            recharge_end_ms: recharge_start_ms + u64::from(recovery_ms),
        });
        true
    }

    pub fn modify_charge_recovery_time(
        &mut self,
        charge_category_id: u32,
        cooldown_delta_ms: i64,
        now_ms: u64,
    ) -> bool {
        let Some(queue) = self.charges.get_mut(&charge_category_id) else {
            return false;
        };
        if queue.is_empty() {
            return false;
        }

        for charge in queue.iter_mut() {
            charge.recharge_start_ms = apply_ms_delta(charge.recharge_start_ms, cooldown_delta_ms);
            charge.recharge_end_ms = apply_ms_delta(charge.recharge_end_ms, cooldown_delta_ms);
        }

        while queue
            .front()
            .is_some_and(|charge| charge.recharge_end_ms < now_ms)
        {
            queue.pop_front();
        }

        true
    }

    pub fn restore_charge(&mut self, charge_category_id: u32) -> bool {
        self.charges
            .get_mut(&charge_category_id)
            .and_then(VecDeque::pop_back)
            .is_some()
    }

    pub fn clear_charges(&mut self, charge_category_id: u32) -> bool {
        self.charges.remove(&charge_category_id).is_some()
    }

    pub fn reset_all_charges(&mut self) {
        self.charges.clear();
    }

    /// Restore the canonical round without requiring loaded rows or removing
    /// an empty category. SpellEffects.cpp:5896-5903; the existing single-charge
    /// transition preserves SpellHistory::RestoreCharge's pop-back.
    pub fn restore_charges(&mut self, category_id: u32, count: i32) -> u32 {
        let mut restored = 0;
        for _ in 0..count {
            if self.restore_charge(category_id) {
                restored += 1;
            }
        }
        restored
    }

    /// The loaded-history round has its own admission and per-iteration owner
    /// access in the application. It removes a category only after a success.
    pub fn restore_loaded_charge(&mut self, category_id: u32) -> bool {
        if !self.charges_loaded {
            return false;
        }
        let Some(charges) = self.charges.get_mut(&category_id) else {
            return false;
        };
        if charges.pop_back().is_none() {
            return false;
        }
        if charges.is_empty() {
            self.charges.remove(&category_id);
        }
        true
    }
}

#[cfg(test)]
mod tests;
