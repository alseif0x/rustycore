// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Group invariant tests for [`super`].
//!
//! Moved verbatim from `wow_network::group_registry` by issue #137. Following
//! the extraction convention of issue #214, the only textual change is
//! dedenting by one level, which lets rustfmt collapse some argument lists and
//! drop their trailing commas.

#![cfg(test)]

use std::collections::BTreeMap;
use std::sync::atomic::Ordering;

use wow_core::{ObjectGuid, Position};
use wow_data::DifficultyStore;

use super::*;

/// The loaded-difficulty port is satisfied by the real DB2 store, so these
/// invariants keep exercising C++'s exact validation instead of a hand-written
/// stand-in. `wow-data` is a development-only edge here.
impl GroupDifficultyValidatorLikeCpp for DifficultyStore {
    fn check_loaded_dungeon_difficulty_id_like_cpp(&self, difficulty: u32) -> u32 {
        DifficultyStore::check_loaded_dungeon_difficulty_id_like_cpp(self, difficulty)
    }

    fn check_loaded_raid_difficulty_id_like_cpp(&self, difficulty: u32) -> u32 {
        DifficultyStore::check_loaded_raid_difficulty_id_like_cpp(self, difficulty)
    }

    fn check_loaded_legacy_raid_difficulty_id_like_cpp(&self, difficulty: u32) -> u32 {
        DifficultyStore::check_loaded_legacy_raid_difficulty_id_like_cpp(self, difficulty)
    }
}

#[test]
fn group_registry_reads_are_owned_and_absent_groups_stay_absent() {
    let registry = GroupRegistry::new();
    let leader = ObjectGuid::create_player(1, 42);
    let group = GroupInfo::new(leader);
    let group_guid = group.group_guid;
    registry.register_group_like_cpp(group_guid, group);

    let mut snapshot = registry.get(&group_guid).expect("group snapshot");
    snapshot.leader_guid = ObjectGuid::create_player(1, 99);

    assert_eq!(registry.get(&group_guid).unwrap().leader_guid, leader);
    assert!(registry.get(&u64::MAX).is_none());
    assert!(!registry.contains_key(&u64::MAX));
}

#[test]
fn pending_invite_reads_are_owned_and_absent_invites_stay_absent() {
    let pending = PendingInvites::new();
    let leader = ObjectGuid::create_player(1, 42);
    let invited = ObjectGuid::create_player(1, 43);
    let invite = PendingInviteLikeCpp::new_pending_group(leader, 0);
    pending.seed_invite_like_cpp(invited, invite);

    let snapshot = pending.get(&invited).expect("invite snapshot");

    assert_eq!(snapshot, invite);
    assert_eq!(pending.matching_guids(invite), vec![invited]);
    assert!(pending.get(&ObjectGuid::create_player(1, 99)).is_none());
    assert!(!pending.contains_key(&ObjectGuid::create_player(1, 99)));
}

#[test]
fn new_group_uses_cpp_personal_loot_default() {
    let leader = ObjectGuid::create_player(1, 42);
    let group = GroupInfo::new(leader);

    assert_eq!(group.loot_method, LOOT_METHOD_PERSONAL_LIKE_CPP);
    assert_eq!(group.looter_guid, leader);
    assert_eq!(group.loot_threshold, ITEM_QUALITY_UNCOMMON_LIKE_CPP);
    assert_eq!(group.dungeon_difficulty_id, DIFFICULTY_NORMAL_LIKE_CPP);
    assert_eq!(group.raid_difficulty_id, DIFFICULTY_NORMAL_RAID_LIKE_CPP);
    assert_eq!(group.legacy_raid_difficulty_id, DIFFICULTY_10_N_LIKE_CPP);
}

#[test]
fn new_group_separates_runtime_guid_from_cpp_db_store_id() {
    let leader = ObjectGuid::create_player(1, 42);
    let group = GroupInfo::new(leader);

    assert_ne!(group.db_store_id, 0);
    assert_ne!(group.group_guid, 0);
}

#[test]
fn group_is_full_uses_cpp_party_and_raid_limits() {
    let leader = ObjectGuid::create_player(1, 42);
    let mut party = GroupInfo::new(leader);
    for counter in 43..47 {
        party.add_member(ObjectGuid::create_player(1, counter));
    }
    assert!(party.is_full_like_cpp());

    let mut raid = party.clone();
    raid.convert_to_raid_like_cpp();
    assert!(!raid.is_full_like_cpp());
    for counter in 47..82 {
        raid.members.push(ObjectGuid::create_player(1, counter));
    }
    assert!(raid.is_full_like_cpp());
}

#[test]
fn concurrent_final_party_slot_accepts_exactly_one_invite_like_cpp() {
    let registry = std::sync::Arc::new(GroupRegistry::default());
    let pending = std::sync::Arc::new(PendingInvites::default());
    let leader = ObjectGuid::create_player(1, 42);
    let mut party = GroupInfo::new(leader);
    for counter in 43..46 {
        assert!(party.add_member(ObjectGuid::create_player(1, counter)));
    }
    let group_guid = party.group_guid;
    registry.register_group_like_cpp(group_guid, party);

    let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
    let candidates = [
        ObjectGuid::create_player(1, 46),
        ObjectGuid::create_player(1, 47),
    ];
    for candidate in candidates {
        pending.seed_invite_like_cpp(
            candidate,
            PendingInviteLikeCpp::new_existing_group(
                leader,
                group_guid,
                GROUP_CATEGORY_HOME_LIKE_CPP,
            ),
        );
    }
    let handles: Vec<_> = candidates
        .into_iter()
        .map(|candidate| {
            let registry = std::sync::Arc::clone(&registry);
            let pending = std::sync::Arc::clone(&pending);
            let barrier = std::sync::Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                registry.accept_invite_like_cpp(&pending, candidate, None, Some(leader))
            })
        })
        .collect();

    barrier.wait();
    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().expect("join attempt thread"))
        .collect();
    assert_eq!(
        results
            .iter()
            .filter(|result| {
                matches!(
                    result,
                    AcceptGroupInviteResultLikeCpp::JoinedExisting { .. }
                )
            })
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, AcceptGroupInviteResultLikeCpp::GroupFull))
            .count(),
        1
    );

    let party = registry.get(&group_guid).expect("party remains registered");
    assert_eq!(party.members.len(), MAX_GROUP_SIZE_LIKE_CPP);
    assert_ne!(
        party.members.contains(&candidates[0]),
        party.members.contains(&candidates[1]),
        "only one simultaneous invitee owns the final party slot"
    );
}

#[test]
fn one_pending_invite_can_be_consumed_only_once() {
    let registry = std::sync::Arc::new(GroupRegistry::default());
    let pending = std::sync::Arc::new(PendingInvites::default());
    let leader = ObjectGuid::create_player(1, 42);
    let invitee = ObjectGuid::create_player(1, 77);
    let group = GroupInfo::new(leader);
    let group_guid = group.group_guid;
    registry.register_group_like_cpp(group_guid, group);
    pending.seed_invite_like_cpp(
        invitee,
        PendingInviteLikeCpp::new_existing_group(leader, group_guid, GROUP_CATEGORY_HOME_LIKE_CPP),
    );

    let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let registry = std::sync::Arc::clone(&registry);
            let pending = std::sync::Arc::clone(&pending);
            let barrier = std::sync::Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                registry.accept_invite_like_cpp(&pending, invitee, None, Some(leader))
            })
        })
        .collect();
    barrier.wait();
    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().expect("accept thread"))
        .collect();

    assert_eq!(
        results
            .iter()
            .filter(|result| {
                matches!(
                    result,
                    AcceptGroupInviteResultLikeCpp::JoinedExisting { .. }
                )
            })
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, AcceptGroupInviteResultLikeCpp::NoInvite))
            .count(),
        1
    );
    let group = registry.get(&group_guid).expect("group remains registered");
    assert_eq!(
        group
            .members
            .iter()
            .filter(|guid| **guid == invitee)
            .count(),
        1
    );
}

#[test]
fn concurrent_pending_group_accepts_create_once_then_join_once() {
    let registry = std::sync::Arc::new(GroupRegistry::default());
    let pending = std::sync::Arc::new(PendingInvites::default());
    let leader = ObjectGuid::create_player(1, 42);
    let invitees = [
        ObjectGuid::create_player(1, 77),
        ObjectGuid::create_player(1, 78),
    ];
    let invite = PendingInviteLikeCpp::new_pending_group(leader, GROUP_CATEGORY_HOME_LIKE_CPP);
    pending.seed_invite_like_cpp(leader, invite);
    for invitee in invitees {
        pending.seed_invite_like_cpp(invitee, invite);
    }

    let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
    let handles: Vec<_> = invitees
        .into_iter()
        .map(|invitee| {
            let registry = std::sync::Arc::clone(&registry);
            let pending = std::sync::Arc::clone(&pending);
            let barrier = std::sync::Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                registry.accept_invite_like_cpp(&pending, invitee, None, Some(leader))
            })
        })
        .collect();
    barrier.wait();
    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().expect("accept thread"))
        .collect();

    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, AcceptGroupInviteResultLikeCpp::Created { .. }))
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|result| {
                matches!(
                    result,
                    AcceptGroupInviteResultLikeCpp::JoinedExisting { .. }
                )
            })
            .count(),
        1
    );
    let groups = registry.snapshots();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].members.len(), 3);
    assert!(groups[0].members.contains(&leader));
    assert!(
        invitees
            .iter()
            .all(|invitee| groups[0].members.contains(invitee))
    );
}

