// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Handler bodies over the locked `TradeData` transitions (#38 slice 2).
//!
//! C++ `TradeHandler.cpp`: `HandleSetTradeItemOpcode` (`:719-765`),
//! `HandleClearTradeItemOpcode` (`:767-780`), `HandleSetTradeGoldOpcode`
//! (`:708-717`), `HandleSetTradeSpellOpcode` (`:786-821`),
//! `HandleUnacceptTradeOpcode` (`:536-543`) and the state-index half of
//! `HandleAcceptTradeOpcode` (`:236-261`).

use wow_core::ObjectGuid;
use wow_entities::{Item, NULL_BAG, NULL_SLOT, is_bag_pos, make_item_pos};
use wow_packet::packets::misc::{
    TRADE_SLOT_COUNT_LIKE_CPP, TRADE_STATUS_CANCELLED_LIKE_CPP, TradeStatus,
};
use wow_world_core::session::mailbox::{SendRepresentedTradeStatusLikeCppCommand, SessionCommand};
use wow_world_core::session::{TradePairAccessLikeCpp, TradeSideLikeCpp};

use crate::TradeHandlerCxLikeCpp;
use crate::trade_contents::{
    TRADE_SLOT_NONTRADED_LIKE_CPP, TradeEffectsLikeCpp, TradeTransitionCxLikeCpp,
    set_accepted_like_cpp, set_item_like_cpp, set_money_like_cpp, set_spell_like_cpp,
};

/// One transition draws at most this many `rand32()` server state indices
/// (`SetItem` on the non-traded slot: own item, trader spell, own spell).
const SERVER_STATE_INDEX_DRAWS_LIKE_CPP: usize = 4;

