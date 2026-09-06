// Explicit database imports: this module reaches its parent through
// `use super::*`, and the persistence inventory cannot resolve a glob, so
// without these every database access in the file is invisible to the
// ratchet (see #277).

use super::*;
use crate::session::{SessionPlayerController, VendorBuyItemTestOverrideLikeCpp};
use crate::vendor_trade_persistence_test_fixture::{
    VendorTradePersistencePortFixtureLikeCpp, register_vendor_for_trade_test_like_cpp,
};
use std::sync::Arc;
use wow_constants::ServerOpcodes;
use wow_core::ObjectGuidGenerator;
use wow_persistence::VendorTradePersistenceRequestLikeCpp;

fn make_vendor_session() -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_packet_tx, packet_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(4);
    let mut session = WorldSession::new(
        1,
        "TestAccount".into(),
        0,
        2,
        9,
        54261,
        vec![0u8; 40],
        "esES".into(),
        packet_rx,
        send_tx,
    );
    session.set_item_guid_generator_like_cpp(Arc::new(ObjectGuidGenerator::new(HighGuid::Item, 1)));
    let player_guid = ObjectGuid::create_player(1, 42);
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(0.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_map_manager(Arc::new(std::sync::RwLock::new(
        crate::map_manager::MapManager::new(),
    )));
    (session, send_rx)
}

fn drain_server_opcodes(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<ServerOpcodes> {
    send_rx
        .try_iter()
        .filter_map(|bytes| WorldPacket::from_bytes(&bytes).server_opcode())
        .collect()
}

#[tokio::test]
async fn vendor_currency_purchase_definite_rollback_keeps_runtime_unchanged_like_cpp() {
    let (mut session, send_rx) = make_vendor_session();
    let vendor = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 43);
    register_vendor_for_trade_test_like_cpp(&mut session, vendor, 2456);
    session.set_currency_types_store(Arc::new(CurrencyTypesStore::from_entries([
        wow_data::CurrencyTypesEntry {
            id: 395,
            category_id: 0,
            inventory_icon_file_id: 0,
            spell_weight: 0,
            spell_category: 0,
            max_qty: 0,
            max_earnable_per_week: 0,
            quality: 0,
            faction_id: 0,
            award_condition_id: 0,
            flags: wow_constants::CurrencyTypesFlags::empty(),
            flags_b: wow_constants::CurrencyTypesFlagsB::empty(),
        },
    ])));
    session.set_item_extended_cost_store(Arc::new(ItemExtendedCostStore::from_entries([
        wow_data::ItemExtendedCostEntry {
            id: 12,
            required_arena_rating: 0,
            arena_bracket: 0,
            flags: wow_constants::ItemExtendedCostFlags::empty(),
            min_faction_id: 0,
            min_reputation: 0,
            required_achievement: 0,
            item_id: [0; wow_data::MAX_ITEM_EXT_COST_ITEMS],
            item_count: [0; wow_data::MAX_ITEM_EXT_COST_ITEMS],
            currency_id: [0; wow_data::MAX_ITEM_EXT_COST_CURRENCIES],
            currency_count: [0; wow_data::MAX_ITEM_EXT_COST_CURRENCIES],
        },
    ])));
    session.set_vendor_buy_item_test_override_like_cpp(VendorBuyItemTestOverrideLikeCpp {
        item_id: 395,
        item_type: ItemVendorType::Currency as i32,
        max_count: 1,
        incr_time: 0,
        player_condition_id: 0,
        has_vendor_conditions: false,
        extended_cost: 12,
        buy_price: 0,
        max_durability: 0,
        buy_count: 1,
    });

    let (port, requests) =
        VendorTradePersistencePortFixtureLikeCpp::definitely_rolled_back_like_cpp();
    session.set_vendor_trade_persistence_port_like_cpp(Arc::new(port));

    session
        .handle_buy_item(BuyItem {
            vendor_guid: vendor,
            container_guid: ObjectGuid::EMPTY,
            quantity: 1,
            muid: 1,
            slot: 0,
            item_type: ItemVendorType::Currency as i32,
            item_id: 395,
        })
        .await;

    assert_eq!(session.player_currency_quantity(395), Some(0));
    let opcodes = drain_server_opcodes(&send_rx);
    let requests = requests.lock().unwrap();
    assert_eq!(
        requests.len(),
        1,
        "server opcodes before persistence: {opcodes:?}"
    );
    let VendorTradePersistenceRequestLikeCpp::CurrencyPurchase(request) = &requests[0] else {
        panic!("currency purchase must use its semantic persistence variant");
    };
    assert_eq!(request.player_guid, 42);
    assert_eq!(request.money_before, request.money_after);
    assert!(request.item_turnins.is_empty());
    assert_eq!(opcodes, vec![ServerOpcodes::BuyFailed]);
    assert!(
        session
            .durable_loot_money_persistence_tracker_like_cpp()
            .begin_like_cpp()
            .is_ok(),
        "a definite rollback must reopen payout/save admission"
    );
}