#[test]
fn invite_transition_failures_do_not_partially_mutate_state() {
    let registry = GroupRegistry::default();
    let pending = PendingInvites::default();
    let leader = ObjectGuid::create_player(1, 42);
    let invitee = ObjectGuid::create_player(1, 77);
    let other_leader = ObjectGuid::create_player(1, 90);
    let existing =
        PendingInviteLikeCpp::new_pending_group(other_leader, GROUP_CATEGORY_HOME_LIKE_CPP);
    pending.seed_invite_like_cpp(invitee, existing);

    assert_eq!(
        registry.create_invite_like_cpp(
            &pending,
            leader,
            invitee,
            None,
            GROUP_CATEGORY_HOME_LIKE_CPP,
            GROUP_CATEGORY_HOME_LIKE_CPP,
        ),
        CreateGroupInviteResultLikeCpp::TargetAlreadyInvited
    );
    assert_eq!(pending.get(&invitee), Some(existing));
    assert!(pending.get(&leader).is_none());

    let missing_target = ObjectGuid::create_player(1, 78);
    assert_eq!(
        registry.create_invite_like_cpp(
            &pending,
            leader,
            missing_target,
            Some(u64::MAX),
            GROUP_CATEGORY_HOME_LIKE_CPP,
            GROUP_CATEGORY_HOME_LIKE_CPP,
        ),
        CreateGroupInviteResultLikeCpp::MissingInviterGroup
    );
    assert!(pending.get(&missing_target).is_none());

    let missing_group_invite =
        PendingInviteLikeCpp::new_existing_group(leader, u64::MAX, GROUP_CATEGORY_HOME_LIKE_CPP);
    pending.seed_invite_like_cpp(invitee, missing_group_invite);
    assert!(matches!(
        registry.accept_invite_like_cpp(&pending, invitee, None, Some(leader)),
        AcceptGroupInviteResultLikeCpp::MissingGroup
    ));
    assert_eq!(pending.get(&invitee), Some(missing_group_invite));
    assert!(registry.snapshots().is_empty());

    pending.seed_invite_like_cpp(invitee, existing);
    assert!(matches!(
        registry.accept_invite_like_cpp(
            &pending,
            invitee,
            Some(GROUP_CATEGORY_INSTANCE_LIKE_CPP),
            Some(leader),
        ),
        AcceptGroupInviteResultLikeCpp::WrongCategory
    ));
    assert_eq!(pending.get(&invitee), Some(existing));

    let mut duplicate_group = GroupInfo::new(leader);
    assert!(duplicate_group.add_member(invitee));
    let duplicate_group_guid = duplicate_group.group_guid;
    registry.register_group_like_cpp(duplicate_group_guid, duplicate_group.clone());
    pending.seed_invite_like_cpp(
        invitee,
        PendingInviteLikeCpp::new_existing_group(
            leader,
            duplicate_group_guid,
            GROUP_CATEGORY_HOME_LIKE_CPP,
        ),
    );
    assert!(matches!(
        registry.accept_invite_like_cpp(&pending, invitee, None, Some(leader)),
        AcceptGroupInviteResultLikeCpp::AlreadyMember
    ));
    assert!(pending.get(&invitee).is_none());
    assert_eq!(
        registry.get(&duplicate_group_guid).unwrap().members,
        duplicate_group.members
    );
}

#[test]
fn stale_delivery_failure_cannot_cancel_a_replacement_invite() {
    let registry = GroupRegistry::default();
    let pending = PendingInvites::default();
    let invitee = ObjectGuid::create_player(1, 77);
    let stale = PendingInviteLikeCpp::new_pending_group(
        ObjectGuid::create_player(1, 42),
        GROUP_CATEGORY_HOME_LIKE_CPP,
    );
    let replacement = PendingInviteLikeCpp::new_pending_group(
        ObjectGuid::create_player(1, 43),
        GROUP_CATEGORY_HOME_LIKE_CPP,
    );
    pending.seed_invite_like_cpp(invitee, replacement);

    assert!(!registry.cancel_invite_like_cpp(&pending, invitee, stale));
    assert_eq!(pending.get(&invitee), Some(replacement));
    assert!(!registry.replace_invite_like_cpp(&pending, invitee, stale, replacement));
    assert!(registry.replace_invite_like_cpp(&pending, invitee, replacement, stale));
    assert_eq!(pending.get(&invitee), Some(stale));
    assert!(registry.expire_invite_like_cpp(&pending, invitee, stale));
    assert!(pending.get(&invitee).is_none());

    let decline = PendingInviteLikeCpp::new_pending_group(
        ObjectGuid::create_player(1, 44),
        GROUP_CATEGORY_HOME_LIKE_CPP,
    );
    pending.seed_invite_like_cpp(decline.leader_guid, decline);
    pending.seed_invite_like_cpp(invitee, decline);
    assert!(
        registry
            .decline_invite_like_cpp(&pending, invitee, Some(GROUP_CATEGORY_INSTANCE_LIKE_CPP),)
            .is_none()
    );
    assert_eq!(pending.get(&invitee), Some(decline));
    assert_eq!(
        registry.decline_invite_like_cpp(&pending, invitee, None),
        Some(decline)
    );
    assert!(pending.get(&invitee).is_none());
    assert!(pending.get(&decline.leader_guid).is_none());
}

#[test]
fn free_group_db_store_id_ignores_zero_like_cpp_unallocated_storage() {
    free_group_db_store_id_like_cpp(0);
}

#[test]
fn group_db_store_registers_and_finds_group_by_storage_id_like_cpp() {
    let registry = GroupRegistry::default();
    let leader = ObjectGuid::create_player(1, 42);
    let group = GroupInfo::loaded_from_db_like_cpp(
        90,
        1234,
        leader,
        LOOT_METHOD_PERSONAL_LIKE_CPP,
        leader,
        ITEM_QUALITY_UNCOMMON_LIKE_CPP,
        0,
        DIFFICULTY_NORMAL_LIKE_CPP,
        DIFFICULTY_NORMAL_RAID_LIKE_CPP,
        DIFFICULTY_10_N_LIKE_CPP,
        ObjectGuid::EMPTY,
    );
    registry.register_group_like_cpp(group.group_guid, group);

    register_group_db_store_id_like_cpp(1234, 90);

    let found = get_group_by_db_store_id_like_cpp(&registry, 1234)
        .expect("registered storage id should resolve to its group");
    assert_eq!(found.group_guid, 90);
    assert_eq!(found.db_store_id, 1234);
}

#[test]
fn group_db_store_free_clears_lookup_like_cpp() {
    let registry = GroupRegistry::default();
    let leader = ObjectGuid::create_player(1, 43);
    let group = GroupInfo::loaded_from_db_like_cpp(
        91,
        1235,
        leader,
        LOOT_METHOD_PERSONAL_LIKE_CPP,
        leader,
        ITEM_QUALITY_UNCOMMON_LIKE_CPP,
        0,
        DIFFICULTY_NORMAL_LIKE_CPP,
        DIFFICULTY_NORMAL_RAID_LIKE_CPP,
        DIFFICULTY_10_N_LIKE_CPP,
        ObjectGuid::EMPTY,
    );
    registry.register_group_like_cpp(group.group_guid, group);
    register_group_db_store_id_like_cpp(1235, 91);

    free_group_db_store_id_like_cpp(1235);

    assert!(get_group_by_db_store_id_like_cpp(&registry, 1235).is_none());
}

#[test]
fn loaded_group_row_preserves_cpp_group_db_fields_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let looter = ObjectGuid::create_player(1, 77);
    let master = ObjectGuid::create_player(1, 88);
    let group = GroupInfo::loaded_from_db_like_cpp(
        900,
        17,
        leader,
        3,
        looter,
        4,
        GROUP_FLAG_RAID_LIKE_CPP,
        2,
        15,
        5,
        master,
    );

    assert_eq!(group.group_guid, 900);
    assert_eq!(group.db_store_id, 17);
    assert_eq!(group.leader_guid, leader);
    assert!(group.members.is_empty());
    assert_eq!(group.loot_method, 3);
    assert_eq!(group.looter_guid, looter);
    assert_eq!(group.loot_threshold, 4);
    assert_eq!(group.group_flags, GROUP_FLAG_RAID_LIKE_CPP);
    assert_eq!(group.dungeon_difficulty_id, 2);
    assert_eq!(group.raid_difficulty_id, 15);
    assert_eq!(group.legacy_raid_difficulty_id, 5);
    assert_eq!(group.master_looter_guid, master);
}

#[test]
fn recent_instance_defaults_to_leader_and_zero_instance_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let group = GroupInfo::new(leader);

    assert_eq!(group.recent_instance_owner_like_cpp(631), leader);
    assert_eq!(group.recent_instance_id_like_cpp(631), 0);
}

#[test]
fn set_recent_instance_tracks_owner_and_instance_by_map_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let owner = ObjectGuid::create_player(1, 77);
    let mut group = GroupInfo::new(leader);

    group.set_recent_instance_like_cpp(631, owner, 9001);

    assert_eq!(group.recent_instance_owner_like_cpp(631), owner);
    assert_eq!(group.recent_instance_id_like_cpp(631), 9001);
    assert_eq!(
        group.recent_instance_owner_like_cpp(533),
        leader,
        "other maps still fall back to C++ leader guid"
    );
    assert_eq!(group.recent_instance_id_like_cpp(533), 0);
}

#[test]
fn set_recent_instance_replaces_same_map_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let first_owner = ObjectGuid::create_player(1, 77);
    let second_owner = ObjectGuid::create_player(1, 88);
    let mut group = GroupInfo::new(leader);

    group.set_recent_instance_like_cpp(631, first_owner, 9001);
    group.set_recent_instance_like_cpp(631, second_owner, 9002);

    assert_eq!(group.recent_instance_owner_like_cpp(631), second_owner);
    assert_eq!(group.recent_instance_id_like_cpp(631), 9002);
}

#[test]
fn forget_recent_instance_erases_map_binding_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let owner = ObjectGuid::create_player(1, 77);
    let mut group = GroupInfo::new(leader);

    group.set_recent_instance_like_cpp(631, owner, 9001);

    assert!(group.forget_recent_instance_like_cpp(631));
    assert!(!group.forget_recent_instance_like_cpp(631));
    assert_eq!(group.recent_instance_owner_like_cpp(631), leader);
    assert_eq!(group.recent_instance_id_like_cpp(631), 0);
}

#[test]
fn link_owned_instance_tracks_unique_instance_map_references_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let mut group = GroupInfo::new(leader);

    assert!(group.link_owned_instance_like_cpp(631, 9001));
    assert!(!group.link_owned_instance_like_cpp(631, 9001));
    assert!(group.link_owned_instance_like_cpp(631, 9002));

    let owned: Vec<_> = group.owned_instances_like_cpp().collect();
    assert_eq!(
        owned,
        vec![
            GroupOwnedInstanceLikeCpp {
                map_id: 631,
                instance_id: 9001,
            },
            GroupOwnedInstanceLikeCpp {
                map_id: 631,
                instance_id: 9002,
            },
        ]
    );
}

#[test]
fn unlink_owned_instance_removes_reference_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let mut group = GroupInfo::new(leader);

    group.link_owned_instance_like_cpp(631, 9001);

    assert!(group.unlink_owned_instance_like_cpp(631, 9001));
    assert!(!group.unlink_owned_instance_like_cpp(631, 9001));
    assert_eq!(group.owned_instances_like_cpp().count(), 0);
}

#[test]
fn reset_success_and_cannot_reset_forget_recent_instance_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let owner = ObjectGuid::create_player(1, 77);
    let mut group = GroupInfo::new(leader);

    group.set_recent_instance_like_cpp(631, owner, 9001);
    assert!(group.apply_owned_instance_reset_result_like_cpp(
        631,
        GroupInstanceResetResultLikeCpp::Success,
        GroupInstanceResetMethodLikeCpp::Manual,
    ));
    assert_eq!(group.recent_instance_id_like_cpp(631), 0);

    group.set_recent_instance_like_cpp(631, owner, 9002);
    assert!(group.apply_owned_instance_reset_result_like_cpp(
        631,
        GroupInstanceResetResultLikeCpp::CannotReset,
        GroupInstanceResetMethodLikeCpp::Manual,
    ));
    assert_eq!(group.recent_instance_id_like_cpp(631), 0);
}

