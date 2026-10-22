//! Trade contents and acceptance reset (#38 slice 2): C++ `TradeData` and the
//! `TradeHandler.cpp` handlers that drive it, on two canonical sessions.

use super::initiate::{TradePairLikeCpp, initiate_trade_packet, trade_pair_like_cpp};
use super::*;
use crate::session::WorldSession;
use wow_packet::packets::misc::{
    EQUIP_ERR_NOT_ENOUGH_MONEY_LIKE_CPP, TRADE_STATUS_ACCEPTED_LIKE_CPP,
    TRADE_STATUS_FAILED_LIKE_CPP, TRADE_STATUS_STATE_CHANGED_LIKE_CPP,
    TRADE_STATUS_UNACCEPTED_LIKE_CPP,
};
use wow_world_social::TradePolicyLikeCpp;

const TRADE_STATUS: u16 = ServerOpcodes::TradeStatus as u16;
const TRADE_UPDATED: u16 = ServerOpcodes::TradeUpdated as u16;

pub(super) async fn open_pair() -> TradePairLikeCpp {
    let mut pair = trade_pair_like_cpp(Position::new(3.0, 1.0, 0.0, 0.0), 1, 10, false);
    pair.source
        .handle_initiate_trade_with_policy(
            initiate_trade_packet(pair.partner_guid),
            TradePolicyLikeCpp::default(),
        )
        .await;
    pair.partner
        .process_represented_session_commands_like_cpp()
        .await;
    let _ = pair.partner_rx.try_iter().count();
    pair
}

pub(super) async fn drain(pair: &mut TradePairLikeCpp) -> (Vec<Vec<u8>>, Vec<Vec<u8>>) {
    pair.partner
        .process_represented_session_commands_like_cpp()
        .await;
    pair.source
        .process_represented_session_commands_like_cpp()
        .await;
    (
        pair.source_rx.try_iter().collect(),
        pair.partner_rx.try_iter().collect(),
    )
}

/// A backpack item on the session's canonical Player, in every view
/// `Player::GetItemByPos`/`GetItemByGuid` reads.
pub(super) fn insert_trade_item(
    session: &mut WorldSession,
    owner: ObjectGuid,
    slot: u8,
    item_guid: ObjectGuid,
    entry: u32,
) {
    insert_trade_test_item(session, owner, slot, item_guid, entry);
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player.set_inventory_slot_count(16);
            player.store_top_level_item(slot, item_guid).unwrap();
        })
        .unwrap();
}

fn opcode(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

pub(super) fn status(bytes: &[u8]) -> u8 {
    assert_eq!(opcode(bytes), TRADE_STATUS);
    (bytes[2] >> 2) & 0x1F
}

pub(super) fn trade(session: &WorldSession) -> wow_entities::PlayerTradeStateLikeCpp {
    wow_world_social::player_trade_state_snapshot_like_cpp(&session.core, &session.social)
        .flatten()
        .expect("open trade")
}

pub(super) fn accept_both(pair: &mut TradePairLikeCpp) {
    for session in [&mut pair.source, &mut pair.partner] {
        session
            .core
            .with_owned_player_mut_like_cpp(|player| player.set_trade_accepted_like_cpp(true))
            .unwrap();
    }
}

/// `TradeUpdated` header: WhichPlayer, ClientStateIndex, CurrentStateIndex, Gold.
fn trade_updated_header(bytes: &[u8]) -> (u8, u32, u32, u64) {
    assert_eq!(opcode(bytes), TRADE_UPDATED);
    let body = &bytes[2..];
    (
        body[0],
        u32::from_le_bytes(body[5..9].try_into().unwrap()),
        u32::from_le_bytes(body[9..13].try_into().unwrap()),
        u64::from_le_bytes(body[13..21].try_into().unwrap()),
    )
}

fn trade_updated_item_count(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes[2 + 33..2 + 37].try_into().unwrap())
}

