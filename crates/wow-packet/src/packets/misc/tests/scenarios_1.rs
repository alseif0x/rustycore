//! Miscellaneous packet regressions, part 1 of 6.
//!
//! Moved out of the tests.rs root under #640; every test is unchanged.

use super::*;

#[test]
fn auction_hello_response_writes_cpp_layout_without_auction_house_id() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 123, 1);
    let bytes = AuctionHelloResponse::open(guid).to_bytes();
    let mut packet = WorldPacket::from_bytes(&bytes);

    assert_eq!(
        packet.read_uint16().expect("opcode"),
        ServerOpcodes::AuctionHelloResponse as u16
    );
    assert_eq!(packet.read_packed_guid().expect("auctioneer guid"), guid);
    assert_eq!(packet.read_uint32().expect("purchased delivery delay"), 0);
    assert_eq!(packet.read_uint32().expect("cancelled delivery delay"), 0);
    assert!(packet.read_bit().expect("open for business"));
    assert!(
        packet.is_empty(),
        "C++ AuctionHelloResponse does not serialize AuctionHouseID"
    );
}

#[test]
fn show_trade_skill_reads_null_like_cpp() {
    let mut pkt = WorldPacket::from_bytes(&[0x01, 0x02, 0x03, 0x04]);

    ShowTradeSkill::read(&mut pkt).expect("ShowTradeSkill null packet");

    assert!(pkt.is_empty());
}

#[test]
fn can_duel_reads_raw_guid_then_bit_like_cpp() {
    let guid = ObjectGuid::create_player(1, 42);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bytes(&guid.to_raw_bytes());
    pkt.write_bit(true);
    pkt.flush_bits();
    pkt.reset_read();

    let parsed = CanDuel::read(&mut pkt).unwrap();

    assert_eq!(parsed.target_guid, guid);
    assert!(parsed.to_the_death);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn can_duel_result_writes_raw_guid_then_bit_like_cpp() {
    let guid = ObjectGuid::create_player(1, 42);
    let packet = CanDuelResult {
        target_guid: guid,
        result: true,
    };
    let bytes = packet.to_bytes();
    let mut body = WorldPacket::from_bytes(&bytes);

    assert_eq!(body.server_opcode(), Some(ServerOpcodes::CanDuelResult));
    assert_eq!(
        body.read_uint16().unwrap(),
        ServerOpcodes::CanDuelResult as u16
    );
    let guid_bytes = body.read_bytes(16).unwrap();
    let mut raw = [0u8; 16];
    raw.copy_from_slice(&guid_bytes);
    assert_eq!(ObjectGuid::from_raw_bytes(&raw), guid);
    assert!(body.read_bit().unwrap());
    assert_eq!(body.remaining(), 0);
}

#[test]
fn duel_response_reads_raw_arbiter_guid_then_bits_like_cpp() {
    let arbiter_guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 9, 1);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bytes(&arbiter_guid.to_raw_bytes());
    pkt.write_bit(true);
    pkt.write_bit(false);
    pkt.flush_bits();
    pkt.reset_read();

    let parsed = DuelResponse::read(&mut pkt).unwrap();

    assert_eq!(parsed.arbiter_guid, arbiter_guid);
    assert!(parsed.accepted);
    assert!(!parsed.forfeited);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn duel_countdown_writes_uint32_like_cpp() {
    let bytes = DuelCountdown { countdown_ms: 3000 }.to_bytes();
    let mut body = WorldPacket::from_bytes(&bytes);

    assert_eq!(body.server_opcode(), Some(ServerOpcodes::DuelCountdown));
    assert_eq!(
        body.read_uint16().unwrap(),
        ServerOpcodes::DuelCountdown as u16
    );
    assert_eq!(body.read_uint32().unwrap(), 3000);
    assert_eq!(body.remaining(), 0);
}

#[test]
fn duel_requested_writes_three_raw_guids_then_bit_like_cpp() {
    let arbiter_guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 9, 1);
    let requested_by_guid = ObjectGuid::create_player(1, 42);
    let requested_by_wow_account = ObjectGuid::create_global(HighGuid::WowAccount, 0, 7);
    let bytes = DuelRequested {
        arbiter_guid,
        requested_by_guid,
        requested_by_wow_account,
        to_the_death: true,
    }
    .to_bytes();
    let mut body = WorldPacket::from_bytes(&bytes);

    assert_eq!(body.server_opcode(), Some(ServerOpcodes::DuelRequested));
    assert_eq!(
        body.read_uint16().unwrap(),
        ServerOpcodes::DuelRequested as u16
    );
    for expected in [arbiter_guid, requested_by_guid, requested_by_wow_account] {
        let guid_bytes = body.read_bytes(16).unwrap();
        let mut raw = [0u8; 16];
        raw.copy_from_slice(&guid_bytes);
        assert_eq!(ObjectGuid::from_raw_bytes(&raw), expected);
    }
    assert!(body.read_bit().unwrap());
    assert_eq!(body.remaining(), 0);
}

