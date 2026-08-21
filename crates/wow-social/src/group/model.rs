// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Group identity, owned state and storage core.
//!
//! Owns the runtime/database identifier allocators, the `GroupInfo` record,
//! the opaque `GroupRegistry` storage and the database-row load entry points.

use dashmap::DashMap;
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use wow_core::ObjectGuid;

use super::*;

static NEXT_GROUP_ID: AtomicU64 = AtomicU64::new(1);

pub(super) static NEXT_GROUP_DB_STORE_ID: AtomicU32 = AtomicU32::new(1);

pub(super) static GROUP_DB_STORE_ID_ALLOCATOR_LOCK: Mutex<()> = Mutex::new(());

static FREED_GROUP_DB_STORE_IDS: Mutex<Vec<u32>> = Mutex::new(Vec::new());

static GROUP_DB_STORE: Mutex<Vec<Option<u64>>> = Mutex::new(Vec::new());

pub const GROUP_FLAG_RAID_LIKE_CPP: u16 = 0x002;

pub const GROUP_FLAG_LFG_LIKE_CPP: u16 = 0x008;

/// C++ `GroupType` values (`Group.h:86-90`) used by `PlayerData::PartyType`.
pub const GROUP_TYPE_NONE_LIKE_CPP: u8 = 0;

pub const GROUP_TYPE_NORMAL_LIKE_CPP: u8 = 1;

/// C++ `GroupCategory` values (`Group.h:110-116`) represented for HOME/INSTANCE filtering.
pub const GROUP_CATEGORY_HOME_LIKE_CPP: u8 = 0;

pub const GROUP_CATEGORY_INSTANCE_LIKE_CPP: u8 = 1;

pub const MAX_GROUP_CATEGORY_LIKE_CPP: u8 = 2;

pub const LFG_STATE_DUNGEON_LIKE_CPP: u8 = 5;

pub const LFG_STATE_FINISHED_DUNGEON_LIKE_CPP: u8 = 6;

pub(super) fn generate_group_db_store_id_like_cpp() -> u32 {
    let _allocator_guard = GROUP_DB_STORE_ID_ALLOCATOR_LOCK.lock().ok();
    if let Ok(mut freed) = FREED_GROUP_DB_STORE_IDS.lock() {
        if let Some((index, _)) = freed.iter().enumerate().min_by_key(|(_, id)| *id) {
            return freed.swap_remove(index);
        }
    }

    NEXT_GROUP_DB_STORE_ID.fetch_add(1, Ordering::Relaxed)
}

pub(super) fn generate_group_id_like_cpp() -> u64 {
    NEXT_GROUP_ID.fetch_add(1, Ordering::Relaxed)
}

pub(super) fn advance_next_group_db_store_id_after_load_like_cpp(storage_id: u32) {
    let _ = NEXT_GROUP_DB_STORE_ID.compare_exchange(
        storage_id,
        storage_id.saturating_add(1),
        Ordering::Relaxed,
        Ordering::Relaxed,
    );
}

pub(super) fn represented_lfg_db_state_like_cpp(
    group_flags: u16,
    dungeon_id: Option<u32>,
    state: Option<u8>,
) -> Option<GroupLfgDbStateLikeCpp> {
    if (group_flags & GROUP_FLAG_LFG_LIKE_CPP) == 0 {
        return None;
    }

    let dungeon_id = dungeon_id.unwrap_or_default();
    let state = state.unwrap_or_default();
    if dungeon_id == 0 || state == 0 {
        return None;
    }

    Some(GroupLfgDbStateLikeCpp {
        dungeon_id,
        state: match state {
            LFG_STATE_DUNGEON_LIKE_CPP | LFG_STATE_FINISHED_DUNGEON_LIKE_CPP => Some(state),
            _ => None,
        },
    })
}

pub(super) fn free_group_db_store_id_like_cpp(storage_id: u32) {
    if storage_id == 0 {
        return;
    }

    if let Ok(mut store) = GROUP_DB_STORE.lock() {
        if let Some(slot) = store.get_mut(storage_id as usize) {
            *slot = None;
        }
    }

    if let Ok(mut freed) = FREED_GROUP_DB_STORE_IDS.lock() {
        if !freed.contains(&storage_id) {
            freed.push(storage_id);
        }
    }
}

