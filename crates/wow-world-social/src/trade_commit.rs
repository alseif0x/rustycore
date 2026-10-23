// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `CMSG_ACCEPT_TRADE` through the atomic exchange (#38 slice 3).
//!
//! C++ `WorldSession::HandleAcceptTradeOpcode` (`TradeHandler.cpp:236-520`):
//! the checks in C++ order, then — when the trader already accepted —
//! `setAcceptTradeMode`, the trade spells, `TRADE_STATUS_ACCEPTED` to the
//! trader, `CanStoreItems` for both, `MoveItemFromInventory` for every traded
//! item, `moveItems`, `ModifyMoney` four times, both `m_trade` deleted, one
//! Character transaction with both `SaveInventoryAndGoldToDB`, and
//! `TRADE_STATUS_COMPLETE` to both.
//!
//! Rust order (each step keeps the C++ outcome of that step):
//! 1. the C++ checks with both Players under one map-manager lock;
//! 2. the coordinator's exclusive money fence, then the partner parked with
//!    its own fence (see `wow_world_core::session::mailbox::TradeCommitRequestLikeCpp`);
//! 3. `CanStoreItems` on detached copies of both canonical Players;
//! 4. one detached worker: the single COMMIT, then — only after it — both
//!    canonical Players change under one lock, and each side gets the
//!    publication of its own client;
//! 5. each session publishes its side, then queues its `MoneyChanged` and
//!    drains it, which raises `player.money_changed` once per Player after the
//!    commit and outside every guard (#169).

use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use wow_constants::InventoryResult;
use wow_core::ObjectGuid;
use wow_entities::{INVENTORY_SLOT_BAG_0, Item, MAX_MONEY_AMOUNT, Player, TradeTakenItemLikeCpp};
use wow_packet::ServerPacket;
use wow_packet::packets::misc::{
    EQUIP_ERR_NOT_ENOUGH_MONEY_LIKE_CPP, TRADE_DISTANCE_LIKE_CPP, TRADE_STATUS_ACCEPTED_LIKE_CPP,
    TRADE_STATUS_CANCELLED_LIKE_CPP, TRADE_STATUS_COMPLETE_LIKE_CPP,
    TRADE_STATUS_STATE_CHANGED_LIKE_CPP, TRADE_STATUS_TOO_FAR_AWAY_LIKE_CPP, TradeStatus,
};
use wow_persistence::{
    PlayerTradeItemPersistenceLikeCpp, PlayerTradeMoneyPersistenceLikeCpp,
    PlayerTradePersistenceOutcomeLikeCpp, PlayerTradePersistencePortLikeCpp,
    PlayerTradePersistenceRequestLikeCpp, PlayerTradeReconciliationLikeCpp,
    classify_player_trade_reconciliation_like_cpp,
};
use wow_world_core::loot_persistence::DurableLootMoneyPersistenceTrackerLikeCpp;
use wow_world_core::session::mailbox::{
    SessionCommand, TradeCommitParkedLikeCpp, TradeCommitRequestLikeCpp, TradeSideOutcomeLikeCpp,
    TradeSidePublicationLikeCpp,
};
use wow_world_core::session::{TradeCommitTargetLikeCpp, TradeSideLikeCpp};
use wow_world_inventory::{
    TradeStoreActionLikeCpp, TradeStoreCatalogsLikeCpp, TradeStorePlanLikeCpp,
    plan_trade_receive_like_cpp,
};

use crate::TradeHandlerCxLikeCpp;
use crate::trade_contents::{
    TRADE_SLOT_TRADED_COUNT_LIKE_CPP, set_accepted_like_cpp, set_spell_like_cpp,
};

/// C++ `EQUIP_ERR_TOO_MUCH_GOLD`.
pub const EQUIP_ERR_TOO_MUCH_GOLD_LIKE_CPP: i32 = InventoryResult::TooMuchGold as i32;
/// C++ `EQUIP_ERR_TRADE_BOUND_ITEM`.
pub const EQUIP_ERR_TRADE_BOUND_ITEM_LIKE_CPP: i32 = InventoryResult::TradeBoundItem as i32;

/// How long the coordinator waits for its partner to park. The partner
/// parks between two of its own packets; a session that does not answer in
/// this time is treated like one that went away.
pub const TRADE_PARK_TIMEOUT_LIKE_CPP: Duration = Duration::from_secs(5);

pub type TradeFenceLikeCpp = Box<dyn Any + Send>;
pub type TradeHostFutureLikeCpp<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Session capabilities the exchange needs beyond the trade context.
pub trait TradeCommitHostLikeCpp: Send {
    fn trade_commit_cx_like_cpp(&mut self) -> TradeHandlerCxLikeCpp<'_>;

    /// `begin_exclusive_player_money_persistence_like_cpp` for this session.
    fn begin_trade_money_fence_like_cpp(
        &mut self,
    ) -> TradeHostFutureLikeCpp<'_, Option<TradeFenceLikeCpp>>;

    fn player_trade_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn PlayerTradePersistencePortLikeCpp>>;

    /// This session's money persistence tracker (indeterminate blocks every
    /// later full save of the Player).
    fn trade_money_tracker_like_cpp(&self) -> Arc<DurableLootMoneyPersistenceTrackerLikeCpp>;

    /// Publish this session's committed side: item objects and slots, money,
    /// `TRADE_STATUS_COMPLETE`, then `MoneyChanged` queued and drained.
    fn publish_trade_side_like_cpp(
        &mut self,
        side: TradeSidePublicationLikeCpp,
        item_guid_generator: Arc<wow_core::ObjectGuidGenerator>,
    ) -> TradeHostFutureLikeCpp<'_, ()>;