#[test]
fn account_data_times_global() {
    let pkt = AccountDataTimes::global();
    let bytes = pkt.to_bytes();
    // opcode(2) + packed empty ObjectGuid(2) + server_time(8) + 15*i64(120) = 132
    assert_eq!(bytes.len(), 132);

    let mut body = WorldPacket::from_bytes(&bytes);
    assert_eq!(
        body.read_uint16().unwrap(),
        ServerOpcodes::AccountDataTimes as u16
    );
    assert_eq!(body.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    let _server_time = body.read_int64().unwrap();
    for _ in 0..NUM_ACCOUNT_DATA_TYPES {
        assert_eq!(body.read_int64().unwrap(), 0);
    }
    assert_eq!(body.remaining(), 0);
}

#[test]
fn account_data_times_player() {
    let guid = ObjectGuid::create_player(1, 42);
    let pkt = AccountDataTimes::for_player(guid);
    let bytes = pkt.to_bytes();

    let mut body = WorldPacket::from_bytes(&bytes);
    assert_eq!(
        body.read_uint16().unwrap(),
        ServerOpcodes::AccountDataTimes as u16
    );
    assert_eq!(body.read_packed_guid().unwrap(), guid);
    let _server_time = body.read_int64().unwrap();
    for _ in 0..NUM_ACCOUNT_DATA_TYPES {
        assert_eq!(body.read_int64().unwrap(), 0);
    }
    assert_eq!(body.remaining(), 0);
}

#[test]
fn repop_request_reads_check_instance_bit_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(true);
    pkt.flush_bits();
    pkt.reset_read();

    let parsed = RepopRequest::read(&mut pkt).unwrap();

    assert!(parsed.check_instance);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn port_graveyard_reads_empty_packet_like_cpp() {
    let mut pkt = WorldPacket::new_empty();

    let parsed = PortGraveyard::read(&mut pkt).unwrap();

    assert_eq!(parsed, PortGraveyard);
    assert_eq!(pkt.remaining(), 0);

    let mut non_empty = WorldPacket::new_empty();
    non_empty.write_uint8(1);
    non_empty.reset_read();
    assert!(PortGraveyard::read(&mut non_empty).is_err());
}

#[test]
fn reclaim_corpse_reads_raw_corpse_guid_like_cpp() {
    let corpse_guid = ObjectGuid::create_world_object(HighGuid::Corpse, 0, 1, 571, 0, 0, 42);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bytes(&corpse_guid.to_raw_bytes());
    pkt.reset_read();

    let parsed = ReclaimCorpse::read(&mut pkt).unwrap();

    assert_eq!(parsed.corpse_guid, corpse_guid);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn request_account_data_reads_cpp_shape() {
    let guid = ObjectGuid::create_player(1, 42);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&guid);
    pkt.write_bits(7, 4);
    pkt.flush_bits();
    pkt.reset_read();

    let parsed = RequestAccountData::read(&mut pkt).unwrap();

    assert_eq!(parsed.player_guid, guid);
    assert_eq!(parsed.data_type, 7);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn user_client_update_account_data_reads_cpp_shape() {
    let guid = ObjectGuid::create_player(1, 42);
    let compressed_data = compress_account_data_like_cpp("layout-cache").unwrap();
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&guid);
    pkt.write_int64(1234);
    pkt.write_uint32("layout-cache".len() as u32);
    pkt.write_bits(6, 4);
    pkt.write_uint32(compressed_data.len() as u32);
    pkt.write_bytes(&compressed_data);
    pkt.reset_read();

    let parsed = UserClientUpdateAccountData::read(&mut pkt).unwrap();

    assert_eq!(parsed.player_guid, guid);
    assert_eq!(parsed.time, 1234);
    assert_eq!(parsed.size, "layout-cache".len() as u32);
    assert_eq!(parsed.data_type, 6);
    assert_eq!(parsed.compressed_data, compressed_data);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn update_account_data_writes_cpp_shape_and_roundtrips_zlib_cstring() {
    let guid = ObjectGuid::create_player(1, 42);
    let payload = "cache body without nul";
    let compressed_data = compress_account_data_like_cpp(payload).unwrap();
    let pkt = UpdateAccountData {
        player_guid: guid,
        time: 5678,
        size: payload.len() as u32,
        data_type: 4,
        compressed_data: compressed_data.clone(),
    };
    let encoded = pkt.to_bytes();
    let mut bytes = WorldPacket::new_client(encoded.as_slice().into());
    bytes.skip_opcode();

    assert_eq!(bytes.read_packed_guid().unwrap(), guid);
    assert_eq!(bytes.read_int64().unwrap(), 5678);
    assert_eq!(bytes.read_uint32().unwrap(), payload.len() as u32);
    assert_eq!(bytes.read_bits(4).unwrap(), 4);
    assert_eq!(bytes.read_uint32().unwrap(), compressed_data.len() as u32);
    assert_eq!(
        bytes.read_bytes(compressed_data.len()).unwrap(),
        compressed_data
    );
    assert_eq!(
        decompress_account_data_like_cpp(&pkt.compressed_data, pkt.size).unwrap(),
        payload
    );
    assert_eq!(bytes.remaining(), 0);
}

#[test]
fn loading_screen_notify_reads_cpp_map_and_showing_bit() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(571);
    pkt.write_bit(true);
    pkt.flush_bits();
    pkt.reset_read();

    let parsed = LoadingScreenNotify::read(&mut pkt).unwrap();
    assert_eq!(parsed.map_id, 571);
    assert!(parsed.showing);
}

#[test]
fn set_taxi_benchmark_mode_reads_cpp_enable_bit() {
    for enable in [false, true] {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(enable);
        pkt.flush_bits();
        pkt.reset_read();

        let parsed = SetTaxiBenchmarkMode::read(&mut pkt).unwrap();
        assert_eq!(parsed.enable, enable);
    }
}

#[test]
fn activate_taxi_reads_cpp_vendor_node_ground_and_flying_mount_order() {
    let vendor = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::Creature,
        0,
        1,
        571,
        0,
        9,
        12_345,
    );
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&vendor);
    pkt.write_uint32(7);
    pkt.write_uint32(111);
    pkt.write_uint32(222);
    pkt.reset_read();

    let parsed = ActivateTaxi::read(&mut pkt).unwrap();

    assert_eq!(parsed.vendor, vendor);
    assert_eq!(parsed.node, 7);
    assert_eq!(parsed.ground_mount_id, 111);
    assert_eq!(parsed.flying_mount_id, 222);
}

