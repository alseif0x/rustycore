// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot handler family: `CMSG_LOOT_ROLL` (C++ `WorldSession::HandleLootRoll`).

use super::*;

/// CMSG_LOOT_ROLL — vote on a pending group loot roll.
///
/// C++ `HandleLootRoll` silently returns when `GetLootRoll` finds no
/// canonical roll state. Rust does not yet port that state machine, so this
/// represented handler preserves the current wire behavior without emitting
/// synthetic errors.
pub(super) async fn handle_loot_roll_with_generator_like_cpp<H, C>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    item_valuation: &ItemValuationCatalogsLikeCpp,
    roll: LootRoll,
) where
    H: LootHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    let Some(player_guid) = host.loot_unit_hub_ref_like_cpp().core.player_guid() else {
        return;
    };

    if host
        .loot_roll_player_vote_like_cpp(item_guid_generator, item_valuation, &roll, player_guid)
        .await
    {
        return;
    }

    if host
        .loot_unit_loot_ref_like_cpp()
        .route_represented_remote_loot_roll_vote_to_owner_like_cpp(
            host.loot_unit_hub_ref_like_cpp(),
            &roll,
            player_guid,
        )
    {
        return;
    }

    debug!(
        account = host.loot_unit_hub_ref_like_cpp().core.account_id,
        loot_obj = ?roll.loot_obj,
        loot_list_id = roll.loot_list_id,
        roll_type = roll.roll_type,
        "CMSG_LOOT_ROLL ignored: canonical LootRoll state is not ported yet"
    );
}
