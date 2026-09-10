//! Query packet regressions.
//!
//! Moved out of query.rs under #685; every test is unchanged.

use super::*;

#[test]
fn query_creature_response_not_found() {
    let resp = QueryCreatureResponse {
        creature_id: 12345,
        allow: false,
        stats: None,
    };
    let bytes = resp.to_bytes();
    // opcode(2) + creature_id(4) + bit(allow=false, flushed to 1 byte) = 7
    assert_eq!(bytes.len(), 7);
}

#[test]
fn query_corpse_location_reads_full_player_guid_like_cpp() {
    let player = ObjectGuid::create_player(1, 0xAABB_CCDD);
    let mut data = (ClientOpcodes::QueryCorpseLocationFromClient as u16)
        .to_le_bytes()
        .to_vec();
    data.extend_from_slice(&player.to_raw_bytes());
    let mut pkt = WorldPacket::from_bytes(&data);
    pkt.skip_opcode();

    let query = QueryCorpseLocationFromClient::read(&mut pkt).unwrap();

    assert_eq!(query.player, player);
}

#[test]
fn corpse_location_not_found_writes_cpp_shape() {
    let player = ObjectGuid::create_player(1, 0xAABB_CCDD);
    let bytes = CorpseLocation::not_found_like_cpp(player).to_bytes();

    assert_eq!(
        bytes[0..2],
        (ServerOpcodes::CorpseLocation as u16).to_le_bytes()
    );
    assert_eq!(bytes[2], 0x00);
    assert_eq!(&bytes[3..19], &player.to_raw_bytes());
    assert_eq!(&bytes[19..23], &0_i32.to_le_bytes());
    assert_eq!(&bytes[23..27], &0.0_f32.to_le_bytes());
    assert_eq!(&bytes[27..31], &0.0_f32.to_le_bytes());
    assert_eq!(&bytes[31..35], &0.0_f32.to_le_bytes());
    assert_eq!(&bytes[35..39], &0_i32.to_le_bytes());
    assert_eq!(&bytes[39..55], &ObjectGuid::EMPTY.to_raw_bytes());
    assert_eq!(bytes.len(), 55);
}

#[test]
fn query_corpse_transport_reads_full_guids_like_cpp() {
    let player = ObjectGuid::create_player(1, 0xAABB_CCDD);
    let transport = ObjectGuid::create_world_object(HighGuid::Transport, 0, 1, 571, 0, 77, 42);
    let mut data = (ClientOpcodes::QueryCorpseTransport as u16)
        .to_le_bytes()
        .to_vec();
    data.extend_from_slice(&player.to_raw_bytes());
    data.extend_from_slice(&transport.to_raw_bytes());
    let mut pkt = WorldPacket::from_bytes(&data);
    pkt.skip_opcode();

    let query = QueryCorpseTransport::read(&mut pkt).unwrap();

    assert_eq!(query.player, player);
    assert_eq!(query.transport, transport);
}

#[test]
fn corpse_transport_query_not_found_writes_cpp_shape() {
    let player = ObjectGuid::create_player(1, 0xAABB_CCDD);
    let bytes = CorpseTransportQuery::not_found_like_cpp(player).to_bytes();

    assert_eq!(
        bytes[0..2],
        (ServerOpcodes::CorpseTransportQuery as u16).to_le_bytes()
    );
    assert_eq!(&bytes[2..18], &player.to_raw_bytes());
    assert_eq!(&bytes[18..22], &0.0_f32.to_le_bytes());
    assert_eq!(&bytes[22..26], &0.0_f32.to_le_bytes());
    assert_eq!(&bytes[26..30], &0.0_f32.to_le_bytes());
    assert_eq!(&bytes[30..34], &0.0_f32.to_le_bytes());
    assert_eq!(bytes.len(), 34);
}

#[test]
fn query_creature_response_found() {
    let mut stats = CreatureStats::default();
    stats.names[0] = "Test Creature".to_string();
    stats.creature_type = 1; // Beast
    stats.display = CreatureDisplayStats {
        displays: vec![CreatureXDisplay {
            creature_display_id: 856,
            scale: 1.0,
            probability: 1.0,
        }],
        total_probability: 1.0,
    };

    let resp = QueryCreatureResponse {
        creature_id: 100,
        allow: true,
        stats: Some(stats),
    };
    let bytes = resp.to_bytes();
    // Should be a reasonable size (name + fields)
    assert!(
        bytes.len() > 50,
        "Response too small: {} bytes",
        bytes.len()
    );
}

