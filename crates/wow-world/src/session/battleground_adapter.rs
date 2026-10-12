// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Battleground adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

#[cfg(test)]
use super::RepresentedWargameInviteAcceptanceLikeCpp;
use super::{BattlemasterListStore, DISABLE_TYPE_BATTLEGROUND, WorldSession};

pub(crate) use wow_world_core::session::RepresentedBattlegroundQueueTypeIdLikeCpp;
#[cfg(test)]
pub(crate) use wow_world_core::session::{
    RepresentedBattlefieldListLikeCpp, RepresentedBattlefieldPortLikeCpp,
    RepresentedBattlemasterHelloLikeCpp, RepresentedBattlemasterJoinArenaLikeCpp,
    RepresentedBattlemasterJoinLikeCpp, RepresentedBattlemasterJoinSkirmishLikeCpp,
};

pub(in crate::session) fn arena_skirmish_type_like_cpp(bg_type_id: u32, bracket_id: u32) -> u8 {
    if bg_type_id == 3 || bg_type_id == 5 {
        bg_type_id as u8
    } else if bracket_id == 3 || bracket_id == 5 {
        bracket_id as u8
    } else {
        2
    }
}

impl WorldSession {
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn battlemaster_join_skirmish_like_cpp(
        &mut self,
        battlemaster_lists: &BattlemasterListStore,
        bg_type_id: u32,
        bracket_id: u32,
        as_group: u8,
        is_rated: u8,
    ) -> bool {
        if crate::session::hub_ref(self).player_in_represented_battleground_like_cpp() {
            return false;
        }

        let arena_type = arena_skirmish_type_like_cpp(bg_type_id, bracket_id);
        let Some(entry) = battlemaster_lists.get(wow_data::BATTLEGROUND_AA_LIKE_CPP) else {
            return false;
        };
        if entry.instance_type != wow_data::MAP_ARENA_LIKE_CPP {
            return false;
        }
        if self
            .catalogs
            .disable_mgr
            .as_ref()
            .map(|disable_mgr| {
                disable_mgr.is_disabled_for_like_cpp(
                    DISABLE_TYPE_BATTLEGROUND,
                    wow_data::BATTLEGROUND_AA_LIKE_CPP,
                    None,
                    0,
                    None,
                )
            })
            .unwrap_or(false)
        {
            return false;
        }

        let join_as_group = as_group != 0;
        let group_guid = if join_as_group {
            let (Some(player_guid), Some(group_guid), Some(group_registry)) = (
                self.player_guid(),
                self.resolved_group_guid_like_cpp(),
                self.core.directory.group_registry.as_ref(),
            ) else {
                return false;
            };
            let is_group_leader = group_registry
                .get(&group_guid)
                .map(|group| {
                    group.members.contains(&player_guid) && group.is_leader_like_cpp(player_guid)
                })
                .unwrap_or(false);
            if !is_group_leader {
                return false;
            }
            Some(group_guid)
        } else {
            None
        };

        let queue_type_id = RepresentedBattlegroundQueueTypeIdLikeCpp {
            battlemaster_list_id: wow_data::BATTLEGROUND_AA_LIKE_CPP as u16,
            queue_type: 4,
            rated: false,
            team_size: arena_type,
        };

        // C++ continues with PVPDifficulty lookup, BattlegroundQueue::AddGroup,
        // solo queue-slot checks, Group::CanJoinBattlegroundQueue, status packet
        // fanout and ScheduleQueueUpdate. Rust records the bounded intent after
        // the currently represented gates without pretending that live queueing exists.
        #[cfg(test)]
        self.fixtures
            .battleground
            .represented_battlemaster_join_skirmishes_like_cpp
            .push(RepresentedBattlemasterJoinSkirmishLikeCpp {
                bg_type_id,
                bracket_id,
                as_group: join_as_group,
                is_rated_packet_value: is_rated,
                arena_type,
                group_guid,
                queue_type_id,
            });
        true
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn accept_represented_wargame_invite_like_cpp(&mut self, inviter_name: &str) {
        let (
            Some(player_guid),
            Some(player_group_guid),
            Some(player_registry),
            Some(group_registry),
        ) = (
            self.player_guid(),
            self.resolved_group_guid_like_cpp(),
            self.core.player_registry.as_ref(),
            self.core.directory.group_registry.as_ref(),
        )
        else {
            return;
        };

        let Some(inviter) = player_registry.social_recipient_by_name(inviter_name) else {
            return;
        };
        let inviter_guid = inviter.guid;

        let Some(player_group) = group_registry.get(&player_group_guid) else {
            return;
        };
        if !player_group.members.contains(&player_guid) {
            return;
        }
        let player_group_size = player_group.members.len();
        drop(player_group);

        let Some(inviter_group) = group_registry
            .snapshots()
            .into_iter()
            .find(|group| group.members.contains(&inviter_guid))
        else {
            return;
        };
        let inviter_group_guid = inviter_group.group_guid;
        let inviter_group_size = inviter_group.members.len();

        if player_group_size != inviter_group_size {
            return;
        }

        #[cfg(test)]
        self.fixtures
            .battleground
            .represented_wargame_invite_acceptances_like_cpp
            .push(RepresentedWargameInviteAcceptanceLikeCpp {
                inviter_name: inviter_name.to_string(),
                inviter_guid,
                player_group_guid,
                inviter_group_guid,
                group_size: player_group_size,
            });
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/battleground_adapter/f3_shims.rs"]
mod f3_shims;
