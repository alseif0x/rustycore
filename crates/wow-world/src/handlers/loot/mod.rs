// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot handlers, organised by feature.
//!
//! Issue #224 split the former 13,606-line `handlers/loot.rs` into private
//! feature modules. The logical owner, every registration, opcode and
//! dispatcher arm are unchanged; this module keeps the shared constants,
//! helper types and free functions the features build on.

use wow_loot::{
    LOOT_METHOD_FREE_FOR_ALL_LIKE_CPP, LOOT_METHOD_GROUP_LIKE_CPP, LOOT_METHOD_MASTER_LIKE_CPP,
    LOOT_METHOD_NEED_BEFORE_GREED_LIKE_CPP, LOOT_METHOD_PERSONAL_LIKE_CPP,
    LOOT_METHOD_ROUND_ROBIN_LIKE_CPP, creature_loot_is_allowed_to_player_like_cpp,
    loot_can_be_opened_by_player_like_cpp, loot_has_over_threshold_item_like_cpp,
    loot_is_looted_like_cpp, loot_item_is_looted_for_player_like_cpp,
    mark_loot_item_looted_for_player_like_cpp,
};

use wow_loot::{
    ROLL_VOTE_DISENCHANT_LIKE_CPP, ROLL_VOTE_GREED_LIKE_CPP, ROLL_VOTE_NEED_LIKE_CPP,
    ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP, ROLL_VOTE_NOT_VALID_LIKE_CPP, ROLL_VOTE_PASS_LIKE_CPP,
    represented_loot_roll_current_winner_like_cpp, represented_loot_roll_finish_winner_like_cpp,
};

#[cfg(test)]
use wow_loot::ROLL_FLAG_TYPE_NEED_LIKE_CPP;

mod authority;
mod claims;
mod combat_commands;
mod fanout;
mod handlers;
mod loot_host;
mod money;
mod persistence;
mod random_properties;
mod requests;
mod rolls;
mod session_loot_host;
mod sources;
mod storage_plans;
mod visibility_commands;

use std::collections::HashMap;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::{Duration, Instant};

use tracing::{debug, warn};

use crate::session::directory::{PlayerRegistry, PrepareLootMoneyApplicationLikeCpp};
use crate::session::mailbox::{
    ApplyCreatureMeleeDamageLikeCppCommand, ApplyGroupJoinLikeCppCommand,
    ApplyGroupRemovalLikeCppCommand, ApplyLootMoneyLikeCppCommand, ApplyLootMoneyResultLikeCpp,
    CreatureAttackStartLikeCppCommand, CreatureAttackStopLikeCppCommand, KickLikeCppCommand,
    LootRollCommandIdentityLikeCpp, LootRollStoreWinnerCommand, LootRollVoteCommand,
    MasterLootGiveCommand, MasterLootGiveResult, NotifyLootMoneyRemovedLikeCppCommand,
    ReconcilePvpCombatExpiryLikeCppCommand, RefreshVisibleWorldCreaturesLikeCppCommand,
    SendAddonIfRegisteredLikeCppCommand, SendCreatureLootReleaseValuesUpdateLikeCppCommand,
    SendCreatureSpellCastIfVisibleLikeCppCommand, SendIfVisibleLikeCppCommand,
    SendPartyUpdateLikeCppCommand, SessionCommand,
};
use wow_constants::{ClientOpcodes, InventoryResult, ItemContext, ItemFieldFlags, UnitDynFlags};
use wow_core::ObjectGuid;
use wow_entities::{
    INVENTORY_DEFAULT_SIZE, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_END,
    INVENTORY_SLOT_ITEM_START, ItemPosCount, MAX_MONEY_AMOUNT, make_item_pos,
};
use wow_handler::{PacketProcessing, SessionStatus};

// Test-only represented GameObject/decay constants; production paths read the
// same values through their own imports.
#[cfg(test)]
use wow_entities::{
    GAMEOBJECT_TYPE_AREADAMAGE, GAMEOBJECT_TYPE_BINDER, GAMEOBJECT_TYPE_CHAIR,
    GAMEOBJECT_TYPE_DOOR, GAMEOBJECT_TYPE_GUILD_BANK, GAMEOBJECT_TYPE_QUESTGIVER,
};
use wow_world_core::session::OwnedLootAuthorityLookupOutcomeLikeCpp;
#[cfg(test)]
use wow_world_core::session::looted_corpse_decay_secs_like_cpp;

