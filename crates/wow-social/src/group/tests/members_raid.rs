// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_core::ObjectGuid;

use super::*;

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
