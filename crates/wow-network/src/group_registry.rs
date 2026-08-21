//! Shared registry of active groups for cross-session party management.

use dashmap::{DashMap, mapref::entry::Entry};
use std::{
    collections::BTreeMap,
    sync::{
        Mutex, MutexGuard,
        atomic::{AtomicU32, AtomicU64, Ordering},
    },
};
use wow_core::{ObjectGuid, Position};
use wow_data::DifficultyStore;

static NEXT_GROUP_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_GROUP_DB_STORE_ID: AtomicU32 = AtomicU32::new(1);
static GROUP_DB_STORE_ID_ALLOCATOR_LOCK: Mutex<()> = Mutex::new(());
static FREED_GROUP_DB_STORE_IDS: Mutex<Vec<u32>> = Mutex::new(Vec::new());
static GROUP_DB_STORE: Mutex<Vec<Option<u64>>> = Mutex::new(Vec::new());

pub const GROUP_FLAG_RAID_LIKE_CPP: u16 = 0x002;
pub const GROUP_FLAG_LFG_LIKE_CPP: u16 = 0x008;
/// C++ `GROUP_FLAG_DESTROYED` (`Group.h:100`): marks the `PartyUpdate` that
/// tears down the removed member's party frames.
pub const GROUP_FLAG_DESTROYED_LIKE_CPP: u16 = 0x010;
/// C++ `LFG_GROUP_MAX_KICKS` (`LFGGroupData.h:28`): kicks each LFG group
/// starts with; only the vote-kick flow decrements it there.
pub const LFG_GROUP_MAX_KICKS_LIKE_CPP: u8 = 3;
/// C++ `LFG_GROUP_KICK_VOTES_NEEDED` (`LFGMgr.h:62`): an LFG kick requires
/// strictly more members than this.
pub const LFG_GROUP_KICK_VOTES_NEEDED_LIKE_CPP: usize = 3;
pub const GROUP_FLAG_EVERYONE_ASSISTANT_LIKE_CPP: u16 = 0x040;
pub const MEMBER_FLAG_ASSISTANT_LIKE_CPP: u8 = 0x01;
pub const MEMBER_FLAG_MAINTANK_LIKE_CPP: u8 = 0x02;
pub const MEMBER_FLAG_MAINASSIST_LIKE_CPP: u8 = 0x04;
pub const GROUP_ASSIGN_MAINTANK_LIKE_CPP: u8 = 0;
pub const GROUP_ASSIGN_MAINASSIST_LIKE_CPP: u8 = 1;
/// C++ `GroupType` values (`Group.h:86-90`) used by `PlayerData::PartyType`.
pub const GROUP_TYPE_NONE_LIKE_CPP: u8 = 0;
pub const GROUP_TYPE_NORMAL_LIKE_CPP: u8 = 1;
/// C++ `GroupCategory` values (`Group.h:110-116`) represented for HOME/INSTANCE filtering.
pub const GROUP_CATEGORY_HOME_LIKE_CPP: u8 = 0;
pub const GROUP_CATEGORY_INSTANCE_LIKE_CPP: u8 = 1;
pub const MAX_GROUP_CATEGORY_LIKE_CPP: u8 = 2;
pub const LOOT_METHOD_FREE_FOR_ALL_LIKE_CPP: u8 = 0;
pub const LOOT_METHOD_PERSONAL_LIKE_CPP: u8 = 5;
pub const ITEM_QUALITY_UNCOMMON_LIKE_CPP: u8 = 2;
pub const DIFFICULTY_NORMAL_LIKE_CPP: u32 = 1;
pub const DIFFICULTY_NORMAL_RAID_LIKE_CPP: u32 = 14;
pub const DIFFICULTY_10_N_LIKE_CPP: u32 = 3;
pub const TARGET_ICONS_COUNT_LIKE_CPP: usize = 8;
pub const EMPTY_TARGET_ICON_RAW_LIKE_CPP: [u8; 16] = [0; 16];
pub const RAID_MARKERS_COUNT_LIKE_CPP: usize = 8;
pub const LFG_STATE_DUNGEON_LIKE_CPP: u8 = 5;
pub const LFG_STATE_FINISHED_DUNGEON_LIKE_CPP: u8 = 6;
pub const MAX_GROUP_SIZE_LIKE_CPP: usize = 5;
pub const MAX_RAID_SIZE_LIKE_CPP: usize = 40;
pub const MAX_RAID_SUBGROUPS_LIKE_CPP: usize = MAX_RAID_SIZE_LIKE_CPP / MAX_GROUP_SIZE_LIKE_CPP;
pub const MISSING_MEMBER_GROUP_LIKE_CPP: u8 = (MAX_RAID_SUBGROUPS_LIKE_CPP as u8) + 1;
pub const READYCHECK_DURATION_MS_LIKE_CPP: i64 = 35_000;

fn generate_group_db_store_id_like_cpp() -> u32 {
    let _allocator_guard = GROUP_DB_STORE_ID_ALLOCATOR_LOCK.lock().ok();
    if let Ok(mut freed) = FREED_GROUP_DB_STORE_IDS.lock() {
        if let Some((index, _)) = freed.iter().enumerate().min_by_key(|(_, id)| *id) {
            return freed.swap_remove(index);
        }
    }

    NEXT_GROUP_DB_STORE_ID.fetch_add(1, Ordering::Relaxed)
}

fn generate_group_id_like_cpp() -> u64 {
    NEXT_GROUP_ID.fetch_add(1, Ordering::Relaxed)
}

fn advance_next_group_db_store_id_after_load_like_cpp(storage_id: u32) {
    let _ = NEXT_GROUP_DB_STORE_ID.compare_exchange(
        storage_id,
        storage_id.saturating_add(1),
        Ordering::Relaxed,
        Ordering::Relaxed,
    );
}

