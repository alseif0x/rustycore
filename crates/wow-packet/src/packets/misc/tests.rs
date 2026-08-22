// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Misc packet tests for [`super`].
//!
//! Extracted from the inline module by issue #227.

#![cfg(test)]

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

#[test]
fn gm_ticket_system_status_matches_cpp_int32_shape() {
    let bytes = GmTicketSystemStatus::from_support_enabled_like_cpp(true).to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::GmTicketSystemStatus as u16
    );

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_int32().unwrap(), GmTicketSystemStatus::ENABLED);
    assert_eq!(pkt.remaining(), 0);

    let bytes = GmTicketSystemStatus::from_support_enabled_like_cpp(false).to_bytes();
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_int32().unwrap(), GmTicketSystemStatus::DISABLED);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn gm_ticket_acknowledge_survey_reads_case_id_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(42);

    let survey = GmTicketAcknowledgeSurvey::read(&mut pkt).unwrap();
    assert_eq!(survey.case_id, 42);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn complaint_reads_chat_variant_like_cpp() {
    let offender_guid = ObjectGuid::create_player(1, 42);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint8(SUPPORT_SPAM_TYPE_CHAT_LIKE_CPP);
    pkt.write_packed_guid(&offender_guid);
    pkt.write_uint32(0x0102_0304);
    pkt.write_uint32(55);
    pkt.write_uint32(7);
    pkt.write_uint32(9);
    pkt.write_bits(11, 12);
    pkt.write_string("hello world");

    let complaint = Complaint::read(&mut pkt).unwrap();

    assert_eq!(complaint.complaint_type, SUPPORT_SPAM_TYPE_CHAT_LIKE_CPP);
    assert_eq!(complaint.offender.player_guid, offender_guid);
    assert_eq!(complaint.offender.realm_address, 0x0102_0304);
    assert_eq!(complaint.offender.time_since_offence, 55);
    assert!(complaint.mail_id.is_none());
    let chat = complaint.chat.expect("chat complaint payload");
    assert_eq!(chat.command, 7);
    assert_eq!(chat.channel_id, 9);
    assert_eq!(chat.message_log, "hello world");
    assert!(complaint.calendar_event_guid.is_none());
    assert!(complaint.calendar_invite_guid.is_none());
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn submit_user_feedback_reads_header_note_and_suggestion_bit_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(571);
    pkt.write_float(1.25);
    pkt.write_float(2.5);
    pkt.write_float(3.75);
    pkt.write_float(4.0);
    pkt.write_int32(9);
    pkt.write_bits(6, 24); // "hello" plus null terminator
    pkt.write_bit(true);
    pkt.write_string("hello");
    pkt.write_uint8(0);

    let feedback = SubmitUserFeedback::read(&mut pkt).unwrap();

    assert_eq!(feedback.header.map_id, 571);
    assert_eq!(feedback.header.position, Position::xyz(1.25, 2.5, 3.75));
    assert_eq!(feedback.header.facing, 4.0);
    assert_eq!(feedback.header.program, 9);
    assert!(feedback.is_suggestion);
    assert_eq!(feedback.note, "hello");
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn support_ticket_submit_suggestion_reads_10_bit_message_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    let message = "future idea text";
    pkt.write_bits(message.len() as u32, 10);
    pkt.write_string(message);

    let suggestion = SupportTicketSubmitSuggestion::read(&mut pkt).unwrap();

    assert_eq!(suggestion.message, message);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn support_ticket_submit_bug_reads_header_and_10_bit_message_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    let message = "broken thing";
    pkt.write_int32(571);
    pkt.write_float(1.25);
    pkt.write_float(2.5);
    pkt.write_float(3.75);
    pkt.write_float(4.0);
    pkt.write_int32(9);
    pkt.write_bits(message.len() as u32, 10);
    pkt.write_string(message);

    let bug = SupportTicketSubmitBug::read(&mut pkt).unwrap();

    assert_eq!(bug.header.map_id, 571);
    assert_eq!(bug.header.position, Position::xyz(1.25, 2.5, 3.75));
    assert_eq!(bug.header.facing, 4.0);
    assert_eq!(bug.header.program, 9);
    assert_eq!(bug.message, message);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn support_ticket_submit_complaint_reads_chatlog_note_and_mail_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    let target = ObjectGuid::create_player(1, 42);
    let note = "report note";
    let chat_text = "bad text";
    let mail_body = "mail body";
    let mail_subject = "subject";

    pkt.write_int32(571);
    pkt.write_float(1.25);
    pkt.write_float(2.5);
    pkt.write_float(3.75);
    pkt.write_float(4.0);
    pkt.write_int32(9);
    pkt.write_packed_guid(&target);
    pkt.write_int32(1);
    pkt.write_int32(2);
    pkt.write_int32(4);
    pkt.write_uint32(1); // ChatLog.Lines.Count
    pkt.write_bit(true); // ReportLineIndex.HasValue
    pkt.write_int64(12345);
    pkt.write_bits(chat_text.len() as u32, 12);
    pkt.write_string(chat_text);
    pkt.write_uint32(0);
    pkt.write_bits(note.len() as u32, 10);
    pkt.write_bit(true); // MailInfo
    pkt.write_bit(false); // CalendarInfo
    pkt.write_bit(false); // PetInfo
    pkt.write_bit(false); // GuildInfo
    pkt.write_bit(false); // LFGListSearchResult
    pkt.write_bit(false); // LFGListApplicant
    pkt.write_bit(false); // ClubMessage
    pkt.write_bit(false); // ClubFinderResult
    pkt.write_bit(false); // Unused910
    pkt.flush_bits();
    pkt.write_uint32(0); // HorusChatLog.Lines.Count
    pkt.write_string(note);
    pkt.write_int64(77);
    pkt.write_bits(mail_body.len() as u32, 13);
    pkt.write_bits(mail_subject.len() as u32, 9);
    pkt.write_string(mail_body);
    pkt.write_string(mail_subject);

    let complaint = SupportTicketSubmitComplaint::read(&mut pkt).unwrap();

    assert_eq!(complaint.header.map_id, 571);
    assert_eq!(complaint.target_character_guid, target);
    assert_eq!(complaint.report_type, 1);
    assert_eq!(complaint.major_category, 2);
    assert_eq!(complaint.minor_category_flags, 4);
    assert_eq!(complaint.chat_log.lines.len(), 1);
    assert_eq!(complaint.chat_log.lines[0].timestamp, 12345);
    assert_eq!(complaint.chat_log.lines[0].text, chat_text);
    assert_eq!(complaint.chat_log.report_line_index, Some(0));
    assert!(complaint.horus_chat_log.lines.is_empty());
    assert_eq!(complaint.note, note);
    let mail = complaint.mail_info.expect("mail info");
    assert_eq!(mail.mail_id, 77);
    assert_eq!(mail.mail_body, mail_body);
    assert_eq!(mail.mail_subject, mail_subject);
    assert!(complaint.calendar_info.is_none());
    assert!(complaint.pet_info.is_none());
    assert!(complaint.guild_info.is_none());
    assert!(complaint.lfg_list_search_result.is_none());
    assert!(complaint.lfg_list_applicant.is_none());
    assert!(complaint.community_message.is_none());
    assert!(complaint.club_finder_result.is_none());
    assert!(complaint.unused910.is_none());
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn lfg_player_info_empty_matches_cpp_shape() {
    let bytes = LfgPlayerInfo::empty().to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::LfgPlayerInfo as u16
    );

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), 0); // Dungeon.Count
    assert!(!pkt.has_bit().unwrap()); // BlackList.PlayerGuid.HasValue
    assert_eq!(pkt.read_uint32().unwrap(), 0); // BlackList.Slot.Count
}

#[test]
fn lfg_party_info_empty_matches_cpp_shape() {
    let bytes = LfgPartyInfo::empty().to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::LfgPartyInfo as u16
    );

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
}

#[test]
fn gm_ticket_case_status_empty_matches_cpp_todo_handler_shape() {
    let bytes = GmTicketCaseStatus::empty().to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::GmTicketCaseStatus as u16
    );
    assert_eq!(bytes.len(), 2 + 4);

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
}

#[test]
fn complaint_result_matches_cpp_shape() {
    let bytes = ComplaintResult {
        complaint_type: SUPPORT_SPAM_TYPE_CHAT_LIKE_CPP as u32,
        result: ComplaintResult::OK_LIKE_CPP,
    }
    .to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::ComplaintResult as u16
    );
    assert_eq!(bytes.len(), 2 + 5);

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(
        pkt.read_uint32().unwrap(),
        SUPPORT_SPAM_TYPE_CHAT_LIKE_CPP as u32
    );
    assert_eq!(pkt.read_uint8().unwrap(), ComplaintResult::OK_LIKE_CPP);
}

#[test]
fn calendar_send_num_pending_matches_cpp_shape() {
    let bytes = CalendarSendNumPending { num_pending: 3 }.to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::CalendarSendNumPending as u16
    );
    assert_eq!(bytes.len(), 2 + 4);

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), 3);
}

#[test]
fn calendar_send_calendar_empty_matches_cpp_header_shape() {
    let bytes = CalendarSendCalendar::empty_at_unix(946_684_800).to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::CalendarSendCalendar as u16
    );
    assert_eq!(bytes.len(), 2 + 4 + 4 + 4 + 4);

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), 0x0000_3000); // 2000-01-01 00:00 UTC
    assert_eq!(pkt.read_uint32().unwrap(), 0); // Invites.Count
    assert_eq!(pkt.read_uint32().unwrap(), 0); // Events.Count
    assert_eq!(pkt.read_uint32().unwrap(), 0); // RaidLockouts.Count
}

#[test]
fn calendar_raid_lockout_added_matches_cpp_field_order() {
    let bytes = CalendarRaidLockoutAdded::new_at_unix(9001, 946_684_800, 631, 4, 86_400).to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::CalendarRaidLockoutAdded as u16
    );
    assert_eq!(bytes.len(), 2 + 8 + 4 + 4 + 4 + 4);

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint64().unwrap(), 9001);
    assert_eq!(pkt.read_uint32().unwrap(), 0x0000_3000); // 2000-01-01 00:00 UTC
    assert_eq!(pkt.read_int32().unwrap(), 631);
    assert_eq!(pkt.read_uint32().unwrap(), 4);
    assert_eq!(pkt.read_int32().unwrap(), 86_400);
}

#[test]
fn calendar_raid_lockout_removed_matches_cpp_field_order() {
    let bytes = CalendarRaidLockoutRemoved {
        instance_id: 9001,
        map_id: 631,
        difficulty_id: 4,
    }
    .to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::CalendarRaidLockoutRemoved as u16
    );
    assert_eq!(bytes.len(), 2 + 8 + 4 + 4);

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint64().unwrap(), 9001);
    assert_eq!(pkt.read_int32().unwrap(), 631);
    assert_eq!(pkt.read_uint32().unwrap(), 4);
}

#[test]
fn calendar_raid_lockout_updated_matches_cpp_field_order() {
    let bytes =
        CalendarRaidLockoutUpdated::new_at_unix(946_684_800, 631, 4, 3_600, 86_400).to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::CalendarRaidLockoutUpdated as u16
    );
    assert_eq!(bytes.len(), 2 + 4 + 4 + 4 + 4 + 4);

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), 0x0000_3000); // 2000-01-01 00:00 UTC
    assert_eq!(pkt.read_int32().unwrap(), 631);
    assert_eq!(pkt.read_uint32().unwrap(), 4);
    assert_eq!(pkt.read_int32().unwrap(), 3_600);
    assert_eq!(pkt.read_int32().unwrap(), 86_400);
}

#[test]
fn set_saved_instance_extend_reads_cpp_field_order() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(631);
    pkt.write_uint32(4);
    pkt.write_bit(true);
    pkt.flush_bits();
    pkt.reset_read();

    let query = SetSavedInstanceExtend::read(&mut pkt).unwrap();
    assert_eq!(query.map_id, 631);
    assert_eq!(query.difficulty_id, 4);
    assert!(query.extend);
}

#[test]
fn calendar_community_invite_reads_cpp_field_order() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint64(0x0102_0304_0506_0708);
    pkt.write_uint8(10);
    pkt.write_uint8(70);
    pkt.write_uint8(3);

    let query = CalendarCommunityInvite::read(&mut pkt).unwrap();
    assert_eq!(query.club_id, 0x0102_0304_0506_0708);
    assert_eq!(query.min_level, 10);
    assert_eq!(query.max_level, 70);
    assert_eq!(query.max_rank_order, 3);
}

#[test]
fn calendar_get_event_reads_cpp_event_id() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint64(0x0102_0304_0506_0708);

    let query = CalendarGetEvent::read(&mut pkt).unwrap();
    assert_eq!(query.event_id, 0x0102_0304_0506_0708);
}

#[test]
fn calendar_remove_event_reads_cpp_field_order() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint64(0x1111_2222_3333_4444);
    pkt.write_uint64(0x5555_6666_7777_8888);
    pkt.write_uint64(0x9999_AAAA_BBBB_CCCC);
    pkt.write_uint32(0xDEAD_BEEF);

    let query = CalendarRemoveEvent::read(&mut pkt).unwrap();
    assert_eq!(query.event_id, 0x1111_2222_3333_4444);
    assert_eq!(query.moderator_id, 0x5555_6666_7777_8888);
    assert_eq!(query.club_id, 0x9999_AAAA_BBBB_CCCC);
    assert_eq!(query.flags, 0xDEAD_BEEF);
}

#[test]
fn calendar_copy_event_reads_cpp_field_order() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint64(0x1111_2222_3333_4444);
    pkt.write_uint64(0x5555_6666_7777_8888);
    pkt.write_uint64(0x9999_AAAA_BBBB_CCCC);
    pkt.write_uint32(0xDEAD_BEEF);

    let query = CalendarCopyEvent::read(&mut pkt).unwrap();
    assert_eq!(query.event_id, 0x1111_2222_3333_4444);
    assert_eq!(query.moderator_id, 0x5555_6666_7777_8888);
    assert_eq!(query.event_club_id, 0x9999_AAAA_BBBB_CCCC);
    assert_eq!(query.date, 0xDEAD_BEEF);
}

#[test]
fn calendar_remove_invite_reads_cpp_field_order() {
    let guid = ObjectGuid::new(0x0102_0304_0506_0708, 0x1111_2222_3333_4444);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&guid);
    pkt.write_uint64(0x5555_6666_7777_8888);
    pkt.write_uint64(0x9999_AAAA_BBBB_CCCC);
    pkt.write_uint64(0xDEAD_BEEF_CAFE_BABE);

    let query = CalendarRemoveInvite::read(&mut pkt).unwrap();
    assert_eq!(query.guid, guid);
    assert_eq!(query.invite_id, 0x5555_6666_7777_8888);
    assert_eq!(query.moderator_id, 0x9999_AAAA_BBBB_CCCC);
    assert_eq!(query.event_id, 0xDEAD_BEEF_CAFE_BABE);
}

#[test]
fn calendar_event_sign_up_reads_cpp_field_order_and_tentative_bit() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint64(0x1111_2222_3333_4444);
    pkt.write_uint64(0x5555_6666_7777_8888);
    pkt.write_bit(true);
    pkt.flush_bits();

    let query = CalendarEventSignUp::read(&mut pkt).unwrap();
    assert_eq!(query.event_id, 0x1111_2222_3333_4444);
    assert_eq!(query.club_id, 0x5555_6666_7777_8888);
    assert!(query.tentative);
}

#[test]
fn calendar_invite_reads_cpp_field_order() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint64(0x1111_2222_3333_4444);
    pkt.write_uint64(0x5555_6666_7777_8888);
    pkt.write_uint64(0x9999_AAAA_BBBB_CCCC);
    pkt.write_bits(4, 9);
    pkt.write_bit(false);
    pkt.write_bit(true);
    pkt.flush_bits();
    pkt.write_string("Test");

    let query = CalendarInvite::read(&mut pkt).unwrap();
    assert_eq!(query.event_id, 0x1111_2222_3333_4444);
    assert_eq!(query.moderator_id, 0x5555_6666_7777_8888);
    assert_eq!(query.club_id, 0x9999_AAAA_BBBB_CCCC);
    assert!(!query.creating);
    assert!(query.is_sign_up);
    assert_eq!(query.name, "Test");
}

