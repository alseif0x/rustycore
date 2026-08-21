// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Group settings: loot method and looter rotation, difficulty selection,
//! target icons and raid markers.

use wow_core::{ObjectGuid, Position};

use super::*;

pub const LOOT_METHOD_FREE_FOR_ALL_LIKE_CPP: u8 = 0;

pub const LOOT_METHOD_PERSONAL_LIKE_CPP: u8 = 5;

pub const ITEM_QUALITY_UNCOMMON_LIKE_CPP: u8 = 2;

pub const DIFFICULTY_NORMAL_LIKE_CPP: u32 = 1;

pub const DIFFICULTY_NORMAL_RAID_LIKE_CPP: u32 = 14;

pub const DIFFICULTY_10_N_LIKE_CPP: u32 = 3;

pub const TARGET_ICONS_COUNT_LIKE_CPP: usize = 8;

pub const EMPTY_TARGET_ICON_RAW_LIKE_CPP: [u8; 16] = [0; 16];

pub const RAID_MARKERS_COUNT_LIKE_CPP: usize = 8;

/// C++ `RaidMarker` (`Group.h`): one world position plus optional transport.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RaidMarkerLikeCpp {
    pub map_id: u32,
    pub position: Position,
    pub transport_guid: ObjectGuid,
}

impl GroupInfo {
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
}

impl GroupRegistry {
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
            persistence: Vec::new(),
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
            persistence: Vec::new(),
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
            persistence: Vec::new(),
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
            persistence: Vec::new(),
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
            persistence: Vec::new(),
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
            persistence: Vec::new(),
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
            persistence: Vec::new(),
            group: group.clone(),
            facts,
        })
    }

    pub fn set_difficulty_transition_like_cpp(
        &self,
        group_guid: u64,
        actor_guid: ObjectGuid,
        difficulty_id: u32,
        kind: GroupDifficultyKindLikeCpp,
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
            GroupDifficultyKindLikeCpp::Dungeon => {
                group.set_dungeon_difficulty_id_like_cpp(difficulty_id)
            }
            GroupDifficultyKindLikeCpp::Raid => {
                group.set_raid_difficulty_id_like_cpp(difficulty_id)
            }
            GroupDifficultyKindLikeCpp::LegacyRaid => {
                group.set_legacy_raid_difficulty_id_like_cpp(difficulty_id)
            }
        };
        if !changed {
            return Err(GroupAuthorityErrorLikeCpp::NoChange);
        }
        Ok(GroupTransitionOutcomeLikeCpp {
            persistence: vec![GroupPersistenceIntentLikeCpp::UpdateDifficulty {
                db_store_id: group.db_store_id,
                kind,
                difficulty_id,
            }],
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
            persistence: Vec::new(),
            group: group.clone(),
            facts,
        })
    }
}
