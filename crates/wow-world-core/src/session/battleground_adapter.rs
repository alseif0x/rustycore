// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_core::ObjectGuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedBattlemasterHelloLikeCpp {
    pub unit: ObjectGuid,
    pub entry: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedBattlefieldListLikeCpp {
    pub list_id: u32,
}

pub type RepresentedBattlegroundQueueTypeIdLikeCpp =
    wow_entities::PlayerBattlegroundQueueTypeIdLikeCpp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentedBattlemasterJoinLikeCpp {
    pub packed_queue_id: u64,
    pub queue_type_id: RepresentedBattlegroundQueueTypeIdLikeCpp,
    pub roles: u8,
    pub blacklist_map: [i32; 2],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentedBattlemasterJoinArenaLikeCpp {
    pub team_size_index: u8,
    pub roles: u8,
    pub arena_type: u8,
    pub group_guid: u64,
    pub queue_type_id: RepresentedBattlegroundQueueTypeIdLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentedBattlemasterJoinSkirmishLikeCpp {
    pub bg_type_id: u32,
    pub bracket_id: u32,
    pub as_group: bool,
    pub is_rated_packet_value: u8,
    pub arena_type: u8,
    pub group_guid: Option<u64>,
    pub queue_type_id: RepresentedBattlegroundQueueTypeIdLikeCpp,
}

#[cfg(any(test, feature = "test-fixtures"))]
pub type RepresentedBattlegroundQueueSlotLikeCpp =
    wow_entities::PlayerBattlegroundQueueSlotLikeCpp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentedBattlefieldPortLikeCpp {
    pub ticket: wow_packet::packets::misc::LfgRideTicket,
    pub accepted_invite: bool,
    pub queue_type_id: RepresentedBattlegroundQueueTypeIdLikeCpp,
    pub invited_instance_guid: u32,
}
