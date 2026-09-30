// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `WorldSession::battleground` sub-state (#1241 F2): moved fields, no logic.

use super::*;

/// Battleground and arena membership and the represented battlemaster, battlefield and wargame
/// request sinks.
pub(in crate::session) struct BattlegroundState {
    #[cfg(test)]
    pub(in crate::session) represented_arena_team_id_invited_like_cpp: u32,
    #[cfg(test)]
    pub(in crate::session) represented_wargame_invite_acceptances_like_cpp:
        Vec<RepresentedWargameInviteAcceptanceLikeCpp>,
    /// Represented `Player::GetBattleground()->GetTypeID()` for C++ battleground object use.
    #[cfg(test)]
    pub(in crate::session) player_battleground_type_id_like_cpp: Option<u32>,
    /// Represented `Player::GetBattleground()->GetMapId()` until live Battleground ownership exists.
    #[cfg(test)]
    pub(in crate::session) player_battleground_map_id_like_cpp: Option<u32>,
    /// Represented `Battleground::GetStatus()` until live Battleground ownership exists.
    #[cfg(test)]
    pub(in crate::session) represented_battleground_status_like_cpp: Option<u8>,
    /// Count of represented `Player::LeaveBattleground()` requests.
    #[cfg(test)]
    pub(in crate::session) represented_battleground_leave_requests_like_cpp: u32,
    /// Represented `BattlegroundMgr::SendBattlegroundList` intents from battlemaster hello.
    #[cfg(test)]
    pub(in crate::session) represented_battlemaster_hellos_like_cpp:
        Vec<RepresentedBattlemasterHelloLikeCpp>,
    /// Represented `BattlegroundMgr::SendBattlegroundList` intents from CMSG_BATTLEFIELD_LIST.
    #[cfg(test)]
    pub(in crate::session) represented_battlefield_lists_like_cpp:
        Vec<RepresentedBattlefieldListLikeCpp>,
    /// Represented `BattlegroundQueue::AddGroup` intents from CMSG_BATTLEMASTER_JOIN.
    #[cfg(test)]
    pub(in crate::session) represented_battlemaster_joins_like_cpp:
        Vec<RepresentedBattlemasterJoinLikeCpp>,
    /// Represented rated arena queue intents from CMSG_BATTLEMASTER_JOIN_ARENA.
    #[cfg(test)]
    pub(in crate::session) represented_battlemaster_join_arenas_like_cpp:
        Vec<RepresentedBattlemasterJoinArenaLikeCpp>,
    /// Represented arena skirmish queue intents from CMSG_BATTLEMASTER_JOIN_SKIRMISH.
    #[cfg(test)]
    pub(in crate::session) represented_battlemaster_join_skirmishes_like_cpp:
        Vec<RepresentedBattlemasterJoinSkirmishLikeCpp>,
    /// Represented `Player::m_bgBattlegroundQueueID[PLAYER_MAX_BATTLEGROUND_QUEUES]`.
    #[cfg(test)]
    pub(in crate::session) represented_battleground_queue_slots_like_cpp:
        Vec<RepresentedBattlegroundQueueSlotLikeCpp>,
    /// Represented accepted/leave requests from CMSG_BATTLEFIELD_PORT before live BattlegroundMgr.
    #[cfg(test)]
    pub(in crate::session) represented_battlefield_ports_like_cpp:
        Vec<RepresentedBattlefieldPortLikeCpp>,
}
