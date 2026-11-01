// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Trade contents and acceptance reset (#38 slice 2): C++ `TradeData`
//! (`TradeData.cpp`) and the handlers that drive it (`TradeHandler.cpp`
//! `HandleSetTradeItemOpcode`, `HandleClearTradeItemOpcode`,
//! `HandleSetTradeGoldOpcode`, `HandleSetTradeSpellOpcode`,
//! `HandleUnacceptTradeOpcode`) plus `WorldSession::SendUpdateTrade`.
//!
//! Every transition runs with both canonical Players under one map-manager
//! lock (`SessionCore::with_trade_pair_like_cpp`) and only records the packets
//! C++ would send, in C++ order; they are published after the lock is gone.
//! Packets for the partner's client are delivered by the partner's own session.

use wow_core::ObjectGuid;
use wow_entities::{Item, Player};
use wow_packet::ServerPacket;
use wow_packet::packets::item::{ItemInstance, ItemMod, ItemModList};
use wow_packet::packets::misc::{
    EQUIP_ERR_NOT_ENOUGH_MONEY_LIKE_CPP, TRADE_STATUS_UNACCEPTED_LIKE_CPP, TradeItemGemLikeCpp,
    TradeItemLikeCpp, TradeStatus, TradeUpdated, UnwrappedTradeItemLikeCpp,
};
use wow_world_core::session::{TradePairAccessLikeCpp, TradeSideLikeCpp};

/// C++ `TRADE_SLOT_NONTRADED`.
pub const TRADE_SLOT_NONTRADED_LIKE_CPP: u8 = 6;
/// C++ `TRADE_SLOT_TRADED_COUNT`.
pub const TRADE_SLOT_TRADED_COUNT_LIKE_CPP: u8 = 6;
/// C++ `PERM_ENCHANTMENT_SLOT` / `USE_ENCHANTMENT_SLOT` (`ItemDefines.h:179,186`).
const PERM_ENCHANTMENT_SLOT_LIKE_CPP: usize = 0;
const USE_ENCHANTMENT_SLOT_LIKE_CPP: usize = 7;

/// Packets owed by one locked trade transition, in C++ send order.
#[derive(Debug, Default)]
pub struct TradeEffectsLikeCpp {
    pub packets: Vec<(TradeSideLikeCpp, Vec<u8>)>,
}

impl TradeEffectsLikeCpp {
    fn push(&mut self, side: TradeSideLikeCpp, bytes: Vec<u8>) {
        self.packets.push((side, bytes));
    }
}

/// Inputs a locked transition cannot compute itself.
pub struct TradeTransitionCxLikeCpp<'f> {
    /// `PartnerIsSameBnetAccount` (symmetric).
    pub same_bnet: bool,
    /// Pre-drawn `rand32()` values for `UpdateServerStateIndex`.
    pub server_state_indices: &'f mut dyn Iterator<Item = u32>,
    /// `ItemTemplate::GetLockID` by entry.
    pub lock_id: &'f dyn Fn(u32) -> u16,
}

impl TradeTransitionCxLikeCpp<'_> {
    fn status(&self, status: TradeStatus) -> Vec<u8> {
        status
            .with_partner_is_same_bnet_account(self.same_bnet)
            .to_bytes()
    }

    fn next_index(&mut self) -> u32 {
        self.server_state_indices.next().unwrap_or(0)
    }
}

/// C++ `TradeData::SetAccepted(state, forTrader)`.
pub fn set_accepted_like_cpp(
    pair: &mut TradePairAccessLikeCpp<'_>,
    cx: &TradeTransitionCxLikeCpp<'_>,
    side: TradeSideLikeCpp,
    state: bool,
    for_trader: bool,
    fx: &mut TradeEffectsLikeCpp,
) {
    if !pair
        .side(side, |player| player.set_trade_accepted_like_cpp(state))
        .unwrap_or(false)
    {
        return;
    }
    if !state {
        let target = if for_trader { side.other() } else { side };
        fx.push(
            target,
            cx.status(TradeStatus::status_only_like_cpp(
                TRADE_STATUS_UNACCEPTED_LIKE_CPP,
            )),
        );
    }
}