#[test]
fn calendar_add_event_reads_cpp_field_order_with_invite_optionals() {
    let mut pkt = WorldPacket::new_empty();
    let invite_guid = ObjectGuid::new(0x0102_0304_0506_0708_i64, 0x1112_1314_1516_1718_i64);
    let optional_guid = ObjectGuid::new(0x2122_2324_2526_2728_i64, 0x3132_3334_3536_3738_i64);

    pkt.write_uint64(0x1111_2222_3333_4444);
    pkt.write_uint8(7);
    pkt.write_int32(-1234);
    pkt.write_uint32(0x0102_0304);
    pkt.write_uint32(0x0000_0440);
    pkt.write_uint32(1);
    pkt.write_bits(5, 8);
    pkt.write_bits(4, 11);
    pkt.write_guid(&invite_guid);
    pkt.write_uint8(3);
    pkt.write_uint8(2);
    pkt.write_bit(true);
    pkt.write_bit(true);
    pkt.write_bit(true);
    pkt.write_guid(&optional_guid);
    pkt.write_uint64(0x4142_4344_4546_4748);
    pkt.write_uint64(0x5152_5354_5556_5758);
    pkt.write_string("Title");
    pkt.write_string("Desc");
    pkt.write_uint32(99);

    let query = CalendarAddEvent::read(&mut pkt).unwrap();
    assert_eq!(query.club_id, 0x1111_2222_3333_4444);
    assert_eq!(query.event_type, 7);
    assert_eq!(query.texture_id, -1234);
    assert_eq!(query.time_packed, 0x0102_0304);
    assert_eq!(query.flags, 0x0000_0440);
    assert_eq!(query.title, "Title");
    assert_eq!(query.description, "Desc");
    assert_eq!(query.max_size, 99);
    assert_eq!(query.invites.len(), 1);
    assert_eq!(query.invites[0].guid, invite_guid);
    assert_eq!(query.invites[0].status, 3);
    assert_eq!(query.invites[0].moderator, 2);
    assert_eq!(query.invites[0].unused_801_1, Some(optional_guid));
    assert_eq!(query.invites[0].unused_801_2, Some(0x4142_4344_4546_4748));
    assert_eq!(query.invites[0].unused_801_3, Some(0x5152_5354_5556_5758));
}

#[test]
fn calendar_update_event_reads_cpp_field_order() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint64(0x1111_2222_3333_4444);
    pkt.write_uint64(0x5555_6666_7777_8888);
    pkt.write_uint64(0x9999_AAAA_BBBB_CCCC);
    pkt.write_uint8(7);
    pkt.write_uint32(0x0102_0304);
    pkt.write_uint32(0x0506_0708);
    pkt.write_uint32(0x090A_0B0C);
    pkt.write_bits(5, 8);
    pkt.write_bits(4, 11);
    pkt.flush_bits();
    pkt.write_string("Title");
    pkt.write_string("Desc");
    pkt.write_uint32(99);

    let query = CalendarUpdateEvent::read(&mut pkt).unwrap();
    assert_eq!(query.club_id, 0x1111_2222_3333_4444);
    assert_eq!(query.event_id, 0x5555_6666_7777_8888);
    assert_eq!(query.moderator_id, 0x9999_AAAA_BBBB_CCCC);
    assert_eq!(query.event_type, 7);
    assert_eq!(query.texture_id, 0x0102_0304);
    assert_eq!(query.time_packed, 0x0506_0708);
    assert_eq!(query.flags, 0x090A_0B0C);
    assert_eq!(query.title, "Title");
    assert_eq!(query.description, "Desc");
    assert_eq!(query.max_size, 99);
}

#[test]
fn calendar_rsvp_reads_cpp_field_order() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint64(0x1111_2222_3333_4444);
    pkt.write_uint64(0x5555_6666_7777_8888);
    pkt.write_uint8(9);

    let query = CalendarRsvp::read(&mut pkt).unwrap();
    assert_eq!(query.event_id, 0x1111_2222_3333_4444);
    assert_eq!(query.invite_id, 0x5555_6666_7777_8888);
    assert_eq!(query.status, 9);
}

#[test]
fn calendar_status_reads_cpp_field_order() {
    let guid = ObjectGuid::new(0x0102_0304_0506_0708, 0x1111_2222_3333_4444);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&guid);
    pkt.write_uint64(0x5555_6666_7777_8888);
    pkt.write_uint64(0x9999_AAAA_BBBB_CCCC);
    pkt.write_uint64(0xDEAD_BEEF_CAFE_BABE);
    pkt.write_uint8(9);

    let query = CalendarStatus::read(&mut pkt).unwrap();
    assert_eq!(query.guid, guid);
    assert_eq!(query.event_id, 0x5555_6666_7777_8888);
    assert_eq!(query.invite_id, 0x9999_AAAA_BBBB_CCCC);
    assert_eq!(query.moderator_id, 0xDEAD_BEEF_CAFE_BABE);
    assert_eq!(query.status, 9);
}

#[test]
fn calendar_moderator_status_query_reads_cpp_field_order() {
    let guid = ObjectGuid::new(0x0102_0304_0506_0708, 0x1111_2222_3333_4444);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&guid);
    pkt.write_uint64(0x5555_6666_7777_8888);
    pkt.write_uint64(0x9999_AAAA_BBBB_CCCC);
    pkt.write_uint64(0xDEAD_BEEF_CAFE_BABE);
    pkt.write_uint8(9);

    let query = CalendarModeratorStatusQuery::read(&mut pkt).unwrap();
    assert_eq!(query.guid, guid);
    assert_eq!(query.event_id, 0x5555_6666_7777_8888);
    assert_eq!(query.invite_id, 0x9999_AAAA_BBBB_CCCC);
    assert_eq!(query.moderator_id, 0xDEAD_BEEF_CAFE_BABE);
    assert_eq!(query.status, 9);
}

#[test]
fn calendar_command_result_event_invalid_matches_cpp_shape() {
    let bytes = CalendarCommandResult::event_invalid_like_cpp().to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::CalendarCommandResult as u16
    );
    assert_eq!(bytes.len(), 2 + 4);

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint8().unwrap(), 1);
    assert_eq!(pkt.read_uint8().unwrap(), 6);
    assert_eq!(pkt.read_bits(9).unwrap(), 0);
}

#[test]
fn calendar_command_result_no_invite_matches_cpp_shape() {
    let bytes = CalendarCommandResult::no_invite_like_cpp().to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::CalendarCommandResult as u16
    );
    assert_eq!(bytes.len(), 2 + 4);

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint8().unwrap(), 1);
    assert_eq!(pkt.read_uint8().unwrap(), 29);
    assert_eq!(pkt.read_bits(9).unwrap(), 0);
}

#[test]
fn calendar_complain_reads_cpp_guid_event_invite_order() {
    let invited_by_guid = ObjectGuid::create_player(1, 0xAABB_CCDD);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&invited_by_guid);
    pkt.write_uint64(0x0102_0304_0506_0708);
    pkt.write_uint64(0x1112_1314_1516_1718);

    let complain = CalendarComplain::read(&mut pkt).unwrap();
    assert_eq!(complain.invited_by_guid, invited_by_guid);
    assert_eq!(complain.event_id, 0x0102_0304_0506_0708);
    assert_eq!(complain.invite_id, 0x1112_1314_1516_1718);
}

#[test]
fn arena_team_roster_reads_cpp_team_id() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(0x0102_0304);

    let request = ArenaTeamRoster::read(&mut pkt).unwrap();
    assert_eq!(request.team_id, 0x0102_0304);
}

#[test]
fn arena_team_decline_reads_empty_cpp_packet() {
    let mut pkt = WorldPacket::new_empty();

    ArenaTeamDecline::read(&mut pkt).unwrap();
}

#[test]
fn arena_team_accept_reads_empty_cpp_packet() {
    let mut pkt = WorldPacket::new_empty();

    ArenaTeamAccept::read(&mut pkt).unwrap();
}

#[test]
fn arena_team_leave_reads_empty_cpp_packet() {
    let mut pkt = WorldPacket::new_empty();

    ArenaTeamLeave::read(&mut pkt).unwrap();
}

#[test]
fn arena_team_remove_reads_team_id_and_9bit_target_name_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(0x0102_0304);
    pkt.write_bits(7, 9);
    pkt.write_string("Playerx");
    pkt.reset_read();

    let request = ArenaTeamRemove::read(&mut pkt).unwrap();

    assert_eq!(request.team_id, 0x0102_0304);
    assert_eq!(request.target_name, "Playerx");
}

#[test]
fn arena_team_disband_reads_team_id_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(0x1122_3344);
    pkt.reset_read();

    let request = ArenaTeamDisband::read(&mut pkt).unwrap();

    assert_eq!(request.team_id, 0x1122_3344);
}

#[test]
fn arena_team_leader_reads_team_id_and_9bit_target_name_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(0x5566_7788);
    pkt.write_bits(6, 9);
    pkt.write_string("Leader");
    pkt.reset_read();

    let request = ArenaTeamLeader::read(&mut pkt).unwrap();

    assert_eq!(request.team_id, 0x5566_7788);
    assert_eq!(request.target_name, "Leader");
}

#[test]
fn query_arena_team_reads_team_id_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(0xAABB_CCDD);
    pkt.reset_read();

    let request = QueryArenaTeam::read(&mut pkt).unwrap();

    assert_eq!(request.team_id, 0xAABB_CCDD);
}

#[test]
fn busy_trade_reads_empty_cpp_packet() {
    let mut pkt = WorldPacket::new_empty();

    BusyTrade::read(&mut pkt).unwrap();
}

#[test]
fn begin_trade_reads_empty_cpp_packet() {
    let mut pkt = WorldPacket::new_empty();

    BeginTrade::read(&mut pkt).unwrap();
}

#[test]
fn accept_trade_reads_state_index_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(0x1122_3344);
    pkt.reset_read();

    let packet = AcceptTrade::read(&mut pkt).unwrap();

    assert_eq!(packet.state_index, 0x1122_3344);
}

#[test]
fn clear_trade_item_reads_trade_slot_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint8(5);
    pkt.reset_read();

    let packet = ClearTradeItem::read(&mut pkt).unwrap();

    assert_eq!(packet.trade_slot, 5);
}

#[test]
fn set_trade_item_reads_slots_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint8(2);
    pkt.write_uint8(255);
    pkt.write_uint8(18);
    pkt.reset_read();

    let packet = SetTradeItem::read(&mut pkt).unwrap();

    assert_eq!(packet.trade_slot, 2);
    assert_eq!(packet.pack_slot, 255);
    assert_eq!(packet.item_slot_in_pack, 18);
}

#[test]
fn set_trade_spell_reads_spell_and_slots_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(7418);
    pkt.write_uint8(255);
    pkt.write_uint8(23);
    pkt.reset_read();

    let packet = SetTradeSpell::read(&mut pkt).unwrap();

    assert_eq!(packet.spell_id, 7418);
    assert_eq!(packet.pack_slot, 255);
    assert_eq!(packet.item_slot_in_pack, 23);
}

#[test]
fn sign_petition_reads_guid_and_choice_like_cpp() {
    let petition_guid = ObjectGuid::create_item(1, 0x0102_0304_0506_0708);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bytes(&petition_guid.to_raw_bytes());
    pkt.write_uint8(1);
    pkt.reset_read();

    let packet = SignPetition::read(&mut pkt).unwrap();

    assert_eq!(packet.petition_guid, petition_guid);
    assert_eq!(packet.choice, 1);
}

#[test]
fn decline_petition_reads_guid_like_cpp() {
    let petition_guid = ObjectGuid::create_item(1, 0x1112_1314_1516_1718);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bytes(&petition_guid.to_raw_bytes());
    pkt.reset_read();

    let packet = DeclinePetition::read(&mut pkt).unwrap();

    assert_eq!(packet.petition_guid, petition_guid);
}

#[test]
fn query_petition_reads_id_then_guid_like_cpp() {
    let item_guid = ObjectGuid::create_item(1, 0x2122_2324_2526_2728);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(0x1122_3344);
    pkt.write_bytes(&item_guid.to_raw_bytes());
    pkt.reset_read();

    let packet = QueryPetition::read(&mut pkt).unwrap();

    assert_eq!(packet.petition_id, 0x1122_3344);
    assert_eq!(packet.item_guid, item_guid);
}

#[test]
fn query_petition_not_found_response_writes_id_and_allow_false_like_cpp() {
    let item_guid = ObjectGuid::create_item(1, 0x3132_3334_3536_3738);
    let bytes = QueryPetitionResponse::not_found_like_cpp(item_guid).to_bytes();
    let mut body = WorldPacket::from_bytes(&bytes);

    assert_eq!(
        body.server_opcode(),
        Some(ServerOpcodes::QueryPetitionResponse)
    );
    assert_eq!(
        body.read_uint16().unwrap(),
        ServerOpcodes::QueryPetitionResponse as u16
    );
    assert_eq!(body.read_uint32().unwrap(), item_guid.counter() as u32);
    assert!(!body.read_bit().unwrap());
    assert_eq!(body.remaining(), 0);
}

#[test]
fn set_trade_gold_reads_coinage_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint64(0x1122_3344_5566_7788);
    pkt.reset_read();

    let packet = SetTradeGold::read(&mut pkt).unwrap();

    assert_eq!(packet.coinage, 0x1122_3344_5566_7788);
}

#[test]
fn unaccept_trade_reads_empty_cpp_packet() {
    let mut pkt = WorldPacket::new_empty();

    UnacceptTrade::read(&mut pkt).unwrap();
}

#[test]
fn ignore_trade_reads_empty_cpp_packet() {
    let mut pkt = WorldPacket::new_empty();

    IgnoreTrade::read(&mut pkt).unwrap();
}

#[test]
fn trade_status_player_busy_writes_cancel_status_bits_like_cpp() {
    let bytes = TradeStatus::cancel_like_cpp(TRADE_STATUS_PLAYER_BUSY_LIKE_CPP).to_bytes();

    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::TradeStatus as u16
    );
    assert_eq!(bytes.len(), 3);
    assert_eq!(bytes[2], TRADE_STATUS_PLAYER_BUSY_LIKE_CPP << 1);
}

#[test]
fn trade_status_initiated_writes_id_payload_like_cpp() {
    let bytes = TradeStatus::initiated_like_cpp(0x1122_3344).to_bytes();

    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::TradeStatus as u16
    );
    assert_eq!(bytes.len(), 7);
    assert_eq!(bytes[2], TRADE_STATUS_INITIATED_LIKE_CPP << 2);
    assert_eq!(
        u32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]),
        0x1122_3344
    );
}

#[test]
fn trade_status_failed_writes_bag_result_like_cpp() {
    let bytes = TradeStatus::failed_like_cpp(EQUIP_ERR_NOT_ENOUGH_MONEY_LIKE_CPP, 0).to_bytes();

    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::TradeStatus as u16
    );
    assert_eq!(bytes.len(), 11);
    assert_eq!(bytes[2], TRADE_STATUS_FAILED_LIKE_CPP << 2);
    assert_eq!(
        i32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]),
        EQUIP_ERR_NOT_ENOUGH_MONEY_LIKE_CPP
    );
    assert_eq!(
        i32::from_le_bytes([bytes[7], bytes[8], bytes[9], bytes[10]]),
        0
    );
}

#[test]
fn trade_status_cancelled_writes_cancel_status_bits_like_cpp() {
    let bytes = TradeStatus::cancel_like_cpp(TRADE_STATUS_CANCELLED_LIKE_CPP).to_bytes();

    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::TradeStatus as u16
    );
    assert_eq!(bytes.len(), 3);
    assert_eq!(bytes[2], TRADE_STATUS_CANCELLED_LIKE_CPP << 2);
}

#[test]
fn trade_status_player_ignored_writes_cancel_status_bits_like_cpp() {
    let bytes = TradeStatus::cancel_like_cpp(TRADE_STATUS_PLAYER_IGNORED_LIKE_CPP).to_bytes();

    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::TradeStatus as u16
    );
    assert_eq!(bytes.len(), 3);
    assert_eq!(bytes[2], TRADE_STATUS_PLAYER_IGNORED_LIKE_CPP << 2);
}

#[test]
fn guild_bank_remaining_withdraw_money_matches_cpp_shape() {
    let bytes = GuildBankRemainingWithdrawMoney {
        remaining_withdraw_money: 123_456_789,
    }
    .to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::GuildBankRemainingWithdrawMoney as u16
    );
    assert_eq!(bytes.len(), 2 + 8);

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_int64().unwrap(), 123_456_789);
}

#[test]
fn commerce_token_get_log_reads_cpp_uint32() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(0x1122_3344);

    let request = CommerceTokenGetLog::read(&mut pkt).unwrap();
    assert_eq!(request.unk_int, 0x1122_3344);
}

#[test]
fn auctionable_token_sell_reads_empty_stub_like_cpp_wotlk() {
    let mut pkt = WorldPacket::new_empty();

    let request = AuctionableTokenSell::read(&mut pkt).unwrap();
    assert_eq!(request, AuctionableTokenSell);
}

