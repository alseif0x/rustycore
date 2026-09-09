//! Session scenarios exercising the represented login responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn character_login_claim_allows_only_one_live_session_like_cpp() {
    let guid = ObjectGuid::create_player(1, 0x7FFF_FF01);
    let (mut first, _, _) = make_session();
    let (mut second, _, _) = make_session();

    assert!(first.try_claim_character_login_like_cpp(guid));
    assert!(first.try_claim_character_login_like_cpp(guid));
    assert!(!second.try_claim_character_login_like_cpp(guid));

    first.release_character_login_claim_like_cpp();
    assert!(second.try_claim_character_login_like_cpp(guid));
    second.release_character_login_claim_like_cpp();
}
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
fn account_data_times_respect_global_and_character_masks_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    assert_eq!(
        GLOBAL_CACHE_MASK_LIKE_CPP | PER_CHARACTER_CACHE_MASK_LIKE_CPP,
        ALL_ACCOUNT_DATA_CACHE_MASK_LIKE_CPP
    );

    assert!(session.set_account_data_like_cpp(0, 10, "global-0".to_string()));
    assert!(session.set_account_data_like_cpp(1, 20, "character-1".to_string()));
    assert!(session.set_account_data_like_cpp(4, 40, "global-4".to_string()));
    assert!(session.set_account_data_like_cpp(14, 140, "character-14".to_string()));

    let global_times =
        session.account_data_times_like_cpp(ObjectGuid::EMPTY, GLOBAL_CACHE_MASK_LIKE_CPP);
    assert_eq!(global_times.player_guid, ObjectGuid::EMPTY);
    assert_eq!(global_times.account_times[0], 10);
    assert_eq!(global_times.account_times[1], 0);
    assert_eq!(global_times.account_times[4], 40);
    assert_eq!(global_times.account_times[14], 0);

    let player_times =
        session.account_data_times_like_cpp(player_guid, PER_CHARACTER_CACHE_MASK_LIKE_CPP);
    assert_eq!(player_times.player_guid, player_guid);
    assert_eq!(player_times.account_times[0], 0);
    assert_eq!(player_times.account_times[1], 20);
    assert_eq!(player_times.account_times[4], 0);
    assert_eq!(player_times.account_times[14], 140);
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
    session.player_quests.insert(
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
    session.player_quests.insert(
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
    session.rewarded_quests.insert(200);
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
fn ensure_login_player_controller_is_idempotent_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let guid = ObjectGuid::create_player(1, 43);
    let start = Position::new(1.0, 2.0, 3.0, 4.0);

    assert!(session.ensure_login_player_controller_like_cpp(
        guid,
        "LoginTester".to_string(),
        start,
        571,
        1,
        8,
        70,
        0,
    ));
    assert_eq!(session.player_guid(), Some(guid));
    assert_eq!(session.player_name_like_cpp(), Some("LoginTester"));
    assert_eq!(session.player_position_like_cpp(), Some(start));
    assert_eq!(session.player_map_id_like_cpp(), 571);
    assert_eq!(session.fall_information_like_cpp(), (0, start.z));

    session.set_player_gold_like_cpp(1234);
    session.set_player_xp_like_cpp(55);
    session.set_known_spells_like_cpp(vec![118, 133]);
    session.set_fall_information_like_cpp(1_200, 80.0);

    let moved = Position::new(5.0, 6.0, 7.0, 8.0);
    assert!(!session.ensure_login_player_controller_like_cpp(
        guid,
        "LoginTesterRenamed".to_string(),
        moved,
        1,
        2,
        3,
        71,
        1,
    ));

    assert_eq!(session.player_guid(), Some(guid));
    assert_eq!(session.player_name_like_cpp(), Some("LoginTesterRenamed"));
    assert_eq!(session.player_position_like_cpp(), Some(moved));
    assert_eq!(session.player_map_id_like_cpp(), 1);
    assert_eq!(session.fall_information_like_cpp(), (0, moved.z));
    assert_eq!(session.player_race_like_cpp(), 2);
    assert_eq!(session.player_class_like_cpp(), 3);
    assert_eq!(session.player_level_like_cpp(), 71);
    assert_eq!(session.player_gender_like_cpp(), 1);
    assert_eq!(session.player_gold_like_cpp(), 1234);
    assert_eq!(session.player_xp_like_cpp(), 55);
    assert_eq!(session.known_spells_like_cpp(), &[118, 133]);
}
#[test]
fn first_login_start_all_explored_sets_all_cpp_blocks_and_sends_update() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 0xE001);
    session.ensure_login_player_controller_like_cpp(
        player_guid,
        "Explorer".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    );
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 571, 0);

    assert_eq!(
        session.apply_represented_first_login_explored_zones_like_cpp(),
        0
    );
    assert!(send_rx.try_recv().is_err());

    session.set_start_all_explored_like_cpp(true);
    let applied = session.apply_represented_first_login_explored_zones_like_cpp();

    assert_eq!(
        applied,
        wow_entities::PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP,
        "C++ loops PLAYER_EXPLORED_ZONES_SIZE and applies UI64_MAX to each block"
    );
    assert!(
        session
            .represented_explored_zones_db_string_like_cpp()
            .expect("test Player explored-zones owner resolves")
            .split_whitespace()
            .take(2)
            .eq(["4294967295", "4294967295"]),
        "first-login AddExploredZones must also update the represented DB snapshot"
    );
    {
        let manager = canonical.lock().unwrap();
        let player = manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_typed_player(player_guid)
            .unwrap();
        assert!(
            (0..wow_entities::PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP)
                .all(|index| player.explored_zones_block_like_cpp(index) == Some(u64::MAX))
        );
    }

    let packet = drain_server_packet_bytes(&send_rx)
        .into_iter()
        .find(|bytes| {
            wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
                == Some(ServerOpcodes::UpdateObject)
        })
        .expect("explored-zone field update");
    assert!(!packet.is_empty());
}
#[test]
fn offline_rested_xp_zero_logout_time_is_rejected_instead_of_cpp_wrap() {
    let (mut session, _, _) = make_session();
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    session.set_player_next_level_xp_like_cpp(72_000);
    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_NORMAL_LIKE_CPP, 0.0);

    let extra = session.apply_offline_xp_rest_bonus_like_cpp(0, 4_600, true);

    assert_eq!(extra, 0.0);
    assert_eq!(session.represented_xp_rest_bonus_like_cpp(), 0.0);
    assert_eq!(
        session.represented_xp_rest_state_like_cpp(),
        REST_STATE_NORMAL_LIKE_CPP
    );
}
#[test]
fn offline_rested_xp_future_logout_time_is_rejected_instead_of_cpp_wrap() {
    let (mut session, _, _) = make_session();
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    session.set_player_next_level_xp_like_cpp(72_000);
    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_NORMAL_LIKE_CPP, 0.0);

    let extra = session.apply_offline_xp_rest_bonus_like_cpp(4_601, 4_600, true);

    assert_eq!(extra, 0.0);
    assert_eq!(session.represented_xp_rest_bonus_like_cpp(), 0.0);
    assert_eq!(
        session.represented_xp_rest_state_like_cpp(),
        REST_STATE_NORMAL_LIKE_CPP
    );
}
#[test]
fn offline_rested_xp_login_does_not_modify_current_health_or_power_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    session.set_player_next_level_xp_like_cpp(72_000);
    session.set_player_health_like_cpp(41, 100);
    session.set_loaded_player_powers_like_cpp([17, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_NORMAL_LIKE_CPP, 0.0);

    let extra = session.apply_offline_xp_rest_bonus_like_cpp(1_000, 4_600, true);

    assert!(extra > 0.0);
    assert_eq!(session.player_health_like_cpp(), 41);
    assert_eq!(
        session
            .represented_player_power_values_like_cpp()
            .expect("power snapshot should be authoritative")[0],
        17
    );
}
#[test]
fn logout_resting_only_selects_offline_rate_and_does_not_restore_online_rest_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 0xE1B0);
    let canonical = Arc::new(Mutex::new(wow_map::MapManager::new(60_000, 1)));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    session.ensure_login_player_controller_like_cpp(
        guid,
        "RestLogin".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        1,
        1,
        8,
        10,
        0,
    );
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 1, 0);
    session.set_player_next_level_xp_like_cpp(72_000);
    session.set_player_zone_area_like_cpp(10, 100);
    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_RESTED_LIKE_CPP, 123.0);

    assert!(!session.represented_is_resting_like_cpp());
    assert!(
        !session
            .canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_RESTING_LIKE_CPP)
            .unwrap_or(false)
    );

    let applied = session.apply_offline_xp_rest_bonus_like_cpp(1_000, 1_100, true);

    assert!(
        applied > 0.0,
        "the persisted bit still selects the tavern/city offline rate"
    );
    assert!(!session.represented_is_resting_like_cpp());
    assert_eq!(session.represented_inn_area_trigger_id_like_cpp, 0);
    assert_eq!(session.represented_rest_time_secs_like_cpp, 0);
    assert!(
        !session
            .canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_RESTING_LIKE_CPP)
            .unwrap_or(true)
    );
    assert!(session.represented_xp_rest_bonus_like_cpp() > 123.0);
    assert_eq!(
        session.represented_xp_rest_state_like_cpp(),
        REST_STATE_RESTED_LIKE_CPP
    );

    let bonus_after_offline_accrual = session.represented_xp_rest_bonus_like_cpp();
    assert_eq!(
        session.update_represented_online_xp_rest_bonus_like_cpp(1_200),
        (0.0, 0),
        "C++ LoadRestBonus does not initialize RestMgr::_restTime"
    );
    assert_eq!(
        session.represented_xp_rest_bonus_like_cpp(),
        bonus_after_offline_accrual
    );
    assert!(!session.represented_is_resting_like_cpp());
    assert!(
        !session
            .canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_RESTING_LIKE_CPP)
            .unwrap_or(true)
    );
}
#[test]
fn ffa_realm_login_sets_ffa_only_for_non_resting_non_gm_player_like_cpp() {
    for (case, (resting, game_master, expected_ffa)) in [
        (false, false, true),
        (true, false, false),
        (false, true, false),
    ]
    .into_iter()
    .enumerate()
    {
        let (mut session, _, send_rx) = make_session();
        let guid = ObjectGuid::create_player(1, 0xFFA0 + case as i64);
        let canonical = shared_canonical_map_manager();
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.ensure_login_player_controller_like_cpp(
            guid,
            "FfaLogin".to_string(),
            Position::new(1.0, 2.0, 3.0, 0.0),
            1,
            1,
            8,
            10,
            0,
        );
        insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 1, 0);
        session.set_ffa_pvp_realm_like_cpp(true);
        session.set_player_game_master_like_cpp(game_master);
        if resting {
            assert!(session.set_represented_rest_flag_like_cpp(REST_FLAG_IN_TAVERN_LIKE_CPP, 42,));
        }
        let _ = drain_server_packet_bytes(&send_rx);

        session.set_state(SessionState::LoggedIn);

        assert_eq!(
            session
                .canonical_player_pvp_flags_like_cpp(guid)
                .is_some_and(|flags| flags.contains(UnitPvpFlags::FFA_PVP)),
            expected_ffa
        );
        let update_count = drain_server_packet_bytes(&send_rx)
            .iter()
            .filter(|packet| {
                WorldPacket::from_bytes(packet).server_opcode() == Some(ServerOpcodes::UpdateObject)
            })
            .count();
        assert_eq!(update_count, usize::from(expected_ffa));
    }
}
#[test]
fn login_update_zone_rebuilds_city_and_faction_rest_when_ids_are_preseeded_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 0xE1A3);
    session.ensure_login_player_controller_like_cpp(
        player_guid,
        "LoginCityRest".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        1,
        1,
        10,
        0,
    );
    session.set_player_zone_area_like_cpp(20, 101);
    session.set_area_table_store(Arc::new(wow_data::AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 20,
            continent_id: 571,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: wow_data::AREA_FLAG_LINKED_CHAT_LIKE_CPP,
        },
        wow_data::AreaTableEntry {
            id: 101,
            continent_id: 571,
            parent_area_id: 20,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: wow_data::AREA_FLAG_ALLIANCE_RESTING_LIKE_CPP,
        },
    ])));

    assert!(!session.update_zone_represented_like_cpp(20, 101));

    assert!(session.represented_is_resting_like_cpp());
    assert_ne!(
        session.represented_rest_flag_mask_like_cpp & REST_FLAG_IN_CITY_LIKE_CPP,
        0
    );
    assert_ne!(
        session.represented_rest_flag_mask_like_cpp & REST_FLAG_IN_FACTION_AREA_LIKE_CPP,
        0
    );
    assert_ne!(session.represented_rest_time_secs_like_cpp, 0);
    assert!(session.represented_area_zone_criteria_like_cpp().is_empty());
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
