// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Arena skirmish join and wargame invite acceptance (#1363 slice 4b; moved
//! from the World `handlers/battlegrounds/pvp.rs` and
//! `session/battleground_adapter.rs`).
//!
//! C++ refs: `WorldSession::HandleBattlemasterJoinSkirmish` and
//! `WorldSession::HandleAcceptWargameInvite` (`Handlers/BattleGroundHandler.cpp`).
//! The registrations stay in the application `ApplicationBattleground` area
//! registrar. The former `#[cfg(test)]` fixture steps run through
//! `SessionHubHostLikeCpp::test_fixtures_like_cpp`.

use tracing::warn;
use wow_data::{BattlemasterListStore, DISABLE_TYPE_BATTLEGROUND};
use wow_packet::ClientPacket;
use wow_packet::packets::misc::{AcceptWargameInvite, BattlemasterJoinSkirmish};
use wow_world_core::session::RepresentedBattlegroundQueueTypeIdLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_core::session::{
    RepresentedBattlemasterJoinSkirmishLikeCpp, RepresentedWargameInviteAcceptanceLikeCpp,
};

use crate::social_host::SocialSessionHostLikeCpp;

pub(crate) fn arena_skirmish_type_like_cpp(bg_type_id: u32, bracket_id: u32) -> u8 {
    if bg_type_id == 3 || bg_type_id == 5 {
        bg_type_id as u8
    } else if bracket_id == 3 || bracket_id == 5 {
        bracket_id as u8
    } else {
        2
    }
}

/// CMSG_BATTLEMASTER_JOIN_SKIRMISH — player asks to join an arena skirmish queue.
/// C++ ref: `WorldSession::HandleBattlemasterJoinSkirmish`.
pub async fn handle_battlemaster_join_skirmish_with_catalog_like_cpp<
    H: SocialSessionHostLikeCpp,
>(
    host: &mut H,
    battlemaster_lists: &BattlemasterListStore,
    mut pkt: wow_packet::WorldPacket,
) {
    let join = match BattlemasterJoinSkirmish::read(&mut pkt) {
        Ok(join) => join,
        Err(error) => {
            warn!(
                account = host.core().account_id,
                "BattlemasterJoinSkirmish parse failed: {error}"
            );
            return;
        }
    };

    // C++ ignores IsRated here, derives 2v2/3v3/5v5 from BgTypeId/BracketId,
    // and only applies group/leader gates when AsGroup is set. Queue add and
    // status fanout remain represented until live BattlegroundQueue is ported.
    let _accepted = battlemaster_join_skirmish_like_cpp(
        host,
        battlemaster_lists,
        join.bg_type_id,
        join.bracket_id,
        join.as_group,
        join.is_rated,
    );
}

pub async fn handle_accept_wargame_invite<H: SocialSessionHostLikeCpp>(
    host: &mut H,
    mut pkt: wow_packet::WorldPacket,
) {
    let packet = match AcceptWargameInvite::read(&mut pkt) {
        Ok(packet) => packet,
        Err(error) => {
            warn!(
                account = host.core().account_id,
                "AcceptWargameInvite parse failed: {error}"
            );
            return;
        }
    };

    accept_represented_wargame_invite_like_cpp(host, &packet.inviter_name);
}

#[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_variables))]
pub(crate) fn battlemaster_join_skirmish_like_cpp<H: SocialSessionHostLikeCpp>(
    host: &mut H,
    battlemaster_lists: &BattlemasterListStore,
    bg_type_id: u32,
    bracket_id: u32,
    as_group: u8,
    is_rated: u8,
) -> bool {
    if host.hub_ref().player_in_represented_battleground_like_cpp() {
        return false;
    }

    let arena_type = arena_skirmish_type_like_cpp(bg_type_id, bracket_id);
    let Some(entry) = battlemaster_lists.get(wow_data::BATTLEGROUND_AA_LIKE_CPP) else {
        return false;
    };
    if entry.instance_type != wow_data::MAP_ARENA_LIKE_CPP {
        return false;
    }
    if host
        .hub_ref()
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
            host.player_guid(),
            host.resolved_group_guid_like_cpp(),
            host.core().directory.group_registry.as_ref(),
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
    #[cfg(any(test, feature = "test-fixtures"))]
    if let Some(fixtures) = host.test_fixtures_like_cpp() {
        fixtures
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
    }
    true
}

#[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_variables))]
pub(crate) fn accept_represented_wargame_invite_like_cpp<H: SocialSessionHostLikeCpp>(
    host: &mut H,
    inviter_name: &str,
) {
    let (Some(player_guid), Some(player_group_guid), Some(player_registry), Some(group_registry)) = (
        host.player_guid(),
        host.resolved_group_guid_like_cpp(),
        host.core().player_registry.as_ref(),
        host.core().directory.group_registry.as_ref(),
    ) else {
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

    #[cfg(any(test, feature = "test-fixtures"))]
    if let Some(fixtures) = host.test_fixtures_like_cpp() {
        fixtures
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
