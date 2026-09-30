//! Account collection preparation, before the caller's collection snapshot.
//!
//! C++ a5f8da2e, CollectionMgr.cpp:113–140 and :174–214. Catalog lookup
//! stays with the application; duplicate rows retain the current last-write behavior.

use std::collections::BTreeMap;

use super::PlayerCollectionStateLikeCpp;
use crate::player_gameplay_state::PlayerAccountHeirloomDataLikeCpp;

const TOY_FLAG_FAVORITE: u32 = 0x01;
const TOY_FLAG_HAS_FANFARE: u32 = 0x02;

impl PlayerCollectionStateLikeCpp {
    pub fn prepare_heirlooms(
        heirloom_rows: impl IntoIterator<Item = (u32, u32)>,
        mut resolve_bonus: impl FnMut(u32, u32) -> Option<u32>,
    ) -> BTreeMap<u32, PlayerAccountHeirloomDataLikeCpp> {
        let mut heirlooms = BTreeMap::new();
        for (item_id, flags) in heirloom_rows {
            let Some(bonus_id) = resolve_bonus(item_id, flags) else {
                continue;
            };
            heirlooms.insert(
                item_id,
                PlayerAccountHeirloomDataLikeCpp { flags, bonus_id },
            );
        }
        heirlooms
    }

    pub fn toy_flags(is_favorite: bool, has_fanfare: bool) -> u32 {
        let mut flags = 0_u32;
        if is_favorite {
            flags |= TOY_FLAG_FAVORITE;
        }
        if has_fanfare {
            flags |= TOY_FLAG_HAS_FANFARE;
        }
        flags
    }

    pub fn prepare_toys(
        toy_rows: impl IntoIterator<Item = (u32, bool, bool)>,
    ) -> BTreeMap<u32, u32> {
        let mut toys = BTreeMap::new();
        for (item_id, is_favorite, has_fanfare) in toy_rows {
            let flags = Self::toy_flags(is_favorite, has_fanfare);
            toys.insert(item_id, flags);
        }
        toys
    }
}

#[cfg(test)]
mod tests;
