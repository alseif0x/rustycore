//! Quest scenarios for [`super`].
//!
//! Split out of quest_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn quest_giver_query_quest_reads_respond_to_giver_as_bit_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 123, 456);

    for (bit_byte, expected) in [(0x80, true), (0x00, false), (0x01, false)] {
        let mut packet = quest_giver_cmsg_packet(guid, 7001, bit_byte);
        let (parsed_guid, quest_id, respond_to_giver) =
            read_quest_giver_query_quest_like_cpp(&mut packet).unwrap();

        assert_eq!(parsed_guid, guid);
        assert_eq!(quest_id, 7001);
        assert_eq!(
            respond_to_giver, expected,
            "C++ ReadBit reads the high bit; byte {bit_byte:#04x} must not be treated as bool"
        );
    }
}
#[test]
fn quest_giver_accept_quest_reads_start_cheat_as_bit_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 123, 456);

    for (bit_byte, expected) in [(0x80, true), (0x00, false), (0x01, false)] {
        let mut packet = quest_giver_cmsg_packet(guid, 7002, bit_byte);
        let (parsed_guid, quest_id, start_cheat) =
            read_quest_giver_accept_quest_like_cpp(&mut packet).unwrap();

        assert_eq!(parsed_guid, guid);
        assert_eq!(quest_id, 7002);
        assert_eq!(
            start_cheat, expected,
            "C++ ReadBit reads the high bit; byte {bit_byte:#04x} must not be treated as bool"
        );
    }
}
#[tokio::test]
async fn quest_giver_accept_emits_player_quest_log_update_like_cpp() {
    let (mut session, send_rx) = make_session();
    let quest_id = 7201;
    let gameobject_entry = 9301;
    let source_guid = gameobject_guid(gameobject_entry, 301);
    let mut quest = quest_template(quest_id);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: 44,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    let mut store = QuestStore::from_quests_like_cpp([quest]);
    assert!(store.insert_gameobject_starter_relation_like_cpp(gameobject_entry, quest_id));
    session.set_quest_store(Arc::new(store));
    let mut manager = wow_map::MapManager::default();
    insert_gameobject(&mut manager, source_guid, gameobject_entry);
    attach_map_manager(&mut session, manager);
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        source_guid,
        gameobject_entry,
        Position::new(10.0, 0.0, 0.0, 0.0),
        wow_entities::GAMEOBJECT_TYPE_QUESTGIVER as u8,
    );

    session
        .handle_quest_giver_accept_quest(quest_giver_cmsg_packet(source_guid, quest_id, 0x00))
        .await;

    let status = session
        .player_quests
        .get(&quest_id)
        .expect("accepted quest should enter the represented quest log");
    assert_eq!(status.slot, 0);
    assert_eq!(status.status, QUEST_STATUS_INCOMPLETE_LIKE_CPP);

    let update = send_rx
        .try_recv()
        .expect("C++ SetQuestSlot must become an immediate player UpdateObject");
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&update).server_opcode(),
        Some(wow_constants::ServerOpcodes::UpdateObject)
    );
    assert!(
        update
            .windows(std::mem::size_of::<u32>())
            .any(|window| window == quest_id.to_le_bytes()),
        "quest-log UpdateObject should carry the accepted QuestID"
    );

    let complete = send_rx
        .try_recv()
        .expect("legacy represented accept confirmation should still be sent");
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&complete).server_opcode(),
        Some(wow_constants::ServerOpcodes::QuestGiverQuestComplete)
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_giver_accept_rejected_source_sends_no_quest_log_update_like_cpp() {
    let (mut session, send_rx) = make_session();
    let quest_id = 7202;
    let gameobject_entry = 9302;
    let source_guid = gameobject_guid(gameobject_entry, 302);
    session.set_quest_store(Arc::new(store_with_quests(&[quest_id])));
    let mut manager = wow_map::MapManager::default();
    insert_gameobject(&mut manager, source_guid, gameobject_entry);
    attach_map_manager(&mut session, manager);

    session
        .handle_quest_giver_accept_quest(quest_giver_cmsg_packet(source_guid, quest_id, 0x00))
        .await;

    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn adventure_map_start_quest_records_request_after_cpp_gates() {
    let (mut session, _send_rx) = make_session();
    session.set_quest_store(Arc::new(store_with_quests(&[7001])));
    session.set_adventure_map_poi_store(Arc::new(AdventureMapPoiStore::from_entries([
        adventure_map_poi(10, 7002, 0),
        adventure_map_poi(20, 7001, 0),
    ])));

    session
        .handle_adventure_map_start_quest(adventure_map_start_quest_packet(7001))
        .await;

    assert_eq!(
        session.represented_adventure_map_start_quest_requests_like_cpp(),
        &[RepresentedAdventureMapStartQuestLikeCpp {
            quest_id: 7001,
            adventure_map_poi_id: 20,
            player_condition_id: 0,
        }]
    );
}
#[tokio::test]
async fn adventure_map_start_quest_unknown_quest_returns_silently_like_cpp() {
    let (mut session, _send_rx) = make_session();
    session.set_quest_store(Arc::new(store_with_quests(&[7001])));
    session.set_adventure_map_poi_store(Arc::new(AdventureMapPoiStore::from_entries([
        adventure_map_poi(20, 7002, 0),
    ])));

    session
        .handle_adventure_map_start_quest(adventure_map_start_quest_packet(7002))
        .await;

    assert!(
        session
            .represented_adventure_map_start_quest_requests_like_cpp()
            .is_empty()
    );
}
#[tokio::test]
async fn adventure_map_start_quest_missing_player_condition_store_returns_silently_like_cpp() {
    let (mut session, _send_rx) = make_session();
    session.set_quest_store(Arc::new(store_with_quests(&[7001])));
    session.set_adventure_map_poi_store(Arc::new(AdventureMapPoiStore::from_entries([
        adventure_map_poi(20, 7001, 42),
    ])));

    session
        .handle_adventure_map_start_quest(adventure_map_start_quest_packet(7001))
        .await;

    assert!(
        session
            .represented_adventure_map_start_quest_requests_like_cpp()
            .is_empty()
    );
}
#[tokio::test]
async fn quest_poi_query_filters_to_active_quest_slots_like_cpp() {
    let (mut session, send_rx) = make_session();
    let (realm_tx, realm_rx) = flume::bounded(8);
    session.install_realm_send_channel_for_test(realm_tx);
    add_active_quest(&mut session, 77);
    session.quest_poi_store_like_cpp = Some(Arc::new(HashMap::from([
        (
            77,
            wow_packet::packets::query::QuestPoiData {
                quest_id: 77,
                blobs: vec![wow_packet::packets::query::QuestPoiBlobData {
                    blob_index: 1,
                    objective_index: -1,
                    quest_objective_id: 2,
                    quest_object_id: 3,
                    map_id: 571,
                    ui_map_id: 486,
                    priority: 4,
                    flags: 5,
                    world_effect_id: 6,
                    player_condition_id: 7,
                    navigation_player_condition_id: 8,
                    spawn_tracking_id: 9,
                    points: vec![wow_packet::packets::query::QuestPoiBlobPoint {
                        x: 10,
                        y: 11,
                        z: 12,
                    }],
                    always_allow_merging_blobs: false,
                }],
            },
        ),
        (
            88,
            wow_packet::packets::query::QuestPoiData {
                quest_id: 88,
                blobs: Vec::new(),
            },
        ),
    ])));

    let mut missing_quest_pois =
        [0; wow_packet::packets::query::QUEST_POI_QUERY_MISSING_QUEST_POIS_LIKE_CPP];
    missing_quest_pois[0] = 77;
    missing_quest_pois[1] = 88;
    missing_quest_pois[2] = 77;

    session
        .handle_quest_poi_query(wow_packet::packets::query::QuestPoiQuery {
            missing_quest_count: 3,
            missing_quest_pois,
        })
        .await;

    let bytes = realm_rx.try_recv().expect("quest POI response");
    assert!(send_rx.try_recv().is_err());
    let mut packet = WorldPacket::from_bytes(&bytes);
    assert_eq!(
        packet.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::QuestPoiQueryResponse as u16
    );
    assert_eq!(packet.read_int32().unwrap(), 1);
    assert_eq!(packet.read_int32().unwrap(), 1);
    assert_eq!(packet.read_int32().unwrap(), 77);
    assert_eq!(packet.read_int32().unwrap(), 1);
}
#[test]
fn quest_poi_typed_rows_join_points_and_skip_unknown_groups_like_cpp() {
    let store = build_quest_poi_store_like_cpp(
        vec![QuestPoiPointLoadRowLikeCpp {
            quest_id: 77,
            idx1: 3,
            x: 10,
            y: 11,
            z: 12,
        }],
        vec![
            quest_poi_blob_row_like_cpp(77, 3),
            quest_poi_blob_row_like_cpp(88, 9),
        ],
    );

    assert_eq!(store.len(), 1);
    assert_eq!(store[&77].blobs[0].points[0].x, 10);
    assert!(!store.contains_key(&88));
}
#[tokio::test]
async fn quest_poi_cache_consumes_typed_port_rows_and_caches_the_result() {
    let (mut session, _) = make_session();
    session.set_quest_poi_persistence_port_like_cpp(Arc::new(QuestPoiPortFixtureLikeCpp(
        QuestPoiLoadOutcomeLikeCpp::Loaded {
            points: vec![QuestPoiPointLoadRowLikeCpp {
                quest_id: 77,
                idx1: 3,
                x: 10,
                y: 11,
                z: 12,
            }],
            blobs: vec![quest_poi_blob_row_like_cpp(77, 3)],
        },
    )));

    let first = session.quest_poi_store_like_cpp().await;
    let second = session.quest_poi_store_like_cpp().await;
    assert_eq!(first[&77].blobs.len(), 1);
    assert!(Arc::ptr_eq(&first, &second));
}
#[tokio::test]
async fn missing_or_failed_quest_poi_port_caches_the_existing_empty_result() {
    let (mut missing, _) = make_session();
    let missing_first = missing.quest_poi_store_like_cpp().await;
    let missing_second = missing.quest_poi_store_like_cpp().await;
    assert!(missing_first.is_empty());
    assert!(Arc::ptr_eq(&missing_first, &missing_second));

    let (mut failed, _) = make_session();
    failed.set_quest_poi_persistence_port_like_cpp(Arc::new(QuestPoiPortFixtureLikeCpp(
        QuestPoiLoadOutcomeLikeCpp::Failed {
            stage: QuestPoiLoadStageLikeCpp::Points,
            reason: "world DB unavailable".to_owned(),
        },
    )));
    let failed_store = failed.quest_poi_store_like_cpp().await;
    assert!(failed_store.is_empty());
}
#[tokio::test]
async fn quest_giver_status_queries_borrow_process_metadata_not_session_catalog() {
    use crate::session::SessionHandlerCatalogsLikeCpp;
    use wow_constants::ClientOpcodes;
    use wow_handler::{PacketProcessing, SessionStatus};
    for (metadata, expected) in [
        (None, quest_giver_status::QUEST),
        (Some((2, 0x400)), quest_giver_status::IMPORTANT_QUEST),
        (Some((15, 0)), quest_giver_status::COVENANT_CALLING_QUEST),
    ] {
        for opcode in [
            ClientOpcodes::QuestGiverStatusQuery,
            ClientOpcodes::QuestGiverStatusMultipleQuery,
            ClientOpcodes::QuestGiverStatusTrackedQuery,
        ] {
            let (mut session, send_rx) = make_session();
            let mut quest = quest_template(9010);
            quest.quest_level = 80;
            quest.quest_info_id = 710;
            let mut store = QuestStore::from_quests_like_cpp([quest]);
            store.starter_quests.entry(9010).or_default().push(9010);
            session.set_quest_store(Arc::new(store));
            // Deliberately disagree with each borrowed catalog. A hidden Session
            // lookup or empty-catalog fallback would change the packet assertion.
            let cached = if expected == quest_giver_status::IMPORTANT_QUEST {
                0
            } else {
                0x400
            };
            session.set_quest_info_store(Arc::new(QuestInfoStore::from_entries([
                quest_info_entry_like_cpp(710, 2, cached),
            ])));
            let catalogs = SessionHandlerCatalogsLikeCpp {
                quest_info: Arc::new(QuestInfoStore::from_entries(
                    metadata.map(|(tag, modifiers)| quest_info_entry_like_cpp(710, tag, modifiers)),
                )),
                ..Default::default()
            };
            let guid = creature_guid(9010, 10);
            let mut manager = wow_map::MapManager::default();
            insert_creature(&mut manager, guid, 9010);
            attach_map_manager(&mut session, manager);
            mark_visible(&mut session, guid);
            let entry = crate::session::registry::get_handler(opcode).unwrap();
            assert_eq!(entry.status, SessionStatus::LoggedIn);
            assert_eq!(
                entry.processing,
                if opcode == ClientOpcodes::QuestGiverStatusMultipleQuery {
                    PacketProcessing::ThreadUnsafe
                } else {
                    PacketProcessing::Inplace
                }
            );
            let mut request = WorldPacket::new_empty();
            if opcode == ClientOpcodes::QuestGiverStatusTrackedQuery {
                request.write_uint32(1);
            }
            if opcode != ClientOpcodes::QuestGiverStatusMultipleQuery {
                request.write_packed_guid(&guid);
            }
            (entry.handler)(&mut session, &catalogs, request).await;
            if opcode == ClientOpcodes::QuestGiverStatusQuery {
                assert_eq!(recv_status(&send_rx), (guid, expected));
            } else {
                assert_eq!(recv_status_multiple(&send_rx), [(guid, expected)]);
            }
            assert!(send_rx.is_empty());
            assert_eq!(Arc::strong_count(&catalogs.quest_info), 1);
        }
    }
}
#[test]
fn represented_quest_objective_completable_accepts_cpp_storing_value_previous_types() {
    let quest_id = 7100;
    let mut quest = quest_template(quest_id);
    quest.objectives = vec![
        QuestObjective {
            id: quest_id * 10,
            quest_id,
            obj_type: QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL,
            order: 0,
            storage_index: 0,
            object_id: 44,
            amount: 1,
            flags: 0,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        },
        QuestObjective {
            id: quest_id * 10 + 1,
            quest_id,
            obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
            order: 1,
            storage_index: 1,
            object_id: 55,
            amount: 1,
            flags: QUEST_OBJECTIVE_FLAG_SEQUENCED_LIKE_CPP_LOCAL,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        },
    ];
    let status = PlayerQuestStatus {
        quest_id,
        status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        explored: false,
        accept_time_secs: 0,
        end_time_secs: 0,
        objective_counts: vec![1, 0],
        slot: 0,
    };

    assert!(
        crate::handlers::quest_rules::represented_quest_objective_completable_like_cpp(
            &status, &quest, 1
        )
    );
}
#[test]
fn quest_giver_choose_reward_choice_parser_rejects_truncated_cpp_wire() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bits(u32::from(QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP), 2);
    write_cpp_item_instance_like_cpp(&mut pkt, 19019, 0, 0, &[], None);

    assert!(WorldSession::read_quest_choice_item_like_cpp(&mut pkt).is_err());
}
#[test]
fn quest_giver_choose_reward_choice_validation_matches_loaded_cpp_type() {
    let mut quest = quest_template(7002);
    quest.reward_choice_items[0] = (19019, 1);
    quest.reward_choice_item_types[0] = QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP;
    quest.reward_choice_items[1] = (392, 5);
    quest.reward_choice_item_types[1] = QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP;

    assert!(
        WorldSession::represented_reward_choice_matches_loaded_type_like_cpp(
            &quest,
            QuestChoiceItemLikeCpp {
                loot_item_type: QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                item_id: 19019,
                quantity: 1,
            }
        )
    );
    assert!(
        WorldSession::represented_reward_choice_matches_loaded_type_like_cpp(
            &quest,
            QuestChoiceItemLikeCpp {
                loot_item_type: QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP,
                item_id: 392,
                quantity: 5,
            }
        )
    );
    assert!(
        !WorldSession::represented_reward_choice_matches_loaded_type_like_cpp(
            &quest,
            QuestChoiceItemLikeCpp {
                loot_item_type: QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                item_id: 392,
                quantity: 5,
            }
        )
    );
}
#[tokio::test]
async fn quest_giver_choose_reward_accepts_existing_reward_currency_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7004;
    let currency_id = 392;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.reward_choice_items[0] = (currency_id, 5);
    quest.reward_choice_item_types[0] = QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP;
    session.set_player_gold_like_cpp(5);
    session.set_currency_types_store(Arc::new(CurrencyTypesStore::from_entries([
        currency_entry_like_cpp(currency_id),
    ])));
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP,
            currency_id,
        ))
        .await;

    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(session.rewarded_quests.contains(&quest_id));
    assert_eq!(session.player_gold_like_cpp(), 42);
    assert_eq!(session.player_currency_quantity(currency_id), Some(5));
    assert_eq!(
        send_rx.try_recv().unwrap(),
        wow_packet::packets::misc::SetCurrency {
            type_id: currency_id as i32,
            quantity: 5,
            flags: 0,
            weekly_quantity: None,
            tracked_quantity: None,
            max_quantity: None,
            total_earned: None,
            suppress_chat_log: false,
            quantity_change: Some(5),
            quantity_gain_source: Some(CurrencyGainSourceLikeCpp::QuestReward as i32),
            quantity_lost_source: None,
            first_craft_operation_id: None,
            next_recharge_time: None,
            recharge_cycle_start_time: None,
            overflown_currency_id: None,
        }
        .to_bytes()
    );
    let opcodes = std::iter::from_fn(|| send_rx.try_recv().ok())
        .map(|bytes| wow_packet::WorldPacket::from_bytes(&bytes).server_opcode())
        .collect::<Vec<_>>();
    assert_eq!(
        opcodes,
        vec![
            Some(wow_constants::ServerOpcodes::UpdateObject),
            Some(wow_constants::ServerOpcodes::QuestGiverQuestComplete),
            Some(wow_constants::ServerOpcodes::QuestUpdateComplete),
        ]
    );
}
#[tokio::test]
async fn quest_giver_choose_reward_fixed_currency_rewards_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7014;
    let currency_id = 393;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP | QUEST_FLAGS_DAILY_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.reward_currencies[0] = currency_id;
    quest.reward_currency_amounts[0] = 7;
    session.set_player_gold_like_cpp(5);
    session.set_currency_types_store(Arc::new(CurrencyTypesStore::from_entries([
        currency_entry_like_cpp(currency_id),
    ])));
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(session.rewarded_quests.contains(&quest_id));
    assert_eq!(session.player_gold_like_cpp(), 42);
    assert_eq!(session.player_currency_quantity(currency_id), Some(7));
    assert_eq!(
        send_rx.try_recv().unwrap(),
        wow_packet::packets::misc::SetCurrency {
            type_id: currency_id as i32,
            quantity: 7,
            flags: 0,
            weekly_quantity: None,
            tracked_quantity: None,
            max_quantity: None,
            total_earned: None,
            suppress_chat_log: false,
            quantity_change: Some(7),
            quantity_gain_source: Some(CurrencyGainSourceLikeCpp::DailyQuestReward as i32),
            quantity_lost_source: None,
            first_craft_operation_id: None,
            next_recharge_time: None,
            recharge_cycle_start_time: None,
            overflown_currency_id: None,
        }
        .to_bytes()
    );
    let opcodes = std::iter::from_fn(|| send_rx.try_recv().ok())
        .map(|bytes| wow_packet::WorldPacket::from_bytes(&bytes).server_opcode())
        .collect::<Vec<_>>();
    assert_eq!(
        opcodes,
        vec![
            Some(wow_constants::ServerOpcodes::UpdateObject),
            Some(wow_constants::ServerOpcodes::QuestGiverQuestComplete),
            Some(wow_constants::ServerOpcodes::QuestUpdateComplete),
        ]
    );
}
#[tokio::test]
async fn quest_giver_choose_reward_removes_timed_quest_before_rewards_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7020;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    session.set_player_gold_like_cpp(5);
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 100,
            end_time_secs: 700,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert_eq!(
        session.represented_timed_quest_removals_like_cpp(),
        &[quest_id]
    );
    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(session.rewarded_quests.contains(&quest_id));
    assert_eq!(session.player_gold_like_cpp(), 42);
}
#[tokio::test]
async fn quest_giver_choose_reward_non_timed_quest_records_no_timed_removal_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7021;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 100,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert!(
        session
            .represented_timed_quest_removals_like_cpp()
            .is_empty()
    );
    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(session.rewarded_quests.contains(&quest_id));
}
#[tokio::test]
async fn quest_giver_choose_reward_emits_reward_skill_fields_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7022;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.reward_skill_line_id = 333;
    quest.reward_skill_points = 5;
    session.set_player_gold_like_cpp(5);
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert_eq!(
        session.represented_quest_reward_skill_updates_like_cpp(),
        &[(333, 5)]
    );
    let update = send_rx.try_recv().unwrap();
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&update).server_opcode(),
        Some(wow_constants::ServerOpcodes::UpdateObject)
    );
    let complete = send_rx.try_recv().unwrap();
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&complete).server_opcode(),
        Some(wow_constants::ServerOpcodes::QuestGiverQuestComplete)
    );
    assert_eq!(&complete[18..22], &333u32.to_le_bytes());
    assert_eq!(&complete[22..26], &5u32.to_le_bytes());
    assert_eq!(session.player_gold_like_cpp(), 42);
}
#[tokio::test]
async fn quest_giver_choose_reward_records_title_and_talent_rewards_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7025;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_title_id = 77;
    quest.reward_skill_points = 3;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert_eq!(
        session.represented_quest_reward_titles_like_cpp(),
        &[RepresentedQuestRewardTitleLikeCpp {
            quest_id,
            title_id: 77,
            char_title_lookup_unrepresented: true,
            set_title_runtime_unrepresented: true,
        }]
    );
    assert_eq!(
        session.represented_quest_reward_talent_points_like_cpp(),
        &[RepresentedQuestRewardTalentPointsLikeCpp {
            quest_id,
            points: 3,
            init_talent_for_level_unrepresented: true,
        }]
    );
}
#[tokio::test]
async fn quest_giver_choose_reward_records_reward_mail_sender_entry_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7026;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_mail_template_id = 55;
    quest.reward_mail_delay_secs = 900;
    quest.reward_mail_sender_entry = 1234;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert_eq!(
        session.represented_quest_reward_mails_like_cpp(),
        &[RepresentedQuestRewardMailLikeCpp {
            quest_id,
            mail_template_id: 55,
            delay_secs: 900,
            sender_entry: Some(1234),
            quest_giver_guid: None,
            mail_template_lookup_unrepresented: true,
            mail_draft_runtime_unrepresented: true,
            character_db_transaction_unrepresented: true,
        }]
    );
}
