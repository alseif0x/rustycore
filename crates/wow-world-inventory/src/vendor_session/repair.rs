// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Repair-vendor item repair.

use tracing::debug;
use wow_constants::NPCFlags1;
use wow_packet::packets::misc::RepairItem;

use super::VendorSessionHostLikeCpp;

/// CMSG_REPAIR_ITEM — player repairs item at a repair vendor.
/// C++ ref: WorldSession::HandleRepairItemOpcode.
pub async fn handle_repair_item_with_generator_like_cpp<H: VendorSessionHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    repair: RepairItem,
) {
    let Some(repair_npc) = host.hub_ref().represented_npc_can_interact_with_like_cpp(
        repair.npc_guid,
        NPCFlags1::REPAIR.bits(),
        0,
    ) else {
        debug!(
            npc_guid = ?repair.npc_guid,
            account = host.core().account_id,
            "RepairItem rejected: NPC missing, out of range, dead, or lacks REPAIR flag"
        );
        return;
    };

    host.remove_represented_feign_death_if_needed_like_cpp();

    // C++ uses GetReputationPriceDiscount(unit) and RATE_REPAIRCOST.
    let discount_mod = host
        .hub_ref()
        .reputation_price_discount_for_faction_template_like_cpp(repair_npc.faction_template_id);
    let repair_cost_rate = host.hub_ref().config.repair_cost_rate_like_cpp();

    if !repair.item_guid.is_empty() {
        let repaired = host
            .repair_inventory_item_durability_with_generator_like_cpp(
                item_guid_generator,
                repair.item_guid,
                true,
                discount_mod,
                repair_cost_rate,
            )
            .await;
        debug!(
            npc_guid = ?repair.npc_guid,
            item_guid = ?repair.item_guid,
            repaired,
            account = host.core().account_id,
            "RepairItem single-item represented runtime"
        );
        return;
    }

    if repair.use_guild_bank {
        let repaired = host
            .repair_all_inventory_item_durability_with_guild_bank_and_generator_like_cpp(
                item_guid_generator,
                discount_mod,
                repair_cost_rate,
            )
            .await;
        debug!(
            npc_guid = ?repair.npc_guid,
            repaired,
            account = host.core().account_id,
            "RepairItem all-items represented guild-bank runtime"
        );
        return;
    }

    let repaired = host
        .repair_all_inventory_item_durability_with_player_money_and_generator_like_cpp(
            item_guid_generator,
            discount_mod,
            repair_cost_rate,
        )
        .await;
    debug!(
        npc_guid = ?repair.npc_guid,
        repaired,
        account = host.core().account_id,
        "RepairItem all-items represented runtime"
    );
}
