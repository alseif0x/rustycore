// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot handler family: `CMSG_LOOT_UNIT` (C++ `WorldSession::HandleLootOpcode`).

use super::*;

/// CMSG_LOOT_UNIT — player right-clicks a dead creature to loot it.
///
/// C++ `WorldSession::HandleLootOpcode` (`Handlers/LootHandler.cpp:216`,
/// registered `Opcodes.cpp:590` as `STATUS_LOGGEDIN`/`PROCESS_THREADUNSAFE`).
/// The body keeps its original parse, gates, order, log strings and packets;
/// only the access path changed.
pub(super) async fn handle_loot_unit_with_catalogs_like_cpp<H, C>(
    host: &mut H,
    item_valuation: &ItemValuationCatalogsLikeCpp,
    mut pkt: WorldPacket,
) where
    H: LootHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    let req = match LootUnit::read(&mut pkt) {
        Ok(r) => r,
        Err(e) => {
            warn!("Bad LootUnit: {e}");
            return;
        }
    };

    let player_guid = match host.loot_unit_hub_ref_like_cpp().core.player_guid() {
        Some(g) => g,
        None => return,
    };

    debug!(account = host.loot_unit_hub_ref_like_cpp().core.account_id, target = ?req.unit, "CMSG_LOOT_UNIT");

    if host
        .loot_unit_hub_ref_like_cpp()
        .resolved_player_is_alive_like_cpp()
        != Some(true)
    {
        return;
    }

    if !req.unit.is_creature_or_vehicle() {
        return;
    }

    // Check creature exists and is dead.
    let creature_state = match host.loot_unit_represented_creature_loot_state_like_cpp(req.unit) {
        Some(state) => state,
        None => {
            warn!("LootUnit: creature {:?} not found", req.unit);
            return;
        }
    };

    if creature_state.is_alive {
        return;
    }

    if host
        .loot_unit_hub_ref_like_cpp()
        .player_position_like_cpp()
        .is_some_and(|player| !player.is_within_dist(&creature_state.position, 30.0))
    {
        return;
    }

    host.loot_unit_interrupt_non_melee_spell_cast_like_cpp();
    host.loot_unit_remove_auras_with_looting_interrupt_flags_like_cpp();

    let ae_owner_guids = if host
        .loot_unit_hub_ref_like_cpp()
        .config
        .enable_ae_loot_like_cpp()
    {
        host.loot_unit_ae_loot_creature_targets_like_cpp(req.unit, player_guid)
            .await
    } else {
        Vec::new()
    };

    if !ae_owner_guids.is_empty() {
        host.loot_unit_hub_ref_like_cpp()
            .core
            .send_packet(&AELootTargets {
                count: ae_owner_guids.len() as u32 + 1,
            });
    }

    let Some(response) = host
        .loot_unit_loot_response_for_owner_like_cpp(req.unit, player_guid, false)
        .await
    else {
        return;
    };
    if host
        .loot_unit_loot_ref_like_cpp()
        .has_active_non_item_loot_views_like_cpp()
    {
        host.loot_unit_release_all_like_cpp(player_guid).await;
    }
    host.loot_unit_loot_mut_like_cpp()
        .set_active_loot_guid(req.unit);
    host.loot_unit_on_loot_opened_with_catalogs_like_cpp(
        item_valuation,
        req.unit,
        player_guid,
        response,
    );

    if !ae_owner_guids.is_empty() {
        host.loot_unit_hub_ref_like_cpp()
            .core
            .send_packet(&AELootTargetsAck);

        for owner_guid in ae_owner_guids {
            if let Some(response) = host
                .loot_unit_loot_response_for_owner_like_cpp(owner_guid, player_guid, true)
                .await
            {
                host.loot_unit_loot_mut_like_cpp()
                    .add_active_loot_view_owner_like_cpp(owner_guid);
                host.loot_unit_on_loot_opened_with_catalogs_like_cpp(
                    item_valuation,
                    owner_guid,
                    player_guid,
                    response,
                );
                host.loot_unit_hub_ref_like_cpp()
                    .core
                    .send_packet(&AELootTargetsAck);
            }
        }
    }
}
