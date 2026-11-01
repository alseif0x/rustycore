// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot session-command receivers: the remote roll vote, the master-loot give
//! and the roll winner store (#1363 slice 5c; moved from the World
//! `handlers/loot/handlers.rs`). `CMSG_LOOT_MONEY` is in `money`.
//!
//! The receiver half of the creature loot-release VALUES fanout followed in
//! slice 7a. The creature melee-damage receiver that shared the World file
//! moved to `wow-world-entities` (`combat_session`).

use super::*;

mod money;

pub use money::*;

pub async fn handle_represented_loot_roll_vote_command_with_generator_like_cpp<
    H: LootSessionHostLikeCpp,
>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    item_valuation: &ItemValuationCatalogsLikeCpp,
    command: LootRollVoteCommand,
) {
    let roll_key = (command.loot_obj, command.loot_list_id);
    let Some(current_roll) = host
        .loot()
        .represented_loot_roll_like_cpp(roll_key.0, roll_key.1)
    else {
        return;
    };
    if !command.targets_identity_like_cpp(&current_roll.command_identity) {
        return;
    }

    let roll = LootRoll {
        loot_obj: command.loot_obj,
        loot_list_id: command.loot_list_id,
        roll_type: command.roll_type,
    };

    let _ = represented_player_vote_on_loot_roll_with_pass_state_and_generator_like_cpp(
        host,
        item_guid_generator,
        item_valuation,
        &roll,
        command.voter_guid,
        command.pass_on_group_loot,
    )
    .await;
}

pub async fn handle_represented_master_loot_give_command_with_generator_like_cpp<
    H: LootSessionHostLikeCpp,
>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    command: MasterLootGiveCommand,
) {
    let Some(player_guid) = host.player_guid() else {
        let _ = command.result_tx.send(MasterLootGiveResult::TargetMismatch);
        return;
    };

    if command.entry.allowed_looters.is_empty()
        || !command.entry.allowed_looters.contains(&player_guid)
    {
        let _ = command.result_tx.send(MasterLootGiveResult::StoreFailed(
            LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
        ));
        return;
    }

    if let Some(error) = represented_master_loot_can_store_error_like_cpp(
        host,
        player_guid,
        command.entry.item_id,
        command.entry.quantity,
    ) {
        let _ = command
            .result_tx
            .send(MasterLootGiveResult::StoreFailed(error));
        return;
    }

    let stored = if let Some(claim) = command.claim.as_ref() {
        store_claimed_direct_loot_item_from_owner_with_generator_like_cpp(
            host,
            item_guid_generator,
            &command.entry,
            command.dungeon_encounter_id,
            command.loot_owner,
            command.loot_obj,
            claim,
        )
        .await
    } else {
        store_direct_loot_item_from_owner_with_generator_like_cpp(
            host,
            item_guid_generator,
            &command.entry,
            command.dungeon_encounter_id,
            command.loot_owner,
        )
        .await
    };
    let result = if stored {
        MasterLootGiveResult::Stored
    } else {
        MasterLootGiveResult::StoreFailed(LOOT_ERROR_MASTER_OTHER_LIKE_CPP)
    };

    debug!(
        account = host.core().account_id,
        master = ?command.master_guid,
        owner = ?command.loot_owner,
        loot_obj = ?command.loot_obj,
        loot_list_id = command.loot_list_id,
        ?result,
        "processed represented remote master-loot give command"
    );

    let _ = command.result_tx.send(result);
}

pub async fn handle_represented_loot_roll_store_winner_command_with_generator_like_cpp<
    H: LootSessionHostLikeCpp,
