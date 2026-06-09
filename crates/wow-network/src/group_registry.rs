//! Shared registry of active groups for cross-session party management.

use dashmap::DashMap;
use std::{
    collections::BTreeMap,
    sync::{
        Mutex,
        atomic::{AtomicU32, AtomicU64, Ordering},
    },
};
use wow_core::ObjectGuid;
use wow_data::DifficultyStore;

static NEXT_GROUP_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_GROUP_DB_STORE_ID: AtomicU32 = AtomicU32::new(1);
static FREED_GROUP_DB_STORE_IDS: Mutex<Vec<u32>> = Mutex::new(Vec::new());
static GROUP_DB_STORE: Mutex<Vec<Option<u64>>> = Mutex::new(Vec::new());

pub const GROUP_FLAG_RAID_LIKE_CPP: u16 = 0x002;
pub const GROUP_FLAG_LFG_LIKE_CPP: u16 = 0x008;
pub const GROUP_FLAG_EVERYONE_ASSISTANT_LIKE_CPP: u16 = 0x040;
pub const MEMBER_FLAG_ASSISTANT_LIKE_CPP: u8 = 0x01;
pub const MEMBER_FLAG_MAINTANK_LIKE_CPP: u8 = 0x02;
pub const MEMBER_FLAG_MAINASSIST_LIKE_CPP: u8 = 0x04;
pub const GROUP_ASSIGN_MAINTANK_LIKE_CPP: u8 = 0;
pub const GROUP_ASSIGN_MAINASSIST_LIKE_CPP: u8 = 1;
pub const LOOT_METHOD_PERSONAL_LIKE_CPP: u8 = 5;
pub const ITEM_QUALITY_UNCOMMON_LIKE_CPP: u8 = 2;
pub const DIFFICULTY_NORMAL_LIKE_CPP: u32 = 1;
pub const DIFFICULTY_NORMAL_RAID_LIKE_CPP: u32 = 14;
pub const DIFFICULTY_10_N_LIKE_CPP: u32 = 3;
pub const TARGET_ICONS_COUNT_LIKE_CPP: usize = 8;
pub const EMPTY_TARGET_ICON_RAW_LIKE_CPP: [u8; 16] = [0; 16];
pub const LFG_STATE_DUNGEON_LIKE_CPP: u8 = 5;
pub const LFG_STATE_FINISHED_DUNGEON_LIKE_CPP: u8 = 6;
pub const MAX_GROUP_SIZE_LIKE_CPP: usize = 5;
pub const MAX_RAID_SIZE_LIKE_CPP: usize = 40;
pub const MAX_RAID_SUBGROUPS_LIKE_CPP: usize = MAX_RAID_SIZE_LIKE_CPP / MAX_GROUP_SIZE_LIKE_CPP;
pub const MISSING_MEMBER_GROUP_LIKE_CPP: u8 = (MAX_RAID_SUBGROUPS_LIKE_CPP as u8) + 1;
pub const READYCHECK_DURATION_MS_LIKE_CPP: i64 = 35_000;

fn generate_group_db_store_id_like_cpp() -> u32 {
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
    /// All member GUIDs (including leader), in join order.
    pub members: Vec<ObjectGuid>,
    /// C++ `Group::m_memberSlots` represented metadata.
    pub member_slots: Vec<GroupMemberSlotLikeCpp>,
    /// 0=FreeForAll, 1=RoundRobin, 2=MasterLoot, 3=GroupLoot, 4=NeedBeforeGreed
    pub loot_method: u8,
    pub looter_guid: ObjectGuid,
    pub loot_threshold: u8,
    pub master_looter_guid: ObjectGuid,
    pub dungeon_difficulty_id: u32,
    pub raid_difficulty_id: u32,
    pub legacy_raid_difficulty_id: u32,
    pub target_icons: [[u8; 16]; TARGET_ICONS_COUNT_LIKE_CPP],
    pub lfg_db_state: Option<GroupLfgDbStateLikeCpp>,
    pub raid_subgroup_counts: Option<[u8; MAX_RAID_SUBGROUPS_LIKE_CPP]>,
    pub ready_check_started: bool,
    /// Represented `Group::m_readyCheckTimer`/duration in milliseconds. Rust does
    /// not yet have a `Group::Update` tick loop for timeout expiry.
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
            lfg_db_state: None,
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
            lfg_db_state: None,
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

    pub fn add_member(&mut self, guid: ObjectGuid) {
        if !self.members.contains(&guid) {
            if !self.subgroup_counter_increase_like_cpp(0) {
                return;
            }
            self.members.push(guid);
            self.member_slots.push(GroupMemberSlotLikeCpp {
                guid,
                name: String::new(),
                race: 0,
                class: 0,
                subgroup: 0,
                flags: 0,
                roles: 0,
                ready_checked: false,
            });
            self.sequence_num += 1;
        }
    }

    pub fn member_slot_like_cpp(&self, guid: ObjectGuid) -> Option<&GroupMemberSlotLikeCpp> {
        self.member_slots.iter().find(|slot| slot.guid == guid)
    }

    pub fn member_group_like_cpp(&self, guid: ObjectGuid) -> u8 {
        self.member_slot_like_cpp(guid)
            .map(|slot| slot.subgroup)
            .unwrap_or(MISSING_MEMBER_GROUP_LIKE_CPP)
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

/// Thread-safe registry of all active groups, keyed by group GUID.
pub type GroupRegistry = DashMap<u64, GroupInfo>;

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

/// Pending invites: invited_guid → inviter_guid.
pub type PendingInvites = DashMap<ObjectGuid, ObjectGuid>;

#[cfg(test)]
mod tests {
    use super::*;

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
    fn loaded_group_row_validates_difficulties_like_cpp() {
        let leader = ObjectGuid::create_player(1, 42);
        let difficulty_store = DifficultyStore::from_entries([
            wow_data::DifficultyEntry {
                id: 2,
                instance_type: 1,
                flags: wow_constants::shared::DifficultyFlags::CAN_SELECT.bits(),
            },
            wow_data::DifficultyEntry {
                id: 15,
                instance_type: 2,
                flags: wow_constants::shared::DifficultyFlags::CAN_SELECT.bits(),
            },
            wow_data::DifficultyEntry {
                id: 3,
                instance_type: 2,
                flags: (wow_constants::shared::DifficultyFlags::CAN_SELECT
                    | wow_constants::shared::DifficultyFlags::LEGACY)
                    .bits(),
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
            group.set_group_member_flag_like_cpp(member, true, 0x02),
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
    fn load_group_from_db_row_preserves_target_icons_and_validates_difficulties_like_cpp() {
        let difficulty_store = DifficultyStore::from_entries([
            wow_data::DifficultyEntry {
                id: 2,
                instance_type: 1,
                flags: wow_constants::shared::DifficultyFlags::CAN_SELECT.bits(),
            },
            wow_data::DifficultyEntry {
                id: 15,
                instance_type: 2,
                flags: wow_constants::shared::DifficultyFlags::CAN_SELECT.bits(),
            },
            wow_data::DifficultyEntry {
                id: 3,
                instance_type: 2,
                flags: (wow_constants::shared::DifficultyFlags::CAN_SELECT
                    | wow_constants::shared::DifficultyFlags::LEGACY)
                    .bits(),
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

        NEXT_GROUP_DB_STORE_ID.store(900_001, Ordering::Relaxed);
        load_groups_from_db_rows_like_cpp(
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

        assert_eq!(NEXT_GROUP_DB_STORE_ID.load(Ordering::Relaxed), 900_003);
    }
}
