//! Shared registry of active groups for cross-session party management.

use std::sync::atomic::{AtomicU64, Ordering};
use dashmap::DashMap;
use wow_core::ObjectGuid;

static NEXT_GROUP_ID: AtomicU64 = AtomicU64::new(1);

/// Information about one group/party.
#[derive(Debug, Clone)]
pub struct GroupInfo {
    pub group_guid: u64,
    pub leader_guid: ObjectGuid,
    /// All member GUIDs (including leader), in join order.
    pub members: Vec<ObjectGuid>,
    /// 0=FreeForAll, 1=RoundRobin, 2=MasterLoot, 3=GroupLoot, 4=NeedBeforeGreed
    pub loot_method: u8,
    pub sequence_num: u32,
}

impl GroupInfo {
    pub fn new(leader: ObjectGuid) -> Self {
        Self {
            group_guid: NEXT_GROUP_ID.fetch_add(1, Ordering::Relaxed),
            leader_guid: leader,
            members: vec![leader],
            loot_method: 0,
            sequence_num: 1,
        }
    }

    pub fn add_member(&mut self, guid: ObjectGuid) {
        if !self.members.contains(&guid) {
            self.members.push(guid);
            self.sequence_num += 1;
        }
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
