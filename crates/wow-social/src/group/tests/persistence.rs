// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::collections::BTreeMap;
use std::sync::atomic::Ordering;

use wow_core::ObjectGuid;

use super::*;

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

