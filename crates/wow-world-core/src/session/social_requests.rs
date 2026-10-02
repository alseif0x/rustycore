// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_core::ObjectGuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentedWargameInviteAcceptanceLikeCpp {
    pub inviter_name: String,
    pub inviter_guid: ObjectGuid,
    pub player_group_guid: u64,
    pub inviter_group_guid: u64,
    pub group_size: usize,
}
