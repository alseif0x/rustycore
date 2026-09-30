//! Canonical toy insertion and favorite/fanfare transitions.
//!
//! C++ a5f8da2e, CollectionMgr.cpp:102–164. Unknown toys stay absent;
//! successful updates of known toys also succeed when their bits are unchanged.

use std::collections::BTreeMap;

use super::PlayerCollectionStateLikeCpp;

impl PlayerCollectionStateLikeCpp {
    /// C++ `CollectionMgr::AddToy` (`:102`) through `UpdateAccountToys`
    /// (`:140`), whose `_toys.insert` leaves an already collected toy alone;
    /// the caller learns that from the result, as C++ does from `.second`.
    pub fn add_toy_like_cpp(&mut self, item_id: u32, flags: u32) -> bool {
        if self.toys.contains_key(&item_id) {
            return false;
        }
        self.toys.insert(item_id, flags);
        true
    }

    /// Forget one toy, as the represented removal does.
    pub fn remove_toy_like_cpp(&mut self, item_id: u32) -> bool {
        self.toys.remove(&item_id).is_some()
    }

    /// Set and clear flag bits on one collected toy
    /// (C++ `ToySetFavorite` `:145` and `ToyClearFanfare` `:157`).
    pub fn update_toy_flags_like_cpp(&mut self, item_id: u32, set: u32, clear: u32) -> bool {
        let Some(flags) = self.toys.get_mut(&item_id) else {
            return false;
        };
        *flags |= set;
        *flags &= !clear;
        true
    }

    /// Install the authoritative loaded toys
    /// (`CollectionMgr::LoadAccountToys` `:113`).
    pub fn replace_toys_like_cpp(&mut self, toys: BTreeMap<u32, u32>) {
        self.toys = toys;
    }

    /// C++ `CollectionMgr::ToyClearFanfare`.
    pub fn clear_toy_fanfare(&mut self, item_id: u32) -> bool {
        self.update_toy_flags_like_cpp(item_id, 0, Self::toy_flags(false, true))
    }

    /// C++ `CollectionMgr::ToySetFavorite`.
    pub fn set_toy_favorite(&mut self, item_id: u32, favorite: bool) -> bool {
        if favorite {
            self.update_toy_flags_like_cpp(item_id, Self::toy_flags(true, false), 0)
        } else {
            self.update_toy_flags_like_cpp(item_id, 0, Self::toy_flags(true, false))
        }
    }
}

#[cfg(test)]
mod tests;
