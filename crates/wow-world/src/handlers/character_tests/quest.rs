//! Quest scenarios for [`super`].
//!
//! Split out of character_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn persisted_transport_login_requests_world_row_by_guid_and_keeps_absence_unknown_like_cpp() {
    let port = CollectionLoadPortLikeCpp::for_login_transports([
        PlayerLoginTransportLoadOutcomeLikeCpp::Loaded(Vec::new()),
        PlayerLoginTransportLoadOutcomeLikeCpp::Failed {
            reason: "world transport read failed".to_owned(),
        },
    ]);
    let (mut session, _) = make_session_with_send_capacity(1);
    session.set_player_lifecycle_port_like_cpp(port.clone());

    let empty = session
        .resolve_persisted_transport_login_like_cpp(77, 571, Position::new(1.0, 2.0, 3.0, 4.0))
        .await;
    let failed = session
        .resolve_persisted_transport_login_like_cpp(88, 571, Position::new(1.0, 2.0, 3.0, 4.0))
        .await;

    assert!(empty.is_none());
    assert!(failed.is_none());
    assert_eq!(
        port.login_transport_requests(),
        vec![
            PlayerLoginTransportLoadRequestLikeCpp::ByGuid { guid_low: 77 },
            PlayerLoginTransportLoadRequestLikeCpp::ByGuid { guid_low: 88 },
        ]
    );
}
#[test]
fn skill_rewarded_quest_fallback_uses_future_player_condition_like_cpp() {
    let (mut session, _) = make_session_with_send_capacity(1);
    session.set_loaded_player_identity_like_cpp(0, 1, 1, 10, 0);
    session.set_spell_misc_store(Arc::new(SpellMiscStore::from_entries([
        SpellMiscEntry {
            id: 1,
            spell_id: 900,
            show_future_spell_player_condition_id: 77,
            ..SpellMiscEntry::default()
        },
        SpellMiscEntry {
            id: 2,
            spell_id: 901,
            show_future_spell_player_condition_id: 0,
            ..SpellMiscEntry::default()
        },
    ])));
    session.set_player_condition_store(Arc::new(wow_data::PlayerConditionStore::from_entries([
        PlayerConditionEntry {
            id: 77,
            class_mask: 1,
            ..PlayerConditionEntry::default()
        },
    ])));

    assert!(session.skill_rewarded_quest_fallback_allowed_like_cpp(900));
    assert!(
        !session.skill_rewarded_quest_fallback_allowed_like_cpp(901),
        "C++ MeetsFutureSpellPlayerCondition returns false when the condition id is zero"
    );

    session.set_loaded_player_identity_like_cpp(0, 1, 2, 10, 0);
    assert!(
        !session.skill_rewarded_quest_fallback_allowed_like_cpp(900),
        "the fallback must evaluate the real PlayerCondition against the current player"
    );
}
#[tokio::test]
async fn homebind_repair_writes_typed_delete_and_insert_requests_nonfatally_like_cpp() {
    let (mut session, _, _) = make_bank_slot_session(4);
    let port = HomebindPortFixtureLikeCpp::new([
        PersistenceOutcomeLikeCpp::Failed {
            reason: "delete failed".to_owned(),
        },
        PersistenceOutcomeLikeCpp::Failed {
            reason: "insert failed".to_owned(),
        },
    ]);
    session.set_player_lifecycle_port_like_cpp(port.clone());
    let guid = ObjectGuid::create_player(1, 77);

    session
        .delete_invalid_character_homebind_like_cpp(guid)
        .await;
    let create_position = PlayerCreatePositionLikeCpp {
        map_id: 0,
        position: Position::new(1.0, 2.0, 3.0, 4.0),
        transport_guid: None,
    };
    let repaired = session
        .repair_character_homebind_like_cpp(
            guid,
            1,
            PlayerCreateInfoLikeCpp {
                create_position,
                create_position_npe: None,
            },
            wow_data::PLAYER_CREATE_MODE_NORMAL_LIKE_CPP,
            true,
        )
        .await
        .expect("nonfatal persistence failure does not discard selected homebind");
    assert_eq!(repaired.map_id, 0);
    assert_eq!(
        port.requests(),
        vec![
            PlayerHomebindPersistenceRequestLikeCpp::DeleteInvalid {
                player_guid: guid.counter() as u64,
            },
            PlayerHomebindPersistenceRequestLikeCpp::InsertRepaired {
                player_guid: guid.counter() as u64,
                map_id: 0,
                area_id: 0,
                x: 1.0,
                y: 2.0,
                z: 3.0,
                orientation: 4.0,
            },
        ]
    );
}
#[test]
fn recursive_destroy_plans_child_and_parent_quest_removal_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let quest_id = 91_001;
    let child_entry = 700;
    let parent_entry = 600;
    let mut quest = quest_template(quest_id);
    quest.objectives = [child_entry, parent_entry]
        .into_iter()
        .enumerate()
        .map(|(index, entry_id)| QuestObjective {
            id: quest_id * 10 + index as u32,
            quest_id,
            obj_type: 1,
            order: index as u8,
            storage_index: index as i8,
            object_id: entry_id as i32,
            amount: 1,
            flags: 0,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        })
        .collect();
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![1, 1],
            slot: 0,
        },
    );

    let planned = session
        .plan_destroyed_inventory_quest_persistence_like_cpp(&[
            DestroyQuestItemLikeCpp {
                bag: INVENTORY_SLOT_BAG_START,
                slot: 0,
                entry_id: child_entry,
                count: 1,
            },
            DestroyQuestItemLikeCpp {
                bag: INVENTORY_SLOT_BAG_0,
                slot: INVENTORY_SLOT_BAG_START,
                entry_id: parent_entry,
                count: 1,
            },
        ])
        .expect("fixture canonical inventory owner");

    assert_eq!(planned.len(), 1);
    assert_eq!(planned[0].objective_counts, vec![0, 0]);
    assert_eq!(
        planned[0].status,
        crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP
    );
}
#[tokio::test]
async fn alter_appearance_on_represented_barber_chair_records_request_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 22);
    let chair_position = Position::new(1.0, 2.0, 3.0, 0.0);

    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    assert!(session.use_represented_gameobject_barber_chair_like_cpp(
        gameobject_guid,
        player_guid,
        chair_position,
        wow_entities::BarberChairUseSource {
            chair_height: 2,
            sit_anim_kit: 0,
            customization_scope: 7,
        },
    ));
    let _enable_barber_shop = send_rx.try_recv().unwrap();

    session
        .handle_alter_appearance(alter_appearance_packet(1, 7, 11, &[(20, 200), (10, 100)]))
        .await;

    assert_eq!(
        read_barber_shop_result(send_rx.try_recv().unwrap()),
        BARBER_SHOP_RESULT_SUCCESS_LIKE_CPP
    );
    assert_eq!(
        session.represented_alter_appearance_requests_like_cpp(),
        &[RepresentedAlterAppearanceLikeCpp {
            new_sex: 1,
            customizations: vec![
                ChrCustomizationChoice {
                    option_id: 10,
                    choice_id: 100,
                },
                ChrCustomizationChoice {
                    option_id: 20,
                    choice_id: 200,
                },
            ],
            customized_race: 7,
            customized_chr_model_id: 11,
            cost: 0,
        }]
    );
}
#[tokio::test]
async fn confirm_barbers_choice_records_request_without_success_packet_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);

    session
        .handle_confirm_barbers_choice(confirm_barbers_choice_packet(&[(20, 200), (10, 100)]))
        .await;

    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        session.represented_confirm_barbers_choice_requests_like_cpp(),
        &[RepresentedConfirmBarbersChoiceLikeCpp {
            customizations: vec![
                ChrCustomizationChoice {
                    option_id: 20,
                    choice_id: 200,
                },
                ChrCustomizationChoice {
                    option_id: 10,
                    choice_id: 100,
                },
            ],
            cost: 0,
        }]
    );
}
#[tokio::test]
async fn request_stabled_pets_without_stable_master_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let stable_master = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 22, 1);
    let mut request = WorldPacket::new_empty();
    request.write_packed_guid(&stable_master);

    session.handle_request_stabled_pets(request).await;

    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn gossip_hello_questgiver_without_db_gossip_menu_opens_quest_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    let entry = 9306;
    let guid = creature_guid(entry, 306);
    let mut store = store_with_quests(&[3006]);
    store.starter_quests.entry(entry).or_default().push(3006);
    session.set_quest_store(Arc::new(store));
    attach_legacy_creature(
        &mut session,
        guid,
        entry,
        NPCFlags1::GOSSIP.bits() | NPCFlags1::QUEST_GIVER.bits(),
    );

    session.handle_gossip_hello(Hello { unit: guid }).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::QuestGiverQuestDetails]
    );
}
#[tokio::test]
async fn gossip_hello_questgiver_without_quest_relation_keeps_empty_gossip_fallback() {
    let (mut session, send_rx) = make_quest_status_session();
    let entry = 9307;
    let guid = creature_guid(entry, 307);
    session.set_quest_store(Arc::new(store_with_quests(&[3007])));
    attach_legacy_creature(
        &mut session,
        guid,
        entry,
        NPCFlags1::GOSSIP.bits() | NPCFlags1::QUEST_GIVER.bits(),
    );

    session.handle_gossip_hello(Hello { unit: guid }).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::GossipMessage]
    );
}
#[tokio::test]
async fn gossip_hello_hostile_questgiver_fallback_is_rejected_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    let entry = 9310;
    let guid = creature_guid(entry, 310);
    let mut store = store_with_quests(&[3010]);
    store.starter_quests.entry(entry).or_default().push(3010);
    session.set_quest_store(Arc::new(store));
    session.set_player_faction_template_like_cpp(1);
    session.set_faction_template_store(Arc::new(
        wow_data::progression_rewards::FactionTemplateStore::from_entries([
            faction_template_entry(35, 35, 0, 0, 1),
            faction_template_entry(1, 1, 0, 0, 0),
        ]),
    ));
    attach_legacy_creature(
        &mut session,
        guid,
        entry,
        NPCFlags1::GOSSIP.bits() | NPCFlags1::QUEST_GIVER.bits(),
    );

    session.handle_gossip_hello(Hello { unit: guid }).await;

    assert!(
        send_rx.try_recv().is_err(),
        "C++ HandleGossipHelloOpcode returns when GetNPCIfCanInteractWith rejects a hostile questgiver"
    );
}
#[tokio::test]
async fn quest_giver_hello_trainer_questgiver_sends_mixed_gossip_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    let entry = 15_513;
    let guid = creature_guid(entry, 513);
    let mut store = store_with_quests(&[9_393]);
    store.starter_quests.entry(entry).or_default().push(9_393);
    session.set_quest_store(Arc::new(store));
    attach_legacy_creature(
        &mut session,
        guid,
        entry,
        NPCFlags1::GOSSIP.bits()
            | NPCFlags1::QUEST_GIVER.bits()
            | NPCFlags1::TRAINER.bits()
            | NPCFlags1::TRAINER_CLASS.bits(),
    );

    session
        .handle_quest_giver_hello(quest_giver_hello_packet(guid))
        .await;

    let bytes = send_rx.try_recv().expect("mixed prepared gossip menu");
    assert_eq!(gossip_message_counts(&bytes, guid), (1, 1));
    assert_eq!(
        session.player_interaction_source_guid_like_cpp(),
        Some(guid)
    );
    assert_eq!(session.player_interaction_trainer_id_like_cpp(), 0);
    assert_eq!(session.gossip_options.len(), 1);
    assert_eq!(
        session.gossip_options[0].gossip_option_id,
        GOSSIP_OPTION_ID_AUTO_TRAINER_LIKE_CPP
    );
    assert_eq!(
        session.gossip_options[0].option_npc,
        GOSSIP_OPTION_NPC_TRAINER_LIKE_CPP
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_giver_hello_plain_questgiver_keeps_direct_quest_open_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    let entry = 9308;
    let guid = creature_guid(entry, 308);
    let mut store = store_with_quests(&[3008]);
    store.starter_quests.entry(entry).or_default().push(3008);
    session.set_quest_store(Arc::new(store));
    attach_legacy_creature(
        &mut session,
        guid,
        entry,
        NPCFlags1::GOSSIP.bits() | NPCFlags1::QUEST_GIVER.bits(),
    );

    session
        .handle_quest_giver_hello(quest_giver_hello_packet(guid))
        .await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::QuestGiverQuestDetails]
    );
}
#[test]
fn gossip_quest_text_filters_race_class_and_level_like_cpp() {
    let (mut session, _send_rx) = make_quest_status_session();
    session.set_loaded_player_identity_like_cpp(571, 10, 3, 3, 0);
    let entry = 15_278;

    let blood_elf_mask = 1u64 << (10 - 1);
    let hunter_mask = 1u32 << (3 - 1);
    let mage_mask = 1u32 << (8 - 1);
    let human_mask = 1u64 << (1 - 1);

    let mut generic = quest_template(8_325);
    generic.allowable_races = blood_elf_mask;
    generic.min_level = 1;
    generic.log_title = "Reclaiming Sunstrider Isle".into();

    let mut hunter = quest_template(9_393);
    hunter.allowable_races = blood_elf_mask;
    hunter.allowable_classes = hunter_mask;
    hunter.min_level = 1;
    hunter.log_title = "Hunter Training".into();

    let mut mage = quest_template(8_328);
    mage.allowable_races = blood_elf_mask;
    mage.allowable_classes = mage_mask;
    mage.min_level = 1;
    mage.log_title = "Mage Training".into();

    let mut too_high = quest_template(99_001);
    too_high.allowable_races = blood_elf_mask;
    too_high.min_level = 4;

    let mut wrong_race = quest_template(99_002);
    wrong_race.allowable_races = human_mask;
    wrong_race.min_level = 1;

    let mut store = QuestStore::from_quests_like_cpp([generic, hunter, mage, too_high, wrong_race]);
    store
        .starter_quests
        .entry(entry)
        .or_default()
        .extend([8_325, 9_393, 8_328, 99_001, 99_002]);
    session.set_quest_store(Arc::new(store));

    let quest_text = session.represented_creature_gossip_text_like_cpp(entry);

    assert_eq!(
        quest_text
            .iter()
            .map(|text| text.quest_id)
            .collect::<Vec<_>>(),
        vec![8_325, 9_393]
    );
    assert!(quest_text.iter().all(|text| text.quest_type == 2));
}
#[test]
fn gossip_quest_text_offers_sallina_followup_after_hunter_training_rewarded_like_cpp() {
    let (mut session, _send_rx) = make_quest_status_session();
    session.set_loaded_player_identity_like_cpp(530, 10, 3, 3, 0);
    let sallina_entry = 15_513;
    let blood_elf_mask = 1u64 << (10 - 1);
    let hunter_mask = 1u32 << (3 - 1);

    let mut hunter_training = quest_template(9_393);
    hunter_training.allowable_races = blood_elf_mask;
    hunter_training.allowable_classes = hunter_mask;
    hunter_training.min_level = 1;
    hunter_training.log_title = "Hunter Training".into();

    let mut followup = quest_template(10_070);
    followup.allowable_races = blood_elf_mask;
    followup.allowable_classes = hunter_mask;
    followup.min_level = 2;
    followup.prev_quest_id = 9_393;
    followup.exclusive_group = 10_068;
    followup.log_title = "Well Watcher Solanian".into();

    let mut store = QuestStore::from_quests_like_cpp([hunter_training, followup]);
    store
        .ender_quests
        .entry(sallina_entry)
        .or_default()
        .push(9_393);
    store
        .starter_quests
        .entry(sallina_entry)
        .or_default()
        .push(10_070);
    session.set_quest_store(Arc::new(store));
    session.rewarded_quests.insert(9_393);

    let quest_text = session.represented_creature_gossip_text_like_cpp(sallina_entry);

    assert_eq!(
        quest_text
            .iter()
            .map(|text| (text.quest_id, text.quest_title.as_str(), text.quest_type))
            .collect::<Vec<_>>(),
        vec![(10_070, "Well Watcher Solanian", 2)]
    );
}
#[tokio::test]
async fn quest_giver_status_tracked_duplicate_guid_emits_single_status_like_cpp_set() {
    let (mut session, send_rx) = make_quest_status_session();
    let mut store = store_with_quests(&[3003]);
    store.starter_quests.entry(9303).or_default().push(3003);
    session.set_quest_store(Arc::new(store));
    let guid = creature_guid(9303, 303);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9303);
    attach_map_manager(&mut session, manager);

    session
        .handle_quest_giver_status_tracked_query(tracked_query_packet(&[guid, guid]))
        .await;

    assert_eq!(recv_status_multiple(&send_rx).len(), 1);
}
#[tokio::test]
async fn quest_giver_status_tracked_count_over_cpp_max_sends_no_packet() {
    let (mut session, send_rx) = make_quest_status_session();
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(QUEST_GIVER_STATUS_TRACKED_QUERY_MAX_GUIDS_LIKE_CPP + 1);

    session.handle_quest_giver_status_tracked_query(pkt).await;

    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_giver_status_tracked_short_payload_sends_no_packet() {
    let (mut session, send_rx) = make_quest_status_session();
    let guid = creature_guid(9304, 304);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(1);
    pkt.write_packed_guid(&guid);
    let mut bytes = pkt.into_data();
    bytes.pop();

    session
        .handle_quest_giver_status_tracked_query(WorldPacket::from_bytes(&bytes))
        .await;

    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_giver_status_tracked_unsupported_missing_guid_sends_empty_multiple_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    attach_map_manager(&mut session, wow_map::MapManager::default());
    session.set_quest_store(Arc::new(store_with_quests(&[3005])));
    let missing_guid = creature_guid(9305, 305);
    let player_guid = ObjectGuid::create_player(1, 305);
    let item_guid = ObjectGuid::create_item(1, 305);

    session
        .handle_quest_giver_status_tracked_query(tracked_query_packet(&[
            missing_guid,
            player_guid,
            item_guid,
        ]))
        .await;

    assert!(recv_status_multiple(&send_rx).is_empty());
}
#[tokio::test]
async fn hotfix_request_local_db2_blob_is_not_sent_as_typed_cpp_storage() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let mut cache = wow_data::HotfixBlobCache::new();
    cache.insert_blob(0x919B_E54E, 198647, vec![0xAA; 408]);
    cache.insert_hotfix_record_like_cpp(wow_data::HotfixRecord {
        table_hash: 0x919B_E54E,
        record_id: 198647,
        id: wow_data::HotfixId {
            push_id: 77,
            unique_id: 88,
        },
        status: wow_data::HotfixRecordStatus::Valid,
        available_locales_mask: wow_data::hotfix_locale_mask("esES"),
    });
    session
        .handle_hotfix_request(
            &cache,
            wow_packet::packets::misc::HotfixRequest {
                client_build: 54261,
                data_build: 54261,
                hotfixes: vec![77],
            },
        )
        .await;

    let bytes = send_rx.try_recv().expect("hotfix connect");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::HotfixConnect as u16
    );
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), 1);
    assert_eq!(pkt.read_int32().unwrap(), 77);
    assert_eq!(pkt.read_uint32().unwrap(), 88);
    assert_eq!(pkt.read_uint32().unwrap(), 0x919B_E54E);
    assert_eq!(pkt.read_int32().unwrap(), 198647);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    assert_eq!(pkt.read_bits(3).unwrap(), 3);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn hotfix_request_sql_hotfix_blob_keeps_valid_cpp_blob_path() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let mut cache = wow_data::HotfixBlobCache::new();
    cache.insert_hotfix_blob(0xAABB_CCDD, 123, vec![1, 2, 3, 4]);
    cache.insert_hotfix_record_like_cpp(wow_data::HotfixRecord {
        table_hash: 0xAABB_CCDD,
        record_id: 123,
        id: wow_data::HotfixId {
            push_id: 78,
            unique_id: 89,
        },
        status: wow_data::HotfixRecordStatus::Valid,
        available_locales_mask: wow_data::hotfix_locale_mask("esES"),
    });
    session
        .handle_hotfix_request(
            &cache,
            wow_packet::packets::misc::HotfixRequest {
                client_build: 54261,
                data_build: 54261,
                hotfixes: vec![78],
            },
        )
        .await;

    let bytes = send_rx.try_recv().expect("hotfix connect");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::HotfixConnect as u16
    );
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), 1);
    assert_eq!(pkt.read_int32().unwrap(), 78);
    assert_eq!(pkt.read_uint32().unwrap(), 89);
    assert_eq!(pkt.read_uint32().unwrap(), 0xAABB_CCDD);
    assert_eq!(pkt.read_int32().unwrap(), 123);
    assert_eq!(pkt.read_uint32().unwrap(), 4);
    assert_eq!(pkt.read_bits(3).unwrap(), 1);
    assert_eq!(pkt.read_uint32().unwrap(), 4);
    assert_eq!(pkt.read_bytes(4).unwrap(), vec![1, 2, 3, 4]);
    assert!(send_rx.try_recv().is_err());
}
