//! Session scenarios exercising the represented login responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn logout_closes_old_money_tracker_and_character_select_rotates_fresh_tracker() {
    let (mut session, _, _) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 70_001)));
    let old_tracker = session.durable_loot_money_persistence_tracker_like_cpp();

    session.set_player_logout_like_cpp(true);
    assert!(old_tracker.begin_like_cpp().is_err());
    session.set_player_guid(None);
    let fresh_tracker = session.durable_loot_money_persistence_tracker_like_cpp();

    assert!(!Arc::ptr_eq(&old_tracker, &fresh_tracker));
    assert!(old_tracker.begin_like_cpp().is_err());
    assert!(fresh_tracker.begin_like_cpp().is_ok());
}

#[test]
fn represented_player_condition_context_uses_live_session_state_like_cpp() {
    let (mut session, _, _) = make_session();
    session.player_race = 1;
    session.player_class = 2;
    session.player_gender = 1;
    session.set_known_spells_like_cpp(vec![635, -1, 19740]);
    session
        .set_player_skill_values_like_cpp(HashMap::from([(SKILL_RIDING_LIKE_CPP, 75), (333, 125)]));
    session.player_currencies.insert(
        81,
        PlayerCurrency {
            state: PlayerCurrencyState::Unchanged,
            quantity: 25,
            weekly_quantity: 0,
            tracked_quantity: 0,
            increased_cap_quantity: 0,
            earned_quantity: 0,
            flags: 0,
        },
    );
    session.quest_test_fixture_like_cpp.player_quests.insert(
        100,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id: 100,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![],
            slot: 0,
        },
    );
    session.quest_test_fixture_like_cpp.player_quests.insert(
        101,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id: 101,
            status: crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![],
            slot: 0,
        },
    );
    session
        .quest_test_fixture_like_cpp
        .rewarded_quests
        .insert(200);
    session.set_player_zone_area_like_cpp(12, 34);

    let owned = session
        .represented_player_condition_context_like_cpp()
        .expect("fixture canonical inventory owner");
    let context = owned
        .as_context(&session)
        .expect("test Player condition owner");

    assert_eq!(context.race, 1);
    assert_eq!(context.class_mask, 0b10);
    assert_eq!(context.gender, 1);
    assert_eq!(context.area_id, 34);
    assert_eq!(context.expansion, 2);
    assert_eq!(context.server_expansion, 9);
    assert_eq!(context.spells, &[635, 19740]);
    assert!(context.skills.contains(&PlayerConditionSkillLikeCpp {
        id: SKILL_RIDING_LIKE_CPP,
        value: 75,
    }));
    assert_eq!(
        session.player_skill_value_like_cpp(SKILL_RIDING_LIKE_CPP),
        75
    );
    assert_eq!(
        context.currencies,
        &[PlayerConditionCountLikeCpp { id: 81, count: 25 }]
    );
    assert!(context.current_quests.contains(&100));
    assert!(context.current_quests.contains(&101));
    assert_eq!(context.complete_quests, &[101]);
    assert_eq!(context.completed_quests, &[200]);
}
#[test]
fn represented_mount_capability_for_type_uses_session_state_like_cpp() {
    let (mut session, _, _) = make_session();
    session.current_map_id = 1;
    install_canonical_player_owner_for_test(&mut session, 1, 0);
    session.set_player_zone_area_like_cpp(10, 77);
    session.set_known_spells_like_cpp(vec![456]);
    session.set_player_skill_values_like_cpp(HashMap::from([(SKILL_RIDING_LIKE_CPP, 75)]));
    session
        .apply_aura(123, ObjectGuid::EMPTY, 30_000, 0)
        .unwrap();
    session.set_area_table_store(Arc::new(wow_data::AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 10,
            continent_id: 0,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: i32::from(wow_data::AREA_MOUNT_FLAG_ALLOW_GROUND_MOUNTS),
            flags: 0,
        },
        wow_data::AreaTableEntry {
            id: 77,
            continent_id: 0,
            parent_area_id: 10,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: i32::from(wow_data::AREA_MOUNT_FLAG_ALLOW_GROUND_MOUNTS),
            flags: 0,
        },
    ])));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        wow_data::MapEntry {
            id: 1,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: 0,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.set_mount_capability_store(Arc::new(wow_data::MountCapabilityStore::from_entries([
        wow_data::MountCapabilityEntry {
            id: 10,
            flags: wow_data::MOUNT_CAPABILITY_FLAG_FLYING,
            req_riding_skill: 0,
            req_area_id: 0,
            req_spell_aura_id: 0,
            req_spell_known_id: 0,
            mod_spell_aura_id: 1000,
            req_map_id: -1,
        },
        wow_data::MountCapabilityEntry {
            id: 11,
            flags: wow_data::MOUNT_CAPABILITY_FLAG_GROUND,
            req_riding_skill: 75,
            req_area_id: 10,
            req_spell_aura_id: 123,
            req_spell_known_id: 456,
            mod_spell_aura_id: 1001,
            req_map_id: 0,
        },
    ])));
    session.set_mount_type_x_capability_store(Arc::new(
        wow_data::MountTypeXCapabilityStore::from_entries([
            wow_data::MountTypeXCapabilityEntry {
                id: 1,
                mount_type_id: 7,
                mount_capability_id: 10,
                order_index: 0,
            },
            wow_data::MountTypeXCapabilityEntry {
                id: 2,
                mount_type_id: 7,
                mount_capability_id: 11,
                order_index: 1,
            },
        ]),
    ));

    assert_eq!(
        session
            .represented_mount_capability_for_type_from_session_like_cpp(7, None)
            .map(|capability| capability.id),
        Some(11)
    );
    session.set_player_skill_values_like_cpp(HashMap::from([(SKILL_RIDING_LIKE_CPP, 74)]));
    assert!(
        session
            .represented_mount_capability_for_type_from_session_like_cpp(7, None)
            .is_none()
    );
    assert_eq!(
        session
            .represented_mount_capability_selection_for_type_like_cpp(
                7,
                u32::from(session.player_skill_value_like_cpp(SKILL_RIDING_LIKE_CPP)),
                None,
                false,
                false,
            )
            .unwrap_err(),
        wow_data::MountCapabilityRejectLikeCpp::RidingSkill
    );
    session.set_player_skill_values_like_cpp(HashMap::from([(SKILL_RIDING_LIKE_CPP, 75)]));
    session.set_known_spells_like_cpp(vec![]);
    assert_eq!(
        session
            .represented_mount_capability_selection_for_type_like_cpp(7, 75, None, false, false)
            .unwrap_err(),
        wow_data::MountCapabilityRejectLikeCpp::KnownSpell
    );
}