pub(super) fn register_group_db_store_id_like_cpp(storage_id: u32, runtime_group_guid: u64) {
    if let Ok(mut store) = GROUP_DB_STORE.lock() {
        let index = storage_id as usize;
        if index >= store.len() {
            store.resize(index + 1, None);
        }
        store[index] = Some(runtime_group_guid);
    }
}

pub fn group_guid_by_db_store_id_like_cpp(storage_id: u32) -> Option<u64> {
    GROUP_DB_STORE
        .lock()
        .ok()
        .and_then(|store| store.get(storage_id as usize).copied().flatten())
}

/// Loaded-difficulty validation the Group owner needs but does not own.
///
/// C++ resolves these through `sDifficultyStore` inside `Group::LoadGroupFromDB`
/// (`Group.cpp:236-244`). A DB2 store is a data adapter, so `wow-social` states
/// the requirement as a narrow port and the composition root supplies the
/// concrete `wow_data::DifficultyStore` implementation. The methods keep C++'s
/// exact contract: an unusable id falls back to the normal difficulty of its
/// instance kind instead of failing the load.
pub trait GroupDifficultyValidatorLikeCpp {
    /// C++ `Player::CheckLoadedDungeonDifficultyID`.
    fn check_loaded_dungeon_difficulty_id_like_cpp(&self, difficulty: u32) -> u32;
    /// C++ `Player::CheckLoadedRaidDifficultyID`.
    fn check_loaded_raid_difficulty_id_like_cpp(&self, difficulty: u32) -> u32;
    /// C++ `Player::CheckLoadedLegacyRaidDifficultyID`.
    fn check_loaded_legacy_raid_difficulty_id_like_cpp(&self, difficulty: u32) -> u32;
}

/// Character-cache projection used by C++ `Group::LoadMemberFromDB`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupMemberCharacterLikeCpp {
    pub name: String,
    pub race: u8,
    pub class: u8,
}

/// C++ `MemberSlot` subset needed by represented group load/update flows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupMemberSlotLikeCpp {
    pub guid: ObjectGuid,
    pub name: String,
    pub race: u8,
    pub class: u8,
    pub subgroup: u8,
    pub flags: u8,
    pub roles: u8,
    pub ready_checked: bool,
}

/// Row shape selected by C++ `GroupMgr::LoadGroups` for `Group::LoadGroupFromDB`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupDbRowLikeCpp {
    pub leader_guid_low: u64,
    pub loot_method: u8,
    pub looter_guid_low: u64,
    pub loot_threshold: u8,
    pub target_icons: [[u8; 16]; TARGET_ICONS_COUNT_LIKE_CPP],
    pub group_flags: u16,
    pub dungeon_difficulty_id: u32,
    pub raid_difficulty_id: u32,
    pub legacy_raid_difficulty_id: u32,
    pub master_looter_guid_low: u64,
    pub db_store_id: u32,
    pub lfg_dungeon_id: Option<u32>,
    pub lfg_state: Option<u8>,
}

/// Row shape selected by C++ `GroupMgr::LoadGroups` for `Group::LoadMemberFromDB`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupMemberDbRowLikeCpp {
    pub db_store_id: u32,
    pub member_guid_low: u64,
    pub member_flags: u8,
    pub subgroup: u8,
    pub roles: u8,
}

/// C++ `Group::m_recentInstances[mapId] = { instanceOwner, instanceId }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupRecentInstanceLikeCpp {
    pub instance_owner: ObjectGuid,
    pub instance_id: u32,
}

