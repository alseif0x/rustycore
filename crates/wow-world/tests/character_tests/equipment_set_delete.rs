use std::sync::Arc;

use wow_core::guid::HighGuid;
use wow_core::{EquipmentSetGuidGeneratorLikeCpp, ObjectGuidGenerator};
use wow_packet::WorldPacket;
use wow_world::session::WorldSession;
use wow_world::test_fixtures::{
    insert_represented_equipment_set_for_test, represented_equipment_set_for_test,
    set_equipment_set_guid_generator_for_test,
};

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
    set_equipment_set_guid_generator_for_test(
        &mut session,
        Arc::new(EquipmentSetGuidGeneratorLikeCpp::new(1)),
    );
    (session, send_rx)
}

fn delete_equipment_set_packet(id: u64) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint64(id);
    pkt
}

#[tokio::test]
async fn delete_equipment_set_marks_existing_set_deleted_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    insert_represented_equipment_set_for_test(
        &mut session,
        100,
        wow_entities::PlayerEquipmentSetLikeCpp::equipment(
            7,
            -1,
            wow_entities::PlayerEquipmentSetUpdateStateLikeCpp::Unchanged,
        ),
    );

    session
        .handle_delete_equipment_set(delete_equipment_set_packet(100))
        .await;

    let equipment_set = represented_equipment_set_for_test(&session, 100).unwrap();
    assert_eq!(
        equipment_set.state,
        wow_entities::PlayerEquipmentSetUpdateStateLikeCpp::Deleted
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn delete_equipment_set_removes_new_set_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    insert_represented_equipment_set_for_test(
        &mut session,
        100,
        wow_entities::PlayerEquipmentSetLikeCpp::equipment(
            7,
            -1,
            wow_entities::PlayerEquipmentSetUpdateStateLikeCpp::New,
        ),
    );

    session
        .handle_delete_equipment_set(delete_equipment_set_packet(100))
        .await;

    assert!(represented_equipment_set_for_test(&session, 100).is_none());
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn delete_equipment_set_missing_id_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);

    session
        .handle_delete_equipment_set(delete_equipment_set_packet(404))
        .await;

    assert!(send_rx.try_recv().is_err());
}
