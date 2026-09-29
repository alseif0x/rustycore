use crate::session::{SessionPlayerController, WorldSession};
use crate::test_fixtures::set_player_faction_template_for_test;
use std::sync::{Arc, Mutex};
use wow_core::guid::HighGuid;
use wow_core::{EquipmentSetGuidGeneratorLikeCpp, ObjectGuid, ObjectGuidGenerator, Position};
use wow_data::{BankBagSlotPricesEntry, BankBagSlotPricesStore};
use wow_entities::{Creature, MapObjectRecord};
use wow_packet::WorldPacket;

fn make_session_with_send_capacity(
    capacity: usize,
) -> (WorldSession, flume::Receiver<Vec<u8>>) {
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

pub(super) fn make_bank_slot_session(
    capacity: usize,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    Arc<Mutex<wow_map::MapManager>>,
) {
    let (mut session, send_rx) = make_session_with_send_capacity(capacity);
    let canonical = Arc::new(Mutex::new(wow_map::MapManager::new(60_000, 10)));
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
    set_player_faction_template_for_test(&mut session, 1);
    session.set_bank_bag_slot_prices_store(Arc::new(BankBagSlotPricesStore::from_entries([
        BankBagSlotPricesEntry { id: 1, cost: 100 },
        BankBagSlotPricesEntry { id: 2, cost: 200 },
    ])));
    session.set_player_gold_like_cpp(150);
    session.set_player_bank_bag_slot_count_like_cpp(0);
    (session, send_rx, canonical)
}

pub(super) fn insert_binder_creature(
    manager: &Arc<Mutex<wow_map::MapManager>>,
    guid: ObjectGuid,
    npc_flags: u32,
) {
    let mut creature = Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().object_mut().set_entry(2456);
    creature.unit_mut().world_mut().set_map(571, 0).unwrap();
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::new(5.0, 0.0, 0.0, 0.0));
    creature.unit_mut().world_mut().set_combat_reach(1.0);
    creature.unit_mut().set_level(80);
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.set_ai_identity_runtime(1, 35, npc_flags, 0);
    creature.unit_mut().world_mut().object_mut().add_to_world();

    manager
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
}
