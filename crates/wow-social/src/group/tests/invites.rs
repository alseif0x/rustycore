// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_core::ObjectGuid;

use super::*;

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