    /// A COMMIT whose outcome is unknown: no further money mutation and no
    /// play on this runtime state (the loot-money quarantine rule).
    fn quarantine_trade_like_cpp(&mut self, reason: &str);
}

/// Both sides' traded items and offers, frozen when the coordinator passed
/// the C++ checks and set the accept process.
#[derive(Debug, Clone)]
struct TradeCommitSnapshotLikeCpp {
    own_guid: ObjectGuid,
    partner_guid: ObjectGuid,
    same_bnet: bool,
    own_items: Vec<ObjectGuid>,
    partner_items: Vec<ObjectGuid>,
    own_offer: u64,
    partner_offer: u64,
}

impl TradeCommitSnapshotLikeCpp {
    /// The same exchange seen from the trader's side.
    fn mirrored(self) -> Self {
        Self {
            own_guid: self.partner_guid,
            partner_guid: self.own_guid,
            same_bnet: self.same_bnet,
            own_items: self.partner_items,
            partner_items: self.own_items,
            own_offer: self.partner_offer,
            partner_offer: self.own_offer,
        }
    }
}

/// A planned exchange, ready for the worker.
struct TradeCommitPlanLikeCpp {
    snapshot: TradeCommitSnapshotLikeCpp,
    own_receives: Vec<TradeStorePlanLikeCpp>,
    partner_receives: Vec<TradeStorePlanLikeCpp>,
    own_money_before: u64,
    partner_money_before: u64,
    request: PlayerTradePersistenceRequestLikeCpp,
}

fn status_bytes(status: TradeStatus, same_bnet: bool) -> Vec<u8> {
    status
        .with_partner_is_same_bnet_account(same_bnet)
        .to_bytes()
}

fn traded_items(player: &Player) -> Vec<ObjectGuid> {
    player
        .trade_state_snapshot_like_cpp()
        .map(|trade| {
            trade.items[..usize::from(TRADE_SLOT_TRADED_COUNT_LIKE_CPP)]
                .iter()
                .filter_map(|guid| *guid)
                .filter(|guid| player.trade_item_by_guid_like_cpp(*guid).is_some())
                .collect()
        })
        .unwrap_or_default()
}

/// The part of `Item::CanBeTraded(false, true)` a canonical Player answers
/// without its session (`Item.cpp:927-950`): loot, binding and bag state.
fn canonical_item_can_be_traded_like_cpp(
    player: &Player,
    item: &Item,
    container_slots: &dyn Fn(u32) -> u8,
) -> bool {
    if item.loot_generated() {
        return false;
    }
    if item.is_soul_bound() && !item.is_bop_tradeable() {
        return false;
    }
    if container_slots(item.object().entry()) > 0 {
        if item.bag_slot() == INVENTORY_SLOT_BAG_0
            && wow_world_inventory::is_represented_bag_slot(item.slot())
        {
            return false;
        }
        let guid = item.object().guid();
        if player
            .inventory_runtime_like_cpp()
            .item_objects()
            .values()
            .any(|child| child.container_guid() == guid)
        {
            return false;
        }
    }
    true
}

