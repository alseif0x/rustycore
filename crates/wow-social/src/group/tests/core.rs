// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;

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
#[should_panic(expected = "group registry key must match group identity")]
fn registry_rejects_mismatched_materialized_identity() {
    let registry = GroupRegistry::new();
    let group = GroupInfo::new(ObjectGuid::create_player(1, 119));
    registry.register_group_like_cpp(group.group_guid + 1, group);
}

