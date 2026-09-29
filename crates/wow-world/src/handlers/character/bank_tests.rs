use crate::handlers::character::bank_test_support::{
    insert_binder_creature, make_bank_slot_session,
};
use crate::test_fixtures::{player_gold_for_test, CollectionLoadPortLikeCpp};
use wow_constants::unit::NPCFlags1;
use wow_core::guid::HighGuid;
use wow_core::ObjectGuid;
use wow_packet::packets::misc::BuyBankSlot;

#[tokio::test]
async fn buy_bank_slot_buys_next_slot_and_spends_money_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 1);
    insert_binder_creature(&canonical, banker, NPCFlags1::BANKER.bits());
    let port = CollectionLoadPortLikeCpp::for_bank_slot_purchase([
        wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::Committed,
    ]);
    session.set_player_lifecycle_port_like_cpp(port.clone());

    session
        .handle_buy_bank_slot(BuyBankSlot { guid: banker })
        .await;

    assert_eq!(session.player_bank_bag_slot_count_like_cpp(), 1);
    assert_eq!(player_gold_for_test(&session), 50);
    assert!(
        send_rx.try_recv().is_ok(),
        "bank slot update should be sent"
    );
    assert!(send_rx.try_recv().is_ok(), "money update should be sent");
    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        port.bank_slot_purchase_requests(),
        vec![wow_persistence::PlayerBankSlotPurchaseRequestLikeCpp {
            player_guid: 42,
            money_after: 50,
            bank_slot_count: 1,
        }]
    );
}

#[tokio::test]
async fn buy_bank_slot_definite_rollback_keeps_runtime_and_packets_unchanged_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 4);
    insert_binder_creature(&canonical, banker, NPCFlags1::BANKER.bits());
    let port = CollectionLoadPortLikeCpp::for_bank_slot_purchase([
        wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::DefinitelyRolledBack {
            reason: "fixture rollback".to_owned(),
        },
    ]);
    session.set_player_lifecycle_port_like_cpp(port.clone());

    session
        .handle_buy_bank_slot(BuyBankSlot { guid: banker })
        .await;

    assert_eq!(session.player_bank_bag_slot_count_like_cpp(), 0);
    assert_eq!(player_gold_for_test(&session), 150);
    assert!(send_rx.try_recv().is_err());
    assert!(
        session
            .durable_loot_money_persistence_tracker_like_cpp()
            .begin_like_cpp()
            .is_ok(),
        "a definite rollback must reopen payout admission"
    );
    assert_eq!(
        port.bank_slot_purchase_requests(),
        vec![wow_persistence::PlayerBankSlotPurchaseRequestLikeCpp {
            player_guid: 42,
            money_after: 50,
            bank_slot_count: 1,
        }]
    );
}

#[tokio::test]
async fn buy_bank_slot_rejects_non_banker_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(1);
    let creature = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 2);
    insert_binder_creature(&canonical, creature, NPCFlags1::QUEST_GIVER.bits());

    session
        .handle_buy_bank_slot(BuyBankSlot { guid: creature })
        .await;

    assert_eq!(session.player_bank_bag_slot_count_like_cpp(), 0);
    assert_eq!(player_gold_for_test(&session), 150);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn buy_bank_slot_rejects_missing_price_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(1);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 3);
    insert_binder_creature(&canonical, banker, NPCFlags1::BANKER.bits());
    session.set_player_bank_bag_slot_count_like_cpp(2);

    session
        .handle_buy_bank_slot(BuyBankSlot { guid: banker })
        .await;

    assert_eq!(session.player_bank_bag_slot_count_like_cpp(), 2);
    assert_eq!(player_gold_for_test(&session), 150);
    assert!(send_rx.try_recv().is_err());
}
