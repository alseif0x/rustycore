// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! auction capability handler tests.

use super::*;
use crate::session::{
    RepresentedAuctionPlaceBidLikeCpp, RepresentedAuctionRemoveItemLikeCpp,
    RepresentedAuctionReplicateRequestLikeCpp, RepresentedAuctionSellItemLikeCpp,
};
use wow_constants::unit::NPCFlags1;
use wow_packet::packets::misc::{
    AuctionPlaceBid, AuctionRemoveItem, AuctionReplicateItems, AuctionSellItem,
};

#[tokio::test]
async fn commerce_token_get_log_echoes_request_and_empty_success_like_cpp_todo_handler() {
    let (mut session, send_rx) = make_session();
    let mut request = WorldPacket::new_empty();
    request.write_uint32(0x1122_3344);

    session.handle_commerce_token_get_log(request).await;

    let bytes = send_rx.try_recv().expect("commerce token get log response");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::CommerceTokenGetLogResponse as u16
    );

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), 0x1122_3344);
    assert_eq!(
        pkt.read_uint32().unwrap(),
        wow_packet::packets::misc::TOKEN_RESULT_SUCCESS_LIKE_CPP
    );
    assert_eq!(pkt.read_uint32().unwrap(), 0);
}

#[tokio::test]
async fn auctionable_token_sell_is_silent_wotlk_stub_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_auctionable_token_sell(WorldPacket::new_empty())
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn auction_list_items_is_silent_legacy_stub_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_auction_list_items(wow_packet::packets::misc::AuctionListItems)
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn auction_place_bid_records_request_after_auctioneer_gate_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 42);
    let auctioneer = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 90_002, 79);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        player_guid,
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        0,
    );
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
    add_canonical_auctioneer_for_misc_test(
        &canonical,
        auctioneer,
        Position::new(12.0, 0.0, 0.0, 0.0),
        NPCFlags1::AUCTIONEER.bits(),
    );

    session
        .handle_auction_place_bid(AuctionPlaceBid {
            auctioneer,
            auction_id: 1234,
            bid_amount: 12_300,
            tainted_by: Some(wow_packet::packets::misc::AuctionAddonInfo {
                name: "Trade".to_string(),
                version: "1.0".to_string(),
                loaded: true,
                disabled: false,
            }),
        })
        .await;

    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        session.represented_auction_place_bids_like_cpp(),
        &[RepresentedAuctionPlaceBidLikeCpp {
            auctioneer,
            auction_id: 1234,
            bid_amount: 12_300,
            tainted_by_present: true,
            copper_rejected: false,
        }]
    );
}

#[tokio::test]
async fn auction_place_bid_rejects_missing_auctioneer_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 42);
    let missing_auctioneer =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 90_002, 80);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        player_guid,
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        0,
    );
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());

    session
        .handle_auction_place_bid(AuctionPlaceBid {
            auctioneer: missing_auctioneer,
            auction_id: 1234,
            bid_amount: 12_300,
            tainted_by: None,
        })
        .await;

    assert!(send_rx.try_recv().is_err());
    assert!(session.represented_auction_place_bids_like_cpp().is_empty());
}

#[tokio::test]
async fn auction_remove_item_records_request_after_auctioneer_gate_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 42);
    let auctioneer = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 90_002, 82);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        player_guid,
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        0,
    );
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
    add_canonical_auctioneer_for_misc_test(
        &canonical,
        auctioneer,
        Position::new(12.0, 0.0, 0.0, 0.0),
        NPCFlags1::AUCTIONEER.bits(),
    );

    session
        .handle_auction_remove_item(AuctionRemoveItem {
            auctioneer,
            auction_id: 1234,
            item_id: 19019,
            tainted_by: Some(wow_packet::packets::misc::AuctionAddonInfo {
                name: "Trade".to_string(),
                version: "1.0".to_string(),
                loaded: true,
                disabled: false,
            }),
        })
        .await;

    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        session.represented_auction_remove_items_like_cpp(),
        &[RepresentedAuctionRemoveItemLikeCpp {
            auctioneer,
            auction_id: 1234,
            item_id: 19019,
            tainted_by_present: true,
        }]
    );
}

#[tokio::test]
async fn auction_remove_item_rejects_missing_auctioneer_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 42);
    let missing_auctioneer =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 90_002, 83);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        player_guid,
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        0,
    );
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());

    session
        .handle_auction_remove_item(AuctionRemoveItem {
            auctioneer: missing_auctioneer,
            auction_id: 1234,
            item_id: 19019,
            tainted_by: None,
        })
        .await;

    assert!(send_rx.try_recv().is_err());
    assert!(
        session
            .represented_auction_remove_items_like_cpp()
            .is_empty()
    );
}

