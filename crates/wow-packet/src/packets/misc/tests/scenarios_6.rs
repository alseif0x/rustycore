//! Miscellaneous packet regressions, part 6 of 6.
//!
//! Moved out of the tests.rs root under #640; every test is unchanged.

use super::*;

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
