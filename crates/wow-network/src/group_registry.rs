//! Shared registry of active groups for cross-session party management.

use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use wow_core::ObjectGuid;

static NEXT_GROUP_ID: AtomicU64 = AtomicU64::new(1);

pub const GROUP_FLAG_RAID_LIKE_CPP: u16 = 0x002;
pub const LOOT_METHOD_PERSONAL_LIKE_CPP: u8 = 5;

/// Information about one group/party.
#[derive(Debug, Clone)]
pub struct GroupInfo {
    pub group_guid: u64,
    pub leader_guid: ObjectGuid,
    /// All member GUIDs (including leader), in join order.
    pub members: Vec<ObjectGuid>,
    /// 0=FreeForAll, 1=RoundRobin, 2=MasterLoot, 3=GroupLoot, 4=NeedBeforeGreed
    pub loot_method: u8,
    pub master_looter_guid: ObjectGuid,
    pub sequence_num: u32,
    pub group_flags: u16,
}

impl GroupInfo {
    pub fn new(leader: ObjectGuid) -> Self {
        Self {
            group_guid: NEXT_GROUP_ID.fetch_add(1, Ordering::Relaxed),
            leader_guid: leader,
            members: vec![leader],
            loot_method: LOOT_METHOD_PERSONAL_LIKE_CPP,
            master_looter_guid: ObjectGuid::EMPTY,
            sequence_num: 1,
            group_flags: 0,
        }
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
    }
}