#[test]
fn activate_taxi_reply_writes_cpp_four_bit_reply() {
    let bytes = ActivateTaxiReply { reply: 4 }.to_bytes();

    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::ActivateTaxiReply as u16
    );
    let mut payload = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(payload.read_bits(4).unwrap(), 4);
}

#[test]
fn set_advanced_combat_logging_reads_cpp_enable_bit() {
    for enable in [false, true] {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(enable);
        pkt.flush_bits();
        pkt.reset_read();

        let parsed = SetAdvancedCombatLogging::read(&mut pkt).unwrap();
        assert_eq!(parsed.enable, enable);
    }
}

#[test]
fn set_currency_flags_reads_cpp_uint32_then_uint8() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(395);
    pkt.write_uint8(0x1f);
    pkt.reset_read();

    let parsed = SetCurrencyFlags::read(&mut pkt).unwrap();
    assert_eq!(parsed.currency_id, 395);
    assert_eq!(parsed.flags, 0x1f);
}

#[test]
fn random_roll_client_reads_optional_party_index_then_signed_bounds_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(true);
    pkt.write_int32(1);
    pkt.write_int32(100);
    pkt.write_uint8(0);
    pkt.reset_read();

    let parsed = RandomRollClient::read(&mut pkt).unwrap();

    assert_eq!(
        parsed,
        RandomRollClient {
            min: 1,
            max: 100,
            party_index: Some(0),
        }
    );
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn random_roll_client_reads_absent_party_index_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(false);
    pkt.write_int32(-5);
    pkt.write_int32(5);
    pkt.reset_read();

    let parsed = RandomRollClient::read(&mut pkt).unwrap();

    assert_eq!(
        parsed,
        RandomRollClient {
            min: -5,
            max: 5,
            party_index: None,
        }
    );
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn random_roll_writes_full_guids_then_signed_values_like_cpp() {
    let roller = ObjectGuid::create_player(1, 42);
    let account = ObjectGuid::new((HighGuid::WowAccount as i64) << 58, 7);
    let bytes = RandomRoll {
        roller,
        roller_wow_account: account,
        min: 1,
        max: 100,
        result: 77,
    }
    .to_bytes();
    let mut pkt = WorldPacket::from_bytes(&bytes);

    assert_eq!(pkt.server_opcode(), Some(ServerOpcodes::RandomRoll));
    assert_eq!(pkt.read_uint16().unwrap(), ServerOpcodes::RandomRoll as u16);
    assert_eq!(pkt.read_guid().unwrap(), roller);
    assert_eq!(pkt.read_guid().unwrap(), account);
    assert_eq!(pkt.read_int32().unwrap(), 1);
    assert_eq!(pkt.read_int32().unwrap(), 100);
    assert_eq!(pkt.read_int32().unwrap(), 77);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn set_difficulty_id_reads_cpp_uint32() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(23);
    pkt.reset_read();

    let parsed = SetDifficultyId::read(&mut pkt).unwrap();

    assert_eq!(parsed.difficulty_id, 23);
}

