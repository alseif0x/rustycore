// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Group invite lifecycle and the `PendingInvites` directory.

use dashmap::DashMap;
use std::sync::{Mutex, MutexGuard};
use wow_core::ObjectGuid;

use super::*;

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

    /// Explicit fixture/materialization boundary. Production invite changes
    /// use the atomic `GroupRegistry` transition methods.
    pub fn seed_invite_like_cpp(
        &self,
        invited_guid: ObjectGuid,
        invite: PendingInviteLikeCpp,
    ) -> Option<PendingInviteLikeCpp> {
        let _transition = self.lock_transition();
        self.invites.insert(invited_guid, invite)
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
    JoinedExisting {
        group: GroupInfo,
        subgroup: u8,
        persistence: Vec<GroupPersistenceIntentLikeCpp>,
    },
    Created {
        group: GroupInfo,
        subgroup: u8,
        persistence: Vec<GroupPersistenceIntentLikeCpp>,
    },
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
                persistence: vec![GroupPersistenceIntentLikeCpp::InsertMember {
                    db_store_id: group.db_store_id,
                    member_guid: invitee_guid,
                    member_flags: 0,
                    subgroup,
                    roles: 0,
                }],
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

        let persistence = vec![
            GroupPersistenceIntentLikeCpp::InsertGroup {
                db_store_id,
                leader_guid: group.leader_guid,
                loot_method: group.loot_method,
                looter_guid: group.looter_guid,
                loot_threshold: group.loot_threshold,
                group_flags: group.group_flags,
                dungeon_difficulty_id: group.dungeon_difficulty_id,
                raid_difficulty_id: group.raid_difficulty_id,
                legacy_raid_difficulty_id: group.legacy_raid_difficulty_id,
                master_looter_guid: group.master_looter_guid,
            },
            GroupPersistenceIntentLikeCpp::InsertMember {
                db_store_id,
                member_guid: group.leader_guid,
                member_flags: 0,
                subgroup: 0,
                roles: 0,
            },
            GroupPersistenceIntentLikeCpp::InsertMember {
                db_store_id,
                member_guid: invitee_guid,
                member_flags: 0,
                subgroup,
                roles: 0,
            },
        ];
        AcceptGroupInviteResultLikeCpp::Created {
            group,
            subgroup,
            persistence,
        }
    }
}