#[test]
fn auction_list_items_reads_empty_legacy_packet_like_cpp() {
    let mut pkt = WorldPacket::new_empty();

    let request = AuctionListItems::read(&mut pkt).unwrap();
    assert_eq!(request, AuctionListItems);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn auction_replicate_items_reads_no_tainted_by_like_cpp() {
    let auctioneer = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9_001, 7);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&auctioneer);
    pkt.write_uint32(11);
    pkt.write_uint32(22);
    pkt.write_uint32(33);
    pkt.write_uint32(44);
    pkt.write_bit(false);
    pkt.flush_bits();
    pkt.reset_read();

    let request = AuctionReplicateItems::read(&mut pkt).unwrap();
    assert_eq!(request.auctioneer, auctioneer);
    assert_eq!(request.change_number_global, 11);
    assert_eq!(request.change_number_cursor, 22);
    assert_eq!(request.change_number_tombstone, 33);
    assert_eq!(request.count, 44);
    assert!(request.tainted_by.is_none());
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn auction_replicate_items_reads_tainted_by_like_cpp() {
    let auctioneer = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9_002, 8);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&auctioneer);
    pkt.write_uint32(1);
    pkt.write_uint32(2);
    pkt.write_uint32(3);
    pkt.write_uint32(4);
    pkt.write_bit(true);
    pkt.flush_bits();
    pkt.write_bits(6, 10); // "Trade" + '\0'
    pkt.write_bits(4, 10); // "1.0" + '\0'
    pkt.write_bit(true);
    pkt.write_bit(false);
    pkt.flush_bits();
    pkt.write_string("Trade");
    pkt.write_uint8(0);
    pkt.write_string("1.0");
    pkt.write_uint8(0);
    pkt.reset_read();

    let request = AuctionReplicateItems::read(&mut pkt).unwrap();
    assert_eq!(request.auctioneer, auctioneer);
    assert_eq!(request.change_number_global, 1);
    assert_eq!(request.change_number_cursor, 2);
    assert_eq!(request.change_number_tombstone, 3);
    assert_eq!(request.count, 4);
    assert_eq!(
        request.tainted_by,
        Some(AuctionAddonInfo {
            name: "Trade".to_string(),
            version: "1.0".to_string(),
            loaded: true,
            disabled: false,
        })
    );
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn auction_place_bid_reads_no_tainted_by_like_cpp() {
    let auctioneer = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9_003, 9);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&auctioneer);
    pkt.write_int32(1234);
    pkt.write_uint64(12_300);
    pkt.write_bit(false);
    pkt.flush_bits();
    pkt.reset_read();

    let request = AuctionPlaceBid::read(&mut pkt).unwrap();
    assert_eq!(request.auctioneer, auctioneer);
    assert_eq!(request.auction_id, 1234);
    assert_eq!(request.bid_amount, 12_300);
    assert!(request.tainted_by.is_none());
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn auction_place_bid_reads_tainted_by_like_cpp() {
    let auctioneer = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9_004, 10);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&auctioneer);
    pkt.write_int32(5678);
    pkt.write_uint64(45_600);
    pkt.write_bit(true);
    pkt.flush_bits();
    pkt.write_bits(6, 10); // "Trade" + '\0'
    pkt.write_bits(4, 10); // "1.0" + '\0'
    pkt.write_bit(true);
    pkt.write_bit(false);
    pkt.flush_bits();
    pkt.write_string("Trade");
    pkt.write_uint8(0);
    pkt.write_string("1.0");
    pkt.write_uint8(0);
    pkt.reset_read();

    let request = AuctionPlaceBid::read(&mut pkt).unwrap();
    assert_eq!(request.auctioneer, auctioneer);
    assert_eq!(request.auction_id, 5678);
    assert_eq!(request.bid_amount, 45_600);
    assert_eq!(
        request.tainted_by,
        Some(AuctionAddonInfo {
            name: "Trade".to_string(),
            version: "1.0".to_string(),
            loaded: true,
            disabled: false,
        })
    );
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn auction_remove_item_reads_no_tainted_by_like_cpp() {
    let auctioneer = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9_005, 11);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&auctioneer);
    pkt.write_int32(1234);
    pkt.write_int32(19019);
    pkt.write_bit(false);
    pkt.flush_bits();
    pkt.reset_read();

    let request = AuctionRemoveItem::read(&mut pkt).unwrap();
    assert_eq!(request.auctioneer, auctioneer);
    assert_eq!(request.auction_id, 1234);
    assert_eq!(request.item_id, 19019);
    assert!(request.tainted_by.is_none());
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn auction_remove_item_reads_tainted_by_like_cpp() {
    let auctioneer = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9_006, 12);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&auctioneer);
    pkt.write_int32(5678);
    pkt.write_int32(4306);
    pkt.write_bit(true);
    pkt.flush_bits();
    pkt.write_bits(6, 10); // "Trade" + '\0'
    pkt.write_bits(4, 10); // "1.0" + '\0'
    pkt.write_bit(true);
    pkt.write_bit(false);
    pkt.flush_bits();
    pkt.write_string("Trade");
    pkt.write_uint8(0);
    pkt.write_string("1.0");
    pkt.write_uint8(0);
    pkt.reset_read();

    let request = AuctionRemoveItem::read(&mut pkt).unwrap();
    assert_eq!(request.auctioneer, auctioneer);
    assert_eq!(request.auction_id, 5678);
    assert_eq!(request.item_id, 4306);
    assert_eq!(
        request.tainted_by,
        Some(AuctionAddonInfo {
            name: "Trade".to_string(),
            version: "1.0".to_string(),
            loaded: true,
            disabled: false,
        })
    );
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn auction_sell_item_reads_single_item_no_tainted_by_like_cpp() {
    let auctioneer = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9_007, 13);
    let item_guid = ObjectGuid::create_item(1, 19_019);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&auctioneer);
    pkt.write_uint64(10_000);
    pkt.write_uint64(25_000);
    pkt.write_uint32(720);
    pkt.write_bit(false);
    pkt.write_bits(1, 6);
    pkt.flush_bits();
    pkt.write_guid(&item_guid);
    pkt.write_uint32(1);
    pkt.reset_read();

    let request = AuctionSellItem::read(&mut pkt).unwrap();
    assert_eq!(request.auctioneer, auctioneer);
    assert_eq!(request.min_bid, 10_000);
    assert_eq!(request.buyout_price, 25_000);
    assert_eq!(request.runtime, 720);
    assert!(request.tainted_by.is_none());
    assert_eq!(
        request.items,
        vec![AuctionItemForSale {
            guid: item_guid,
            use_count: 1,
        }]
    );
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn auction_sell_item_reads_tainted_by_like_cpp() {
    let auctioneer = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9_008, 14);
    let item_guid = ObjectGuid::create_item(1, 43_006);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&auctioneer);
    pkt.write_uint64(20_000);
    pkt.write_uint64(50_000);
    pkt.write_uint32(1440);
    pkt.write_bit(true);
    pkt.write_bits(1, 6);
    pkt.flush_bits();
    pkt.write_bits(6, 10); // "Trade" + '\0'
    pkt.write_bits(4, 10); // "1.0" + '\0'
    pkt.write_bit(true);
    pkt.write_bit(false);
    pkt.flush_bits();
    pkt.write_string("Trade");
    pkt.write_uint8(0);
    pkt.write_string("1.0");
    pkt.write_uint8(0);
    pkt.write_guid(&item_guid);
    pkt.write_uint32(1);
    pkt.reset_read();

    let request = AuctionSellItem::read(&mut pkt).unwrap();
    assert_eq!(request.auctioneer, auctioneer);
    assert_eq!(request.min_bid, 20_000);
    assert_eq!(request.buyout_price, 50_000);
    assert_eq!(request.runtime, 1440);
    assert_eq!(
        request.tainted_by,
        Some(AuctionAddonInfo {
            name: "Trade".to_string(),
            version: "1.0".to_string(),
            loaded: true,
            disabled: false,
        })
    );
    assert_eq!(
        request.items,
        vec![AuctionItemForSale {
            guid: item_guid,
            use_count: 1,
        }]
    );
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn auctionable_token_sell_at_market_price_reads_empty_stub_like_cpp_wotlk() {
    let mut pkt = WorldPacket::new_empty();

    let request = AuctionableTokenSellAtMarketPrice::read(&mut pkt).unwrap();
    assert_eq!(request, AuctionableTokenSellAtMarketPrice);
}

#[test]
fn commerce_token_get_log_response_success_empty_matches_cpp_todo_handler() {
    let bytes = CommerceTokenGetLogResponse::success_empty(0x1122_3344).to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::CommerceTokenGetLogResponse as u16
    );
    assert_eq!(bytes.len(), 2 + 12);

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), 0x1122_3344);
    assert_eq!(pkt.read_uint32().unwrap(), TOKEN_RESULT_SUCCESS_LIKE_CPP);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
}

#[test]
fn tutorial_flags_all_shown() {
    let pkt = TutorialFlags::all_shown();
    let bytes = pkt.to_bytes();
    // opcode(2) + 8*u32(32) = 34
    assert_eq!(bytes.len(), 34);
}

#[test]
fn tutorial_flags_none_shown_matches_cpp_default() {
    let pkt = TutorialFlags::none_shown();
    assert_eq!(pkt.tutorial_data, [0; 8]);
}

#[test]
fn feature_system_status_serializes() {
    let pkt = FeatureSystemStatus::default_wotlk();
    let bytes = pkt.to_bytes();
    assert_eq!(bytes.len(), 192);
    // Verify opcode is FeatureSystemStatus (0x25bf)
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x25bf);
}

#[test]
fn feature_system_status_uses_cpp_config_flags() {
    let config = FeatureSystemConfigLikeCpp {
        support_tickets_enabled: true,
        support_bugs_enabled: false,
        support_complaints_enabled: true,
        support_suggestions_enabled: false,
        char_undelete_enabled: true,
        bpay_store_enabled: true,
    };
    let pkt = FeatureSystemStatus::from_config_like_cpp(config, true);
    let bytes = pkt.to_bytes();
    let mut payload = WorldPacket::from_bytes(&bytes[2..]);

    payload.skip(73).unwrap();
    let flags: Vec<bool> = (0..42).map(|_| payload.read_bit().unwrap()).collect();
    assert!(!flags[0]); // VoiceEnabled
    assert!(flags[1]); // EuropaTicketSystemStatus.HasValue
    assert!(flags[2]); // BpayStoreEnabled
    assert!(flags[10]); // CharUndeleteEnabled
    assert!(flags[27]); // IsMuted = !CanSpeak()

    payload.reset_bits();
    assert!(!payload.read_bit().unwrap()); // QuickJoinConfig.ToastsDisabled
    payload.skip(22 * 4).unwrap();
    assert!(!payload.read_bit().unwrap()); // Squelch.IsSquelched
    payload.skip(4).unwrap(); // two empty packed GUIDs

    assert!(payload.read_bit().unwrap()); // TicketsEnabled
    assert!(!payload.read_bit().unwrap()); // BugsEnabled
    assert!(payload.read_bit().unwrap()); // ComplaintsEnabled
    assert!(!payload.read_bit().unwrap()); // SuggestionsEnabled
}

#[test]
fn feature_system_status_glue_screen_serializes() {
    let pkt = FeatureSystemStatusGlueScreen::default_wotlk();
    let bytes = pkt.to_bytes();
    assert!(bytes.len() > 20);
    // Verify opcode is FeatureSystemStatusGlueScreen (0x25c0)
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x25c0);
}

#[test]
fn feature_system_status_glue_screen_uses_cpp_config_fields() {
    let config = FeatureSystemConfigLikeCpp {
        support_tickets_enabled: true,
        support_bugs_enabled: true,
        support_complaints_enabled: false,
        support_suggestions_enabled: true,
        char_undelete_enabled: true,
        bpay_store_enabled: true,
    };
    let pkt = FeatureSystemStatusGlueScreen::from_config_like_cpp(config, 123, 9);
    let bytes = pkt.to_bytes();
    let mut payload = WorldPacket::from_bytes(&bytes[2..]);

    let flags: Vec<bool> = (0..27).map(|_| payload.read_bit().unwrap()).collect();
    assert!(flags[0]); // BpayStoreEnabled
    assert!(flags[3]); // CharUndeleteEnabled
    assert!(flags[19]); // EuropaTicketSystemStatus.HasValue

    payload.reset_bits();
    assert!(payload.read_bit().unwrap()); // TicketsEnabled
    assert!(payload.read_bit().unwrap()); // BugsEnabled
    assert!(!payload.read_bit().unwrap()); // ComplaintsEnabled
    assert!(payload.read_bit().unwrap()); // SuggestionsEnabled

    payload.skip(16).unwrap(); // SavedThrottleObjectState
    assert_eq!(payload.read_uint32().unwrap(), 0); // TokenPollTimeSeconds
    assert_eq!(payload.read_uint32().unwrap(), 0); // KioskSessionMinutes
    assert_eq!(payload.read_uint64().unwrap(), 0); // TokenBalanceAmount
    assert_eq!(payload.read_int32().unwrap(), 123); // MaxCharactersPerRealm
    assert_eq!(payload.read_uint32().unwrap(), 0); // LiveRegionCharacterCopySourceRegions
    assert_eq!(payload.read_uint32().unwrap(), 0); // BpayStoreProductDeliveryDelay
    assert_eq!(payload.read_int32().unwrap(), 0); // ActiveCharacterUpgradeBoostType
    assert_eq!(payload.read_int32().unwrap(), 0); // ActiveClassTrialBoostType
    assert_eq!(payload.read_int32().unwrap(), 0); // MinimumExpansionLevel
    assert_eq!(payload.read_int32().unwrap(), 9); // MaximumExpansionLevel
}

#[test]
fn transfer_aborted_matches_cpp_layout() {
    let bytes = TransferAborted {
        map_id: 571,
        arg: 0,
        map_difficulty_x_condition_id: 0,
        transfer_abort: 16,
    }
    .to_bytes();

    assert_eq!(bytes.len(), 12);
    assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x2703);
    assert_eq!(&bytes[2..6], &571u32.to_le_bytes());
    assert_eq!(bytes[6], 0);
    assert_eq!(&bytes[7..11], &0i32.to_le_bytes());
    assert_eq!(bytes[11], 0x40);
}

#[test]
fn client_cache_version_serializes() {
    let pkt = ClientCacheVersion { cache_version: 42 };
    let bytes = pkt.to_bytes();
    // opcode(2) + uint32(4) = 6
    assert_eq!(bytes.len(), 6);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x291c);
}

#[test]
fn phase_shift_change_default_matches_cpp_empty_layout() {
    let pkt = PhaseShiftChange::default_for(ObjectGuid::EMPTY);
    let mut body = crate::WorldPacket::new_empty();
    pkt.write(&mut body);
    let bytes = body.into_data();

    assert_eq!(bytes.len(), 24);
    assert_eq!(&bytes[0..2], &[0, 0]); // packed Client GUID
    assert_eq!(u32::from_le_bytes(bytes[2..6].try_into().unwrap()), 0x08);
    assert_eq!(u32::from_le_bytes(bytes[6..10].try_into().unwrap()), 0);
    assert_eq!(&bytes[10..12], &[0, 0]); // packed PersonalGUID
    assert_eq!(u32::from_le_bytes(bytes[12..16].try_into().unwrap()), 0);
    assert_eq!(u32::from_le_bytes(bytes[16..20].try_into().unwrap()), 0);
    assert_eq!(u32::from_le_bytes(bytes[20..24].try_into().unwrap()), 0);
}

#[test]
fn phase_shift_change_visible_map_ids_use_cpp_byte_size_prefix() {
    let pkt = PhaseShiftChange::with_visible_map_ids(ObjectGuid::EMPTY, vec![609, 700]);
    let mut body = crate::WorldPacket::new_empty();
    pkt.write(&mut body);
    let bytes = body.into_data();

    assert_eq!(bytes.len(), 28);
    assert_eq!(u32::from_le_bytes(bytes[12..16].try_into().unwrap()), 4);
    assert_eq!(u16::from_le_bytes(bytes[16..18].try_into().unwrap()), 609);
    assert_eq!(u16::from_le_bytes(bytes[18..20].try_into().unwrap()), 700);
    assert_eq!(u32::from_le_bytes(bytes[20..24].try_into().unwrap()), 0);
    assert_eq!(u32::from_le_bytes(bytes[24..28].try_into().unwrap()), 0);
}

#[test]
fn available_hotfixes_empty_serializes() {
    let pkt = AvailableHotfixes {
        virtual_realm_address: 1,
        hotfixes: Vec::new(),
    };
    let bytes = pkt.to_bytes();
    // opcode(2) + uint32(4) + int32(4) = 10
    assert_eq!(bytes.len(), 10);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x290f);
}

