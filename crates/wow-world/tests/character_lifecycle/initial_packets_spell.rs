use super::fixtures::*;
// External application scenarios migrated with their original assertions.

use super::fixtures::{CharacterLoginLocationLikeCpp, WorldSession};
use std::sync::Arc;
use wow_constants::ServerOpcodes;
use wow_core::guid::HighGuid;
use wow_core::{EquipmentSetGuidGeneratorLikeCpp, ObjectGuid, ObjectGuidGenerator, Position};
use wow_packet::WorldPacket;

fn make_session_with_send_capacity(capacity: usize) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(capacity);
    let mut session = WorldSession::new_character_lifecycle_fixture(
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
    session.character_set_equipment_set_guid_generator_for_test(Arc::new(
        EquipmentSetGuidGeneratorLikeCpp::new(1),
    ));
    (session, send_rx)
}

fn drain_server_opcodes(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<ServerOpcodes> {
    let mut opcodes = Vec::new();
    while let Ok(bytes) = send_rx.try_recv() {
        let packet = WorldPacket::from_bytes(&bytes);
        if let Some(opcode) = packet.server_opcode() {
            opcodes.push(opcode);
        }
    }
    opcodes
}

#[tokio::test]
async fn before_add_spell_packets_keep_cpp_order_without_name_query_injection() {
    let (mut session, send_rx) = make_session_with_send_capacity(64);
    let guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(guid));

    assert!(
        session
            .character_send_initial_packets_before_add_to_map_for_test(
                guid,
                &Position::ZERO,
                571,
                0,
                CharacterLoginLocationLikeCpp {
                    map_id: 571,
                    bind_area_id: Some(0),
                    position: Position::ZERO,
                },
                vec![123],
                Vec::new(),
                Vec::new(),
                Vec::new(),
                [0; 180],
                Vec::new(),
                false,
            )
            .await
    );

    let opcodes = drain_server_opcodes(&send_rx);
    assert!(
        !opcodes.contains(&ServerOpcodes::QueryPlayerNamesResponse),
        "C++ ContactList serialization does not synchronously publish name-query results"
    );
    let expected = [
        ServerOpcodes::ContactList,
        ServerOpcodes::BindPointUpdate,
        ServerOpcodes::UpdateTalentData,
        ServerOpcodes::SendKnownSpells,
        ServerOpcodes::SendUnlearnSpells,
        ServerOpcodes::SendSpellHistory,
        ServerOpcodes::SendSpellCharges,
        ServerOpcodes::ActiveGlyphs,
    ];
    let positions = expected.map(|opcode| {
        opcodes
            .iter()
            .position(|candidate| *candidate == opcode)
            .unwrap_or_else(|| panic!("missing {opcode:?}"))
    });
    assert!(
        positions.windows(2).all(|pair| pair[0] < pair[1]),
        "C++ orders ContactList -> talents -> known/unlearn/history/charges -> ActiveGlyphs"
    );
}