/// C++ `TradeData::Update(forTrader)` → `SendUpdateTrade`: the packet always
/// describes `side`'s `TradeData`; `forTrader` sends it to the trader with
/// `WhichPlayer = 1`, otherwise to `side` itself with `WhichPlayer = 0`.
pub fn update_like_cpp(
    pair: &mut TradePairAccessLikeCpp<'_>,
    cx: &TradeTransitionCxLikeCpp<'_>,
    side: TradeSideLikeCpp,
    for_trader: bool,
    fx: &mut TradeEffectsLikeCpp,
) {
    let Some(Some(packet)) = pair.side(side, |player| {
        trade_updated_like_cpp(player, u8::from(for_trader), cx.lock_id)
    }) else {
        return;
    };
    let target = if for_trader { side.other() } else { side };
    fx.push(target, packet.to_bytes());
}

/// C++ `TradeData::SetItem(slot, item, update)` (`TradeData.cpp:57-80`).
pub fn set_item_like_cpp(
    pair: &mut TradePairAccessLikeCpp<'_>,
    cx: &mut TradeTransitionCxLikeCpp<'_>,
    side: TradeSideLikeCpp,
    slot: u8,
    item: Option<ObjectGuid>,
    update: bool,
    fx: &mut TradeEffectsLikeCpp,
) {
    let index = cx.next_index();
    let changed = pair
        .side(side, |player| {
            // `UpdateServerStateIndex` follows the two `SetAccepted(false)`
            // calls in C++; nothing observes the order inside the lock.
            player.set_trade_slot_item_like_cpp(slot, item, update, index)
        })
        .flatten();
    if changed != Some(true) {
        return;
    }
    // `SetAccepted(false)` already cleared `_accepted` in the transition;
    // publish its UNACCEPTED, then the trader's.
    fx.push(
        side,
        cx.status(TradeStatus::status_only_like_cpp(
            TRADE_STATUS_UNACCEPTED_LIKE_CPP,
        )),
    );
    set_accepted_like_cpp(pair, cx, side.other(), false, false, fx);
    update_like_cpp(pair, cx, side, true, fx);
    if slot == TRADE_SLOT_NONTRADED_LIKE_CPP {
        set_spell_like_cpp(pair, cx, side.other(), 0, None, fx);
    }
    set_spell_like_cpp(pair, cx, side, 0, None, fx);
}

/// C++ `TradeData::SetSpell(spell_id, castItem)` (`TradeData.cpp:82-98`).
pub fn set_spell_like_cpp(
    pair: &mut TradePairAccessLikeCpp<'_>,
    cx: &mut TradeTransitionCxLikeCpp<'_>,
    side: TradeSideLikeCpp,
    spell_id: u32,
    cast_item: Option<ObjectGuid>,
    fx: &mut TradeEffectsLikeCpp,
) {
    let index = cx.next_index();
    let changed = pair
        .side(side, |player| {
            player.set_trade_spell_value_like_cpp(spell_id, cast_item, index)
        })
        .flatten();
    if changed != Some(true) {
        return;
    }
    fx.push(
        side,
        cx.status(TradeStatus::status_only_like_cpp(
            TRADE_STATUS_UNACCEPTED_LIKE_CPP,
        )),
    );
    set_accepted_like_cpp(pair, cx, side.other(), false, false, fx);
    update_like_cpp(pair, cx, side, true, fx);
    update_like_cpp(pair, cx, side, false, fx);
}