#[test]
fn available_hotfixes_serializes_ids() {
    let pkt = AvailableHotfixes {
        virtual_realm_address: 0x1122_3344,
        hotfixes: vec![HotfixId {
            push_id: 7,
            unique_id: 9,
        }],
    };
    let bytes = pkt.to_bytes();
    assert_eq!(bytes.len(), 18);
    assert_eq!(
        u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]),
        0x1122_3344
    );
    assert_eq!(
        u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]),
        1
    );
    assert_eq!(
        i32::from_le_bytes([bytes[10], bytes[11], bytes[12], bytes[13]]),
        7
    );
    assert_eq!(
        u32::from_le_bytes([bytes[14], bytes[15], bytes[16], bytes[17]]),
        9
    );
}

#[test]
fn connection_status_serializes() {
    let pkt = ConnectionStatus {
        state: 1,
        suppress_notification: true,
    };
    let bytes = pkt.to_bytes();
    // opcode(2) + 3 bits flushed to 1 byte = 3
    assert_eq!(bytes.len(), 3);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x2809);
}

#[test]
fn set_timezone_utc() {
    let pkt = SetTimeZoneInformation::utc();
    let bytes = pkt.to_bytes();
    // Should contain "Etc/UTC" x3
    assert!(bytes.len() > 20);
}

#[test]
fn login_set_time_speed_now() {
    let pkt = LoginSetTimeSpeed::now();
    let bytes = pkt.to_bytes();
    // opcode(2) + 4*i32(16) + float(4) = 22
    assert_eq!(bytes.len(), 22);
}

#[test]
fn setup_currency_empty() {
    let pkt = SetupCurrency::empty();
    let bytes = pkt.to_bytes();
    // opcode(2) + i32(4) = 6
    assert_eq!(bytes.len(), 6);
}

#[test]
fn setup_currency_record_matches_cpp_bit_and_field_order() {
    let pkt = SetupCurrency::from_records(vec![SetupCurrencyRecord {
        type_id: 395,
        quantity: 123,
        weekly_quantity: Some(20),
        max_weekly_quantity: Some(50),
        tracked_quantity: Some(7),
        max_quantity: Some(200),
        total_earned: Some(300),
        next_recharge_time: None,
        recharge_cycle_start_time: None,
        flags: 0x0c,
    }]);
    let bytes = pkt.to_bytes();
    let mut body = WorldPacket::from_bytes(&bytes);
    assert_eq!(body.read_uint16().unwrap(), 0x2573);
    assert_eq!(body.read_uint32().unwrap(), 1);
    assert_eq!(body.read_int32().unwrap(), 395);
    assert_eq!(body.read_int32().unwrap(), 123);
    assert!(body.read_bit().unwrap());
    assert!(body.read_bit().unwrap());
    assert!(body.read_bit().unwrap());
    assert!(body.read_bit().unwrap());
    assert!(body.read_bit().unwrap());
    assert!(!body.read_bit().unwrap());
    assert!(!body.read_bit().unwrap());
    assert_eq!(body.read_bits(5).unwrap(), 0x0c);
    assert_eq!(body.read_uint32().unwrap(), 20);
    assert_eq!(body.read_uint32().unwrap(), 50);
    assert_eq!(body.read_uint32().unwrap(), 7);
    assert_eq!(body.read_int32().unwrap(), 200);
    assert_eq!(body.read_int32().unwrap(), 300);
}

#[test]
fn set_currency_vendor_loss_matches_cpp_field_order() {
    let pkt = SetCurrency::vendor_loss(395, 90, 10);
    let bytes = pkt.to_bytes();
    assert_eq!(bytes.len(), 28);
    assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x2574);
    assert_eq!(i32::from_le_bytes(bytes[2..6].try_into().unwrap()), 395);
    assert_eq!(i32::from_le_bytes(bytes[6..10].try_into().unwrap()), 90);
    assert_eq!(u32::from_le_bytes(bytes[10..14].try_into().unwrap()), 0);
    assert_eq!(u32::from_le_bytes(bytes[14..18].try_into().unwrap()), 0);
    assert_eq!(bytes[18], 0x05);
    assert_eq!(bytes[19], 0x00);
    assert_eq!(i32::from_le_bytes(bytes[20..24].try_into().unwrap()), -10);
    assert_eq!(i32::from_le_bytes(bytes[24..28].try_into().unwrap()), 4);
}

#[test]
fn set_currency_vendor_gain_matches_cpp_source() {
    let pkt = SetCurrency::vendor_gain(395, 110, 10);
    let bytes = pkt.to_bytes();
    assert_eq!(bytes.len(), 28);
    assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x2574);
    assert_eq!(i32::from_le_bytes(bytes[2..6].try_into().unwrap()), 395);
    assert_eq!(i32::from_le_bytes(bytes[6..10].try_into().unwrap()), 110);
    assert_eq!(u32::from_le_bytes(bytes[10..14].try_into().unwrap()), 0);
    assert_eq!(u32::from_le_bytes(bytes[14..18].try_into().unwrap()), 0);
    assert_eq!(bytes[18], 0x06);
    assert_eq!(bytes[19], 0x00);
    assert_eq!(i32::from_le_bytes(bytes[20..24].try_into().unwrap()), 10);
    assert_eq!(i32::from_le_bytes(bytes[24..28].try_into().unwrap()), 5);
}

#[test]
fn set_currency_item_refund_gain_matches_cpp_source() {
    let pkt = SetCurrency::item_refund_gain(395, 110, 10, None, None, None, false);
    let bytes = pkt.to_bytes();
    assert_eq!(bytes.len(), 28);
    assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x2574);
    assert_eq!(i32::from_le_bytes(bytes[2..6].try_into().unwrap()), 395);
    assert_eq!(i32::from_le_bytes(bytes[6..10].try_into().unwrap()), 110);
    assert_eq!(bytes[18], 0x06);
    assert_eq!(bytes[19], 0x00);
    assert_eq!(i32::from_le_bytes(bytes[20..24].try_into().unwrap()), 10);
    assert_eq!(i32::from_le_bytes(bytes[24..28].try_into().unwrap()), 2);
}

#[test]
fn init_world_states_empty() {
    let pkt = InitWorldStates::new(0, 12);
    let bytes = pkt.to_bytes();
    // opcode(2) + 4*i32(16) = 18
    assert_eq!(bytes.len(), 18);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x2746);
}

#[test]
fn init_world_states_serializes_cpp_worldstate_pairs() {
    let pkt = InitWorldStates::with_world_states(571, 4395, 4613, vec![(46, 1), (24098, 0)]);
    let bytes = pkt.to_bytes();
    assert_eq!(bytes.len(), 34);
    assert_eq!(u16::from_le_bytes(bytes[0..2].try_into().unwrap()), 0x2746);
    assert_eq!(i32::from_le_bytes(bytes[2..6].try_into().unwrap()), 571);
    assert_eq!(i32::from_le_bytes(bytes[6..10].try_into().unwrap()), 4395);
    assert_eq!(i32::from_le_bytes(bytes[10..14].try_into().unwrap()), 4613);
    assert_eq!(u32::from_le_bytes(bytes[14..18].try_into().unwrap()), 2);
    assert_eq!(i32::from_le_bytes(bytes[18..22].try_into().unwrap()), 46);
    assert_eq!(i32::from_le_bytes(bytes[22..26].try_into().unwrap()), 1);
    assert_eq!(i32::from_le_bytes(bytes[26..30].try_into().unwrap()), 24098);
    assert_eq!(i32::from_le_bytes(bytes[30..34].try_into().unwrap()), 0);
}

#[test]
fn update_talent_data_empty() {
    let pkt = UpdateTalentData::default();
    let bytes = pkt.to_bytes();
    // opcode(2) + int32(4) + uint8(1) + int32(4) +
    // TalentGroupInfo: uint8(1)+uint32(4)+uint8(1)+uint32(4)+uint8(1)+6*uint16(12) +
    // trailing bit(IsPetTalents) is not flushed by C++ `UpdateTalentData::Write`.
    assert_eq!(bytes.len(), 34);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x25d7);
}

#[test]
fn update_talent_data_writes_glyph_ids_like_cpp() {
    let mut group = TalentGroupInfoLikeCpp::default();
    group.spec_id = 4;
    group.glyph_ids = [101, 0, 202, 0, 0, 303];
    let pkt = UpdateTalentData {
        active_group: 2,
        groups: vec![group],
        ..UpdateTalentData::default()
    };
    let bytes = pkt.to_bytes();

    assert_eq!(bytes[6], 2);
    assert_eq!(bytes[21], 4); // C++ writes SpecID after glyph/talent counts.
    let glyphs_start = 22;
    assert_eq!(
        u16::from_le_bytes([bytes[glyphs_start], bytes[glyphs_start + 1]]),
        101
    );
    assert_eq!(
        u16::from_le_bytes([bytes[glyphs_start + 4], bytes[glyphs_start + 5]]),
        202
    );
    assert_eq!(
        u16::from_le_bytes([bytes[glyphs_start + 10], bytes[glyphs_start + 11]]),
        303
    );
}

#[test]
fn send_known_spells_empty() {
    let pkt = SendKnownSpells::empty();
    let bytes = pkt.to_bytes();
    // opcode(2) + bit(flush)+int32(4)+int32(4) = 2+1+4+4 = 11
    assert_eq!(bytes.len(), 11);
}

#[test]
fn send_known_spells_with_data() {
    let pkt = SendKnownSpells {
        initial_login: true,
        known_spells: vec![6603, 78, 2457],
        favorite_spells: vec![2457],
    };
    let bytes = pkt.to_bytes();
    // opcode(2) + bit(flush)(1) + count(4) + fav_count(4) + 4*i32(16) = 27
    assert_eq!(bytes.len(), 27);
}

#[test]
fn send_spell_history_empty() {
    let pkt = SendSpellHistory::empty();
    let bytes = pkt.to_bytes();
    // opcode(2) + int32(4) = 6
    assert_eq!(bytes.len(), 6);
}

#[test]
fn send_spell_history_with_entry_matches_cpp_layout() {
    let pkt = SendSpellHistory {
        entries: vec![SpellHistoryEntry {
            spell_id: 133,
            item_id: 6948,
            category: 12,
            recovery_time_ms: 30_000,
            category_recovery_time_ms: 10_000,
            mod_rate: 1.0,
            on_hold: false,
        }],
    };
    let bytes = pkt.to_bytes();
    // opcode(2) + count(4) + entry(5*u32/i32 + f32 + 3 bits flushed to 1 byte) = 31
    assert_eq!(bytes.len(), 31);
}

#[test]
fn send_spell_charges_empty() {
    let pkt = SendSpellCharges::empty();
    let bytes = pkt.to_bytes();
    // opcode(2) + uint32(4) = 6
    assert_eq!(bytes.len(), 6);
}

#[test]
fn send_spell_charges_with_entry_matches_cpp_layout() {
    let pkt = SendSpellCharges {
        entries: vec![SpellChargeEntry {
            category: 42,
            next_recovery_time_ms: 45_000,
            charge_mod_rate: 1.0,
            consumed_charges: 2,
        }],
    };
    let bytes = pkt.to_bytes();
    // opcode(2) + count(4) + category(4) + next(4) + mod_rate(4) + consumed(1) = 19
    assert_eq!(bytes.len(), 19);
}

#[test]
fn update_action_buttons_empty() {
    let pkt = UpdateActionButtons::empty();
    let bytes = pkt.to_bytes();
    // opcode(2) + 180*i64(1440) + uint8(1) = 1443
    assert_eq!(bytes.len(), 1443);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x25e0);
}

#[test]
fn update_action_buttons_pack() {
    // Spell 6603 (Auto Attack) as type 0 (Spell)
    let packed = UpdateActionButtons::pack_button(6603, 0);
    assert_eq!(packed, 6603);

    // Spell 78 (Heroic Strike) as type 0
    let packed = UpdateActionButtons::pack_button(78, 0);
    assert_eq!(packed, 78);

    // Item action as type 2
    let packed = UpdateActionButtons::pack_button(12345, 2);
    // C++ player action buttons use `action | (type << 24)`.
    assert_eq!(packed, 12345 | (2i64 << 24));
}

#[test]
fn initialize_factions_empty() {
    let pkt = InitializeFactions::default();
    let bytes = pkt.to_bytes();
    // opcode(2) + 1000*(uint16+int32) + ceil(1000/8) = 2 + 6000 + 125 = 6127
    assert_eq!(bytes.len(), 6127);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x2724);
}

#[test]
fn bind_point_update_serializes() {
    let pkt = BindPointUpdate {
        x: 1.0,
        y: 2.0,
        z: 3.0,
        map_id: 0,
        area_id: 12,
    };
    let bytes = pkt.to_bytes();
    // opcode(2) + 3*f32(12) + 2*i32(8) = 22
    assert_eq!(bytes.len(), 22);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x257d);
}

#[test]
fn bind_point_update_preserves_full_uint32_area_like_cpp() {
    let bytes = BindPointUpdate {
        x: 1.0,
        y: 2.0,
        z: 3.0,
        map_id: 571,
        area_id: u32::MAX,
    }
    .to_bytes();
    assert_eq!(
        u32::from_le_bytes(bytes[18..22].try_into().unwrap()),
        u32::MAX
    );
}

#[test]
fn player_bound_serializes_packed_guid_and_area_like_cpp() {
    let binder_id = wow_core::ObjectGuid::new(0x0102_0304_0506_0708, 0x1112_1314_1516_1718);
    let pkt = PlayerBound {
        binder_id,
        area_id: 42,
    };

    let bytes = pkt.to_bytes();
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x2ff8);

    let mut payload = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(payload.read_packed_guid().unwrap(), binder_id);
    assert_eq!(payload.read_uint32().unwrap(), 42);
    assert_eq!(payload.remaining(), 0);
}

#[test]
fn world_server_info_serializes() {
    let pkt = WorldServerInfo::default_open_world();
    let bytes = pkt.to_bytes();
    // opcode(2) + int32(4) + 5 bits flushed to 1 byte = 7
    assert_eq!(bytes.len(), 7);
}

#[test]
fn initial_setup_wotlk() {
    let pkt = InitialSetup::wotlk();
    let bytes = pkt.to_bytes();
    // opcode(2) + uint8(1) + uint8(1) = 4
    assert_eq!(bytes.len(), 4);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x2580);
}

#[test]
fn time_sync_request_serializes() {
    let pkt = TimeSyncRequest { sequence_index: 0 };
    let bytes = pkt.to_bytes();
    // opcode(2) + u32(4) = 6
    assert_eq!(bytes.len(), 6);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x2dd2);
}

#[test]
fn time_sync_response_reads_cpp_wire_order() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&7u32.to_le_bytes());
    bytes.extend_from_slice(&123_456u32.to_le_bytes());

    let mut pkt = WorldPacket::from_bytes(&bytes);
    let response = TimeSyncResponse::read(&mut pkt).expect("valid TimeSyncResponse");

    assert_eq!(response.sequence_index, 7);
    assert_eq!(response.client_time, 123_456);
}

#[test]
fn contact_list_empty() {
    let pkt = ContactList::all();
    let bytes = pkt.to_bytes();
    // opcode(2) + u32(4) + bits(8→1 byte) = 7
    assert_eq!(bytes.len(), 7);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x278c);
    // Flags = 7 (All)
    let flags = u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
    assert_eq!(flags, 7);
}

#[test]
fn active_glyphs_empty() {
    let pkt = ActiveGlyphs {
        glyphs: Vec::new(),
        is_full_update: true,
    };
    let bytes = pkt.to_bytes();
    // opcode(2) + i32(4) + 1 bit flushed to 1 byte = 7
    assert_eq!(bytes.len(), 7);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x2c51);
}

#[test]
fn active_glyphs_writes_bindings_like_cpp() {
    let pkt = ActiveGlyphs {
        glyphs: vec![GlyphBindingLikeCpp {
            spell_id: 12345,
            glyph_id: 678,
        }],
        is_full_update: false,
    };
    let bytes = pkt.to_bytes();

    assert_eq!(bytes.len(), 13);
    assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x2c51);
    assert_eq!(u32::from_le_bytes(bytes[2..6].try_into().unwrap()), 1);
    assert_eq!(u32::from_le_bytes(bytes[6..10].try_into().unwrap()), 12345);
    assert_eq!(u16::from_le_bytes(bytes[10..12].try_into().unwrap()), 678);
}

#[test]
fn load_equipment_set_empty() {
    let pkt = LoadEquipmentSet::default();
    let bytes = pkt.to_bytes();
    // opcode(2) + i32(4) = 6
    assert_eq!(bytes.len(), 6);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x270e);
}

#[test]
fn all_account_criteria_empty() {
    let pkt = AllAccountCriteria;
    let bytes = pkt.to_bytes();
    // opcode(2) + i32(4) = 6
    assert_eq!(bytes.len(), 6);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x2571);
}

