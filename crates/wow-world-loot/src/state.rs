use std::collections::{HashMap, HashSet};
#[cfg(any(test, feature = "test-fixtures"))]
use std::sync::{Arc, atomic::AtomicUsize};
use std::time::Duration;

use wow_core::ObjectGuid;
use wow_loot::OwnedLootAuthority;
use wow_packet::packets::loot::CreatureLoot;

#[cfg(any(test, feature = "test-fixtures"))]
use crate::RepresentedLootRollCriteriaEvent;
use crate::RepresentedLootRollState;

mod active_views;
mod authority;
mod authority_access;
mod cache_access;
mod claims;
mod corpse;
mod creature;
mod fanout;
#[cfg(any(test, feature = "test-fixtures"))]
mod fixtures;
mod gameobject;
mod item_retirement;
mod item_storage;
mod money;
mod object_transitions;
mod player_settings;
mod request_cache;
mod request_context;
mod request_state;
mod roll_access;
mod roll_publication;
mod rolls;

const REMOTE_MASTER_LOOT_COMMAND_TIMEOUT: Duration = Duration::from_millis(250);
const DISENCHANT_LOOT_ROLL_CRITERIA_SPELL_LIKE_CPP: u32 = 13_262;

/// Loot windows and AE-loot views with their authorities and generations, loot rolls, personal loot
/// money and the loot test hooks.
pub struct LootState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) pass_on_group_loot: bool,
    /// C++ ActivePlayerData::LootSpecID represented session state.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) loot_specialization_id: u32,

    // ── Loot ──────────────────────────────────────────────────────
    /// Active loot windows keyed by creature GUID.
    pub(crate) loot_table: HashMap<ObjectGuid, CreatureLoot>,
    /// Object-owned loot generation represented by each session-local packet cache.
    pub(crate) represented_loot_cache_generations_like_cpp: HashMap<ObjectGuid, u64>,
    /// Mirrors C++ PlayerData::LootTargetGUID for guards that compare active loot by GUID.
    pub(crate) active_loot_guid: ObjectGuid,
    /// Represented owner GUIDs currently visible through C++ `Player::m_AELootView`.
    pub(crate) active_loot_view_owners: HashSet<ObjectGuid>,
    /// Object-owned generation that was actually opened for each active loot view.
    ///
    /// GUIDs are reused across creature respawns and gameobject restocks.  A delayed
    /// packet from an older window must therefore not be authorized merely because
    /// the replacement lifetime has the same owner/loot GUID and player eligibility.
    pub(crate) active_loot_view_generations_like_cpp: HashMap<ObjectGuid, u64>,
    /// Exact backing allocation opened for each view. Scope epochs restart at
    /// one in a newly allocated authority, so the generation map alone cannot
    /// prevent ABA when a creature GUID is recreated.
    pub(crate) active_loot_view_authorities_like_cpp: HashMap<ObjectGuid, OwnedLootAuthority>,
    /// Represented pending group/NBG loot rolls keyed by `(LootObj, LootListID)`.
    pub(crate) represented_loot_rolls: HashMap<(ObjectGuid, u8), RepresentedLootRollState>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) loot_item_store_test_grants_like_cpp: Option<Arc<AtomicUsize>>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) loot_item_store_test_success_like_cpp: bool,
    /// Optional test-only COMMIT gate for exercising remote command timeout
    /// and release while the authority claim is already persistence-owned.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) loot_item_store_test_commit_gate_like_cpp: Option<Arc<tokio::sync::Notify>>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_loot_roll_criteria_events: Vec<RepresentedLootRollCriteriaEvent>,
    /// Session-local representation of `GameObject::m_unique_users` for no-GetLootId chest uses.
    pub(crate) represented_unique_gameobject_uses: HashSet<ObjectGuid>,
    /// Session-local representation of `GameObject::m_tapList` for personal encounter loot.
    pub(crate) represented_gameobject_tap_lists: HashMap<ObjectGuid, Vec<ObjectGuid>>,
    /// Handle-less compatibility for represented encounter-loot fixtures.
    /// Production `Player::IsLockedToDungeonEncounter` resolves the immutable
    /// DungeonEncounter catalog and shared InstanceLockMgr.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_locked_dungeon_encounters: HashSet<(ObjectGuid, u32)>,
    /// Session-local per-player money for represented personal encounter loot.
    pub(crate) represented_personal_loot_money: HashMap<(ObjectGuid, ObjectGuid), u32>,
    /// Owners whose money must be read from `represented_personal_loot_money`.
    pub(crate) represented_personal_loot_owners: HashSet<ObjectGuid>,
}

impl LootState {
    pub fn new_like_cpp() -> Self {
        Self {
            #[cfg(any(test, feature = "test-fixtures"))]
            pass_on_group_loot: false,
            #[cfg(any(test, feature = "test-fixtures"))]
            loot_specialization_id: 0,
            loot_table: std::collections::HashMap::new(),
            represented_loot_cache_generations_like_cpp: std::collections::HashMap::new(),
            active_loot_guid: ObjectGuid::EMPTY,
            active_loot_view_owners: std::collections::HashSet::new(),
            active_loot_view_generations_like_cpp: std::collections::HashMap::new(),
            active_loot_view_authorities_like_cpp: std::collections::HashMap::new(),
            represented_loot_rolls: std::collections::HashMap::new(),
            #[cfg(any(test, feature = "test-fixtures"))]
            loot_item_store_test_grants_like_cpp: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            loot_item_store_test_success_like_cpp: true,
            #[cfg(any(test, feature = "test-fixtures"))]
            loot_item_store_test_commit_gate_like_cpp: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_loot_roll_criteria_events: Vec::new(),
            represented_unique_gameobject_uses: std::collections::HashSet::new(),
            represented_gameobject_tap_lists: std::collections::HashMap::new(),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_locked_dungeon_encounters: std::collections::HashSet::new(),
            represented_personal_loot_money: std::collections::HashMap::new(),
            represented_personal_loot_owners: std::collections::HashSet::new(),
        }
    }
}