#[tokio::test]
async fn set_trade_item_resets_both_acceptances_and_updates_trader_like_cpp() {
    let mut pair = open_pair().await;
    let item_guid = ObjectGuid::create_item(1, 1234);
    insert_trade_item(&mut pair.source, pair.source_guid, 35, item_guid, 700);
    accept_both(&mut pair);

    pair.source
        .handle_set_trade_item(set_trade_item_packet(2, 255, 35))
        .await;
    let (source, partner) = drain(&mut pair).await;

    let state = trade(&pair.source);
    assert_eq!(state.items[2], Some(item_guid));
    assert_eq!(state.client_state_index, 2, "UpdateClientStateIndex");
    assert!(!state.accepted);
    assert!(!trade(&pair.partner).accepted);
    // SetAccepted(false) → own UNACCEPTED; trader SetAccepted(false) →
    // trader UNACCEPTED; Update() → the trader receives our TradeData.
    assert_eq!(source.len(), 1);
    assert_eq!(status(&source[0]), TRADE_STATUS_UNACCEPTED_LIKE_CPP);
    assert_eq!(partner.len(), 2);
    assert_eq!(status(&partner[0]), TRADE_STATUS_UNACCEPTED_LIKE_CPP);
    let (which, client, server, gold) = trade_updated_header(&partner[1]);
    assert_eq!(
        (which, client, server, gold),
        (1, 2, state.server_state_index, 0)
    );
    assert_eq!(trade_updated_item_count(&partner[1]), 1);
    assert_eq!(partner[1][2 + 37], 2, "trade slot");
}

#[tokio::test]
async fn set_trade_item_refusals_cancel_without_client_state_like_cpp() {
    let mut pair = open_pair().await;
    let item_guid = ObjectGuid::create_item(1, 1234);
    insert_trade_item(&mut pair.source, pair.source_guid, 35, item_guid, 700);

    // Invalid trade slot and missing item: CANCELLED before
    // UpdateClientStateIndex.
    pair.source
        .handle_set_trade_item(set_trade_item_packet(7, 255, 35))
        .await;
    pair.source
        .handle_set_trade_item(set_trade_item_packet(1, 255, 36))
        .await;
    let (source, partner) = drain(&mut pair).await;
    assert_eq!(source.len(), 2);
    assert!(
        source
            .iter()
            .all(|bytes| status(bytes) == TRADE_STATUS_CANCELLED_LIKE_CPP)
    );
    assert!(partner.is_empty());
    assert_eq!(trade(&pair.source).client_state_index, 1);

    pair.source
        .handle_set_trade_item(set_trade_item_packet(1, 255, 35))
        .await;
    let _ = drain(&mut pair).await;
    // The same item in a second slot.
    pair.source
        .handle_set_trade_item(set_trade_item_packet(3, 255, 35))
        .await;
    let (source, _) = drain(&mut pair).await;
    assert_eq!(source.len(), 1);
    assert_eq!(status(&source[0]), TRADE_STATUS_CANCELLED_LIKE_CPP);
    assert_eq!(trade(&pair.source).client_state_index, 2);
    assert_eq!(trade(&pair.source).items[3], None);
}

#[tokio::test]
async fn set_trade_item_soulbound_item_is_cancelled_like_cpp() {
    let mut pair = open_pair().await;
    let item_guid = ObjectGuid::create_item(1, 1234);
    insert_trade_item(&mut pair.source, pair.source_guid, 35, item_guid, 700);
    pair.source
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .inventory_runtime_mut_like_cpp()
                .item_objects_mut()
                .get_mut(&item_guid)
                .unwrap()
                .set_binding(true)
        })
        .unwrap();

    pair.source
        .handle_set_trade_item(set_trade_item_packet(0, 255, 35))
        .await;
    let (source, partner) = drain(&mut pair).await;
    assert_eq!(status(&source[0]), TRADE_STATUS_CANCELLED_LIKE_CPP);
    assert!(partner.is_empty());

    // The non-traded slot skips CanBeTraded and IsBindedNotWith.
    pair.source
        .handle_set_trade_item(set_trade_item_packet(6, 255, 35))
        .await;
    let (source, _) = drain(&mut pair).await;
    assert_eq!(status(&source[0]), TRADE_STATUS_UNACCEPTED_LIKE_CPP);
    assert_eq!(trade(&pair.source).items[6], Some(item_guid));
}

