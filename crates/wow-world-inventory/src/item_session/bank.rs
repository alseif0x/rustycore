// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The autobank and autostore-bank moves.

use tracing::debug;
use wow_entities::{NULL_BAG, NULL_SLOT};
use wow_packet::packets::misc::{AutoBankItem, AutoStoreBankItem};
use wow_world_entities::CreatureSpawnCatalogsLikeCpp;

use super::{ItemSessionHostLikeCpp, autostore_bank_quest_checks_like_cpp};
use crate::{
    InventoryStorageQuestChecksLikeCpp, InventoryStorageTargetLikeCpp,
    RepresentedBankItemMoveLikeCpp, autostore_bank_target_like_cpp,
};

/// CMSG_AUTOBANK_ITEM — player moves an inventory item into bank storage.
///
/// C++ ref: `WorldSession::HandleAutoBankItemOpcode`.
pub async fn handle_autobank_item_with_generator_like_cpp<H: ItemSessionHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    packet: AutoBankItem,
) {
    if !crate::can_use_current_bank_with_access_like_cpp(
        host.interaction(),
        &host.hub_ref().trainer_npc_interaction_access_like_cpp(),
        &host.core().owned_inventory_access_like_cpp(),
    ) {
        debug!(
            bag = packet.bag,
            slot = packet.slot,
            account = host.core().account_id,
            "AutoBankItem rejected: player cannot use current bank"
        );
        return;
    }

    let represented_move = RepresentedBankItemMoveLikeCpp {
        to_bank: true,
        inv_update_items: packet.inv_update.items,
        bag: packet.bag,
        slot: packet.slot,
    };
    host.execute_inventory_storage_move_like_cpp(
        item_guid_generator,
        creature_spawn_catalogs,
        packet.bag,
        packet.slot,
        NULL_BAG,
        NULL_SLOT,
        InventoryStorageTargetLikeCpp::Bank,
        InventoryStorageQuestChecksLikeCpp::AutoBankItemRemoved,
        Some(represented_move),
    )
    .await;
}

/// CMSG_AUTOSTORE_BANK_ITEM — player moves a bank item back to inventory, or inventory to bank.
///
/// C++ ref: `WorldSession::HandleAutoStoreBankItemOpcode`.
pub async fn handle_autostore_bank_item_with_generator_like_cpp<H: ItemSessionHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    packet: AutoStoreBankItem,
) {
    if !crate::can_use_current_bank_with_access_like_cpp(
        host.interaction(),
        &host.hub_ref().trainer_npc_interaction_access_like_cpp(),
        &host.core().owned_inventory_access_like_cpp(),
    ) {
        debug!(
            bag = packet.bag,
            slot = packet.slot,
            account = host.core().account_id,
            "AutoStoreBankItem rejected: player cannot use current bank"
        );
        return;
    }

    let target = autostore_bank_target_like_cpp(packet.bag, packet.slot);
    let represented_move = RepresentedBankItemMoveLikeCpp {
        to_bank: target == InventoryStorageTargetLikeCpp::Bank,
        inv_update_items: packet.inv_update.items,
        bag: packet.bag,
        slot: packet.slot,
    };
    host.execute_inventory_storage_move_like_cpp(
        item_guid_generator,
        creature_spawn_catalogs,
        packet.bag,
        packet.slot,
        NULL_BAG,
        NULL_SLOT,
        target,
        autostore_bank_quest_checks_like_cpp(target),
        Some(represented_move),
    )
    .await;
}
