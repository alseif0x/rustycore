//! Shared registry of active groups for cross-session party management.

use dashmap::DashMap;
use std::sync::{
    Mutex,
    atomic::{AtomicU32, AtomicU64, Ordering},
};
use wow_core::ObjectGuid;
use wow_data::DifficultyStore;

static NEXT_GROUP_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_GROUP_DB_STORE_ID: AtomicU32 = AtomicU32::new(1);
static FREED_GROUP_DB_STORE_IDS: Mutex<Vec<u32>> = Mutex::new(Vec::new());
static GROUP_DB_STORE: Mutex<Vec<Option<u64>>> = Mutex::new(Vec::new());

pub const GROUP_FLAG_RAID_LIKE_CPP: u16 = 0x002;
pub const LOOT_METHOD_PERSONAL_LIKE_CPP: u8 = 5;
pub const ITEM_QUALITY_UNCOMMON_LIKE_CPP: u8 = 2;
pub const DIFFICULTY_NORMAL_LIKE_CPP: u32 = 1;
pub const DIFFICULTY_NORMAL_RAID_LIKE_CPP: u32 = 14;
pub const DIFFICULTY_10_N_LIKE_CPP: u32 = 3;

fn generate_group_db_store_id_like_cpp() -> u32 {
    if let Ok(mut freed) = FREED_GROUP_DB_STORE_IDS.lock() {
        if let Some((index, _)) = freed.iter().enumerate().min_by_key(|(_, id)| *id) {
            return freed.swap_remove(index);
        }
    }

    NEXT_GROUP_DB_STORE_ID.fetch_add(1, Ordering::Relaxed)
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
    /// 0=FreeForAll, 1=RoundRobin, 2=MasterLoot, 3=GroupLoot, 4=NeedBeforeGreed
    pub loot_method: u8,
    pub looter_guid: ObjectGuid,
    pub loot_threshold: u8,
    pub master_looter_guid: ObjectGuid,
    pub dungeon_difficulty_id: u32,
    pub raid_difficulty_id: u32,
    pub legacy_raid_difficulty_id: u32,
    pub sequence_num: u32,
    pub group_flags: u16,
}

impl GroupInfo {
    pub fn new(leader: ObjectGuid) -> Self {
        Self {
            group_guid: NEXT_GROUP_ID.fetch_add(1, Ordering::Relaxed),
            db_store_id: generate_group_db_store_id_like_cpp(),
            leader_guid: leader,
            members: vec![leader],
            loot_method: LOOT_METHOD_PERSONAL_LIKE_CPP,
            looter_guid: leader,
            loot_threshold: ITEM_QUALITY_UNCOMMON_LIKE_CPP,
            master_looter_guid: ObjectGuid::EMPTY,
            dungeon_difficulty_id: DIFFICULTY_NORMAL_LIKE_CPP,
            raid_difficulty_id: DIFFICULTY_NORMAL_RAID_LIKE_CPP,
            legacy_raid_difficulty_id: DIFFICULTY_10_N_LIKE_CPP,
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
            loot_method,
            looter_guid,
            loot_threshold,
            master_looter_guid,
            dungeon_difficulty_id,
            raid_difficulty_id,
            legacy_raid_difficulty_id,
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

    pub fn add_member(&mut self, guid: ObjectGuid) {
        if !self.members.contains(&guid) {
            self.members.push(guid);
            self.sequence_num += 1;
        }
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
}
