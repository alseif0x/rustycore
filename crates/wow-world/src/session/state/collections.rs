// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `WorldSession::collections` sub-state (#1241 F2): moved fields, no logic.

use super::*;

/// Account collections: mounts, heirlooms, toys, item appearances, transmog illusions and completed
/// achievements.
pub(crate) struct CollectionsState {
    /// C++ `CollectionMgr::_mounts` represented account mount collection.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) account_mounts_like_cpp: HashMap<i32, u8>,

    /// C++ `CollectionMgr::_heirlooms`, represented until account collection runtime is complete.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_account_heirlooms_like_cpp: BTreeMap<u32, AccountHeirloomDataLikeCpp>,
    /// C++ `CollectionMgr::_toys`, represented until account collection runtime is complete.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_account_toys_like_cpp: BTreeMap<u32, u32>,
    /// C++ `CollectionMgr::_appearances`, represented until account collection persistence is ported.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_item_appearances_like_cpp: HashSet<u32>,
    /// C++ `CollectionMgr::LoadAccountItemAppearances` block vector used by
    /// `ActivePlayerData::Transmog`. This preserves sparse `blobIndex` rows
    /// that cannot be recovered from `_appearances` alone.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_item_appearance_blocks_like_cpp: Vec<u32>,
    /// C++ `CollectionMgr::_temporaryAppearances`, represented until account collection persistence is ported.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_temporary_item_appearances_like_cpp: HashMap<u32, HashSet<ObjectGuid>>,
    /// C++ `CollectionMgr::_favoriteAppearances`, represented until account collection persistence is ported.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_favorite_item_appearances_like_cpp:
        HashMap<u32, FavoriteAppearanceStateLikeCpp>,
    /// C++ `CollectionMgr::_transmogIllusions`, represented until account collection runtime is complete.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_transmog_illusions_like_cpp: HashSet<u32>,

    /// C++ `Player::HasAchieved`, represented per-session until character achievements are fully loaded.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_completed_achievements_like_cpp: HashSet<u32>,
}