use crate::session::registry::PacketHandlerEntry;
use wow_loot::{
    LootClaimCommitError, LootClaimError, LootClaimLease, LootClaimPayload, OwnedLootAuthority,
    OwnedLootSnapshot, loot_condition_reference_self_references_like_cpp,
    loot_condition_row_normalize_without_external_stores_like_cpp,
};
use wow_packet::ServerPacket;
use wow_packet::packets::item::ItemInstance;
use wow_packet::packets::loot::{
    CoinRemoved, LOOT_ERROR_MASTER_OTHER_LIKE_CPP, LOOT_ERROR_NO_LOOT_LIKE_CPP,
    LOOT_ERROR_TOO_FAR_LIKE_CPP, LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP,
    LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP, LootAllPassed, LootEntry, LootItemData, LootList,
    LootMoney, LootMoneyNotify, LootReleaseAll, LootRemoved, LootResponse, LootRoll,
    LootRollBroadcast, LootRollWon, SLootRelease, SetLootSpecialization, StartLootRoll,
};
use wow_packet::packets::update::{ItemCreateData, ItemEnchantmentValuesUpdate, UpdateObject};
use wow_persistence::{
    PersistenceOutcomeLikeCpp, StoredItemMoneyPersistenceAttemptLikeCpp,
    StoredItemMoneyPersistenceRequestLikeCpp, StoredItemMoneyReconciliationLikeCpp,
    StoredItemMoneyRollbackKindLikeCpp,
};
#[cfg(test)]
use wow_persistence::{
    STORED_ITEM_MONEY_SOURCE_ROWS_EXPECTED_LIKE_CPP, StoredItemMoneyPersistenceOutcomeLikeCpp,
    classify_stored_item_money_reconciliation_like_cpp,
    stored_item_money_zero_without_source_outcome_like_cpp,
};

use crate::session::{
    DurableItemLootCompletionLikeCpp, DurableItemLootPersistenceGuardLikeCpp,
    DurableLootItemFanoutLikeCpp, InventoryItem, ItemValuationCatalogsLikeCpp,
    LootMoneyDeliveryAddressLikeCpp, LootMoneyPersistenceErrorLikeCpp,
    LootMoneyViewerFanoutLikeCpp, RepresentedLootRollState, RepresentedLootRollVote,
    RepresentedQuestObjectiveProgressEventLikeCpp, WorldSession,
    loot_money_durable_outcome_like_cpp,
};
use random_properties::{LootStoreRandomProperties, loot_store_data_can_stack_with_item};
use storage_plans::{
    LootItemClaimCommitContextLikeCpp, PlannedDirectLootExistingStack,
    PlannedDisenchantExistingPush, PlannedDisenchantExistingStack, PlannedDisenchantGrant,
    PlannedDisenchantNewPush, PlannedLootNewStack,
};
const LOOT_ROLL_TIMEOUT_MS_LIKE_CPP: u32 = 60_000;
#[cfg(test)]
const ROLL_ALL_TYPE_NO_DISENCHANT_LIKE_CPP: u8 = 0x07;
#[cfg(test)]
const LOCK_KEY_SKILL_LIKE_CPP: u8 = 2;
#[cfg(test)]
const LOCK_KEY_SPELL_LIKE_CPP: u8 = 3;
#[cfg(test)]
const SPELL_EFFECT_OPEN_LOCK_LIKE_CPP: u32 = 33;

#[cfg(test)]
use super::loot_test_shims::*;
use wow_world_loot::{loot_type_for_client_like_cpp, represented_loot_response_items_like_cpp};

// Gameobject interaction geometry is shared with the application layer's
// LootRelease context; the canonical implementation lives in
// `wow-world-entities`.
#[cfg(test)]
pub(crate) use wow_world_entities::represented_gameobject_display_box_contains_like_cpp;

/// Unit fixtures that predate canonical map objects may exercise packet-cache
/// behavior locally. This is a compile-time false branch in production: live
/// Creature/GameObject claims fail closed without their map-owned authority.
const fn represented_local_loot_fixture_allowed_like_cpp() -> bool {
    cfg!(test)
}

