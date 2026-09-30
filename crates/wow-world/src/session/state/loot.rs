// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `WorldSession::loot` sub-state (#1241 F2): moved fields, no logic.

use super::*;

/// Loot windows and AE-loot views with their authorities and generations, loot rolls, personal loot
/// money and the loot test hooks.
pub(crate) struct LootState {
    #[cfg(test)]
    pub(crate) pass_on_group_loot: bool,
    /// C++ ActivePlayerData::LootSpecID represented session state.
    #[cfg(test)]
    pub(in crate::session) loot_specialization_id: u32,

    // ── Loot ──────────────────────────────────────────────────────
    /// Active loot windows keyed by creature GUID.
    pub(crate) loot_table:
        std::collections::HashMap<wow_core::ObjectGuid, wow_packet::packets::loot::CreatureLoot>,
    /// Object-owned loot generation represented by each session-local packet cache.
    pub(crate) represented_loot_cache_generations_like_cpp:
        std::collections::HashMap<wow_core::ObjectGuid, u64>,
    /// Mirrors C++ PlayerData::LootTargetGUID for guards that compare active loot by GUID.
    pub(crate) active_loot_guid: wow_core::ObjectGuid,
    /// Represented owner GUIDs currently visible through C++ `Player::m_AELootView`.
    pub(crate) active_loot_view_owners: std::collections::HashSet<wow_core::ObjectGuid>,
    /// Object-owned generation that was actually opened for each active loot view.
    ///
    /// GUIDs are reused across creature respawns and gameobject restocks.  A delayed
    /// packet from an older window must therefore not be authorized merely because
    /// the replacement lifetime has the same owner/loot GUID and player eligibility.
    pub(crate) active_loot_view_generations_like_cpp:
        std::collections::HashMap<wow_core::ObjectGuid, u64>,
    /// Exact backing allocation opened for each view. Scope epochs restart at
    /// one in a newly allocated authority, so the generation map alone cannot
    /// prevent ABA when a creature GUID is recreated.
    pub(crate) active_loot_view_authorities_like_cpp:
        std::collections::HashMap<wow_core::ObjectGuid, OwnedLootAuthority>,
    /// Represented pending group/NBG loot rolls keyed by `(LootObj, LootListID)`.
    pub(crate) represented_loot_rolls:
        std::collections::HashMap<(wow_core::ObjectGuid, u8), RepresentedLootRollState>,
    #[cfg(test)]
    pub(crate) loot_item_store_test_grants_like_cpp: Option<Arc<AtomicUsize>>,
    #[cfg(test)]
    pub(crate) loot_item_store_test_success_like_cpp: bool,
    /// Optional test-only COMMIT gate for exercising remote command timeout
    /// and release while the authority claim is already persistence-owned.
    #[cfg(test)]
    pub(crate) loot_item_store_test_commit_gate_like_cpp: Option<Arc<tokio::sync::Notify>>,
    #[cfg(test)]
    pub(crate) represented_loot_roll_criteria_events: Vec<RepresentedLootRollCriteriaEvent>,
    /// Session-local representation of `GameObject::m_unique_users` for no-GetLootId chest uses.
    pub(crate) represented_unique_gameobject_uses: std::collections::HashSet<wow_core::ObjectGuid>,
    /// Session-local representation of `GameObject::m_tapList` for personal encounter loot.
    pub(crate) represented_gameobject_tap_lists:
        std::collections::HashMap<wow_core::ObjectGuid, Vec<wow_core::ObjectGuid>>,
    /// Handle-less compatibility for represented encounter-loot fixtures.
    /// Production `Player::IsLockedToDungeonEncounter` resolves the immutable
    /// DungeonEncounter catalog and shared InstanceLockMgr.
    #[cfg(test)]
    pub(crate) represented_locked_dungeon_encounters:
        std::collections::HashSet<(wow_core::ObjectGuid, u32)>,
    /// Session-local per-player money for represented personal encounter loot.
    pub(crate) represented_personal_loot_money:
        std::collections::HashMap<(wow_core::ObjectGuid, wow_core::ObjectGuid), u32>,
    /// Owners whose money must be read from `represented_personal_loot_money`.
    pub(crate) represented_personal_loot_owners: std::collections::HashSet<wow_core::ObjectGuid>,
}