impl TradeHandlerCxLikeCpp<'_> {
    /// Steps 1 of the exchange: the C++ checks of `HandleAcceptTradeOpcode`
    /// up to `setAcceptTradeMode`. `Some` when both accepted and the accept
    /// process is now set on both `TradeData`.
    fn accept_checks_like_cpp(&mut self, state_index: u32) -> Option<TradeCommitSnapshotLikeCpp> {
        let own_trade = self.player_trade_state_snapshot_like_cpp().flatten()?;
        // `Item::CanBeTraded` and `IsBindedNotWith(trader)` for our own items
        // need this session's inventory owner, which takes the map lock
        // itself: evaluate them before the pair lock, in slot order.
        let own_item_checks: Vec<(bool, bool)> = own_trade.items
            [..usize::from(TRADE_SLOT_TRADED_COUNT_LIKE_CPP)]
            .iter()
            .filter_map(|guid| {
                let guid = (*guid)?;
                let item = self
                    .inventory
                    .resolved_inventory_item_object_like_cpp(self.hub.shared(), guid)?;
                let can_be_traded =
                    self.item_can_be_traded_like_cpp(&item, item.bag_slot(), item.slot());
                let bop_allowed = item.is_soulbound_trade_allowed_for(own_trade.partner_guid);
                let bound_not_with = self
                    .hub
                    .catalogs
                    .item_storage_template(item.object().entry())
                    .is_some_and(|template| {
                        item.is_binded_not_with(own_trade.partner_guid, &template, bop_allowed)
                    });
                Some((can_be_traded, bound_not_with))
            })
            .collect();
        let container_slots = {
            let catalogs = self.hub.catalogs;
            move |entry: u32| {
                catalogs
                    .item_storage_template(entry)
                    .map_or(0, |template| template.container_slots)
            }
        };

        self.run_trade_transition_like_cpp(|pair, cx, fx| {
            let partner_trade = pair
                .partner(|partner| partner.trade_state_snapshot_like_cpp())
                .flatten()?;
            // `my_trade->SetAccepted(true)` before every check.
            let _ = pair.own(|player| player.set_trade_accepted_like_cpp(true));
            if partner_trade.server_state_index != state_index {
                fx.packets.push((
                    TradeSideLikeCpp::Own,
                    status_bytes(
                        TradeStatus::status_only_like_cpp(TRADE_STATUS_STATE_CHANGED_LIKE_CPP),
                        cx.same_bnet,
                    ),
                ));
                set_accepted_like_cpp(pair, cx, TradeSideLikeCpp::Own, false, false, fx);
                return None;
            }
            let within = pair
                .read_both(|own, partner| {
                    own.unit().world().is_within_dist_in_map(
                        partner.unit().world(),
                        TRADE_DISTANCE_LIKE_CPP,
                        false,
                    )
                })
                .unwrap_or(false);
            if !within {
                fx.packets.push((
                    TradeSideLikeCpp::Own,
                    status_bytes(
                        TradeStatus::status_only_like_cpp(TRADE_STATUS_TOO_FAR_AWAY_LIKE_CPP),
                        cx.same_bnet,
                    ),
                ));
                set_accepted_like_cpp(pair, cx, TradeSideLikeCpp::Own, false, false, fx);
                return None;
            }
            let own_money = pair.own(|player| player.money())?;
            let partner_money = pair.partner(|player| player.money())?;
            let own_offer = own_trade.money;
            let partner_offer = partner_trade.money;
            let money_failure = |side: TradeSideLikeCpp, bag_result: i32| {
                (
                    side,
                    status_bytes(TradeStatus::failed_like_cpp(bag_result, 0), cx.same_bnet),
                )
            };
            if own_money < own_offer {
                fx.packets.push(money_failure(
                    TradeSideLikeCpp::Own,
                    EQUIP_ERR_NOT_ENOUGH_MONEY_LIKE_CPP,
                ));
                set_accepted_like_cpp(pair, cx, TradeSideLikeCpp::Own, false, true, fx);
                return None;
            }
            if partner_money < partner_offer {
                fx.packets.push(money_failure(
                    TradeSideLikeCpp::Partner,
                    EQUIP_ERR_NOT_ENOUGH_MONEY_LIKE_CPP,
                ));
                set_accepted_like_cpp(pair, cx, TradeSideLikeCpp::Partner, false, true, fx);
                return None;
            }
            if own_money > MAX_MONEY_AMOUNT.saturating_sub(partner_offer) {
                fx.packets.push(money_failure(
                    TradeSideLikeCpp::Own,
                    EQUIP_ERR_TOO_MUCH_GOLD_LIKE_CPP,
                ));
                set_accepted_like_cpp(pair, cx, TradeSideLikeCpp::Own, false, true, fx);
                return None;
            }
            if partner_money > MAX_MONEY_AMOUNT.saturating_sub(own_offer) {
                fx.packets.push(money_failure(
                    TradeSideLikeCpp::Partner,
                    EQUIP_ERR_TOO_MUCH_GOLD_LIKE_CPP,
                ));
                set_accepted_like_cpp(pair, cx, TradeSideLikeCpp::Partner, false, true, fx);
                return None;
            }

            // Item checks, slot by slot: ours, then the trader's.
            let partner_checks: Vec<bool> = pair
                .partner(|partner| {
                    partner_trade.items[..usize::from(TRADE_SLOT_TRADED_COUNT_LIKE_CPP)]
                        .iter()
                        .filter_map(|guid| partner.trade_item_by_guid_like_cpp((*guid)?))
                        .map(|item| {
                            canonical_item_can_be_traded_like_cpp(partner, item, &container_slots)
                        })
                        .collect()
                })
                .unwrap_or_default();
            let cancelled = status_bytes(
                TradeStatus::status_only_like_cpp(TRADE_STATUS_CANCELLED_LIKE_CPP),
                cx.same_bnet,
            );
            let slots = own_item_checks.len().max(partner_checks.len());
            for index in 0..slots {
                if let Some((can_be_traded, bound_not_with)) = own_item_checks.get(index) {
                    if !can_be_traded {
                        fx.packets.push((TradeSideLikeCpp::Own, cancelled.clone()));
                        return None;
                    }
                    if *bound_not_with {
                        fx.packets.push((
                            TradeSideLikeCpp::Own,
                            status_bytes(
                                TradeStatus::failed_like_cpp(
                                    EQUIP_ERR_TRADE_BOUND_ITEM_LIKE_CPP,
                                    0,
                                ),
                                cx.same_bnet,
                            ),
                        ));
                        return None;
                    }
                }
                if partner_checks.get(index) == Some(&false) {
                    fx.packets.push((TradeSideLikeCpp::Own, cancelled.clone()));
                    return None;
                }
            }

            let accepted =
                |status: u8| status_bytes(TradeStatus::status_only_like_cpp(status), cx.same_bnet);
            if !partner_trade.accepted {
                fx.packets.push((
                    TradeSideLikeCpp::Partner,
                    accepted(TRADE_STATUS_ACCEPTED_LIKE_CPP),
                ));
                return None;
            }
            // `setAcceptTradeMode`: a second accept while one exchange runs
            // (the trader accepting at the same moment) does nothing.
            if own_trade.in_accept_process || partner_trade.in_accept_process {
                return None;
            }
            let _ = pair.own(|player| player.set_trade_in_accept_process_like_cpp(true));
            let _ = pair.partner(|player| player.set_trade_in_accept_process_like_cpp(true));

            // The trade spells (`TradeHandler.cpp:373-436`). RustyCore has no
            // trade-item spell target to cast a proposed enchantment on, so a
            // set spell takes C++'s "can't be cast now" branch:
            // `clearAcceptTradeMode`, then `SetSpell(0)`.
            for side in [TradeSideLikeCpp::Own, TradeSideLikeCpp::Partner] {
                let spell = pair
                    .side(side, |player| {
                        player.trade_state_snapshot_like_cpp().map(|t| t.spell_id)
                    })
                    .flatten()
                    .unwrap_or(0);
                if spell != 0 {
                    let _ = pair.own(|player| player.set_trade_in_accept_process_like_cpp(false));
                    let _ =
                        pair.partner(|player| player.set_trade_in_accept_process_like_cpp(false));
                    set_spell_like_cpp(pair, cx, side, 0, None, fx);
                    return None;
                }
            }

            // "inform partner client".
            fx.packets.push((
                TradeSideLikeCpp::Partner,
                accepted(TRADE_STATUS_ACCEPTED_LIKE_CPP),
            ));
            let own_items = pair.own(|player| traded_items(player))?;
            let partner_items = pair.partner(|player| traded_items(player))?;
            Some(TradeCommitSnapshotLikeCpp {
                own_guid: pair.own_guid(),
                partner_guid: pair.partner_guid(),
                same_bnet: cx.same_bnet,
                own_items,
                partner_items,
                own_offer,
                partner_offer,
            })
        })
        .flatten()
    }

    /// Undo `setAcceptTradeMode` after an exchange that could not start, and
    /// reset both acceptances so either client can accept again.
    fn abandon_accept_process_like_cpp(&mut self) {
        let _ = self.run_trade_transition_like_cpp(|pair, cx, fx| {
            let _ = pair.own(|player| player.set_trade_in_accept_process_like_cpp(false));
            let _ = pair.partner(|player| player.set_trade_in_accept_process_like_cpp(false));
            set_accepted_like_cpp(pair, cx, TradeSideLikeCpp::Own, false, false, fx);
            set_accepted_like_cpp(pair, cx, TradeSideLikeCpp::Partner, false, false, fx);
        });
    }

    /// Step 3: `CanStoreItems` for both, on detached copies of both canonical
    /// Players. On a missing space C++ sends `TRADE_STATUS_FAILED` with the
    /// bag result and item to both (`FailureForYou` on the side that lacks
    /// space) and resets both acceptances.
    fn plan_trade_commit_like_cpp(
        &mut self,
        snapshot: TradeCommitSnapshotLikeCpp,
    ) -> Result<TradeCommitPlanLikeCpp, ()> {
        let catalogs = self.hub.catalogs;
        let template = move |entry: u32| catalogs.item_storage_template(entry);
        let limit_category = move |category: u32| {
            if category == 0 {
                return None;
            }
            catalogs
                .items
                .limit_category_store
                .as_ref()
                .and_then(|store| store.get(category))
                .map(|entry| wow_entities::ItemLimitCategoryTemplate {
                    id: entry.id,
                    quantity: entry.quantity,
                    flags: entry.flags,
                })
        };
        let store_catalogs = TradeStoreCatalogsLikeCpp {
            template: &template,
            limit_category: &limit_category,
        };

        let copies =
            self.hub.shared().core.with_trade_pair_like_cpp(|pair| {
                let own = pair.own(|player| player.clone())?;
                let partner = pair.partner(|player| player.clone())?;
                let still_open = own.trade_state_snapshot_like_cpp().is_some_and(|trade| {
                    trade.in_accept_process && trade.money == snapshot.own_offer
                }) && partner.trade_state_snapshot_like_cpp().is_some_and(
                    |trade| trade.in_accept_process && trade.money == snapshot.partner_offer,
                ) && traded_items(&own) == snapshot.own_items
                    && traded_items(&partner) == snapshot.partner_items;
                still_open.then_some((own, partner))
            });
        let Some(Some((own, partner))) = copies else {
            self.abandon_accept_process_like_cpp();
            return Err(());
        };
        let own_outgoing: Vec<Item> = snapshot
            .own_items
            .iter()
            .filter_map(|guid| own.trade_item_by_guid_like_cpp(*guid).cloned())
            .collect();
        let partner_outgoing: Vec<Item> = snapshot
            .partner_items
            .iter()
            .filter_map(|guid| partner.trade_item_by_guid_like_cpp(*guid).cloned())
            .collect();

        // C++ computes the trader's result first, then ours, and reports
        // ours first (`TradeHandler.cpp:445-485`).
        let partner_result = plan_trade_receive_like_cpp(
            &partner,
            &snapshot.partner_items,
            &own_outgoing,
            &store_catalogs,
        );
        let own_result = plan_trade_receive_like_cpp(
            &own,
            &snapshot.own_items,
            &partner_outgoing,
            &store_catalogs,
        );
        let failure = match (&own_result, &partner_result) {
            (Err((result, item)), _) => Some((TradeSideLikeCpp::Own, *result, *item)),
            (Ok(_), Err((result, item))) => Some((TradeSideLikeCpp::Partner, *result, *item)),
            _ => None,
        };
        if let Some((lacking, result, item)) = failure {
            let _ = self.run_trade_transition_like_cpp(|pair, cx, fx| {
                let _ = pair.own(|player| player.set_trade_in_accept_process_like_cpp(false));
                let _ = pair.partner(|player| player.set_trade_in_accept_process_like_cpp(false));
                let mut info = TradeStatus::failed_like_cpp(result as i32, item as i32);
                fx.packets
                    .push((lacking.other(), status_bytes(info, cx.same_bnet)));
                info.failure_for_you = true;
                fx.packets.push((lacking, status_bytes(info, cx.same_bnet)));
                set_accepted_like_cpp(pair, cx, TradeSideLikeCpp::Own, false, false, fx);
                set_accepted_like_cpp(pair, cx, TradeSideLikeCpp::Partner, false, false, fx);
            });
            return Err(());
        }
        let (Ok(own_receives), Ok(partner_receives)) = (own_result, partner_result) else {
            return Err(());
        };

        let own_money_before = own.money();
        let partner_money_before = partner.money();
        let request = trade_persistence_request_like_cpp(
            &snapshot,
            &own,
            &partner,
            &own_receives,
            &partner_receives,
            own_money_before,
            partner_money_before,
        );
        Ok(TradeCommitPlanLikeCpp {
            snapshot,
            own_receives,
            partner_receives,
            own_money_before,
            partner_money_before,
            request,
        })
    }
}

