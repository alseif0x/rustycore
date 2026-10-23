//! The partner half of a completing trade (#38).
//!
//! C++ runs `HandleAcceptTradeOpcode` synchronously for both Players: while it
//! moves items and money nothing else touches either inventory. Rust sessions
//! run concurrently, and a world step waits for every session's pass, so no
//! session may wait inside its pass for another session's pass. The second
//! accepter (the coordinator) therefore hands its exclusive money fence and
//! its frozen snapshot to the partner in [`TradeCommitRequestLikeCpp`] and
//! returns; it dispatches none of its own packets until its outcome arrives.
//! The partner (the executor) claims the handover when it drains the command,
//! takes its own fence, plans, and waits only for the detached worker (one
//! COMMIT plus the runtime application), which sends the coordinator its
//! outcome. A coordinator whose request was never claimed revokes it.

use std::sync::{Arc, Mutex};

use wow_core::ObjectGuid;
use wow_entities::Item;

/// What one participant's session publishes after a committed trade.
#[derive(Debug, Clone, PartialEq)]
pub struct TradeSidePublicationLikeCpp {
    /// Items that left this Player: (guid, bag, slot).
    pub removed: Vec<(ObjectGuid, u8, u8)>,
    /// Items stored for this Player, as stored (bag, slot inside `item`).
    pub stored: Vec<Item>,
    /// Existing stacks of this Player that grew: (guid, new count).
    pub merged: Vec<(ObjectGuid, u32)>,
    pub money_before: u64,
    pub money_after: u64,
    /// `TRADE_STATUS_COMPLETE` with this side's `PartnerIsSameBnetAccount`.
    pub complete_packet: Vec<u8>,
}

/// The coordinator's verdict for one participant.
#[derive(Debug, Clone, PartialEq)]
pub enum TradeSideOutcomeLikeCpp {
    Committed(TradeSidePublicationLikeCpp),
    /// Nothing changed; send these trade packets in order.
    Aborted {
        packets: Vec<Vec<u8>>,
    },
    /// The COMMIT outcome is unknown: the runtime was not applied and the
    /// session must not keep playing on it.
    Unknown {
        reason: String,
    },
}

/// The coordinator's handover: its exclusive money fence, held until its
/// outcome is sent, the frozen exchange snapshot, and where that outcome goes.
pub struct TradeCommitParkedLikeCpp {
    pub money_fence: Box<dyn std::any::Any + Send>,
    /// The coordinator's money persistence tracker: the worker marks it
    /// indeterminate before releasing the fence when the COMMIT outcome is
    /// unknown, so no later full save can write the pre-trade runtime.
    pub money_tracker: Arc<crate::loot_persistence::DurableLootMoneyPersistenceTrackerLikeCpp>,
    pub snapshot: Box<dyn std::any::Any + Send>,
    pub outcome_tx: tokio::sync::oneshot::Sender<TradeSideOutcomeLikeCpp>,
}

impl std::fmt::Debug for TradeCommitParkedLikeCpp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TradeCommitParkedLikeCpp")
            .finish_non_exhaustive()
    }
}

/// Payload of [`super::SessionCommand::TradeCommitLikeCpp`]. The coordinator
/// keeps a clone so it can revoke a handover nobody claimed.
#[derive(Clone)]
pub struct TradeCommitRequestLikeCpp {
    pub coordinator: ObjectGuid,
    handover: Arc<Mutex<Option<TradeCommitParkedLikeCpp>>>,
}

impl std::fmt::Debug for TradeCommitRequestLikeCpp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TradeCommitRequestLikeCpp")
            .field("coordinator", &self.coordinator)
            .finish_non_exhaustive()
    }
}

impl TradeCommitRequestLikeCpp {
    #[must_use]
    pub fn new(coordinator: ObjectGuid, handover: TradeCommitParkedLikeCpp) -> Self {
        Self {
            coordinator,
            handover: Arc::new(Mutex::new(Some(handover))),
        }
    }

    /// Whether the handover is still unclaimed.
    #[must_use]
    pub fn is_live(&self) -> bool {
        self.handover
            .lock()
            .ok()
            .is_some_and(|handover| handover.is_some())
    }

    /// Take the handover: the executor to run the exchange, or the
    /// coordinator to revoke it. Exactly one caller gets it.
    pub fn claim(&self) -> Option<TradeCommitParkedLikeCpp> {
        self.handover
            .lock()
            .ok()
            .and_then(|mut handover| handover.take())
    }
}