#[test]
fn query_creature_response_empty_string_lengths_match_cpp() {
    let resp = QueryCreatureResponse {
        creature_id: 100,
        allow: true,
        stats: Some(CreatureStats::default()),
    };
    let bytes = resp.to_bytes();

    // C++ writes 118 bits of string-length metadata after Allow, then
    // flushes to 15 bytes even when all strings are empty (length = 1).
    assert_eq!(bytes.len(), 106);
}

#[test]
fn query_game_object_response_not_found() {
    let resp = QueryGameObjectResponse {
        game_object_id: 999,
        guid: ObjectGuid::EMPTY,
        allow: false,
        stats: None,
    };
    let bytes = resp.to_bytes();
    // C++ always writes uint32(statsData.size()), even when Allow=false.
    assert!(bytes.len() > 6);
    assert_eq!(&bytes[bytes.len() - 4..], &[0, 0, 0, 0]);
}

#[test]
fn query_game_object_response_writes_data34_and_content_tuning() {
    let mut stats = GameObjectStats::default();
    stats.names[0] = "Test GameObject".to_string();
    stats.data[34] = 0x1122_3344;
    stats.content_tuning_id = 0x5566_7788;

    let resp = QueryGameObjectResponse {
        game_object_id: 100,
        guid: ObjectGuid::EMPTY,
        allow: true,
        stats: Some(stats),
    };

    let bytes = resp.to_bytes();
    assert!(bytes.windows(4).any(|w| w == 0x1122_3344i32.to_le_bytes()));
    assert!(bytes.windows(4).any(|w| w == 0x5566_7788i32.to_le_bytes()));
}

#[test]
fn query_page_text_reads_cpp_page_id_then_raw_item_guid() {
    let guid = ObjectGuid::create_world_object(wow_core::guid::HighGuid::Item, 0, 1, 0, 0, 7, 9);
    let mut data = (ClientOpcodes::QueryPageText as u16).to_le_bytes().to_vec();
    data.extend_from_slice(&123_u32.to_le_bytes());
    data.extend_from_slice(&guid.to_raw_bytes());
    let mut pkt = WorldPacket::from_bytes(&data);
    pkt.skip_opcode();

    let query = QueryPageText::read(&mut pkt).unwrap();
    assert_eq!(query.page_text_id, 123);
    assert_eq!(query.item_guid, guid);
}

#[test]
fn query_page_text_response_writes_cpp_allow_false_shape() {
    let bytes = QueryPageTextResponse {
        page_text_id: 123,
        allow: false,
        pages: Vec::new(),
    }
    .to_bytes();

    assert_eq!(
        bytes[0..2],
        (ServerOpcodes::QueryPageTextResponse as u16).to_le_bytes()
    );
    assert_eq!(&bytes[2..6], &123_u32.to_le_bytes());
    assert_eq!(bytes[6], 0x00);
    assert_eq!(bytes.len(), 7);
}

#[test]
fn query_page_text_response_writes_pages_like_cpp() {
    let bytes = QueryPageTextResponse {
        page_text_id: 123,
        allow: true,
        pages: vec![PageTextInfo {
            id: 123,
            next_page_id: 124,
            player_condition_id: -5,
            flags: 7,
            text: "abc".to_string(),
        }],
    }
    .to_bytes();

    assert_eq!(
        bytes[0..2],
        (ServerOpcodes::QueryPageTextResponse as u16).to_le_bytes()
    );
    assert_eq!(&bytes[2..6], &123_u32.to_le_bytes());
    assert_eq!(bytes[6], 0x80);
    assert_eq!(&bytes[7..11], &1_u32.to_le_bytes());
    assert_eq!(&bytes[11..15], &123_u32.to_le_bytes());
    assert_eq!(&bytes[15..19], &124_u32.to_le_bytes());
    assert_eq!(&bytes[19..23], &(-5_i32).to_le_bytes());
    assert_eq!(bytes[23], 7);
    assert_eq!(bytes[24], 0x00);
    assert_eq!(bytes[25], 0x30);
    assert_eq!(&bytes[26..29], b"abc");
    assert_eq!(bytes.len(), 29);
}

#[test]
fn item_text_query_reads_cpp_raw_item_guid() {
    let guid = ObjectGuid::create_world_object(wow_core::guid::HighGuid::Item, 0, 1, 0, 0, 7, 9);
    let mut data = (ClientOpcodes::ItemTextQuery as u16).to_le_bytes().to_vec();
    data.extend_from_slice(&guid.to_raw_bytes());
    let mut pkt = WorldPacket::from_bytes(&data);
    pkt.skip_opcode();

    let query = ItemTextQuery::read(&mut pkt).unwrap();
    assert_eq!(query.id, guid);
}