fn money_after_like_cpp(before: u64, gives: u64, receives: u64) -> u64 {
    before.saturating_sub(gives).saturating_add(receives)
}

fn item_db_guid(guid: ObjectGuid) -> u64 {
    guid.counter() as u64
}

fn bag_db_guid(receiver: &Player, bag: u8) -> u64 {
    if bag == INVENTORY_SLOT_BAG_0 {
        return 0;
    }
    // The runtime record is the production authority for the bag item.
    receiver
        .trade_bag_guid_like_cpp(bag)
        .map_or(0, item_db_guid)
}

fn trade_persistence_request_like_cpp(
    snapshot: &TradeCommitSnapshotLikeCpp,
    own: &Player,
    partner: &Player,
    own_receives: &[TradeStorePlanLikeCpp],
    partner_receives: &[TradeStorePlanLikeCpp],
    own_money_before: u64,
    partner_money_before: u64,
) -> PlayerTradePersistenceRequestLikeCpp {
    let mut items = Vec::new();
    let mut push_side = |giver: &Player, receiver: &Player, plans: &[TradeStorePlanLikeCpp]| {
        for plan in plans {
            let Some(item) = giver.trade_item_by_guid_like_cpp(plan.item_guid) else {
                continue;
            };
            for action in &plan.actions {
                match action {
                    TradeStoreActionLikeCpp::Merge {
                        target_guid,
                        new_count,
                        ..
                    } => items.push(PlayerTradeItemPersistenceLikeCpp::MergeInto {
                        item_guid: item_db_guid(*target_guid),
                        new_count: *new_count,
                    }),
                    TradeStoreActionLikeCpp::Store { bag, slot, count } => {
                        items.push(PlayerTradeItemPersistenceLikeCpp::Move {
                            item_guid: item_db_guid(plan.item_guid),
                            from_owner_guid: item_db_guid(giver.guid()),
                            to_owner_guid: item_db_guid(receiver.guid()),
                            bag_guid: bag_db_guid(receiver, *bag),
                            slot: *slot,
                            new_count: (*count != item.count()).then_some(*count),
                            wrapped: item.is_wrapped(),
                        });
                    }
                }
            }
            if plan.fully_merged() {
                items.push(PlayerTradeItemPersistenceLikeCpp::FullyMerged {
                    item_guid: item_db_guid(plan.item_guid),
                    from_owner_guid: item_db_guid(giver.guid()),
                });
            }
        }
    };
    // `SaveInventoryAndGoldToDB` for us, then for the trader.
    push_side(partner, own, own_receives);
    push_side(own, partner, partner_receives);
    PlayerTradePersistenceRequestLikeCpp {
        money: vec![
            PlayerTradeMoneyPersistenceLikeCpp {
                player_guid: item_db_guid(snapshot.own_guid),
                money_before: own_money_before,
                money_after: money_after_like_cpp(
                    own_money_before,
                    snapshot.own_offer,
                    snapshot.partner_offer,
                ),
            },
            PlayerTradeMoneyPersistenceLikeCpp {
                player_guid: item_db_guid(snapshot.partner_guid),
                money_before: partner_money_before,
                money_after: money_after_like_cpp(
                    partner_money_before,
                    snapshot.partner_offer,
                    snapshot.own_offer,
                ),
            },
        ],
        items,
    }
}