#[test]
fn toggle_difficulty_reads_cpp_null_packet() {
    let mut pkt = WorldPacket::new_empty();

    let parsed = ToggleDifficulty::read(&mut pkt).unwrap();

    assert_eq!(parsed, ToggleDifficulty);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn set_dungeon_difficulty_reads_cpp_uint32() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(2);
    pkt.reset_read();

    let parsed = SetDungeonDifficulty::read(&mut pkt).unwrap();

    assert_eq!(parsed.difficulty_id, 2);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn set_raid_difficulty_reads_cpp_int32_then_legacy_u8() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(4);
    pkt.write_uint8(1);
    pkt.reset_read();

    let parsed = SetRaidDifficulty::read(&mut pkt).unwrap();

    assert_eq!(parsed.difficulty_id, 4);
    assert_eq!(parsed.legacy, 1);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn addon_list_reads_cpp_count_bits_flush_and_names() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(3);
    pkt.write_bits(5, 10);
    pkt.flush_bits();
    pkt.write_string("Atlas");
    pkt.write_bits(7, 10);
    pkt.flush_bits();
    pkt.write_string("Questie");
    pkt.reset_read();

    let parsed = AddonList::read(&mut pkt).unwrap();
    assert_eq!(parsed.addons, vec!["Atlas", "Questie"]);
}

#[test]
fn violence_level_reads_cpp_uint8() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint8(2);
    pkt.reset_read();

    let parsed = ViolenceLevel::read(&mut pkt).unwrap();
    assert_eq!(parsed.violence_level, 2);
}

