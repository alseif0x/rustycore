// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spell cast handlers — CMSG_CAST_SPELL, CMSG_CANCEL_CAST, CMSG_CANCEL_CHANNELLING.
//!
//! Normal requests decode/adapt into `player_cast`; canonical state and
//! publication adapters live under `session/player_cast`. Immediate and queued
//! requests share preparation, while the existing driver consumes timed casts.
//! Other represented spell/item handlers retain their explicit operation paths.
//! Reference: Classic Game/Handlers/SpellHandler.cpp, Player.cpp and Spell.cpp.

use std::collections::HashMap;

use rand::Rng;
use tracing::{debug, warn};

use wow_constants::{
    BagFamilyMask, ClientOpcodes, InventoryResult, ItemFieldFlags, ItemFlags, ItemUpdateState,
    TypeId,
};
use wow_core::ObjectGuid;
use wow_data::{DISABLE_TYPE_SPELL, DisableWorldObjectRefLikeCpp};
use wow_entities::INVENTORY_SLOT_BAG_0;
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_loot::{
    LootConditionRowLikeCpp, condition_compare_values_like_cpp,
    loot_condition_reference_ids_like_cpp, loot_condition_reference_self_references_like_cpp,
    loot_condition_row_normalize_without_external_stores_like_cpp,
    loot_conditions_allow_player_with_references_like_cpp_representable,
};
use wow_packet::ClientPacket;
use wow_packet::packets::item::{ItemExpirePurchaseRefund, ItemInstance};
use wow_packet::packets::loot::{
    CreatureLoot, LOOT_TYPE_ITEM_LIKE_CPP, LootEntry, LootEntryFlags, LootItemData, LootResponse,
};
use wow_packet::packets::pet::PetCancelAura;
use wow_packet::packets::spell::{
    CancelAura, CancelAutoRepeatSpell, CancelCast, CancelChannelling, CancelGrowthAura,
    CancelModSpeedNoControlAuras, CancelMountAura, CancelQueuedSpell, CastSpellRequest, OpenItem,
    SelfRes, SpellClick,
};
use wow_packet::packets::totem::TotemDestroyed;

use crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP;
use crate::session::{
    AreaTriggerCatalogsLikeCpp, RepresentedPendingSpellCastRequestLikeCpp, WorldSession,
};

mod ops_1;
mod ops_2;
mod state;
#[allow(unused_imports)]
pub use ops_1::*;
#[allow(unused_imports)]
pub use ops_2::*;
#[allow(unused_imports)]
pub use state::*;

// ── Handler registrations ─────────────────────────────────────────

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CastSpell,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_cast_spell",
        handler: |session, catalogs, pkt| {
            Box::pin(async move {
                session
                    .handle_cast_spell_with_catalogs_like_cpp(
                        catalogs.area_triggers.as_ref(),
                        catalogs.creature_spawns.as_ref(),
                        catalogs.progression.as_ref(),
                        &catalogs.player_grid_loader,
                        catalogs.id_generators.item.as_ref(),
                        pkt,
                    )
                    .await
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CancelCast,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_cancel_cast",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_cancel_cast(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CancelAura,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_cancel_aura",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_cancel_aura(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CancelAutoRepeatSpell,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_cancel_auto_repeat_spell",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_cancel_auto_repeat_spell(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CancelChannelling,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_cancel_channelling",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_cancel_channelling(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CancelGrowthAura,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_cancel_growth_aura",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_cancel_growth_aura(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CancelMountAura,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_cancel_mount_aura",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_cancel_mount_aura(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CancelQueuedSpell,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_cancel_queued_spell",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_cancel_queued_spell(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::OpenItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_open_item",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_open_item(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SelfRes,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_self_res",
        handler: |session, catalogs, pkt| {
            Box::pin(async move {
                session
                    .handle_self_res_with_generator_like_cpp(
                        catalogs.id_generators.item.as_ref(),
                        catalogs.creature_spawns.as_ref(),
                        pkt,
                    )
                    .await
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::PetCancelAura,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_pet_cancel_aura",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_pet_cancel_aura(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::TotemDestroyed,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_totem_destroyed",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_totem_destroyed(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SpellClick,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_spell_click",
        handler: |session, catalogs, pkt| {
            Box::pin(async move {
                session
                    .handle_spell_click_with_generator_like_cpp(
                        catalogs.id_generators.item.as_ref(),
                        catalogs.creature_spawns.as_ref(),
                        pkt,
                    )
                    .await
            })
        },
    }
}

// ── Handler implementations ───────────────────────────────────────

#[cfg(test)]
#[path = "spell/tests/mod.rs"]
mod tests;