#[tokio::test]
async fn clear_trade_item_follows_cpp_like_cpp() {
    let mut pair = open_pair().await;
    // An invalid and an empty slot only advance the client state.
    pair.source
        .handle_clear_trade_item(clear_trade_item_packet(7))
        .await;
    pair.source
        .handle_clear_trade_item(clear_trade_item_packet(2))
        .await;
    let (source, partner) = drain(&mut pair).await;
    assert!(source.is_empty() && partner.is_empty());
    assert_eq!(trade(&pair.source).client_state_index, 3);
    assert_eq!(trade(&pair.source).server_state_index, 1);

    let item_guid = ObjectGuid::create_item(1, 1234);
    insert_trade_item(&mut pair.source, pair.source_guid, 35, item_guid, 700);
    pair.source
        .handle_set_trade_item(set_trade_item_packet(2, 255, 35))
        .await;
    let _ = drain(&mut pair).await;
    accept_both(&mut pair);
    pair.source
        .handle_clear_trade_item(clear_trade_item_packet(2))
        .await;
    let (source, partner) = drain(&mut pair).await;
    assert_eq!(trade(&pair.source).items[2], None);
    assert!(!trade(&pair.partner).accepted);
    assert_eq!(status(&source[0]), TRADE_STATUS_UNACCEPTED_LIKE_CPP);
    assert_eq!(status(&partner[0]), TRADE_STATUS_UNACCEPTED_LIKE_CPP);
    assert_eq!(trade_updated_item_count(&partner[1]), 0);
}

#[tokio::test]
async fn set_trade_gold_follows_trade_data_set_money_like_cpp() {
    let mut pair = open_pair().await;
    assert!(pair.source.set_player_gold_like_cpp(100));

    // The same amount (0): only the client state advances.
    pair.source
        .handle_set_trade_gold(set_trade_gold_packet(0))
        .await;
    let (source, partner) = drain(&mut pair).await;
    assert!(source.is_empty() && partner.is_empty());

    // Not enough money: FAILED / NOT_ENOUGH_MONEY to the owner only.
    pair.source
        .handle_set_trade_gold(set_trade_gold_packet(500))
        .await;
    let (source, partner) = drain(&mut pair).await;
    assert_eq!(status(&source[0]), TRADE_STATUS_FAILED_LIKE_CPP);
    assert_eq!(
        i32::from_le_bytes(source[0][3..7].try_into().unwrap()),
        EQUIP_ERR_NOT_ENOUGH_MONEY_LIKE_CPP
    );
    assert!(partner.is_empty());
    assert_eq!(trade(&pair.source).money, 0);

    accept_both(&mut pair);
    pair.source
        .handle_set_trade_gold(set_trade_gold_packet(60))
        .await;
    let (source, partner) = drain(&mut pair).await;
    let state = trade(&pair.source);
    assert_eq!((state.money, state.client_state_index), (60, 4));
    assert!(!state.accepted && !trade(&pair.partner).accepted);
    assert_eq!(status(&source[0]), TRADE_STATUS_UNACCEPTED_LIKE_CPP);
    assert_eq!(status(&partner[0]), TRADE_STATUS_UNACCEPTED_LIKE_CPP);
    assert_eq!(trade_updated_header(&partner[1]).3, 60);
}

