use crate::handlers::test_support::world::quest_template;
use crate::session::WorldSession;
use std::sync::{Arc, Mutex};
use wow_constants::unit::NPCFlags1;
use wow_core::guid::HighGuid;
use wow_core::{EquipmentSetGuidGeneratorLikeCpp, ObjectGuid, ObjectGuidGenerator, Position};
use wow_data::quest::QuestStore;
use wow_packet::WorldPacket;
use wow_packet::packets::quest::quest_giver_status;

fn make_quest_status_session() -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_faction_template_like_cpp(1);
    session.set_player_position_like_cpp(Position::new(10.0, 0.0, 0.0, 0.0));
    (session, send_rx)
}

fn make_session_with_send_capacity(capacity: usize) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(capacity);
    let mut session = WorldSession::new(
        1,
        "TestAccount".into(),
        0,
        2,
        9,
        54261,
        vec![0u8; 40],
        "esES".into(),
        pkt_rx,
        send_tx,
    );
    session.set_item_guid_generator_like_cpp(Arc::new(ObjectGuidGenerator::new(HighGuid::Item, 1)));
    session.set_equipment_set_guid_generator_like_cpp(Arc::new(
        EquipmentSetGuidGeneratorLikeCpp::new(1),
    ));
    (session, send_rx)
}

fn store_with_quests(ids: &[u32]) -> QuestStore {
    QuestStore::from_quests_like_cpp(ids.iter().copied().map(quest_template))
}

fn creature_guid(entry: u32, counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, entry, counter)
}

fn insert_creature(manager: &mut wow_map::MapManager, guid: ObjectGuid, entry: u32) {
    let mut creature = wow_entities::Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(entry);
    creature.unit_mut().world_mut().set_map(571, 0).unwrap();
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::new(10.0, 0.0, 0.0, 0.0));
    creature.unit_mut().set_level(80);
    creature.set_ai_identity_runtime(1, 35, NPCFlags1::QUEST_GIVER.bits(), 0);
    manager
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
}

fn attach_map_manager(session: &mut WorldSession, manager: wow_map::MapManager) {
    session.set_canonical_map_manager(Arc::new(Mutex::new(manager)));
}

fn tracked_query_packet(guids: &[ObjectGuid]) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(guids.len() as u32);
    for guid in guids {
        pkt.write_packed_guid(guid);
    }
    pkt
}

fn recv_status_multiple(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<(ObjectGuid, u64)> {
    let bytes = send_rx
        .try_recv()
        .expect("quest giver status multiple packet");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::QuestGiverStatusMultiple as u16
    );
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    let count = pkt.read_int32().unwrap();
    assert!(count >= 0);
    let mut statuses = Vec::new();
    for _ in 0..count {
        statuses.push((pkt.read_packed_guid().unwrap(), pkt.read_uint64().unwrap()));
    }
    statuses
}

#[tokio::test]
async fn quest_giver_status_tracked_supplied_creature_not_visible_sends_available_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    let mut store = store_with_quests(&[3001]);
    store.starter_quests.entry(9301).or_default().push(3001);
    session.set_quest_store(Arc::new(store));
    let guid = creature_guid(9301, 301);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9301);
    attach_map_manager(&mut session, manager);
    assert!(!session.client_visible_guids_like_cpp.contains(&guid));

    session
        .handle_quest_giver_status_tracked_query(tracked_query_packet(&[guid]))
        .await;

    assert_eq!(
        recv_status_multiple(&send_rx),
        vec![(guid, quest_giver_status::TRIVIAL)]
    );
}
