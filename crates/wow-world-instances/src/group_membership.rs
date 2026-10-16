// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The represented group-membership queries and the group update-sequence
//! reset, moved from the World `session/social/group.rs` (#1363 slice 12b5).
//!
//! C++ reads them from `Player::m_group` (`Player::GetGroup`,
//! `Group::isRaidGroup`, `Group::IsMember`, `Player::IsInSameRaidWith`) and
//! `Player::ResetGroupUpdateSequenceIfNeeded`. Each body resolves the group
//! guid through the host and reads the shared group registry; the World
//! session's host impls name these functions. The host-level group-guid
//! resolution itself moved here in #1363 slice 12b7 (#1418 follow-up 1); it is
//! reached by this module's path because the crate root already exports the
//! hub-level function of the same name.

use wow_core::ObjectGuid;

use wow_world_core::session::SessionHubHostLikeCpp;

use crate::{GroupSessionHostLikeCpp, InstanceSessionHostLikeCpp};

/// `cfg!(test)` of the World crate, read through the host: `true` only in
/// the World test build, whose session impl returns its fixtures (a private
/// copy of the `wow-world-inventory` helper).
fn world_test_consumer_like_cpp<H: SessionHubHostLikeCpp + ?Sized>(host: &H) -> bool {
    #[cfg(any(test, feature = "test-fixtures"))]
    let world_test = host.test_fixtures_ref_like_cpp().is_some();
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let world_test = {
        let _ = host;
        false
    };
    world_test
}

pub fn current_player_is_in_raid_group_like_cpp<H: InstanceSessionHostLikeCpp>(host: &H) -> bool {
    let (Some(group_guid), Some(group_registry), Some(player_guid)) = (
        host.resolved_group_guid_like_cpp(),
        host.core().directory.group_registry.as_ref(),
        host.player_guid(),
    ) else {
        return false;
    };

    group_registry
        .get(&group_guid)
        .is_some_and(|group| group.is_raid_group() && group.members.contains(&player_guid))
}
pub fn represented_player_is_same_raid_with_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &H,
    player_guid: ObjectGuid,
    owner_guid: ObjectGuid,
) -> bool {
    let (Some(group_guid), Some(group_registry)) = (
        host.resolved_group_guid_like_cpp(),
        host.core().directory.group_registry.as_ref(),
    ) else {
        return false;
    };
    group_registry.get(&group_guid).is_some_and(|group| {
        group.members.contains(&player_guid) && group.members.contains(&owner_guid)
    })
}
pub fn current_group_member_guids_for_tap_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &H,
    player_guid: ObjectGuid,
) -> Vec<ObjectGuid> {
    // #743: tap rights follow C++ `Player::GetGroup()`, which the group
    // owner clears on removal. Resolve them through the authority so a
    // removal notification still in flight cannot keep granting them.
    let (Some(group_guid), Some(group_registry)) = (
        authoritative_group_membership_like_cpp(host),
        host.core().directory.group_registry.as_ref(),
    ) else {
        return Vec::new();
    };
    group_registry
        .get(&group_guid)
        .map(|group| {
            group
                .members
                .iter()
                .copied()
                .filter(|member| *member != player_guid)
                .collect()
        })
        .unwrap_or_default()
}
/// Whether this session currently belongs to `group_guid` by the authority.
///
/// C++ readers dereference `Player::m_group`, which the group owner
/// clears in the same operation. A membership-sensitive Rust reader must
/// not grant rights from the owned snapshot alone while a notification can
/// still be in flight.
pub fn authoritative_group_membership_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &H,
) -> Option<u64> {
    let (Some(group_guid), Some(group_registry), Some(player_guid)) = (
        host.resolved_group_guid_like_cpp(),
        host.core().directory.group_registry.as_ref(),
        host.player_guid(),
    ) else {
        return None;
    };
    group_registry
        .get(&group_guid)
        .filter(|group| group.members.contains(&player_guid))
        .map(|group| group.group_guid)
}

/// C++ `Player::ResetGroupUpdateSequenceIfNeeded` resets the per-player
/// sequence for a group category only when the loaded group guid changed.
pub fn reset_group_update_sequence_if_needed_like_cpp<H: GroupSessionHostLikeCpp>(
    host: &mut H,
) -> bool {
    let (Some(group_guid), Some(group_registry)) = (
        host.resolved_group_guid_like_cpp(),
        host.core().directory.group_registry.as_ref(),
    ) else {
        return false;
    };

    let Some(group) = group_registry.get(&group_guid) else {
        return false;
    };
    let category = group.group_category_like_cpp();
    if category >= wow_social::group::MAX_GROUP_CATEGORY_LIKE_CPP {
        return false;
    }

    let canonical = host.core().with_owned_player_mut_like_cpp(|player| {
        player.reset_group_update_sequence_if_needed_like_cpp(usize::from(category), group_guid)
    });
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some()
        && canonical.is_none()
        && host.core().player_handle_like_cpp.is_none()
    {
        let sequence = &mut host
            .split_social_mut()
            .0
            .represented_group_update_sequences_like_cpp[usize::from(category)];
        if sequence.group_guid == Some(group_guid) {
            return false;
        }
        sequence.group_guid = Some(group_guid);
        sequence.update_sequence_number = 1;
        return true;
    }
    canonical.unwrap_or(false)
}

/// Resolve C++ `Player::m_group` through this session incarnation's
/// generation-checked canonical Player handle. An unresolved owner never
/// falls back in production.
pub fn resolved_group_guid_like_cpp<H: GroupSessionHostLikeCpp + ?Sized>(host: &H) -> Option<u64> {
    let (state, hub) = host.split_social_ref();
    crate::resolved_group_guid_like_cpp(hub, state, world_test_consumer_like_cpp(host))
}