>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    command: LootRollStoreWinnerCommand,
) {
    let Some(player_guid) = host.player_guid() else {
        let _ = command.result_tx.send(MasterLootGiveResult::TargetMismatch);
        return;
    };

    if command.entries.is_empty()
        || (!command.is_disenchant && command.entries.len() != 1)
        || command.entries.iter().any(|entry| {
            entry.allowed_looters.is_empty()
                || !entry.allowed_looters.contains(&player_guid)
                || !entry.roll_winner_allows_like_cpp(player_guid)
        })
    {
        let _ = command.result_tx.send(MasterLootGiveResult::StoreFailed(
            LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
        ));
        return;
    }

    if let Some(error) = command.entries.iter().find_map(|entry| {
        represented_master_loot_can_store_error_like_cpp(
            host,
            player_guid,
            entry.item_id,
            entry.quantity,
        )
    }) {
        let _ = command
            .result_tx
            .send(MasterLootGiveResult::StoreFailed(error));
        return;
    }

    let stored = if command.is_disenchant {
        store_direct_disenchant_batch_with_generator_like_cpp(
            host,
            item_guid_generator,
            &command.entries,
            command.dungeon_encounter_id,
            command.claim.as_ref(),
            command
                .claim
                .as_ref()
                .map(|claim| LootItemClaimCommitContextLikeCpp {
                    owner_guid: command.loot_owner,
                    loot_obj: command.loot_obj,
                    loot_list_id: command.loot_list_id,
                    player_guid,
                    free_for_all: match claim.payload_like_cpp() {
                        LootClaimPayload::Item(entry) => entry.flags.freeforall,
                        LootClaimPayload::Money(_) => false,
                    },
                }),
        )
        .await
    } else if let Some(claim) = command.claim.as_ref() {
        store_claimed_direct_loot_item_from_owner_with_generator_like_cpp(
            host,
            item_guid_generator,
            &command.entries[0],
            command.dungeon_encounter_id,
            command.loot_owner,
            command.loot_obj,
            claim,
        )
        .await
    } else {
        store_direct_loot_item_from_owner_with_generator_like_cpp(
            host,
            item_guid_generator,
            &command.entries[0],
            command.dungeon_encounter_id,
            command.loot_owner,
        )
        .await
    };
    let result = if stored {
        MasterLootGiveResult::Stored
    } else {
        MasterLootGiveResult::StoreFailed(LOOT_ERROR_MASTER_OTHER_LIKE_CPP)
    };

    debug!(
        account = host.core().account_id,
        owner = ?command.loot_owner,
        loot_obj = ?command.loot_obj,
        loot_list_id = command.loot_list_id,
        ?result,
        "processed represented remote loot-roll winner store command"
    );

    let _ = command.result_tx.send(result);
}

/// Receiver-owned half of the loot-release VALUES fanout. Applying
/// `Player::isAllowedToLoot` here preserves session-local pending-bind
/// state and avoids serialising one player's dynamic flags for another.
pub fn handle_send_creature_loot_release_values_update_command_like_cpp<
    H: LootSessionHostLikeCpp,
>(
    host: &mut H,
    command: SendCreatureLootReleaseValuesUpdateLikeCppCommand,
) {
    if host.state() != SessionState::LoggedIn
        || host.core().player_map_id_like_cpp() != command.map_id
    {
        return;
    }
    let instance_id = host
        .core()
        .current_canonical_player_map_key_like_cpp()
        .map(|key| key.instance_id)
        .unwrap_or(0);
    if instance_id != command.instance_id
        || !host
            .core()
            .client_visible_guids_like_cpp
            .contains(&command.creature_guid)
    {
        return;
    }
    if host.represented_can_receive_creature_message_to_set_by_guid_like_cpp(
        command.creature_guid,
        command.map_id,
        command.instance_id,
        false,
    ) != Some(true)
    {
        return;
    }
    let Some(expected_authority) = command.authority.as_ref() else {
        return;
    };
    // Both non-found facts refuse the queued VALUES update, and neither
    // reconstructs an authority from the other store.
    let OwnedLootAuthorityLookupOutcomeLikeCpp::Found(current_authority) =
        represented_owned_loot_authority_outcome_like_cpp(host, command.creature_guid)
    else {
        return;
    };
    if !current_authority.shares_storage_like_cpp(expected_authority) {
        // The queued update belongs to an older corpse generation. C++
        // publishes synchronously before respawn; Rust must not apply the
        // delayed VALUES delta to a replacement creature with the same GUID.
        return;
    }
    let Some(viewer_guid) = host.player_guid() else {
        return;
    };
    let viewer_update = creature_loot_release_values_for_viewer_like_cpp(
        host,
        command.creature_guid,
        viewer_guid,
        host.has_pending_bind_like_cpp(),
        Some(expected_authority),
        command.unit_values_update,
    );
    host.send_packet(&UpdateObject::unit_values_update(
        command.creature_guid,
        command.map_id,
        viewer_update,
    ));
}
