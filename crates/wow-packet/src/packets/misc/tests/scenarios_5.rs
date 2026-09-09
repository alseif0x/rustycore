//! Miscellaneous packet regressions, part 5 of 6.
//!
//! Moved out of the tests.rs root under #640; every test is unchanged.

use super::*;

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
