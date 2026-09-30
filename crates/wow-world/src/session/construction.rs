// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Construction: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{Rng, RngCore, SeedableRng, StdRng, WorldSession};

mod world_session;

impl WorldSession {
    pub(in crate::session) const MIN_ITEM_LEVEL_LIKE_CPP: u32 = 1;
    pub(in crate::session) const MAX_ITEM_LEVEL_LIKE_CPP: u32 = 1300;

    #[cfg(test)]
    pub(crate) fn seed_represented_runtime_rng_like_cpp(&mut self, seed: u64) {
        self.driver.represented_runtime_rng_like_cpp = StdRng::seed_from_u64(seed);
    }

    pub(crate) fn represented_urand_u32_like_cpp(&mut self, min: u32, max: u32) -> u32 {
        if min >= max {
            return min;
        }
        self.driver
            .represented_runtime_rng_like_cpp
            .gen_range(min..=max)
    }

    pub(crate) fn represented_runtime_subrng_like_cpp(&mut self) -> StdRng {
        StdRng::seed_from_u64(self.driver.represented_runtime_rng_like_cpp.next_u64())
    }
}