#[tokio::test]
async fn set_trade_spell_updates_both_views_like_cpp() {
    let mut pair = open_pair().await;
    install_trade_test_spell(&mut pair.source, 7418);
    let cast_item = ObjectGuid::create_item(1, 1234);
    insert_trade_item(&mut pair.source, pair.source_guid, 35, cast_item, 700);

    pair.source
        .handle_set_trade_spell(set_trade_spell_packet(7418, 255, 35))
        .await;
    let (source, partner) = drain(&mut pair).await;
    let state = trade(&pair.source);
    assert_eq!(
        (state.spell_id, state.spell_cast_item_guid),
        (7418, Some(cast_item))
    );
    assert_eq!(
        state.client_state_index, 1,
        "SetTradeSpell keeps the client index"
    );
    // Own UNACCEPTED, trader UNACCEPTED, Update(true) to the trader,
    // Update(false) to ourselves with WhichPlayer 0.
    assert_eq!(status(&source[0]), TRADE_STATUS_UNACCEPTED_LIKE_CPP);
    assert_eq!(trade_updated_header(&source[1]).0, 0);
    assert_eq!(status(&partner[0]), TRADE_STATUS_UNACCEPTED_LIKE_CPP);
    assert_eq!(trade_updated_header(&partner[1]).0, 1);

    // The same spell and cast item is a no-op; an unknown spell clears it.
    pair.source
        .handle_set_trade_spell(set_trade_spell_packet(7418, 255, 35))
        .await;
    let (source, partner) = drain(&mut pair).await;
    assert!(source.is_empty() && partner.is_empty());
    pair.source
        .handle_set_trade_spell(set_trade_spell_packet(9999, 255, 255))
        .await;
    let _ = drain(&mut pair).await;
    assert_eq!(trade(&pair.source).spell_id, 0);
}

#[tokio::test]
async fn non_traded_slot_change_clears_trader_spell_like_cpp() {
    let mut pair = open_pair().await;
    pair.partner
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player.set_trade_spell_value_like_cpp(7418, None, 5)
        })
        .unwrap();
    let item_guid = ObjectGuid::create_item(1, 1234);
    insert_trade_item(&mut pair.source, pair.source_guid, 35, item_guid, 700);

    pair.source
        .handle_set_trade_item(set_trade_item_packet(6, 255, 35))
        .await;
    let (_, partner) = drain(&mut pair).await;
    assert_eq!(trade(&pair.partner).spell_id, 0);
    // Our SetItem (UNACCEPTED + our view), then the trader's SetSpell(0):
    // its UNACCEPTED and its own Update(false) view.
    assert_eq!(partner.len(), 4);
    assert_eq!(trade_updated_header(&partner[3]).0, 0);
}

#[tokio::test]
async fn unaccept_trade_notifies_only_the_trader_like_cpp() {
    let mut pair = open_pair().await;
    accept_both(&mut pair);
    pair.source
        .handle_unaccept_trade(WorldPacket::new_empty())
        .await;
    let (source, partner) = drain(&mut pair).await;
    assert!(!trade(&pair.source).accepted);
    assert!(trade(&pair.partner).accepted);
    assert!(source.is_empty());
    assert_eq!(status(&partner[0]), TRADE_STATUS_UNACCEPTED_LIKE_CPP);
}

#[tokio::test]
async fn accept_trade_checks_the_traders_server_state_index_like_cpp() {
    let mut pair = open_pair().await;
    let partner_index = trade(&pair.partner).server_state_index;

    pair.source
        .handle_accept_trade(accept_trade_packet(partner_index.wrapping_add(1)))
        .await;
    let (source, partner) = drain(&mut pair).await;
    assert!(!trade(&pair.source).accepted);
    assert_eq!(status(&source[0]), TRADE_STATUS_STATE_CHANGED_LIKE_CPP);
    assert_eq!(status(&source[1]), TRADE_STATUS_UNACCEPTED_LIKE_CPP);
    assert!(partner.is_empty());

    pair.source
        .handle_accept_trade(accept_trade_packet(partner_index))
        .await;
    let (source, partner) = drain(&mut pair).await;
    assert!(trade(&pair.source).accepted);
    assert!(source.is_empty());
    assert_eq!(status(&partner[0]), TRADE_STATUS_ACCEPTED_LIKE_CPP);
}

fn with_item(
    pair: &mut TradePairLikeCpp,
    item_guid: ObjectGuid,
    f: impl FnOnce(&mut wow_entities::Item),
) {
    pair.source
        .core
        .with_owned_player_mut_like_cpp(|player| {
            f(player
                .inventory_runtime_mut_like_cpp()
                .item_objects_mut()
                .get_mut(&item_guid)
                .unwrap())
        })
        .unwrap();
}

