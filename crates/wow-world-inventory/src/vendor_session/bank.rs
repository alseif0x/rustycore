// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Banker activation and personal bank slot purchase.

use std::sync::Arc;

use tracing::{debug, info};
use wow_constants::NPCFlags1;
use wow_packet::packets::gossip::Hello;
use wow_packet::packets::misc::BuyBankSlot;
use wow_packet::packets::update::UpdateObject;

use super::VendorSessionHostLikeCpp;

/// CMSG_BANKER_ACTIVATE — player talks to a banker.
/// C++ ref: `HandleBankerActivateOpcode`
/// (`Handlers/BankHandler.cpp:60-65`) opens banker interaction UI.
pub async fn handle_banker_activate<H: VendorSessionHostLikeCpp>(host: &mut H, hello: Hello) {
    info!(
        "BankerActivate {:?} account {}",
        hello.unit,
        host.core().account_id
    );
    let Some(_banker) = host.hub_ref().represented_npc_can_interact_with_like_cpp(
        hello.unit,
        NPCFlags1::BANKER.bits(),
        0,
    ) else {
        debug!(
            banker_guid = ?hello.unit,
            account = host.core().account_id,
            "BankerActivate rejected: NPC missing, out of range, dead, or lacks BANKER flag"
        );
        return;
    };

    // C++ removes fake death only after GetNPCIfCanInteractWith accepts the
    // banker and before SendShowBank replaces InteractionData.
    host.remove_represented_feign_death_if_needed_like_cpp();
    host.send_show_bank_like_cpp(hello.unit);
}

/// CMSG_BUY_BANK_SLOT — player buys the next personal bank bag slot.
///
/// C++ ref: `WorldSession::HandleBuyBankSlotOpcode`. C++ mutates the bank
/// slot count and money in one serialized session turn; its later
/// `Player::SaveToDB`/`SaveInventoryAndGoldToDB` persists both values from
/// that coherent state. Rust must cross SQL here because detached group
/// payouts use the character money row as their durable cap authority, so
/// persist both fields atomically before publishing either runtime field.
pub async fn handle_buy_bank_slot_with_prices_and_generator_like_cpp<
    H: VendorSessionHostLikeCpp,
>(
    host: &mut H,
    prices: &wow_data::BankBagSlotPricesStore,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    buy: BuyBankSlot,
) {
    let Some(player_guid) = host.player_guid() else {
        return;
    };
    let Some(_banker) = host.hub_ref().represented_npc_can_interact_with_like_cpp(
        buy.guid,
        NPCFlags1::BANKER.bits(),
        0,
    ) else {
        debug!(
            banker_guid = ?buy.guid,
            account = host.core().account_id,
            "BuyBankSlot rejected: NPC missing, out of range, dead, or lacks BANKER flag"
        );
        return;
    };

    let Some(current_bank_slots) = ({
        let (state, hub) = host.split_inventory_ref();
        state.resolved_player_bank_bag_slot_count_like_cpp(hub)
    }) else {
        return;
    };
    let next_slot = u32::from(current_bank_slots) + 1;
    let Some(price) = prices.get(next_slot).map(|entry| entry.cost) else {
        debug!(
            next_slot,
            account = host.core().account_id,
            "BuyBankSlot rejected: missing BankBagSlotPrices.db2 row"
        );
        return;
    };

    #[cfg(any(test, feature = "test-fixtures"))]
    let test_commit_result = if host.test_fixtures_ref_like_cpp().is_some() {
        host.lifecycle()
            .loot_money_persistence_test_result_like_cpp()
    } else {
        None
    };
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let test_commit_result: Option<bool> = None;

    let lifecycle_port = if test_commit_result.is_some() {
        None
    } else {
        let Some(port) = host
            .lifecycle()
            .player_lifecycle_port_like_cpp()
            .map(Arc::clone)
        else {
            return;
        };
        Some(port)
    };

    // Close payout admission before reading money. A previously admitted
    // payout either completes and is reconciled first, or this purchase
    // closes first and the payout retries its still-available pool.
    let Some(money_persistence) = host
        .begin_exclusive_player_money_persistence_like_cpp()
        .await
    else {
        return;
    };

    let Some(old_money) = host.resolved_player_money_like_cpp() else {
        return;
    };
    if old_money < u64::from(price) {
        debug!(
            next_slot,
            price,
            old_money,
            account = host.core().account_id,
            "BuyBankSlot rejected: not enough money"
        );
        return;
    }

    let new_count = u8::try_from(next_slot).unwrap_or(u8::MAX);
    let new_money = old_money - u64::from(price);

    let money_persistence = if let Some(success) = test_commit_result {
        if !success {
            return;
        }
        money_persistence
    } else {
        let request = wow_persistence::PlayerBankSlotPurchaseRequestLikeCpp {
            player_guid: player_guid.counter() as u64,
            money_after: new_money,
            bank_slot_count: new_count,
        };
        let Some(money_persistence) = host
            .await_exclusive_player_money_transaction_outcome_like_cpp(
                money_persistence,
                lifecycle_port
                    .as_ref()
                    .expect("production bank-slot purchase retains its lifecycle port")
                    .persist_bank_slot_purchase_like_cpp(request),
                old_money,
                new_money,
                "bank-slot purchase",
            )
            .await
        else {
            return;
        };
        money_persistence
    };

    // Publish both runtime fields only after the combined SQL COMMIT. Set
    // the values synchronously while admission is still closed, then drop
    // the fence before criteria processing can re-enter persistence.
    if !({
        let (state, mut hub) = host.split_inventory_mut();
        state.set_player_gold_like_cpp(&mut hub, new_money)
    }) {
        host.core_mut()
            .kick("canonical Player money owner became unavailable after bank-slot COMMIT");
        return;
    }
    if !({
        let (state, mut hub) = host.split_inventory_mut();
        state.set_player_bank_bag_slot_count_like_cpp(&mut hub, new_count)
    }) {
        host.core_mut()
            .kick("canonical Player bank-slot owner became unavailable after COMMIT");
        return;
    }
    {
        let (s, h) = host.split_inventory_ref();
        s.send_player_bank_bag_slots_update_like_cpp(h, new_count)
    };
    host.enqueue_represented_quest_objective_money_changed_like_cpp(old_money, new_money);
    host.sync_player_registry_state_like_cpp();
    drop(money_persistence);

    host.send_packet(&UpdateObject::player_money_update(
        player_guid,
        host.core().player_map_id_like_cpp(),
        new_money,
        None,
    ));
    host.drain_represented_quest_objective_progress_with_generator_like_cpp(item_guid_generator)
        .await;
}