#[test]
fn reset_not_empty_forgets_only_on_change_difficulty_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let owner = ObjectGuid::create_player(1, 77);
    let mut group = GroupInfo::new(leader);

    group.set_recent_instance_like_cpp(631, owner, 9001);
    assert!(!group.apply_owned_instance_reset_result_like_cpp(
        631,
        GroupInstanceResetResultLikeCpp::NotEmpty,
        GroupInstanceResetMethodLikeCpp::Manual,
    ));
    assert_eq!(group.recent_instance_id_like_cpp(631), 9001);

    assert!(group.apply_owned_instance_reset_result_like_cpp(
        631,
        GroupInstanceResetResultLikeCpp::NotEmpty,
        GroupInstanceResetMethodLikeCpp::OnChangeDifficulty,
    ));
    assert_eq!(group.recent_instance_id_like_cpp(631), 0);
}

#[test]
fn reset_other_result_keeps_recent_instance_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let owner = ObjectGuid::create_player(1, 77);
    let mut group = GroupInfo::new(leader);

    group.set_recent_instance_like_cpp(631, owner, 9001);
    assert!(!group.apply_owned_instance_reset_result_like_cpp(
        631,
        GroupInstanceResetResultLikeCpp::Other,
        GroupInstanceResetMethodLikeCpp::Manual,
    ));
    assert_eq!(group.recent_instance_id_like_cpp(631), 9001);
}

#[test]
fn loaded_group_row_validates_difficulties_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let difficulty_store = DifficultyStore::from_entries([
        wow_data::DifficultyEntry {
            id: 2,
            instance_type: 1,
            flags: wow_constants::shared::DifficultyFlags::CAN_SELECT.bits(),
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        },
        wow_data::DifficultyEntry {
            id: 15,
            instance_type: 2,
            flags: wow_constants::shared::DifficultyFlags::CAN_SELECT.bits(),
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        },
        wow_data::DifficultyEntry {
            id: 3,
            instance_type: 2,
            flags: (wow_constants::shared::DifficultyFlags::CAN_SELECT
                | wow_constants::shared::DifficultyFlags::LEGACY)
                .bits(),
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        },
    ]);

    let valid = GroupInfo::loaded_from_db_validated_like_cpp(
        901,
        18,
        leader,
        LOOT_METHOD_PERSONAL_LIKE_CPP,
        leader,
        ITEM_QUALITY_UNCOMMON_LIKE_CPP,
        0,
        2,
        15,
        3,
        ObjectGuid::EMPTY,
        &difficulty_store,
    );
    assert_eq!(valid.dungeon_difficulty_id, 2);
    assert_eq!(valid.raid_difficulty_id, 15);
    assert_eq!(valid.legacy_raid_difficulty_id, 3);

    let fallback = GroupInfo::loaded_from_db_validated_like_cpp(
        902,
        19,
        leader,
        LOOT_METHOD_PERSONAL_LIKE_CPP,
        leader,
        ITEM_QUALITY_UNCOMMON_LIKE_CPP,
        0,
        15,
        3,
        15,
        ObjectGuid::EMPTY,
        &difficulty_store,
    );
    assert_eq!(fallback.dungeon_difficulty_id, DIFFICULTY_NORMAL_LIKE_CPP);
    assert_eq!(fallback.raid_difficulty_id, DIFFICULTY_NORMAL_RAID_LIKE_CPP);
    assert_eq!(fallback.legacy_raid_difficulty_id, DIFFICULTY_10_N_LIKE_CPP);
}

#[test]
fn load_member_from_db_skips_missing_character_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let mut group = GroupInfo::loaded_from_db_like_cpp(
        903,
        20,
        leader,
        LOOT_METHOD_PERSONAL_LIKE_CPP,
        leader,
        ITEM_QUALITY_UNCOMMON_LIKE_CPP,
        0,
        DIFFICULTY_NORMAL_LIKE_CPP,
        DIFFICULTY_NORMAL_RAID_LIKE_CPP,
        DIFFICULTY_10_N_LIKE_CPP,
        ObjectGuid::EMPTY,
    );

    assert!(!group.load_member_from_db_like_cpp(77, 0, 1, 2, None));
    assert!(group.members.is_empty());
    assert!(group.member_slots.is_empty());
}

#[test]
fn load_member_from_db_preserves_slot_fields_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let mut group = GroupInfo::loaded_from_db_like_cpp(
        904,
        21,
        leader,
        LOOT_METHOD_PERSONAL_LIKE_CPP,
        leader,
        ITEM_QUALITY_UNCOMMON_LIKE_CPP,
        GROUP_FLAG_RAID_LIKE_CPP,
        DIFFICULTY_NORMAL_LIKE_CPP,
        DIFFICULTY_NORMAL_RAID_LIKE_CPP,
        DIFFICULTY_10_N_LIKE_CPP,
        ObjectGuid::EMPTY,
    );

    assert!(group.load_member_from_db_like_cpp(
        77,
        0x04,
        3,
        2,
        Some(GroupMemberCharacterLikeCpp {
            name: "Member".to_string(),
            race: 4,
            class: 8,
        }),
    ));

    let member_guid = ObjectGuid::create_player(1, 77);
    assert_eq!(group.members, vec![member_guid]);
    let slot = group
        .member_slot_like_cpp(member_guid)
        .expect("loaded DB member should have a represented slot");
    assert_eq!(slot.name, "Member");
    assert_eq!(slot.race, 4);
    assert_eq!(slot.class, 8);
    assert_eq!(slot.subgroup, 3);
    assert_eq!(slot.flags, 0x04);
    assert_eq!(slot.roles, 2);
    assert!(!slot.ready_checked);
}

#[test]
fn load_member_from_db_everyone_assistant_adds_assistant_flag_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let mut group = GroupInfo::loaded_from_db_like_cpp(
        905,
        22,
        leader,
        LOOT_METHOD_PERSONAL_LIKE_CPP,
        leader,
        ITEM_QUALITY_UNCOMMON_LIKE_CPP,
        GROUP_FLAG_EVERYONE_ASSISTANT_LIKE_CPP,
        DIFFICULTY_NORMAL_LIKE_CPP,
        DIFFICULTY_NORMAL_RAID_LIKE_CPP,
        DIFFICULTY_10_N_LIKE_CPP,
        ObjectGuid::EMPTY,
    );

    assert!(group.load_member_from_db_like_cpp(
        78,
        0,
        0,
        0,
        Some(GroupMemberCharacterLikeCpp {
            name: "Assistant".to_string(),
            race: 1,
            class: 2,
        }),
    ));

    let slot = group
        .member_slot_like_cpp(ObjectGuid::create_player(1, 78))
        .expect("loaded DB member should have a represented slot");
    assert_eq!(
        slot.flags & MEMBER_FLAG_ASSISTANT_LIKE_CPP,
        MEMBER_FLAG_ASSISTANT_LIKE_CPP
    );
}

#[test]
fn loaded_raid_group_tracks_subgroup_counts_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let mut group = GroupInfo::loaded_from_db_like_cpp(
        906,
        23,
        leader,
        LOOT_METHOD_PERSONAL_LIKE_CPP,
        leader,
        ITEM_QUALITY_UNCOMMON_LIKE_CPP,
        GROUP_FLAG_RAID_LIKE_CPP,
        DIFFICULTY_NORMAL_LIKE_CPP,
        DIFFICULTY_NORMAL_RAID_LIKE_CPP,
        DIFFICULTY_10_N_LIKE_CPP,
        ObjectGuid::EMPTY,
    );

    assert!(group.has_free_slot_sub_group_like_cpp(3));
    for guid_low in 100..105 {
        assert!(group.load_member_from_db_like_cpp(
            guid_low,
            0,
            3,
            0,
            Some(GroupMemberCharacterLikeCpp {
                name: format!("Member{guid_low}"),
                race: 1,
                class: 1,
            }),
        ));
    }

    assert!(!group.has_free_slot_sub_group_like_cpp(3));
    assert_eq!(
        group.member_group_like_cpp(ObjectGuid::create_player(1, 104)),
        3
    );
    assert_eq!(
        group.member_group_like_cpp(ObjectGuid::create_player(1, 999)),
        MISSING_MEMBER_GROUP_LIKE_CPP
    );

    group.remove_member(&ObjectGuid::create_player(1, 104));
    assert!(group.has_free_slot_sub_group_like_cpp(3));
}

#[test]
fn convert_to_raid_initializes_subgroup_counts_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let mut group = GroupInfo::new(leader);
    assert!(!group.has_free_slot_sub_group_like_cpp(0));

    group.convert_to_raid_like_cpp();

    assert!(group.has_free_slot_sub_group_like_cpp(0));
    for guid_low in 200..204 {
        group.add_member(ObjectGuid::create_player(1, guid_low));
    }
    assert!(!group.has_free_slot_sub_group_like_cpp(0));
}

#[test]
fn loaded_raid_group_rejects_out_of_range_subgroup_without_panicking_boundary() {
    let leader = ObjectGuid::create_player(1, 42);
    let mut group = GroupInfo::loaded_from_db_like_cpp(
        906,
        24,
        leader,
        LOOT_METHOD_PERSONAL_LIKE_CPP,
        leader,
        ITEM_QUALITY_UNCOMMON_LIKE_CPP,
        GROUP_FLAG_RAID_LIKE_CPP,
        DIFFICULTY_NORMAL_LIKE_CPP,
        DIFFICULTY_NORMAL_RAID_LIKE_CPP,
        DIFFICULTY_10_N_LIKE_CPP,
        ObjectGuid::EMPTY,
    );

    assert!(!group.load_member_from_db_like_cpp(
        300,
        0,
        MAX_RAID_SUBGROUPS_LIKE_CPP as u8,
        0,
        Some(GroupMemberCharacterLikeCpp {
            name: "Invalid".to_string(),
            race: 1,
            class: 1,
        }),
    ));
    assert!(group.members.is_empty());
    assert!(group.member_slots.is_empty());
}

#[test]
fn group_member_flag_toggles_assistant_in_raid_without_uniqueness_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let first = ObjectGuid::create_player(1, 390);
    let second = ObjectGuid::create_player(1, 391);
    let mut group = GroupInfo::new(leader);
    group.add_member(first);
    group.add_member(second);
    group.convert_to_raid_like_cpp();
    let sequence_before = group.sequence_num;

    assert_eq!(
        group.set_assistant_leader_flag_like_cpp(first, true),
        Some(MEMBER_FLAG_ASSISTANT_LIKE_CPP)
    );
    assert_eq!(
        group.set_assistant_leader_flag_like_cpp(second, true),
        Some(MEMBER_FLAG_ASSISTANT_LIKE_CPP)
    );
    assert_eq!(
        group.member_slot_like_cpp(first).unwrap().flags & MEMBER_FLAG_ASSISTANT_LIKE_CPP,
        MEMBER_FLAG_ASSISTANT_LIKE_CPP
    );
    assert_eq!(
        group.member_slot_like_cpp(second).unwrap().flags & MEMBER_FLAG_ASSISTANT_LIKE_CPP,
        MEMBER_FLAG_ASSISTANT_LIKE_CPP
    );
    assert_eq!(group.sequence_num, sequence_before + 2);

    assert_eq!(
        group.set_assistant_leader_flag_like_cpp(first, false),
        Some(0)
    );
    assert_eq!(group.member_slot_like_cpp(first).unwrap().flags, 0);
}

