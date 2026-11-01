//! SQLx-free persistence contract for one completed player trade (#38).
//!
//! C++ `WorldSession::HandleAcceptTradeOpcode` (`TradeHandler.cpp:503-507`)
//! saves both participants with `SaveInventoryAndGoldToDB` in one Character
//! transaction. Rust writes exactly the rows the trade changed — both money
//! rows, every moved item's owner and inventory link, merged stack counts and
//! fully merged item rows — in one transaction with one COMMIT.

use crate::PersistenceFutureLikeCpp;

/// One participant's absolute money write; `before` is the runtime value the
/// exclusive money fence guaranteed when the trade was planned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerTradeMoneyPersistenceLikeCpp {
    pub player_guid: u64,
    pub money_before: u64,
    pub money_after: u64,
}

/// One item effect of `moveItems` (`TradeHandler.cpp:110-190`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerTradeItemPersistenceLikeCpp {
    /// The traded item keeps its row and changes owner and position
    /// (`MoveItemFromInventory` + `MoveItemToInventory` storing it).
    Move {
        item_guid: u64,
        from_owner_guid: u64,
        to_owner_guid: u64,
        bag_guid: u64,
        slot: u8,
        /// Present when part of the stack merged elsewhere first and the
        /// stored item keeps only the rest.
        new_count: Option<u32>,
        /// `ITEM_FIELD_FLAG_WRAPPED`: `character_gifts` follows the owner.
        wrapped: bool,
    },
    /// Part of a traded stack merged into an existing stack of the receiver.
    MergeInto { item_guid: u64, new_count: u32 },
    /// The traded item merged completely and its row is deleted.
    FullyMerged {
        item_guid: u64,
        from_owner_guid: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerTradePersistenceRequestLikeCpp {
    pub money: Vec<PlayerTradeMoneyPersistenceLikeCpp>,
    pub items: Vec<PlayerTradeItemPersistenceLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerTradePersistenceOutcomeLikeCpp {
    Committed,
    DefinitelyRolledBack {
        reason: String,
    },
    /// The COMMIT reply was lost. `observed_money` holds each changed money
    /// row read back afterwards (`None` when it could not be read).
    CommitOutcomeUnknown {
        reason: String,
        observed_money: Vec<(u64, Option<u64>)>,
    },
}

/// Classification of a lost COMMIT from the money rows read back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerTradeReconciliationLikeCpp {
    Committed,
    RolledBack,
    Indeterminate,
}

/// All changed money rows at `after`: committed; all at `before`: rolled
/// back; anything else (mixed, unreadable, or no money row changed, so
/// nothing proves either side): indeterminate.
#[must_use]
pub fn classify_player_trade_reconciliation_like_cpp(
    request: &PlayerTradePersistenceRequestLikeCpp,
    observed_money: &[(u64, Option<u64>)],
) -> PlayerTradeReconciliationLikeCpp {
    let changed: Vec<_> = request
        .money
        .iter()
        .filter(|money| money.money_before != money.money_after)
        .collect();
    if changed.is_empty() {
        return PlayerTradeReconciliationLikeCpp::Indeterminate;
    }
    let observed_of = |guid: u64| {
        observed_money
            .iter()
            .find(|(observed_guid, _)| *observed_guid == guid)
            .and_then(|(_, money)| *money)
    };
    if changed
        .iter()
        .all(|money| observed_of(money.player_guid) == Some(money.money_after))
    {
        PlayerTradeReconciliationLikeCpp::Committed
    } else if changed
        .iter()
        .all(|money| observed_of(money.player_guid) == Some(money.money_before))
    {
        PlayerTradeReconciliationLikeCpp::RolledBack
    } else {
        PlayerTradeReconciliationLikeCpp::Indeterminate
    }
}

pub trait PlayerTradePersistencePortLikeCpp: Send + Sync {
    fn persist_player_trade_like_cpp(
        &self,
        request: PlayerTradePersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PlayerTradePersistenceOutcomeLikeCpp>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(before: [u64; 2], after: [u64; 2]) -> PlayerTradePersistenceRequestLikeCpp {
        PlayerTradePersistenceRequestLikeCpp {
            money: vec![
                PlayerTradeMoneyPersistenceLikeCpp {
                    player_guid: 1,
                    money_before: before[0],
                    money_after: after[0],
                },
                PlayerTradeMoneyPersistenceLikeCpp {
                    player_guid: 2,
                    money_before: before[1],
                    money_after: after[1],
                },
            ],
            items: Vec::new(),
        }
    }

    #[test]
    fn lost_commit_is_classified_from_the_changed_money_rows() {
        let trade = request([100, 0], [40, 60]);
        assert_eq!(
            classify_player_trade_reconciliation_like_cpp(&trade, &[(1, Some(40)), (2, Some(60))]),
            PlayerTradeReconciliationLikeCpp::Committed
        );
        assert_eq!(
            classify_player_trade_reconciliation_like_cpp(&trade, &[(1, Some(100)), (2, Some(0))]),
            PlayerTradeReconciliationLikeCpp::RolledBack
        );
        assert_eq!(
            classify_player_trade_reconciliation_like_cpp(&trade, &[(1, Some(40)), (2, Some(0))]),
            PlayerTradeReconciliationLikeCpp::Indeterminate
        );
        assert_eq!(
            classify_player_trade_reconciliation_like_cpp(&trade, &[(1, None), (2, Some(60))]),
            PlayerTradeReconciliationLikeCpp::Indeterminate
        );
        // An items-only trade has no money row to prove either outcome.
        assert_eq!(
            classify_player_trade_reconciliation_like_cpp(&request([5, 5], [5, 5]), &[]),
            PlayerTradeReconciliationLikeCpp::Indeterminate
        );
    }
}