#[test]
fn all_achievement_data_empty() {
    let pkt = AllAchievementData;
    let bytes = pkt.to_bytes();
    // opcode(2) + i32(4) + i32(4) = 10
    assert_eq!(bytes.len(), 10);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x2570);
}

#[test]
fn account_mount_update_empty() {
    let pkt = AccountMountUpdate::empty_full();
    let bytes = pkt.to_bytes();
    // opcode(2) + 1 bit(padded to 1 byte) + i32(4) = 7
    // wait: write_bit(true) → 1 bit buffered, then write_int32(0)
    // auto-flushes → 1 byte (bit), then 4 bytes (i32), then flush_bits (no-op) = 7
    assert_eq!(bytes.len(), 7);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x25ae);
}

#[test]
fn account_mount_update_writes_mount_entries_like_cpp() {
    let pkt = AccountMountUpdate::full(vec![
        AccountMount {
            spell_id: 100,
            flags: 0x01,
        },
        AccountMount {
            spell_id: 200,
            flags: 0x12,
        },
    ]);
    let bytes = pkt.to_bytes();

    assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x25ae);
    assert_eq!(bytes[2], 0x80);
    assert_eq!(
        i32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]),
        2
    );
    assert_eq!(
        i32::from_le_bytes([bytes[7], bytes[8], bytes[9], bytes[10]]),
        100
    );
    assert_eq!(bytes[11], 0x10);
    assert_eq!(
        i32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
        200
    );
    assert_eq!(bytes[16], 0x20);
}

#[test]
fn account_heirloom_update_writes_items_then_flags_like_cpp() {
    let pkt = AccountHeirloomUpdate::full(vec![
        AccountHeirloom {
            item_id: 44_000,
            flags: 0x01,
        },
        AccountHeirloom {
            item_id: 44_001,
            flags: 0x04,
        },
    ]);
    let bytes = pkt.to_bytes();

    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::UpdateCapturePoint as u16
    );
    assert_eq!(bytes[2], 0x80);
    assert_eq!(
        i32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]),
        0
    );
    assert_eq!(
        u32::from_le_bytes([bytes[7], bytes[8], bytes[9], bytes[10]]),
        2
    );
    assert_eq!(
        u32::from_le_bytes([bytes[11], bytes[12], bytes[13], bytes[14]]),
        2
    );
    assert_eq!(
        i32::from_le_bytes([bytes[15], bytes[16], bytes[17], bytes[18]]),
        44_000
    );
    assert_eq!(
        i32::from_le_bytes([bytes[19], bytes[20], bytes[21], bytes[22]]),
        44_001
    );
    assert_eq!(
        u32::from_le_bytes([bytes[23], bytes[24], bytes[25], bytes[26]]),
        0x01
    );
    assert_eq!(
        u32::from_le_bytes([bytes[27], bytes[28], bytes[29], bytes[30]]),
        0x04
    );
}

#[test]
fn account_mount_update_partial_clears_full_update_bit_like_cpp() {
    let pkt = AccountMountUpdate::partial(vec![AccountMount {
        spell_id: 100,
        flags: 0x01,
    }]);
    let bytes = pkt.to_bytes();

    assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x25ae);
    assert_eq!(bytes[2], 0x00);
    assert_eq!(
        i32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]),
        1
    );
}

#[test]
fn mount_result_writes_result_int32_like_cpp() {
    let bytes = MountResult {
        result: MOUNT_RESULT_SHAPESHIFTED_LIKE_CPP,
    }
    .to_bytes();

    assert_eq!(
        bytes[0..2],
        (ServerOpcodes::MountResult as u16).to_le_bytes()
    );
    assert_eq!(
        i32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]),
        MOUNT_RESULT_SHAPESHIFTED_LIKE_CPP
    );
    assert_eq!(bytes.len(), 6);
}

#[test]
fn mount_set_favorite_reads_cpp_field_order() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::MountSetFavorite as u16);
    pkt.write_uint32(1234);
    pkt.write_bit(true);
    pkt.flush_bits();

    let decoded = MountSetFavorite::read(&mut pkt).unwrap();
    assert_eq!(
        decoded,
        MountSetFavorite {
            mount_spell_id: 1234,
            is_favorite: true,
        }
    );
}

#[test]
fn mount_special_reads_count_sequence_and_visual_kits_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::MountSpecialAnim as u16);
    pkt.write_uint32(2);
    pkt.write_int32(-7);
    pkt.write_int32(111);
    pkt.write_int32(222);

    let decoded = MountSpecial::read(&mut pkt).unwrap();
    assert_eq!(
        decoded,
        MountSpecial {
            spell_visual_kit_ids: vec![111, 222],
            sequence_variation: -7,
        }
    );
}

#[test]
fn special_mount_anim_writes_guid_count_sequence_and_visual_kits_like_cpp() {
    let guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Player, 0, 1, 571, 0, 0, 42);
    let bytes = SpecialMountAnim {
        unit_guid: guid,
        spell_visual_kit_ids: vec![111, -222],
        sequence_variation: 3,
    }
    .to_bytes();

    assert_eq!(
        bytes[0..2],
        (ServerOpcodes::SpecialMountAnim as u16).to_le_bytes()
    );
    assert_eq!(&bytes[2..18], &guid.to_raw_bytes());
    assert_eq!(&bytes[18..22], &2_u32.to_le_bytes());
    assert_eq!(&bytes[22..26], &3_i32.to_le_bytes());
    assert_eq!(&bytes[26..30], &111_i32.to_le_bytes());
    assert_eq!(&bytes[30..34], &(-222_i32).to_le_bytes());
    assert_eq!(bytes.len(), 34);
}

#[test]
fn account_toy_update_empty() {
    let pkt = AccountToyUpdate::full(Vec::new());
    let bytes = pkt.to_bytes();
    // opcode(2) + 1 bit(padded to 1 byte) + 3*i32(12) = 15
    assert_eq!(bytes.len(), 15);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x25b0);
}

#[test]
fn save_cuf_profiles_reads_cpp_shape() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::SaveCufProfiles as u16);
    pkt.write_uint32(1);
    pkt.write_bits(4, 7);
    for option in 0..CUF_BOOL_OPTIONS_COUNT_LIKE_CPP {
        pkt.write_bit(matches!(option, 0 | 5 | 26));
    }
    pkt.write_uint16(72);
    pkt.write_uint16(128);
    pkt.write_uint8(2);
    pkt.write_uint8(3);
    pkt.write_uint8(4);
    pkt.write_uint8(5);
    pkt.write_uint8(6);
    pkt.write_uint16(7);
    pkt.write_uint16(8);
    pkt.write_uint16(9);
    pkt.write_string("Raid");

    let mut packet = WorldPacket::from_bytes(pkt.data());
    let parsed = SaveCufProfiles::read(&mut packet).expect("valid SaveCUFProfiles");

    assert_eq!(parsed.profiles.len(), 1);
    let profile = &parsed.profiles[0];
    assert_eq!(profile.profile_name, "Raid");
    assert_eq!(profile.frame_height, 72);
    assert_eq!(profile.frame_width, 128);
    assert_eq!(profile.sort_by, 2);
    assert_eq!(profile.health_text, 3);
    assert_eq!(profile.top_point, 4);
    assert_eq!(profile.bottom_point, 5);
    assert_eq!(profile.left_point, 6);
    assert_eq!(profile.top_offset, 7);
    assert_eq!(profile.bottom_offset, 8);
    assert_eq!(profile.left_offset, 9);
    assert_eq!(profile.bool_options, (1 << 0) | (1 << 5) | (1 << 26));
}

#[test]
fn tutorial_set_flag_reads_update_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::Tutorial as u16);
    pkt.write_bits(TUTORIAL_ACTION_UPDATE_LIKE_CPP as u32, 2);
    pkt.write_uint32(37);

    let mut packet = WorldPacket::from_bytes(pkt.data());
    let parsed = TutorialSetFlag::read(&mut packet).expect("valid CMSG_TUTORIAL update");

    assert_eq!(parsed.action, TUTORIAL_ACTION_UPDATE_LIKE_CPP);
    assert_eq!(parsed.tutorial_bit, Some(37));
}

#[test]
fn tutorial_set_flag_reads_clear_without_bit_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::Tutorial as u16);
    pkt.write_bits(TUTORIAL_ACTION_CLEAR_LIKE_CPP as u32, 2);
    pkt.flush_bits();

    let mut packet = WorldPacket::from_bytes(pkt.data());
    let parsed = TutorialSetFlag::read(&mut packet).expect("valid CMSG_TUTORIAL clear");

    assert_eq!(parsed.action, TUTORIAL_ACTION_CLEAR_LIKE_CPP);
    assert_eq!(parsed.tutorial_bit, None);
}

#[test]
fn load_cuf_profiles_writes_count_and_fields_like_cpp() {
    let bytes = LoadCufProfiles {
        profiles: vec![CufProfile {
            profile_name: "Raid".to_string(),
            frame_height: 72,
            frame_width: 128,
            sort_by: 2,
            health_text: 3,
            top_point: 4,
            bottom_point: 5,
            left_point: 6,
            top_offset: 7,
            bottom_offset: 8,
            left_offset: 9,
            bool_options: (1 << 0) | (1 << 5) | (1 << 26),
        }],
    }
    .to_bytes();

    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::LoadCufProfiles as u16
    );
    assert_eq!(
        u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]),
        1
    );
}

#[test]
fn account_toy_update_writes_ids_then_flag_bits_like_cpp() {
    let pkt = AccountToyUpdate::full(vec![
        AccountToy {
            item_id: 30_000,
            is_favorite: true,
            has_fanfare: false,
        },
        AccountToy {
            item_id: 30_001,
            is_favorite: false,
            has_fanfare: true,
        },
    ]);
    let bytes = pkt.to_bytes();

    assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x25b0);
    assert_eq!(bytes[2], 0x80);
    assert_eq!(
        i32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]),
        2
    );
    assert_eq!(
        i32::from_le_bytes([bytes[7], bytes[8], bytes[9], bytes[10]]),
        2
    );
    assert_eq!(
        i32::from_le_bytes([bytes[11], bytes[12], bytes[13], bytes[14]]),
        2
    );
    assert_eq!(
        u32::from_le_bytes([bytes[15], bytes[16], bytes[17], bytes[18]]),
        30_000
    );
    assert_eq!(
        u32::from_le_bytes([bytes[19], bytes[20], bytes[21], bytes[22]]),
        30_001
    );
    assert_eq!(bytes[23], 0b1001_0000);
}

#[test]
fn add_toy_reads_cpp_guid_payload() {
    let guid = ObjectGuid::create_item(1, 99);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::AddToy as u16);
    pkt.write_packed_guid(&guid);

    let decoded = AddToy::read(&mut pkt).unwrap();
    assert_eq!(decoded.item_guid, guid);
}

#[test]
fn toy_clear_fanfare_reads_cpp_item_id_payload() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::ToyClearFanfare as u16);
    pkt.write_uint32(30_000);

    let decoded = ToyClearFanfare::read(&mut pkt).unwrap();
    assert_eq!(decoded.item_id, 30_000);
}

fn write_minimal_toy_spell_cast(
    pkt: &mut WorldPacket,
    cast_id: ObjectGuid,
    item_id: i32,
    spell_id: i32,
) {
    use crate::packets::spell::{SpellCastVisual, SpellTargetData};

    pkt.write_packed_guid(&cast_id);
    pkt.write_int32(item_id);
    pkt.write_int32(0);
    pkt.write_int32(spell_id);
    SpellCastVisual::default().write(pkt);
    pkt.write_float(0.0);
    pkt.write_float(0.0);
    pkt.write_packed_guid(&ObjectGuid::EMPTY);
    pkt.write_uint32(0);
    pkt.write_uint32(0);
    pkt.write_uint32(0);
    pkt.write_bits(0, 5);
    pkt.write_bit(false);
    pkt.write_bits(0, 2);
    pkt.write_bit(false);
    pkt.flush_bits();
    SpellTargetData::default().write(pkt);
}

#[test]
fn use_toy_reads_spell_cast_request_like_cpp() {
    let cast_id = ObjectGuid::create_player(1, 123);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::UseToy as u16);
    write_minimal_toy_spell_cast(&mut pkt, cast_id, 30_000, 12_345);

    let decoded = UseToy::read(&mut pkt).unwrap();
    assert_eq!(decoded.cast.cast_id, cast_id);
    assert_eq!(decoded.cast.misc[0], 30_000);
    assert_eq!(decoded.cast.spell_id, 12_345);
}

#[test]
fn load_cuf_profiles_empty() {
    let pkt = LoadCufProfiles::empty();
    let bytes = pkt.to_bytes();
    // opcode(2) + i32(4) = 6
    assert_eq!(bytes.len(), 6);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x25bc);
}

#[test]
fn aura_update_empty() {
    let guid = ObjectGuid::create_player(1, 42);
    let pkt = AuraUpdate::empty_for(guid);
    let bytes = pkt.to_bytes();
    // opcode(2) + 10 bits(padded to 2 bytes) + packed_guid(variable)
    assert!(bytes.len() > 4);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x2c1f);
    // Byte 2: UpdateAll=1(MSB) + first 7 bits of count(0) = 0x80
    assert_eq!(bytes[2], 0x80);
}

#[test]
fn aura_update_single_passive_matches_cpp_shape() {
    let unit_guid = ObjectGuid::create_player(1, 2);
    let cast_id =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Cast, 3, 1, 571, 0, 822, 1);
    let pkt = AuraUpdate {
        unit_guid,
        update_all: false,
        auras: vec![AuraInfoLikeCpp {
            slot: 0,
            aura_data: Some(AuraDataInfoLikeCpp {
                cast_id,
                spell_id: 822,
                flags: 0x0301,
                active_flags: 0x1,
                caster_guid: unit_guid,
                cast_level: 80,
                applications: 0,
                duration_ms: None,
                remaining_ms: None,
                points: Vec::new(),
            }),
        }],
    };
    let bytes = pkt.to_bytes();
    let expected = [
        0x1f, 0x2c, 0x00, 0x40, 0x00, 0x80, 0x01, 0xbb, 0x01, 0x83, 0xcd, 0x60, 0x47, 0x04, 0xbc,
        0x36, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x03, 0x01, 0x00, 0x00, 0x00, 0x50,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0xa0, 0x02, 0x04, 0x08,
    ];
    assert_eq!(bytes, expected);
}

#[test]
fn battle_pet_journal_lock_acquired_empty() {
    let pkt = BattlePetJournalLockAcquired;
    let bytes = pkt.to_bytes();
    // opcode(2) + no payload = 2
    assert_eq!(bytes.len(), 2);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x25ed);
}

#[test]
fn battle_pet_journal_lock_denied_empty() {
    let pkt = BattlePetJournalLockDenied;
    let bytes = pkt.to_bytes();
    assert_eq!(bytes.len(), 2);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x25ee);
}

#[test]
fn battle_pet_deleted_writes_packed_guid_like_cpp() {
    let pet_guid = ObjectGuid::new(0, 0x4330);
    let bytes = BattlePetDeleted { pet_guid }.to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::BattlePetDeleted as u16
    );

    let mut body = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(body.read_packed_guid().unwrap(), pet_guid);
    assert_eq!(body.remaining(), 0);
}

#[test]
fn battle_pet_error_writes_result_bits_then_creature_id_like_cpp() {
    let bytes =
        BattlePetError::new(BattlePetErrorCodeLikeCpp::TooHighLevelToUncage, 12_345).to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::BattlePetError as u16
    );

    let mut body = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(
        body.read_bits(4).unwrap(),
        BattlePetErrorCodeLikeCpp::TooHighLevelToUncage as u32
    );
    assert_eq!(body.read_int32().unwrap(), 12_345);
    assert_eq!(body.remaining(), 0);
}

#[test]
fn battle_pet_request_journal_reads_empty_payload_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::BattlePetRequestJournal as u16);

    assert_eq!(
        BattlePetRequestJournal::read(&mut pkt).unwrap(),
        BattlePetRequestJournal
    );
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn battle_pet_request_journal_lock_reads_empty_payload_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::BattlePetRequestJournalLock as u16);

    assert_eq!(
        BattlePetRequestJournalLock::read(&mut pkt).unwrap(),
        BattlePetRequestJournalLock
    );
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn battle_pet_journal_writes_empty_default_slots_like_cpp() {
    let bytes = BattlePetJournal::empty_with_default_slots(true).to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::BattlePetJournal as u16
    );

    let mut body = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(body.read_uint16().unwrap(), 0);
    assert_eq!(body.read_uint32().unwrap(), 3);
    assert_eq!(body.read_uint32().unwrap(), 0);
    assert!(body.read_bit().unwrap());

    for index in 0..3 {
        let slot_guid = body.read_packed_guid().unwrap();
        assert_eq!(slot_guid, empty_battle_pet_guid_like_cpp());
        assert_eq!(slot_guid.high_type(), HighGuid::BattlePet);
        assert_eq!(body.read_uint32().unwrap(), 0);
        assert_eq!(body.read_uint8().unwrap(), index);
        assert!(body.read_bit().unwrap());
    }
    assert_eq!(body.remaining(), 0);
}

