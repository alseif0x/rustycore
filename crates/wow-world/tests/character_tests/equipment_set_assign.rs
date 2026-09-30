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

fn assign_equipment_set_spec_packet(set_id: u32, spec_index: u32) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(set_id);
    pkt.write_uint32(spec_index);
    pkt
}

#[tokio::test]
async fn assign_equipment_set_spec_updates_matching_equipment_set_like_cpp() {
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
        .handle_assign_equipment_set_spec(assign_equipment_set_spec_packet(7, 2))
        .await;

    let equipment_set = represented_equipment_set_for_test(&session, 100).unwrap();
    assert_eq!(equipment_set.assigned_spec_index, 2);
    assert_eq!(
        equipment_set.state,
        wow_entities::PlayerEquipmentSetUpdateStateLikeCpp::Changed
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn assign_equipment_set_spec_preserves_new_state_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
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
        .handle_assign_equipment_set_spec(assign_equipment_set_spec_packet(7, 3))
        .await;

    let equipment_set = represented_equipment_set_for_test(&session, 100).unwrap();
    assert_eq!(equipment_set.assigned_spec_index, 3);
    assert_eq!(
        equipment_set.state,
        wow_entities::PlayerEquipmentSetUpdateStateLikeCpp::New
    );
}