impl TradeHandlerCxLikeCpp<'_> {
    /// Run one locked `TradeData` transition and publish its packets in C++
    /// order once the lock is released. `None` without an open trade.
    pub(crate) fn run_trade_transition_like_cpp<R>(
        &mut self,
        transition: impl FnOnce(
            &mut TradePairAccessLikeCpp<'_>,
            &mut TradeTransitionCxLikeCpp<'_>,
            &mut TradeEffectsLikeCpp,
        ) -> R,
    ) -> Option<R> {
        let partner = self
            .player_trade_state_snapshot_like_cpp()
            .flatten()
            .map(|trade| trade.partner_guid)?;
        let same_bnet =
            crate::partner_is_same_bnet_account_like_cpp(self.hub.shared(), Some(partner));
        let indices: Vec<u32> = (0..SERVER_STATE_INDEX_DRAWS_LIKE_CPP)
            .map(|_| self.hub.core.represented_urand_u32_like_cpp(0, u32::MAX))
            .collect();
        let stats = self.hub.catalogs.items.stats_store.clone();
        let lock_id = move |entry: u32| {
            stats
                .as_ref()
                .and_then(|store| store.sparse_template(entry))
                .map_or(0, |sparse| sparse.lock_id)
        };
        let mut fx = TradeEffectsLikeCpp::default();
        let result = self.hub.shared().core.with_trade_pair_like_cpp(|pair| {
            let mut iter = indices.into_iter();
            let mut cx = TradeTransitionCxLikeCpp {
                same_bnet,
                server_state_indices: &mut iter,
                lock_id: &lock_id,
            };
            transition(pair, &mut cx, &mut fx)
        });
        self.publish_trade_effects_like_cpp(partner, fx);
        result
    }

    pub(crate) fn publish_trade_effects_like_cpp(
        &self,
        partner: ObjectGuid,
        fx: TradeEffectsLikeCpp,
    ) {
        let core = self.hub.shared().core;
        for (side, bytes) in fx.packets {
            match side {
                TradeSideLikeCpp::Own => core.send_raw_packet(&bytes),
                TradeSideLikeCpp::Partner => core.try_send_connected_player_command_like_cpp(
                    partner,
                    SessionCommand::SendRepresentedTradeStatusLikeCpp(
                        SendRepresentedTradeStatusLikeCppCommand {
                            packet_bytes: bytes,
                        },
                    ),
                ),
            }
        }
    }

    /// CMSG_SET_TRADE_ITEM body.
    pub fn set_represented_trade_item_like_cpp(
        &mut self,
        trade_slot: u8,
        pack_slot: u8,
        item_slot_in_pack: u8,
    ) {
        let Some(Some(trade)) = self.player_trade_state_snapshot_like_cpp() else {
            return;
        };
        let cancelled = TradeStatus::status_only_like_cpp(TRADE_STATUS_CANCELLED_LIKE_CPP);
        if trade_slot >= TRADE_SLOT_COUNT_LIKE_CPP {
            self.send_own_trade_status_like_cpp(cancelled);
            return;
        }
        // `_player->GetItemByPos(PackSlot, ItemSlotInPack)`.
        let item = self
            .inventory
            .get_inventory_item_by_pos(self.hub.shared(), pack_slot, item_slot_in_pack)
            .and_then(|item| {
                self.inventory
                    .resolved_inventory_item_object_like_cpp(self.hub.shared(), item.guid)
            });
        let Some(item) = item else {
            self.send_own_trade_status_like_cpp(cancelled);
            return;
        };
        if trade_slot != TRADE_SLOT_NONTRADED_LIKE_CPP
            && !self.item_can_be_traded_like_cpp(&item, pack_slot, item_slot_in_pack)
        {
            self.send_own_trade_status_like_cpp(cancelled);
            return;
        }
        let item_guid = item.object().guid();
        if trade.items.contains(&Some(item_guid)) {
            self.send_own_trade_status_like_cpp(cancelled);
            return;
        }

        let _ = self.hub.core.with_owned_player_mut_like_cpp(|player| {
            player.advance_trade_client_state_index_like_cpp()
        });

        if trade_slot != TRADE_SLOT_NONTRADED_LIKE_CPP {
            let partner = trade.partner_guid;
            let bop_allowed = item.is_soulbound_trade_allowed_for(partner);
            // C++ `Item::IsBindedNotWith`; only `IsBoundAccountWide` reads the
            // template, so an unknown template is not account-wide.
            let template = self
                .hub
                .catalogs
                .item_storage_template(item.object().entry())
                .unwrap_or_else(|| {
                    wow_entities::ItemStorageTemplate::regular_item(item.object().entry(), 1)
                });
            let binded_not_with = item.is_binded_not_with(partner, &template, bop_allowed);
            if binded_not_with {
                self.send_own_trade_status_like_cpp(TradeStatus::not_on_taplist_like_cpp(
                    trade_slot,
                ));
                return;
            }
        }

        let _ = self.run_trade_transition_like_cpp(|pair, cx, fx| {
            set_item_like_cpp(
                pair,
                cx,
                TradeSideLikeCpp::Own,
                trade_slot,
                Some(item_guid),
                false,
                fx,
            );
        });
    }

    /// CMSG_CLEAR_TRADE_ITEM body.
    pub fn clear_represented_trade_item_like_cpp(&mut self, trade_slot: u8) {
        if !matches!(self.player_trade_state_snapshot_like_cpp(), Some(Some(_))) {
            return;
        }
        let _ = self.hub.core.with_owned_player_mut_like_cpp(|player| {
            player.advance_trade_client_state_index_like_cpp()
        });
        if trade_slot >= TRADE_SLOT_COUNT_LIKE_CPP {
            return;
        }
        let _ = self.run_trade_transition_like_cpp(|pair, cx, fx| {
            set_item_like_cpp(pair, cx, TradeSideLikeCpp::Own, trade_slot, None, false, fx);
        });
    }

    /// CMSG_SET_TRADE_GOLD body.
    pub fn set_represented_trade_gold_like_cpp(&mut self, coinage: u64) {
        if !matches!(self.player_trade_state_snapshot_like_cpp(), Some(Some(_))) {
            return;
        }
        let _ = self.hub.core.with_owned_player_mut_like_cpp(|player| {
            player.advance_trade_client_state_index_like_cpp()
        });
        let _ = self.run_trade_transition_like_cpp(|pair, cx, fx| {
            set_money_like_cpp(pair, cx, TradeSideLikeCpp::Own, coinage, fx);
        });
    }

    /// CMSG_SET_TRADE_SPELL body: the validation runs on the session's own
    /// Player, then `TradeData::SetSpell`.
    pub(crate) fn set_represented_trade_spell_like_cpp(
        &mut self,
        spell_id: u32,
        pack_slot: u8,
        item_slot_in_pack: u8,
    ) {
        if !matches!(self.player_trade_state_snapshot_like_cpp(), Some(Some(_))) {
            return;
        }
        let mut cast_item = None;
        let spell_id = if spell_id == 0 {
            0
        } else {
            if pack_slot != NULL_BAG || item_slot_in_pack != NULL_SLOT {
                cast_item = self
                    .inventory
                    .get_inventory_item_by_pos(self.hub.shared(), pack_slot, item_slot_in_pack)
                    .map(|item| item.guid);
            }
            let known = i32::try_from(spell_id).ok().filter(|id| {
                self.hub
                    .catalogs
                    .spell_store()
                    .is_some_and(|store| store.get(*id).is_some())
            });
            match known {
                Some(id) if self.known_spells_like_cpp().contains(&id) => spell_id,
                _ => {
                    cast_item = None;
                    0
                }
            }
        };
        let _ = self.run_trade_transition_like_cpp(|pair, cx, fx| {
            set_spell_like_cpp(pair, cx, TradeSideLikeCpp::Own, spell_id, cast_item, fx);
        });
    }

    /// CMSG_UNACCEPT_TRADE: `my_trade->SetAccepted(false, true)`.
    pub fn unaccept_represented_trade_like_cpp(&mut self) {
        let _ = self.run_trade_transition_like_cpp(|pair, cx, fx| {
            set_accepted_like_cpp(pair, cx, TradeSideLikeCpp::Own, false, true, fx);
        });
    }

    /// C++ `Item::CanBeTraded(false, true)` (`Item.cpp:927-950`).
    pub(crate) fn item_can_be_traded_like_cpp(&self, item: &Item, bag: u8, slot: u8) -> bool {
        if item.loot_generated() {
            return false;
        }
        if item.is_soul_bound() && !item.is_bop_tradeable() {
            return false;
        }
        let hub = self.hub.shared();
        let template = hub.catalogs.item_storage_template(item.object().entry());
        let is_bag = template
            .as_ref()
            .is_some_and(|template| template.inventory_type == wow_constants::InventoryType::Bag);
        let not_empty_bag = is_bag
            && self
                .inventory
                .direct_item_contains_items(hub, item.object().guid());
        if is_bag && (is_bag_pos(make_item_pos(bag, slot)) || not_empty_bag) {
            return false;
        }
        if self.inventory.can_unequip_inventory_item_at_like_cpp(
            hub,
            bag,
            slot,
            false,
            Some(item),
            template.as_ref(),
            not_empty_bag,
        ) != wow_constants::InventoryResult::Ok
        {
            return false;
        }
        if self.active_loot_guid == item.object().guid() {
            return false;
        }
        !self.item_is_bound_by_enchant_like_cpp(item)
    }

    /// C++ `Item::IsBoundByEnchant`.
    fn item_is_bound_by_enchant_like_cpp(&self, item: &Item) -> bool {
        let Some(store) = self
            .hub
            .catalogs
            .spell_catalogs
            .spell_item_enchantment_store
            .as_ref()
        else {
            return false;
        };
        item.data().enchantments.iter().any(|enchantment| {
            u32::try_from(enchantment.id)
                .ok()
                .filter(|id| *id != 0)
                .and_then(|id| store.get(id))
                .is_some_and(|entry| {
                    entry
                        .flags
                        .contains(wow_constants::SpellItemEnchantmentFlags::SOULBOUND)
                })
        })
    }
}