#[test]
fn group_member_flag_returns_final_flags_even_when_unchanged_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 392);
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    group.convert_to_raid_like_cpp();

    assert_eq!(
        group.set_assistant_leader_flag_like_cpp(member, true),
        Some(MEMBER_FLAG_ASSISTANT_LIKE_CPP)
    );
    let sequence_after_change = group.sequence_num;
    assert_eq!(
        group.set_assistant_leader_flag_like_cpp(member, true),
        Some(MEMBER_FLAG_ASSISTANT_LIKE_CPP)
    );
    assert_eq!(group.sequence_num, sequence_after_change);
}

#[test]
fn group_member_flag_rejects_non_raid_missing_or_unsupported_flag_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 393);
    let missing = ObjectGuid::create_player(1, 394);
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    let sequence_before = group.sequence_num;

    assert_eq!(group.set_assistant_leader_flag_like_cpp(member, true), None);
    group.convert_to_raid_like_cpp();
    assert_eq!(
        group.set_assistant_leader_flag_like_cpp(missing, true),
        None
    );
    assert_eq!(
        group.set_group_member_flag_like_cpp(member, true, 0x08),
        None
    );
    assert_eq!(group.member_slot_like_cpp(member).unwrap().flags, 0);
    assert_eq!(group.sequence_num, sequence_before + 1);
}

#[test]
fn everyone_is_assistant_apply_marks_group_and_all_members_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let first = ObjectGuid::create_player(1, 395);
    let second = ObjectGuid::create_player(1, 396);
    let mut group = GroupInfo::new(leader);
    group.add_member(first);
    group.add_member(second);
    let sequence_before = group.sequence_num;

    let (group_flags, db_store_id) = group.set_everyone_is_assistant_like_cpp(true);

    assert_eq!(db_store_id, group.db_store_id);
    assert_eq!(
        group_flags & GROUP_FLAG_EVERYONE_ASSISTANT_LIKE_CPP,
        GROUP_FLAG_EVERYONE_ASSISTANT_LIKE_CPP
    );
    for guid in [leader, first, second] {
        assert_eq!(
            group.member_slot_like_cpp(guid).unwrap().flags & MEMBER_FLAG_ASSISTANT_LIKE_CPP,
            MEMBER_FLAG_ASSISTANT_LIKE_CPP
        );
    }
    assert_eq!(group.sequence_num, sequence_before + 1);
}

#[test]
fn everyone_is_assistant_clear_unmarks_group_and_all_assistants_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let first = ObjectGuid::create_player(1, 397);
    let mut group = GroupInfo::new(leader);
    group.add_member(first);
    group.set_everyone_is_assistant_like_cpp(true);
    let sequence_after_apply = group.sequence_num;

    let (group_flags, db_store_id) = group.set_everyone_is_assistant_like_cpp(false);

    assert_eq!(db_store_id, group.db_store_id);
    assert_eq!(group_flags & GROUP_FLAG_EVERYONE_ASSISTANT_LIKE_CPP, 0);
    for guid in [leader, first] {
        assert_eq!(
            group.member_slot_like_cpp(guid).unwrap().flags & MEMBER_FLAG_ASSISTANT_LIKE_CPP,
            0
        );
    }
    assert_eq!(group.sequence_num, sequence_after_apply + 1);
}

#[test]
fn everyone_is_assistant_works_in_non_raid_group_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 398);
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    assert!(!group.is_raid_group());

    group.set_everyone_is_assistant_like_cpp(true);

    assert_eq!(
        group.group_flags & GROUP_FLAG_EVERYONE_ASSISTANT_LIKE_CPP,
        GROUP_FLAG_EVERYONE_ASSISTANT_LIKE_CPP
    );
    assert_eq!(
        group.member_slot_like_cpp(member).unwrap().flags & MEMBER_FLAG_ASSISTANT_LIKE_CPP,
        MEMBER_FLAG_ASSISTANT_LIKE_CPP
    );
}

#[test]
fn everyone_is_assistant_idempotent_returns_final_flags_without_sequence_bump_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 399);
    let mut group = GroupInfo::new(leader);
    group.add_member(member);

    let (first_flags, first_db_store_id) = group.set_everyone_is_assistant_like_cpp(true);
    let sequence_after_apply = group.sequence_num;
    let (second_flags, second_db_store_id) = group.set_everyone_is_assistant_like_cpp(true);

    assert_eq!(second_flags, first_flags);
    assert_eq!(second_db_store_id, first_db_store_id);
    assert_eq!(group.sequence_num, sequence_after_apply);
}

#[test]
fn change_leader_like_cpp_sets_leader_and_clears_assistant_flag() {
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 400);
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    group.convert_to_raid_like_cpp();
    assert_eq!(
        group.set_assistant_leader_flag_like_cpp(member, true),
        Some(MEMBER_FLAG_ASSISTANT_LIKE_CPP)
    );
    let previous_sequence = group.sequence_num;

    assert_eq!(group.change_leader_like_cpp(member), Some(0));

    assert_eq!(group.leader_guid, member);
    assert_eq!(group.member_slot_like_cpp(member).unwrap().flags, 0);
    assert_eq!(group.sequence_num, previous_sequence + 1);
}

#[test]
fn change_leader_like_cpp_rejects_missing_member() {
    let leader = ObjectGuid::create_player(1, 42);
    let missing = ObjectGuid::create_player(1, 401);
    let mut group = GroupInfo::new(leader);

    assert_eq!(group.change_leader_like_cpp(missing), None);
    assert_eq!(group.leader_guid, leader);
}

#[test]
fn change_member_group_updates_raid_subgroup_counts_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let mut group = GroupInfo::loaded_from_db_like_cpp(
        907,
        25,
        leader,
        LOOT_METHOD_PERSONAL_LIKE_CPP,
        leader,
        ITEM_QUALITY_UNCOMMON_LIKE_CPP,
        GROUP_FLAG_RAID_LIKE_CPP,
        DIFFICULTY_NORMAL_LIKE_CPP,
        DIFFICULTY_NORMAL_RAID_LIKE_CPP,
        DIFFICULTY_10_N_LIKE_CPP,
        ObjectGuid::EMPTY,
    );
    let member = ObjectGuid::create_player(1, 400);
    assert!(group.load_member_from_db_like_cpp(
        400,
        0,
        0,
        0,
        Some(GroupMemberCharacterLikeCpp {
            name: "Mover".to_string(),
            race: 1,
            class: 1,
        }),
    ));

    assert!(group.change_member_group_like_cpp(member, 2));
    assert_eq!(group.member_group_like_cpp(member), 2);

    for guid_low in 401..406 {
        assert!(group.load_member_from_db_like_cpp(
            guid_low,
            0,
            0,
            0,
            Some(GroupMemberCharacterLikeCpp {
                name: format!("Member{guid_low}"),
                race: 1,
                class: 1,
            }),
        ));
    }
    assert!(!group.has_free_slot_sub_group_like_cpp(0));
    assert!(group.has_free_slot_sub_group_like_cpp(2));
}

#[test]
fn change_member_group_rejects_non_raid_missing_full_or_same_group_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 500);
    let mut party = GroupInfo::new(leader);
    party.add_member(member);
    assert!(!party.change_member_group_like_cpp(member, 1));

    let mut raid = GroupInfo::loaded_from_db_like_cpp(
        908,
        26,
        leader,
        LOOT_METHOD_PERSONAL_LIKE_CPP,
        leader,
        ITEM_QUALITY_UNCOMMON_LIKE_CPP,
        GROUP_FLAG_RAID_LIKE_CPP,
        DIFFICULTY_NORMAL_LIKE_CPP,
        DIFFICULTY_NORMAL_RAID_LIKE_CPP,
        DIFFICULTY_10_N_LIKE_CPP,
        ObjectGuid::EMPTY,
    );
    assert!(!raid.change_member_group_like_cpp(member, 1));
    assert!(raid.load_member_from_db_like_cpp(
        500,
        0,
        0,
        0,
        Some(GroupMemberCharacterLikeCpp {
            name: "Mover".to_string(),
            race: 1,
            class: 1,
        }),
    ));
    assert!(!raid.change_member_group_like_cpp(member, 0));
    assert!(!raid.change_member_group_like_cpp(member, MAX_RAID_SUBGROUPS_LIKE_CPP as u8));
}

#[test]
fn swap_members_groups_like_cpp_swaps_raid_members_without_counter_drift() {
    let leader = ObjectGuid::create_player(1, 42);
    let first = ObjectGuid::create_player(1, 600);
    let second = ObjectGuid::create_player(1, 601);
    let mut group = GroupInfo::loaded_from_db_like_cpp(
        909,
        27,
        leader,
        LOOT_METHOD_PERSONAL_LIKE_CPP,
        leader,
        ITEM_QUALITY_UNCOMMON_LIKE_CPP,
        GROUP_FLAG_RAID_LIKE_CPP,
        DIFFICULTY_NORMAL_LIKE_CPP,
        DIFFICULTY_NORMAL_RAID_LIKE_CPP,
        DIFFICULTY_10_N_LIKE_CPP,
        ObjectGuid::EMPTY,
    );
    assert!(group.load_member_from_db_like_cpp(
        600,
        0,
        1,
        0,
        Some(GroupMemberCharacterLikeCpp {
            name: "First".to_string(),
            race: 1,
            class: 1,
        }),
    ));
    assert!(group.load_member_from_db_like_cpp(
        601,
        0,
        2,
        0,
        Some(GroupMemberCharacterLikeCpp {
            name: "Second".to_string(),
            race: 1,
            class: 1,
        }),
    ));
    let counts_before = group.raid_subgroup_counts;
    let sequence_before = group.sequence_num;

    let updates = group
        .swap_members_groups_like_cpp(first, second)
        .expect("different raid subgroups should swap");

    assert_eq!(updates, [(first, 2), (second, 1)]);
    assert_eq!(group.member_group_like_cpp(first), 2);
    assert_eq!(group.member_group_like_cpp(second), 1);
    assert_eq!(group.raid_subgroup_counts, counts_before);
    assert!(group.has_free_slot_sub_group_like_cpp(1));
    assert!(group.has_free_slot_sub_group_like_cpp(2));
    assert_eq!(group.sequence_num, sequence_before + 1);
}

