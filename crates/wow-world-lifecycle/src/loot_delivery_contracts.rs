// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot delivery contracts shared with the detached durable workers.
//!
//! Session-independent: moved out of `wow-world` under #1263 F5.

use std::collections::HashSet;
use std::sync::Arc;

use wow_core::ObjectGuid;
use wow_entities::MAX_MONEY_AMOUNT;
use wow_loot::OwnedLootAuthority;
use wow_world_core::session::directory::{PlayerRegistration, PlayerRegistry};
use wow_world_core::session::mailbox::SessionCommand;

pub use crate::{
    DurableItemLootCompletionLikeCpp, DurableItemLootPersistenceGuardLikeCpp,
    DurableItemLootPersistenceTrackerLikeCpp, DurableLootItemFanoutLikeCpp,
};

/// Routing data retained by the detached durable worker so viewers that open
/// the same C++ `Loot` while SQL is in flight are not missed. The worker
/// samples the authority only after the money claim commits; an opener before
/// that point saw the non-zero pool and receives `CoinRemoved`, while a later
/// opener observes zero directly in its `LootResponse`.
pub struct LootMoneyViewerFanoutLikeCpp {
    pub scope_player: ObjectGuid,
    pub source_player: ObjectGuid,
    pub source_command_tx: flume::Sender<SessionCommand>,
    pub player_registry: Option<Arc<PlayerRegistry>>,
    pub map_id: u16,
    pub instance_id: u32,
    pub loot_owner: ObjectGuid,
    pub loot_obj: ObjectGuid,
    pub authority: OwnedLootAuthority,
    pub authority_generation: u64,
    pub payout_recipients: HashSet<ObjectGuid>,
}

/// Opaque destination for one already-durable loot-money publication.
#[derive(Clone)]
pub enum LootMoneyDeliveryAddressLikeCpp {
    /// The source session owns this sender directly.
    Source(flume::Sender<SessionCommand>),
    /// A remote session is addressed by its directory incarnation.
    Directory {
        registry: Arc<PlayerRegistry>,
        registration: PlayerRegistration,
    },
}

impl LootMoneyDeliveryAddressLikeCpp {
    pub fn queue_reliably_like_cpp(self, command: SessionCommand) {
        match self {
            Self::Source(command_tx) => {
                if let Err(error) = command_tx.try_send(command) {
                    let command = error.into_inner();
                    tokio::spawn(async move {
                        let _ = command_tx.send_async(command).await;
                    });
                }
            }
            Self::Directory {
                registry,
                registration,
            } => {
                let _ = registry.queue_current_command_reliably(registration, command);
            }
        }
    }
}

/// C++ `Player::ModifyMoney` accepts the whole positive delta or leaves the
/// balance unchanged when it would cross `MAX_MONEY_AMOUNT`.
pub fn loot_money_durable_outcome_like_cpp(
    current_money: u64,
    requested_delta: u64,
) -> (u64, u64) {
    current_money
        .checked_add(requested_delta)
        .filter(|new_money| *new_money <= MAX_MONEY_AMOUNT)
        .map_or((current_money, 0), |new_money| (new_money, requested_delta))
}
pub use wow_loot::RepresentedLootRollVote;