#[tokio::test]
async fn set_trade_item_bop_tradeable_not_for_trader_is_not_on_taplist_like_cpp() {
    let mut pair = open_pair().await;
    let item_guid = ObjectGuid::create_item(1, 1234);
    insert_trade_item(&mut pair.source, pair.source_guid, 35, item_guid, 700);
    let stranger = ObjectGuid::create_player(1, 4242);
    with_item(&mut pair, item_guid, |item| {
        item.set_binding(true);
        item.set_soulbound_tradeable([stranger]);
    });
    pair.source
        .handle_set_trade_item(set_trade_item_packet(4, 255, 35))
        .await;
    let (source, partner) = drain(&mut pair).await;
    assert_eq!(
        &source[0][2..],
        &[
            wow_packet::packets::misc::TRADE_STATUS_NOT_ON_TAPLIST_LIKE_CPP << 2,
            4
        ]
    );
    assert!(partner.is_empty());
    // `UpdateClientStateIndex` ran before the refusal; the slot stays empty.
    assert_eq!(trade(&pair.source).client_state_index, 2);
    assert_eq!(trade(&pair.source).items[4], None);

    // Allowed for the trader: the item is offered.
    let partner_guid = pair.partner_guid;
    with_item(&mut pair, item_guid, |item| {
        item.set_soulbound_tradeable([partner_guid])
    });
    pair.source
        .handle_set_trade_item(set_trade_item_packet(4, 255, 35))
        .await;
    let _ = drain(&mut pair).await;
    assert_eq!(trade(&pair.source).items[4], Some(item_guid));
}

#[tokio::test]
async fn set_trade_item_looted_items_are_cancelled_like_cpp() {
    // `m_lootGenerated`.
    let mut pair = open_pair().await;
    let item_guid = ObjectGuid::create_item(1, 1234);
    insert_trade_item(&mut pair.source, pair.source_guid, 35, item_guid, 700);
    with_item(&mut pair, item_guid, |item| item.set_loot_generated(true));
    pair.source
        .handle_set_trade_item(set_trade_item_packet(0, 255, 35))
        .await;
    let (source, _) = drain(&mut pair).await;
    assert_eq!(status(&source[0]), TRADE_STATUS_CANCELLED_LIKE_CPP);

    // `owner->GetLootGUID() == GetGUID()`.
    let mut pair = open_pair().await;
    insert_trade_item(&mut pair.source, pair.source_guid, 35, item_guid, 700);
    pair.source.loot.set_active_loot_guid(item_guid);
    pair.source
        .handle_set_trade_item(set_trade_item_packet(0, 255, 35))
        .await;
    let (source, _) = drain(&mut pair).await;
    assert_eq!(status(&source[0]), TRADE_STATUS_CANCELLED_LIKE_CPP);
    assert_eq!(trade(&pair.source).items[0], None);
}

#[tokio::test]
async fn partner_statuses_carry_the_same_bnet_flag_like_cpp() {
    let mut pair = trade_pair_like_cpp(Position::new(3.0, 1.0, 0.0, 0.0), 1, 10, true);
    pair.source
        .handle_initiate_trade_with_policy(
            initiate_trade_packet(pair.partner_guid),
            TradePolicyLikeCpp::default(),
        )
        .await;
    let _ = drain(&mut pair).await;
    accept_both(&mut pair);
    pair.source
        .handle_set_trade_gold(set_trade_gold_packet(0))
        .await;
    assert!(pair.source.set_player_gold_like_cpp(100));
    pair.source
        .handle_set_trade_gold(set_trade_gold_packet(10))
        .await;
    let (source, partner) = drain(&mut pair).await;
    // UNACCEPTED to ourselves and to the trader, both flagged.
    assert_eq!(source[0][2], 0x80 | (TRADE_STATUS_UNACCEPTED_LIKE_CPP << 2));
    assert_eq!(
        partner[0][2],
        0x80 | (TRADE_STATUS_UNACCEPTED_LIKE_CPP << 2)
    );

    let index = trade(&pair.partner).server_state_index;
    pair.source
        .handle_accept_trade(accept_trade_packet(index))
        .await;
    let (_, partner) = drain(&mut pair).await;
    assert_eq!(partner[0][2], 0x80 | (TRADE_STATUS_ACCEPTED_LIKE_CPP << 2));
}