fn represented_lfg_db_state_like_cpp(
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

pub fn free_group_db_store_id_like_cpp(storage_id: u32) {
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

pub fn register_group_db_store_id_like_cpp(storage_id: u32, runtime_group_guid: u64) {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadyCheckEventLikeCpp {
    Started {
        party_index: u8,
        party_guid: u64,
        initiator_guid: ObjectGuid,
        duration_ms: i64,
    },
    Response {
        party_guid: u64,
        player: ObjectGuid,
        is_ready: bool,
    },
    Completed {
        party_index: u8,
        party_guid: u64,
    },
}

/// C++ `RaidMarker` (`Group.h`): one world position plus optional transport.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RaidMarkerLikeCpp {
    pub map_id: u32,
    pub position: Position,
    pub transport_guid: ObjectGuid,
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
        difficulty_store: &DifficultyStore,
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
        difficulty_store: &DifficultyStore,
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

    pub fn add_member(&mut self, guid: ObjectGuid) -> bool {
        if self.members.contains(&guid) {
            return false;
        }

        let subgroup = if let Some(counts) = self.raid_subgroup_counts {
            let Some((index, _)) = counts
                .iter()
                .enumerate()
                .find(|(_, count)| usize::from(**count) < MAX_GROUP_SIZE_LIKE_CPP)
            else {
                return false;
            };
            index as u8
        } else {
            0
        };
        if !self.subgroup_counter_increase_like_cpp(subgroup) {
            return false;
        }
        self.members.push(guid);
        self.member_slots.push(GroupMemberSlotLikeCpp {
            guid,
            name: String::new(),
            race: 0,
            class: 0,
            subgroup,
            flags: 0,
            roles: 0,
            ready_checked: false,
        });
        self.sequence_num += 1;
        true
    }

    pub fn member_slot_like_cpp(&self, guid: ObjectGuid) -> Option<&GroupMemberSlotLikeCpp> {
        self.member_slots.iter().find(|slot| slot.guid == guid)
    }

    pub fn member_group_like_cpp(&self, guid: ObjectGuid) -> u8 {
        self.member_slot_like_cpp(guid)
            .map(|slot| slot.subgroup)
            .unwrap_or(MISSING_MEMBER_GROUP_LIKE_CPP)
    }

    pub fn get_lfg_roles_like_cpp(&self, guid: ObjectGuid) -> u8 {
        self.member_slot_like_cpp(guid)
            .map(|slot| slot.roles)
            .unwrap_or_default()
    }

    pub fn set_lfg_roles_like_cpp(&mut self, guid: ObjectGuid, roles: u8) -> bool {
        let Some(slot) = self.member_slots.iter_mut().find(|slot| slot.guid == guid) else {
            return false;
        };
        slot.roles = roles;
        true
    }

    pub fn has_free_slot_sub_group_like_cpp(&self, subgroup: u8) -> bool {
        let Some(counts) = self.raid_subgroup_counts else {
            return false;
        };
        counts
            .get(usize::from(subgroup))
            .is_some_and(|count| usize::from(*count) < MAX_GROUP_SIZE_LIKE_CPP)
    }

    pub fn swap_members_groups_like_cpp(
        &mut self,
        first: ObjectGuid,
        second: ObjectGuid,
    ) -> Option<[(ObjectGuid, u8); 2]> {
        if !self.is_raid_group() {
            return None;
        }

        let first_index = self
            .member_slots
            .iter()
            .position(|slot| slot.guid == first)?;
        let second_index = self
            .member_slots
            .iter()
            .position(|slot| slot.guid == second)?;
        if first_index == second_index {
            return None;
        }

        let first_subgroup = self.member_slots[first_index].subgroup;
        let second_subgroup = self.member_slots[second_index].subgroup;
        if first_subgroup == second_subgroup {
            return None;
        }

        self.member_slots[first_index].subgroup = second_subgroup;
        self.member_slots[second_index].subgroup = first_subgroup;
        self.sequence_num += 1;

        Some([(first, second_subgroup), (second, first_subgroup)])
    }

    pub fn remove_unique_group_member_flag_like_cpp(&mut self, flag: u8) -> bool {
        if !matches!(
            flag,
            MEMBER_FLAG_MAINTANK_LIKE_CPP | MEMBER_FLAG_MAINASSIST_LIKE_CPP
        ) {
            return false;
        }

        let mut changed = false;
        for slot in &mut self.member_slots {
            if (slot.flags & flag) != 0 {
                slot.flags &= !flag;
                changed = true;
            }
        }
        if changed {
            self.sequence_num += 1;
        }
        changed
    }

    pub fn set_group_member_flag_updates_like_cpp(
        &mut self,
        guid: ObjectGuid,
        apply: bool,
        flag: u8,
    ) -> Option<Vec<(ObjectGuid, u8)>> {
        if !self.is_raid_group() {
            return None;
        }

        let slot_index = self
            .member_slots
            .iter()
            .position(|slot| slot.guid == guid)?;
        match flag {
            MEMBER_FLAG_ASSISTANT_LIKE_CPP
            | MEMBER_FLAG_MAINTANK_LIKE_CPP
            | MEMBER_FLAG_MAINASSIST_LIKE_CPP => {}
            _ => return None,
        }

        let previous_member_flags: Vec<(ObjectGuid, u8)> = self
            .member_slots
            .iter()
            .map(|slot| (slot.guid, slot.flags))
            .collect();
        if matches!(
            flag,
            MEMBER_FLAG_MAINTANK_LIKE_CPP | MEMBER_FLAG_MAINASSIST_LIKE_CPP
        ) {
            for slot in &mut self.member_slots {
                slot.flags &= !flag;
            }
        }

        if apply {
            self.member_slots[slot_index].flags |= flag;
        } else {
            self.member_slots[slot_index].flags &= !flag;
        }

        let changed = self.member_slots.iter().any(|slot| {
            previous_member_flags
                .iter()
                .any(|(guid, flags)| *guid == slot.guid && *flags != slot.flags)
        });
        if changed {
            self.sequence_num += 1;
        }

        Some(vec![(guid, self.member_slots[slot_index].flags)])
    }

    pub fn set_group_member_flag_like_cpp(
        &mut self,
        guid: ObjectGuid,
        apply: bool,
        flag: u8,
    ) -> Option<u8> {
        self.set_group_member_flag_updates_like_cpp(guid, apply, flag)
            .and_then(|updates| {
                updates
                    .into_iter()
                    .find_map(|(member_guid, flags)| (member_guid == guid).then_some(flags))
            })
    }

    pub fn set_assistant_leader_flag_like_cpp(
        &mut self,
        guid: ObjectGuid,
        apply: bool,
    ) -> Option<u8> {
        self.set_group_member_flag_like_cpp(guid, apply, MEMBER_FLAG_ASSISTANT_LIKE_CPP)
    }

    pub fn target_icon_list_like_cpp(&self) -> Vec<(u8, ObjectGuid)> {
        self.target_icons
            .iter()
            .enumerate()
            .map(|(symbol, raw)| (symbol as u8, ObjectGuid::from_raw_bytes(raw)))
            .collect()
    }

    pub fn active_raid_markers_mask_like_cpp(&self) -> u32 {
        self.raid_markers
            .iter()
            .enumerate()
            .fold(0u32, |mask, (index, marker)| {
                if marker.is_some() {
                    mask | (1u32 << index)
                } else {
                    mask
                }
            })
    }

    pub fn raid_marker_list_like_cpp(&self) -> Vec<RaidMarkerLikeCpp> {
        self.raid_markers.iter().flatten().copied().collect()
    }

    /// C++ `Group::AddRaidMarker`: ignore ids outside `[0, 8)` and occupied slots.
    pub fn add_raid_marker_like_cpp(
        &mut self,
        marker_id: u8,
        map_id: u32,
        position: Position,
        transport_guid: ObjectGuid,
    ) -> bool {
        let index = usize::from(marker_id);
        let Some(slot) = self.raid_markers.get_mut(index) else {
            return false;
        };
        if slot.is_some() {
            return false;
        }
        *slot = Some(RaidMarkerLikeCpp {
            map_id,
            position,
            transport_guid,
        });
        true
    }

    /// C++ `Group::DeleteRaidMarker`: ids `0..=7` clear one slot, `8` clears all.
    pub fn delete_raid_marker_like_cpp(&mut self, marker_id: u8) -> bool {
        if usize::from(marker_id) > RAID_MARKERS_COUNT_LIKE_CPP {
            return false;
        }

        let mut changed = false;
        for (index, marker) in self.raid_markers.iter_mut().enumerate() {
            if marker.is_some()
                && (usize::from(marker_id) == index
                    || usize::from(marker_id) == RAID_MARKERS_COUNT_LIKE_CPP)
            {
                *marker = None;
                changed = true;
            }
        }
        changed
    }

    pub fn looter_guid_like_cpp(&self) -> ObjectGuid {
        if self.loot_method == LOOT_METHOD_FREE_FOR_ALL_LIKE_CPP {
            ObjectGuid::EMPTY
        } else {
            self.looter_guid
        }
    }

    /// C++ `Group::UpdateLooterGuid`.
    ///
    /// The caller supplies the represented equivalent of
    /// `ObjectAccessor::FindPlayer(slot.guid)` plus
    /// `Player::IsAtGroupRewardDistance(pLootedObject)`: only members present in
    /// `eligible_reward_distance_members` are treated as found and close enough.
    /// The bool return means C++ would call `SendUpdate()`.
    pub fn update_looter_guid_like_cpp(
        &mut self,
        eligible_reward_distance_members: impl IntoIterator<Item = ObjectGuid>,
        ifneed: bool,
    ) -> bool {
        if self.loot_method == LOOT_METHOD_FREE_FOR_ALL_LIKE_CPP {
            return false;
        }

        let eligible: Vec<ObjectGuid> = eligible_reward_distance_members.into_iter().collect();
        let old_looter_guid = self.looter_guid_like_cpp();
        let current_index = self
            .member_slots
            .iter()
            .position(|slot| slot.guid == old_looter_guid);
        let start_index = match current_index {
            Some(index) => {
                if ifneed && eligible.contains(&old_looter_guid) {
                    return false;
                }
                index.saturating_add(1)
            }
            None => 0,
        };

        let new_looter = self.member_slots[start_index..]
            .iter()
            .chain(self.member_slots[..start_index].iter())
            .find_map(|slot| eligible.contains(&slot.guid).then_some(slot.guid));

        if new_looter == Some(old_looter_guid) {
            return false;
        }

        if let Some(new_looter) = new_looter {
            self.looter_guid = new_looter;
            self.sequence_num += 1;
            true
        } else {
            self.looter_guid = ObjectGuid::EMPTY;
            self.sequence_num += 1;
            true
        }
    }

    pub fn set_target_icon_like_cpp(
        &mut self,
        symbol: u8,
        target: ObjectGuid,
    ) -> Option<Vec<(u8, ObjectGuid)>> {
        let symbol_index = usize::from(symbol);
        if symbol_index >= TARGET_ICONS_COUNT_LIKE_CPP {
            return None;
        }

        let mut updates = Vec::new();
        if !target.is_empty() {
            let target_raw = target.to_raw_bytes();
            for clear_symbol in 0..TARGET_ICONS_COUNT_LIKE_CPP {
                if self.target_icons[clear_symbol] == target_raw {
                    self.target_icons[clear_symbol] = EMPTY_TARGET_ICON_RAW_LIKE_CPP;
                    updates.push((clear_symbol as u8, ObjectGuid::EMPTY));
                }
            }
        }

        self.target_icons[symbol_index] = target.to_raw_bytes();
        updates.push((symbol, target));
        self.sequence_num += 1;
        Some(updates)
    }

    pub fn is_leader_like_cpp(&self, guid: ObjectGuid) -> bool {
        self.leader_guid == guid
    }

    pub fn is_assistant_like_cpp(&self, guid: ObjectGuid) -> bool {
        self.member_slots
            .iter()
            .any(|slot| slot.guid == guid && (slot.flags & MEMBER_FLAG_ASSISTANT_LIKE_CPP) != 0)
    }

    pub fn change_leader_like_cpp(&mut self, new_leader_guid: ObjectGuid) -> Option<u8> {
        let slot = self
            .member_slots
            .iter_mut()
            .find(|slot| slot.guid == new_leader_guid)?;
        let previous_leader = self.leader_guid;
        let previous_flags = slot.flags;

        self.leader_guid = new_leader_guid;
        slot.flags &= !MEMBER_FLAG_ASSISTANT_LIKE_CPP;
        if previous_leader != new_leader_guid || previous_flags != slot.flags {
            self.sequence_num += 1;
        }

        Some(slot.flags)
    }

    pub fn set_everyone_is_assistant_like_cpp(&mut self, apply: bool) -> (u16, u32) {
        let previous_group_flags = self.group_flags;
        if apply {
            self.group_flags |= GROUP_FLAG_EVERYONE_ASSISTANT_LIKE_CPP;
        } else {
            self.group_flags &= !GROUP_FLAG_EVERYONE_ASSISTANT_LIKE_CPP;
        }

        let mut changed = self.group_flags != previous_group_flags;
        for slot in &mut self.member_slots {
            let previous_flags = slot.flags;
            if apply {
                slot.flags |= MEMBER_FLAG_ASSISTANT_LIKE_CPP;
            } else {
                slot.flags &= !MEMBER_FLAG_ASSISTANT_LIKE_CPP;
            }
            changed |= slot.flags != previous_flags;
        }

        if changed {
            self.sequence_num += 1;
        }

        (self.group_flags, self.db_store_id)
    }

    pub fn change_member_group_like_cpp(&mut self, guid: ObjectGuid, subgroup: u8) -> bool {
        if !self.is_raid_group() {
            return false;
        }
        if usize::from(subgroup) >= MAX_RAID_SUBGROUPS_LIKE_CPP {
            return false;
        }
        if !self.has_free_slot_sub_group_like_cpp(subgroup) {
            return false;
        }

        let Some(slot_index) = self.member_slots.iter().position(|slot| slot.guid == guid) else {
            return false;
        };
        let previous_subgroup = self.member_slots[slot_index].subgroup;
        if previous_subgroup == subgroup {
            return false;
        }

        self.member_slots[slot_index].subgroup = subgroup;
        self.subgroup_counter_increase_like_cpp(subgroup);
        self.subgroup_counter_decrease_like_cpp(previous_subgroup);
        self.sequence_num += 1;
        true
    }

    fn subgroup_counter_increase_like_cpp(&mut self, subgroup: u8) -> bool {
        let Some(counts) = self.raid_subgroup_counts.as_mut() else {
            return true;
        };
        let Some(count) = counts.get_mut(usize::from(subgroup)) else {
            return false;
        };
        *count = count.saturating_add(1);
        true
    }

    fn subgroup_counter_decrease_like_cpp(&mut self, subgroup: u8) {
        let Some(counts) = self.raid_subgroup_counts.as_mut() else {
            return;
        };
        if let Some(count) = counts.get_mut(usize::from(subgroup)) {
            *count = count.saturating_sub(1);
        }
    }

    fn init_raid_subgroups_counter_like_cpp(&mut self) {
        let mut counts = [0u8; MAX_RAID_SUBGROUPS_LIKE_CPP];
        for slot in &self.member_slots {
            if let Some(count) = counts.get_mut(usize::from(slot.subgroup)) {
                *count = (*count).saturating_add(1);
            }
        }
        self.raid_subgroup_counts = Some(counts);
    }

    pub fn load_member_from_db_like_cpp(
        &mut self,
        guid_low: u64,
        mut member_flags: u8,
        subgroup: u8,
        roles: u8,
        character: Option<GroupMemberCharacterLikeCpp>,
    ) -> bool {
        let Some(character) = character else {
            return false;
        };

        if (self.group_flags & GROUP_FLAG_EVERYONE_ASSISTANT_LIKE_CPP) != 0 {
            member_flags |= MEMBER_FLAG_ASSISTANT_LIKE_CPP;
        }

        let Ok(guid_db_id) = i64::try_from(guid_low) else {
            return false;
        };
        if self.raid_subgroup_counts.is_some()
            && usize::from(subgroup) >= MAX_RAID_SUBGROUPS_LIKE_CPP
        {
            return false;
        }
        let guid = ObjectGuid::create_player(1, guid_db_id);
        if let Some(slot) = self.member_slots.iter().find(|slot| slot.guid == guid) {
            self.subgroup_counter_decrease_like_cpp(slot.subgroup);
        }
        self.members.retain(|member_guid| *member_guid != guid);
        self.member_slots.retain(|slot| slot.guid != guid);
        if !self.subgroup_counter_increase_like_cpp(subgroup) {
            return false;
        }
        self.members.push(guid);
        self.member_slots.push(GroupMemberSlotLikeCpp {
            guid,
            name: character.name,
            race: character.race,
            class: character.class,
            subgroup,
            flags: member_flags,
            roles,
            ready_checked: false,
        });
        true
    }

    pub fn reset_member_ready_checked_like_cpp(&mut self) {
        for slot in &mut self.member_slots {
            slot.ready_checked = false;
        }
    }

    pub fn is_ready_check_completed_like_cpp(&self) -> bool {
        self.member_slots.iter().all(|slot| slot.ready_checked)
    }

    fn end_ready_check_like_cpp(&mut self, events: &mut Vec<ReadyCheckEventLikeCpp>) {
        if !self.ready_check_started {
            return;
        }

        self.ready_check_started = false;
        self.ready_check_timer_ms = 0;
        self.reset_member_ready_checked_like_cpp();
        events.push(ReadyCheckEventLikeCpp::Completed {
            party_index: 0,
            party_guid: self.group_guid,
        });
    }

    /// C++ `Group::UpdateReadyCheck(uint32 diff)` at Group.cpp:1445-1453.
    ///
    /// NOOP when no ready check is active. Otherwise subtracts `diff_ms` from
    /// the timer and, if it has expired (<= 0), calls `end_ready_check_like_cpp`
    /// which resets all state and emits exactly one `Completed` event.
    pub fn update_ready_check_like_cpp(&mut self, diff_ms: u32) -> Vec<ReadyCheckEventLikeCpp> {
        if !self.ready_check_started {
            return Vec::new();
        }

        self.ready_check_timer_ms -= i64::from(diff_ms);
        if self.ready_check_timer_ms <= 0 {
            let mut events = Vec::new();
            self.end_ready_check_like_cpp(&mut events);
            events
        } else {
            Vec::new()
        }
    }

    fn set_member_ready_checked_like_cpp(
        &mut self,
        slot_index: usize,
        events: &mut Vec<ReadyCheckEventLikeCpp>,
    ) {
        self.member_slots[slot_index].ready_checked = true;
        if self.is_ready_check_completed_like_cpp() {
            self.end_ready_check_like_cpp(events);
        }
    }

    fn set_member_ready_check_slot_like_cpp(
        &mut self,
        slot_index: usize,
        ready: bool,
        events: &mut Vec<ReadyCheckEventLikeCpp>,
    ) {
        let player = self.member_slots[slot_index].guid;
        events.push(ReadyCheckEventLikeCpp::Response {
            party_guid: self.group_guid,
            player,
            is_ready: ready,
        });
        self.set_member_ready_checked_like_cpp(slot_index, events);
    }

    pub fn start_ready_check_like_cpp(
        &mut self,
        starter_guid: ObjectGuid,
        connected_members: impl IntoIterator<Item = ObjectGuid>,
    ) -> Vec<ReadyCheckEventLikeCpp> {
        let mut events = Vec::new();
        if self.ready_check_started {
            return events;
        }

        let Some(starter_index) = self
            .member_slots
            .iter()
            .position(|slot| slot.guid == starter_guid)
        else {
            return events;
        };

        self.ready_check_started = true;
        self.ready_check_timer_ms = READYCHECK_DURATION_MS_LIKE_CPP;

        let connected: Vec<ObjectGuid> = connected_members.into_iter().collect();
        let offline_indices: Vec<usize> = self
            .member_slots
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| (!connected.contains(&slot.guid)).then_some(index))
            .collect();
        for index in offline_indices {
            if self.ready_check_started {
                self.set_member_ready_check_slot_like_cpp(index, false, &mut events);
            }
        }

        if self.ready_check_started {
            self.set_member_ready_checked_like_cpp(starter_index, &mut events);
        }

        events.push(ReadyCheckEventLikeCpp::Started {
            party_index: 0,
            party_guid: self.group_guid,
            initiator_guid: starter_guid,
            duration_ms: READYCHECK_DURATION_MS_LIKE_CPP,
        });
        events
    }

    pub fn set_member_ready_check_like_cpp(
        &mut self,
        guid: ObjectGuid,
        ready: bool,
    ) -> Vec<ReadyCheckEventLikeCpp> {
        let mut events = Vec::new();
        if !self.ready_check_started {
            return events;
        }

        if let Some(slot_index) = self.member_slots.iter().position(|slot| slot.guid == guid) {
            self.set_member_ready_check_slot_like_cpp(slot_index, ready, &mut events);
        }

        events
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

    pub fn set_dungeon_difficulty_id_like_cpp(&mut self, difficulty_id: u32) -> bool {
        if self.dungeon_difficulty_id == difficulty_id {
            return false;
        }
        self.dungeon_difficulty_id = difficulty_id;
        true
    }

    pub fn set_raid_difficulty_id_like_cpp(&mut self, difficulty_id: u32) -> bool {
        if self.raid_difficulty_id == difficulty_id {
            return false;
        }
        self.raid_difficulty_id = difficulty_id;
        true
    }

    pub fn set_legacy_raid_difficulty_id_like_cpp(&mut self, difficulty_id: u32) -> bool {
        if self.legacy_raid_difficulty_id == difficulty_id {
            return false;
        }
        self.legacy_raid_difficulty_id = difficulty_id;
        true
    }

    pub fn convert_to_raid_like_cpp(&mut self) {
        if !self.is_raid_group() {
            self.group_flags |= GROUP_FLAG_RAID_LIKE_CPP;
            self.init_raid_subgroups_counter_like_cpp();
            self.sequence_num += 1;
        }
    }

    pub fn convert_to_group_like_cpp(&mut self) -> bool {
        if self.members.len() > 5 {
            return false;
        }
        if self.is_raid_group() {
            self.group_flags &= !GROUP_FLAG_RAID_LIKE_CPP;
            self.raid_subgroup_counts = None;
            self.sequence_num += 1;
        }
        true
    }

    pub fn remove_member(&mut self, guid: &ObjectGuid) {
        if let Some(slot) = self.member_slots.iter().find(|slot| &slot.guid == guid) {
            self.subgroup_counter_decrease_like_cpp(slot.subgroup);
        }
        self.members.retain(|g| g != guid);
        self.member_slots.retain(|slot| &slot.guid != guid);
        self.sequence_num += 1;
    }

    pub fn is_empty(&self) -> bool {
        self.members.len() < 2
    }
}

/// Thread-safe owner of all active groups, keyed by group GUID.
///
/// Read access returns owned snapshots so callers cannot retain a backing-map
/// guard. The narrowly retained mutable compatibility methods are removed by
/// the transition issues named on each method (#197/#198/#199).
#[derive(Debug, Default)]
pub struct GroupRegistry {
    groups: DashMap<u64, GroupInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupAuthorityErrorLikeCpp {
    MissingGroup,
    MissingMember,
    NotLeader,
    NotLeaderOrAssistant,
    InvalidSubgroup,
    SubgroupFull,
    NotRaid,
    GroupTooLarge,
    NoChange,
    LfgGroup,
    LfgBootLimit,
    LfgBootTooFewPlayers,
    LfgBootDungeonComplete,
    LfgBootLootRolls,
    LfgBootInCombat,
    LfgKickOwnedByVote,
    InviteRestricted,
    TargetIsLeader,
}

#[derive(Debug, Clone)]
pub struct GroupTransitionOutcomeLikeCpp<T> {
    pub group: GroupInfo,
    pub facts: T,
}

#[derive(Debug, Clone)]
pub struct GroupMemberRemovalFactsLikeCpp {
    pub removed_guid: ObjectGuid,
    pub db_store_id: u32,
    pub disbanded: bool,
    pub new_leader_guid: Option<ObjectGuid>,
    pub remaining_members: Vec<ObjectGuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupMemberRemovalKindLikeCpp {
    Leave,
    Kick {
        actor_guid: ObjectGuid,
        actor_in_battleground: bool,
        target_has_loot_rolls: bool,
        any_member_in_actor_map_combat: bool,
    },
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

    /// Transitional database-load/test compatibility; removed by #199.
    pub fn insert(&self, group_guid: u64, group: GroupInfo) -> Option<GroupInfo> {
        self.groups.insert(group_guid, group)
    }

    /// Transitional test-fixture compatibility; removed by #199.
    pub fn remove(&self, group_guid: &u64) -> Option<(u64, GroupInfo)> {
        self.groups.remove(group_guid)
    }

    /// Transitional database-load/test-fixture compatibility; removed by #199.
    pub fn get_mut(
        &self,
        group_guid: &u64,
    ) -> Option<dashmap::mapref::one::RefMut<'_, u64, GroupInfo>> {
        self.groups.get_mut(group_guid)
    }

    pub fn remove_member_like_cpp(
        &self,
        group_guid: u64,
        member_guid: ObjectGuid,
        kind: GroupMemberRemovalKindLikeCpp,
        connected_members_in_order: &[ObjectGuid],
    ) -> Result<
        GroupTransitionOutcomeLikeCpp<GroupMemberRemovalFactsLikeCpp>,
        GroupAuthorityErrorLikeCpp,
    > {
        let Entry::Occupied(mut entry) = self.groups.entry(group_guid) else {
            return Err(GroupAuthorityErrorLikeCpp::MissingGroup);
        };
        let group = entry.get_mut();

        if let GroupMemberRemovalKindLikeCpp::Kick {
            actor_guid,
            actor_in_battleground,
            target_has_loot_rolls,
            any_member_in_actor_map_combat,
        } = kind
        {
            if group.is_lfg_group_like_cpp() {
                if group.lfg_kicks_left_like_cpp == 0 {
                    return Err(GroupAuthorityErrorLikeCpp::LfgBootLimit);
                }
                if group.members.len() <= LFG_GROUP_KICK_VOTES_NEEDED_LIKE_CPP {
                    return Err(GroupAuthorityErrorLikeCpp::LfgBootTooFewPlayers);
                }
                if group.lfg_db_state.as_ref().and_then(|state| state.state)
                    == Some(LFG_STATE_FINISHED_DUNGEON_LIKE_CPP)
                {
                    return Err(GroupAuthorityErrorLikeCpp::LfgBootDungeonComplete);
                }
                if target_has_loot_rolls {
                    return Err(GroupAuthorityErrorLikeCpp::LfgBootLootRolls);
                }
                if any_member_in_actor_map_combat {
                    return Err(GroupAuthorityErrorLikeCpp::LfgBootInCombat);
                }
            } else {
                if !group.is_leader_like_cpp(actor_guid) && !group.is_assistant_like_cpp(actor_guid)
                {
                    return Err(GroupAuthorityErrorLikeCpp::NotLeaderOrAssistant);
                }
                if actor_in_battleground {
                    return Err(GroupAuthorityErrorLikeCpp::InviteRestricted);
                }
                if group.leader_guid == member_guid {
                    return Err(GroupAuthorityErrorLikeCpp::TargetIsLeader);
                }
            }
        }

        if !group.members.contains(&member_guid) {
            return Err(GroupAuthorityErrorLikeCpp::MissingMember);
        }
        if matches!(kind, GroupMemberRemovalKindLikeCpp::Kick { .. })
            && group.is_lfg_group_like_cpp()
        {
            return Err(GroupAuthorityErrorLikeCpp::LfgKickOwnedByVote);
        }

        let previous_leader = group.leader_guid;
        group.remove_member(&member_guid);
        let db_store_id = group.db_store_id;
        let mut new_leader_guid = None;
        if group.members.len() >= 2 && previous_leader == member_guid {
            if let Some(successor) = connected_members_in_order
                .iter()
                .copied()
                .find(|candidate| group.members.contains(candidate))
            {
                if group.change_leader_like_cpp(successor).is_some() {
                    new_leader_guid = Some(successor);
                }
            }
        }

        let disbanded = group.members.len() < 2;
        if disbanded {
            let group = entry.remove();
            let remaining_members = group.members.clone();
            return Ok(GroupTransitionOutcomeLikeCpp {
                group,
                facts: GroupMemberRemovalFactsLikeCpp {
                    removed_guid: member_guid,
                    db_store_id,
                    disbanded,
                    new_leader_guid,
                    remaining_members,
                },
            });
        }

        let group = group.clone();
        let remaining_members = group.members.clone();
        Ok(GroupTransitionOutcomeLikeCpp {
            group,
            facts: GroupMemberRemovalFactsLikeCpp {
                removed_guid: member_guid,
                db_store_id,
                disbanded,
                new_leader_guid,
                remaining_members,
            },
        })
    }

    pub fn convert_group_like_cpp(
        &self,
        group_guid: u64,
        actor_guid: ObjectGuid,
        raid: bool,
    ) -> Result<GroupTransitionOutcomeLikeCpp<(u16, u32)>, GroupAuthorityErrorLikeCpp> {
        let mut group = self
            .groups
            .get_mut(&group_guid)
            .ok_or(GroupAuthorityErrorLikeCpp::MissingGroup)?;
        if !group.is_leader_like_cpp(actor_guid) {
            return Err(GroupAuthorityErrorLikeCpp::NotLeader);
        }
        if group.members.len() < 2 {
            return Err(GroupAuthorityErrorLikeCpp::MissingMember);
        }
        if raid {
            group.convert_to_raid_like_cpp();
        } else if !group.convert_to_group_like_cpp() {
            return Err(GroupAuthorityErrorLikeCpp::GroupTooLarge);
        }
        let facts = (group.group_flags, group.db_store_id);
        Ok(GroupTransitionOutcomeLikeCpp {
            group: group.clone(),
            facts,
        })
    }

    pub fn change_member_subgroup_like_cpp(
        &self,
        group_guid: u64,
        actor_guid: ObjectGuid,
        member_guid: ObjectGuid,
        subgroup: u8,
    ) -> Result<GroupTransitionOutcomeLikeCpp<(ObjectGuid, u8)>, GroupAuthorityErrorLikeCpp> {
        let mut group = self
            .groups
            .get_mut(&group_guid)
            .ok_or(GroupAuthorityErrorLikeCpp::MissingGroup)?;
        if !group.is_leader_like_cpp(actor_guid) && !group.is_assistant_like_cpp(actor_guid) {
            return Err(GroupAuthorityErrorLikeCpp::NotLeaderOrAssistant);
        }
        if !group.is_raid_group() {
            return Err(GroupAuthorityErrorLikeCpp::NotRaid);
        }
        if usize::from(subgroup) >= MAX_RAID_SUBGROUPS_LIKE_CPP {
            return Err(GroupAuthorityErrorLikeCpp::InvalidSubgroup);
        }
        if !group.has_free_slot_sub_group_like_cpp(subgroup) {
            return Err(GroupAuthorityErrorLikeCpp::SubgroupFull);
        }
        if !group.members.contains(&member_guid) {
            return Err(GroupAuthorityErrorLikeCpp::MissingMember);
        }
        if !group.change_member_group_like_cpp(member_guid, subgroup) {
            return Err(GroupAuthorityErrorLikeCpp::NoChange);
        }
        Ok(GroupTransitionOutcomeLikeCpp {
            group: group.clone(),
            facts: (member_guid, subgroup),
        })
    }

    pub fn swap_member_subgroups_like_cpp(
        &self,
        group_guid: u64,
        actor_guid: ObjectGuid,
        first: ObjectGuid,
        second: ObjectGuid,
    ) -> Result<GroupTransitionOutcomeLikeCpp<Vec<(ObjectGuid, u8)>>, GroupAuthorityErrorLikeCpp>
    {
        let mut group = self
            .groups
            .get_mut(&group_guid)
            .ok_or(GroupAuthorityErrorLikeCpp::MissingGroup)?;
        if !group.is_leader_like_cpp(actor_guid) && !group.is_assistant_like_cpp(actor_guid) {
            return Err(GroupAuthorityErrorLikeCpp::NotLeaderOrAssistant);
        }
        let facts = group
            .swap_members_groups_like_cpp(first, second)
            .ok_or(GroupAuthorityErrorLikeCpp::NoChange)?;
        Ok(GroupTransitionOutcomeLikeCpp {
            group: group.clone(),
            facts: facts.to_vec(),
        })
    }

    pub fn change_leader_transition_like_cpp(
        &self,
        group_guid: u64,
        actor_guid: ObjectGuid,
        new_leader_guid: ObjectGuid,
    ) -> Result<GroupTransitionOutcomeLikeCpp<(u32, u8)>, GroupAuthorityErrorLikeCpp> {
        let mut group = self
            .groups
            .get_mut(&group_guid)
            .ok_or(GroupAuthorityErrorLikeCpp::MissingGroup)?;
        if !group.is_leader_like_cpp(actor_guid) {
            return Err(GroupAuthorityErrorLikeCpp::NotLeader);
        }
        if !group.members.contains(&new_leader_guid) {
            return Err(GroupAuthorityErrorLikeCpp::MissingMember);
        }
        let db_store_id = group.db_store_id;
        let final_flags = group
            .change_leader_like_cpp(new_leader_guid)
            .ok_or(GroupAuthorityErrorLikeCpp::NoChange)?;
        Ok(GroupTransitionOutcomeLikeCpp {
            group: group.clone(),
            facts: (db_store_id, final_flags),
        })
    }

    pub fn set_member_flag_transition_like_cpp(
        &self,
        group_guid: u64,
        actor_guid: ObjectGuid,
        member_guid: ObjectGuid,
        apply: bool,
        flag: u8,
    ) -> Result<GroupTransitionOutcomeLikeCpp<u8>, GroupAuthorityErrorLikeCpp> {
        let mut group = self
            .groups
            .get_mut(&group_guid)
            .ok_or(GroupAuthorityErrorLikeCpp::MissingGroup)?;
        if !group.is_leader_like_cpp(actor_guid) {
            return Err(GroupAuthorityErrorLikeCpp::NotLeader);
        }
        let facts = group
            .set_group_member_flag_like_cpp(member_guid, apply, flag)
            .ok_or(GroupAuthorityErrorLikeCpp::MissingMember)?;
        Ok(GroupTransitionOutcomeLikeCpp {
            group: group.clone(),
            facts,
        })
    }

    pub fn set_everyone_assistant_transition_like_cpp(
        &self,
        group_guid: u64,
        actor_guid: ObjectGuid,
        apply: bool,
    ) -> Result<GroupTransitionOutcomeLikeCpp<(u16, u32)>, GroupAuthorityErrorLikeCpp> {
        let mut group = self
            .groups
            .get_mut(&group_guid)
            .ok_or(GroupAuthorityErrorLikeCpp::MissingGroup)?;
        if !group.is_leader_like_cpp(actor_guid) {
            return Err(GroupAuthorityErrorLikeCpp::NotLeader);
        }
        let facts = group.set_everyone_is_assistant_like_cpp(apply);
        Ok(GroupTransitionOutcomeLikeCpp {
            group: group.clone(),
            facts,
        })
    }

    pub fn set_party_assignment_transition_like_cpp(
        &self,
        group_guid: u64,
        actor_guid: ObjectGuid,
        member_guid: ObjectGuid,
        assignment: u8,
        apply: bool,
    ) -> Result<GroupTransitionOutcomeLikeCpp<Vec<(ObjectGuid, u8)>>, GroupAuthorityErrorLikeCpp>
    {
        let mut group = self
            .groups
            .get_mut(&group_guid)
            .ok_or(GroupAuthorityErrorLikeCpp::MissingGroup)?;
        if !group.is_leader_like_cpp(actor_guid) && !group.is_assistant_like_cpp(actor_guid) {
            return Err(GroupAuthorityErrorLikeCpp::NotLeaderOrAssistant);
        }
        let flag = match assignment {
            GROUP_ASSIGN_MAINASSIST_LIKE_CPP => MEMBER_FLAG_MAINASSIST_LIKE_CPP,
            GROUP_ASSIGN_MAINTANK_LIKE_CPP => MEMBER_FLAG_MAINTANK_LIKE_CPP,
            _ => {
                return Ok(GroupTransitionOutcomeLikeCpp {
                    group: group.clone(),
                    facts: Vec::new(),
                });
            }
        };
        group.remove_unique_group_member_flag_like_cpp(flag);
        let facts = group
            .set_group_member_flag_updates_like_cpp(member_guid, apply, flag)
            .unwrap_or_default();
        Ok(GroupTransitionOutcomeLikeCpp {
            group: group.clone(),
            facts,
        })
    }

    pub fn start_ready_check_transition_like_cpp(
        &self,
        group_guid: u64,
        actor_guid: ObjectGuid,
        connected_members: impl IntoIterator<Item = ObjectGuid>,
    ) -> Result<
        GroupTransitionOutcomeLikeCpp<Vec<ReadyCheckEventLikeCpp>>,
        GroupAuthorityErrorLikeCpp,
    > {
        let mut group = self
            .groups
            .get_mut(&group_guid)
            .ok_or(GroupAuthorityErrorLikeCpp::MissingGroup)?;
        if !group.is_leader_like_cpp(actor_guid) && !group.is_assistant_like_cpp(actor_guid) {
            return Err(GroupAuthorityErrorLikeCpp::NotLeaderOrAssistant);
        }
        let facts = group.start_ready_check_like_cpp(actor_guid, connected_members);
        if facts.is_empty() {
            return Err(GroupAuthorityErrorLikeCpp::NoChange);
        }
        Ok(GroupTransitionOutcomeLikeCpp {
            group: group.clone(),
            facts,
        })
    }

    pub fn respond_ready_check_transition_like_cpp(
        &self,
        group_guid: u64,
        member_guid: ObjectGuid,
        ready: bool,
    ) -> Result<
        GroupTransitionOutcomeLikeCpp<Vec<ReadyCheckEventLikeCpp>>,
        GroupAuthorityErrorLikeCpp,
    > {
        let mut group = self
            .groups
            .get_mut(&group_guid)
            .ok_or(GroupAuthorityErrorLikeCpp::MissingGroup)?;
        if !group.members.contains(&member_guid) {
            return Err(GroupAuthorityErrorLikeCpp::MissingMember);
        }
        let facts = group.set_member_ready_check_like_cpp(member_guid, ready);
        if facts.is_empty() {
            return Err(GroupAuthorityErrorLikeCpp::NoChange);
        }
        Ok(GroupTransitionOutcomeLikeCpp {
            group: group.clone(),
            facts,
        })
    }

    pub fn set_lfg_role_transition_like_cpp(
        &self,
        group_guid: u64,
        member_guid: ObjectGuid,
        role: u8,
    ) -> Result<GroupTransitionOutcomeLikeCpp<(u8, bool)>, GroupAuthorityErrorLikeCpp> {
        let mut group = self
            .groups
            .get_mut(&group_guid)
            .ok_or(GroupAuthorityErrorLikeCpp::MissingGroup)?;
        let old_role = group.get_lfg_roles_like_cpp(member_guid);
        if old_role == role {
            return Err(GroupAuthorityErrorLikeCpp::NoChange);
        }
        let mutated = group.set_lfg_roles_like_cpp(member_guid, role);
        Ok(GroupTransitionOutcomeLikeCpp {
            group: group.clone(),
            facts: (old_role, mutated),
        })
    }

    pub fn set_target_icon_transition_like_cpp(
        &self,
        group_guid: u64,
        actor_guid: ObjectGuid,
        symbol: u8,
        target: ObjectGuid,
    ) -> Result<GroupTransitionOutcomeLikeCpp<Vec<(u8, ObjectGuid)>>, GroupAuthorityErrorLikeCpp>
    {
        let mut group = self
            .groups
            .get_mut(&group_guid)
            .ok_or(GroupAuthorityErrorLikeCpp::MissingGroup)?;
        if group.is_raid_group()
            && !group.is_leader_like_cpp(actor_guid)
            && !group.is_assistant_like_cpp(actor_guid)
        {
            return Err(GroupAuthorityErrorLikeCpp::NotLeaderOrAssistant);
        }
        let facts = group
            .set_target_icon_like_cpp(symbol, target)
            .ok_or(GroupAuthorityErrorLikeCpp::InvalidSubgroup)?;
        Ok(GroupTransitionOutcomeLikeCpp {
            group: group.clone(),
            facts,
        })
    }

    pub fn delete_raid_marker_transition_like_cpp(
        &self,
        group_guid: u64,
        actor_guid: ObjectGuid,
        marker_id: u8,
    ) -> Result<GroupTransitionOutcomeLikeCpp<bool>, GroupAuthorityErrorLikeCpp> {
        let mut group = self
            .groups
            .get_mut(&group_guid)
            .ok_or(GroupAuthorityErrorLikeCpp::MissingGroup)?;
        if group.is_raid_group()
            && !group.is_leader_like_cpp(actor_guid)
            && !group.is_assistant_like_cpp(actor_guid)
        {
            return Err(GroupAuthorityErrorLikeCpp::NotLeaderOrAssistant);
        }
        if usize::from(marker_id) > RAID_MARKERS_COUNT_LIKE_CPP {
            return Err(GroupAuthorityErrorLikeCpp::InvalidSubgroup);
        }
        let facts = group.delete_raid_marker_like_cpp(marker_id);
        Ok(GroupTransitionOutcomeLikeCpp {
            group: group.clone(),
            facts,
        })
    }

    pub fn add_raid_marker_transition_like_cpp(
        &self,
        group_guid: u64,
        actor_guid: ObjectGuid,
        marker_id: u8,
        map_id: u32,
        position: Position,
        transport_guid: ObjectGuid,
    ) -> Result<GroupTransitionOutcomeLikeCpp<()>, GroupAuthorityErrorLikeCpp> {
        let mut group = self
            .groups
            .get_mut(&group_guid)
            .ok_or(GroupAuthorityErrorLikeCpp::MissingGroup)?;
        if !group.members.contains(&actor_guid) {
            return Err(GroupAuthorityErrorLikeCpp::MissingMember);
        }
        if group.is_raid_group()
            && !group.is_leader_like_cpp(actor_guid)
            && !group.is_assistant_like_cpp(actor_guid)
        {
            return Err(GroupAuthorityErrorLikeCpp::NotLeaderOrAssistant);
        }
        if !group.add_raid_marker_like_cpp(marker_id, map_id, position, transport_guid) {
            return Err(GroupAuthorityErrorLikeCpp::NoChange);
        }
        Ok(GroupTransitionOutcomeLikeCpp {
            group: group.clone(),
            facts: (),
        })
    }

    pub fn set_recent_instance_transition_like_cpp(
        &self,
        group_guid: u64,
        map_id: u32,
        owner_guid: ObjectGuid,
        instance_id: u32,
    ) -> Result<GroupTransitionOutcomeLikeCpp<()>, GroupAuthorityErrorLikeCpp> {
        let mut group = self
            .groups
            .get_mut(&group_guid)
            .ok_or(GroupAuthorityErrorLikeCpp::MissingGroup)?;
        group.set_recent_instance_like_cpp(map_id, owner_guid, instance_id);
        Ok(GroupTransitionOutcomeLikeCpp {
            group: group.clone(),
            facts: (),
        })
    }

    pub fn link_owned_instance_transition_like_cpp(
        &self,
        group_guid: u64,
        map_id: u32,
        instance_id: u32,
    ) -> Result<GroupTransitionOutcomeLikeCpp<bool>, GroupAuthorityErrorLikeCpp> {
        let mut group = self
            .groups
            .get_mut(&group_guid)
            .ok_or(GroupAuthorityErrorLikeCpp::MissingGroup)?;
        let facts = group.link_owned_instance_like_cpp(map_id, instance_id);
        Ok(GroupTransitionOutcomeLikeCpp {
            group: group.clone(),
            facts,
        })
    }

    pub fn unlink_owned_instance_transition_like_cpp(
        &self,
        group_guid: u64,
        map_id: u32,
        instance_id: u32,
    ) -> Result<GroupTransitionOutcomeLikeCpp<bool>, GroupAuthorityErrorLikeCpp> {
        let mut group = self
            .groups
            .get_mut(&group_guid)
            .ok_or(GroupAuthorityErrorLikeCpp::MissingGroup)?;
        let facts = group.unlink_owned_instance_like_cpp(map_id, instance_id);
        Ok(GroupTransitionOutcomeLikeCpp {
            group: group.clone(),
            facts,
        })
    }

    pub fn apply_instance_reset_transition_like_cpp(
        &self,
        group_guid: u64,
        map_id: u32,
        result: GroupInstanceResetResultLikeCpp,
        method: GroupInstanceResetMethodLikeCpp,
    ) -> Result<GroupTransitionOutcomeLikeCpp<bool>, GroupAuthorityErrorLikeCpp> {
        let mut group = self
            .groups
            .get_mut(&group_guid)
            .ok_or(GroupAuthorityErrorLikeCpp::MissingGroup)?;
        let facts = group.apply_owned_instance_reset_result_like_cpp(map_id, result, method);
        Ok(GroupTransitionOutcomeLikeCpp {
            group: group.clone(),
            facts,
        })
    }

    pub fn set_difficulty_transition_like_cpp(
        &self,
        group_guid: u64,
        actor_guid: ObjectGuid,
        difficulty_id: u32,
        kind: crate::player_registry::GroupDifficultyKindLikeCpp,
    ) -> Result<GroupTransitionOutcomeLikeCpp<u32>, GroupAuthorityErrorLikeCpp> {
        let mut group = self
            .groups
            .get_mut(&group_guid)
            .ok_or(GroupAuthorityErrorLikeCpp::MissingGroup)?;
        if !group.is_leader_like_cpp(actor_guid) {
            return Err(GroupAuthorityErrorLikeCpp::NotLeader);
        }
        if group.is_lfg_group_like_cpp() {
            return Err(GroupAuthorityErrorLikeCpp::LfgGroup);
        }
        let changed = match kind {
            crate::player_registry::GroupDifficultyKindLikeCpp::Dungeon => {
                group.set_dungeon_difficulty_id_like_cpp(difficulty_id)
            }
            crate::player_registry::GroupDifficultyKindLikeCpp::Raid => {
                group.set_raid_difficulty_id_like_cpp(difficulty_id)
            }
            crate::player_registry::GroupDifficultyKindLikeCpp::LegacyRaid => {
                group.set_legacy_raid_difficulty_id_like_cpp(difficulty_id)
            }
        };
        if !changed {
            return Err(GroupAuthorityErrorLikeCpp::NoChange);
        }
        Ok(GroupTransitionOutcomeLikeCpp {
            facts: group.db_store_id,
            group: group.clone(),
        })
    }

    pub fn advance_looter_transition_like_cpp(
        &self,
        group_guid: u64,
        eligible_members: impl IntoIterator<Item = ObjectGuid>,
    ) -> Result<GroupTransitionOutcomeLikeCpp<bool>, GroupAuthorityErrorLikeCpp> {
        let mut group = self
            .groups
            .get_mut(&group_guid)
            .ok_or(GroupAuthorityErrorLikeCpp::MissingGroup)?;
        let facts = group.update_looter_guid_like_cpp(eligible_members, false);
        Ok(GroupTransitionOutcomeLikeCpp {
            group: group.clone(),
            facts,
        })
    }

    pub fn tick_ready_checks_like_cpp(
        &self,
        diff_ms: u32,
    ) -> Vec<(u64, Vec<ReadyCheckEventLikeCpp>)> {
        let active_keys: Vec<u64> = self
            .groups
            .iter()
            .filter(|entry| entry.value().ready_check_started)
            .map(|entry| *entry.key())
            .collect();
        let mut results = Vec::new();
        for group_guid in active_keys {
            if let Some(mut group) = self.groups.get_mut(&group_guid) {
                let events = group.update_ready_check_like_cpp(diff_ms);
                if !events.is_empty() {
                    results.push((group_guid, events));
                }
            }
        }
        results
    }
}

/// C++ `Group::UpdateReadyCheck` fanout: ticks every active group's
/// ready-check timer and collects expired `Completed` events without
/// holding any lock during packet fanout.
///
/// Returns `(group_guid, events)` for groups whose ready check expired this
/// tick. Caller is responsible for broadcasting the events to connected
/// players via `PlayerRegistry`.
pub fn tick_all_group_ready_checks_like_cpp(
    registry: &GroupRegistry,
    diff_ms: u32,
) -> Vec<(u64, Vec<ReadyCheckEventLikeCpp>)> {
    registry.tick_ready_checks_like_cpp(diff_ms)
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
    difficulty_store: &DifficultyStore,
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
        registry.insert(runtime_group_guid, group);
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
        let Some(mut group) = registry.get_mut(&runtime_group_guid) else {
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

/// C++ `Player::m_groupInvite` represented as one pending group pointer per
/// invited player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingInviteLikeCpp {
    pub leader_guid: ObjectGuid,
    pub group_guid: Option<u64>,
    pub group_category: u8,
}

impl PendingInviteLikeCpp {
    pub fn new_pending_group(leader_guid: ObjectGuid, group_category: u8) -> Self {
        Self {
            leader_guid,
            group_guid: None,
            group_category,
        }
    }

    pub fn new_existing_group(
        leader_guid: ObjectGuid,
        group_guid: u64,
        group_category: u8,
    ) -> Self {
        Self {
            leader_guid,
            group_guid: Some(group_guid),
            group_category,
        }
    }
}

/// Owner of pending invites: invited_guid → represented C++ group invite.
#[derive(Debug, Default)]
pub struct PendingInvites {
    invites: DashMap<ObjectGuid, PendingInviteLikeCpp>,
    transition_lock: Mutex<()>,
}

impl PendingInvites {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock_transition(&self) -> MutexGuard<'_, ()> {
        self.transition_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Return an immutable owned invite snapshot.
    pub fn get(&self, invited_guid: &ObjectGuid) -> Option<PendingInviteLikeCpp> {
        let _transition = self.lock_transition();
        self.invites.get(invited_guid).map(|invite| *invite)
    }

    pub fn contains_key(&self, invited_guid: &ObjectGuid) -> bool {
        let _transition = self.lock_transition();
        self.invites.contains_key(invited_guid)
    }

    pub fn matching_guids(&self, invite: PendingInviteLikeCpp) -> Vec<ObjectGuid> {
        let _transition = self.lock_transition();
        self.matching_guids_unlocked(invite)
    }

    fn matching_guids_unlocked(&self, invite: PendingInviteLikeCpp) -> Vec<ObjectGuid> {
        self.invites
            .iter()
            .filter(|entry| *entry.value() == invite)
            .map(|entry| *entry.key())
            .collect()
    }

    /// Transitional test-fixture compatibility; removed by #199.
    pub fn insert(
        &self,
        invited_guid: ObjectGuid,
        invite: PendingInviteLikeCpp,
    ) -> Option<PendingInviteLikeCpp> {
        let _transition = self.lock_transition();
        self.invites.insert(invited_guid, invite)
    }

    /// Transitional test-fixture compatibility; removed by #199.
    pub fn remove(&self, invited_guid: &ObjectGuid) -> Option<(ObjectGuid, PendingInviteLikeCpp)> {
        let _transition = self.lock_transition();
        self.invites.remove(invited_guid)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateGroupInviteResultLikeCpp {
    Created(PendingInviteLikeCpp),
    TargetAlreadyInvited,
    TargetAlreadyGrouped,
    InviterNotLeaderOrAssistant,
    GroupFull,
    MissingInviterGroup,
    WrongCategory,
}

#[derive(Debug, Clone)]
pub enum AcceptGroupInviteResultLikeCpp {
    NoInvite,
    WrongCategory,
    SelfInvite,
    GroupFull,
    AddFailed,
    AlreadyMember,
    MissingGroup,
    MissingLeader,
    JoinedExisting { group: GroupInfo, subgroup: u8 },
    Created { group: GroupInfo, subgroup: u8 },
}

impl GroupRegistry {
    /// Atomically validate and record the represented C++ group invite.
    pub fn create_invite_like_cpp(
        &self,
        pending: &PendingInvites,
        inviter_guid: ObjectGuid,
        invitee_guid: ObjectGuid,
        inviter_group_guid: Option<u64>,
        lookup_group_category: u8,
        new_group_category: u8,
    ) -> CreateGroupInviteResultLikeCpp {
        let _transition = pending.lock_transition();

        if pending.invites.contains_key(&invitee_guid) {
            return CreateGroupInviteResultLikeCpp::TargetAlreadyInvited;
        }
        if self.groups.iter().any(|group| {
            group.group_category_like_cpp() == lookup_group_category
                && group.members.contains(&invitee_guid)
        }) {
            return CreateGroupInviteResultLikeCpp::TargetAlreadyGrouped;
        }

        let resolved_inviter_group_guid = inviter_group_guid
            .filter(|group_guid| {
                self.groups.get(group_guid).is_some_and(|group| {
                    group.group_category_like_cpp() == lookup_group_category
                        && group.members.contains(&inviter_guid)
                })
            })
            .or_else(|| {
                self.groups
                    .iter()
                    .find(|group| {
                        group.group_category_like_cpp() == lookup_group_category
                            && group.members.contains(&inviter_guid)
                    })
                    .map(|group| *group.key())
            });
        if inviter_group_guid.is_some() && resolved_inviter_group_guid.is_none() {
            return CreateGroupInviteResultLikeCpp::MissingInviterGroup;
        }

        let invite = if let Some(group_guid) = resolved_inviter_group_guid {
            let Some(group) = self.groups.get(&group_guid) else {
                return CreateGroupInviteResultLikeCpp::MissingInviterGroup;
            };
            if group.group_category_like_cpp() != lookup_group_category {
                return CreateGroupInviteResultLikeCpp::WrongCategory;
            }
            if !group.is_leader_like_cpp(inviter_guid) && !group.is_assistant_like_cpp(inviter_guid)
            {
                return CreateGroupInviteResultLikeCpp::InviterNotLeaderOrAssistant;
            }
            if group.is_full_like_cpp() {
                return CreateGroupInviteResultLikeCpp::GroupFull;
            }
            PendingInviteLikeCpp::new_existing_group(
                group.leader_guid,
                group_guid,
                group.group_category_like_cpp(),
            )
        } else if let Some(invite) = pending.invites.get(&inviter_guid).map(|invite| *invite) {
            invite
        } else {
            let invite = PendingInviteLikeCpp::new_pending_group(inviter_guid, new_group_category);
            pending.invites.insert(inviter_guid, invite);
            invite
        };

        pending.invites.insert(invitee_guid, invite);
        CreateGroupInviteResultLikeCpp::Created(invite)
    }

    fn cancel_invite_unlocked_like_cpp(
        pending: &PendingInvites,
        invitee_guid: ObjectGuid,
        expected: PendingInviteLikeCpp,
    ) -> bool {
        if pending.invites.get(&invitee_guid).map(|invite| *invite) != Some(expected) {
            return false;
        }
        pending.invites.remove(&invitee_guid);
        if expected.group_guid.is_none() && pending.matching_guids_unlocked(expected).len() <= 1 {
            for guid in pending.matching_guids_unlocked(expected) {
                pending.invites.remove(&guid);
            }
        }
        true
    }

    /// Cancel one exact invite without deleting a newer replacement.
    pub fn cancel_invite_like_cpp(
        &self,
        pending: &PendingInvites,
        invitee_guid: ObjectGuid,
        expected: PendingInviteLikeCpp,
    ) -> bool {
        let _transition = pending.lock_transition();
        Self::cancel_invite_unlocked_like_cpp(pending, invitee_guid, expected)
    }

    /// Replace one exact invite and clean up an abandoned pending group.
    pub fn replace_invite_like_cpp(
        &self,
        pending: &PendingInvites,
        invitee_guid: ObjectGuid,
        expected: PendingInviteLikeCpp,
        replacement: PendingInviteLikeCpp,
    ) -> bool {
        let _transition = pending.lock_transition();
        if !Self::cancel_invite_unlocked_like_cpp(pending, invitee_guid, expected) {
            return false;
        }
        if replacement.group_guid.is_none() {
            pending
                .invites
                .entry(replacement.leader_guid)
                .or_insert(replacement);
        }
        pending.invites.insert(invitee_guid, replacement);
        true
    }

    /// Expire one exact invite without touching a newer replacement.
    pub fn expire_invite_like_cpp(
        &self,
        pending: &PendingInvites,
        invitee_guid: ObjectGuid,
        expected: PendingInviteLikeCpp,
    ) -> bool {
        self.cancel_invite_like_cpp(pending, invitee_guid, expected)
    }

    /// Cancel every invite belonging to the exact pending group identity.
    pub fn cancel_pending_group_like_cpp(
        &self,
        pending: &PendingInvites,
        expected: PendingInviteLikeCpp,
    ) -> usize {
        let _transition = pending.lock_transition();
        let guids = pending.matching_guids_unlocked(expected);
        for guid in &guids {
            pending.invites.remove(guid);
        }
        guids.len()
    }

    /// Consume a decline only when its optional category still matches.
    pub fn decline_invite_like_cpp(
        &self,
        pending: &PendingInvites,
        invitee_guid: ObjectGuid,
        party_index: Option<u8>,
    ) -> Option<PendingInviteLikeCpp> {
        let invite = pending.get(&invitee_guid)?;
        if party_index.is_some_and(|index| invite.group_category != index) {
            return None;
        }
        self.cancel_invite_like_cpp(pending, invitee_guid, invite)
            .then_some(invite)
    }

    /// Atomically consume an invite and create or join its group.
    pub fn accept_invite_like_cpp(
        &self,
        pending: &PendingInvites,
        invitee_guid: ObjectGuid,
        party_index: Option<u8>,
        available_new_group_leader: Option<ObjectGuid>,
    ) -> AcceptGroupInviteResultLikeCpp {
        let _transition = pending.lock_transition();
        let Some(invite) = pending.invites.get(&invitee_guid).map(|invite| *invite) else {
            return AcceptGroupInviteResultLikeCpp::NoInvite;
        };
        if party_index.is_some_and(|index| invite.group_category != index) {
            return AcceptGroupInviteResultLikeCpp::WrongCategory;
        }

        if invite.leader_guid == invitee_guid {
            // C++ removes the invite before rejecting self-acceptance.
            pending.invites.remove(&invitee_guid);
            return AcceptGroupInviteResultLikeCpp::SelfInvite;
        }

        if let Some(group_guid) = invite.group_guid {
            let Some(mut group) = self.groups.get_mut(&group_guid) else {
                return AcceptGroupInviteResultLikeCpp::MissingGroup;
            };
            if group.group_category_like_cpp() != invite.group_category {
                return AcceptGroupInviteResultLikeCpp::WrongCategory;
            }
            // C++ consumes a valid invite before its full/AddMember checks.
            pending.invites.remove(&invitee_guid);
            if group.is_full_like_cpp() {
                return AcceptGroupInviteResultLikeCpp::GroupFull;
            }
            if group.members.contains(&invitee_guid) {
                return AcceptGroupInviteResultLikeCpp::AlreadyMember;
            }
            if !group.add_member(invitee_guid) {
                return AcceptGroupInviteResultLikeCpp::AddFailed;
            }
            let subgroup = group
                .member_slot_like_cpp(invitee_guid)
                .map(|slot| slot.subgroup)
                .unwrap_or_default();
            return AcceptGroupInviteResultLikeCpp::JoinedExisting {
                group: group.clone(),
                subgroup,
            };
        }

        pending.invites.remove(&invitee_guid);
        if available_new_group_leader != Some(invite.leader_guid) {
            for guid in pending.matching_guids_unlocked(invite) {
                pending.invites.remove(&guid);
            }
            return AcceptGroupInviteResultLikeCpp::MissingLeader;
        }

        let mut group = GroupInfo::new(invite.leader_guid);
        if !group.add_member(invitee_guid) {
            return AcceptGroupInviteResultLikeCpp::AddFailed;
        }
        let group_guid = group.group_guid;
        let db_store_id = group.db_store_id;
        let subgroup = group
            .member_slot_like_cpp(invitee_guid)
            .map(|slot| slot.subgroup)
            .unwrap_or_default();
        self.groups.insert(group_guid, group.clone());
        register_group_db_store_id_like_cpp(db_store_id, group_guid);
        pending.invites.remove(&invite.leader_guid);
        let promoted = PendingInviteLikeCpp::new_existing_group(
            invite.leader_guid,
            group_guid,
            invite.group_category,
        );
        for guid in pending.matching_guids_unlocked(invite) {
            pending.invites.insert(guid, promoted);
        }

        AcceptGroupInviteResultLikeCpp::Created { group, subgroup }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_registry_reads_are_owned_and_absent_groups_stay_absent() {
        let registry = GroupRegistry::new();
        let leader = ObjectGuid::create_player(1, 42);
        let group = GroupInfo::new(leader);
        let group_guid = group.group_guid;
        registry.insert(group_guid, group);

        let mut snapshot = registry.get(&group_guid).expect("group snapshot");
        snapshot.leader_guid = ObjectGuid::create_player(1, 99);

        assert_eq!(registry.get(&group_guid).unwrap().leader_guid, leader);
        assert!(registry.get(&u64::MAX).is_none());
        assert!(!registry.contains_key(&u64::MAX));
    }

    #[test]
    fn pending_invite_reads_are_owned_and_absent_invites_stay_absent() {
        let pending = PendingInvites::new();
        let leader = ObjectGuid::create_player(1, 42);
        let invited = ObjectGuid::create_player(1, 43);
        let invite = PendingInviteLikeCpp::new_pending_group(leader, 0);
        pending.insert(invited, invite);

        let snapshot = pending.get(&invited).expect("invite snapshot");

        assert_eq!(snapshot, invite);
        assert_eq!(pending.matching_guids(invite), vec![invited]);
        assert!(pending.get(&ObjectGuid::create_player(1, 99)).is_none());
        assert!(!pending.contains_key(&ObjectGuid::create_player(1, 99)));
    }

    #[test]
    fn new_group_uses_cpp_personal_loot_default() {
        let leader = ObjectGuid::create_player(1, 42);
        let group = GroupInfo::new(leader);

        assert_eq!(group.loot_method, LOOT_METHOD_PERSONAL_LIKE_CPP);
        assert_eq!(group.looter_guid, leader);
        assert_eq!(group.loot_threshold, ITEM_QUALITY_UNCOMMON_LIKE_CPP);
        assert_eq!(group.dungeon_difficulty_id, DIFFICULTY_NORMAL_LIKE_CPP);
        assert_eq!(group.raid_difficulty_id, DIFFICULTY_NORMAL_RAID_LIKE_CPP);
        assert_eq!(group.legacy_raid_difficulty_id, DIFFICULTY_10_N_LIKE_CPP);
    }

    #[test]
    fn new_group_separates_runtime_guid_from_cpp_db_store_id() {
        let leader = ObjectGuid::create_player(1, 42);
        let group = GroupInfo::new(leader);

        assert_ne!(group.db_store_id, 0);
        assert_ne!(group.group_guid, 0);
    }

    #[test]
    fn group_is_full_uses_cpp_party_and_raid_limits() {
        let leader = ObjectGuid::create_player(1, 42);
        let mut party = GroupInfo::new(leader);
        for counter in 43..47 {
            party.add_member(ObjectGuid::create_player(1, counter));
        }
        assert!(party.is_full_like_cpp());

        let mut raid = party.clone();
        raid.convert_to_raid_like_cpp();
        assert!(!raid.is_full_like_cpp());
        for counter in 47..82 {
            raid.members.push(ObjectGuid::create_player(1, counter));
        }
        assert!(raid.is_full_like_cpp());
    }

    #[test]
    fn concurrent_final_party_slot_accepts_exactly_one_invite_like_cpp() {
        let registry = std::sync::Arc::new(GroupRegistry::default());
        let pending = std::sync::Arc::new(PendingInvites::default());
        let leader = ObjectGuid::create_player(1, 42);
        let mut party = GroupInfo::new(leader);
        for counter in 43..46 {
            assert!(party.add_member(ObjectGuid::create_player(1, counter)));
        }
        let group_guid = party.group_guid;
        registry.insert(group_guid, party);

        let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
        let candidates = [
            ObjectGuid::create_player(1, 46),
            ObjectGuid::create_player(1, 47),
        ];
        for candidate in candidates {
            pending.insert(
                candidate,
                PendingInviteLikeCpp::new_existing_group(
                    leader,
                    group_guid,
                    GROUP_CATEGORY_HOME_LIKE_CPP,
                ),
            );
        }
        let handles: Vec<_> = candidates
            .into_iter()
            .map(|candidate| {
                let registry = std::sync::Arc::clone(&registry);
                let pending = std::sync::Arc::clone(&pending);
                let barrier = std::sync::Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    registry.accept_invite_like_cpp(&pending, candidate, None, Some(leader))
                })
            })
            .collect();

        barrier.wait();
        let results: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().expect("join attempt thread"))
            .collect();
        assert_eq!(
            results
                .iter()
                .filter(|result| {
                    matches!(
                        result,
                        AcceptGroupInviteResultLikeCpp::JoinedExisting { .. }
                    )
                })
                .count(),
            1
        );
        assert_eq!(
            results
                .iter()
                .filter(|result| matches!(result, AcceptGroupInviteResultLikeCpp::GroupFull))
                .count(),
            1
        );

        let party = registry.get(&group_guid).expect("party remains registered");
        assert_eq!(party.members.len(), MAX_GROUP_SIZE_LIKE_CPP);
        assert_ne!(
            party.members.contains(&candidates[0]),
            party.members.contains(&candidates[1]),
            "only one simultaneous invitee owns the final party slot"
        );
    }

    #[test]
    fn one_pending_invite_can_be_consumed_only_once() {
        let registry = std::sync::Arc::new(GroupRegistry::default());
        let pending = std::sync::Arc::new(PendingInvites::default());
        let leader = ObjectGuid::create_player(1, 42);
        let invitee = ObjectGuid::create_player(1, 77);
        let group = GroupInfo::new(leader);
        let group_guid = group.group_guid;
        registry.insert(group_guid, group);
        pending.insert(
            invitee,
            PendingInviteLikeCpp::new_existing_group(
                leader,
                group_guid,
                GROUP_CATEGORY_HOME_LIKE_CPP,
            ),
        );

        let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let registry = std::sync::Arc::clone(&registry);
                let pending = std::sync::Arc::clone(&pending);
                let barrier = std::sync::Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    registry.accept_invite_like_cpp(&pending, invitee, None, Some(leader))
                })
            })
            .collect();
        barrier.wait();
        let results: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().expect("accept thread"))
            .collect();

        assert_eq!(
            results
                .iter()
                .filter(|result| {
                    matches!(
                        result,
                        AcceptGroupInviteResultLikeCpp::JoinedExisting { .. }
                    )
                })
                .count(),
            1
        );
        assert_eq!(
            results
                .iter()
                .filter(|result| matches!(result, AcceptGroupInviteResultLikeCpp::NoInvite))
                .count(),
            1
        );
        let group = registry.get(&group_guid).expect("group remains registered");
        assert_eq!(
            group
                .members
                .iter()
                .filter(|guid| **guid == invitee)
                .count(),
            1
        );
    }

    #[test]
    fn concurrent_pending_group_accepts_create_once_then_join_once() {
        let registry = std::sync::Arc::new(GroupRegistry::default());
        let pending = std::sync::Arc::new(PendingInvites::default());
        let leader = ObjectGuid::create_player(1, 42);
        let invitees = [
            ObjectGuid::create_player(1, 77),
            ObjectGuid::create_player(1, 78),
        ];
        let invite = PendingInviteLikeCpp::new_pending_group(leader, GROUP_CATEGORY_HOME_LIKE_CPP);
        pending.insert(leader, invite);
        for invitee in invitees {
            pending.insert(invitee, invite);
        }

        let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
        let handles: Vec<_> = invitees
            .into_iter()
            .map(|invitee| {
                let registry = std::sync::Arc::clone(&registry);
                let pending = std::sync::Arc::clone(&pending);
                let barrier = std::sync::Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    registry.accept_invite_like_cpp(&pending, invitee, None, Some(leader))
                })
            })
            .collect();
        barrier.wait();
        let results: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().expect("accept thread"))
            .collect();

        assert_eq!(
            results
                .iter()
                .filter(|result| matches!(result, AcceptGroupInviteResultLikeCpp::Created { .. }))
                .count(),
            1
        );
        assert_eq!(
            results
                .iter()
                .filter(|result| {
                    matches!(
                        result,
                        AcceptGroupInviteResultLikeCpp::JoinedExisting { .. }
                    )
                })
                .count(),
            1
        );
        let groups = registry.snapshots();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].members.len(), 3);
        assert!(groups[0].members.contains(&leader));
        assert!(
            invitees
                .iter()
                .all(|invitee| groups[0].members.contains(invitee))
        );
    }

    #[test]
    fn invite_transition_failures_do_not_partially_mutate_state() {
        let registry = GroupRegistry::default();
        let pending = PendingInvites::default();
        let leader = ObjectGuid::create_player(1, 42);
        let invitee = ObjectGuid::create_player(1, 77);
        let other_leader = ObjectGuid::create_player(1, 90);
        let existing =
            PendingInviteLikeCpp::new_pending_group(other_leader, GROUP_CATEGORY_HOME_LIKE_CPP);
        pending.insert(invitee, existing);

        assert_eq!(
            registry.create_invite_like_cpp(
                &pending,
                leader,
                invitee,
                None,
                GROUP_CATEGORY_HOME_LIKE_CPP,
                GROUP_CATEGORY_HOME_LIKE_CPP,
            ),
            CreateGroupInviteResultLikeCpp::TargetAlreadyInvited
        );
        assert_eq!(pending.get(&invitee), Some(existing));
        assert!(pending.get(&leader).is_none());

        let missing_target = ObjectGuid::create_player(1, 78);
        assert_eq!(
            registry.create_invite_like_cpp(
                &pending,
                leader,
                missing_target,
                Some(u64::MAX),
                GROUP_CATEGORY_HOME_LIKE_CPP,
                GROUP_CATEGORY_HOME_LIKE_CPP,
            ),
            CreateGroupInviteResultLikeCpp::MissingInviterGroup
        );
        assert!(pending.get(&missing_target).is_none());

        let missing_group_invite = PendingInviteLikeCpp::new_existing_group(
            leader,
            u64::MAX,
            GROUP_CATEGORY_HOME_LIKE_CPP,
        );
        pending.insert(invitee, missing_group_invite);
        assert!(matches!(
            registry.accept_invite_like_cpp(&pending, invitee, None, Some(leader)),
            AcceptGroupInviteResultLikeCpp::MissingGroup
        ));
        assert_eq!(pending.get(&invitee), Some(missing_group_invite));
        assert!(registry.snapshots().is_empty());

        pending.insert(invitee, existing);
        assert!(matches!(
            registry.accept_invite_like_cpp(
                &pending,
                invitee,
                Some(GROUP_CATEGORY_INSTANCE_LIKE_CPP),
                Some(leader),
            ),
            AcceptGroupInviteResultLikeCpp::WrongCategory
        ));
        assert_eq!(pending.get(&invitee), Some(existing));

        let mut duplicate_group = GroupInfo::new(leader);
        assert!(duplicate_group.add_member(invitee));
        let duplicate_group_guid = duplicate_group.group_guid;
        registry.insert(duplicate_group_guid, duplicate_group.clone());
        pending.insert(
            invitee,
            PendingInviteLikeCpp::new_existing_group(
                leader,
                duplicate_group_guid,
                GROUP_CATEGORY_HOME_LIKE_CPP,
            ),
        );
        assert!(matches!(
            registry.accept_invite_like_cpp(&pending, invitee, None, Some(leader)),
            AcceptGroupInviteResultLikeCpp::AlreadyMember
        ));
        assert!(pending.get(&invitee).is_none());
        assert_eq!(
            registry.get(&duplicate_group_guid).unwrap().members,
            duplicate_group.members
        );
    }

    #[test]
    fn stale_delivery_failure_cannot_cancel_a_replacement_invite() {
        let registry = GroupRegistry::default();
        let pending = PendingInvites::default();
        let invitee = ObjectGuid::create_player(1, 77);
        let stale = PendingInviteLikeCpp::new_pending_group(
            ObjectGuid::create_player(1, 42),
            GROUP_CATEGORY_HOME_LIKE_CPP,
        );
        let replacement = PendingInviteLikeCpp::new_pending_group(
            ObjectGuid::create_player(1, 43),
            GROUP_CATEGORY_HOME_LIKE_CPP,
        );
        pending.insert(invitee, replacement);

        assert!(!registry.cancel_invite_like_cpp(&pending, invitee, stale));
        assert_eq!(pending.get(&invitee), Some(replacement));
        assert!(!registry.replace_invite_like_cpp(&pending, invitee, stale, replacement));
        assert!(registry.replace_invite_like_cpp(&pending, invitee, replacement, stale));
        assert_eq!(pending.get(&invitee), Some(stale));
        assert!(registry.expire_invite_like_cpp(&pending, invitee, stale));
        assert!(pending.get(&invitee).is_none());

        let decline = PendingInviteLikeCpp::new_pending_group(
            ObjectGuid::create_player(1, 44),
            GROUP_CATEGORY_HOME_LIKE_CPP,
        );
        pending.insert(decline.leader_guid, decline);
        pending.insert(invitee, decline);
        assert!(
            registry
                .decline_invite_like_cpp(&pending, invitee, Some(GROUP_CATEGORY_INSTANCE_LIKE_CPP),)
                .is_none()
        );
        assert_eq!(pending.get(&invitee), Some(decline));
        assert_eq!(
            registry.decline_invite_like_cpp(&pending, invitee, None),
            Some(decline)
        );
        assert!(pending.get(&invitee).is_none());
        assert!(pending.get(&decline.leader_guid).is_none());
    }

    #[test]
    fn free_group_db_store_id_ignores_zero_like_cpp_unallocated_storage() {
        free_group_db_store_id_like_cpp(0);
    }

    #[test]
    fn group_db_store_registers_and_finds_group_by_storage_id_like_cpp() {
        let registry = GroupRegistry::default();
        let leader = ObjectGuid::create_player(1, 42);
        let group = GroupInfo::loaded_from_db_like_cpp(
            90,
            1234,
            leader,
            LOOT_METHOD_PERSONAL_LIKE_CPP,
            leader,
            ITEM_QUALITY_UNCOMMON_LIKE_CPP,
            0,
            DIFFICULTY_NORMAL_LIKE_CPP,
            DIFFICULTY_NORMAL_RAID_LIKE_CPP,
            DIFFICULTY_10_N_LIKE_CPP,
            ObjectGuid::EMPTY,
        );
        registry.insert(group.group_guid, group);

        register_group_db_store_id_like_cpp(1234, 90);

        let found = get_group_by_db_store_id_like_cpp(&registry, 1234)
            .expect("registered storage id should resolve to its group");
        assert_eq!(found.group_guid, 90);
        assert_eq!(found.db_store_id, 1234);
    }

    #[test]
    fn group_db_store_free_clears_lookup_like_cpp() {
        let registry = GroupRegistry::default();
        let leader = ObjectGuid::create_player(1, 43);
        let group = GroupInfo::loaded_from_db_like_cpp(
            91,
            1235,
            leader,
            LOOT_METHOD_PERSONAL_LIKE_CPP,
            leader,
            ITEM_QUALITY_UNCOMMON_LIKE_CPP,
            0,
            DIFFICULTY_NORMAL_LIKE_CPP,
            DIFFICULTY_NORMAL_RAID_LIKE_CPP,
            DIFFICULTY_10_N_LIKE_CPP,
            ObjectGuid::EMPTY,
        );
        registry.insert(group.group_guid, group);
        register_group_db_store_id_like_cpp(1235, 91);

        free_group_db_store_id_like_cpp(1235);

        assert!(get_group_by_db_store_id_like_cpp(&registry, 1235).is_none());
    }

    #[test]
    fn loaded_group_row_preserves_cpp_group_db_fields_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let looter = ObjectGuid::create_player(1, 77);
        let master = ObjectGuid::create_player(1, 88);
        let group = GroupInfo::loaded_from_db_like_cpp(
            900,
            17,
            leader,
            3,
            looter,
            4,
            GROUP_FLAG_RAID_LIKE_CPP,
            2,
            15,
            5,
            master,
        );

        assert_eq!(group.group_guid, 900);
        assert_eq!(group.db_store_id, 17);
        assert_eq!(group.leader_guid, leader);
        assert!(group.members.is_empty());
        assert_eq!(group.loot_method, 3);
        assert_eq!(group.looter_guid, looter);
        assert_eq!(group.loot_threshold, 4);
        assert_eq!(group.group_flags, GROUP_FLAG_RAID_LIKE_CPP);
        assert_eq!(group.dungeon_difficulty_id, 2);
        assert_eq!(group.raid_difficulty_id, 15);
        assert_eq!(group.legacy_raid_difficulty_id, 5);
        assert_eq!(group.master_looter_guid, master);
    }

    #[test]
    fn recent_instance_defaults_to_leader_and_zero_instance_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let group = GroupInfo::new(leader);

        assert_eq!(group.recent_instance_owner_like_cpp(631), leader);
        assert_eq!(group.recent_instance_id_like_cpp(631), 0);
    }

    #[test]
    fn set_recent_instance_tracks_owner_and_instance_by_map_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let owner = ObjectGuid::create_player(1, 77);
        let mut group = GroupInfo::new(leader);

        group.set_recent_instance_like_cpp(631, owner, 9001);

        assert_eq!(group.recent_instance_owner_like_cpp(631), owner);
        assert_eq!(group.recent_instance_id_like_cpp(631), 9001);
        assert_eq!(
            group.recent_instance_owner_like_cpp(533),
            leader,
            "other maps still fall back to C++ leader guid"
        );
        assert_eq!(group.recent_instance_id_like_cpp(533), 0);
    }

    #[test]
    fn set_recent_instance_replaces_same_map_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let first_owner = ObjectGuid::create_player(1, 77);
        let second_owner = ObjectGuid::create_player(1, 88);
        let mut group = GroupInfo::new(leader);

        group.set_recent_instance_like_cpp(631, first_owner, 9001);
        group.set_recent_instance_like_cpp(631, second_owner, 9002);

        assert_eq!(group.recent_instance_owner_like_cpp(631), second_owner);
        assert_eq!(group.recent_instance_id_like_cpp(631), 9002);
    }

    #[test]
    fn forget_recent_instance_erases_map_binding_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let owner = ObjectGuid::create_player(1, 77);
        let mut group = GroupInfo::new(leader);

        group.set_recent_instance_like_cpp(631, owner, 9001);

        assert!(group.forget_recent_instance_like_cpp(631));
        assert!(!group.forget_recent_instance_like_cpp(631));
        assert_eq!(group.recent_instance_owner_like_cpp(631), leader);
        assert_eq!(group.recent_instance_id_like_cpp(631), 0);
    }

    #[test]
    fn link_owned_instance_tracks_unique_instance_map_references_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let mut group = GroupInfo::new(leader);

        assert!(group.link_owned_instance_like_cpp(631, 9001));
        assert!(!group.link_owned_instance_like_cpp(631, 9001));
        assert!(group.link_owned_instance_like_cpp(631, 9002));

        let owned: Vec<_> = group.owned_instances_like_cpp().collect();
        assert_eq!(
            owned,
            vec![
                GroupOwnedInstanceLikeCpp {
                    map_id: 631,
                    instance_id: 9001,
                },
                GroupOwnedInstanceLikeCpp {
                    map_id: 631,
                    instance_id: 9002,
                },
            ]
        );
    }

    #[test]
    fn unlink_owned_instance_removes_reference_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let mut group = GroupInfo::new(leader);

        group.link_owned_instance_like_cpp(631, 9001);

        assert!(group.unlink_owned_instance_like_cpp(631, 9001));
        assert!(!group.unlink_owned_instance_like_cpp(631, 9001));
        assert_eq!(group.owned_instances_like_cpp().count(), 0);
    }

    #[test]
    fn reset_success_and_cannot_reset_forget_recent_instance_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let owner = ObjectGuid::create_player(1, 77);
        let mut group = GroupInfo::new(leader);

        group.set_recent_instance_like_cpp(631, owner, 9001);
        assert!(group.apply_owned_instance_reset_result_like_cpp(
            631,
            GroupInstanceResetResultLikeCpp::Success,
            GroupInstanceResetMethodLikeCpp::Manual,
        ));
        assert_eq!(group.recent_instance_id_like_cpp(631), 0);

        group.set_recent_instance_like_cpp(631, owner, 9002);
        assert!(group.apply_owned_instance_reset_result_like_cpp(
            631,
            GroupInstanceResetResultLikeCpp::CannotReset,
            GroupInstanceResetMethodLikeCpp::Manual,
        ));
        assert_eq!(group.recent_instance_id_like_cpp(631), 0);
    }

    #[test]
    fn reset_not_empty_forgets_only_on_change_difficulty_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let owner = ObjectGuid::create_player(1, 77);
        let mut group = GroupInfo::new(leader);

        group.set_recent_instance_like_cpp(631, owner, 9001);
        assert!(!group.apply_owned_instance_reset_result_like_cpp(
            631,
            GroupInstanceResetResultLikeCpp::NotEmpty,
            GroupInstanceResetMethodLikeCpp::Manual,
        ));
        assert_eq!(group.recent_instance_id_like_cpp(631), 9001);

        assert!(group.apply_owned_instance_reset_result_like_cpp(
            631,
            GroupInstanceResetResultLikeCpp::NotEmpty,
            GroupInstanceResetMethodLikeCpp::OnChangeDifficulty,
        ));
        assert_eq!(group.recent_instance_id_like_cpp(631), 0);
    }

    #[test]
    fn reset_other_result_keeps_recent_instance_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let owner = ObjectGuid::create_player(1, 77);
        let mut group = GroupInfo::new(leader);

        group.set_recent_instance_like_cpp(631, owner, 9001);
        assert!(!group.apply_owned_instance_reset_result_like_cpp(
            631,
            GroupInstanceResetResultLikeCpp::Other,
            GroupInstanceResetMethodLikeCpp::Manual,
        ));
        assert_eq!(group.recent_instance_id_like_cpp(631), 9001);
    }

    #[test]
    fn loaded_group_row_validates_difficulties_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let difficulty_store = DifficultyStore::from_entries([
            wow_data::DifficultyEntry {
                id: 2,
                instance_type: 1,
                flags: wow_constants::shared::DifficultyFlags::CAN_SELECT.bits(),
                fallback_difficulty_id: 0,
                toggle_difficulty_id: 0,
            },
            wow_data::DifficultyEntry {
                id: 15,
                instance_type: 2,
                flags: wow_constants::shared::DifficultyFlags::CAN_SELECT.bits(),
                fallback_difficulty_id: 0,
                toggle_difficulty_id: 0,
            },
            wow_data::DifficultyEntry {
                id: 3,
                instance_type: 2,
                flags: (wow_constants::shared::DifficultyFlags::CAN_SELECT
                    | wow_constants::shared::DifficultyFlags::LEGACY)
                    .bits(),
                fallback_difficulty_id: 0,
                toggle_difficulty_id: 0,
            },
        ]);

        let valid = GroupInfo::loaded_from_db_validated_like_cpp(
            901,
            18,
            leader,
            LOOT_METHOD_PERSONAL_LIKE_CPP,
            leader,
            ITEM_QUALITY_UNCOMMON_LIKE_CPP,
            0,
            2,
            15,
            3,
            ObjectGuid::EMPTY,
            &difficulty_store,
        );
        assert_eq!(valid.dungeon_difficulty_id, 2);
        assert_eq!(valid.raid_difficulty_id, 15);
        assert_eq!(valid.legacy_raid_difficulty_id, 3);

        let fallback = GroupInfo::loaded_from_db_validated_like_cpp(
            902,
            19,
            leader,
            LOOT_METHOD_PERSONAL_LIKE_CPP,
            leader,
            ITEM_QUALITY_UNCOMMON_LIKE_CPP,
            0,
            15,
            3,
            15,
            ObjectGuid::EMPTY,
            &difficulty_store,
        );
        assert_eq!(fallback.dungeon_difficulty_id, DIFFICULTY_NORMAL_LIKE_CPP);
        assert_eq!(fallback.raid_difficulty_id, DIFFICULTY_NORMAL_RAID_LIKE_CPP);
        assert_eq!(fallback.legacy_raid_difficulty_id, DIFFICULTY_10_N_LIKE_CPP);
    }

    #[test]
    fn load_member_from_db_skips_missing_character_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let mut group = GroupInfo::loaded_from_db_like_cpp(
            903,
            20,
            leader,
            LOOT_METHOD_PERSONAL_LIKE_CPP,
            leader,
            ITEM_QUALITY_UNCOMMON_LIKE_CPP,
            0,
            DIFFICULTY_NORMAL_LIKE_CPP,
            DIFFICULTY_NORMAL_RAID_LIKE_CPP,
            DIFFICULTY_10_N_LIKE_CPP,
            ObjectGuid::EMPTY,
        );

        assert!(!group.load_member_from_db_like_cpp(77, 0, 1, 2, None));
        assert!(group.members.is_empty());
        assert!(group.member_slots.is_empty());
    }

    #[test]
    fn load_member_from_db_preserves_slot_fields_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let mut group = GroupInfo::loaded_from_db_like_cpp(
            904,
            21,
            leader,
            LOOT_METHOD_PERSONAL_LIKE_CPP,
            leader,
            ITEM_QUALITY_UNCOMMON_LIKE_CPP,
            GROUP_FLAG_RAID_LIKE_CPP,
            DIFFICULTY_NORMAL_LIKE_CPP,
            DIFFICULTY_NORMAL_RAID_LIKE_CPP,
            DIFFICULTY_10_N_LIKE_CPP,
            ObjectGuid::EMPTY,
        );

        assert!(group.load_member_from_db_like_cpp(
            77,
            0x04,
            3,
            2,
            Some(GroupMemberCharacterLikeCpp {
                name: "Member".to_string(),
                race: 4,
                class: 8,
            }),
        ));

        let member_guid = ObjectGuid::create_player(1, 77);
        assert_eq!(group.members, vec![member_guid]);
        let slot = group
            .member_slot_like_cpp(member_guid)
            .expect("loaded DB member should have a represented slot");
        assert_eq!(slot.name, "Member");
        assert_eq!(slot.race, 4);
        assert_eq!(slot.class, 8);
        assert_eq!(slot.subgroup, 3);
        assert_eq!(slot.flags, 0x04);
        assert_eq!(slot.roles, 2);
        assert!(!slot.ready_checked);
    }

    #[test]
    fn load_member_from_db_everyone_assistant_adds_assistant_flag_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let mut group = GroupInfo::loaded_from_db_like_cpp(
            905,
            22,
            leader,
            LOOT_METHOD_PERSONAL_LIKE_CPP,
            leader,
            ITEM_QUALITY_UNCOMMON_LIKE_CPP,
            GROUP_FLAG_EVERYONE_ASSISTANT_LIKE_CPP,
            DIFFICULTY_NORMAL_LIKE_CPP,
            DIFFICULTY_NORMAL_RAID_LIKE_CPP,
            DIFFICULTY_10_N_LIKE_CPP,
            ObjectGuid::EMPTY,
        );

        assert!(group.load_member_from_db_like_cpp(
            78,
            0,
            0,
            0,
            Some(GroupMemberCharacterLikeCpp {
                name: "Assistant".to_string(),
                race: 1,
                class: 2,
            }),
        ));

        let slot = group
            .member_slot_like_cpp(ObjectGuid::create_player(1, 78))
            .expect("loaded DB member should have a represented slot");
        assert_eq!(
            slot.flags & MEMBER_FLAG_ASSISTANT_LIKE_CPP,
            MEMBER_FLAG_ASSISTANT_LIKE_CPP
        );
    }

    #[test]
    fn loaded_raid_group_tracks_subgroup_counts_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let mut group = GroupInfo::loaded_from_db_like_cpp(
            906,
            23,
            leader,
            LOOT_METHOD_PERSONAL_LIKE_CPP,
            leader,
            ITEM_QUALITY_UNCOMMON_LIKE_CPP,
            GROUP_FLAG_RAID_LIKE_CPP,
            DIFFICULTY_NORMAL_LIKE_CPP,
            DIFFICULTY_NORMAL_RAID_LIKE_CPP,
            DIFFICULTY_10_N_LIKE_CPP,
            ObjectGuid::EMPTY,
        );

        assert!(group.has_free_slot_sub_group_like_cpp(3));
        for guid_low in 100..105 {
            assert!(group.load_member_from_db_like_cpp(
                guid_low,
                0,
                3,
                0,
                Some(GroupMemberCharacterLikeCpp {
                    name: format!("Member{guid_low}"),
                    race: 1,
                    class: 1,
                }),
            ));
        }

        assert!(!group.has_free_slot_sub_group_like_cpp(3));
        assert_eq!(
            group.member_group_like_cpp(ObjectGuid::create_player(1, 104)),
            3
        );
        assert_eq!(
            group.member_group_like_cpp(ObjectGuid::create_player(1, 999)),
            MISSING_MEMBER_GROUP_LIKE_CPP
        );

        group.remove_member(&ObjectGuid::create_player(1, 104));
        assert!(group.has_free_slot_sub_group_like_cpp(3));
    }

    #[test]
    fn convert_to_raid_initializes_subgroup_counts_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let mut group = GroupInfo::new(leader);
        assert!(!group.has_free_slot_sub_group_like_cpp(0));

        group.convert_to_raid_like_cpp();

        assert!(group.has_free_slot_sub_group_like_cpp(0));
        for guid_low in 200..204 {
            group.add_member(ObjectGuid::create_player(1, guid_low));
        }
        assert!(!group.has_free_slot_sub_group_like_cpp(0));
    }

    #[test]
    fn loaded_raid_group_rejects_out_of_range_subgroup_without_panicking_boundary() {
        let leader = ObjectGuid::create_player(1, 42);
        let mut group = GroupInfo::loaded_from_db_like_cpp(
            906,
            24,
            leader,
            LOOT_METHOD_PERSONAL_LIKE_CPP,
            leader,
            ITEM_QUALITY_UNCOMMON_LIKE_CPP,
            GROUP_FLAG_RAID_LIKE_CPP,
            DIFFICULTY_NORMAL_LIKE_CPP,
            DIFFICULTY_NORMAL_RAID_LIKE_CPP,
            DIFFICULTY_10_N_LIKE_CPP,
            ObjectGuid::EMPTY,
        );

        assert!(!group.load_member_from_db_like_cpp(
            300,
            0,
            MAX_RAID_SUBGROUPS_LIKE_CPP as u8,
            0,
            Some(GroupMemberCharacterLikeCpp {
                name: "Invalid".to_string(),
                race: 1,
                class: 1,
            }),
        ));
        assert!(group.members.is_empty());
        assert!(group.member_slots.is_empty());
    }

    #[test]
    fn group_member_flag_toggles_assistant_in_raid_without_uniqueness_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let first = ObjectGuid::create_player(1, 390);
        let second = ObjectGuid::create_player(1, 391);
        let mut group = GroupInfo::new(leader);
        group.add_member(first);
        group.add_member(second);
        group.convert_to_raid_like_cpp();
        let sequence_before = group.sequence_num;

        assert_eq!(
            group.set_assistant_leader_flag_like_cpp(first, true),
            Some(MEMBER_FLAG_ASSISTANT_LIKE_CPP)
        );
        assert_eq!(
            group.set_assistant_leader_flag_like_cpp(second, true),
            Some(MEMBER_FLAG_ASSISTANT_LIKE_CPP)
        );
        assert_eq!(
            group.member_slot_like_cpp(first).unwrap().flags & MEMBER_FLAG_ASSISTANT_LIKE_CPP,
            MEMBER_FLAG_ASSISTANT_LIKE_CPP
        );
        assert_eq!(
            group.member_slot_like_cpp(second).unwrap().flags & MEMBER_FLAG_ASSISTANT_LIKE_CPP,
            MEMBER_FLAG_ASSISTANT_LIKE_CPP
        );
        assert_eq!(group.sequence_num, sequence_before + 2);

        assert_eq!(
            group.set_assistant_leader_flag_like_cpp(first, false),
            Some(0)
        );
        assert_eq!(group.member_slot_like_cpp(first).unwrap().flags, 0);
    }

    #[test]
    fn group_member_flag_returns_final_flags_even_when_unchanged_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 392);
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        group.convert_to_raid_like_cpp();

        assert_eq!(
            group.set_assistant_leader_flag_like_cpp(member, true),
            Some(MEMBER_FLAG_ASSISTANT_LIKE_CPP)
        );
        let sequence_after_change = group.sequence_num;
        assert_eq!(
            group.set_assistant_leader_flag_like_cpp(member, true),
            Some(MEMBER_FLAG_ASSISTANT_LIKE_CPP)
        );
        assert_eq!(group.sequence_num, sequence_after_change);
    }

    #[test]
    fn group_member_flag_rejects_non_raid_missing_or_unsupported_flag_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 393);
        let missing = ObjectGuid::create_player(1, 394);
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        let sequence_before = group.sequence_num;

        assert_eq!(group.set_assistant_leader_flag_like_cpp(member, true), None);
        group.convert_to_raid_like_cpp();
        assert_eq!(
            group.set_assistant_leader_flag_like_cpp(missing, true),
            None
        );
        assert_eq!(
            group.set_group_member_flag_like_cpp(member, true, 0x08),
            None
        );
        assert_eq!(group.member_slot_like_cpp(member).unwrap().flags, 0);
        assert_eq!(group.sequence_num, sequence_before + 1);
    }

    #[test]
    fn everyone_is_assistant_apply_marks_group_and_all_members_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let first = ObjectGuid::create_player(1, 395);
        let second = ObjectGuid::create_player(1, 396);
        let mut group = GroupInfo::new(leader);
        group.add_member(first);
        group.add_member(second);
        let sequence_before = group.sequence_num;

        let (group_flags, db_store_id) = group.set_everyone_is_assistant_like_cpp(true);

        assert_eq!(db_store_id, group.db_store_id);
        assert_eq!(
            group_flags & GROUP_FLAG_EVERYONE_ASSISTANT_LIKE_CPP,
            GROUP_FLAG_EVERYONE_ASSISTANT_LIKE_CPP
        );
        for guid in [leader, first, second] {
            assert_eq!(
                group.member_slot_like_cpp(guid).unwrap().flags & MEMBER_FLAG_ASSISTANT_LIKE_CPP,
                MEMBER_FLAG_ASSISTANT_LIKE_CPP
            );
        }
        assert_eq!(group.sequence_num, sequence_before + 1);
    }

    #[test]
    fn everyone_is_assistant_clear_unmarks_group_and_all_assistants_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let first = ObjectGuid::create_player(1, 397);
        let mut group = GroupInfo::new(leader);
        group.add_member(first);
        group.set_everyone_is_assistant_like_cpp(true);
        let sequence_after_apply = group.sequence_num;

        let (group_flags, db_store_id) = group.set_everyone_is_assistant_like_cpp(false);

        assert_eq!(db_store_id, group.db_store_id);
        assert_eq!(group_flags & GROUP_FLAG_EVERYONE_ASSISTANT_LIKE_CPP, 0);
        for guid in [leader, first] {
            assert_eq!(
                group.member_slot_like_cpp(guid).unwrap().flags & MEMBER_FLAG_ASSISTANT_LIKE_CPP,
                0
            );
        }
        assert_eq!(group.sequence_num, sequence_after_apply + 1);
    }

    #[test]
    fn everyone_is_assistant_works_in_non_raid_group_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 398);
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        assert!(!group.is_raid_group());

        group.set_everyone_is_assistant_like_cpp(true);

        assert_eq!(
            group.group_flags & GROUP_FLAG_EVERYONE_ASSISTANT_LIKE_CPP,
            GROUP_FLAG_EVERYONE_ASSISTANT_LIKE_CPP
        );
        assert_eq!(
            group.member_slot_like_cpp(member).unwrap().flags & MEMBER_FLAG_ASSISTANT_LIKE_CPP,
            MEMBER_FLAG_ASSISTANT_LIKE_CPP
        );
    }

    #[test]
    fn everyone_is_assistant_idempotent_returns_final_flags_without_sequence_bump_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 399);
        let mut group = GroupInfo::new(leader);
        group.add_member(member);

        let (first_flags, first_db_store_id) = group.set_everyone_is_assistant_like_cpp(true);
        let sequence_after_apply = group.sequence_num;
        let (second_flags, second_db_store_id) = group.set_everyone_is_assistant_like_cpp(true);

        assert_eq!(second_flags, first_flags);
        assert_eq!(second_db_store_id, first_db_store_id);
        assert_eq!(group.sequence_num, sequence_after_apply);
    }

    #[test]
    fn change_leader_like_cpp_sets_leader_and_clears_assistant_flag() {
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 400);
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        group.convert_to_raid_like_cpp();
        assert_eq!(
            group.set_assistant_leader_flag_like_cpp(member, true),
            Some(MEMBER_FLAG_ASSISTANT_LIKE_CPP)
        );
        let previous_sequence = group.sequence_num;

        assert_eq!(group.change_leader_like_cpp(member), Some(0));

        assert_eq!(group.leader_guid, member);
        assert_eq!(group.member_slot_like_cpp(member).unwrap().flags, 0);
        assert_eq!(group.sequence_num, previous_sequence + 1);
    }

    #[test]
    fn change_leader_like_cpp_rejects_missing_member() {
        let leader = ObjectGuid::create_player(1, 42);
        let missing = ObjectGuid::create_player(1, 401);
        let mut group = GroupInfo::new(leader);

        assert_eq!(group.change_leader_like_cpp(missing), None);
        assert_eq!(group.leader_guid, leader);
    }

    #[test]
    fn change_member_group_updates_raid_subgroup_counts_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let mut group = GroupInfo::loaded_from_db_like_cpp(
            907,
            25,
            leader,
            LOOT_METHOD_PERSONAL_LIKE_CPP,
            leader,
            ITEM_QUALITY_UNCOMMON_LIKE_CPP,
            GROUP_FLAG_RAID_LIKE_CPP,
            DIFFICULTY_NORMAL_LIKE_CPP,
            DIFFICULTY_NORMAL_RAID_LIKE_CPP,
            DIFFICULTY_10_N_LIKE_CPP,
            ObjectGuid::EMPTY,
        );
        let member = ObjectGuid::create_player(1, 400);
        assert!(group.load_member_from_db_like_cpp(
            400,
            0,
            0,
            0,
            Some(GroupMemberCharacterLikeCpp {
                name: "Mover".to_string(),
                race: 1,
                class: 1,
            }),
        ));

        assert!(group.change_member_group_like_cpp(member, 2));
        assert_eq!(group.member_group_like_cpp(member), 2);

        for guid_low in 401..406 {
            assert!(group.load_member_from_db_like_cpp(
                guid_low,
                0,
                0,
                0,
                Some(GroupMemberCharacterLikeCpp {
                    name: format!("Member{guid_low}"),
                    race: 1,
                    class: 1,
                }),
            ));
        }
        assert!(!group.has_free_slot_sub_group_like_cpp(0));
        assert!(group.has_free_slot_sub_group_like_cpp(2));
    }

    #[test]
    fn change_member_group_rejects_non_raid_missing_full_or_same_group_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 500);
        let mut party = GroupInfo::new(leader);
        party.add_member(member);
        assert!(!party.change_member_group_like_cpp(member, 1));

        let mut raid = GroupInfo::loaded_from_db_like_cpp(
            908,
            26,
            leader,
            LOOT_METHOD_PERSONAL_LIKE_CPP,
            leader,
            ITEM_QUALITY_UNCOMMON_LIKE_CPP,
            GROUP_FLAG_RAID_LIKE_CPP,
            DIFFICULTY_NORMAL_LIKE_CPP,
            DIFFICULTY_NORMAL_RAID_LIKE_CPP,
            DIFFICULTY_10_N_LIKE_CPP,
            ObjectGuid::EMPTY,
        );
        assert!(!raid.change_member_group_like_cpp(member, 1));
        assert!(raid.load_member_from_db_like_cpp(
            500,
            0,
            0,
            0,
            Some(GroupMemberCharacterLikeCpp {
                name: "Mover".to_string(),
                race: 1,
                class: 1,
            }),
        ));
        assert!(!raid.change_member_group_like_cpp(member, 0));
        assert!(!raid.change_member_group_like_cpp(member, MAX_RAID_SUBGROUPS_LIKE_CPP as u8));
    }

    #[test]
    fn swap_members_groups_like_cpp_swaps_raid_members_without_counter_drift() {
        let leader = ObjectGuid::create_player(1, 42);
        let first = ObjectGuid::create_player(1, 600);
        let second = ObjectGuid::create_player(1, 601);
        let mut group = GroupInfo::loaded_from_db_like_cpp(
            909,
            27,
            leader,
            LOOT_METHOD_PERSONAL_LIKE_CPP,
            leader,
            ITEM_QUALITY_UNCOMMON_LIKE_CPP,
            GROUP_FLAG_RAID_LIKE_CPP,
            DIFFICULTY_NORMAL_LIKE_CPP,
            DIFFICULTY_NORMAL_RAID_LIKE_CPP,
            DIFFICULTY_10_N_LIKE_CPP,
            ObjectGuid::EMPTY,
        );
        assert!(group.load_member_from_db_like_cpp(
            600,
            0,
            1,
            0,
            Some(GroupMemberCharacterLikeCpp {
                name: "First".to_string(),
                race: 1,
                class: 1,
            }),
        ));
        assert!(group.load_member_from_db_like_cpp(
            601,
            0,
            2,
            0,
            Some(GroupMemberCharacterLikeCpp {
                name: "Second".to_string(),
                race: 1,
                class: 1,
            }),
        ));
        let counts_before = group.raid_subgroup_counts;
        let sequence_before = group.sequence_num;

        let updates = group
            .swap_members_groups_like_cpp(first, second)
            .expect("different raid subgroups should swap");

        assert_eq!(updates, [(first, 2), (second, 1)]);
        assert_eq!(group.member_group_like_cpp(first), 2);
        assert_eq!(group.member_group_like_cpp(second), 1);
        assert_eq!(group.raid_subgroup_counts, counts_before);
        assert!(group.has_free_slot_sub_group_like_cpp(1));
        assert!(group.has_free_slot_sub_group_like_cpp(2));
        assert_eq!(group.sequence_num, sequence_before + 1);
    }

    #[test]
    fn swap_members_groups_like_cpp_rejects_party_missing_member_or_same_subgroup() {
        let leader = ObjectGuid::create_player(1, 42);
        let first = ObjectGuid::create_player(1, 610);
        let second = ObjectGuid::create_player(1, 611);
        let missing = ObjectGuid::create_player(1, 612);

        let mut party = GroupInfo::new(leader);
        party.add_member(first);
        party.add_member(second);
        assert_eq!(party.swap_members_groups_like_cpp(first, second), None);

        let mut raid = GroupInfo::loaded_from_db_like_cpp(
            910,
            28,
            leader,
            LOOT_METHOD_PERSONAL_LIKE_CPP,
            leader,
            ITEM_QUALITY_UNCOMMON_LIKE_CPP,
            GROUP_FLAG_RAID_LIKE_CPP,
            DIFFICULTY_NORMAL_LIKE_CPP,
            DIFFICULTY_NORMAL_RAID_LIKE_CPP,
            DIFFICULTY_10_N_LIKE_CPP,
            ObjectGuid::EMPTY,
        );
        assert!(raid.load_member_from_db_like_cpp(
            610,
            0,
            3,
            0,
            Some(GroupMemberCharacterLikeCpp {
                name: "First".to_string(),
                race: 1,
                class: 1,
            }),
        ));
        assert!(raid.load_member_from_db_like_cpp(
            611,
            0,
            3,
            0,
            Some(GroupMemberCharacterLikeCpp {
                name: "Second".to_string(),
                race: 1,
                class: 1,
            }),
        ));
        let counts_before = raid.raid_subgroup_counts;
        let sequence_before = raid.sequence_num;

        assert_eq!(raid.swap_members_groups_like_cpp(first, missing), None);
        assert_eq!(raid.swap_members_groups_like_cpp(first, second), None);
        assert_eq!(raid.member_group_like_cpp(first), 3);
        assert_eq!(raid.member_group_like_cpp(second), 3);
        assert_eq!(raid.raid_subgroup_counts, counts_before);
        assert_eq!(raid.sequence_num, sequence_before);
    }

    #[test]
    fn target_icon_list_returns_all_eight_symbols_in_cpp_order() {
        let target = ObjectGuid::create_player(1, 77);
        let mut group = GroupInfo::new(ObjectGuid::create_player(1, 42));
        group.target_icons[3] = target.to_raw_bytes();

        let icons = group.target_icon_list_like_cpp();

        assert_eq!(icons.len(), TARGET_ICONS_COUNT_LIKE_CPP);
        assert_eq!(icons[0], (0, ObjectGuid::EMPTY));
        assert_eq!(icons[3], (3, target));
        assert_eq!(icons[7], (7, ObjectGuid::EMPTY));
    }

    #[test]
    fn set_target_icon_out_of_range_does_not_mutate_like_cpp() {
        let target = ObjectGuid::create_player(1, 77);
        let mut group = GroupInfo::new(ObjectGuid::create_player(1, 42));

        assert_eq!(group.set_target_icon_like_cpp(8, target), None);
        assert!(
            group
                .target_icons
                .iter()
                .all(|raw| *raw == EMPTY_TARGET_ICON_RAW_LIKE_CPP)
        );
    }

    #[test]
    fn set_target_icon_clears_duplicate_target_before_assignment_like_cpp() {
        let target = ObjectGuid::create_player(1, 77);
        let mut group = GroupInfo::new(ObjectGuid::create_player(1, 42));
        group.set_target_icon_like_cpp(2, target).unwrap();

        let updates = group.set_target_icon_like_cpp(5, target).unwrap();

        assert_eq!(updates, vec![(2, ObjectGuid::EMPTY), (5, target)]);
        assert_eq!(group.target_icons[2], EMPTY_TARGET_ICON_RAW_LIKE_CPP);
        assert_eq!(group.target_icons[5], target.to_raw_bytes());
        assert_eq!(
            group
                .target_icon_list_like_cpp()
                .into_iter()
                .filter(|(_, icon_target)| *icon_target == target)
                .count(),
            1
        );
    }

    #[test]
    fn add_raid_marker_preserves_cpp_slots_mask_and_duplicate_rejection() {
        let transport = ObjectGuid::create_transport(wow_core::guid::HighGuid::Transport, 0x55AA);
        let mut group = GroupInfo::new(ObjectGuid::create_player(1, 42));
        let sequence_before = group.sequence_num;
        let position = Position::xyz(12.25, -34.5, 6.75);

        assert!(group.add_raid_marker_like_cpp(3, 571, position, transport));
        assert_eq!(group.active_raid_markers_mask_like_cpp(), 1 << 3);
        assert_eq!(
            group.raid_marker_list_like_cpp(),
            vec![RaidMarkerLikeCpp {
                map_id: 571,
                position,
                transport_guid: transport,
            }]
        );
        assert_eq!(
            group.sequence_num, sequence_before,
            "C++ Group::AddRaidMarker sends RaidMarkersChanged and does not advance PartyUpdate sequence"
        );

        assert!(!group.add_raid_marker_like_cpp(3, 1, Position::ZERO, ObjectGuid::EMPTY));
        assert!(!group.add_raid_marker_like_cpp(8, 1, Position::ZERO, ObjectGuid::EMPTY));
        assert_eq!(group.active_raid_markers_mask_like_cpp(), 1 << 3);
        assert_eq!(group.raid_marker_list_like_cpp().len(), 1);
    }

    #[test]
    fn delete_raid_marker_preserves_cpp_single_all_and_out_of_range_semantics() {
        let mut group = GroupInfo::new(ObjectGuid::create_player(1, 42));
        group.add_raid_marker_like_cpp(1, 571, Position::xyz(1.0, 2.0, 3.0), ObjectGuid::EMPTY);
        group.add_raid_marker_like_cpp(3, 571, Position::xyz(4.0, 5.0, 6.0), ObjectGuid::EMPTY);

        assert!(group.delete_raid_marker_like_cpp(1));
        assert_eq!(group.active_raid_markers_mask_like_cpp(), 1 << 3);
        assert!(!group.delete_raid_marker_like_cpp(1));
        assert_eq!(group.active_raid_markers_mask_like_cpp(), 1 << 3);

        assert!(!group.delete_raid_marker_like_cpp(9));
        assert_eq!(group.active_raid_markers_mask_like_cpp(), 1 << 3);

        assert!(group.delete_raid_marker_like_cpp(RAID_MARKERS_COUNT_LIKE_CPP as u8));
        assert_eq!(group.active_raid_markers_mask_like_cpp(), 0);
        assert!(group.raid_marker_list_like_cpp().is_empty());
    }

    #[test]
    fn update_looter_guid_preserves_cpp_free_for_all_noop() {
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 43);
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        group.loot_method = LOOT_METHOD_FREE_FOR_ALL_LIKE_CPP;
        group.looter_guid = leader;
        let sequence_before = group.sequence_num;

        assert!(!group.update_looter_guid_like_cpp([member], false));

        assert_eq!(group.looter_guid, leader);
        assert_eq!(group.looter_guid_like_cpp(), ObjectGuid::EMPTY);
        assert_eq!(group.sequence_num, sequence_before);
    }

    #[test]
    fn update_looter_guid_ifneed_keeps_current_eligible_looter_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 43);
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        group.looter_guid = leader;
        let sequence_before = group.sequence_num;

        assert!(!group.update_looter_guid_like_cpp([leader, member], true));

        assert_eq!(group.looter_guid, leader);
        assert_eq!(group.sequence_num, sequence_before);
    }

    #[test]
    fn update_looter_guid_rotates_to_next_eligible_member_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let first = ObjectGuid::create_player(1, 43);
        let second = ObjectGuid::create_player(1, 44);
        let mut group = GroupInfo::new(leader);
        group.add_member(first);
        group.add_member(second);
        group.looter_guid = leader;
        let sequence_before = group.sequence_num;

        assert!(group.update_looter_guid_like_cpp([second], false));

        assert_eq!(group.looter_guid, second);
        assert_eq!(group.sequence_num, sequence_before + 1);
    }

    #[test]
    fn update_looter_guid_wraps_without_updating_when_only_current_is_eligible_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 43);
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        group.looter_guid = member;
        let sequence_before = group.sequence_num;

        assert!(!group.update_looter_guid_like_cpp([member], false));

        assert_eq!(group.looter_guid, member);
        assert_eq!(group.sequence_num, sequence_before);
    }

    #[test]
    fn update_looter_guid_clears_when_no_member_is_eligible_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 43);
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        group.looter_guid = member;
        let sequence_before = group.sequence_num;

        assert!(group.update_looter_guid_like_cpp([], false));

        assert_eq!(group.looter_guid, ObjectGuid::EMPTY);
        assert_eq!(group.sequence_num, sequence_before + 1);
    }

    #[test]
    fn load_group_from_db_row_preserves_target_icons_and_validates_difficulties_like_cpp() {
        let difficulty_store = DifficultyStore::from_entries([
            wow_data::DifficultyEntry {
                id: 2,
                instance_type: 1,
                flags: wow_constants::shared::DifficultyFlags::CAN_SELECT.bits(),
                fallback_difficulty_id: 0,
                toggle_difficulty_id: 0,
            },
            wow_data::DifficultyEntry {
                id: 15,
                instance_type: 2,
                flags: wow_constants::shared::DifficultyFlags::CAN_SELECT.bits(),
                fallback_difficulty_id: 0,
                toggle_difficulty_id: 0,
            },
            wow_data::DifficultyEntry {
                id: 3,
                instance_type: 2,
                flags: (wow_constants::shared::DifficultyFlags::CAN_SELECT
                    | wow_constants::shared::DifficultyFlags::LEGACY)
                    .bits(),
                fallback_difficulty_id: 0,
                toggle_difficulty_id: 0,
            },
        ]);
        let mut target_icons = [EMPTY_TARGET_ICON_RAW_LIKE_CPP; TARGET_ICONS_COUNT_LIKE_CPP];
        target_icons[0] = [1; 16];
        target_icons[7] = [8; 16];

        let group = GroupInfo::load_group_from_db_row_validated_like_cpp(
            906,
            GroupDbRowLikeCpp {
                leader_guid_low: 42,
                loot_method: 3,
                looter_guid_low: 77,
                loot_threshold: 4,
                target_icons,
                group_flags: GROUP_FLAG_RAID_LIKE_CPP,
                dungeon_difficulty_id: 15,
                raid_difficulty_id: 3,
                legacy_raid_difficulty_id: 15,
                master_looter_guid_low: 88,
                db_store_id: 23,
                lfg_dungeon_id: Some(100),
                lfg_state: Some(2),
            },
            Some(GroupMemberCharacterLikeCpp {
                name: "Leader".to_string(),
                race: 1,
                class: 1,
            }),
            &difficulty_store,
        )
        .expect("valid leader projection should hydrate represented group row");

        assert_eq!(group.group_guid, 906);
        assert_eq!(group.db_store_id, 23);
        assert_eq!(group.leader_guid, ObjectGuid::create_player(1, 42));
        assert_eq!(group.loot_method, 3);
        assert_eq!(group.looter_guid, ObjectGuid::create_player(1, 77));
        assert_eq!(group.loot_threshold, 4);
        assert_eq!(group.group_flags, GROUP_FLAG_RAID_LIKE_CPP);
        assert_eq!(group.dungeon_difficulty_id, DIFFICULTY_NORMAL_LIKE_CPP);
        assert_eq!(group.raid_difficulty_id, DIFFICULTY_NORMAL_RAID_LIKE_CPP);
        assert_eq!(group.legacy_raid_difficulty_id, DIFFICULTY_10_N_LIKE_CPP);
        assert_eq!(group.master_looter_guid, ObjectGuid::create_player(1, 88));
        assert_eq!(group.target_icons[0], [1; 16]);
        assert_eq!(group.target_icons[7], [8; 16]);
        assert_eq!(group.lfg_db_state, None);
    }

    #[test]
    fn load_group_from_db_row_skips_missing_leader_character_like_cpp_cleanup_boundary() {
        let difficulty_store = DifficultyStore::from_entries([]);
        let group = GroupInfo::load_group_from_db_row_validated_like_cpp(
            907,
            GroupDbRowLikeCpp {
                leader_guid_low: 42,
                loot_method: LOOT_METHOD_PERSONAL_LIKE_CPP,
                looter_guid_low: 42,
                loot_threshold: ITEM_QUALITY_UNCOMMON_LIKE_CPP,
                target_icons: [EMPTY_TARGET_ICON_RAW_LIKE_CPP; TARGET_ICONS_COUNT_LIKE_CPP],
                group_flags: 0,
                dungeon_difficulty_id: DIFFICULTY_NORMAL_LIKE_CPP,
                raid_difficulty_id: DIFFICULTY_NORMAL_RAID_LIKE_CPP,
                legacy_raid_difficulty_id: DIFFICULTY_10_N_LIKE_CPP,
                master_looter_guid_low: 0,
                db_store_id: 24,
                lfg_dungeon_id: None,
                lfg_state: None,
            },
            None,
            &difficulty_store,
        );

        assert!(group.is_none());
    }

    #[test]
    fn load_group_from_db_row_restores_lfg_dungeon_and_dungeon_state_like_cpp() {
        let difficulty_store = DifficultyStore::from_entries([]);
        let group = GroupInfo::load_group_from_db_row_validated_like_cpp(
            908,
            GroupDbRowLikeCpp {
                leader_guid_low: 42,
                loot_method: LOOT_METHOD_PERSONAL_LIKE_CPP,
                looter_guid_low: 42,
                loot_threshold: ITEM_QUALITY_UNCOMMON_LIKE_CPP,
                target_icons: [EMPTY_TARGET_ICON_RAW_LIKE_CPP; TARGET_ICONS_COUNT_LIKE_CPP],
                group_flags: GROUP_FLAG_LFG_LIKE_CPP,
                dungeon_difficulty_id: DIFFICULTY_NORMAL_LIKE_CPP,
                raid_difficulty_id: DIFFICULTY_NORMAL_RAID_LIKE_CPP,
                legacy_raid_difficulty_id: DIFFICULTY_10_N_LIKE_CPP,
                master_looter_guid_low: 0,
                db_store_id: 25,
                lfg_dungeon_id: Some(123),
                lfg_state: Some(LFG_STATE_DUNGEON_LIKE_CPP),
            },
            Some(GroupMemberCharacterLikeCpp {
                name: "Leader".to_string(),
                race: 1,
                class: 1,
            }),
            &difficulty_store,
        )
        .expect("valid LFG group row should hydrate");

        assert_eq!(
            group.lfg_db_state,
            Some(GroupLfgDbStateLikeCpp {
                dungeon_id: 123,
                state: Some(LFG_STATE_DUNGEON_LIKE_CPP),
            })
        );
    }

    #[test]
    fn load_group_from_db_row_preserves_lfg_dungeon_without_unsupported_state_like_cpp() {
        let difficulty_store = DifficultyStore::from_entries([]);
        let group = GroupInfo::load_group_from_db_row_validated_like_cpp(
            909,
            GroupDbRowLikeCpp {
                leader_guid_low: 42,
                loot_method: LOOT_METHOD_PERSONAL_LIKE_CPP,
                looter_guid_low: 42,
                loot_threshold: ITEM_QUALITY_UNCOMMON_LIKE_CPP,
                target_icons: [EMPTY_TARGET_ICON_RAW_LIKE_CPP; TARGET_ICONS_COUNT_LIKE_CPP],
                group_flags: GROUP_FLAG_LFG_LIKE_CPP,
                dungeon_difficulty_id: DIFFICULTY_NORMAL_LIKE_CPP,
                raid_difficulty_id: DIFFICULTY_NORMAL_RAID_LIKE_CPP,
                legacy_raid_difficulty_id: DIFFICULTY_10_N_LIKE_CPP,
                master_looter_guid_low: 0,
                db_store_id: 26,
                lfg_dungeon_id: Some(124),
                lfg_state: Some(2),
            },
            Some(GroupMemberCharacterLikeCpp {
                name: "Leader".to_string(),
                race: 1,
                class: 1,
            }),
            &difficulty_store,
        )
        .expect("valid LFG group row should hydrate");

        assert_eq!(
            group.lfg_db_state,
            Some(GroupLfgDbStateLikeCpp {
                dungeon_id: 124,
                state: None,
            })
        );
    }

    #[test]
    fn load_group_from_db_row_ignores_lfg_columns_when_group_is_not_lfg_like_cpp() {
        let difficulty_store = DifficultyStore::from_entries([]);
        let group = GroupInfo::load_group_from_db_row_validated_like_cpp(
            910,
            GroupDbRowLikeCpp {
                leader_guid_low: 42,
                loot_method: LOOT_METHOD_PERSONAL_LIKE_CPP,
                looter_guid_low: 42,
                loot_threshold: ITEM_QUALITY_UNCOMMON_LIKE_CPP,
                target_icons: [EMPTY_TARGET_ICON_RAW_LIKE_CPP; TARGET_ICONS_COUNT_LIKE_CPP],
                group_flags: 0,
                dungeon_difficulty_id: DIFFICULTY_NORMAL_LIKE_CPP,
                raid_difficulty_id: DIFFICULTY_NORMAL_RAID_LIKE_CPP,
                legacy_raid_difficulty_id: DIFFICULTY_10_N_LIKE_CPP,
                master_looter_guid_low: 0,
                db_store_id: 27,
                lfg_dungeon_id: Some(125),
                lfg_state: Some(LFG_STATE_FINISHED_DUNGEON_LIKE_CPP),
            },
            Some(GroupMemberCharacterLikeCpp {
                name: "Leader".to_string(),
                race: 1,
                class: 1,
            }),
            &difficulty_store,
        )
        .expect("valid non-LFG group row should hydrate");

        assert_eq!(group.lfg_db_state, None);
    }

    #[test]
    fn set_group_member_flag_maintank_is_unique_and_preserves_assistant_bit_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let old_tank = ObjectGuid::create_player(1, 43);
        let new_tank = ObjectGuid::create_player(1, 44);
        let mut group = GroupInfo::new(leader);
        group.add_member(old_tank);
        group.add_member(new_tank);
        group.convert_to_raid_like_cpp();
        group
            .set_group_member_flag_like_cpp(old_tank, true, MEMBER_FLAG_MAINTANK_LIKE_CPP)
            .unwrap();
        group
            .set_group_member_flag_like_cpp(new_tank, true, MEMBER_FLAG_ASSISTANT_LIKE_CPP)
            .unwrap();
        let sequence_before = group.sequence_num;

        let updates = group
            .set_group_member_flag_updates_like_cpp(new_tank, true, MEMBER_FLAG_MAINTANK_LIKE_CPP)
            .unwrap();

        assert_eq!(updates.len(), 1);
        assert!(!updates.iter().any(|(guid, _)| *guid == old_tank));
        assert_eq!(
            updates,
            vec![(
                new_tank,
                MEMBER_FLAG_ASSISTANT_LIKE_CPP | MEMBER_FLAG_MAINTANK_LIKE_CPP
            )]
        );
        assert_eq!(
            group.member_slot_like_cpp(old_tank).unwrap().flags & MEMBER_FLAG_MAINTANK_LIKE_CPP,
            0
        );
        assert_eq!(
            group.member_slot_like_cpp(new_tank).unwrap().flags,
            MEMBER_FLAG_ASSISTANT_LIKE_CPP | MEMBER_FLAG_MAINTANK_LIKE_CPP
        );
        assert!(group.sequence_num > sequence_before);
    }

    #[test]
    fn remove_unique_group_member_flag_clears_only_live_state_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let old_assist = ObjectGuid::create_player(1, 43);
        let other = ObjectGuid::create_player(1, 44);
        let mut group = GroupInfo::new(leader);
        group.add_member(old_assist);
        group.add_member(other);
        group.convert_to_raid_like_cpp();
        group
            .set_group_member_flag_like_cpp(old_assist, true, MEMBER_FLAG_MAINASSIST_LIKE_CPP)
            .unwrap();
        group
            .set_group_member_flag_like_cpp(other, true, MEMBER_FLAG_ASSISTANT_LIKE_CPP)
            .unwrap();
        let sequence_before = group.sequence_num;

        assert!(group.remove_unique_group_member_flag_like_cpp(MEMBER_FLAG_MAINASSIST_LIKE_CPP));

        assert_eq!(
            group.member_slot_like_cpp(old_assist).unwrap().flags & MEMBER_FLAG_MAINASSIST_LIKE_CPP,
            0
        );
        assert_eq!(
            group.member_slot_like_cpp(other).unwrap().flags,
            MEMBER_FLAG_ASSISTANT_LIKE_CPP
        );
        assert!(group.sequence_num > sequence_before);
        assert!(!group.remove_unique_group_member_flag_like_cpp(MEMBER_FLAG_ASSISTANT_LIKE_CPP));
    }

    #[test]
    fn set_group_member_flag_rejects_non_raid_and_missing_target_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 43);
        let missing = ObjectGuid::create_player(1, 44);
        let mut group = GroupInfo::new(leader);
        group.add_member(member);

        assert_eq!(
            group.set_group_member_flag_updates_like_cpp(
                member,
                true,
                MEMBER_FLAG_MAINASSIST_LIKE_CPP
            ),
            None
        );
        group.convert_to_raid_like_cpp();
        assert_eq!(
            group.set_group_member_flag_updates_like_cpp(
                missing,
                true,
                MEMBER_FLAG_MAINASSIST_LIKE_CPP
            ),
            None
        );
        assert_eq!(group.member_slot_like_cpp(member).unwrap().flags, 0);
    }

    #[test]
    fn ready_check_start_marks_offline_starter_and_preserves_cpp_event_order() {
        let leader = ObjectGuid::create_player(1, 42);
        let offline = ObjectGuid::create_player(1, 43);
        let mut group = GroupInfo::new(leader);
        group.add_member(offline);

        let events = group.start_ready_check_like_cpp(leader, [leader]);

        assert_eq!(group.ready_check_timer_ms, 0);
        assert!(!group.ready_check_started);
        assert!(group.member_slots.iter().all(|slot| !slot.ready_checked));
        assert_eq!(
            events,
            vec![
                ReadyCheckEventLikeCpp::Response {
                    party_guid: group.group_guid,
                    player: offline,
                    is_ready: false,
                },
                ReadyCheckEventLikeCpp::Completed {
                    party_index: 0,
                    party_guid: group.group_guid,
                },
                ReadyCheckEventLikeCpp::Started {
                    party_index: 0,
                    party_guid: group.group_guid,
                    initiator_guid: leader,
                    duration_ms: READYCHECK_DURATION_MS_LIKE_CPP,
                },
            ]
        );
    }

    #[test]
    fn ready_check_response_before_started_is_cpp_noop() {
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 43);
        let mut group = GroupInfo::new(leader);
        group.add_member(member);

        let events = group.set_member_ready_check_like_cpp(member, true);

        assert!(events.is_empty());
        assert!(!group.member_slot_like_cpp(member).unwrap().ready_checked);
        assert!(!group.ready_check_started);
    }

    #[test]
    fn ready_check_member_response_broadcasts_and_completes_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 43);
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        let start_events = group.start_ready_check_like_cpp(leader, [leader, member]);

        assert_eq!(start_events.len(), 1);
        assert!(group.ready_check_started);
        assert!(group.member_slot_like_cpp(leader).unwrap().ready_checked);
        assert!(!group.member_slot_like_cpp(member).unwrap().ready_checked);

        let events = group.set_member_ready_check_like_cpp(member, true);

        assert_eq!(
            events,
            vec![
                ReadyCheckEventLikeCpp::Response {
                    party_guid: group.group_guid,
                    player: member,
                    is_ready: true,
                },
                ReadyCheckEventLikeCpp::Completed {
                    party_index: 0,
                    party_guid: group.group_guid,
                },
            ]
        );
        assert!(!group.ready_check_started);
        assert_eq!(group.ready_check_timer_ms, 0);
        assert!(group.member_slots.iter().all(|slot| !slot.ready_checked));
    }

    #[test]
    fn load_groups_from_db_rows_registers_groups_and_members_like_cpp() {
        let registry = GroupRegistry::default();
        let difficulty_store = DifficultyStore::from_entries([]);
        let mut character_cache = BTreeMap::new();
        character_cache.insert(
            5001,
            GroupMemberCharacterLikeCpp {
                name: "Leader".to_string(),
                race: 1,
                class: 2,
            },
        );
        character_cache.insert(
            5002,
            GroupMemberCharacterLikeCpp {
                name: "Member".to_string(),
                race: 3,
                class: 4,
            },
        );

        let summary = load_groups_from_db_rows_like_cpp(
            &registry,
            [GroupDbRowLikeCpp {
                leader_guid_low: 5001,
                loot_method: 3,
                looter_guid_low: 5001,
                loot_threshold: 4,
                target_icons: [EMPTY_TARGET_ICON_RAW_LIKE_CPP; TARGET_ICONS_COUNT_LIKE_CPP],
                group_flags: GROUP_FLAG_RAID_LIKE_CPP,
                dungeon_difficulty_id: DIFFICULTY_NORMAL_LIKE_CPP,
                raid_difficulty_id: DIFFICULTY_NORMAL_RAID_LIKE_CPP,
                legacy_raid_difficulty_id: DIFFICULTY_10_N_LIKE_CPP,
                master_looter_guid_low: 0,
                db_store_id: 5501,
                lfg_dungeon_id: None,
                lfg_state: None,
            }],
            [
                GroupMemberDbRowLikeCpp {
                    db_store_id: 5501,
                    member_guid_low: 5001,
                    member_flags: 0,
                    subgroup: 0,
                    roles: 1,
                },
                GroupMemberDbRowLikeCpp {
                    db_store_id: 5501,
                    member_guid_low: 5002,
                    member_flags: 0x04,
                    subgroup: 2,
                    roles: 3,
                },
            ],
            &character_cache,
            &difficulty_store,
        );

        assert_eq!(
            summary,
            GroupLoadSummaryLikeCpp {
                loaded_groups: 1,
                loaded_member_rows: 2,
                loaded_members: 2,
                skipped_group_rows: 0,
                skipped_member_rows: 0,
            }
        );

        let group = get_group_by_db_store_id_like_cpp(&registry, 5501)
            .expect("loaded group should be registered by DB-store id");
        assert_eq!(group.db_store_id, 5501);
        assert_eq!(group.members.len(), 2);
        let slot = group
            .member_slot_like_cpp(ObjectGuid::create_player(1, 5002))
            .expect("loaded member row should preserve its slot");
        assert_eq!(slot.name, "Member");
        assert_eq!(slot.subgroup, 2);
        assert_eq!(slot.flags, 0x04);
        assert_eq!(slot.roles, 3);
    }

    #[test]
    fn load_groups_from_db_rows_skips_missing_character_cache_rows_like_cpp_boundary() {
        let registry = GroupRegistry::default();
        let difficulty_store = DifficultyStore::from_entries([]);
        let mut character_cache = BTreeMap::new();
        character_cache.insert(
            5101,
            GroupMemberCharacterLikeCpp {
                name: "Leader".to_string(),
                race: 1,
                class: 1,
            },
        );

        let summary = load_groups_from_db_rows_like_cpp(
            &registry,
            [
                GroupDbRowLikeCpp {
                    leader_guid_low: 5101,
                    loot_method: LOOT_METHOD_PERSONAL_LIKE_CPP,
                    looter_guid_low: 5101,
                    loot_threshold: ITEM_QUALITY_UNCOMMON_LIKE_CPP,
                    target_icons: [EMPTY_TARGET_ICON_RAW_LIKE_CPP; TARGET_ICONS_COUNT_LIKE_CPP],
                    group_flags: 0,
                    dungeon_difficulty_id: DIFFICULTY_NORMAL_LIKE_CPP,
                    raid_difficulty_id: DIFFICULTY_NORMAL_RAID_LIKE_CPP,
                    legacy_raid_difficulty_id: DIFFICULTY_10_N_LIKE_CPP,
                    master_looter_guid_low: 0,
                    db_store_id: 5601,
                    lfg_dungeon_id: None,
                    lfg_state: None,
                },
                GroupDbRowLikeCpp {
                    leader_guid_low: 999_999,
                    loot_method: LOOT_METHOD_PERSONAL_LIKE_CPP,
                    looter_guid_low: 999_999,
                    loot_threshold: ITEM_QUALITY_UNCOMMON_LIKE_CPP,
                    target_icons: [EMPTY_TARGET_ICON_RAW_LIKE_CPP; TARGET_ICONS_COUNT_LIKE_CPP],
                    group_flags: 0,
                    dungeon_difficulty_id: DIFFICULTY_NORMAL_LIKE_CPP,
                    raid_difficulty_id: DIFFICULTY_NORMAL_RAID_LIKE_CPP,
                    legacy_raid_difficulty_id: DIFFICULTY_10_N_LIKE_CPP,
                    master_looter_guid_low: 0,
                    db_store_id: 5602,
                    lfg_dungeon_id: None,
                    lfg_state: None,
                },
            ],
            [
                GroupMemberDbRowLikeCpp {
                    db_store_id: 5601,
                    member_guid_low: 5102,
                    member_flags: 0,
                    subgroup: 0,
                    roles: 0,
                },
                GroupMemberDbRowLikeCpp {
                    db_store_id: 888_888,
                    member_guid_low: 5101,
                    member_flags: 0,
                    subgroup: 0,
                    roles: 0,
                },
            ],
            &character_cache,
            &difficulty_store,
        );

        assert_eq!(summary.loaded_groups, 1);
        assert_eq!(summary.skipped_group_rows, 1);
        assert_eq!(summary.loaded_member_rows, 2);
        assert_eq!(summary.loaded_members, 0);
        assert_eq!(summary.skipped_member_rows, 2);
        assert!(get_group_by_db_store_id_like_cpp(&registry, 5601).is_some());
        assert!(get_group_by_db_store_id_like_cpp(&registry, 5602).is_none());
    }

    #[test]
    fn load_groups_from_db_rows_advances_next_storage_id_for_ordered_rows_like_cpp() {
        let registry = GroupRegistry::default();
        let difficulty_store = DifficultyStore::from_entries([]);
        let mut character_cache = BTreeMap::new();
        for guid_low in [900_001, 900_002] {
            character_cache.insert(
                guid_low,
                GroupMemberCharacterLikeCpp {
                    name: format!("Leader{guid_low}"),
                    race: 1,
                    class: 1,
                },
            );
        }

        let _allocator_guard = GROUP_DB_STORE_ID_ALLOCATOR_LOCK.lock().unwrap();
        NEXT_GROUP_DB_STORE_ID.store(900_001, Ordering::Relaxed);
        let summary = load_groups_from_db_rows_like_cpp(
            &registry,
            [
                GroupDbRowLikeCpp {
                    leader_guid_low: 900_001,
                    loot_method: LOOT_METHOD_PERSONAL_LIKE_CPP,
                    looter_guid_low: 900_001,
                    loot_threshold: ITEM_QUALITY_UNCOMMON_LIKE_CPP,
                    target_icons: [EMPTY_TARGET_ICON_RAW_LIKE_CPP; TARGET_ICONS_COUNT_LIKE_CPP],
                    group_flags: 0,
                    dungeon_difficulty_id: DIFFICULTY_NORMAL_LIKE_CPP,
                    raid_difficulty_id: DIFFICULTY_NORMAL_RAID_LIKE_CPP,
                    legacy_raid_difficulty_id: DIFFICULTY_10_N_LIKE_CPP,
                    master_looter_guid_low: 0,
                    db_store_id: 900_001,
                    lfg_dungeon_id: None,
                    lfg_state: None,
                },
                GroupDbRowLikeCpp {
                    leader_guid_low: 900_002,
                    loot_method: LOOT_METHOD_PERSONAL_LIKE_CPP,
                    looter_guid_low: 900_002,
                    loot_threshold: ITEM_QUALITY_UNCOMMON_LIKE_CPP,
                    target_icons: [EMPTY_TARGET_ICON_RAW_LIKE_CPP; TARGET_ICONS_COUNT_LIKE_CPP],
                    group_flags: 0,
                    dungeon_difficulty_id: DIFFICULTY_NORMAL_LIKE_CPP,
                    raid_difficulty_id: DIFFICULTY_NORMAL_RAID_LIKE_CPP,
                    legacy_raid_difficulty_id: DIFFICULTY_10_N_LIKE_CPP,
                    master_looter_guid_low: 0,
                    db_store_id: 900_002,
                    lfg_dungeon_id: None,
                    lfg_state: None,
                },
            ],
            [],
            &character_cache,
            &difficulty_store,
        );

        assert_eq!(summary.loaded_groups, 2);
        assert_eq!(NEXT_GROUP_DB_STORE_ID.load(Ordering::Relaxed), 900_003);
    }

    // ── Ready-check tick tests ──────────────────────────────────────────

    #[test]
    fn update_ready_check_noop_when_not_started() {
        let leader = ObjectGuid::create_player(1, 42);
        let mut group = GroupInfo::new(leader);
        assert!(!group.ready_check_started);

        let events = group.update_ready_check_like_cpp(500);
        assert!(events.is_empty());
        assert!(!group.ready_check_started);
        assert_eq!(group.ready_check_timer_ms, 0);
    }

    #[test]
    fn update_ready_check_decrements_without_completing_when_time_remains() {
        let leader = ObjectGuid::create_player(1, 42);
        let mut group = GroupInfo::new(leader);
        group.ready_check_started = true;
        group.ready_check_timer_ms = READYCHECK_DURATION_MS_LIKE_CPP;

        // Tick 1000ms — timer should go from 35000 to 34000, no events.
        let events = group.update_ready_check_like_cpp(1_000);
        assert!(events.is_empty());
        assert!(group.ready_check_started);
        assert_eq!(
            group.ready_check_timer_ms,
            READYCHECK_DURATION_MS_LIKE_CPP - 1_000
        );

        // Tick another 1000ms
        let events = group.update_ready_check_like_cpp(1_000);
        assert!(events.is_empty());
        assert!(group.ready_check_started);
        assert_eq!(
            group.ready_check_timer_ms,
            READYCHECK_DURATION_MS_LIKE_CPP - 2_000
        );
    }

    #[test]
    fn update_ready_check_expires_and_resets_all_state() {
        let leader = ObjectGuid::create_player(1, 42);
        let member = ObjectGuid::create_player(1, 99);
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        group.ready_check_started = true;
        group.ready_check_timer_ms = READYCHECK_DURATION_MS_LIKE_CPP;
        // Simulate some members already responded.
        for slot in &mut group.member_slots {
            slot.ready_checked = true;
        }

        // Tick more than remaining — should expire.
        let events = group.update_ready_check_like_cpp(36_000);
        assert_eq!(events.len(), 1);
        match events[0] {
            ReadyCheckEventLikeCpp::Completed {
                party_index,
                party_guid,
            } => {
                assert_eq!(party_index, 0);
                assert_eq!(party_guid, group.group_guid);
            }
            _ => panic!("expected Completed event"),
        }
        assert!(!group.ready_check_started);
        assert_eq!(group.ready_check_timer_ms, 0);
        // All members should have been reset.
        assert!(group.member_slots.iter().all(|s| !s.ready_checked));
    }

    #[test]
    fn update_ready_check_exact_zero_expires() {
        let leader = ObjectGuid::create_player(1, 42);
        let mut group = GroupInfo::new(leader);
        group.ready_check_started = true;
        group.ready_check_timer_ms = 500;

        let events = group.update_ready_check_like_cpp(500);
        assert_eq!(events.len(), 1);
        assert!(!group.ready_check_started);
        assert_eq!(group.ready_check_timer_ms, 0);
    }

    #[test]
    fn registry_invalid_subgroup_transition_has_no_partial_state() {
        let registry = GroupRegistry::new();
        let leader = ObjectGuid::create_player(1, 71);
        let member = ObjectGuid::create_player(1, 72);
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        group.convert_to_raid_like_cpp();
        let group_guid = group.group_guid;
        let sequence = group.sequence_num;
        registry.insert(group_guid, group);

        let result = registry.change_member_subgroup_like_cpp(
            group_guid,
            leader,
            member,
            MAX_RAID_SUBGROUPS_LIKE_CPP as u8,
        );

        assert!(matches!(
            result,
            Err(GroupAuthorityErrorLikeCpp::InvalidSubgroup)
        ));
        let group = registry.get(&group_guid).expect("group remains registered");
        assert_eq!(group.sequence_num, sequence);
        assert_eq!(group.member_slot_like_cpp(member).unwrap().subgroup, 0);
    }

    #[test]
    fn registry_missing_member_flag_transition_has_no_partial_state() {
        let registry = GroupRegistry::new();
        let leader = ObjectGuid::create_player(1, 81);
        let missing = ObjectGuid::create_player(1, 82);
        let mut group = GroupInfo::new(leader);
        group.convert_to_raid_like_cpp();
        let group_guid = group.group_guid;
        let sequence = group.sequence_num;
        registry.insert(group_guid, group);

        let result = registry.set_member_flag_transition_like_cpp(
            group_guid,
            leader,
            missing,
            true,
            MEMBER_FLAG_ASSISTANT_LIKE_CPP,
        );

        assert!(matches!(
            result,
            Err(GroupAuthorityErrorLikeCpp::MissingMember)
        ));
        assert_eq!(registry.get(&group_guid).unwrap().sequence_num, sequence);
    }

    #[test]
    fn registry_stale_ready_response_cannot_reopen_completed_check() {
        let registry = GroupRegistry::new();
        let leader = ObjectGuid::create_player(1, 91);
        let member = ObjectGuid::create_player(1, 92);
        let mut group = GroupInfo::new(leader);
        group.add_member(member);
        let group_guid = group.group_guid;
        registry.insert(group_guid, group);

        registry
            .start_ready_check_transition_like_cpp(group_guid, leader, [leader, member])
            .expect("leader starts ready check");
        registry
            .respond_ready_check_transition_like_cpp(group_guid, member, true)
            .expect("final member completes ready check");
        let stale = registry.respond_ready_check_transition_like_cpp(group_guid, member, false);

        assert!(matches!(stale, Err(GroupAuthorityErrorLikeCpp::NoChange)));
        let group = registry.get(&group_guid).unwrap();
        assert!(!group.ready_check_started);
        assert_eq!(group.ready_check_timer_ms, 0);
        assert!(group.member_slots.iter().all(|slot| !slot.ready_checked));
    }

    #[test]
    fn concurrent_leader_transfers_allow_only_current_leader_once() {
        let registry = std::sync::Arc::new(GroupRegistry::new());
        let leader = ObjectGuid::create_player(1, 101);
        let first = ObjectGuid::create_player(1, 102);
        let second = ObjectGuid::create_player(1, 103);
        let mut group = GroupInfo::new(leader);
        group.add_member(first);
        group.add_member(second);
        let group_guid = group.group_guid;
        registry.insert(group_guid, group);
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));

        let handles = [first, second].map(|candidate| {
            let registry = std::sync::Arc::clone(&registry);
            let barrier = std::sync::Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                registry.change_leader_transition_like_cpp(group_guid, leader, candidate)
            })
        });
        barrier.wait();
        let results = handles.map(|handle| handle.join().unwrap());

        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|result| matches!(result, Err(GroupAuthorityErrorLikeCpp::NotLeader)))
                .count(),
            1
        );
        assert!([first, second].contains(&registry.get(&group_guid).unwrap().leader_guid));
    }

    #[test]
    fn concurrent_kicks_remove_each_member_once_and_disband_once() {
        let registry = std::sync::Arc::new(GroupRegistry::new());
        let leader = ObjectGuid::create_player(1, 111);
        let first = ObjectGuid::create_player(1, 112);
        let second = ObjectGuid::create_player(1, 113);
        let mut group = GroupInfo::new(leader);
        group.add_member(first);
        group.add_member(second);
        let group_guid = group.group_guid;
        registry.insert(group_guid, group);
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));

        let handles = [first, second].map(|target| {
            let registry = std::sync::Arc::clone(&registry);
            let barrier = std::sync::Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                registry.remove_member_like_cpp(
                    group_guid,
                    target,
                    GroupMemberRemovalKindLikeCpp::Kick {
                        actor_guid: leader,
                        actor_in_battleground: false,
                        target_has_loot_rolls: false,
                        any_member_in_actor_map_combat: false,
                    },
                    &[],
                )
            })
        });
        barrier.wait();
        let results = handles.map(|handle| handle.join().unwrap().unwrap());

        assert_eq!(
            results
                .iter()
                .filter(|outcome| outcome.facts.disbanded)
                .count(),
            1
        );
        assert!(!registry.contains_key(&group_guid));
    }

    #[test]
    fn instance_transition_outcomes_are_owned_and_do_not_hold_group_guard() {
        let registry = GroupRegistry::new();
        let leader = ObjectGuid::create_player(1, 121);
        let group = GroupInfo::new(leader);
        let group_guid = group.group_guid;
        registry.insert(group_guid, group);

        let recent = registry
            .set_recent_instance_transition_like_cpp(group_guid, 631, leader, 9001)
            .unwrap();
        let linked = registry
            .link_owned_instance_transition_like_cpp(group_guid, 631, 9001)
            .unwrap();
        let reset = registry
            .apply_instance_reset_transition_like_cpp(
                group_guid,
                631,
                GroupInstanceResetResultLikeCpp::Success,
                GroupInstanceResetMethodLikeCpp::Manual,
            )
            .unwrap();

        assert_eq!(recent.group.recent_instance_id_like_cpp(631), 9001);
        assert!(linked.facts);
        assert!(reset.facts);
        assert_eq!(reset.group.recent_instance_id_like_cpp(631), 0);
        assert!(
            reset
                .group
                .owned_instances_like_cpp()
                .any(|instance| instance.instance_id == 9001)
        );
    }
}
