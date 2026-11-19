// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_core::ObjectGuid;

use super::super::*;

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