/// Packets of an exchange that never reached its COMMIT.
fn aborted_side_like_cpp() -> TradeSideOutcomeLikeCpp {
    TradeSideOutcomeLikeCpp::Aborted {
        packets: Vec::new(),
    }
}

async fn run_trade_commit_worker_like_cpp(
    port: Arc<dyn PlayerTradePersistencePortLikeCpp>,
    core_apply: impl FnOnce(
        &TradeCommitPlanLikeCpp,
    ) -> Option<(TradeSidePublicationLikeCpp, TradeSidePublicationLikeCpp)>
    + Send,
    plan: TradeCommitPlanLikeCpp,
    own_fence: TradeFenceLikeCpp,
    own_tracker: Arc<DurableLootMoneyPersistenceTrackerLikeCpp>,
    partner: TradeCommitParkedLikeCpp,
) -> TradeSideOutcomeLikeCpp {
    // From the COMMIT attempt on, an outcome nobody can apply (unknown,
    // unapplied, or this task unwinding) must close both Players' saves
    // before their fences are released. Declared after the parameters, the
    // guard drops first while unwinding: the trackers are marked before the
    // coordinator's outcome sender closes.
    let mut attempt = CommitAttemptGuardLikeCpp {
        trackers: [Arc::clone(&own_tracker), Arc::clone(&partner.money_tracker)],
        armed: true,
    };
    let outcome = port
        .persist_player_trade_like_cpp(plan.request.clone())
        .await;
    let committed = match outcome {
        PlayerTradePersistenceOutcomeLikeCpp::Committed => Ok(true),
        PlayerTradePersistenceOutcomeLikeCpp::DefinitelyRolledBack { reason } => {
            tracing::warn!(error = %reason, "player trade transaction definitely rolled back");
            Ok(false)
        }
        PlayerTradePersistenceOutcomeLikeCpp::CommitOutcomeUnknown {
            reason,
            observed_money,
        } => match classify_player_trade_reconciliation_like_cpp(&plan.request, &observed_money) {
            PlayerTradeReconciliationLikeCpp::Committed => Ok(true),
            PlayerTradeReconciliationLikeCpp::RolledBack => Ok(false),
            PlayerTradeReconciliationLikeCpp::Indeterminate => Err(reason),
        },
    };
    let (own_outcome, partner_outcome) = match committed {
        Ok(true) => match core_apply(&plan) {
            Some((own, partner)) => (
                TradeSideOutcomeLikeCpp::Committed(own),
                TradeSideOutcomeLikeCpp::Committed(partner),
            ),
            None => {
                let reason = "trade committed but a canonical Player became unavailable".to_owned();
                (
                    TradeSideOutcomeLikeCpp::Unknown {
                        reason: reason.clone(),
                    },
                    TradeSideOutcomeLikeCpp::Unknown { reason },
                )
            }
        },
        Ok(false) => (aborted_side_like_cpp(), aborted_side_like_cpp()),
        Err(reason) => (
            TradeSideOutcomeLikeCpp::Unknown {
                reason: reason.clone(),
            },
            TradeSideOutcomeLikeCpp::Unknown { reason },
        ),
    };
    if !matches!(own_outcome, TradeSideOutcomeLikeCpp::Unknown { .. }) {
        attempt.armed = false;
    }
    drop(attempt);
    let _ = partner.outcome_tx.send(partner_outcome);
    drop(partner.money_fence);
    drop(own_fence);
    own_outcome
}

