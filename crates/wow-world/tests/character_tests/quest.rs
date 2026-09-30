//! Public character-handler integration contracts for [`super`].

use super::*;

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
    set_player_faction_template_for_test(&mut session, 1);
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