#[test]
fn swap_members_groups_like_cpp_rejects_party_missing_member_or_same_subgroup() {
    let leader = ObjectGuid::create_player(1, 42);
    let first = ObjectGuid::create_player(1, 610);
    let second = ObjectGuid::create_player(1, 611);
    let missing = ObjectGuid::create_player(1, 612);

    let mut party = GroupInfo::new(leader);
    party.add_member(first);
    party.add_member(second);
    assert_eq!(party.swap_members_groups_like_cpp(first, second), None);

    let mut raid = GroupInfo::loaded_from_db_like_cpp(
        910,
        28,
        leader,
        LOOT_METHOD_PERSONAL_LIKE_CPP,
        leader,
        ITEM_QUALITY_UNCOMMON_LIKE_CPP,
        GROUP_FLAG_RAID_LIKE_CPP,
        DIFFICULTY_NORMAL_LIKE_CPP,
        DIFFICULTY_NORMAL_RAID_LIKE_CPP,
        DIFFICULTY_10_N_LIKE_CPP,
        ObjectGuid::EMPTY,
    );
    assert!(raid.load_member_from_db_like_cpp(
        610,
        0,
        3,
        0,
        Some(GroupMemberCharacterLikeCpp {
            name: "First".to_string(),
            race: 1,
            class: 1,
        }),
    ));
    assert!(raid.load_member_from_db_like_cpp(
        611,
        0,
        3,
        0,
        Some(GroupMemberCharacterLikeCpp {
            name: "Second".to_string(),
            race: 1,
            class: 1,
        }),
    ));
    let counts_before = raid.raid_subgroup_counts;
    let sequence_before = raid.sequence_num;

    assert_eq!(raid.swap_members_groups_like_cpp(first, missing), None);
    assert_eq!(raid.swap_members_groups_like_cpp(first, second), None);
    assert_eq!(raid.member_group_like_cpp(first), 3);
    assert_eq!(raid.member_group_like_cpp(second), 3);
    assert_eq!(raid.raid_subgroup_counts, counts_before);
    assert_eq!(raid.sequence_num, sequence_before);
}

#[test]
fn target_icon_list_returns_all_eight_symbols_in_cpp_order() {
    let target = ObjectGuid::create_player(1, 77);
    let mut group = GroupInfo::new(ObjectGuid::create_player(1, 42));
    group.target_icons[3] = target.to_raw_bytes();

    let icons = group.target_icon_list_like_cpp();

    assert_eq!(icons.len(), TARGET_ICONS_COUNT_LIKE_CPP);
    assert_eq!(icons[0], (0, ObjectGuid::EMPTY));
    assert_eq!(icons[3], (3, target));
    assert_eq!(icons[7], (7, ObjectGuid::EMPTY));
}

#[test]
fn set_target_icon_out_of_range_does_not_mutate_like_cpp() {
    let target = ObjectGuid::create_player(1, 77);
    let mut group = GroupInfo::new(ObjectGuid::create_player(1, 42));

    assert_eq!(group.set_target_icon_like_cpp(8, target), None);
    assert!(
        group
            .target_icons
            .iter()
            .all(|raw| *raw == EMPTY_TARGET_ICON_RAW_LIKE_CPP)
    );
}

#[test]
fn set_target_icon_clears_duplicate_target_before_assignment_like_cpp() {
    let target = ObjectGuid::create_player(1, 77);
    let mut group = GroupInfo::new(ObjectGuid::create_player(1, 42));
    group.set_target_icon_like_cpp(2, target).unwrap();

    let updates = group.set_target_icon_like_cpp(5, target).unwrap();

    assert_eq!(updates, vec![(2, ObjectGuid::EMPTY), (5, target)]);
    assert_eq!(group.target_icons[2], EMPTY_TARGET_ICON_RAW_LIKE_CPP);
    assert_eq!(group.target_icons[5], target.to_raw_bytes());
    assert_eq!(
        group
            .target_icon_list_like_cpp()
            .into_iter()
            .filter(|(_, icon_target)| *icon_target == target)
            .count(),
        1
    );
}

#[test]
fn add_raid_marker_preserves_cpp_slots_mask_and_duplicate_rejection() {
    let transport = ObjectGuid::create_transport(wow_core::guid::HighGuid::Transport, 0x55AA);
    let mut group = GroupInfo::new(ObjectGuid::create_player(1, 42));
    let sequence_before = group.sequence_num;
    let position = Position::xyz(12.25, -34.5, 6.75);

    assert!(group.add_raid_marker_like_cpp(3, 571, position, transport));
    assert_eq!(group.active_raid_markers_mask_like_cpp(), 1 << 3);
    assert_eq!(
        group.raid_marker_list_like_cpp(),
        vec![RaidMarkerLikeCpp {
            map_id: 571,
            position,
            transport_guid: transport,
        }]
    );
    assert_eq!(
        group.sequence_num, sequence_before,
        "C++ Group::AddRaidMarker sends RaidMarkersChanged and does not advance PartyUpdate sequence"
    );

    assert!(!group.add_raid_marker_like_cpp(3, 1, Position::ZERO, ObjectGuid::EMPTY));
    assert!(!group.add_raid_marker_like_cpp(8, 1, Position::ZERO, ObjectGuid::EMPTY));
    assert_eq!(group.active_raid_markers_mask_like_cpp(), 1 << 3);
    assert_eq!(group.raid_marker_list_like_cpp().len(), 1);
}

#[test]
fn delete_raid_marker_preserves_cpp_single_all_and_out_of_range_semantics() {
    let mut group = GroupInfo::new(ObjectGuid::create_player(1, 42));
    group.add_raid_marker_like_cpp(1, 571, Position::xyz(1.0, 2.0, 3.0), ObjectGuid::EMPTY);
    group.add_raid_marker_like_cpp(3, 571, Position::xyz(4.0, 5.0, 6.0), ObjectGuid::EMPTY);

    assert!(group.delete_raid_marker_like_cpp(1));
    assert_eq!(group.active_raid_markers_mask_like_cpp(), 1 << 3);
    assert!(!group.delete_raid_marker_like_cpp(1));
    assert_eq!(group.active_raid_markers_mask_like_cpp(), 1 << 3);

    assert!(!group.delete_raid_marker_like_cpp(9));
    assert_eq!(group.active_raid_markers_mask_like_cpp(), 1 << 3);

    assert!(group.delete_raid_marker_like_cpp(RAID_MARKERS_COUNT_LIKE_CPP as u8));
    assert_eq!(group.active_raid_markers_mask_like_cpp(), 0);
    assert!(group.raid_marker_list_like_cpp().is_empty());
}

#[test]
fn update_looter_guid_preserves_cpp_free_for_all_noop() {
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 43);
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    group.loot_method = LOOT_METHOD_FREE_FOR_ALL_LIKE_CPP;
    group.looter_guid = leader;
    let sequence_before = group.sequence_num;

    assert!(!group.update_looter_guid_like_cpp([member], false));

    assert_eq!(group.looter_guid, leader);
    assert_eq!(group.looter_guid_like_cpp(), ObjectGuid::EMPTY);
    assert_eq!(group.sequence_num, sequence_before);
}

#[test]
fn update_looter_guid_ifneed_keeps_current_eligible_looter_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 43);
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    group.looter_guid = leader;
    let sequence_before = group.sequence_num;

    assert!(!group.update_looter_guid_like_cpp([leader, member], true));

    assert_eq!(group.looter_guid, leader);
    assert_eq!(group.sequence_num, sequence_before);
}

#[test]
fn update_looter_guid_rotates_to_next_eligible_member_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let first = ObjectGuid::create_player(1, 43);
    let second = ObjectGuid::create_player(1, 44);
    let mut group = GroupInfo::new(leader);
    group.add_member(first);
    group.add_member(second);
    group.looter_guid = leader;
    let sequence_before = group.sequence_num;

    assert!(group.update_looter_guid_like_cpp([second], false));

    assert_eq!(group.looter_guid, second);
    assert_eq!(group.sequence_num, sequence_before + 1);
}

#[test]
fn update_looter_guid_wraps_without_updating_when_only_current_is_eligible_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 43);
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    group.looter_guid = member;
    let sequence_before = group.sequence_num;

    assert!(!group.update_looter_guid_like_cpp([member], false));

    assert_eq!(group.looter_guid, member);
    assert_eq!(group.sequence_num, sequence_before);
}

#[test]
fn update_looter_guid_clears_when_no_member_is_eligible_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 43);
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    group.looter_guid = member;
    let sequence_before = group.sequence_num;

    assert!(group.update_looter_guid_like_cpp([], false));

    assert_eq!(group.looter_guid, ObjectGuid::EMPTY);
    assert_eq!(group.sequence_num, sequence_before + 1);
}

#[test]
fn load_group_from_db_row_preserves_target_icons_and_validates_difficulties_like_cpp() {
    let difficulty_store = DifficultyStore::from_entries([
        wow_data::DifficultyEntry {
            id: 2,
            instance_type: 1,
            flags: wow_constants::shared::DifficultyFlags::CAN_SELECT.bits(),
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        },
        wow_data::DifficultyEntry {
            id: 15,
            instance_type: 2,
            flags: wow_constants::shared::DifficultyFlags::CAN_SELECT.bits(),
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        },
        wow_data::DifficultyEntry {
            id: 3,
            instance_type: 2,
            flags: (wow_constants::shared::DifficultyFlags::CAN_SELECT
                | wow_constants::shared::DifficultyFlags::LEGACY)
                .bits(),
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        },
    ]);
    let mut target_icons = [EMPTY_TARGET_ICON_RAW_LIKE_CPP; TARGET_ICONS_COUNT_LIKE_CPP];
    target_icons[0] = [1; 16];
    target_icons[7] = [8; 16];

    let group = GroupInfo::load_group_from_db_row_validated_like_cpp(
        906,
        GroupDbRowLikeCpp {
            leader_guid_low: 42,
            loot_method: 3,
            looter_guid_low: 77,
            loot_threshold: 4,
            target_icons,
            group_flags: GROUP_FLAG_RAID_LIKE_CPP,
            dungeon_difficulty_id: 15,
            raid_difficulty_id: 3,
            legacy_raid_difficulty_id: 15,
            master_looter_guid_low: 88,
            db_store_id: 23,
            lfg_dungeon_id: Some(100),
            lfg_state: Some(2),
        },
        Some(GroupMemberCharacterLikeCpp {
            name: "Leader".to_string(),
            race: 1,
            class: 1,
        }),
        &difficulty_store,
    )
    .expect("valid leader projection should hydrate represented group row");

    assert_eq!(group.group_guid, 906);
    assert_eq!(group.db_store_id, 23);
    assert_eq!(group.leader_guid, ObjectGuid::create_player(1, 42));
    assert_eq!(group.loot_method, 3);
    assert_eq!(group.looter_guid, ObjectGuid::create_player(1, 77));
    assert_eq!(group.loot_threshold, 4);
    assert_eq!(group.group_flags, GROUP_FLAG_RAID_LIKE_CPP);
    assert_eq!(group.dungeon_difficulty_id, DIFFICULTY_NORMAL_LIKE_CPP);
    assert_eq!(group.raid_difficulty_id, DIFFICULTY_NORMAL_RAID_LIKE_CPP);
    assert_eq!(group.legacy_raid_difficulty_id, DIFFICULTY_10_N_LIKE_CPP);
    assert_eq!(group.master_looter_guid, ObjectGuid::create_player(1, 88));
    assert_eq!(group.target_icons[0], [1; 16]);
    assert_eq!(group.target_icons[7], [8; 16]);
    assert_eq!(group.lfg_db_state, None);
}

