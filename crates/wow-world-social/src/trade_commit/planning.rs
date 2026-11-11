use std::collections::HashMap;

use wow_core::ObjectGuid;
use wow_entities::{INVENTORY_SLOT_BAG_0, Item, Player};
use wow_packet::packets::misc::TradeStatus;
use wow_persistence::{
    PlayerTradeItemPersistenceLikeCpp, PlayerTradeMoneyPersistenceLikeCpp,
    PlayerTradePersistenceRequestLikeCpp,
};
use wow_world_core::session::TradeSideLikeCpp;
use wow_world_inventory::{
    TradeStorageSnapshot, TradeStoreActionLikeCpp, TradeStoreCatalogsLikeCpp,
    TradeStorePlanLikeCpp, plan_trade_receive_snapshot_like_cpp,
};

use crate::TradeHandlerCxLikeCpp;
use crate::trade_contents::set_accepted_like_cpp;

use super::{TradeCommitPlanLikeCpp, TradeCommitSnapshotLikeCpp, status_bytes, traded_items};

/// Original canonical facts used only to project the SQL request.
struct TradeSideSqlFactsLikeCpp {
    guid: ObjectGuid,
    money: u64,
    outgoing: Vec<Item>,
    bag_guids: HashMap<u8, ObjectGuid>,
}

struct TradePlanningCaptureLikeCpp {
    own_storage: TradeStorageSnapshot,
    partner_storage: TradeStorageSnapshot,
    own_sql: TradeSideSqlFactsLikeCpp,
    partner_sql: TradeSideSqlFactsLikeCpp,
}

fn capture_side_sql_facts_like_cpp(
    player: &Player,
    outgoing: &[ObjectGuid],
) -> TradeSideSqlFactsLikeCpp {
    let mut bag_guids: HashMap<u8, ObjectGuid> = player
        .inventory()
        .bags
        .iter()
        .enumerate()
        .filter_map(|(slot, storage)| {
            storage
                .as_ref()
                .map(|storage| (slot as u8, storage.bag_guid))
        })
        .collect();
    // Runtime records override native entries, as `trade_bag_guid_like_cpp` does.
    for (&slot, record) in player.inventory_runtime_like_cpp().inventory_items() {
        bag_guids.insert(slot, record.guid);
    }
    TradeSideSqlFactsLikeCpp {
        guid: player.guid(),
        money: player.money(),
        outgoing: outgoing
            .iter()
            .filter_map(|guid| player.trade_item_by_guid_like_cpp(*guid).cloned())
            .collect(),
        bag_guids,
    }
}

fn capture_trade_planning_like_cpp(
    own: &Player,
    partner: &Player,
    snapshot: &TradeCommitSnapshotLikeCpp,
) -> Option<TradePlanningCaptureLikeCpp> {
    let still_open = own
        .trade_state_snapshot_like_cpp()
        .is_some_and(|trade| trade.in_accept_process && trade.money == snapshot.own_offer)
        && partner
            .trade_state_snapshot_like_cpp()
            .is_some_and(|trade| trade.in_accept_process && trade.money == snapshot.partner_offer)
        && traded_items(own) == snapshot.own_items
        && traded_items(partner) == snapshot.partner_items;
    if !still_open {
        return None;
    }
    Some(TradePlanningCaptureLikeCpp {
        own_storage: TradeStorageSnapshot::capture(own),
        partner_storage: TradeStorageSnapshot::capture(partner),
        own_sql: capture_side_sql_facts_like_cpp(own, &snapshot.own_items),
        partner_sql: capture_side_sql_facts_like_cpp(partner, &snapshot.partner_items),
    })
}

impl TradeHandlerCxLikeCpp<'_> {
    /// Plan both disposable inventories with the represented sequential
    /// storage queries. The existing failure transition resets acceptance.
    pub(super) fn plan_trade_commit_like_cpp(
        &mut self,
        snapshot: TradeCommitSnapshotLikeCpp,
    ) -> Result<TradeCommitPlanLikeCpp, ()> {
        let captured = self.hub.shared().core.with_trade_pair_like_cpp(|pair| {
            pair.read_both(|own, partner| capture_trade_planning_like_cpp(own, partner, &snapshot))
                .flatten()
        });
        let Some(Some(captured)) = captured else {
            self.abandon_accept_process_like_cpp();
            return Err(());
        };
        let TradePlanningCaptureLikeCpp {
            own_storage,
            partner_storage,
            own_sql,
            partner_sql,
        } = captured;

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

        // C++ computes the trader's result first, then ours, and reports
        // ours first (`TradeHandler.cpp:445-485`).
        let partner_result = plan_trade_receive_snapshot_like_cpp(
            partner_storage,
            &snapshot.partner_items,
            &own_sql.outgoing,
            &store_catalogs,
        );
        let own_result = plan_trade_receive_snapshot_like_cpp(
            own_storage,
            &snapshot.own_items,
            &partner_sql.outgoing,
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

        let own_money_before = own_sql.money;
        let partner_money_before = partner_sql.money;
        let request = trade_persistence_request_like_cpp(
            &snapshot,
            &own_sql,
            &partner_sql,
            &own_receives,
            &partner_receives,
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

pub(super) fn money_after_like_cpp(before: u64, gives: u64, receives: u64) -> u64 {
    before.saturating_sub(gives).saturating_add(receives)
}

pub(super) fn item_db_guid(guid: ObjectGuid) -> u64 {
    guid.counter() as u64
}

fn bag_db_guid(receiver: &TradeSideSqlFactsLikeCpp, bag: u8) -> u64 {
    if bag == INVENTORY_SLOT_BAG_0 {
        return 0;
    }
    receiver
        .bag_guids
        .get(&bag)
        .copied()
        .map_or(0, item_db_guid)
}

fn trade_persistence_request_like_cpp(
    snapshot: &TradeCommitSnapshotLikeCpp,
    own: &TradeSideSqlFactsLikeCpp,
    partner: &TradeSideSqlFactsLikeCpp,
    own_receives: &[TradeStorePlanLikeCpp],
    partner_receives: &[TradeStorePlanLikeCpp],
) -> PlayerTradePersistenceRequestLikeCpp {
    let mut items = Vec::new();
    let mut push_side = |giver: &TradeSideSqlFactsLikeCpp,
                         receiver: &TradeSideSqlFactsLikeCpp,
                         plans: &[TradeStorePlanLikeCpp]| {
        for plan in plans {
            let Some(item) = giver
                .outgoing
                .iter()
                .find(|item| item.object().guid() == plan.item_guid)
            else {
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
                            from_owner_guid: item_db_guid(giver.guid),
                            to_owner_guid: item_db_guid(receiver.guid),
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
                    from_owner_guid: item_db_guid(giver.guid),
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
                money_before: own.money,
                money_after: money_after_like_cpp(
                    own.money,
                    snapshot.own_offer,
                    snapshot.partner_offer,
                ),
            },
            PlayerTradeMoneyPersistenceLikeCpp {
                player_guid: item_db_guid(snapshot.partner_guid),
                money_before: partner.money,
                money_after: money_after_like_cpp(
                    partner.money,
                    snapshot.partner_offer,
                    snapshot.own_offer,
                ),
            },
        ],
        items,
    }
}

#[cfg(test)]
#[path = "../../unit_tests/trade_commit/planning.rs"]
mod tests;
