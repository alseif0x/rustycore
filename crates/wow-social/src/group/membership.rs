// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Group membership: join, leave, roles, member flags and subgroups.

use dashmap::mapref::entry::Entry;
use wow_core::ObjectGuid;

use super::*;

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

pub const MAX_GROUP_SIZE_LIKE_CPP: usize = 5;

pub const MAX_RAID_SIZE_LIKE_CPP: usize = 40;

pub const MAX_RAID_SUBGROUPS_LIKE_CPP: usize = MAX_RAID_SIZE_LIKE_CPP / MAX_GROUP_SIZE_LIKE_CPP;

pub const MISSING_MEMBER_GROUP_LIKE_CPP: u8 = (MAX_RAID_SUBGROUPS_LIKE_CPP as u8) + 1;

impl GroupInfo {
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
}

impl GroupRegistry {
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
        let persistence = if disbanded {
            vec![
                GroupPersistenceIntentLikeCpp::DeleteGroup { db_store_id },
                GroupPersistenceIntentLikeCpp::DeleteAllMembers { db_store_id },
                GroupPersistenceIntentLikeCpp::DeleteLfgData { db_store_id },
            ]
        } else {
            let mut intents = vec![GroupPersistenceIntentLikeCpp::DeleteMember { member_guid }];
            if let Some(leader_guid) = new_leader_guid {
                intents.push(GroupPersistenceIntentLikeCpp::UpdateLeader {
                    db_store_id,
                    leader_guid,
                });
            }
            intents
        };
        if disbanded {
            let group = entry.remove();
            free_group_db_store_id_like_cpp(group.db_store_id);
            let remaining_members = group.members.clone();
            return Ok(GroupTransitionOutcomeLikeCpp {
                persistence,
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
            persistence,
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
            persistence: vec![GroupPersistenceIntentLikeCpp::UpdateGroupType {
                group_flags: facts.0,
                db_store_id: facts.1,
            }],
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
            persistence: vec![GroupPersistenceIntentLikeCpp::UpdateMemberSubgroup {
                member_guid,
                subgroup,
            }],
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
        let persistence = facts
            .iter()
            .map(
                |&(member_guid, subgroup)| GroupPersistenceIntentLikeCpp::UpdateMemberSubgroup {
                    member_guid,
                    subgroup,
                },
            )
            .collect();
        Ok(GroupTransitionOutcomeLikeCpp {
            persistence,
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
        let mut persistence = Vec::with_capacity(2);
        if db_store_id != 0 {
            persistence.push(GroupPersistenceIntentLikeCpp::UpdateLeader {
                db_store_id,
                leader_guid: new_leader_guid,
            });
        }
        persistence.push(GroupPersistenceIntentLikeCpp::UpdateMemberFlags {
            member_guid: new_leader_guid,
            flags: final_flags,
        });
        Ok(GroupTransitionOutcomeLikeCpp {
            persistence,
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
            persistence: vec![GroupPersistenceIntentLikeCpp::UpdateMemberFlags {
                member_guid,
                flags: facts,
            }],
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
            persistence: vec![GroupPersistenceIntentLikeCpp::UpdateGroupType {
                group_flags: facts.0,
                db_store_id: facts.1,
            }],
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
                    persistence: Vec::new(),
                    group: group.clone(),
                    facts: Vec::new(),
                });
            }
        };
        group.remove_unique_group_member_flag_like_cpp(flag);
        let facts = group
            .set_group_member_flag_updates_like_cpp(member_guid, apply, flag)
            .unwrap_or_default();
        let persistence = facts
            .iter()
            .map(
                |&(member_guid, flags)| GroupPersistenceIntentLikeCpp::UpdateMemberFlags {
                    member_guid,
                    flags,
                },
            )
            .collect();
        Ok(GroupTransitionOutcomeLikeCpp {
            persistence,
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
            persistence: Vec::new(),
            group: group.clone(),
            facts: (old_role, mutated),
        })
    }
}