#[tokio::test]
async fn auction_sell_item_records_request_after_available_gates_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 42);
    let auctioneer = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 90_002, 84);
    let item_guid = ObjectGuid::create_item(1, 19_019);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        player_guid,
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        0,
    );
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
    add_canonical_auctioneer_for_misc_test(
        &canonical,
        auctioneer,
        Position::new(12.0, 0.0, 0.0, 0.0),
        NPCFlags1::AUCTIONEER.bits(),
    );

    session
        .handle_auction_sell_item(AuctionSellItem {
            auctioneer,
            min_bid: 10_000,
            buyout_price: 25_000,
            runtime: 720,
            tainted_by: Some(wow_packet::packets::misc::AuctionAddonInfo {
                name: "Trade".to_string(),
                version: "1.0".to_string(),
                loaded: true,
                disabled: false,
            }),
            items: vec![wow_packet::packets::misc::AuctionItemForSale {
                guid: item_guid,
                use_count: 1,
            }],
        })
        .await;

    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        session.represented_auction_sell_items_like_cpp(),
        &[RepresentedAuctionSellItemLikeCpp {
            auctioneer,
            item_guid: Some(item_guid),
            item_use_count: Some(1),
            min_bid: 10_000,
            buyout_price: 25_000,
            runtime_minutes: 720,
            tainted_by_present: true,
            item_list_rejected: false,
            use_count_rejected: false,
            no_price_rejected: false,
            max_money_rejected: false,
            copper_rejected: false,
            auctioneer_accepted: true,
            runtime_rejected: false,
        }]
    );
}

#[tokio::test]
async fn auction_sell_item_records_pre_auctioneer_packet_rejections_like_cpp() {
    let (mut session, send_rx) = make_session();
    let missing_auctioneer =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 90_002, 85);

    session
        .handle_auction_sell_item(AuctionSellItem {
            auctioneer: missing_auctioneer,
            min_bid: 10_001,
            buyout_price: 0,
            runtime: 720,
            tainted_by: None,
            items: vec![],
        })
        .await;

    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        session.represented_auction_sell_items_like_cpp(),
        &[RepresentedAuctionSellItemLikeCpp {
            auctioneer: missing_auctioneer,
            item_guid: None,
            item_use_count: None,
            min_bid: 10_001,
            buyout_price: 0,
            runtime_minutes: 720,
            tainted_by_present: false,
            item_list_rejected: true,
            use_count_rejected: false,
            no_price_rejected: false,
            max_money_rejected: false,
            copper_rejected: true,
            auctioneer_accepted: false,
            runtime_rejected: false,
        }]
    );
}

#[tokio::test]
async fn auction_sell_item_marks_invalid_runtime_after_auctioneer_gate_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 42);
    let auctioneer = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 90_002, 86);
    let item_guid = ObjectGuid::create_item(1, 19_020);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        player_guid,
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        0,
    );
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
    add_canonical_auctioneer_for_misc_test(
        &canonical,
        auctioneer,
        Position::new(12.0, 0.0, 0.0, 0.0),
        NPCFlags1::AUCTIONEER.bits(),
    );

    session
        .handle_auction_sell_item(AuctionSellItem {
            auctioneer,
            min_bid: 10_000,
            buyout_price: 0,
            runtime: 1,
            tainted_by: None,
            items: vec![wow_packet::packets::misc::AuctionItemForSale {
                guid: item_guid,
                use_count: 1,
            }],
        })
        .await;

    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        session.represented_auction_sell_items_like_cpp(),
        &[RepresentedAuctionSellItemLikeCpp {
            auctioneer,
            item_guid: Some(item_guid),
            item_use_count: Some(1),
            min_bid: 10_000,
            buyout_price: 0,
            runtime_minutes: 1,
            tainted_by_present: false,
            item_list_rejected: false,
            use_count_rejected: false,
            no_price_rejected: false,
            max_money_rejected: false,
            copper_rejected: false,
            auctioneer_accepted: true,
            runtime_rejected: true,
        }]
    );
}

#[tokio::test]
async fn auction_replicate_items_records_request_after_auctioneer_gate_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 42);
    let auctioneer = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 90_002, 77);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        player_guid,
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        0,
    );
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
    add_canonical_auctioneer_for_misc_test(
        &canonical,
        auctioneer,
        Position::new(12.0, 0.0, 0.0, 0.0),
        NPCFlags1::AUCTIONEER.bits(),
    );

    session
        .handle_auction_replicate_items(AuctionReplicateItems {
            auctioneer,
            change_number_global: 11,
            change_number_cursor: 22,
            change_number_tombstone: 33,
            count: 44,
            tainted_by: Some(wow_packet::packets::misc::AuctionAddonInfo {
                name: "Trade".to_string(),
                version: "1.0".to_string(),
                loaded: true,
                disabled: false,
            }),
        })
        .await;

    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        session.represented_auction_replicate_requests_like_cpp(),
        &[RepresentedAuctionReplicateRequestLikeCpp {
            auctioneer,
            change_number_global: 11,
            change_number_cursor: 22,
            change_number_tombstone: 33,
            count: 44,
            tainted_by_present: true,
        }]
    );
}

#[tokio::test]
async fn auction_replicate_items_rejects_missing_auctioneer_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 42);
    let missing_auctioneer =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 90_002, 78);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        player_guid,
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        0,
    );
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());

    session
        .handle_auction_replicate_items(AuctionReplicateItems {
            auctioneer: missing_auctioneer,
            change_number_global: 11,
            change_number_cursor: 22,
            change_number_tombstone: 33,
            count: 44,
            tainted_by: None,
        })
        .await;

    assert!(send_rx.try_recv().is_err());
    assert!(
        session
            .represented_auction_replicate_requests_like_cpp()
            .is_empty()
    );
}

#[tokio::test]
async fn auctionable_token_sell_at_market_price_is_silent_wotlk_stub_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_auctionable_token_sell_at_market_price(WorldPacket::new_empty())
        .await;

    assert!(send_rx.try_recv().is_err());
}