/// Marks both trackers indeterminate unless the worker reached a known,
/// applied (or definitely rolled back) outcome.
struct CommitAttemptGuardLikeCpp {
    trackers: [Arc<DurableLootMoneyPersistenceTrackerLikeCpp>; 2],
    armed: bool,
}

impl Drop for CommitAttemptGuardLikeCpp {
    fn drop(&mut self) {
        if self.armed {
            for tracker in &self.trackers {
                tracker.mark_indeterminate_like_cpp();
            }
        }
    }
}

/// `CMSG_ACCEPT_TRADE` (C++ `HandleAcceptTradeOpcode`).
pub async fn accept_trade_like_cpp<H: TradeCommitHostLikeCpp>(
    host: &mut H,
    state_index: u32,
    item_guid_generator: Arc<wow_core::ObjectGuidGenerator>,
) {
    let _ = item_guid_generator;
    let Some(snapshot) = host
        .trade_commit_cx_like_cpp()
        .accept_checks_like_cpp(state_index)
    else {
        return;
    };

    let Some(own_fence) = host.begin_trade_money_fence_like_cpp().await else {
        host.trade_commit_cx_like_cpp()
            .abandon_accept_process_like_cpp();
        return;
    };
    // A world step waits for every session's pass, so this pass must not wait
    // for the partner's: hand the fence and the snapshot over and return.
    let partner_guid = snapshot.partner_guid;
    let (outcome_tx, outcome_rx) = tokio::sync::oneshot::channel();
    let request = TradeCommitRequestLikeCpp::new(
        snapshot.own_guid,
        TradeCommitParkedLikeCpp {
            money_fence: own_fence,
            money_tracker: host.trade_money_tracker_like_cpp(),
            snapshot: Box::new(snapshot.mirrored()),
            outcome_tx,
        },
    );
    host.trade_commit_cx_like_cpp()
        .hub
        .shared()
        .core
        .try_send_connected_player_command_like_cpp(
            partner_guid,
            SessionCommand::TradeCommitLikeCpp(request.clone()),
        );
    let pending = PendingTradeCommitLikeCpp {
        request,
        outcome_rx,
        deadline: std::time::Instant::now() + TRADE_PARK_TIMEOUT_LIKE_CPP,
    };
    if let Ok(slot) = host
        .trade_commit_cx_like_cpp()
        .social
        .pending_trade_commit_like_cpp
        .get_mut()
    {
        *slot = Some(pending);
    }
}

/// A coordinator's exchange handed to its partner (#38).
pub struct PendingTradeCommitLikeCpp {
    request: TradeCommitRequestLikeCpp,
    outcome_rx: tokio::sync::oneshot::Receiver<TradeSideOutcomeLikeCpp>,
    deadline: std::time::Instant,
}

/// Whether this session handed an exchange over and waits for its outcome.
/// Like C++, where `HandleAcceptTradeOpcode` returns only after the swap, the
/// session dispatches none of its own packets meanwhile.
#[must_use]
pub fn trade_commit_pending_like_cpp(social: &crate::SessionSocialLimits) -> bool {
    social
        .pending_trade_commit_like_cpp
        .lock()
        .is_ok_and(|pending| pending.is_some())
}