fn sample_battle_pet_journal_pet_like_cpp(
    pet_guid: ObjectGuid,
    owner_guid: ObjectGuid,
) -> BattlePetJournalPet {
    BattlePetJournalPet {
        guid: pet_guid,
        species: 11,
        creature_id: 22,
        display_id: 33,
        breed: 44,
        level: 55,
        exp: 66,
        flags: 77,
        power: 88,
        health: 99,
        max_health: 111,
        speed: 222,
        quality: 3,
        owner_info: Some(BattlePetJournalPetOwnerInfo {
            guid: owner_guid,
            player_virtual_realm: 123,
            player_native_realm: 456,
        }),
        name: "Misha".to_string(),
    }
}

fn assert_sample_battle_pet_journal_pet_like_cpp(
    body: &mut WorldPacket,
    pet_guid: ObjectGuid,
    owner_guid: ObjectGuid,
) {
    assert_eq!(body.read_packed_guid().unwrap(), pet_guid);
    assert_eq!(body.read_uint32().unwrap(), 11);
    assert_eq!(body.read_uint32().unwrap(), 22);
    assert_eq!(body.read_uint32().unwrap(), 33);
    assert_eq!(body.read_uint16().unwrap(), 44);
    assert_eq!(body.read_uint16().unwrap(), 55);
    assert_eq!(body.read_uint16().unwrap(), 66);
    assert_eq!(body.read_uint16().unwrap(), 77);
    assert_eq!(body.read_uint32().unwrap(), 88);
    assert_eq!(body.read_uint32().unwrap(), 99);
    assert_eq!(body.read_uint32().unwrap(), 111);
    assert_eq!(body.read_uint32().unwrap(), 222);
    assert_eq!(body.read_uint8().unwrap(), 3);
    assert_eq!(body.read_bits(7).unwrap(), 5);
    assert!(body.read_bit().unwrap());
    assert!(!body.read_bit().unwrap());
    assert_eq!(body.read_string(5).unwrap(), "Misha");
    assert_eq!(body.read_packed_guid().unwrap(), owner_guid);
    assert_eq!(body.read_uint32().unwrap(), 123);
    assert_eq!(body.read_uint32().unwrap(), 456);
}

#[test]
fn battle_pet_journal_writes_pet_rows_like_cpp() {
    let pet_guid = ObjectGuid::new(0, 0x4335);
    let owner_guid = ObjectGuid::create_player(1, 77);
    let bytes = BattlePetJournal {
        trap: 9,
        has_journal_lock: true,
        slots: Vec::new(),
        pets: vec![sample_battle_pet_journal_pet_like_cpp(pet_guid, owner_guid)],
    }
    .to_bytes();

    let mut body = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(body.read_uint16().unwrap(), 9);
    assert_eq!(body.read_uint32().unwrap(), 0);
    assert_eq!(body.read_uint32().unwrap(), 1);
    assert!(body.read_bit().unwrap());
    assert_sample_battle_pet_journal_pet_like_cpp(&mut body, pet_guid, owner_guid);
    assert_eq!(body.remaining(), 0);
}

#[test]
fn battle_pet_updates_writes_count_flag_then_pet_rows_like_cpp() {
    let pet_guid = ObjectGuid::new(0, 0x4336);
    let owner_guid = ObjectGuid::create_player(1, 78);
    let bytes = BattlePetUpdates {
        pets: vec![sample_battle_pet_journal_pet_like_cpp(pet_guid, owner_guid)],
        pet_added: true,
    }
    .to_bytes();

    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::BattlePetUpdates as u16
    );
    let mut body = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(body.read_uint32().unwrap(), 1);
    assert!(body.read_bit().unwrap());
    assert_sample_battle_pet_journal_pet_like_cpp(&mut body, pet_guid, owner_guid);
    assert_eq!(body.remaining(), 0);
}

#[test]
fn pet_battle_slot_updates_writes_flags_then_slots_like_cpp() {
    let pet_guid = ObjectGuid::new(0, 0x4337);
    let bytes = PetBattleSlotUpdates {
        slots: vec![BattlePetJournalSlot {
            pet_guid,
            collar_id: 10,
            index: 2,
            locked: false,
        }],
        auto_slotted: false,
        new_slot: true,
    }
    .to_bytes();

    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::PetBattleSlotUpdates as u16
    );
    let mut body = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(body.read_uint32().unwrap(), 1);
    assert!(body.read_bit().unwrap());
    assert!(!body.read_bit().unwrap());
    assert_eq!(body.read_packed_guid().unwrap(), pet_guid);
    assert_eq!(body.read_uint32().unwrap(), 10);
    assert_eq!(body.read_uint8().unwrap(), 2);
    assert!(!body.read_bit().unwrap());
    assert_eq!(body.remaining(), 0);
}

#[test]
fn battle_pet_set_battle_slot_reads_cpp_shape() {
    let pet_guid = ObjectGuid::new(0, 0x4323);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::BattlePetSetBattleSlot as u16);
    pkt.write_packed_guid(&pet_guid);
    pkt.write_uint8(2);

    let decoded = BattlePetSetBattleSlot::read(&mut pkt).unwrap();
    assert_eq!(decoded, BattlePetSetBattleSlot { pet_guid, slot: 2 });
}

#[test]
fn battle_pet_summon_reads_packed_guid_like_cpp() {
    let pet_guid = ObjectGuid::new(0, 0x4324);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::BattlePetSummon as u16);
    pkt.write_packed_guid(&pet_guid);

    let decoded = BattlePetSummon::read(&mut pkt).unwrap();
    assert_eq!(decoded, BattlePetSummon { pet_guid });
}

#[test]
fn battle_pet_update_notify_reads_packed_guid_like_cpp() {
    let pet_guid = ObjectGuid::new(0, 0x4325);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::BattlePetUpdateNotify as u16);
    pkt.write_packed_guid(&pet_guid);

    let decoded = BattlePetUpdateNotify::read(&mut pkt).unwrap();
    assert_eq!(decoded, BattlePetUpdateNotify { pet_guid });
}

#[test]
fn battle_pet_delete_pet_reads_placeholder_cpp_shape() {
    let pet_guid = ObjectGuid::new(0, 0x4331);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(0xBADD);
    pkt.write_packed_guid(&pet_guid);

    let decoded = BattlePetDeletePet::read_like_cpp(&mut pkt).unwrap();
    assert_eq!(decoded, BattlePetDeletePet { pet_guid });
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn cage_battle_pet_reads_placeholder_cpp_shape() {
    let pet_guid = ObjectGuid::new(0, 0x4334);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(0xBADD);
    pkt.write_packed_guid(&pet_guid);

    let decoded = CageBattlePet::read_like_cpp(&mut pkt).unwrap();
    assert_eq!(decoded, CageBattlePet { pet_guid });
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn battle_pet_modify_name_reads_without_declined_names_like_cpp() {
    let pet_guid = ObjectGuid::new(0, 0x4332);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(0xBADD);
    pkt.write_packed_guid(&pet_guid);
    pkt.write_bits(5, 7);
    pkt.write_bit(false);
    pkt.write_string("Misha");

    let decoded = BattlePetModifyName::read_like_cpp(&mut pkt).unwrap();
    assert_eq!(
        decoded,
        BattlePetModifyName {
            pet_guid,
            name: "Misha".to_string(),
            declined_names: None,
        }
    );
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn battle_pet_modify_name_reads_declined_names_before_name_like_cpp() {
    let pet_guid = ObjectGuid::new(0, 0x4333);
    let declined = ["Mishy", "Mishys", "Mishyu", "Mishy2", "Mishy3"];
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(0xBADD);
    pkt.write_packed_guid(&pet_guid);
    pkt.write_bits(5, 7);
    pkt.write_bit(true);
    for name in declined {
        pkt.write_bits(name.len() as u32, 7);
    }
    for name in declined {
        pkt.write_string(name);
    }
    pkt.write_string("Misha");

    let decoded = BattlePetModifyName::read_like_cpp(&mut pkt).unwrap();
    assert_eq!(decoded.pet_guid, pet_guid);
    assert_eq!(decoded.name, "Misha");
    assert_eq!(
        decoded.declined_names.unwrap().names,
        declined.map(str::to_string)
    );
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn query_battle_pet_name_reads_cpp_shape() {
    let battle_pet_id = ObjectGuid::new(0, 0x4326);
    let unit_guid = ObjectGuid::new(0, 0x4327);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::QueryBattlePetName as u16);
    pkt.write_packed_guid(&battle_pet_id);
    pkt.write_packed_guid(&unit_guid);

    let decoded = QueryBattlePetName::read(&mut pkt).unwrap();
    assert_eq!(
        decoded,
        QueryBattlePetName {
            battle_pet_id,
            unit_guid,
        }
    );
}

#[test]
fn query_battle_pet_name_response_writes_negative_cpp_shape() {
    let battle_pet_id = ObjectGuid::new(0, 0x4328);
    let response = QueryBattlePetNameResponse::not_allowed(battle_pet_id);
    let bytes = response.to_bytes();

    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::QueryBattlePetNameResponse as u16
    );
    let mut body = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(body.read_packed_guid().unwrap(), battle_pet_id);
    assert_eq!(body.read_int32().unwrap(), 0);
    assert_eq!(body.read_int64().unwrap(), 0);
    assert!(!body.read_bit().unwrap());
    assert_eq!(body.remaining(), 0);
}

#[test]
fn query_battle_pet_name_response_writes_positive_without_declined_names_like_cpp() {
    let battle_pet_id = ObjectGuid::new(0, 0x4329);
    let response = QueryBattlePetNameResponse::allowed(
        battle_pet_id,
        91_001,
        1_717_000_123,
        "Rusty".to_string(),
        None,
    );
    let bytes = response.to_bytes();

    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::QueryBattlePetNameResponse as u16
    );
    let mut body = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(body.read_packed_guid().unwrap(), battle_pet_id);
    assert_eq!(body.read_int32().unwrap(), 91_001);
    assert_eq!(body.read_int64().unwrap(), 1_717_000_123);
    assert!(body.read_bit().unwrap());
    assert_eq!(body.read_bits(8).unwrap(), 5);
    assert!(!body.read_bit().unwrap());
    for _ in 0..MAX_DECLINED_NAME_CASES_LIKE_CPP {
        assert_eq!(body.read_bits(7).unwrap(), 0);
    }
    assert_eq!(body.read_string(5).unwrap(), "Rusty");
    assert_eq!(body.remaining(), 0);
}

#[test]
fn query_battle_pet_name_response_writes_positive_with_declined_names_like_cpp() {
    let battle_pet_id = ObjectGuid::new(0, 0x432a);
    let declined = ["Alpha", "Betas", "Gamma", "Delta", "Epsil"].map(str::to_string);
    let response = QueryBattlePetNameResponse::allowed(
        battle_pet_id,
        91_002,
        1_717_000_456,
        "Companion".to_string(),
        Some(DeclinedNamesLikeCpp {
            names: declined.clone(),
        }),
    );
    let bytes = response.to_bytes();

    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::QueryBattlePetNameResponse as u16
    );
    let mut body = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(body.read_packed_guid().unwrap(), battle_pet_id);
    assert_eq!(body.read_int32().unwrap(), 91_002);
    assert_eq!(body.read_int64().unwrap(), 1_717_000_456);
    assert!(body.read_bit().unwrap());
    assert_eq!(body.read_bits(8).unwrap(), 9);
    assert!(body.read_bit().unwrap());
    for name in &declined {
        assert_eq!(body.read_bits(7).unwrap(), name.len() as u32);
    }
    for name in &declined {
        assert_eq!(body.read_string(name.len()).unwrap(), *name);
    }
    assert_eq!(body.read_string(9).unwrap(), "Companion");
    assert_eq!(body.remaining(), 0);
}

#[test]
fn battle_pet_clear_fanfare_reads_packed_guid_like_cpp() {
    let pet_guid = ObjectGuid::new(0, 0x4321);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::BattlePetClearFanfare as u16);
    pkt.write_packed_guid(&pet_guid);

    let decoded = BattlePetClearFanfare::read(&mut pkt).unwrap();
    assert_eq!(decoded.pet_guid, pet_guid);
}

#[test]
fn battle_pet_set_flags_reads_cpp_shape() {
    let pet_guid = ObjectGuid::new(0, 0x4322);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::BattlePetSetFlags as u16);
    pkt.write_packed_guid(&pet_guid);
    pkt.write_uint16(0x12);
    pkt.write_bits(1, 2);
    pkt.flush_bits();

    let decoded = BattlePetSetFlags::read(&mut pkt).unwrap();
    assert_eq!(
        decoded,
        BattlePetSetFlags {
            pet_guid,
            flags: 0x12,
            control_type: 1,
        }
    );
}

#[test]
fn db_reply_not_found() {
    let pkt = DBReply::not_found(0xDF2F53CF, 42);
    let bytes = pkt.to_bytes();
    // opcode(2) + u32(4) + i32(4) + i32(4) + 3 bits flushed(1) + u32(4) = 19
    assert_eq!(bytes.len(), 19);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x290e);
    // table_hash
    let th = u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
    assert_eq!(th, 0xDF2F53CF);
    // record_id
    let rid = i32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]);
    assert_eq!(rid, 42);
    // status byte: 3 bits MSB-first for value 3 = 0b011 → in MSB-first bit layout: 0_1_1_00000 = 0x60
    assert_eq!(bytes[14], 0x60);
    // data size = 0
    let ds = u32::from_le_bytes([bytes[15], bytes[16], bytes[17], bytes[18]]);
    assert_eq!(ds, 0);
}

#[test]
fn db_query_bulk_roundtrip() {
    // Build a DbQueryBulk packet manually with 13-bit count.
    // Use a WorldPacket's bit writer to produce correctly-encoded bits.
    let mut writer = WorldPacket::new_server(ServerOpcodes::DbReply);
    // Overwrite opcode with client opcode (we'll skip it anyway)
    // Just append the payload fields after a dummy 2-byte opcode:
    writer.write_uint32(0xAABBCCDD); // table_hash
    writer.write_bits(3, 13); // count = 3 (13 bits)
    writer.flush_bits();
    writer.write_int32(100);
    writer.write_int32(200);
    writer.write_int32(300);

    // Read it back: from_bytes includes the 2-byte opcode from new_server
    let mut reader = WorldPacket::from_bytes(writer.data());
    reader.skip_opcode(); // skip the 2-byte dummy opcode
    let parsed = DbQueryBulk::read(&mut reader).unwrap();
    assert_eq!(parsed.table_hash, 0xAABBCCDD);
    assert_eq!(parsed.queries, vec![100, 200, 300]);
}

#[test]
fn hotfix_connect_empty() {
    let pkt = HotfixConnect::empty();
    let bytes = pkt.to_bytes();
    // opcode(2) + i32(4) + u32(4) = 10
    assert_eq!(bytes.len(), 10);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x2911);
    // count = 0
    let count = i32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
    assert_eq!(count, 0);
    // content size = 0
    let size = u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]);
    assert_eq!(size, 0);
}

#[test]
fn hotfix_connect_serializes_headers_and_content() {
    let pkt = HotfixConnect {
        hotfixes: vec![HotfixConnectData {
            id: HotfixId {
                push_id: 11,
                unique_id: 12,
            },
            table_hash: 0xDF2F_53CF,
            record_id: 67,
            size: 3,
            status: 1,
        }],
        content: vec![1, 2, 3],
    };
    let bytes = pkt.to_bytes();
    assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x2911);
    assert_eq!(
        u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]),
        1
    );
    assert_eq!(
        i32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]),
        11
    );
    assert_eq!(
        u32::from_le_bytes([bytes[10], bytes[11], bytes[12], bytes[13]]),
        12
    );
    assert_eq!(
        u32::from_le_bytes([bytes[14], bytes[15], bytes[16], bytes[17]]),
        0xDF2F_53CF
    );
    assert_eq!(
        i32::from_le_bytes([bytes[18], bytes[19], bytes[20], bytes[21]]),
        67
    );
    assert_eq!(
        u32::from_le_bytes([bytes[22], bytes[23], bytes[24], bytes[25]]),
        3
    );
    assert_eq!(bytes[26] >> 5, 1);
    assert_eq!(
        u32::from_le_bytes([bytes[27], bytes[28], bytes[29], bytes[30]]),
        3
    );
    assert_eq!(&bytes[31..34], &[1, 2, 3]);
}