#[test]
fn decline_guild_invites_reads_cpp_allow_bit() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(true);
    pkt.flush_bits();
    pkt.reset_read();

    let parsed = DeclineGuildInvites::read(&mut pkt).unwrap();
    assert!(parsed.allow);
}

#[test]
fn decline_guild_invites_rejects_missing_allow_bit() {
    let mut pkt = WorldPacket::new_empty();

    assert!(DeclineGuildInvites::read(&mut pkt).is_err());
}

#[test]
fn accept_guild_invite_reads_empty_cpp_packet() {
    let mut pkt = WorldPacket::new_empty();

    AcceptGuildInvite::read(&mut pkt).unwrap();
}

#[test]
fn guild_set_achievement_tracking_reads_cpp_counted_ids() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(3);
    pkt.write_uint32(100);
    pkt.write_uint32(200);
    pkt.write_uint32(300);
    pkt.reset_read();

    let parsed = GuildSetAchievementTracking::read(&mut pkt).unwrap();
    assert_eq!(parsed.achievement_ids, vec![100, 200, 300]);
}

#[test]
fn guild_set_achievement_tracking_rejects_above_cpp_array_limit() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32((MAX_GUILD_ACHIEVEMENT_TRACKING_IDS_LIKE_CPP + 1) as u32);
    pkt.reset_read();

    assert!(GuildSetAchievementTracking::read(&mut pkt).is_err());
}

#[test]
fn close_interaction_reads_cpp_source_guid() {
    let source_guid = ObjectGuid::create_player(1, 42);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&source_guid);
    pkt.reset_read();

    let parsed = CloseInteraction::read(&mut pkt).unwrap();
    assert_eq!(parsed.source_guid, source_guid);
}

#[test]
fn rated_pvp_info_empty_matches_cpp_default_shape() {
    let bytes = RatedPvpInfo::default().to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::RatedPvpInfo as u16
    );
    assert_eq!(
        bytes.len(),
        2 + RATED_PVP_BRACKET_COUNT_LIKE_CPP * (19 * 4 + 1)
    );

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    for _ in 0..RATED_PVP_BRACKET_COUNT_LIKE_CPP {
        for _ in 0..19 {
            assert_eq!(pkt.read_int32().unwrap(), 0);
        }
        assert!(!pkt.has_bit().unwrap());
    }
}

#[test]
fn request_battlefield_status_reads_empty_cpp_packet() {
    let mut pkt = WorldPacket::new_empty();
    RequestBattlefieldStatus::read(&mut pkt).unwrap();
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn lfg_update_status_removed_from_queue_matches_cpp_empty_branch() {
    let bytes = LfgUpdateStatus::removed_from_queue().to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::LfgUpdateStatus as u16
    );

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    assert_eq!(pkt.read_int64().unwrap(), 0);
    assert!(!pkt.has_bit().unwrap());
    assert_eq!(pkt.read_uint8().unwrap(), LFG_QUEUE_DUNGEON_LIKE_CPP);
    assert_eq!(
        pkt.read_uint8().unwrap(),
        LFG_UPDATE_TYPE_REMOVED_FROM_QUEUE_LIKE_CPP
    );
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    assert_eq!(pkt.read_uint8().unwrap(), 0);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    assert!(!pkt.has_bit().unwrap());
    assert!(pkt.has_bit().unwrap());
    assert!(!pkt.has_bit().unwrap());
    assert!(!pkt.has_bit().unwrap());
    assert!(!pkt.has_bit().unwrap());
    assert!(!pkt.has_bit().unwrap());
}

#[test]
fn lfg_list_blacklist_empty_matches_cpp_shape() {
    let bytes = LfgListBlacklist::empty().to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::LfgListUpdateBlacklist as u16
    );
    assert_eq!(bytes.len(), 2 + 4);

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
}

#[test]
fn lfg_list_blacklist_entry_matches_cpp_order() {
    let bytes = LfgListBlacklist {
        entries: vec![LfgListBlacklistEntry {
            slot: 42,
            reason: 3,
            sub_reason1: 123,
            sub_reason2: -7,
            soft_lock: 0,
        }],
    }
    .to_bytes();

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), 1);
    assert_eq!(pkt.read_uint32().unwrap(), 42);
    assert_eq!(pkt.read_uint32().unwrap(), 3);
    assert_eq!(pkt.read_int32().unwrap(), 123);
    assert_eq!(pkt.read_int32().unwrap(), -7);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
}

