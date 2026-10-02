use rand::{Rng, RngCore, SeedableRng, rngs::StdRng};

use crate::session::state::SessionCore;

impl SessionCore {
    pub fn represented_urand_u32_like_cpp(&mut self, min: u32, max: u32) -> u32 {
        if min >= max {
            return min;
        }
        self.driver
            .represented_runtime_rng_like_cpp
            .gen_range(min..=max)
    }

    pub fn represented_runtime_subrng_like_cpp(&mut self) -> StdRng {
        StdRng::seed_from_u64(self.driver.represented_runtime_rng_like_cpp.next_u64())
    }
}
