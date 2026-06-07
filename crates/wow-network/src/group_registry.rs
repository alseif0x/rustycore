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
pub const LOOT_METHOD_PERSONAL_LIKE_CPP: u8 = 5;
pub const ITEM_QUALITY_UNCOMMON_LIKE_CPP: u8 = 2;
pub const DIFFICULTY_NORMAL_LIKE_CPP: u32 = 1;
pub const DIFFICULTY_NORMAL_RAID_LIKE_CPP: u32 = 14;
pub const DIFFICULTY_10_N_LIKE_CPP: u32 = 3;
pub const TARGET_ICONS_COUNT_LIKE_CPP: usize = 8;
pub const EMPTY_TARGET_ICON_RAW_LIKE_CPP: [u8; 16] = [0; 16];
pub const LFG_STATE_DUNGEON_LIKE_CPP: u8 = 5;
pub const LFG_STATE_FINISHED_DUNGEON_LIKE_CPP: u8 = 6;

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
        let guid = ObjectGuid::create_player(1, guid_db_id);
        self.members.retain(|member_guid| *member_guid != guid);
        self.member_slots.retain(|slot| slot.guid != guid);
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

    pub fn is_raid_group(&self) -> bool {
        (self.group_flags & GROUP_FLAG_RAID_LIKE_CPP) != 0
    }

    pub fn convert_to_raid_like_cpp(&mut self) {
        if !self.is_raid_group() {
            self.group_flags |= GROUP_FLAG_RAID_LIKE_CPP;
            self.sequence_num += 1;
        }
    }

    pub fn convert_to_group_like_cpp(&mut self) -> bool {
        if self.members.len() > 5 {
            return false;
        }
        if self.is_raid_group() {
            self.group_flags &= !GROUP_FLAG_RAID_LIKE_CPP;
            self.sequence_num += 1;
        }
        true
    }

    pub fn remove_member(&mut self, guid: &ObjectGuid) {
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