#[cfg(test)]
fn assign_represented_personal_loot_items_like_cpp<R: Rng + ?Sized>(
    loot: &mut CreatureLoot,
    tappers: &[ObjectGuid],
    rng: &mut R,
) {
    if tappers.is_empty() {
        return;
    }

    loot.unlooted_count = 0;
    loot.player_ffa_items.clear();

    for entry in &mut loot.items {
        entry.allowed_looters.clear();
        entry.flags.counted = false;

        let chosen_tapper = tappers[rng.gen_range(0..tappers.len())];
        entry.add_allowed_looter_like_cpp(chosen_tapper);
    }

    rebuild_represented_personal_loot_counts_like_cpp(loot);
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CreatureLootReleaseCommandQueueOutcomeLikeCpp {
    Queued,
    Retrying,
    Disconnected,
}

#[cfg(test)]
fn queue_creature_loot_release_command_reliably_like_cpp(
    command_tx: &flume::Sender<SessionCommand>,
    command: SessionCommand,
) -> CreatureLootReleaseCommandQueueOutcomeLikeCpp {
    match command_tx.try_send(command) {
        Ok(()) => CreatureLootReleaseCommandQueueOutcomeLikeCpp::Queued,
        Err(flume::TrySendError::Disconnected(_)) => {
            CreatureLootReleaseCommandQueueOutcomeLikeCpp::Disconnected
        }
        Err(flume::TrySendError::Full(command)) => {
            let command_tx = command_tx.clone();
            // Never await another session from the source session loop: two
            // full queues could otherwise wait on each other forever. The
            // detached retry retains the exact command until capacity opens;
            // receiver-side authority/lifecycle gates coalesce its meaning to
            // the current corpse generation and reject stale respawn reuse.
            tokio::spawn(async move {
                if command_tx.send_async(command).await.is_err() {
                    tracing::debug!(
                        "loot-release DynamicFlags retry ended after target session disconnected"
                    );
                }
            });
            CreatureLootReleaseCommandQueueOutcomeLikeCpp::Retrying
        }
    }
}

fn connected_roll_looters_like_cpp(
    entry: &LootEntry,
    player_guid: ObjectGuid,
    current_map_id: u16,
    current_instance_id: u32,
    player_registry: Option<&PlayerRegistry>,
) -> Vec<ObjectGuid> {
    let mut looters = Vec::new();

    for looter in &entry.allowed_looters {
        if *looter == player_guid {
            looters.push(*looter);
            continue;
        }

        let Some(registry) = player_registry else {
            continue;
        };
        let Some(player) = registry.loot_presence(*looter) else {
            continue;
        };
        if player.map_id == current_map_id && player.instance_id == current_instance_id {
            looters.push(*looter);
        }
    }

    looters.sort_by_key(|guid| (guid.high_value(), guid.low_value()));
    looters.dedup();
    looters
}

fn represented_max_enchanting_skill_like_cpp(
    looters: &[ObjectGuid],
    current_player_guid: ObjectGuid,
    current_player_enchanting_skill: Option<u16>,
    player_registry: Option<&PlayerRegistry>,
) -> u16 {
    looters
        .iter()
        .filter_map(|looter| {
            if *looter == current_player_guid {
                current_player_enchanting_skill
            } else {
                player_registry.and_then(|registry| registry.loot_enchanting_skill(*looter))
            }
        })
        .max()
        .unwrap_or(0)
}

fn start_loot_roll_packet_like_cpp(
    loot_obj: ObjectGuid,
    map_id: u16,
    loot_method: u8,
    entry: &LootEntry,
    valid_rolls: u8,
    dungeon_encounter_id: i32,
) -> StartLootRoll {
    StartLootRoll {
        loot_obj,
        map_id: map_id as i32,
        roll_time_ms: LOOT_ROLL_TIMEOUT_MS_LIKE_CPP,
        method: loot_method,
        valid_rolls,
        loot_roll_ineligible_reason: [0; 4],
        item: LootItemData {
            item_type: 0,
            ui_type: LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP,
            can_trade_to_tap_list: entry.allowed_looters.len() > 1,
            loot: ItemInstance {
                item_id: entry.item_id as i32,
                random_properties_id: entry.random_properties_id,
                random_properties_seed: entry.random_properties_seed,
                ..ItemInstance::default()
            },
            loot_list_id: entry.loot_list_id,
            quantity: entry.quantity,
            loot_item_type: 0,
        },
        dungeon_encounter_id,
    }
}

use wow_world_loot::{
    LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP, LOOT_SLOT_TYPE_LOCKED_LIKE_CPP,
    LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP, loot_roll_broadcast_item_like_cpp,
};

fn loot_item_context(context: u8) -> ItemContext {
    <ItemContext as num_traits::FromPrimitive>::from_u8(context).unwrap_or(ItemContext::None)
}

/// Own a loot lease in the same detached task that crosses the durable
/// persistence boundary.  Tokio does not cancel a spawned task when the
/// caller drops its `JoinHandle`, so packet/session cancellation cannot turn a
/// successful SQL commit back into an available object-owned claim.
enum LootClaimPersistenceWorkerError<E> {
    Persistence(E),
    Claim(LootClaimCommitError),
}

impl<E: std::fmt::Debug> std::fmt::Debug for LootClaimPersistenceWorkerError<E> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Persistence(error) => formatter.debug_tuple("Persistence").field(error).finish(),
            Self::Claim(error) => formatter.debug_tuple("Claim").field(error).finish(),
        }
    }
}

