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

#[cfg(test)]
use wow_loot::ROLL_FLAG_TYPE_NEED_LIKE_CPP;

mod authority;
mod claims;
mod fanout;
mod handlers;
mod loot_host;
mod persistence;
mod requests;
mod sources;
mod visibility_commands;

use crate::session::mailbox::{
    RefreshVisibleWorldCreaturesLikeCppCommand, SendAddonIfRegisteredLikeCppCommand,
    SendCreatureSpellCastIfVisibleLikeCppCommand, SendIfVisibleLikeCppCommand,
};
use wow_core::ObjectGuid;

// Test-only represented GameObject/decay constants; production paths read the
// same values through their own imports.
#[cfg(test)]
use wow_entities::{
    GAMEOBJECT_TYPE_AREADAMAGE, GAMEOBJECT_TYPE_BINDER, GAMEOBJECT_TYPE_CHAIR,
    GAMEOBJECT_TYPE_DOOR, GAMEOBJECT_TYPE_GUILD_BANK, GAMEOBJECT_TYPE_QUESTGIVER,
};
#[cfg(test)]
use wow_world_core::session::{looted_corpse_decay_secs_like_cpp, mailbox::SessionCommand};

#[cfg(test)]
use wow_persistence::{
    STORED_ITEM_MONEY_SOURCE_ROWS_EXPECTED_LIKE_CPP, StoredItemMoneyPersistenceOutcomeLikeCpp,
    classify_stored_item_money_reconciliation_like_cpp,
    stored_item_money_zero_without_source_outcome_like_cpp,
};

use crate::session::WorldSession;
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

// Gameobject interaction geometry is shared with the application layer's
// LootRelease context; the canonical implementation lives in
// `wow-world-entities`.
#[cfg(test)]
pub(crate) use wow_world_entities::represented_gameobject_display_box_contains_like_cpp;

/// Unit fixtures that predate canonical map objects may exercise packet-cache
/// behavior locally. This is a compile-time false branch in production: live
/// Creature/GameObject claims fail closed without their map-owned authority.
pub(super) const fn represented_local_loot_fixture_allowed_like_cpp() -> bool {
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

#[cfg(test)]
#[path = "../../../unit_tests/handlers/loot_tests.rs"]
mod tests;