/// The coordinator's second half, run at the head of each pass while an
/// exchange is pending. It never waits: the outcome is taken when the
/// executor's worker has sent it, and a handover nobody claimed by the
/// deadline is revoked.
pub async fn continue_pending_trade_commit_like_cpp<H: TradeCommitHostLikeCpp>(
    host: &mut H,
    item_guid_generator: Arc<wow_core::ObjectGuidGenerator>,
) {
    let own_tracker = host.trade_money_tracker_like_cpp();
    let outcome = {
        let mut cx = host.trade_commit_cx_like_cpp();
        let Ok(slot) = cx.social.pending_trade_commit_like_cpp.get_mut() else {
            return;
        };
        let Some(pending) = slot.as_mut() else {
            return;
        };
        match pending.outcome_rx.try_recv() {
            Ok(outcome) => {
                *slot = None;
                outcome
            }
            Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                *slot = None;
                // No outcome was sent. Before a COMMIT attempt nothing was
                // persisted; after one the worker's guard already marked this
                // session's tracker, and the session must not keep playing.
                if own_tracker.is_indeterminate_like_cpp() {
                    TradeSideOutcomeLikeCpp::Unknown {
                        reason:
                            "player trade worker ended without an outcome after its COMMIT attempt"
                                .to_owned(),
                    }
                } else {
                    aborted_side_like_cpp()
                }
            }
            Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {
                if std::time::Instant::now() < pending.deadline {
                    return;
                }
                // Revoking returns the fence; a claimed handover always ends
                // with an outcome, so keep waiting for it.
                let Some(handover) = pending.request.claim() else {
                    return;
                };
                drop(handover);
                *slot = None;
                drop(cx);
                host.trade_commit_cx_like_cpp()
                    .abandon_accept_process_like_cpp();
                return;
            }
        }
    };
    let rolled_back = matches!(outcome, TradeSideOutcomeLikeCpp::Aborted { .. });
    apply_trade_side_outcome_like_cpp(host, outcome, item_guid_generator).await;
    if rolled_back {
        host.trade_commit_cx_like_cpp()
            .abandon_accept_process_like_cpp();
    }
}

/// The partner half (`SessionCommand::TradeCommitLikeCpp`): claim the
/// coordinator's handover, take this session's fence, then plan, persist and
/// apply the exchange. This pass waits only for the detached worker, which
/// depends on no session pass.
pub async fn park_for_trade_commit_like_cpp<H: TradeCommitHostLikeCpp>(
    host: &mut H,
    request: TradeCommitRequestLikeCpp,
    item_guid_generator: Arc<wow_core::ObjectGuidGenerator>,
) {
    let Some(mut coordinator) = request.claim() else {
        return;
    };
    let snapshot = match std::mem::replace(&mut coordinator.snapshot, Box::new(()))
        .downcast::<TradeCommitSnapshotLikeCpp>()
    {
        Ok(snapshot) => *snapshot,
        Err(_) => {
            let _ = coordinator.outcome_tx.send(aborted_side_like_cpp());
            return;
        }
    };
    let Some(own_fence) = host.begin_trade_money_fence_like_cpp().await else {
        let _ = coordinator.outcome_tx.send(aborted_side_like_cpp());
        return;
    };

    let plan = match host
        .trade_commit_cx_like_cpp()
        .plan_trade_commit_like_cpp(snapshot)
    {
        Ok(plan) => plan,
        Err(()) => {
            let _ = coordinator.outcome_tx.send(aborted_side_like_cpp());
            return;
        }
    };
    let Some(port) = host.player_trade_persistence_port_like_cpp() else {
        let _ = coordinator.outcome_tx.send(aborted_side_like_cpp());
        drop(own_fence);
        host.trade_commit_cx_like_cpp()
            .abandon_accept_process_like_cpp();
        return;
    };
    let target = host
        .trade_commit_cx_like_cpp()
        .hub
        .shared()
        .core
        .trade_commit_target_like_cpp();
    let Some(target) = target else {
        let _ = coordinator.outcome_tx.send(aborted_side_like_cpp());
        host.trade_commit_cx_like_cpp()
            .abandon_accept_process_like_cpp();
        return;
    };
    let worker = tokio::spawn(run_trade_commit_worker_like_cpp(
        port,
        move |plan| apply_committed_trade_on_target_like_cpp(&target, plan),
        plan,
        own_fence,
        host.trade_money_tracker_like_cpp(),
        coordinator,
    ));
    let outcome = worker
        .await
        .unwrap_or_else(|_| TradeSideOutcomeLikeCpp::Unknown {
            reason: "player trade worker terminated".to_owned(),
        });
    let rolled_back = matches!(outcome, TradeSideOutcomeLikeCpp::Aborted { .. });
    apply_trade_side_outcome_like_cpp(host, outcome, item_guid_generator).await;
    if rolled_back {
        // Nothing changed: the trade stays open with both acceptances reset.
        host.trade_commit_cx_like_cpp()
            .abandon_accept_process_like_cpp();
    }
}

async fn apply_trade_side_outcome_like_cpp<H: TradeCommitHostLikeCpp>(
    host: &mut H,
    outcome: TradeSideOutcomeLikeCpp,
    item_guid_generator: Arc<wow_core::ObjectGuidGenerator>,
) {
    match outcome {
        TradeSideOutcomeLikeCpp::Committed(side) => {
            host.publish_trade_side_like_cpp(side, item_guid_generator)
                .await;
        }
        TradeSideOutcomeLikeCpp::Aborted { packets } => {
            let cx = host.trade_commit_cx_like_cpp();
            for packet in packets {
                cx.hub.shared().core.send_raw_packet(&packet);
            }
        }
        TradeSideOutcomeLikeCpp::Unknown { reason } => host.quarantine_trade_like_cpp(&reason),
    }
}