#[test]
fn account_heirloom_rows_filter_by_heirloom_store_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_battlenet_account_id(77);
    session.set_heirloom_store(Arc::new(HeirloomStore::from_entries([HeirloomEntry {
        id: 1,
        source_text: "known".to_string(),
        item_id: 44_000,
        legacy_upgraded_item_id: 0,
        static_upgraded_item_id: 0,
        source_type_enum: 0,
        flags: 0,
        legacy_item_id: 0,
        upgrade_item_id: [0; 6],
        upgrade_item_bonus_list_id: [10, 20, 30, 40, 50, 60],
    }])));

    session.load_represented_account_heirlooms_like_cpp([(44_000, 0x03), (44_001, 0x01)]);

    assert_eq!(
        session.account_heirloom_rows_like_cpp(),
        vec![(44_000, 0x03)]
    );
    assert_eq!(
        session.account_heirloom_save_rows_like_cpp(),
        Some(vec![AccountHeirloomSaveRowLikeCpp {
            bnet_account_id: 77,
            item_id: 44_000,
            flags: 0x03,
        }]),
        "C++ CollectionMgr::SaveAccountHeirlooms appends the battlenet account id to the LoginDatabase transaction"
    );
    assert_eq!(
        session.account_heirloom_packet_rows_like_cpp(),
        vec![AccountHeirloom {
            item_id: 44_000,
            flags: 0x03,
        }]
    );
    assert_eq!(
        session.account_heirloom_active_player_rows_like_cpp(),
        vec![(44_000, 0x03)]
    );
    assert_eq!(session.account_heirloom_bonus_like_cpp(44_000), 20);
    assert_eq!(session.account_heirloom_bonus_like_cpp(44_001), 0);
}
#[test]
fn account_heirloom_update_is_not_sent_while_opcode_is_unresolved_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_heirloom_store(Arc::new(HeirloomStore::from_entries([HeirloomEntry {
        id: 1,
        source_text: "known".to_string(),
        item_id: 44_000,
        legacy_upgraded_item_id: 0,
        static_upgraded_item_id: 0,
        source_type_enum: 0,
        flags: 0,
        legacy_item_id: 0,
        upgrade_item_id: [0; 6],
        upgrade_item_bonus_list_id: [0; 6],
    }])));
    session.load_represented_account_heirlooms_like_cpp([(44_000, 0x03)]);

    assert_eq!(session.account_heirloom_packet_rows_like_cpp().len(), 1);
    session.send_account_heirlooms_like_cpp();

    assert_eq!(
        send_rx.try_recv(),
        Err(flume::TryRecvError::Empty),
        "do not send SMSG_ACCOUNT_HEIRLOOM_UPDATE while legacy C++ keeps it at NULL_OPCODE/0xBADD"
    );
}
#[test]
fn add_account_heirloom_and_player_dynamic_fields_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 77);
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);

    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    session.mutate_canonical_player_like_cpp(|player| player.clear_data_changes());

    assert!(session.add_account_heirloom_like_cpp(44_000, 0x03));
    assert!(!session.add_account_heirloom_like_cpp(44_000, 0x04));
    let update = session
        .add_player_heirloom_dynamic_fields_like_cpp(44_000, 0x03)
        .expect("canonical current player should receive Player::AddHeirloom dynamic fields");

    assert_eq!(
        session.account_heirloom_rows_like_cpp(),
        vec![(44_000, 0x03)]
    );
    assert_eq!(session.account_heirloom_bonus_like_cpp(44_000), 0);
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| {
                (
                    player.heirlooms_like_cpp().to_vec(),
                    player.heirloom_flags_like_cpp().to_vec(),
                )
            })
            .unwrap(),
        (vec![44_000], vec![0x03])
    );
    assert!(
        update
            .active_player_data
            .as_ref()
            .unwrap()
            .mask
            .is_set(wow_entities::ACTIVE_PLAYER_DATA_HEIRLOOMS_BIT)
    );
    assert!(
        update
            .active_player_data
            .as_ref()
            .unwrap()
            .mask
            .is_set(wow_entities::ACTIVE_PLAYER_DATA_HEIRLOOM_FLAGS_BIT)
    );
}
#[test]
fn upgrade_account_heirloom_updates_flags_bonus_and_active_player_field_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 78);
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);

    session.set_heirloom_store(Arc::new(HeirloomStore::from_entries([HeirloomEntry {
        id: 1,
        source_text: "known".to_string(),
        item_id: 44_000,
        legacy_upgraded_item_id: 0,
        static_upgraded_item_id: 0,
        source_type_enum: 0,
        flags: 0,
        legacy_item_id: 0,
        upgrade_item_id: [90_001, 90_002, 0, 0, 0, 0],
        upgrade_item_bonus_list_id: [101, 202, 0, 0, 0, 0],
    }])));
    session.load_represented_account_heirlooms_like_cpp([(44_000, 0x01)]);

    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    session
        .add_player_heirloom_dynamic_fields_like_cpp(44_000, 0x01)
        .unwrap();
    session.mutate_canonical_player_like_cpp(|player| player.clear_data_changes());

    let update = session
        .upgrade_account_heirloom_like_cpp(44_000, 90_002)
        .expect("known owned heirloom should upgrade");

    assert_eq!(
        session.account_heirloom_rows_like_cpp(),
        vec![(44_000, 0x03)]
    );
    assert_eq!(session.account_heirloom_bonus_like_cpp(44_000), 202);
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| {
                (
                    player.heirlooms_like_cpp().to_vec(),
                    player.heirloom_flags_like_cpp().to_vec(),
                )
            })
            .unwrap(),
        (vec![44_000], vec![0x03])
    );
    let active_update = update.active_player_data.as_ref().unwrap();
    assert!(
        !active_update
            .mask
            .is_set(wow_entities::ACTIVE_PLAYER_DATA_HEIRLOOMS_BIT)
    );
    assert!(
        active_update
            .mask
            .is_set(wow_entities::ACTIVE_PLAYER_DATA_HEIRLOOM_FLAGS_BIT)
    );

    session.mutate_canonical_player_like_cpp(|player| player.clear_data_changes());
    assert!(
        session
            .upgrade_account_heirloom_like_cpp(44_000, 123_456)
            .is_some()
    );
    assert_eq!(
        session.account_heirloom_rows_like_cpp(),
        vec![(44_000, 0x03)]
    );
    assert_eq!(session.account_heirloom_bonus_like_cpp(44_000), 0);
}
