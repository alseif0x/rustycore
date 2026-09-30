// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_core::ObjectGuid;

use super::*;

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