#[test]
fn load_group_from_db_row_skips_missing_leader_character_like_cpp_cleanup_boundary() {
    let difficulty_store = DifficultyStore::from_entries([]);
    let group = GroupInfo::load_group_from_db_row_validated_like_cpp(
        907,
        GroupDbRowLikeCpp {
            leader_guid_low: 42,
            loot_method: LOOT_METHOD_PERSONAL_LIKE_CPP,
            looter_guid_low: 42,
            loot_threshold: ITEM_QUALITY_UNCOMMON_LIKE_CPP,
            target_icons: [EMPTY_TARGET_ICON_RAW_LIKE_CPP; TARGET_ICONS_COUNT_LIKE_CPP],
            group_flags: 0,
            dungeon_difficulty_id: DIFFICULTY_NORMAL_LIKE_CPP,
            raid_difficulty_id: DIFFICULTY_NORMAL_RAID_LIKE_CPP,
            legacy_raid_difficulty_id: DIFFICULTY_10_N_LIKE_CPP,
            master_looter_guid_low: 0,
            db_store_id: 24,
            lfg_dungeon_id: None,
            lfg_state: None,
        },
        None,
        &difficulty_store,
    );

    assert!(group.is_none());
}

#[test]
fn load_group_from_db_row_restores_lfg_dungeon_and_dungeon_state_like_cpp() {
    let difficulty_store = DifficultyStore::from_entries([]);
    let group = GroupInfo::load_group_from_db_row_validated_like_cpp(
        908,
        GroupDbRowLikeCpp {
            leader_guid_low: 42,
            loot_method: LOOT_METHOD_PERSONAL_LIKE_CPP,
            looter_guid_low: 42,
            loot_threshold: ITEM_QUALITY_UNCOMMON_LIKE_CPP,
            target_icons: [EMPTY_TARGET_ICON_RAW_LIKE_CPP; TARGET_ICONS_COUNT_LIKE_CPP],
            group_flags: GROUP_FLAG_LFG_LIKE_CPP,
            dungeon_difficulty_id: DIFFICULTY_NORMAL_LIKE_CPP,
            raid_difficulty_id: DIFFICULTY_NORMAL_RAID_LIKE_CPP,
            legacy_raid_difficulty_id: DIFFICULTY_10_N_LIKE_CPP,
            master_looter_guid_low: 0,
            db_store_id: 25,
            lfg_dungeon_id: Some(123),
            lfg_state: Some(LFG_STATE_DUNGEON_LIKE_CPP),
        },
        Some(GroupMemberCharacterLikeCpp {
            name: "Leader".to_string(),
            race: 1,
            class: 1,
        }),
        &difficulty_store,
    )
    .expect("valid LFG group row should hydrate");

    assert_eq!(
        group.lfg_db_state,
        Some(GroupLfgDbStateLikeCpp {
            dungeon_id: 123,
            state: Some(LFG_STATE_DUNGEON_LIKE_CPP),
        })
    );
}

#[test]
fn load_group_from_db_row_preserves_lfg_dungeon_without_unsupported_state_like_cpp() {
    let difficulty_store = DifficultyStore::from_entries([]);
    let group = GroupInfo::load_group_from_db_row_validated_like_cpp(
        909,
        GroupDbRowLikeCpp {
            leader_guid_low: 42,
            loot_method: LOOT_METHOD_PERSONAL_LIKE_CPP,
            looter_guid_low: 42,
            loot_threshold: ITEM_QUALITY_UNCOMMON_LIKE_CPP,
            target_icons: [EMPTY_TARGET_ICON_RAW_LIKE_CPP; TARGET_ICONS_COUNT_LIKE_CPP],
            group_flags: GROUP_FLAG_LFG_LIKE_CPP,
            dungeon_difficulty_id: DIFFICULTY_NORMAL_LIKE_CPP,
            raid_difficulty_id: DIFFICULTY_NORMAL_RAID_LIKE_CPP,
            legacy_raid_difficulty_id: DIFFICULTY_10_N_LIKE_CPP,
            master_looter_guid_low: 0,
            db_store_id: 26,
            lfg_dungeon_id: Some(124),
            lfg_state: Some(2),
        },
        Some(GroupMemberCharacterLikeCpp {
            name: "Leader".to_string(),
            race: 1,
            class: 1,
        }),
        &difficulty_store,
    )
    .expect("valid LFG group row should hydrate");

    assert_eq!(
        group.lfg_db_state,
        Some(GroupLfgDbStateLikeCpp {
            dungeon_id: 124,
            state: None,
        })
    );
}

#[test]
fn load_group_from_db_row_ignores_lfg_columns_when_group_is_not_lfg_like_cpp() {
    let difficulty_store = DifficultyStore::from_entries([]);
    let group = GroupInfo::load_group_from_db_row_validated_like_cpp(
        910,
        GroupDbRowLikeCpp {
            leader_guid_low: 42,
            loot_method: LOOT_METHOD_PERSONAL_LIKE_CPP,
            looter_guid_low: 42,
            loot_threshold: ITEM_QUALITY_UNCOMMON_LIKE_CPP,
            target_icons: [EMPTY_TARGET_ICON_RAW_LIKE_CPP; TARGET_ICONS_COUNT_LIKE_CPP],
            group_flags: 0,
            dungeon_difficulty_id: DIFFICULTY_NORMAL_LIKE_CPP,
            raid_difficulty_id: DIFFICULTY_NORMAL_RAID_LIKE_CPP,
            legacy_raid_difficulty_id: DIFFICULTY_10_N_LIKE_CPP,
            master_looter_guid_low: 0,
            db_store_id: 27,
            lfg_dungeon_id: Some(125),
            lfg_state: Some(LFG_STATE_FINISHED_DUNGEON_LIKE_CPP),
        },
        Some(GroupMemberCharacterLikeCpp {
            name: "Leader".to_string(),
            race: 1,
            class: 1,
        }),
        &difficulty_store,
    )
    .expect("valid non-LFG group row should hydrate");

    assert_eq!(group.lfg_db_state, None);
}

#[test]
fn set_group_member_flag_maintank_is_unique_and_preserves_assistant_bit_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let old_tank = ObjectGuid::create_player(1, 43);
    let new_tank = ObjectGuid::create_player(1, 44);
    let mut group = GroupInfo::new(leader);
    group.add_member(old_tank);
    group.add_member(new_tank);
    group.convert_to_raid_like_cpp();
    group
        .set_group_member_flag_like_cpp(old_tank, true, MEMBER_FLAG_MAINTANK_LIKE_CPP)
        .unwrap();
    group
        .set_group_member_flag_like_cpp(new_tank, true, MEMBER_FLAG_ASSISTANT_LIKE_CPP)
        .unwrap();
    let sequence_before = group.sequence_num;

    let updates = group
        .set_group_member_flag_updates_like_cpp(new_tank, true, MEMBER_FLAG_MAINTANK_LIKE_CPP)
        .unwrap();

    assert_eq!(updates.len(), 1);
    assert!(!updates.iter().any(|(guid, _)| *guid == old_tank));
    assert_eq!(
        updates,
        vec![(
            new_tank,
            MEMBER_FLAG_ASSISTANT_LIKE_CPP | MEMBER_FLAG_MAINTANK_LIKE_CPP
        )]
    );
    assert_eq!(
        group.member_slot_like_cpp(old_tank).unwrap().flags & MEMBER_FLAG_MAINTANK_LIKE_CPP,
        0
    );
    assert_eq!(
        group.member_slot_like_cpp(new_tank).unwrap().flags,
        MEMBER_FLAG_ASSISTANT_LIKE_CPP | MEMBER_FLAG_MAINTANK_LIKE_CPP
    );
    assert!(group.sequence_num > sequence_before);
}

#[test]
fn remove_unique_group_member_flag_clears_only_live_state_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let old_assist = ObjectGuid::create_player(1, 43);
    let other = ObjectGuid::create_player(1, 44);
    let mut group = GroupInfo::new(leader);
    group.add_member(old_assist);
    group.add_member(other);
    group.convert_to_raid_like_cpp();
    group
        .set_group_member_flag_like_cpp(old_assist, true, MEMBER_FLAG_MAINASSIST_LIKE_CPP)
        .unwrap();
    group
        .set_group_member_flag_like_cpp(other, true, MEMBER_FLAG_ASSISTANT_LIKE_CPP)
        .unwrap();
    let sequence_before = group.sequence_num;

    assert!(group.remove_unique_group_member_flag_like_cpp(MEMBER_FLAG_MAINASSIST_LIKE_CPP));

    assert_eq!(
        group.member_slot_like_cpp(old_assist).unwrap().flags & MEMBER_FLAG_MAINASSIST_LIKE_CPP,
        0
    );
    assert_eq!(
        group.member_slot_like_cpp(other).unwrap().flags,
        MEMBER_FLAG_ASSISTANT_LIKE_CPP
    );
    assert!(group.sequence_num > sequence_before);
    assert!(!group.remove_unique_group_member_flag_like_cpp(MEMBER_FLAG_ASSISTANT_LIKE_CPP));
}

#[test]
fn set_group_member_flag_rejects_non_raid_and_missing_target_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 43);
    let missing = ObjectGuid::create_player(1, 44);
    let mut group = GroupInfo::new(leader);
    group.add_member(member);

    assert_eq!(
        group.set_group_member_flag_updates_like_cpp(member, true, MEMBER_FLAG_MAINASSIST_LIKE_CPP),
        None
    );
    group.convert_to_raid_like_cpp();
    assert_eq!(
        group.set_group_member_flag_updates_like_cpp(
            missing,
            true,
            MEMBER_FLAG_MAINASSIST_LIKE_CPP
        ),
        None
    );
    assert_eq!(group.member_slot_like_cpp(member).unwrap().flags, 0);
}

#[test]
fn ready_check_start_marks_offline_starter_and_preserves_cpp_event_order() {
    let leader = ObjectGuid::create_player(1, 42);
    let offline = ObjectGuid::create_player(1, 43);
    let mut group = GroupInfo::new(leader);
    group.add_member(offline);

    let events = group.start_ready_check_like_cpp(leader, [leader]);

    assert_eq!(group.ready_check_timer_ms, 0);
    assert!(!group.ready_check_started);
    assert!(group.member_slots.iter().all(|slot| !slot.ready_checked));
    assert_eq!(
        events,
        vec![
            ReadyCheckEventLikeCpp::Response {
                party_guid: group.group_guid,
                player: offline,
                is_ready: false,
            },
            ReadyCheckEventLikeCpp::Completed {
                party_index: 0,
                party_guid: group.group_guid,
            },
            ReadyCheckEventLikeCpp::Started {
                party_index: 0,
                party_guid: group.group_guid,
                initiator_guid: leader,
                duration_ms: READYCHECK_DURATION_MS_LIKE_CPP,
            },
        ]
    );
}

#[test]
fn ready_check_response_before_started_is_cpp_noop() {
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 43);
    let mut group = GroupInfo::new(leader);
    group.add_member(member);

    let events = group.set_member_ready_check_like_cpp(member, true);

    assert!(events.is_empty());
    assert!(!group.member_slot_like_cpp(member).unwrap().ready_checked);
    assert!(!group.ready_check_started);
}

