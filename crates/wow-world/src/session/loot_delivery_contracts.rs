// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot delivery contracts: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{Arc, HashSet};
use super::{LootMoneyPersistenceErrorLikeCpp, MAX_MONEY_AMOUNT};
use super::{ObjectGuid, OwnedLootAuthority, OwnedLootSnapshot};
use super::{PlayerRegistry, SessionCommand, registry};

pub(crate) use wow_world_lifecycle::{
    DurableItemLootCompletionLikeCpp, DurableItemLootPersistenceGuardLikeCpp,
    DurableItemLootPersistenceTrackerLikeCpp, DurableLootItemFanoutLikeCpp,
};

/// Routing data retained by the detached durable worker so viewers that open
/// the same C++ `Loot` while SQL is in flight are not missed. The worker
/// samples the authority only after the money claim commits; an opener before
/// that point saw the non-zero pool and receives `CoinRemoved`, while a later
/// opener observes zero directly in its `LootResponse`.
pub(crate) struct LootMoneyViewerFanoutLikeCpp {
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
pub(crate) enum LootMoneyDeliveryAddressLikeCpp {
    /// The source session owns this sender directly.
    Source(flume::Sender<SessionCommand>),
    /// A remote session is addressed by its directory incarnation.
    Directory {
        registry: Arc<PlayerRegistry>,
        registration: crate::session::directory::PlayerRegistration,
    },
}

impl LootMoneyDeliveryAddressLikeCpp {
    pub(in crate::session) fn queue_reliably_like_cpp(self, command: SessionCommand) {
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

impl std::fmt::Display for LootMoneyPersistenceErrorLikeCpp {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPlayer => formatter.write_str("loot-money player is missing"),
            Self::MissingCharacterDatabase => {
                formatter.write_str("loot-money character database is missing")
            }
            Self::WorkerTerminated => formatter.write_str("loot-money persistence worker stopped"),
            Self::Claim(error) => write!(formatter, "loot-money claim failure: {error:?}"),
            Self::Persistence(reason) => {
                write!(formatter, "loot-money persistence failure: {reason}")
            }
            Self::CommitOutcomeUnknownPersistence(reason) => {
                write!(formatter, "loot-money COMMIT outcome is unknown: {reason}")
            }
        }
    }
}

impl std::error::Error for LootMoneyPersistenceErrorLikeCpp {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::MissingPlayer
            | Self::MissingCharacterDatabase
            | Self::WorkerTerminated
            | Self::Claim(_)
            | Self::Persistence(_)
            | Self::CommitOutcomeUnknownPersistence(_) => None,
        }
    }
}

/// C++ `Player::ModifyMoney` accepts the whole positive delta or leaves the
/// balance unchanged when it would cross `MAX_MONEY_AMOUNT`.
pub(crate) fn loot_money_durable_outcome_like_cpp(
    current_money: u64,
    requested_delta: u64,
) -> (u64, u64) {
    current_money
        .checked_add(requested_delta)
        .filter(|new_money| *new_money <= MAX_MONEY_AMOUNT)
        .map_or((current_money, 0), |new_money| (new_money, requested_delta))
}
pub(crate) use wow_loot::RepresentedLootRollVote;