fn apply_committed_trade_on_target_like_cpp(
    target: &TradeCommitTargetLikeCpp,
    plan: &TradeCommitPlanLikeCpp,
) -> Option<(TradeSidePublicationLikeCpp, TradeSidePublicationLikeCpp)> {
    let snapshot = &plan.snapshot;
    target
        .with_pair_like_cpp(snapshot.partner_guid, |pair| {
            // `MoveItemFromInventory` for both sides first
            // (`TradeHandler.cpp:470-482`).
            let own_taken = pair.own(|own| take_items_like_cpp(own, &snapshot.own_items))?;
            let partner_taken =
                pair.partner(|partner| take_items_like_cpp(partner, &snapshot.partner_items))?;
            let complete = status_bytes(
                TradeStatus::status_only_like_cpp(TRADE_STATUS_COMPLETE_LIKE_CPP),
                snapshot.same_bnet,
            );
            let own_money_after = money_after_like_cpp(
                plan.own_money_before,
                snapshot.own_offer,
                snapshot.partner_offer,
            );
            let partner_money_after = money_after_like_cpp(
                plan.partner_money_before,
                snapshot.partner_offer,
                snapshot.own_offer,
            );
            let own_side = pair.own(|own| {
                let (stored, merged) =
                    store_items_like_cpp(own, &partner_taken, &plan.own_receives);
                own.set_money(own_money_after);
                own.clear_trade_like_cpp();
                TradeSidePublicationLikeCpp {
                    removed: removed_positions(&own_taken),
                    stored,
                    merged,
                    money_before: plan.own_money_before,
                    money_after: own_money_after,
                    complete_packet: complete.clone(),
                }
            })?;
            let partner_side = pair.partner(|partner| {
                let (stored, merged) =
                    store_items_like_cpp(partner, &own_taken, &plan.partner_receives);
                partner.set_money(partner_money_after);
                partner.clear_trade_like_cpp();
                TradeSidePublicationLikeCpp {
                    removed: removed_positions(&partner_taken),
                    stored,
                    merged,
                    money_before: plan.partner_money_before,
                    money_after: partner_money_after,
                    complete_packet: complete.clone(),
                }
            })?;
            Some((own_side, partner_side))
        })
        .flatten()
}

fn removed_positions(taken: &[TradeTakenItemLikeCpp]) -> Vec<(ObjectGuid, u8, u8)> {
    taken
        .iter()
        .map(|taken| (taken.item.object().guid(), taken.bag, taken.slot))
        .collect()
}

fn take_items_like_cpp(giver: &mut Player, items: &[ObjectGuid]) -> Vec<TradeTakenItemLikeCpp> {
    let giver_guid = giver.guid();
    items
        .iter()
        .filter_map(|guid| {
            let mut taken = giver.trade_take_item_like_cpp(*guid)?;
            // `myItems[i]->SetGiftCreator(_player->GetGUID())`.
            taken.item.set_gift_creator(giver_guid);
            Some(taken)
        })
        .collect()
}

fn store_items_like_cpp(
    receiver: &mut Player,
    incoming: &[TradeTakenItemLikeCpp],
    plans: &[TradeStorePlanLikeCpp],
) -> (Vec<Item>, Vec<(ObjectGuid, u32)>) {
    let mut stored = Vec::new();
    let mut merged = Vec::new();
    for plan in plans {
        let Some(taken) = incoming
            .iter()
            .find(|taken| taken.item.object().guid() == plan.item_guid)
        else {
            continue;
        };
        for action in &plan.actions {
            match *action {
                TradeStoreActionLikeCpp::Merge {
                    target_guid,
                    new_count,
                    ..
                } => {
                    if receiver.trade_set_stack_count_like_cpp(target_guid, new_count) {
                        merged.push((target_guid, new_count));
                    }
                }
                TradeStoreActionLikeCpp::Store { bag, slot, count } => {
                    let mut item = taken.item.clone();
                    item.set_count(count);
                    let record = taken.record.clone();
                    let db_guid = record
                        .as_ref()
                        .map_or(item_db_guid(plan.item_guid), |record| record.db_guid);
                    let inventory_type = record.and_then(|record| record.inventory_type);
                    if receiver.trade_store_item_like_cpp(item, db_guid, inventory_type, bag, slot)
                        && let Some(item) = receiver.trade_item_at_like_cpp(bag, slot)
                    {
                        stored.push(item.clone());
                    }
                }
            }
        }
    }
    (stored, merged)
}

/// Test hook: move the pending handover's revoke deadline to now.
#[cfg(any(test, feature = "test-fixtures"))]
pub fn expire_pending_trade_commit_for_test_like_cpp(social: &mut crate::SessionSocialLimits) {
    if let Ok(Some(pending)) = social
        .pending_trade_commit_like_cpp
        .get_mut()
        .map(Option::as_mut)
    {
        pending.deadline = std::time::Instant::now();
    }
}

/// Test hook: the coordinator's session goes away with its exchange handed
/// over (disconnect or a cancelled pass future drops the pending state).
#[cfg(any(test, feature = "test-fixtures"))]
pub fn drop_pending_trade_commit_for_test_like_cpp(social: &mut crate::SessionSocialLimits) {
    if let Ok(slot) = social.pending_trade_commit_like_cpp.get_mut() {
        *slot = None;
    }
}