/// Represented C++ `GroupInstanceReference` source identity.
///
/// The real C++ object is a linked back-reference from `Group` to a live
/// `InstanceMap`. Rust does not own the live `InstanceMap` here yet, so this
/// stores the stable map identity that `Group::ResetInstances` needs before the
/// live map reset wiring exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupOwnedInstanceLikeCpp {
    pub map_id: u32,
    pub instance_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupInstanceResetMethodLikeCpp {
    Manual,
    OnChangeDifficulty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupInstanceResetResultLikeCpp {
    Success,
    NotEmpty,
    CannotReset,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupDifficultyKindLikeCpp {
    Dungeon,
    Raid,
    LegacyRaid,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GroupLoadSummaryLikeCpp {
    pub loaded_groups: usize,
    /// C++ increments the loaded-member counter for every member row it reads,
    /// even when the referenced group is missing and only an error is logged.
    pub loaded_member_rows: usize,
    pub loaded_members: usize,
    pub skipped_group_rows: usize,
    pub skipped_member_rows: usize,
}

/// Represented subset restored by C++ `LFGMgr::_LoadFromDB` for LFG groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupLfgDbStateLikeCpp {
    pub dungeon_id: u32,
    /// C++ restores only `LFG_STATE_DUNGEON` and
    /// `LFG_STATE_FINISHED_DUNGEON`; other non-zero states keep the dungeon
    /// but leave LFG state at its default.
    pub state: Option<u8>,
}

/// Information about one group/party.
#[derive(Debug, Clone)]
pub struct GroupInfo {
    pub group_guid: u64,
    /// C++ `Group::m_dbStoreId`: persistent `groups.guid` storage id.
    ///
    /// This is intentionally distinct from `group_guid`/`m_guid`, which is the
    /// runtime ObjectGuid counter. Rust also keeps the represented
    /// `GroupDbStore` index used by `GetGroupByDbStoreId`.
    pub db_store_id: u32,
    pub leader_guid: ObjectGuid,
    /// C++ `Group::GetGroupCategory()` represented category.
    ///
    /// RustyCore currently only creates/loads HOME groups; INSTANCE/original/BG/BF
    /// grouping remains an honest boundary until a real source of truth exists.
    pub group_category: u8,
    /// All member GUIDs (including leader), in join order.
    pub members: Vec<ObjectGuid>,
    /// C++ `Group::m_memberSlots` represented metadata.
    pub member_slots: Vec<GroupMemberSlotLikeCpp>,
    /// C++ `LootMethod` (`Loot.h`): 0=FreeForAll, 1=RoundRobin,
    /// 2=MasterLoot, 3=GroupLoot, 4=NeedBeforeGreed, 5=PersonalLoot.
    pub loot_method: u8,
    pub looter_guid: ObjectGuid,
    pub loot_threshold: u8,
    pub master_looter_guid: ObjectGuid,
    pub dungeon_difficulty_id: u32,
    pub raid_difficulty_id: u32,
    pub legacy_raid_difficulty_id: u32,
    pub target_icons: [[u8; 16]; TARGET_ICONS_COUNT_LIKE_CPP],
    pub raid_markers: [Option<RaidMarkerLikeCpp>; RAID_MARKERS_COUNT_LIKE_CPP],
    pub recent_instances: BTreeMap<u32, GroupRecentInstanceLikeCpp>,
    pub owned_instances: BTreeMap<(u32, u32), GroupOwnedInstanceLikeCpp>,
    pub lfg_db_state: Option<GroupLfgDbStateLikeCpp>,
    /// C++ `LfgGroupData::m_KicksLeft`, restored to `LFG_GROUP_MAX_KICKS`
    /// for LFG-flagged groups. Only the direct uninvite gate consumes it
    /// today (there is no vote-kick flow yet to decrement it).
    pub lfg_kicks_left_like_cpp: u8,
    pub raid_subgroup_counts: Option<[u8; MAX_RAID_SUBGROUPS_LIKE_CPP]>,
    pub ready_check_started: bool,
    /// C++ `Group::m_readyCheckTimer` / duration in milliseconds. Decremented
    /// each shared tick by `update_ready_check_like_cpp`; when <= 0 the ready
    /// check expires and `end_ready_check_like_cpp` fires.
    pub ready_check_timer_ms: i64,
    pub sequence_num: u32,
    pub group_flags: u16,
}

/// Thread-safe owner of all active groups, keyed by group GUID.
///
/// Read access returns owned snapshots so callers cannot retain a backing-map
/// guard. The narrowly retained mutable compatibility methods are removed by
/// the transition issues named on each method (#197/#198/#199).
#[derive(Debug, Default)]
pub struct GroupRegistry {
    pub(super) groups: DashMap<u64, GroupInfo>,
}

pub fn get_group_by_db_store_id_like_cpp(
    registry: &GroupRegistry,
    storage_id: u32,
) -> Option<GroupInfo> {
    let group_guid = group_guid_by_db_store_id_like_cpp(storage_id)?;
    registry.get(&group_guid).map(|group| group.clone())
}

pub fn load_groups_from_db_rows_like_cpp(
    registry: &GroupRegistry,
    group_rows: impl IntoIterator<Item = GroupDbRowLikeCpp>,
    member_rows: impl IntoIterator<Item = GroupMemberDbRowLikeCpp>,
    character_cache: &BTreeMap<u64, GroupMemberCharacterLikeCpp>,
    difficulty_store: &impl GroupDifficultyValidatorLikeCpp,
) -> GroupLoadSummaryLikeCpp {
    let mut summary = GroupLoadSummaryLikeCpp::default();

    for row in group_rows {
        let db_store_id = row.db_store_id;
        let leader = character_cache.get(&row.leader_guid_low).cloned();
        let Some(group) = GroupInfo::load_group_from_db_row_validated_like_cpp(
            generate_group_id_like_cpp(),
            row,
            leader,
            difficulty_store,
        ) else {
            summary.skipped_group_rows += 1;
            continue;
        };

        let runtime_group_guid = group.group_guid;
        registry.register_group_like_cpp(runtime_group_guid, group);
        register_group_db_store_id_like_cpp(db_store_id, runtime_group_guid);
        advance_next_group_db_store_id_after_load_like_cpp(db_store_id);
        summary.loaded_groups += 1;
    }

    for row in member_rows {
        summary.loaded_member_rows += 1;
        let Some(runtime_group_guid) = group_guid_by_db_store_id_like_cpp(row.db_store_id) else {
            summary.skipped_member_rows += 1;
            continue;
        };
        let Some(mut group) = registry.groups.get_mut(&runtime_group_guid) else {
            summary.skipped_member_rows += 1;
            continue;
        };

        let character = character_cache.get(&row.member_guid_low).cloned();
        if group.load_member_from_db_like_cpp(
            row.member_guid_low,
            row.member_flags,
            row.subgroup,
            row.roles,
            character,
        ) {
            summary.loaded_members += 1;
        } else {
            summary.skipped_member_rows += 1;
        }
    }

    summary
}

impl GroupInfo {
    pub fn new(leader: ObjectGuid) -> Self {
        Self {
            group_guid: generate_group_id_like_cpp(),
            db_store_id: generate_group_db_store_id_like_cpp(),
            leader_guid: leader,
            group_category: GROUP_CATEGORY_HOME_LIKE_CPP,
            members: vec![leader],
            member_slots: vec![GroupMemberSlotLikeCpp {
                guid: leader,
                name: String::new(),
                race: 0,
                class: 0,
                subgroup: 0,
                flags: 0,
                roles: 0,
                ready_checked: false,
            }],
            loot_method: LOOT_METHOD_PERSONAL_LIKE_CPP,
            looter_guid: leader,
            loot_threshold: ITEM_QUALITY_UNCOMMON_LIKE_CPP,
            master_looter_guid: ObjectGuid::EMPTY,
            dungeon_difficulty_id: DIFFICULTY_NORMAL_LIKE_CPP,
            raid_difficulty_id: DIFFICULTY_NORMAL_RAID_LIKE_CPP,
            legacy_raid_difficulty_id: DIFFICULTY_10_N_LIKE_CPP,
            target_icons: [EMPTY_TARGET_ICON_RAW_LIKE_CPP; TARGET_ICONS_COUNT_LIKE_CPP],
            raid_markers: [None; RAID_MARKERS_COUNT_LIKE_CPP],
            recent_instances: BTreeMap::new(),
            owned_instances: BTreeMap::new(),
            lfg_db_state: None,
            lfg_kicks_left_like_cpp: 0,
            raid_subgroup_counts: None,
            ready_check_started: false,
            ready_check_timer_ms: 0,
            sequence_num: 1,
            group_flags: 0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn loaded_from_db_like_cpp(
        runtime_group_guid: u64,
        db_store_id: u32,
        leader_guid: ObjectGuid,
        loot_method: u8,
        looter_guid: ObjectGuid,
        loot_threshold: u8,
        group_flags: u16,
        dungeon_difficulty_id: u32,
        raid_difficulty_id: u32,
        legacy_raid_difficulty_id: u32,
        master_looter_guid: ObjectGuid,
    ) -> Self {
        Self {
            group_guid: runtime_group_guid,
            db_store_id,
            leader_guid,
            group_category: GROUP_CATEGORY_HOME_LIKE_CPP,
            members: Vec::new(),
            member_slots: Vec::new(),
            loot_method,
            looter_guid,
            loot_threshold,
            master_looter_guid,
            dungeon_difficulty_id,
            raid_difficulty_id,
            legacy_raid_difficulty_id,
            target_icons: [EMPTY_TARGET_ICON_RAW_LIKE_CPP; TARGET_ICONS_COUNT_LIKE_CPP],
            raid_markers: [None; RAID_MARKERS_COUNT_LIKE_CPP],
            recent_instances: BTreeMap::new(),
            owned_instances: BTreeMap::new(),
            lfg_db_state: None,
            lfg_kicks_left_like_cpp: if (group_flags & GROUP_FLAG_LFG_LIKE_CPP) != 0 {
                LFG_GROUP_MAX_KICKS_LIKE_CPP
            } else {
                0
            },
            raid_subgroup_counts: if (group_flags & GROUP_FLAG_RAID_LIKE_CPP) != 0 {
                Some([0; MAX_RAID_SUBGROUPS_LIKE_CPP])
            } else {
                None
            },
            ready_check_started: false,
            ready_check_timer_ms: 0,
            sequence_num: 1,
            group_flags,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn loaded_from_db_validated_like_cpp(
        runtime_group_guid: u64,
        db_store_id: u32,
        leader_guid: ObjectGuid,
        loot_method: u8,
        looter_guid: ObjectGuid,
        loot_threshold: u8,
        group_flags: u16,
        dungeon_difficulty_id: u32,
        raid_difficulty_id: u32,
        legacy_raid_difficulty_id: u32,
        master_looter_guid: ObjectGuid,
        difficulty_store: &impl GroupDifficultyValidatorLikeCpp,
    ) -> Self {
        Self::loaded_from_db_like_cpp(
            runtime_group_guid,
            db_store_id,
            leader_guid,
            loot_method,
            looter_guid,
            loot_threshold,
            group_flags,
            difficulty_store.check_loaded_dungeon_difficulty_id_like_cpp(dungeon_difficulty_id),
            difficulty_store.check_loaded_raid_difficulty_id_like_cpp(raid_difficulty_id),
            difficulty_store
                .check_loaded_legacy_raid_difficulty_id_like_cpp(legacy_raid_difficulty_id),
            master_looter_guid,
        )
    }

    pub fn load_group_from_db_row_validated_like_cpp(
        runtime_group_guid: u64,
        row: GroupDbRowLikeCpp,
        leader: Option<GroupMemberCharacterLikeCpp>,
        difficulty_store: &impl GroupDifficultyValidatorLikeCpp,
    ) -> Option<Self> {
        leader?;
        let leader_guid = ObjectGuid::create_player(1, i64::try_from(row.leader_guid_low).ok()?);
        let looter_guid = ObjectGuid::create_player(1, i64::try_from(row.looter_guid_low).ok()?);
        let master_looter_guid =
            ObjectGuid::create_player(1, i64::try_from(row.master_looter_guid_low).ok()?);

        let mut group = Self::loaded_from_db_validated_like_cpp(
            runtime_group_guid,
            row.db_store_id,
            leader_guid,
            row.loot_method,
            looter_guid,
            row.loot_threshold,
            row.group_flags,
            row.dungeon_difficulty_id,
            row.raid_difficulty_id,
            row.legacy_raid_difficulty_id,
            master_looter_guid,
            difficulty_store,
        );
        group.target_icons = row.target_icons;
        group.lfg_db_state =
            represented_lfg_db_state_like_cpp(row.group_flags, row.lfg_dungeon_id, row.lfg_state);
        Some(group)
    }

    pub fn group_category_like_cpp(&self) -> u8 {
        self.group_category
    }

    pub fn recent_instance_owner_like_cpp(&self, map_id: u32) -> ObjectGuid {
        self.recent_instances
            .get(&map_id)
            .map(|recent| recent.instance_owner)
            .unwrap_or(self.leader_guid)
    }

    pub fn recent_instance_id_like_cpp(&self, map_id: u32) -> u32 {
        self.recent_instances
            .get(&map_id)
            .map(|recent| recent.instance_id)
            .unwrap_or(0)
    }

    pub fn set_recent_instance_like_cpp(
        &mut self,
        map_id: u32,
        instance_owner: ObjectGuid,
        instance_id: u32,
    ) {
        self.recent_instances.insert(
            map_id,
            GroupRecentInstanceLikeCpp {
                instance_owner,
                instance_id,
            },
        );
    }

    pub fn forget_recent_instance_like_cpp(&mut self, map_id: u32) -> bool {
        self.recent_instances.remove(&map_id).is_some()
    }

    pub fn link_owned_instance_like_cpp(&mut self, map_id: u32, instance_id: u32) -> bool {
        self.owned_instances
            .insert(
                (map_id, instance_id),
                GroupOwnedInstanceLikeCpp {
                    map_id,
                    instance_id,
                },
            )
            .is_none()
    }

    pub fn unlink_owned_instance_like_cpp(&mut self, map_id: u32, instance_id: u32) -> bool {
        self.owned_instances
            .remove(&(map_id, instance_id))
            .is_some()
    }

    pub fn owned_instances_like_cpp(&self) -> impl Iterator<Item = GroupOwnedInstanceLikeCpp> + '_ {
        self.owned_instances.values().copied()
    }

    pub fn apply_owned_instance_reset_result_like_cpp(
        &mut self,
        map_id: u32,
        result: GroupInstanceResetResultLikeCpp,
        method: GroupInstanceResetMethodLikeCpp,
    ) -> bool {
        match result {
            GroupInstanceResetResultLikeCpp::Success
            | GroupInstanceResetResultLikeCpp::CannotReset => {
                self.forget_recent_instance_like_cpp(map_id)
            }
            GroupInstanceResetResultLikeCpp::NotEmpty
                if method == GroupInstanceResetMethodLikeCpp::OnChangeDifficulty =>
            {
                self.forget_recent_instance_like_cpp(map_id)
            }
            GroupInstanceResetResultLikeCpp::NotEmpty | GroupInstanceResetResultLikeCpp::Other => {
                false
            }
        }
    }

    pub fn matches_party_index_like_cpp(&self, party_index: Option<u8>) -> bool {
        match party_index {
            None => true,
            Some(index) if index < MAX_GROUP_CATEGORY_LIKE_CPP => self.group_category == index,
            Some(_) => false,
        }
    }

    pub fn is_raid_group(&self) -> bool {
        (self.group_flags & GROUP_FLAG_RAID_LIKE_CPP) != 0
    }

    pub fn is_full_like_cpp(&self) -> bool {
        let max_members = if self.is_raid_group() {
            MAX_RAID_SIZE_LIKE_CPP
        } else {
            MAX_GROUP_SIZE_LIKE_CPP
        };
        self.members.len() >= max_members
    }

    pub fn is_lfg_group_like_cpp(&self) -> bool {
        (self.group_flags & GROUP_FLAG_LFG_LIKE_CPP) != 0
    }

    pub fn is_empty(&self) -> bool {
        self.members.len() < 2
    }
}

impl GroupRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Return an immutable owned snapshot of one group.
    pub fn get(&self, group_guid: &u64) -> Option<GroupInfo> {
        self.groups.get(group_guid).map(|group| group.clone())
    }

    /// Return whether the identified group currently exists.
    pub fn contains_key(&self, group_guid: &u64) -> bool {
        self.groups.contains_key(group_guid)
    }

    /// Return immutable owned snapshots of every current group.
    pub fn snapshots(&self) -> Vec<GroupInfo> {
        self.groups
            .iter()
            .map(|group| group.value().clone())
            .collect()
    }

    /// Register a fully materialized group while preserving runtime/database
    /// identity invariants. Database loading and fixtures use this boundary;
    /// callers never receive a backing-map entry guard.
    pub fn register_group_like_cpp(&self, group_guid: u64, group: GroupInfo) -> Option<GroupInfo> {
        assert_eq!(
            group_guid, group.group_guid,
            "group registry key must match group identity"
        );
        let db_store_id = group.db_store_id;
        register_group_db_store_id_like_cpp(db_store_id, group_guid);
        let previous = self.groups.insert(group_guid, group);
        if let Some(previous) = previous.as_ref()
            && previous.db_store_id != db_store_id
        {
            free_group_db_store_id_like_cpp(previous.db_store_id);
        }
        previous
    }

    /// Explicit teardown used by lifecycle cleanup and fixtures. Storage-id
    /// publication is retired together with the group.
    pub fn unregister_group_like_cpp(&self, group_guid: &u64) -> Option<GroupInfo> {
        let (_, group) = self.groups.remove(group_guid)?;
        free_group_db_store_id_like_cpp(group.db_store_id);
        Some(group)
    }
}