#[test]
fn ready_check_member_response_broadcasts_and_completes_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 43);
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    let start_events = group.start_ready_check_like_cpp(leader, [leader, member]);

    assert_eq!(start_events.len(), 1);
    assert!(group.ready_check_started);
    assert!(group.member_slot_like_cpp(leader).unwrap().ready_checked);
    assert!(!group.member_slot_like_cpp(member).unwrap().ready_checked);

    let events = group.set_member_ready_check_like_cpp(member, true);

    assert_eq!(
        events,
        vec![
            ReadyCheckEventLikeCpp::Response {
                party_guid: group.group_guid,
                player: member,
                is_ready: true,
            },
            ReadyCheckEventLikeCpp::Completed {
                party_index: 0,
                party_guid: group.group_guid,
            },
        ]
    );
    assert!(!group.ready_check_started);
    assert_eq!(group.ready_check_timer_ms, 0);
    assert!(group.member_slots.iter().all(|slot| !slot.ready_checked));
}

#[test]
fn load_groups_from_db_rows_registers_groups_and_members_like_cpp() {
    let registry = GroupRegistry::default();
    let difficulty_store = DifficultyStore::from_entries([]);
    let mut character_cache = BTreeMap::new();
    character_cache.insert(
        5001,
        GroupMemberCharacterLikeCpp {
            name: "Leader".to_string(),
            race: 1,
            class: 2,
        },
    );
    character_cache.insert(
        5002,
        GroupMemberCharacterLikeCpp {
            name: "Member".to_string(),
            race: 3,
            class: 4,
        },
    );

    let summary = load_groups_from_db_rows_like_cpp(
        &registry,
        [GroupDbRowLikeCpp {
            leader_guid_low: 5001,
            loot_method: 3,
            looter_guid_low: 5001,
            loot_threshold: 4,
            target_icons: [EMPTY_TARGET_ICON_RAW_LIKE_CPP; TARGET_ICONS_COUNT_LIKE_CPP],
            group_flags: GROUP_FLAG_RAID_LIKE_CPP,
            dungeon_difficulty_id: DIFFICULTY_NORMAL_LIKE_CPP,
            raid_difficulty_id: DIFFICULTY_NORMAL_RAID_LIKE_CPP,
            legacy_raid_difficulty_id: DIFFICULTY_10_N_LIKE_CPP,
            master_looter_guid_low: 0,
            db_store_id: 5501,
            lfg_dungeon_id: None,
            lfg_state: None,
        }],
        [
            GroupMemberDbRowLikeCpp {
                db_store_id: 5501,
                member_guid_low: 5001,
                member_flags: 0,
                subgroup: 0,
                roles: 1,
            },
            GroupMemberDbRowLikeCpp {
                db_store_id: 5501,
                member_guid_low: 5002,
                member_flags: 0x04,
                subgroup: 2,
                roles: 3,
            },
        ],
        &character_cache,
        &difficulty_store,
    );

    assert_eq!(
        summary,
        GroupLoadSummaryLikeCpp {
            loaded_groups: 1,
            loaded_member_rows: 2,
            loaded_members: 2,
            skipped_group_rows: 0,
            skipped_member_rows: 0,
        }
    );

    let group = get_group_by_db_store_id_like_cpp(&registry, 5501)
        .expect("loaded group should be registered by DB-store id");
    assert_eq!(group.db_store_id, 5501);
    assert_eq!(group.members.len(), 2);
    let slot = group
        .member_slot_like_cpp(ObjectGuid::create_player(1, 5002))
        .expect("loaded member row should preserve its slot");
    assert_eq!(slot.name, "Member");
    assert_eq!(slot.subgroup, 2);
    assert_eq!(slot.flags, 0x04);
    assert_eq!(slot.roles, 3);
}

#[test]
fn load_groups_from_db_rows_skips_missing_character_cache_rows_like_cpp_boundary() {
    let registry = GroupRegistry::default();
    let difficulty_store = DifficultyStore::from_entries([]);
    let mut character_cache = BTreeMap::new();
    character_cache.insert(
        5101,
        GroupMemberCharacterLikeCpp {
            name: "Leader".to_string(),
            race: 1,
            class: 1,
        },
    );

    let summary = load_groups_from_db_rows_like_cpp(
        &registry,
        [
            GroupDbRowLikeCpp {
                leader_guid_low: 5101,
                loot_method: LOOT_METHOD_PERSONAL_LIKE_CPP,
                looter_guid_low: 5101,
                loot_threshold: ITEM_QUALITY_UNCOMMON_LIKE_CPP,
                target_icons: [EMPTY_TARGET_ICON_RAW_LIKE_CPP; TARGET_ICONS_COUNT_LIKE_CPP],
                group_flags: 0,
                dungeon_difficulty_id: DIFFICULTY_NORMAL_LIKE_CPP,
                raid_difficulty_id: DIFFICULTY_NORMAL_RAID_LIKE_CPP,
                legacy_raid_difficulty_id: DIFFICULTY_10_N_LIKE_CPP,
                master_looter_guid_low: 0,
                db_store_id: 5601,
                lfg_dungeon_id: None,
                lfg_state: None,
            },
            GroupDbRowLikeCpp {
                leader_guid_low: 999_999,
                loot_method: LOOT_METHOD_PERSONAL_LIKE_CPP,
                looter_guid_low: 999_999,
                loot_threshold: ITEM_QUALITY_UNCOMMON_LIKE_CPP,
                target_icons: [EMPTY_TARGET_ICON_RAW_LIKE_CPP; TARGET_ICONS_COUNT_LIKE_CPP],
                group_flags: 0,
                dungeon_difficulty_id: DIFFICULTY_NORMAL_LIKE_CPP,
                raid_difficulty_id: DIFFICULTY_NORMAL_RAID_LIKE_CPP,
                legacy_raid_difficulty_id: DIFFICULTY_10_N_LIKE_CPP,
                master_looter_guid_low: 0,
                db_store_id: 5602,
                lfg_dungeon_id: None,
                lfg_state: None,
            },
        ],
        [
            GroupMemberDbRowLikeCpp {
                db_store_id: 5601,
                member_guid_low: 5102,
                member_flags: 0,
                subgroup: 0,
                roles: 0,
            },
            GroupMemberDbRowLikeCpp {
                db_store_id: 888_888,
                member_guid_low: 5101,
                member_flags: 0,
                subgroup: 0,
                roles: 0,
            },
        ],
        &character_cache,
        &difficulty_store,
    );

    assert_eq!(summary.loaded_groups, 1);
    assert_eq!(summary.skipped_group_rows, 1);
    assert_eq!(summary.loaded_member_rows, 2);
    assert_eq!(summary.loaded_members, 0);
    assert_eq!(summary.skipped_member_rows, 2);
    assert!(get_group_by_db_store_id_like_cpp(&registry, 5601).is_some());
    assert!(get_group_by_db_store_id_like_cpp(&registry, 5602).is_none());
}

#[test]
fn load_groups_from_db_rows_advances_next_storage_id_for_ordered_rows_like_cpp() {
    let registry = GroupRegistry::default();
    let difficulty_store = DifficultyStore::from_entries([]);
    let mut character_cache = BTreeMap::new();
    for guid_low in [900_001, 900_002] {
        character_cache.insert(
            guid_low,
            GroupMemberCharacterLikeCpp {
                name: format!("Leader{guid_low}"),
                race: 1,
                class: 1,
            },
        );
    }

    let _allocator_guard = GROUP_DB_STORE_ID_ALLOCATOR_LOCK.lock().unwrap();
    NEXT_GROUP_DB_STORE_ID.store(900_001, Ordering::Relaxed);
    let summary = load_groups_from_db_rows_like_cpp(
        &registry,
        [
            GroupDbRowLikeCpp {
                leader_guid_low: 900_001,
                loot_method: LOOT_METHOD_PERSONAL_LIKE_CPP,
                looter_guid_low: 900_001,
                loot_threshold: ITEM_QUALITY_UNCOMMON_LIKE_CPP,
                target_icons: [EMPTY_TARGET_ICON_RAW_LIKE_CPP; TARGET_ICONS_COUNT_LIKE_CPP],
                group_flags: 0,
                dungeon_difficulty_id: DIFFICULTY_NORMAL_LIKE_CPP,
                raid_difficulty_id: DIFFICULTY_NORMAL_RAID_LIKE_CPP,
                legacy_raid_difficulty_id: DIFFICULTY_10_N_LIKE_CPP,
                master_looter_guid_low: 0,
                db_store_id: 900_001,
                lfg_dungeon_id: None,
                lfg_state: None,
            },
            GroupDbRowLikeCpp {
                leader_guid_low: 900_002,
                loot_method: LOOT_METHOD_PERSONAL_LIKE_CPP,
                looter_guid_low: 900_002,
                loot_threshold: ITEM_QUALITY_UNCOMMON_LIKE_CPP,
                target_icons: [EMPTY_TARGET_ICON_RAW_LIKE_CPP; TARGET_ICONS_COUNT_LIKE_CPP],
                group_flags: 0,
                dungeon_difficulty_id: DIFFICULTY_NORMAL_LIKE_CPP,
                raid_difficulty_id: DIFFICULTY_NORMAL_RAID_LIKE_CPP,
                legacy_raid_difficulty_id: DIFFICULTY_10_N_LIKE_CPP,
                master_looter_guid_low: 0,
                db_store_id: 900_002,
                lfg_dungeon_id: None,
                lfg_state: None,
            },
        ],
        [],
        &character_cache,
        &difficulty_store,
    );

    assert_eq!(summary.loaded_groups, 2);
    assert_eq!(NEXT_GROUP_DB_STORE_ID.load(Ordering::Relaxed), 900_003);
}

// ── Ready-check tick tests ──────────────────────────────────────────

#[test]
fn update_ready_check_noop_when_not_started() {
    let leader = ObjectGuid::create_player(1, 42);
    let mut group = GroupInfo::new(leader);
    assert!(!group.ready_check_started);

    let events = group.update_ready_check_like_cpp(500);
    assert!(events.is_empty());
    assert!(!group.ready_check_started);
    assert_eq!(group.ready_check_timer_ms, 0);
}

#[test]
fn update_ready_check_decrements_without_completing_when_time_remains() {
    let leader = ObjectGuid::create_player(1, 42);
    let mut group = GroupInfo::new(leader);
    group.ready_check_started = true;
    group.ready_check_timer_ms = READYCHECK_DURATION_MS_LIKE_CPP;

    // Tick 1000ms — timer should go from 35000 to 34000, no events.
    let events = group.update_ready_check_like_cpp(1_000);
    assert!(events.is_empty());
    assert!(group.ready_check_started);
    assert_eq!(
        group.ready_check_timer_ms,
        READYCHECK_DURATION_MS_LIKE_CPP - 1_000
    );

    // Tick another 1000ms
    let events = group.update_ready_check_like_cpp(1_000);
    assert!(events.is_empty());
    assert!(group.ready_check_started);
    assert_eq!(
        group.ready_check_timer_ms,
        READYCHECK_DURATION_MS_LIKE_CPP - 2_000
    );
}

