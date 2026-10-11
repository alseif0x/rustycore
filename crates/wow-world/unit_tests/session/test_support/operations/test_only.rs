// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1363 slice 1: production-dead `operations` entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl WorldSession {
    pub(crate) fn loot_money_persistence_test_result_for_worker_like_cpp(&self) -> Option<bool> {
        #[cfg(test)]
        {
            self.lifecycle.loot_money_persistence_test_result_like_cpp()
        }
        #[cfg(not(test))]
        {
            None
        }
    }
}
