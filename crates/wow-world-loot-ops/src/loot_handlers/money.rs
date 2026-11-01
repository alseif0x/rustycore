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
use wow_packet::ServerPacket;
use wow_packet::packets::loot::LOOT_TYPE_CORPSE_LIKE_CPP;

/// C++ `LootHandler.cpp::HandleLootMoneyOpcode` recipient selection.
///
/// The selection body moved here from the World session under #1263 F4: the
/// eligibility and divisor decisions are unchanged, and every participant is
/// read through the host's loot state, the core hub (group/player directory,
/// position and canonical map key) and the existing narrow host capabilities.
pub(super) fn represented_loot_money_recipients_like_cpp<H, C>(
    host: &H,
    loot_guid: ObjectGuid,
) -> Vec<ObjectGuid>
where
    H: LootHandlerHostLikeCpp<C> + ?Sized,
    C: Sync,
{
    let hub = host.loot_unit_hub_ref_like_cpp();
    let Some(player_guid) = hub.core.player_guid() else {
        return Vec::new();
    };

    let Some(loot) = host
        .loot_unit_loot_ref_like_cpp()
        .cached_loot_for_owner_like_cpp(loot_guid)
    else {
        return vec![player_guid];
    };
    // C++ shares only LOOT_CORPSE. Pickpocket money is creature-owned but
    // personal; vehicle corpses still share even though their HighGuid is
    // not Creature (`LootHandler.cpp::HandleLootMoneyOpcode`).
    if loot.loot_type != LOOT_TYPE_CORPSE_LIKE_CPP {
        return vec![player_guid];
    }

    let (Some(group_guid), Some(group_registry), Some(player_registry)) = (
        host.master_loot_resolved_group_guid_like_cpp(),
        hub.core.group_registry(),
        hub.core.player_registry(),
    ) else {
        return vec![player_guid];
    };

    let Some(group) = group_registry.get(&group_guid) else {
        return vec![player_guid];
    };

    let Some(source_position) = hub.player_position_like_cpp() else {
        return vec![player_guid];
    };
    let mut source_instance_id = hub
        .core
        .current_canonical_player_map_key_like_cpp()
        .map(|key| key.instance_id);
    // Old packet fixtures have no canonical map resident. They must still
    // provide an explicit routing-directory placement; never invent an
    // instance identifier for them.
    #[cfg(any(test, feature = "test-fixtures"))]
    if source_instance_id.is_none() {
        source_instance_id = player_registry
            .loot_presence(player_guid)
            .map(|presence| presence.instance_id);
    }
    let Some(source_instance_id) = source_instance_id else {
        return vec![player_guid];
    };
    let mut recipients = Vec::new();

    for member_guid in &group.members {
        if !loot.allowed_looters.contains(member_guid) {
            continue;
        }

        if *member_guid == player_guid {
            recipients.push(*member_guid);
            continue;
        }

        let Some(member) = player_registry.loot_presence(*member_guid) else {
            continue;
        };

        if !member.is_in_world
            || member.map_id != hub.core.player_map_id_like_cpp()
            || member.instance_id != source_instance_id
        {
            continue;
        }

        if current_map_is_dungeon_like_cpp(hub)
            || source_position.is_within_dist(&member.position, 74.0)
        {
            recipients.push(*member_guid);
        }
    }

    if recipients.is_empty() {
        recipients.push(player_guid);
    }

    recipients
}

/// C++ `MapEntry::IsDungeon` of the session's current map, read from the
/// process-owned map catalog the hub carries.
fn current_map_is_dungeon_like_cpp(hub: HubRef<'_>) -> bool {
    hub.catalogs
        .maps
        .store
        .as_ref()
        .and_then(|store| store.get(u32::from(hub.core.player_map_id_like_cpp())))
        .map(|entry| entry.is_dungeon())
        .unwrap_or(false)
}

/// C++ `Loot::NotifyMoneyRemoved` (`Loot.cpp`) for the represented loot cache.
///
/// The complete publication and pruning operation moved here from the World
/// session under #1263 F4: the `CoinRemoved` bytes, the per-looter delivery
/// attempt, the stale-looter pruning and the authority-viewer retirement keep
/// their original order and content.
pub(super) fn represented_notify_money_removed_like_cpp<H, C>(host: &mut H, owner_guid: ObjectGuid)
where
    H: LootHandlerHostLikeCpp<C>,
    C: Sync,
{
    if let Some(player_guid) = host.loot_unit_hub_ref_like_cpp().core.player_guid() {
        let _ = host.master_loot_reconcile_loot_cache_like_cpp(owner_guid, player_guid);
    }
    let Some((loot_obj, players_looting)) = host
        .loot_unit_loot_ref_like_cpp()
        .cached_loot_for_owner_like_cpp(owner_guid)
        .map(|loot| (loot.loot_guid, loot.players_looting.clone()))
    else {
        return;
    };

    let packet = CoinRemoved { loot_obj };
    let bytes = packet.to_bytes();
    let current_player = host.loot_unit_hub_ref_like_cpp().core.player_guid();
    let current_map = host
        .loot_unit_hub_ref_like_cpp()
        .core
        .player_map_id_like_cpp();
    let current_instance = host
        .loot_unit_hub_ref_like_cpp()
        .core
        .current_canonical_player_map_key_like_cpp()
        .map(|key| key.instance_id)
        .unwrap_or(0);
    let registry = host
        .loot_unit_hub_ref_like_cpp()
        .core
        .player_registry()
        .cloned();
    let mut stale_looters = Vec::new();

    for looter in &players_looting {
        if Some(*looter) == current_player {
            host.loot_unit_hub_ref_like_cpp().core.send_packet(&packet);
            continue;
        }

        let Some(registry) = registry.as_ref() else {
            stale_looters.push(*looter);
            continue;
        };
        let Some(registration) =
            registry.loot_delivery_recipient(*looter, current_map, current_instance)
        else {
            stale_looters.push(*looter);
            continue;
        };
        if registry
            .send_current_packet(registration, bytes.clone())
            .is_err()
        {
            stale_looters.push(*looter);
        }
    }

    if !stale_looters.is_empty()
        && let Some(loot) = host
            .loot_unit_loot_mut_like_cpp()
            .cached_loot_for_owner_mut_like_cpp(owner_guid)
    {
        loot.players_looting
            .retain(|looter| !stale_looters.contains(looter));
    }
    if !stale_looters.is_empty()
        && let OwnedLootAuthorityLookupOutcomeLikeCpp::Found(authority) = host
            .loot_money_release_owner_access_like_cpp()
            .represented_owned_loot_authority_outcome_like_cpp(owner_guid)
    {
        for looter in stale_looters {
            authority.remove_viewer_like_cpp(looter);
        }
        if let Some(player_guid) = current_player {
            let _ = host.master_loot_reconcile_loot_cache_like_cpp(owner_guid, player_guid);
        }
    }
}

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
    matches!(
        host.loot_money_release_owner_access_like_cpp()
            .represented_owned_loot_authority_outcome_like_cpp(owner_guid),
        OwnedLootAuthorityLookupOutcomeLikeCpp::Found(authority)
            if authority.shares_storage_like_cpp(expected_authority)
                && authority
                    .snapshot_for_player_like_cpp(player_guid)
                    .is_some_and(|snapshot| snapshot.generation == authority_generation)
    )
}