/// C++ `TradeData::SetMoney(money)` (`TradeData.cpp:100-122`).
pub fn set_money_like_cpp(
    pair: &mut TradePairAccessLikeCpp<'_>,
    cx: &mut TradeTransitionCxLikeCpp<'_>,
    side: TradeSideLikeCpp,
    money: u64,
    fx: &mut TradeEffectsLikeCpp,
) {
    let Some(Some((current, has_enough))) = pair.side(side, |player| {
        player
            .trade_state_snapshot_like_cpp()
            .map(|trade| (trade.money, player.money() >= money))
    }) else {
        return;
    };
    if current == money {
        return;
    }
    if !has_enough {
        fx.push(
            side,
            cx.status(TradeStatus::failed_like_cpp(
                EQUIP_ERR_NOT_ENOUGH_MONEY_LIKE_CPP,
                0,
            )),
        );
        return;
    }
    let index = cx.next_index();
    if pair
        .side(side, |player| {
            player.set_trade_money_value_like_cpp(money, index)
        })
        .flatten()
        != Some(true)
    {
        return;
    }
    fx.push(
        side,
        cx.status(TradeStatus::status_only_like_cpp(
            TRADE_STATUS_UNACCEPTED_LIKE_CPP,
        )),
    );
    set_accepted_like_cpp(pair, cx, side.other(), false, false, fx);
    update_like_cpp(pair, cx, side, true, fx);
}

/// C++ `WorldSession::SendUpdateTrade` for the `TradeData` owned by `player`.
pub fn trade_updated_like_cpp(
    player: &Player,
    which_player: u8,
    lock_id: &dyn Fn(u32) -> u16,
) -> Option<TradeUpdated> {
    let trade = player.trade_state_snapshot_like_cpp()?;
    let items = trade
        .items
        .iter()
        .enumerate()
        .filter_map(|(slot, guid)| {
            let item = player.trade_item_by_guid_like_cpp((*guid)?)?;
            Some(trade_item_like_cpp(slot as u8, item, lock_id))
        })
        .collect();
    Some(TradeUpdated {
        which_player,
        id: 0,
        client_state_index: trade.client_state_index,
        current_state_index: trade.server_state_index,
        gold: trade.money,
        currency_type: 0,
        currency_quantity: 0,
        proposed_enchantment: i32::try_from(trade.spell_id).unwrap_or(0),
        items,
    })
}

/// C++ `ItemInstance::Initialize(Item const*)` (`ItemPacketsCommon.cpp:49`).
pub fn item_instance_like_cpp(item: &Item) -> ItemInstance {
    let data = item.data();
    ItemInstance {
        item_id: item.object().entry() as i32,
        random_properties_seed: data.property_seed,
        random_properties_id: data.random_properties_id,
        item_bonus: None,
        modifications: ItemModList {
            values: data
                .modifiers
                .iter()
                .enumerate()
                .filter(|(_, value)| **value != 0)
                .map(|(kind, value)| ItemMod::new(*value as i32, kind as u8))
                .collect(),
        },
    }
}

fn trade_item_like_cpp(slot: u8, item: &Item, lock_id: &dyn Fn(u32) -> u16) -> TradeItemLikeCpp {
    let data = item.data();
    let unwrapped = (!item.is_wrapped()).then(|| UnwrappedTradeItemLikeCpp {
        enchant_id: data.enchantments[PERM_ENCHANTMENT_SLOT_LIKE_CPP].id,
        on_use_enchantment_id: data.enchantments[USE_ENCHANTMENT_SLOT_LIKE_CPP].id,
        creator: data.creator,
        charges: data.spell_charges[0],
        lock: lock_id(item.object().entry()) != 0 && item.is_locked(),
        max_durability: data.max_durability,
        durability: data.durability,
        gems: data
            .gems
            .iter()
            .enumerate()
            .filter(|(_, gem)| gem.item_id != 0)
            .map(|(gem_slot, gem)| TradeItemGemLikeCpp {
                slot: gem_slot as u8,
                item: ItemInstance {
                    item_id: gem.item_id,
                    item_bonus: {
                        let bonus_list_ids: Vec<i32> = gem
                            .bonus_list_ids
                            .iter()
                            .filter(|id| **id != 0)
                            .map(|id| i32::from(*id))
                            .collect();
                        (gem.context != 0 || !bonus_list_ids.is_empty()).then_some(
                            wow_packet::packets::item::ItemBonuses {
                                context: gem.context,
                                bonus_list_ids,
                            },
                        )
                    },
                    ..ItemInstance::default()
                },
            })
            .collect(),
    });
    TradeItemLikeCpp {
        slot,
        item: item_instance_like_cpp(item),
        stack_count: item.count() as i32,
        gift_creator: data.gift_creator,
        unwrapped,
    }
}