#[cfg(test)]
fn spawn_loot_claim_persistence_worker_like_cpp<F, E>(
    persistence: F,
    claim: Option<LootClaimLease>,
    durable_item_completion: Option<(
        DurableItemLootPersistenceGuardLikeCpp,
        DurableItemLootCompletionLikeCpp,
    )>,
) -> Result<
    tokio::task::JoinHandle<Result<(), LootClaimPersistenceWorkerError<E>>>,
    LootClaimCommitError,
>
where
    F: std::future::Future<Output = Result<(), E>> + Send + 'static,
    E: Send + 'static,
{
    let persistence_guard = claim
        .as_ref()
        .map(LootClaimLease::begin_persistence_guard_like_cpp)
        .transpose()?;
    drop(claim);
    Ok(tokio::spawn(async move {
        let mut durable_item_completion = durable_item_completion;
        persistence
            .await
            .map_err(LootClaimPersistenceWorkerError::Persistence)?;
        if let Some(mut guard) = persistence_guard {
            let (_, committed_snapshot) = guard
                .commit_with_snapshot_like_cpp()
                .map_err(LootClaimPersistenceWorkerError::Claim)?;
            if let (Some(snapshot), Some((_, completion))) =
                (committed_snapshot, durable_item_completion.as_ref())
                && let Some(fanout) = completion.item_fanout.as_ref()
            {
                // Publish the serialization cut before exposing the durable
                // completion to the session. Sampling the authority later can
                // include an opener that already saw the consumed slot.
                let _ = fanout.committed_snapshot.set(snapshot);
            }
        }
        if let Some((guard, completion)) = durable_item_completion.as_mut() {
            guard.mark_committed_like_cpp(completion.clone());
        }
        Ok(())
    }))
}

/// Outcome-aware persistence worker for consume-and-grant item transactions.
/// An unknown COMMIT cannot be treated as rollback: the old object allocation
/// is quarantined permanently and the player is kicked to reload whichever
/// durable state the concrete adapter ultimately kept.
fn spawn_loot_item_persistence_worker_like_cpp<F>(
    persistence: F,
    claim: Option<LootClaimLease>,
    durable_item_completion: Option<(
        DurableItemLootPersistenceGuardLikeCpp,
        DurableItemLootCompletionLikeCpp,
    )>,
    command_tx: flume::Sender<SessionCommand>,
) -> Result<
    tokio::task::JoinHandle<Result<(), LootClaimPersistenceWorkerError<String>>>,
    LootClaimCommitError,
>
where
    F: std::future::Future<Output = PersistenceOutcomeLikeCpp> + Send + 'static,
{
    let mut persistence_guard = claim
        .as_ref()
        .map(LootClaimLease::begin_persistence_guard_like_cpp)
        .transpose()?;
    drop(claim);
    Ok(tokio::spawn(async move {
        let mut durable_item_completion = durable_item_completion;
        match persistence.await {
            PersistenceOutcomeLikeCpp::Applied { .. } => {}
            PersistenceOutcomeLikeCpp::Failed { reason } => {
                return Err(LootClaimPersistenceWorkerError::Persistence(reason));
            }
            PersistenceOutcomeLikeCpp::Unknown { reason } => {
                if let Some(guard) = persistence_guard.as_mut() {
                    let _ = guard.quarantine_commit_unknown_like_cpp();
                }
                let kick = SessionCommand::KickLikeCpp(KickLikeCppCommand {
                    reason: "loot item COMMIT outcome is unknown; relog required".to_string(),
                });
                if let Err(send_error) = command_tx.try_send(kick) {
                    let kick = send_error.into_inner();
                    tokio::spawn(async move {
                        let _ = command_tx.send_async(kick).await;
                    });
                }
                return Err(LootClaimPersistenceWorkerError::Persistence(reason));
            }
        }
        if let Some(mut guard) = persistence_guard {
            let (_, committed_snapshot) = guard
                .commit_with_snapshot_like_cpp()
                .map_err(LootClaimPersistenceWorkerError::Claim)?;
            if let (Some(snapshot), Some((_, completion))) =
                (committed_snapshot, durable_item_completion.as_ref())
                && let Some(fanout) = completion.item_fanout.as_ref()
            {
                let _ = fanout.committed_snapshot.set(snapshot);
            }
        }
        if let Some((guard, completion)) = durable_item_completion.as_mut() {
            guard.mark_committed_like_cpp(completion.clone());
        }
        Ok(())
    }))
}

#[cfg(test)]
#[path = "../../../unit_tests/handlers/loot_tests.rs"]
mod tests;