#[test]
fn df_get_system_info_reads_cpp_bits() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(true); // Player
    pkt.write_bit(true); // PartyIndex.HasValue
    pkt.write_uint8(7);

    let request = DfGetSystemInfo::read(&mut pkt).unwrap();
    assert!(request.player);
    assert_eq!(request.party_index, Some(7));
}

#[test]
fn df_get_join_status_reads_empty_cpp_packet() {
    let mut pkt = WorldPacket::new_empty();
    DfGetJoinStatus::read(&mut pkt).unwrap();
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn toggle_pvp_reads_empty_cpp_packet() {
    let mut pkt = WorldPacket::new_empty();
    TogglePvp::read(&mut pkt).unwrap();
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn set_pvp_reads_cpp_enable_bit() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(true);
    pkt.flush_bits();
    pkt.reset_read();

    let parsed = SetPvp::read(&mut pkt).unwrap();

    assert!(parsed.enable_pvp);
}

#[test]
fn assign_equipment_set_spec_reads_cpp_uint32_pair() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(7);
    pkt.write_uint32(2);
    pkt.reset_read();

    let parsed = AssignEquipmentSetSpec::read(&mut pkt).unwrap();

    assert_eq!(parsed.set_id, 7);
    assert_eq!(parsed.spec_index, 2);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn save_equipment_set_reads_cpp_equipment_set_data_shape() {
    let item_guid = ObjectGuid::create_item(1, 55);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(0);
    pkt.write_uint64(0x0102_0304_0506_0708);
    pkt.write_uint32(7);
    pkt.write_uint32(0);
    for i in 0..EQUIPMENT_SET_SLOTS_LIKE_CPP {
        let guid = if i == 0 { item_guid } else { ObjectGuid::EMPTY };
        pkt.write_guid(&guid);
        pkt.write_int32(i as i32 + 10);
    }
    pkt.write_int32(123);
    pkt.write_int32(456);
    pkt.write_int32(11);
    pkt.write_int32(2);
    pkt.write_int32(22);
    pkt.write_int32(16);
    pkt.write_bit(true);
    pkt.write_bits(4, 8);
    pkt.write_bits(6, 9);
    pkt.write_int32(3);
    pkt.write_string("Tank");
    pkt.write_string("INV_01");
    pkt.reset_read();

    let parsed = SaveEquipmentSet::read(&mut pkt).unwrap();

    assert_eq!(parsed.set.set_type, 0);
    assert_eq!(parsed.set.guid, 0x0102_0304_0506_0708);
    assert_eq!(parsed.set.set_id, 7);
    assert_eq!(parsed.set.pieces[0], item_guid);
    assert_eq!(parsed.set.appearances[2], 12);
    assert_eq!(parsed.set.enchants, [123, 456]);
    assert_eq!(parsed.set.secondary_shoulder_appearance_id, 11);
    assert_eq!(parsed.set.secondary_shoulder_slot, 2);
    assert_eq!(parsed.set.secondary_weapon_appearance_id, 22);
    assert_eq!(parsed.set.secondary_weapon_slot, 16);
    assert_eq!(parsed.set.assigned_spec_index, 3);
    assert_eq!(parsed.set.set_name, "Tank");
    assert_eq!(parsed.set.set_icon, "INV_01");
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn load_equipment_set_writes_cpp_equipment_set_data_shape() {
    let item_guid = ObjectGuid::create_item(1, 55);
    let mut pieces = [ObjectGuid::EMPTY; EQUIPMENT_SET_SLOTS_LIKE_CPP];
    pieces[0] = item_guid;
    let mut appearances = [0; EQUIPMENT_SET_SLOTS_LIKE_CPP];
    appearances[2] = 12;

    let pkt = LoadEquipmentSet {
        sets: vec![EquipmentSetDataLikeCpp {
            set_type: 0,
            guid: 0x0102_0304_0506_0708,
            set_id: 7,
            ignore_mask: 3,
            pieces,
            appearances,
            enchants: [123, 456],
            secondary_shoulder_appearance_id: 11,
            secondary_shoulder_slot: 2,
            secondary_weapon_appearance_id: 22,
            secondary_weapon_slot: 16,
            assigned_spec_index: 3,
            set_name: "Tank".to_string(),
            set_icon: "INV_01".to_string(),
        }],
    };
    let bytes = pkt.to_bytes();
    let mut body = WorldPacket::from_bytes(&bytes[2..]);

    assert_eq!(u32::try_from(body.read_int32().unwrap()).unwrap(), 1);
    assert_eq!(body.read_int32().unwrap(), 0);
    assert_eq!(body.read_uint64().unwrap(), 0x0102_0304_0506_0708);
    assert_eq!(body.read_uint32().unwrap(), 7);
    assert_eq!(body.read_uint32().unwrap(), 3);
    assert_eq!(body.read_guid().unwrap(), item_guid);
    assert_eq!(body.read_int32().unwrap(), 0);
    for i in 1..EQUIPMENT_SET_SLOTS_LIKE_CPP {
        assert_eq!(body.read_guid().unwrap(), ObjectGuid::EMPTY);
        assert_eq!(body.read_int32().unwrap(), if i == 2 { 12 } else { 0 });
    }
    assert_eq!(body.read_int32().unwrap(), 123);
    assert_eq!(body.read_int32().unwrap(), 456);
    assert_eq!(body.read_int32().unwrap(), 11);
    assert_eq!(body.read_int32().unwrap(), 2);
    assert_eq!(body.read_int32().unwrap(), 22);
    assert_eq!(body.read_int32().unwrap(), 16);
    assert!(body.read_bit().unwrap());
    assert_eq!(body.read_bits(8).unwrap(), 4);
    assert_eq!(body.read_bits(9).unwrap(), 6);
    assert_eq!(body.read_int32().unwrap(), 3);
    assert_eq!(body.read_string(4).unwrap(), "Tank");
    assert_eq!(body.read_string(6).unwrap(), "INV_01");
    assert_eq!(body.remaining(), 0);
}

#[test]
fn delete_equipment_set_reads_cpp_uint64_id() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint64(0x0102_0304_0506_0708);
    pkt.reset_read();

    let parsed = DeleteEquipmentSet::read(&mut pkt).unwrap();

    assert_eq!(parsed.id, 0x0102_0304_0506_0708);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn use_equipment_set_reads_cpp_inv_items_and_guid() {
    let item_guid = ObjectGuid::create_item(1, 55);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bits(1, 2);
    pkt.write_uint8(255);
    pkt.write_uint8(36);
    for i in 0..EQUIPMENT_SET_SLOTS_LIKE_CPP {
        let guid = if i == 0 { item_guid } else { ObjectGuid::EMPTY };
        pkt.write_guid(&guid);
        pkt.write_uint8(255);
        pkt.write_uint8(i as u8);
    }
    pkt.write_uint64(0x0102_0304_0506_0708);
    pkt.reset_read();

    let parsed = UseEquipmentSet::read(&mut pkt).unwrap();

    assert_eq!(parsed.inv_update.items, vec![(255, 36)]);
    assert_eq!(parsed.items[0].item, item_guid);
    assert_eq!(parsed.items[0].container_slot, 255);
    assert_eq!(parsed.items[0].slot, 0);
    assert_eq!(parsed.guid, 0x0102_0304_0506_0708);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn use_equipment_set_result_writes_cpp_guid_and_reason() {
    let bytes = UseEquipmentSetResult {
        guid: 0x0102_0304_0506_0708,
        reason: 4,
    }
    .to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::UseEquipmentSetResult as u16
    );

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint64().unwrap(), 0x0102_0304_0506_0708);
    assert_eq!(pkt.read_uint8().unwrap(), 4);
    assert_eq!(pkt.remaining(), 0);
}
