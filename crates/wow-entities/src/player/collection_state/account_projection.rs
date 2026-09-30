//! Immutable account projections for application packet and persistence adapters.
//!
//! C++ a5f8da2e, CollectionMgr.cpp:126–140, :201–214 and :348–358.
//! Signed identifier filtering and wire types remain explicit at each caller.
//! Mount sorting stays at the original application point.

use super::PlayerCollectionStateLikeCpp;

impl PlayerCollectionStateLikeCpp {
    pub fn project_heirlooms<T>(
        &self,
        mut emit: impl FnMut(u32, u32, u32) -> Option<T>,
    ) -> Vec<T> {
        self.heirlooms
            .iter()
            .filter_map(|(item_id, data)| emit(*item_id, data.flags, data.bonus_id))
            .collect()
    }

    pub fn project_toys<T>(
        &self,
        mut emit: impl FnMut(u32, u32) -> Option<T>,
    ) -> Vec<T> {
        self.toys
            .iter()
            .filter_map(|(item_id, flags)| emit(*item_id, *flags))
            .collect()
    }

    pub fn project_toy_status<T>(
        &self,
        mut emit: impl FnMut(u32, bool, bool) -> Option<T>,
    ) -> Vec<T> {
        self.project_toys(|item_id, flags| {
            emit(
                item_id,
                (flags & Self::toy_flags(true, false)) != 0,
                (flags & Self::toy_flags(false, true)) != 0,
            )
        })
    }

    pub fn project_mounts<T>(
        &self,
        mut emit: impl FnMut(i32, u8) -> Option<T>,
    ) -> Vec<T> {
        self.mounts
            .iter()
            .filter_map(|(spell_id, flags)| emit(*spell_id, *flags))
            .collect()
    }
}

#[cfg(test)]
mod tests;
