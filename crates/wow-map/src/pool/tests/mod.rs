//! Pool template and group selection regression scenarios.
//!
//! Separated from the pool.rs root under #644.

use super::*;

fn choose_first_indices(_candidates: &[PoolObjectLikeCpp], count: usize) -> Vec<usize> {
    (0..count).collect()
}

fn group_with_one(kind: PoolMemberKindLikeCpp, pool_id: u32, guid: u64) -> PoolGroupLikeCpp {
    let mut group = PoolGroupLikeCpp::with_pool_id(kind, pool_id);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(guid, 0.0), 1);
    group
}

mod scenarios_1;
mod scenarios_2;
