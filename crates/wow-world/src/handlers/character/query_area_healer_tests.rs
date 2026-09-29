use std::sync::Arc;

use crate::session::{SessionPlayerController, WorldSession};
use crate::test_fixtures::set_equipment_set_guid_generator_for_test;
use wow_constants::unit::NPCFlags1;
use wow_core::guid::HighGuid;
use wow_core::{EquipmentSetGuidGeneratorLikeCpp, ObjectGuid, ObjectGuidGenerator, Position};
use wow_packet::WorldPacket;

fn make_area_spirit_healer_session(
    capacity: usize,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    Arc<std::sync::Mutex<wow_map::MapManager>>,
) {
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
    set_equipment_set_guid_generator_for_test(
        &mut session,
        Arc::new(EquipmentSetGuidGeneratorLikeCpp::new(1)),
    );
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 10)));
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(0.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_player_alive_like_cpp(false);
    (session, send_rx, canonical)
}

fn insert_area_spirit_healer_creature(
    manager: &Arc<std::sync::Mutex<wow_map::MapManager>>,
    guid: ObjectGuid,
    position: Position,
    npc_flags: u32,
    npc_flags2: u32,
) {
    let mut creature = wow_entities::Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().object_mut().set_entry(91);
    creature.unit_mut().world_mut().set_map(571, 0).unwrap();
    creature.unit_mut().world_mut().relocate(position);
    creature.unit_mut().world_mut().set_combat_reach(1.0);
    creature.unit_mut().set_level(80);
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.set_ai_identity_runtime(1, 35, npc_flags, 0);
    creature.set_npc_flags2_runtime_like_cpp(npc_flags2);
    creature.unit_mut().world_mut().object_mut().add_to_world();

    manager
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
}

#[tokio::test]
async fn area_spirit_healer_query_sends_time_for_valid_healer_like_cpp() {
    let (mut session, send_rx, canonical) = make_area_spirit_healer_session(4);
    let healer = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 91, 1);
    insert_area_spirit_healer_creature(
        &canonical,
        healer,
        Position::new(10.0, 0.0, 0.0, 0.0),
        NPCFlags1::AREA_SPIRIT_HEALER.bits(),
        0,
    );
    let mut request = WorldPacket::new_empty();
    request.write_packed_guid(&healer);

    session.handle_area_spirit_healer_query(request).await;

    let bytes = send_rx.try_recv().expect("area spirit healer time");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::AreaSpiritHealerTime as u16
    );
    let mut body = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(body.read_packed_guid().unwrap(), healer);
    assert_eq!(body.read_int32().unwrap(), 0);
}

#[tokio::test]
async fn area_spirit_healer_query_rejects_out_of_range_healer_like_cpp() {
    let (mut session, send_rx, canonical) = make_area_spirit_healer_session(1);
    let healer = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 91, 2);
    insert_area_spirit_healer_creature(
        &canonical,
        healer,
        Position::new(20.1, 0.0, 0.0, 0.0),
        NPCFlags1::AREA_SPIRIT_HEALER.bits(),
        0,
    );
    let mut request = WorldPacket::new_empty();
    request.write_packed_guid(&healer);

    session.handle_area_spirit_healer_query(request).await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn area_spirit_healer_queue_records_valid_healer_like_cpp() {
    let (mut session, send_rx, canonical) = make_area_spirit_healer_session(1);
    let healer = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 91, 3);
    insert_area_spirit_healer_creature(
        &canonical,
        healer,
        Position::new(10.0, 0.0, 0.0, 0.0),
        NPCFlags1::AREA_SPIRIT_HEALER.bits(),
        0,
    );
    let mut request = WorldPacket::new_empty();
    request.write_packed_guid(&healer);

    session.handle_area_spirit_healer_queue(request).await;

    assert_eq!(session.area_spirit_healer_guid_like_cpp(), Some(healer));
    assert!(send_rx.try_recv().is_err());
}
