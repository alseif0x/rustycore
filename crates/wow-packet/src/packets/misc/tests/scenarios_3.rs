//! Miscellaneous packet regressions, part 3 of 6.
//!
//! Moved out of the tests.rs root under #640; every test is unchanged.

use super::*;

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
