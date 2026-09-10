//! Quest poi tests for the query module.
//!
//! Separated from query.rs under #687.

use super::*;
use num_traits::ToPrimitive;

fn client_payload(bytes: &[u8]) -> WorldPacket {
    let mut data = Vec::from(ClientOpcodes::QuestPoiQuery.to_u16().unwrap().to_le_bytes());
    data.extend_from_slice(bytes);
    let mut pkt = WorldPacket::from_bytes(&data);
    pkt.skip_opcode();
    pkt
}

#[test]
fn quest_poi_query_reads_fixed_quest_log_array_like_cpp() {
    let mut payload = Vec::new();
    payload.extend_from_slice(&2i32.to_le_bytes());
    for i in 0..QUEST_POI_QUERY_MISSING_QUEST_POIS_LIKE_CPP {
        payload.extend_from_slice(&(1000 + i as i32).to_le_bytes());
    }

    let mut pkt = client_payload(&payload);
    let parsed = QuestPoiQuery::read(&mut pkt).unwrap();

    assert_eq!(parsed.missing_quest_count, 2);
    assert_eq!(parsed.missing_quest_pois[0], 1000);
    assert_eq!(parsed.missing_quest_pois[1], 1001);
    assert_eq!(parsed.missing_quest_pois[24], 1024);
}

#[test]
fn quest_poi_query_reads_real_client_len_106_shape_like_cpp() {
    let mut payload = Vec::new();
    payload.extend_from_slice(&25i32.to_le_bytes());
    for i in 0..25 {
        payload.extend_from_slice(&(2000i32 + i).to_le_bytes());
    }

    let mut pkt = client_payload(&payload);
    let parsed = QuestPoiQuery::read(&mut pkt).unwrap();

    assert_eq!(parsed.missing_quest_count, 25);
    assert_eq!(parsed.missing_quest_pois[0], 2000);
    assert_eq!(parsed.missing_quest_pois[24], 2024);
}

#[test]
fn quest_poi_query_rejects_short_fixed_quest_log_array_like_cpp() {
    let mut payload = Vec::new();
    payload.extend_from_slice(&1i32.to_le_bytes());
    payload.extend_from_slice(&123i32.to_le_bytes());

    let mut pkt = client_payload(&payload);

    assert!(matches!(
        QuestPoiQuery::read(&mut pkt),
        Err(PacketError::ReadPastEnd { .. })
    ));
}

#[test]
fn quest_poi_query_response_serializes_cpp_shape() {
    let response = QuestPoiQueryResponse {
        quest_poi_data_stats: vec![QuestPoiData {
            quest_id: 77,
            blobs: vec![QuestPoiBlobData {
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
                points: vec![QuestPoiBlobPoint {
                    x: 10,
                    y: -11,
                    z: 12,
                }],
                always_allow_merging_blobs: true,
            }],
        }],
    };

    let bytes = response.to_bytes();
    let mut expected = Vec::from(
        ServerOpcodes::QuestPoiQueryResponse
            .to_u16()
            .unwrap()
            .to_le_bytes(),
    );
    expected.extend_from_slice(&1i32.to_le_bytes());
    expected.extend_from_slice(&1i32.to_le_bytes());
    expected.extend_from_slice(&77i32.to_le_bytes());
    expected.extend_from_slice(&1i32.to_le_bytes());
    for value in [1, -1, 2, 3, 571, 486, 4, 5, 6, 7, 8, 9, 1] {
        expected.extend_from_slice(&i32::to_le_bytes(value));
    }
    expected.extend_from_slice(&10i16.to_le_bytes());
    expected.extend_from_slice(&(-11i16).to_le_bytes());
    expected.extend_from_slice(&12i16.to_le_bytes());
    expected.push(0x80);

    assert_eq!(bytes, expected);
}
