// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_core::ObjectGuid;

use super::super::*;

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
