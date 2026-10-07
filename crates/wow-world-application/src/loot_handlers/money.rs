// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot handler family: the `LootMoney` **command** receivers.
//!
//! These are session-command receivers, not packet registrations: the
//! `CMSG_LOOT_MONEY` opcode stays registered in the World shell for
//! `handle_loot_money_with_generator_like_cpp`, which is the last loot consumer
//! left there. The mailbox rail the source-side persistence worker feeds
//! (`LootHandler.cpp::HandleLootMoneyOpcode`, the shared-pool money path) is
//! applied here, and no opcode, packet layout or handler metadata is involved.
//!
//! C++ source of truth: `src/server/game/Handlers/LootHandler.cpp`
//! (`WorldSession::HandleLootMoneyOpcode`) plus `Loot.cpp`'s
//! `NotifyMoneyRemoved` publication. Each body keeps its original gates, order,
//! log strings and packets; only the access path changed.

use super::*;
use std::sync::atomic::Ordering;

/// Applies one already-durable shared-loot payout to its recipient session.
///
/// C++ computes each member's share and updates the balance on the source
/// session's opcode path; the represented flow fans the same payout to every
/// recipient rail, so this receiver re-checks the recipient, the exact-once
/// `applied`/`published` gates and the object-owned generation before it
/// mutates canonical money or publishes.
pub(super) async fn handle_apply_loot_money_with_generator_like_cpp_command<H, C>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    command: ApplyLootMoneyLikeCppCommand,
) where
    H: LootHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    if host.loot_unit_hub_ref_like_cpp().core.player_guid() != Some(command.recipient) {
        return;
    }
    let apply_money = !command.applied.swap(true, Ordering::SeqCst);
    let publish = !command.published.swap(true, Ordering::SeqCst);
    if !apply_money && !publish {
        return;
    }

    if publish
        && command.send_coin_removed.load(Ordering::Acquire)
        && command.authority_committed.load(Ordering::Acquire)
        && represented_loot_money_command_targets_active_generation_like_cpp(
            host,
            command.loot_owner,
            &command.authority,
            command.authority_generation,
        )
    {
        host.loot_unit_hub_ref_like_cpp()
            .core
            .send_packet(&CoinRemoved {
                loot_obj: command.loot_obj,
            });
        host.loot_money_release_owner_access_like_cpp()
            .refresh_owned_loot_summary_like_cpp(command.loot_owner);
        if let Some(player_guid) = host.loot_unit_hub_ref_like_cpp().core.player_guid() {
            let _ = host.master_loot_reconcile_loot_cache_like_cpp(command.loot_owner, player_guid);
        }
    }
    let durable_applied_amount = command.durable_applied_amount.load(Ordering::Acquire);
    let _ = host
        .loot_money_apply_durable_payout_like_cpp(
            item_guid_generator,
            command.amount,
            durable_applied_amount,
            command.sole_looter,
            apply_money,
            publish,
        )
        .await;
}

/// C++ `Loot::NotifyMoneyRemoved` delivery to a viewer that is not itself a
/// payout recipient.
pub(super) fn handle_notify_loot_money_removed_like_cpp_command<H, C>(
    host: &mut H,
    command: NotifyLootMoneyRemovedLikeCppCommand,
) where
    H: LootHandlerHostLikeCpp<C>,
    C: Sync,
{
    let recipient_matches =
        host.loot_unit_hub_ref_like_cpp().core.player_guid() == Some(command.recipient);
    if !recipient_matches
        || !command.authority_committed.load(Ordering::Acquire)
        || !represented_loot_money_command_targets_active_generation_like_cpp(
            host,
            command.loot_owner,
            &command.authority,
            command.authority_generation,
        )
    {
        return;
    }

    host.loot_unit_hub_ref_like_cpp()
        .core
        .send_packet(&CoinRemoved {
            loot_obj: command.loot_obj,
        });
    host.loot_money_release_owner_access_like_cpp()
        .refresh_owned_loot_summary_like_cpp(command.loot_owner);
    if let Some(player_guid) = host.loot_unit_hub_ref_like_cpp().core.player_guid() {
        let _ = host.master_loot_reconcile_loot_cache_like_cpp(command.loot_owner, player_guid);
    }
}

/// The recorded generation gate of both money command receivers: the command
/// must still target the active object-owned generation, so a durable payout
/// whose pool was replaced by a respawn cannot touch the replacement.
fn represented_loot_money_command_targets_active_generation_like_cpp<H, C>(
    host: &mut H,
    owner_guid: ObjectGuid,
    expected_authority: &OwnedLootAuthority,
    authority_generation: u64,
) -> bool
where
    H: LootHandlerHostLikeCpp<C>,
    C: Sync,
{
    if !host
        .loot_unit_loot_ref_like_cpp()
        .active_loot_view_authority_like_cpp(owner_guid)
        .is_some_and(|active| active.shares_storage_like_cpp(expected_authority))
        || !host
            .loot_unit_loot_ref_like_cpp()
            .active_loot_view_generation_like_cpp(owner_guid)
            .is_some_and(|active| *active == authority_generation)
    {
        return false;
    }
    let Some(player_guid) = host.loot_unit_hub_ref_like_cpp().core.player_guid() else {
        return false;
    };
    host.loot_money_release_owner_access_like_cpp()
        .represented_owned_loot_authority_like_cpp(owner_guid)
        .is_some_and(|authority| {
            authority.shares_storage_like_cpp(expected_authority)
                && authority
                    .snapshot_for_player_like_cpp(player_guid)
                    .is_some_and(|snapshot| snapshot.generation == authority_generation)
        })
}