#[test]
fn update_ready_check_expires_and_resets_all_state() {
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 99);
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    group.ready_check_started = true;
    group.ready_check_timer_ms = READYCHECK_DURATION_MS_LIKE_CPP;
    // Simulate some members already responded.
    for slot in &mut group.member_slots {
        slot.ready_checked = true;
    }

    // Tick more than remaining — should expire.
    let events = group.update_ready_check_like_cpp(36_000);
    assert_eq!(events.len(), 1);
    match events[0] {
        ReadyCheckEventLikeCpp::Completed {
            party_index,
            party_guid,
        } => {
            assert_eq!(party_index, 0);
            assert_eq!(party_guid, group.group_guid);
        }
        _ => panic!("expected Completed event"),
    }
    assert!(!group.ready_check_started);
    assert_eq!(group.ready_check_timer_ms, 0);
    // All members should have been reset.
    assert!(group.member_slots.iter().all(|s| !s.ready_checked));
}

#[test]
fn update_ready_check_exact_zero_expires() {
    let leader = ObjectGuid::create_player(1, 42);
    let mut group = GroupInfo::new(leader);
    group.ready_check_started = true;
    group.ready_check_timer_ms = 500;

    let events = group.update_ready_check_like_cpp(500);
    assert_eq!(events.len(), 1);
    assert!(!group.ready_check_started);
    assert_eq!(group.ready_check_timer_ms, 0);
}

#[test]
fn registry_invalid_subgroup_transition_has_no_partial_state() {
    let registry = GroupRegistry::new();
    let leader = ObjectGuid::create_player(1, 71);
    let member = ObjectGuid::create_player(1, 72);
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    group.convert_to_raid_like_cpp();
    let group_guid = group.group_guid;
    let sequence = group.sequence_num;
    registry.register_group_like_cpp(group_guid, group);

    let result = registry.change_member_subgroup_like_cpp(
        group_guid,
        leader,
        member,
        MAX_RAID_SUBGROUPS_LIKE_CPP as u8,
    );

    assert!(matches!(
        result,
        Err(GroupAuthorityErrorLikeCpp::InvalidSubgroup)
    ));
    let group = registry.get(&group_guid).expect("group remains registered");
    assert_eq!(group.sequence_num, sequence);
    assert_eq!(group.member_slot_like_cpp(member).unwrap().subgroup, 0);
}

#[test]
fn registry_missing_member_flag_transition_has_no_partial_state() {
    let registry = GroupRegistry::new();
    let leader = ObjectGuid::create_player(1, 81);
    let missing = ObjectGuid::create_player(1, 82);
    let mut group = GroupInfo::new(leader);
    group.convert_to_raid_like_cpp();
    let group_guid = group.group_guid;
    let sequence = group.sequence_num;
    registry.register_group_like_cpp(group_guid, group);

    let result = registry.set_member_flag_transition_like_cpp(
        group_guid,
        leader,
        missing,
        true,
        MEMBER_FLAG_ASSISTANT_LIKE_CPP,
    );

    assert!(matches!(
        result,
        Err(GroupAuthorityErrorLikeCpp::MissingMember)
    ));
    assert_eq!(registry.get(&group_guid).unwrap().sequence_num, sequence);
}

#[test]
fn registry_stale_ready_response_cannot_reopen_completed_check() {
    let registry = GroupRegistry::new();
    let leader = ObjectGuid::create_player(1, 91);
    let member = ObjectGuid::create_player(1, 92);
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    let group_guid = group.group_guid;
    registry.register_group_like_cpp(group_guid, group);

    registry
        .start_ready_check_transition_like_cpp(group_guid, leader, [leader, member])
        .expect("leader starts ready check");
    registry
        .respond_ready_check_transition_like_cpp(group_guid, member, true)
        .expect("final member completes ready check");
    let stale = registry.respond_ready_check_transition_like_cpp(group_guid, member, false);

    assert!(matches!(stale, Err(GroupAuthorityErrorLikeCpp::NoChange)));
    let group = registry.get(&group_guid).unwrap();
    assert!(!group.ready_check_started);
    assert_eq!(group.ready_check_timer_ms, 0);
    assert!(group.member_slots.iter().all(|slot| !slot.ready_checked));
}

#[test]
fn concurrent_leader_transfers_allow_only_current_leader_once() {
    let registry = std::sync::Arc::new(GroupRegistry::new());
    let leader = ObjectGuid::create_player(1, 101);
    let first = ObjectGuid::create_player(1, 102);
    let second = ObjectGuid::create_player(1, 103);
    let mut group = GroupInfo::new(leader);
    group.add_member(first);
    group.add_member(second);
    let group_guid = group.group_guid;
    registry.register_group_like_cpp(group_guid, group);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));

    let handles = [first, second].map(|candidate| {
        let registry = std::sync::Arc::clone(&registry);
        let barrier = std::sync::Arc::clone(&barrier);
        std::thread::spawn(move || {
            barrier.wait();
            registry.change_leader_transition_like_cpp(group_guid, leader, candidate)
        })
    });
    barrier.wait();
    let results = handles.map(|handle| handle.join().unwrap());

    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, Err(GroupAuthorityErrorLikeCpp::NotLeader)))
            .count(),
        1
    );
    assert!([first, second].contains(&registry.get(&group_guid).unwrap().leader_guid));
}

#[test]
fn concurrent_kicks_remove_each_member_once_and_disband_once() {
    let registry = std::sync::Arc::new(GroupRegistry::new());
    let leader = ObjectGuid::create_player(1, 111);
    let first = ObjectGuid::create_player(1, 112);
    let second = ObjectGuid::create_player(1, 113);
    let mut group = GroupInfo::new(leader);
    group.add_member(first);
    group.add_member(second);
    let group_guid = group.group_guid;
    registry.register_group_like_cpp(group_guid, group);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));

    let handles = [first, second].map(|target| {
        let registry = std::sync::Arc::clone(&registry);
        let barrier = std::sync::Arc::clone(&barrier);
        std::thread::spawn(move || {
            barrier.wait();
            registry.remove_member_like_cpp(
                group_guid,
                target,
                GroupMemberRemovalKindLikeCpp::Kick {
                    actor_guid: leader,
                    actor_in_battleground: false,
                    target_has_loot_rolls: false,
                    any_member_in_actor_map_combat: false,
                },
                &[],
            )
        })
    });
    barrier.wait();
    let results = handles.map(|handle| handle.join().unwrap().unwrap());

    assert_eq!(
        results
            .iter()
            .filter(|outcome| outcome.facts.disbanded)
            .count(),
        1
    );
    assert!(!registry.contains_key(&group_guid));
}

#[test]
fn removal_outcome_preserves_cpp_persistence_order_like_cpp() {
    let registry = GroupRegistry::new();
    let leader = ObjectGuid::create_player(1, 114);
    let first = ObjectGuid::create_player(1, 115);
    let second = ObjectGuid::create_player(1, 116);
    let mut group = GroupInfo::new(leader);
    group.add_member(first);
    group.add_member(second);
    let group_guid = group.group_guid;
    let db_store_id = group.db_store_id;
    registry.register_group_like_cpp(group_guid, group);

    let leave = registry
        .remove_member_like_cpp(
            group_guid,
            leader,
            GroupMemberRemovalKindLikeCpp::Leave,
            &[second, first],
        )
        .unwrap();
    assert_eq!(
        leave.persistence,
        vec![
            GroupPersistenceIntentLikeCpp::DeleteMember {
                member_guid: leader,
            },
            GroupPersistenceIntentLikeCpp::UpdateLeader {
                db_store_id,
                leader_guid: second,
            },
        ]
    );

    let disband = registry
        .remove_member_like_cpp(
            group_guid,
            first,
            GroupMemberRemovalKindLikeCpp::Kick {
                actor_guid: second,
                actor_in_battleground: false,
                target_has_loot_rolls: false,
                any_member_in_actor_map_combat: false,
            },
            &[],
        )
        .unwrap();
    assert!(disband.facts.disbanded);
    assert_eq!(
        disband.persistence,
        vec![
            GroupPersistenceIntentLikeCpp::DeleteGroup { db_store_id },
            GroupPersistenceIntentLikeCpp::DeleteAllMembers { db_store_id },
            GroupPersistenceIntentLikeCpp::DeleteLfgData { db_store_id },
        ]
    );
}

#[test]
fn invite_acceptance_emits_creation_persistence_before_publication_like_cpp() {
    let registry = GroupRegistry::new();
    let pending = PendingInvites::default();
    let leader = ObjectGuid::create_player(1, 117);
    let invitee = ObjectGuid::create_player(1, 118);
    pending.seed_invite_like_cpp(
        invitee,
        PendingInviteLikeCpp::new_pending_group(leader, GROUP_CATEGORY_HOME_LIKE_CPP),
    );

    let AcceptGroupInviteResultLikeCpp::Created {
        group,
        subgroup,
        persistence,
    } = registry.accept_invite_like_cpp(&pending, invitee, None, Some(leader))
    else {
        panic!("expected represented group creation");
    };

    assert_eq!(persistence.len(), 3);
    assert!(matches!(
        persistence[0],
        GroupPersistenceIntentLikeCpp::InsertGroup { db_store_id, .. }
            if db_store_id == group.db_store_id
    ));
    assert_eq!(
        persistence[1],
        GroupPersistenceIntentLikeCpp::InsertMember {
            db_store_id: group.db_store_id,
            member_guid: leader,
            member_flags: 0,
            subgroup: 0,
            roles: 0,
        }
    );
    assert_eq!(
        persistence[2],
        GroupPersistenceIntentLikeCpp::InsertMember {
            db_store_id: group.db_store_id,
            member_guid: invitee,
            member_flags: 0,
            subgroup,
            roles: 0,
        }
    );
}

#[test]
#[should_panic(expected = "group registry key must match group identity")]
fn registry_rejects_mismatched_materialized_identity() {
    let registry = GroupRegistry::new();
    let group = GroupInfo::new(ObjectGuid::create_player(1, 119));
    registry.register_group_like_cpp(group.group_guid + 1, group);
}

#[test]
fn instance_transition_outcomes_are_owned_and_do_not_hold_group_guard() {
    let registry = GroupRegistry::new();
    let leader = ObjectGuid::create_player(1, 121);
    let group = GroupInfo::new(leader);
    let group_guid = group.group_guid;
    registry.register_group_like_cpp(group_guid, group);

    let recent = registry
        .set_recent_instance_transition_like_cpp(group_guid, 631, leader, 9001)
        .unwrap();
    let linked = registry
        .link_owned_instance_transition_like_cpp(group_guid, 631, 9001)
        .unwrap();
    let reset = registry
        .apply_instance_reset_transition_like_cpp(
            group_guid,
            631,
            GroupInstanceResetResultLikeCpp::Success,
            GroupInstanceResetMethodLikeCpp::Manual,
        )
        .unwrap();

    assert_eq!(recent.group.recent_instance_id_like_cpp(631), 9001);
    assert!(linked.facts);
    assert!(reset.facts);
    assert_eq!(reset.group.recent_instance_id_like_cpp(631), 0);
    assert!(
        reset
            .group
            .owned_instances_like_cpp()
            .any(|instance| instance.instance_id == 9001)
    );
}