#[test]
fn query_item_text_response_writes_cpp_invalid_shape() {
    let guid = ObjectGuid::create_world_object(wow_core::guid::HighGuid::Item, 0, 1, 0, 0, 7, 9);
    let bytes = QueryItemTextResponse::invalid_like_cpp(guid).to_bytes();

    assert_eq!(
        bytes[0..2],
        (ServerOpcodes::QueryItemTextResponse as u16).to_le_bytes()
    );
    assert_eq!(bytes[2], 0x00);
    assert_eq!(bytes[3], 0x00);
    assert_eq!(bytes[4], 0x00);
    assert_eq!(&bytes[5..21], &guid.to_raw_bytes());
    assert_eq!(bytes.len(), 21);
}

#[test]
fn query_item_text_response_writes_cpp_valid_text_then_id() {
    let guid = ObjectGuid::create_world_object(wow_core::guid::HighGuid::Item, 0, 1, 0, 0, 7, 9);
    let bytes = QueryItemTextResponse::valid_like_cpp(guid, "abc").to_bytes();

    assert_eq!(
        bytes[0..2],
        (ServerOpcodes::QueryItemTextResponse as u16).to_le_bytes()
    );
    assert_eq!(bytes[2], 0x80);
    assert_eq!(bytes[3], 0x00);
    assert_eq!(bytes[4], 0x18);
    assert_eq!(&bytes[5..8], b"abc");
    assert_eq!(&bytes[8..24], &guid.to_raw_bytes());
    assert_eq!(bytes.len(), 24);
}

#[test]
fn query_pet_name_reads_cpp_unit_guid() {
    let guid = ObjectGuid::create_world_object(wow_core::guid::HighGuid::Pet, 0, 1, 571, 0, 7, 9);
    let mut data = (ClientOpcodes::QueryPetName as u16).to_le_bytes().to_vec();
    data.extend_from_slice(&guid.to_raw_bytes());
    let mut pkt = WorldPacket::from_bytes(&data);
    pkt.skip_opcode();

    let query = QueryPetName::read(&mut pkt).unwrap();
    assert_eq!(query.unit_guid, guid);
}

#[test]
fn query_pet_name_response_writes_cpp_allow_false_shape() {
    let guid = ObjectGuid::create_world_object(wow_core::guid::HighGuid::Pet, 0, 1, 571, 0, 7, 9);
    let bytes = QueryPetNameResponse::not_allowed(guid).to_bytes();

    assert_eq!(
        bytes[0..2],
        (ServerOpcodes::QueryPetNameResponse as u16).to_le_bytes()
    );
    assert_eq!(&bytes[2..18], &guid.to_raw_bytes());
    assert_eq!(bytes[18], 0x00);
    assert_eq!(bytes.len(), 19);
}

#[test]
fn query_pet_name_response_writes_name_timestamp_and_declined_like_cpp() {
    let guid = ObjectGuid::create_world_object(wow_core::guid::HighGuid::Pet, 0, 1, 571, 0, 7, 9);
    let response = QueryPetNameResponse {
        unit_guid: guid,
        allow: true,
        has_declined: true,
        declined_names: PetDeclinedNamesLikeCpp {
            names: [
                "Alpha".to_string(),
                "Beta".to_string(),
                "Gamma".to_string(),
                "Delta".to_string(),
                "Epsilon".to_string(),
            ],
        },
        timestamp: 123_456,
        name: "Misha".to_string(),
    };

    let bytes = response.to_bytes();
    assert_eq!(
        bytes[0..2],
        (ServerOpcodes::QueryPetNameResponse as u16).to_le_bytes()
    );
    assert_eq!(&bytes[2..18], &guid.to_raw_bytes());
    assert_eq!(bytes[18] & 0x80, 0x80);
    assert!(bytes.windows(4).any(|w| w == 123_456_u32.to_le_bytes()));
    assert!(bytes.windows(5).any(|w| w == b"Misha"));
    assert!(bytes.windows(5).any(|w| w == b"Alpha"));
    assert!(bytes.windows(7).any(|w| w == b"Epsilon"));
}

#[test]
fn query_player_names_response_found() {
    let guid = ObjectGuid::create_player(1, 42);
    let data = PlayerGuidLookupData {
        name: "TestPlayer".to_string(),
        race: 1,
        sex: 0,
        class: 1,
        level: 10,
        guid_actual: guid,
        account_id: ObjectGuid::EMPTY,
        bnet_account_id: ObjectGuid::EMPTY,
        virtual_realm_address: 0x0100_0001,
        ..Default::default()
    };
    let resp = QueryPlayerNamesResponse {
        players: vec![NameCacheLookupResult {
            player: guid,
            result: 0,
            data: Some(data),
        }],
    };
    let bytes = resp.to_bytes();
    assert!(
        bytes.len() > 20,
        "Response too small: {} bytes",
        bytes.len()
    );
}