#[test]
fn dungeon_difficulty_set_normal() {
    let pkt = DungeonDifficultySet::normal();
    let bytes = pkt.to_bytes();
    // opcode(2) + i32(4) = 6
    assert_eq!(bytes.len(), 6);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x26a4);
    let difficulty = i32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
    assert_eq!(difficulty, 0);
}

#[test]
fn raid_difficulty_set_writes_legacy_flag_like_cpp() {
    let pkt = RaidDifficultySet {
        difficulty_id: 4,
        legacy: true,
    };
    let bytes = pkt.to_bytes();
    // opcode(2) + i32(4) + uint8(1) = 7
    assert_eq!(bytes.len(), 7);
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x27ad);
    let difficulty = i32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
    assert_eq!(difficulty, 4);
    assert_eq!(bytes[6], 1);
}

#[test]
fn move_set_active_mover() {
    let guid = ObjectGuid::create_player(1, 42);
    let pkt = MoveSetActiveMover { mover_guid: guid };
    let bytes = pkt.to_bytes();
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x2dd5);
    // C++ writes `ObjectGuid` directly through operator<<, which is the
    // packed ObjectGuid layout: low/high masks followed by non-zero bytes.
    let mut body = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(body.read_packed_guid().unwrap(), guid);
    assert_eq!(body.remaining(), 0);
}

#[test]
fn set_spell_modifier_flat_empty() {
    let bytes = SetSpellModifier::flat_empty().to_bytes();
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x2c33);
    // opcode(2) + i32(4) = 6
    assert_eq!(bytes.len(), 6);
    let count = i32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
    assert_eq!(count, 0);
}

#[test]
fn set_spell_modifier_pct_empty() {
    let bytes = SetSpellModifier::pct_empty().to_bytes();
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x2c34);
    assert_eq!(bytes.len(), 6);
}

#[test]
fn set_proficiency_weapon() {
    let pkt = SetProficiency::default_weapons(1); // Warrior
    let bytes = pkt.to_bytes();
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x2735);
    // opcode(2) + u32(4) + u8(1) = 7
    assert_eq!(bytes.len(), 7);
    // Class byte = 2 (Weapon)
    assert_eq!(bytes[6], 2);
}

#[test]
fn logout_request_read() {
    let mut writer = WorldPacket::new_server(ServerOpcodes::DbReply); // dummy opcode
    writer.write_bit(true); // idle_logout
    writer.flush_bits();
    let mut reader = WorldPacket::from_bytes(writer.data());
    reader.skip_opcode();
    let req = LogoutRequest::read(&mut reader).unwrap();
    assert!(req.idle_logout);
}

#[test]
fn logout_response_instant_ok() {
    let pkt = LogoutResponse::instant_ok();
    let bytes = pkt.to_bytes();
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x2683);
    // i32(4) + 1 bit flushed(1) = 7 total
    assert_eq!(bytes.len(), 7);
    // result = 0
    let result = i32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
    assert_eq!(result, 0);
    // instant = true → MSB bit set
    assert_eq!(bytes[6], 0x80);
}

#[test]
fn logout_response_delayed_ok() {
    let pkt = LogoutResponse::delayed_ok();
    let bytes = pkt.to_bytes();
    assert_eq!(bytes.len(), 7);
    // instant = false → 0x00
    assert_eq!(bytes[6], 0x00);
}

#[test]
fn logout_complete_empty() {
    let pkt = LogoutComplete;
    let bytes = pkt.to_bytes();
    assert_eq!(bytes.len(), 2); // opcode only
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x2684);
}

#[test]
fn logout_cancel_ack_empty() {
    let pkt = LogoutCancelAck;
    let bytes = pkt.to_bytes();
    assert_eq!(bytes.len(), 2); // opcode only
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(opcode, 0x2685);
}

#[test]
fn buy_failed_serializes_cpp_reason_byte() {
    let pkt = BuyFailed {
        vendor_guid: ObjectGuid::EMPTY,
        muid: 123,
        reason: BuyResult::DistanceTooFar,
    };
    let bytes = pkt.to_bytes();

    assert_eq!(bytes[bytes.len() - 1], BuyResult::DistanceTooFar as u8);
}

#[test]
fn buy_back_item_reads_cpp_guid_and_slot() {
    let vendor_guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 0, 1, 123, 456);
    let mut writer = WorldPacket::new_server(ServerOpcodes::DbReply);
    writer.write_packed_guid(&vendor_guid);
    writer.write_uint32(94);

    let mut reader = WorldPacket::from_bytes(writer.data());
    reader.skip_opcode();
    let pkt = BuyBackItem::read(&mut reader).unwrap();

    assert_eq!(pkt.vendor_guid, vendor_guid);
    assert_eq!(pkt.slot, 94);
}

#[test]
fn repair_item_reads_cpp_guids_and_guild_bank_bit() {
    let npc_guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 0, 1, 123, 456);
    let item_guid = ObjectGuid::create_item(1, 777);
    let mut writer = WorldPacket::new_server(ServerOpcodes::DbReply);
    writer.write_packed_guid(&npc_guid);
    writer.write_packed_guid(&item_guid);
    writer.write_bit(true);
    writer.flush_bits();

    let mut reader = WorldPacket::from_bytes(writer.data());
    reader.skip_opcode();
    let pkt = RepairItem::read(&mut reader).unwrap();

    assert_eq!(pkt.npc_guid, npc_guid);
    assert_eq!(pkt.item_guid, item_guid);
    assert!(pkt.use_guild_bank);
}

#[test]
fn request_stabled_pets_reads_cpp_stable_master_guid() {
    let stable_master =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 571, 0, 345, 678);
    let mut writer = WorldPacket::new_server(ServerOpcodes::DbReply);
    writer.write_packed_guid(&stable_master);

    let mut reader = WorldPacket::from_bytes(writer.data());
    reader.skip_opcode();
    let pkt = RequestStabledPets::read(&mut reader).unwrap();

    assert_eq!(pkt.stable_master, stable_master);
}

#[test]
fn spirit_healer_activate_reads_cpp_healer_guid() {
    let healer =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 571, 0, 9, 1);
    let mut writer = WorldPacket::new_server(ServerOpcodes::DbReply);
    writer.write_packed_guid(&healer);

    let mut reader = WorldPacket::from_bytes(writer.data());
    reader.skip_opcode();
    let pkt = SpiritHealerActivate::read(&mut reader).unwrap();

    assert_eq!(pkt.healer, healer);
}

#[test]
fn area_spirit_healer_query_reads_cpp_healer_guid() {
    let healer =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 571, 0, 9, 2);
    let mut writer = WorldPacket::new_server(ServerOpcodes::DbReply);
    writer.write_packed_guid(&healer);

    let mut reader = WorldPacket::from_bytes(writer.data());
    reader.skip_opcode();
    let pkt = AreaSpiritHealerQuery::read(&mut reader).unwrap();

    assert_eq!(pkt.healer_guid, healer);
}

#[test]
fn area_spirit_healer_queue_reads_cpp_healer_guid() {
    let healer =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 571, 0, 9, 3);
    let mut writer = WorldPacket::new_server(ServerOpcodes::DbReply);
    writer.write_packed_guid(&healer);

    let mut reader = WorldPacket::from_bytes(writer.data());
    reader.skip_opcode();
    let pkt = AreaSpiritHealerQueue::read(&mut reader).unwrap();

    assert_eq!(pkt.healer_guid, healer);
}

#[test]
fn area_spirit_healer_time_writes_cpp_guid_and_time_left() {
    let healer =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 571, 0, 9, 4);
    let packet = AreaSpiritHealerTime {
        healer_guid: healer,
        time_left_ms: 12_345,
    };

    let mut bytes = (ServerOpcodes::AreaSpiritHealerTime as u16)
        .to_le_bytes()
        .to_vec();
    let mut payload = WorldPacket::new_empty();
    payload.write_packed_guid(&healer);
    payload.write_int32(12_345);
    bytes.extend_from_slice(payload.data());

    assert_eq!(packet.to_bytes(), bytes);
}

#[test]
fn hearth_and_resurrect_reads_empty_cpp_packet() {
    let mut pkt = WorldPacket::new_empty();

    HearthAndResurrect::read(&mut pkt).unwrap();
}

#[test]
fn resurrect_response_reads_guid_and_response_like_cpp() {
    let resurrecter = ObjectGuid::create_player(1, 77);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&resurrecter);
    pkt.write_uint32(1);

    let parsed = ResurrectResponse::read(&mut pkt).unwrap();

    assert_eq!(parsed.resurrecter, resurrecter);
    assert_eq!(parsed.response, 1);
}

#[test]
fn battlefield_leave_reads_empty_cpp_packet() {
    let mut pkt = WorldPacket::new_empty();

    BattlefieldLeave::read(&mut pkt).unwrap();
}

#[test]
fn battlefield_port_reads_ticket_and_accepted_bit_like_cpp() {
    let requester = ObjectGuid::create_player(1, 42);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&requester);
    pkt.write_uint32(1);
    pkt.write_uint32(2);
    pkt.write_int64(1_234_567);
    pkt.write_bit(true);
    pkt.flush_bits();
    pkt.write_bit(true);
    pkt.flush_bits();
    pkt.reset_read();

    let parsed = BattlefieldPort::read(&mut pkt).unwrap();

    assert_eq!(parsed.ticket.requester_guid, requester);
    assert_eq!(parsed.ticket.id, 1);
    assert_eq!(parsed.ticket.ride_type, 2);
    assert_eq!(parsed.ticket.time, 1_234_567);
    assert!(parsed.ticket.unknown925);
    assert!(parsed.accepted_invite);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn battlefield_list_request_reads_list_id_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(3);
    pkt.reset_read();

    let parsed = BattlefieldListRequest::read(&mut pkt).unwrap();

    assert_eq!(parsed.list_id, 3);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn battlemaster_join_reads_queue_roles_blacklist_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(2);
    pkt.write_uint8(0x07);
    pkt.write_int32(10);
    pkt.write_int32(-1);
    pkt.write_uint64(0x1F10_0000_0000_0003);
    pkt.write_uint64(0x1F10_0000_0001_0003);
    pkt.reset_read();

    let parsed = BattlemasterJoin::read(&mut pkt).unwrap();

    assert_eq!(
        parsed.queue_ids,
        [0x1F10_0000_0000_0003, 0x1F10_0000_0001_0003]
    );
    assert_eq!(parsed.roles, 0x07);
    assert_eq!(parsed.blacklist_map, [10, -1]);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn battlemaster_join_arena_reads_team_size_index_and_roles_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint8(1);
    pkt.write_uint8(0x07);
    pkt.reset_read();

    let parsed = BattlemasterJoinArena::read(&mut pkt).unwrap();

    assert_eq!(parsed.team_size_index, 1);
    assert_eq!(parsed.roles, 0x07);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn battlemaster_join_skirmish_reads_ids_group_and_rated_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(5);
    pkt.write_uint32(3);
    pkt.write_uint8(1);
    pkt.write_uint8(0);
    pkt.reset_read();

    let parsed = BattlemasterJoinSkirmish::read(&mut pkt).unwrap();

    assert_eq!(parsed.bg_type_id, 5);
    assert_eq!(parsed.bracket_id, 3);
    assert_eq!(parsed.as_group, 1);
    assert_eq!(parsed.is_rated, 0);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn accept_wargame_invite_reads_cstring_inviter_name_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_string("Inviter");
    pkt.write_uint8(0);
    pkt.reset_read();

    let parsed = AcceptWargameInvite::read(&mut pkt).unwrap();

    assert_eq!(parsed.inviter_name, "Inviter");
}

#[test]
fn buy_item_resets_bitpos_between_item_bonus_and_mod_list_like_cpp() {
    let vendor_guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 0, 1, 123, 456);
    let container_guid = ObjectGuid::create_player(1, 42);
    let mut writer = WorldPacket::new_server(ServerOpcodes::DbReply);
    writer.write_packed_guid(&vendor_guid);
    writer.write_packed_guid(&container_guid);
    writer.write_int32(2);
    writer.write_int32(7);
    writer.write_int32(3);
    writer.write_int32(1);
    writer.write_int32(700);
    writer.write_int32(11);
    writer.write_int32(-22);
    writer.write_bit(false);
    writer.flush_bits();
    writer.write_bits(1, 6);
    writer.flush_bits();
    writer.write_int32(1234);
    writer.write_uint8(5);

    let mut reader = WorldPacket::from_bytes(writer.data());
    reader.skip_opcode();
    let pkt = BuyItem::read(&mut reader).unwrap();

    assert_eq!(pkt.vendor_guid, vendor_guid);
    assert_eq!(pkt.container_guid, container_guid);
    assert_eq!(pkt.quantity, 2);
    assert_eq!(pkt.muid, 7);
    assert_eq!(pkt.slot, 3);
    assert_eq!(pkt.item_type, 1);
    assert_eq!(pkt.item_id, 700);
}

#[test]
fn sell_response_serializes_cpp_count_and_reason_before_item_guids() {
    let pkt = SellResponse {
        vendor_guid: ObjectGuid::EMPTY,
        item_guids: Vec::new(),
        reason: SellResult::CantSellItem as i32,
    };
    let bytes = pkt.to_bytes();

    assert_eq!(
        &bytes[bytes.len() - 8..bytes.len() - 4],
        &0u32.to_le_bytes()
    );
    assert_eq!(
        &bytes[bytes.len() - 4..],
        &(SellResult::CantSellItem as i32).to_le_bytes()
    );

    let error = SellResponse::error(
        ObjectGuid::EMPTY,
        ObjectGuid::EMPTY,
        SellResult::YouDontOwnThatItem,
    );
    assert_eq!(error.item_guids.len(), 1);
    assert_eq!(error.reason, SellResult::YouDontOwnThatItem as i32);
}

#[test]
fn set_proficiency_armor() {
    let pkt = SetProficiency::default_armor(1); // Warrior
    let bytes = pkt.to_bytes();
    assert_eq!(bytes.len(), 7);
    // Class byte = 4 (Armor)
    assert_eq!(bytes[6], 4);
    // Mask = 0x5E for warrior (Cloth+Leather+Mail+Plate+Shield)
    let mask = u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
    assert_eq!(mask, 0x5E);
}

#[test]
fn fish_not_hooked_is_empty_server_packet_like_cpp() {
    let bytes = FishNotHooked.to_bytes();
    assert_eq!(bytes, (ServerOpcodes::FishNotHooked as u16).to_le_bytes());
}

#[test]
fn enable_barber_shop_writes_customization_scope_like_cpp() {
    let bytes = EnableBarberShop {
        customization_scope: 7,
    }
    .to_bytes();
    assert_eq!(
        bytes[0..2],
        (ServerOpcodes::EnableBarberShop as u16).to_le_bytes()
    );
    assert_eq!(bytes[2], 7);
    assert_eq!(bytes.len(), 3);
}

#[test]
fn gameobject_interaction_writes_raw_guid_and_interaction_type_like_cpp() {
    let guid = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::GameObject,
        0,
        1,
        571,
        0,
        777,
        23,
    );
    let bytes = GameObjectInteraction {
        object_guid: guid,
        interaction_type: 40,
    }
    .to_bytes();
    assert_eq!(
        bytes[0..2],
        (ServerOpcodes::GameObjectInteraction as u16).to_le_bytes()
    );
    assert_eq!(&bytes[2..18], &guid.to_raw_bytes());
    assert_eq!(&bytes[18..22], &40_i32.to_le_bytes());
    assert_eq!(bytes.len(), 22);
}

#[test]
fn gameobject_custom_anim_writes_guid_anim_and_despawn_bit_like_cpp() {
    let guid = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::GameObject,
        0,
        1,
        571,
        0,
        777,
        23,
    );
    let bytes = GameObjectCustomAnim {
        object_guid: guid,
        custom_anim: 255,
        play_as_despawn: false,
    }
    .to_bytes();
    assert_eq!(
        bytes[0..2],
        (ServerOpcodes::GameObjectCustomAnim as u16).to_le_bytes()
    );
    assert_eq!(&bytes[2..18], &guid.to_raw_bytes());
    assert_eq!(&bytes[18..22], &255_u32.to_le_bytes());
    assert_eq!(bytes[22], 0x00);
    assert_eq!(bytes.len(), 23);

    let despawn_bytes = GameObjectCustomAnim {
        object_guid: guid,
        custom_anim: 7,
        play_as_despawn: true,
    }
    .to_bytes();
    assert_eq!(despawn_bytes[22], 0x80);
}

#[test]
fn gameobject_despawn_writes_raw_guid_like_cpp() {
    let guid = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::GameObject,
        0,
        1,
        571,
        0,
        777,
        23,
    );
    let bytes = GameObjectDespawn { object_guid: guid }.to_bytes();
    assert_eq!(
        bytes[0..2],
        (ServerOpcodes::GameObjectDespawn as u16).to_le_bytes()
    );
    assert_eq!(&bytes[2..18], &guid.to_raw_bytes());
    assert_eq!(bytes.len(), 18);
}

