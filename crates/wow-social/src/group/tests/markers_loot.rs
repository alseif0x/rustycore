// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_core::{ObjectGuid, Position};

use super::*;

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