#[test]
fn capture_point_removed_writes_only_raw_guid_like_cpp() {
    let guid = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::GameObject,
        0,
        1,
        571,
        0,
        777,
        24,
    );
    let bytes = CapturePointRemoved {
        capture_point_guid: guid,
    }
    .to_bytes();
    assert_eq!(
        bytes[0..2],
        (ServerOpcodes::UpdateCapturePoint as u16).to_le_bytes()
    );
    assert_eq!(&bytes[2..18], &guid.to_raw_bytes());
    assert_eq!(bytes.len(), 18);
}

#[test]
fn gameobject_set_state_local_writes_raw_guid_and_state_like_cpp() {
    let guid = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::GameObject,
        0,
        1,
        571,
        0,
        777,
        23,
    );
    let bytes = GameObjectSetStateLocal {
        object_guid: guid,
        state: 2,
    }
    .to_bytes();
    assert_eq!(
        bytes[0..2],
        (ServerOpcodes::GameObjectSetStateLocal as u16).to_le_bytes()
    );
    assert_eq!(&bytes[2..18], &guid.to_raw_bytes());
    assert_eq!(bytes[18], 2);
    assert_eq!(bytes.len(), 19);
}

#[test]
fn update_world_state_writes_visible_default_false_layout_like_cpp() {
    let bytes = UpdateWorldState::new(0x1234_5678, 42).to_bytes();

    assert_eq!(
        bytes[0..2],
        (ServerOpcodes::UpdateWorldState as u16).to_le_bytes()
    );
    assert_eq!(&bytes[2..6], &0x1234_5678_u32.to_le_bytes());
    assert_eq!(&bytes[6..10], &42_i32.to_le_bytes());
    assert_eq!(bytes[10], 0x00);
    assert_eq!(bytes.len(), 11);
}

#[test]
fn update_world_state_writes_hidden_true_bit_like_cpp() {
    let bytes = UpdateWorldState {
        variable_id: 9001,
        value: -7,
        hidden: true,
    }
    .to_bytes();

    assert_eq!(
        bytes[0..2],
        (ServerOpcodes::UpdateWorldState as u16).to_le_bytes()
    );
    assert_eq!(&bytes[2..6], &9001_u32.to_le_bytes());
    assert_eq!(&bytes[6..10], &(-7_i32).to_le_bytes());
    assert_eq!(bytes[10], 0x80);
    assert_eq!(bytes.len(), 11);
}

#[test]
fn update_capture_point_writes_cpp_capture_point_info() {
    let guid = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::GameObject,
        0,
        1,
        571,
        0,
        777,
        24,
    );
    let bytes = UpdateCapturePoint {
        guid,
        position: Position::new(12.5, 34.25, 56.0, 1.0),
        state: 2,
        capture_time_ms: 15_000,
        capture_total_duration_ms: 60_000,
    }
    .to_bytes();
    assert_eq!(
        bytes[0..2],
        (ServerOpcodes::UpdateCapturePoint as u16).to_le_bytes()
    );
    assert_eq!(&bytes[2..18], &guid.to_raw_bytes());
    assert_eq!(&bytes[18..22], &12.5_f32.to_le_bytes());
    assert_eq!(&bytes[22..26], &34.25_f32.to_le_bytes());
    assert_eq!(bytes[26], 2);
    assert_eq!(&bytes[27..31], &15_000_u32.to_le_bytes());
    assert_eq!(&bytes[31..35], &60_000_u32.to_le_bytes());
    assert_eq!(bytes.len(), 35);

    let captured_bytes = UpdateCapturePoint {
        guid,
        position: Position::new(12.5, 34.25, 56.0, 1.0),
        state: 4,
        capture_time_ms: 0,
        capture_total_duration_ms: 60_000,
    }
    .to_bytes();
    assert_eq!(captured_bytes[26], 4);
    assert_eq!(captured_bytes.len(), 27);
}

#[test]
fn page_text_writes_gameobject_guid_like_cpp() {
    let guid = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::GameObject,
        0,
        1,
        571,
        0,
        777,
        23,
    );
    let bytes = PageText {
        gameobject_guid: guid,
    }
    .to_bytes();
    assert_eq!(bytes[0..2], (ServerOpcodes::PageText as u16).to_le_bytes());
    assert_eq!(&bytes[2..18], &guid.to_raw_bytes());
    assert_eq!(bytes.len(), 18);
}

#[test]
fn anim_kit_packets_write_unit_guid_and_anim_kit_id_like_cpp() {
    let guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 571, 0, 1234, 99);

    for (bytes, opcode, anim_kit_id) in [
        (
            SetAiAnimKit {
                unit: guid,
                anim_kit_id: 11,
            }
            .to_bytes(),
            ServerOpcodes::SetAiAnimKit,
            11_u16,
        ),
        (
            SetMovementAnimKit {
                unit: guid,
                anim_kit_id: 22,
            }
            .to_bytes(),
            ServerOpcodes::SetMovementAnimKit,
            22_u16,
        ),
        (
            SetMeleeAnimKit {
                unit: guid,
                anim_kit_id: 33,
            }
            .to_bytes(),
            ServerOpcodes::SetMeleeAnimKit,
            33_u16,
        ),
    ] {
        assert_eq!(bytes[0..2], (opcode as u16).to_le_bytes());
        assert_eq!(&bytes[2..18], &guid.to_raw_bytes());
        assert_eq!(&bytes[18..20], &anim_kit_id.to_le_bytes());
        assert_eq!(bytes.len(), 20);
    }
}

#[test]
fn trigger_cinematic_writes_id_and_conversation_guid_like_cpp() {
    let bytes = TriggerCinematic {
        cinematic_id: 444,
        conversation_guid: ObjectGuid::EMPTY,
    }
    .to_bytes();
    assert_eq!(
        bytes[0..2],
        (ServerOpcodes::TriggerCinematic as u16).to_le_bytes()
    );
    assert_eq!(&bytes[2..6], &444_u32.to_le_bytes());
    assert_eq!(&bytes[6..22], &ObjectGuid::EMPTY.to_raw_bytes());
    assert_eq!(bytes.len(), 22);
}

#[test]
fn trigger_movie_writes_movie_id_like_cpp() {
    let bytes = TriggerMovie { movie_id: 7788 }.to_bytes();
    assert_eq!(
        bytes[0..2],
        (ServerOpcodes::TriggerMovie as u16).to_le_bytes()
    );
    assert_eq!(&bytes[2..6], &7788_u32.to_le_bytes());
    assert_eq!(bytes.len(), 6);
}

#[test]
fn far_sight_reads_enable_bit_true_and_false_like_cpp() {
    for enable in [false, true] {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(enable);
        pkt.flush_bits();
        pkt.reset_read();

        let far_sight = FarSight::read(&mut pkt).unwrap();
        assert_eq!(far_sight.enable, enable);
    }
}

#[test]
fn buy_bank_slot_reads_full_guid_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 12, 34);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&guid);
    pkt.reset_read();

    let buy = BuyBankSlot::read(&mut pkt).unwrap();
    assert_eq!(buy.guid, guid);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn change_bank_bag_slot_flag_reads_slot_flag_and_enabled_bit_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(3);
    pkt.write_uint32(5);
    pkt.write_bit(true);
    pkt.flush_bits();
    pkt.reset_read();

    let change = ChangeBankBagSlotFlag::read(&mut pkt).unwrap();
    assert_eq!(change.slot, 3);
    assert_eq!(change.flag, 5);
    assert!(change.enabled);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn auto_bank_item_reads_343_inv_bag_slot_without_retail_bank_type_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bits(1, 2);
    pkt.flush_bits();
    pkt.write_uint8(255);
    pkt.write_uint8(19);
    pkt.write_uint8(255);
    pkt.write_uint8(19);
    pkt.reset_read();

    let packet = AutoBankItem::read(&mut pkt).unwrap();
    assert_eq!(packet.inv_update.items, vec![(255, 19)]);
    assert_eq!(packet.bag, 255);
    assert_eq!(packet.slot, 19);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn auto_store_bank_item_reads_inv_bag_slot_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bits(1, 2);
    pkt.flush_bits();
    pkt.write_uint8(255);
    pkt.write_uint8(39);
    pkt.write_uint8(255);
    pkt.write_uint8(39);
    pkt.reset_read();

    let packet = AutoStoreBankItem::read(&mut pkt).unwrap();
    assert_eq!(packet.inv_update.items, vec![(255, 39)]);
    assert_eq!(packet.bag, 255);
    assert_eq!(packet.slot, 39);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn guild_bank_activate_reads_guid_then_full_update_bit_like_cpp() {
    let banker = ObjectGuid::new(0x0102_0304_0506_0708_i64, 0x1112_1314_1516_1718_i64);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&banker);
    pkt.write_bit(true);
    pkt.flush_bits();
    pkt.reset_read();

    let parsed = GuildBankActivate::read(&mut pkt).unwrap();

    assert_eq!(parsed.banker, banker);
    assert!(parsed.full_update);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn guild_bank_query_tab_reads_guid_tab_then_full_update_bit_like_cpp() {
    let banker = ObjectGuid::new(0x2122_2324_2526_2728_i64, 0x3132_3334_3536_3738_i64);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&banker);
    pkt.write_uint8(4);
    pkt.write_bit(false);
    pkt.flush_bits();
    pkt.reset_read();

    let parsed = GuildBankQueryTab::read(&mut pkt).unwrap();

    assert_eq!(parsed.banker, banker);
    assert_eq!(parsed.tab, 4);
    assert!(!parsed.full_update);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn guild_bank_buy_tab_reads_guid_then_tab_like_cpp() {
    let banker = ObjectGuid::new(0x8182_8384_8586_8788_u64 as i64, 0x1112_1314_1516_1718_i64);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&banker);
    pkt.write_uint8(5);
    pkt.reset_read();

    let parsed = GuildBankBuyTab::read(&mut pkt).unwrap();

    assert_eq!(parsed.banker, banker);
    assert_eq!(parsed.bank_tab, 5);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn guild_bank_update_tab_reads_guid_tab_name_icon_like_cpp() {
    let banker = ObjectGuid::new(0x9192_9394_9596_9798_u64 as i64, 0x2122_2324_2526_2728_i64);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&banker);
    pkt.write_uint8(2);
    pkt.write_bits(4, 7);
    pkt.write_bits(7, 9);
    pkt.flush_bits();
    pkt.write_string("Main");
    pkt.write_string("inv_tab");
    pkt.reset_read();

    let parsed = GuildBankUpdateTab::read(&mut pkt).unwrap();

    assert_eq!(parsed.banker, banker);
    assert_eq!(parsed.bank_tab, 2);
    assert_eq!(parsed.name, "Main");
    assert_eq!(parsed.icon, "inv_tab");
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn guild_bank_deposit_money_reads_guid_then_money_like_cpp() {
    let banker = ObjectGuid::new(0x4142_4344_4546_4748_i64, 0x5152_5354_5556_5758_i64);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&banker);
    pkt.write_uint64(123_456);
    pkt.reset_read();

    let parsed = GuildBankDepositMoney::read(&mut pkt).unwrap();

    assert_eq!(parsed.banker, banker);
    assert_eq!(parsed.money, 123_456);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn guild_bank_withdraw_money_reads_guid_then_money_like_cpp() {
    let banker = ObjectGuid::new(0x6162_6364_6566_6768_i64, 0x7172_7374_7576_7778_i64);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&banker);
    pkt.write_uint64(654_321);
    pkt.reset_read();

    let parsed = GuildBankWithdrawMoney::read(&mut pkt).unwrap();

    assert_eq!(parsed.banker, banker);
    assert_eq!(parsed.money, 654_321);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn guild_bank_log_and_text_queries_read_tab_like_cpp() {
    let mut log = WorldPacket::new_empty();
    log.write_int32(7);
    log.reset_read();
    let parsed_log = GuildBankLogQuery::read(&mut log).unwrap();
    assert_eq!(parsed_log.tab, 7);
    assert_eq!(log.remaining(), 0);

    let mut text = WorldPacket::new_empty();
    text.write_int32(3);
    text.reset_read();
    let parsed_text = GuildBankTextQuery::read(&mut text).unwrap();
    assert_eq!(parsed_text.tab, 3);
    assert_eq!(text.remaining(), 0);
}

#[test]
fn guild_bank_set_tab_text_reads_tab_length_and_text_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(4);
    pkt.write_bits(11, 14);
    pkt.flush_bits();
    pkt.write_string("raid mats 1");
    pkt.reset_read();

    let parsed = GuildBankSetTabText::read(&mut pkt).unwrap();

    assert_eq!(parsed.tab, 4);
    assert_eq!(parsed.tab_text, "raid mats 1");
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn guild_command_result_player_not_in_guild_view_tab_matches_cpp_shape() {
    let bytes = GuildCommandResult::player_not_in_guild_view_tab_like_cpp().to_bytes();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::GuildCommandResult as u16
    );
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);

    assert_eq!(
        pkt.read_int32().unwrap(),
        GuildCommandResult::ERR_PLAYER_NOT_IN_GUILD_LIKE_CPP
    );
    assert_eq!(
        pkt.read_int32().unwrap(),
        GuildCommandResult::COMMAND_VIEW_TAB_LIKE_CPP
    );
    assert_eq!(pkt.read_bits(8).unwrap(), 0);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn auto_guild_bank_item_reads_cpp_field_order_with_optional_container_slot() {
    let banker = ObjectGuid::new(0x0102_0304_0506_0708_i64, 0x1112_1314_1516_1718_i64);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&banker);
    pkt.write_uint8(2);
    pkt.write_uint8(14);
    pkt.write_uint8(22);
    pkt.write_bit(true);
    pkt.flush_bits();
    pkt.write_uint8(5);
    pkt.reset_read();

    let parsed = AutoGuildBankItem::read(&mut pkt).unwrap();

    assert_eq!(parsed.banker, banker);
    assert_eq!(parsed.bank_tab, 2);
    assert_eq!(parsed.bank_slot, 14);
    assert_eq!(parsed.container_item_slot, 22);
    assert_eq!(parsed.container_slot, Some(5));
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn auto_store_guild_bank_item_reads_cpp_field_order() {
    let banker = ObjectGuid::new(0x2122_2324_2526_2728_i64, 0x3132_3334_3536_3738_i64);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&banker);
    pkt.write_uint8(3);
    pkt.write_uint8(19);
    pkt.reset_read();

    let parsed = AutoStoreGuildBankItem::read(&mut pkt).unwrap();

    assert_eq!(parsed.banker, banker);
    assert_eq!(parsed.bank_tab, 3);
    assert_eq!(parsed.bank_slot, 19);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn bug_report_reads_type_diag_and_text_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(true);
    pkt.write_bits(4, 12);
    pkt.write_bits(3, 10);
    pkt.flush_bits();
    pkt.write_string("diag");
    pkt.write_string("bug");
    pkt.reset_read();

    let report = BugReport::read(&mut pkt).unwrap();
    assert_eq!(report.report_type, 1);
    assert_eq!(report.diag_info, "diag");
    assert_eq!(report.text, "bug");
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn object_update_recovery_reads_guid_like_cpp() {
    let guid = ObjectGuid::create_player(1, 42);
    let mut failed = WorldPacket::new_empty();
    failed.write_packed_guid(&guid);
    failed.reset_read();
    assert_eq!(
        ObjectUpdateFailed::read(&mut failed).unwrap(),
        ObjectUpdateFailed { object_guid: guid }
    );

    let rescued_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 7, 9);
    let mut rescued = WorldPacket::new_empty();
    rescued.write_packed_guid(&rescued_guid);
    rescued.reset_read();
    assert_eq!(
        ObjectUpdateRescued::read(&mut rescued).unwrap(),
        ObjectUpdateRescued {
            object_guid: rescued_guid
        }
    );
}

#[test]
fn stand_state_change_reads_raw_uint32_like_cpp() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(8);
    pkt.reset_read();

    assert_eq!(
        StandStateChange::read(&mut pkt).unwrap(),
        StandStateChange { stand_state: 8 }
    );
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn stand_state_update_writes_anim_kit_then_state_like_cpp() {
    let bytes = StandStateUpdate {
        anim_kit_id: 0,
        stand_state: 1,
    }
    .to_bytes();
    let mut pkt = WorldPacket::from_bytes(&bytes);

    assert_eq!(pkt.server_opcode(), Some(ServerOpcodes::StandStateUpdate));
    assert_eq!(
        pkt.read_uint16().unwrap(),
        ServerOpcodes::StandStateUpdate as u16
    );
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    assert_eq!(pkt.read_uint8().unwrap(), 1);
    assert_eq!(pkt.remaining(), 0);
    assert_eq!(&bytes[2..], &[0, 0, 0, 0, 1]);
}
